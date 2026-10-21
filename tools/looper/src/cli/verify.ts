// `verify` and `affected`: the gate a session runs once before it wraps, and which of its units a change
// reaches, on the project in the current directory. They port `crates/looper/src/cmd/verify.rs`'s
// command and `crates/looper/src/cmd/affected.rs`.
//
// - `verify [--all]` asks for a verify of the tree and waits for its one run, which it starts or joins
//   (`../verify/shared.ts`). It prints the run's lines as they come, and exits as the run does: 1 after
//   the first red step, and with `--all` when any is red. With `--json` the lines go to stderr, and it
//   prints the run's `Verified`. While it waits it tells the hang watch so (`../jobs/hang.ts`).
// - `verify --status` prints the progress items of the tree's newest run without joining it, as `exec
//   progress` prints them. A tree with no run says so, and exits 0.
// - `verify --supervise <run folder> [--all]` is the run's supervisor, which a request spawns. It has
//   no stdio, so it writes what stopped it to the run's `supervisor.log`.
// - `affected` decides as verify does when it starts. It builds the cargo step with `--no-run` to learn
//   its binaries, drops the store's units no longer defined, and marks each reached unit owed. So a
//   reached unit runs at the next verify even if the change is undone first.
// - `affected --dry` builds nothing and writes nothing. A cargo step's units are then the ones the store
//   holds, so a test binary added since the last verify is not named. A failed build reads them from
//   the store the same way, and says so.
// - `affected` prints one line per reached unit with its reason, then a count. `--explain <unit>`
//   prints that unit's verdict and what it rested on at its last green. With `--json` it prints one
//   `Affected`, or with `--explain` one `Explained`.
//
// `affected` exits 0 whatever it finds. A refused config or an unknown unit exits 1.

import { appendFileSync } from "node:fs";
import { join } from "node:path";
import { type Config, loadConfig } from "../goal/config.ts";
import { waitOnPurpose } from "../jobs/hang.ts";
import { compare, explain, peek, type Selection, Tree, type UnitDef, unitDef, reason as reasonOf, type Store } from "../verify/select.ts";
import { request, statusJob, SUPERVISOR_LOG, supervise } from "../verify/shared.ts";
import type { Build } from "../verify/cargo.ts";
import { build, cargoDefinition, cargoUnits, isCargoTest, openStore, projectEnv, splitCargo, stepUnit, steps } from "../verify/verify.ts";
import { type Command, type Context, Refusal, type Report, usage } from "./cli.ts";
import { progress } from "./exec.ts";

/** Why a unit is kept. */
const KEPT = "nothing it rests on changed since its last green";

/** What `affected --json` prints: the reached units first, in the order the selection found them, then the kept ones. */
export interface Affected {
  units: Affect[];
  reached: number;
  total: number;
  dry: boolean;
  /** Why the cargo step's build failed, when it did, so its units are the ones the store holds. */
  build_failed: string | null;
}

/** One of verify's units. */
export interface Affect {
  id: string;
  /** Whether the next `verify` runs it. */
  runs: boolean;
  /** Why it runs, or why it is kept. */
  why: string;
}

/** What `affected --explain <unit> --json` prints. */
export interface Explained extends Affect {
  /** How many files it rested on at its last green. */
  files: number;
  /** Whether it rests on the whole tree, because nothing was attributed to it. */
  whole_tree: boolean;
  /** The directories it rests on every file under. */
  dirs: string[];
  /** The globs it rests on every file matching. */
  globs: string[];
}

/** The project's config, or the refusal that names what is wrong with it. */
function config(root: string): Config {
  try {
    return loadConfig(root);
  } catch (e) {
    throw new Refusal((e as Error).message);
  }
}

/** The project's verify steps, or the refusal that names why there are none to run. */
function stepsOf(cfg: Config) {
  try {
    return steps(cfg);
  } catch (e) {
    throw new Refusal((e as Error).message);
  }
}

/** The supervisor, which has no stdio: whatever stops it is written to the run's log. */
async function supervisor(root: string, runDir: string, all: boolean): Promise<Report> {
  try {
    return { code: await supervise(root, runDir, all), own: true };
  } catch (e) {
    try {
      appendFileSync(join(runDir, SUPERVISOR_LOG), `error: ${(e as Error).message}\n`);
    } catch {}
    return { code: 1, own: true };
  }
}

function status(root: string): Report {
  const job = statusJob(root);
  if (job === null) return { doc: { id: null }, text: `No verify run of ${root}.` };
  return progress([job]);
}

export const verifyCommand: Command = {
  name: "verify",
  about: "Runs the verify steps a change reaches, in the tree's one run: `verify [--all] [--status]`.",
  json: true,
  run: async (args, ctx) => {
    let all = false;
    let isStatus = false;
    let runDir: string | undefined;
    for (let i = 0; i < args.length; i++) {
      const a = args[i] as string;
      if (a === "--all") all = true;
      else if (a === "--status") isStatus = true;
      else if (a === "--supervise") {
        runDir = args[++i];
        if (runDir === undefined) throw usage("`--supervise` takes a run's folder");
      } else throw usage(`\`verify\` does not take \`${a}\``);
    }
    const root = process.cwd();
    if (runDir !== undefined) return supervisor(root, runDir, all);
    if (isStatus) {
      if (all) throw usage("`verify --status` takes no `--all`");
      return status(root);
    }
    stepsOf(config(root));
    // The run goes on outside this call's job, so a quiet step is not a hang of this call.
    const waiting = waitOnPurpose("verify waits for the tree's run");
    const ended = await request(root, all, ctx.line).finally(waiting);
    if (ended.error !== null) throw new Refusal(ended.error);
    return { code: ended.code, doc: ended.report ?? {} };
  },
};

/** Drops the stored units under `scope` that `units` does not name, as verify does. */
function retain(store: Store, scope: string, units: UnitDef[]): void {
  store.retain(
    scope,
    units.map((u) => u.id).filter((id) => id.startsWith(scope)),
  );
}

/** What a change reaches among verify's units of the project at `root`, or with `explainId` why that one unit runs. */
async function affected(root: string, explainId: string | null, dry: boolean, ctx: Context): Promise<Report> {
  const cfg = config(root);
  const list = stepsOf(cfg);
  const store = openStore(root);
  const tree = Tree.list(root);
  const units: UnitDef[] = [];
  let buildFailed: string | null = null;
  // Whether every cargo unit came from a build, so the stored ones it does not name can be dropped.
  let fresh = true;
  for (const step of list) {
    if (!isCargoTest(step.run)) {
      units.push(stepUnit(root, step));
      continue;
    }
    let made: Build | null = null;
    if (!dry) {
      const built = await build(root, cfg.checks, projectEnv(root, cfg), splitCargo(step.run)[0], step.heavy ? ctx.err : null);
      if (typeof built.build === "string") {
        ctx.line(`${built.build}, so the test binaries are the ones the store holds; \`verify\` shows why`);
        buildFailed = built.build;
      } else made = built.build;
    }
    if (made !== null) units.push(...cargoUnits(step, made));
    else {
      fresh = false;
      const definition = cargoDefinition(step);
      units.push(...store.ids("cargo:").map((id) => unitDef(id, definition)));
    }
  }
  let selection: Selection;
  if (dry) selection = peek(store, tree, units);
  else {
    retain(store, "step:", units);
    if (fresh) retain(store, "cargo:", units);
    selection = compare(store, tree, units);
  }
  if (explainId !== null) return explained(store, selection, units, explainId);

  const doc: Affected = { units: [], reached: selection.run.length, total: units.length, dry, build_failed: buildFailed };
  for (const [id, why] of selection.run) doc.units.push({ id, runs: true, why: explain(why) });
  for (const u of units) if (reasonOf(selection, u.id) === undefined) doc.units.push({ id: u.id, runs: false, why: KEPT });
  const lines = doc.units.filter((u) => u.runs).map((u) => `${u.id}: ${u.why}`);
  lines.push(doc.reached === 0 ? `0 of ${doc.total} units reached; every last green holds` : `${doc.reached} of ${doc.total} units reached; \`verify\` runs them`);
  return { doc, text: lines.join("\n") };
}

/** Whether the unit `id` runs and why, then what it rested on at its last green. */
function explained(store: Store, selection: Selection, units: UnitDef[], id: string): Report {
  if (!units.some((u) => u.id === id)) {
    throw new Refusal(
      `\`${id}\` is none of verify's ${units.length} units. A unit is \`step:<name>\` or \`cargo:<package>/<kind>/<target>\`, and \`affected\` names the reached ones.`,
    );
  }
  const why = reasonOf(selection, id);
  const doc: Explained = { id, runs: why !== undefined, why: why === undefined ? KEPT : explain(why), files: store.files(id).length, whole_tree: false, dirs: [], globs: [] };
  const lines = [`${id} ${doc.runs ? "runs" : "is kept"}: ${doc.why}`];
  if (doc.files > 0) {
    const dirs = store.dirs(id);
    doc.whole_tree = dirs.includes("");
    doc.dirs = dirs.filter((d) => d !== "");
    doc.globs = store.globs(id);
    lines.push(`At its last green it rested on ${doc.files} files.`);
    if (doc.whole_tree) lines.push("It rests on the whole tree, because nothing was attributed to it.");
    else if (doc.dirs.length > 0) lines.push(`It rests on every file under ${doc.dirs.join(", ")}.`);
    if (doc.globs.length > 0) lines.push(`It rests on every file matching ${doc.globs.join(", ")}.`);
  }
  return { doc, text: lines.join("\n") };
}

export const affectedCommand: Command = {
  name: "affected",
  about: "Names the units of `verify` that a change reaches, and why, without running any: `affected [--explain <unit>] [--dry]`.",
  json: true,
  run: (args, ctx) => {
    let explainId: string | null = null;
    let dry = false;
    for (let i = 0; i < args.length; i++) {
      const a = args[i] as string;
      if (a === "--dry") dry = true;
      else if (a === "--explain") {
        const unit = args[++i];
        if (unit === undefined) throw usage("`--explain` takes a unit");
        explainId = unit;
      } else throw usage(`\`affected\` does not take \`${a}\``);
    }
    return affected(process.cwd(), explainId, dry, ctx);
  },
};
