// The verify steps of one run: `looper.toml`'s `[verify]` steps in order, each run only when a change
// reaches it, stopping after the first red one. It ports the steps of `crates/looper/src/cmd/verify.rs`.
// The one run per tree that calls `runSteps` is `shared.ts`'s.
//
// - A step is one unit, `step:<name>`, resting on its `inputs`, or with none on the whole tree. Its
//   definition holds its argv, its inputs and the project's root. A step whose last green still holds
//   prints one `·` line and does not run.
// - A step whose argv is `cargo test …` is built once with `--no-run --message-format=json`, then
//   `cargo metadata` is read, and both parse into a `Build` (`cargo.ts`). Each test binary is a unit,
//   `cargo:<package>/<kind>/<target>`, run by its path from its package's directory with the arguments
//   after `--`. Every reached binary runs, those red last time first, and the step is red when any is.
// - A `cargo test` step with `inputs` is also gated by its own unit, `step:<name>`, resting on them.
//   When nothing they cover changed since the step last ran whole and green, the step neither builds
//   nor runs, and keeps every binary. Its `inputs` must cover every file its binaries read.
// - The run writes its progress items through `items.ts`: one per step and per test binary, kept or
//   run. A step's command is a child job, whose own items nest under the step's item.
// - Every command is a `verify` job of the project's job store (`../jobs/spawn.ts`), so a timeout
//   kills its whole tree. It gets `[checks] env`, then `LOOPER_GOAL`, `LOOPER_STAGE` and
//   `LOOPER_EXTRA`. A unit runs for at most `VERIFY_TIMEOUT_S`, and a cargo step's build and `cargo
//   metadata` each for at most `[checks] timeout`.
// - A heavy step's units, and a cargo step's build, each hold a heavy-job slot for their run only
//   (`../jobs/slots.ts`). A wait for one prints the service's line and marks the item waiting.
// - Before a unit runs, every file it rests on is digested. A unit whose files changed while it ran
//   records no green. A step that is one unit then runs once more on the new content, so a formatter
//   that rewrote files is judged on its output. A unit void after that is red with the files named.
// - The `Watch` tells of each request that joins the run. A unit in flight whose files a request found
//   changed is killed, and the run goes back to the first step that new content reaches. A red mark or
//   a debt is no new content. Each restart prints a line, and a unit restarted `MAX_RESTARTS` times
//   waits for `QUIET_MS` with no new request before it runs again.
// - A cargo step's build is never killed. Between its binaries, a request that finds any file of the
//   tree changed since the build began makes the step build again.
// - A green run selects once more before it ends, and goes back when an edit came without a request.
//   When its `FINAL_SELECTIONS`th final selection still finds new content, the run ends unverified.
// - A red unit names the files changed since its last green, each with the live session that claims it.
// - A step's log is `<scratch>/verify-<name>.log`, and a test binary's
//   `<scratch>/verify-<name>/<package>.<kind>.<target>.log`. Only a red step's last `TAIL_LINES` lines
//   are printed.
// - `all` runs every step and every test binary, and goes on past a red step. A unit red when the
//   selection had not reached it is added to the store's selection-miss list, with a `!` line.
//
// # The bounds it keeps
//
// - A unit's log is a copy of its job's log, which keeps at most three parts (`../jobs/log.ts`).
// - A cargo command's output is held as its first and last `OUTPUT_HALF` bytes (`output.ts`).
// - A red step's tail is read from the last `TAIL_BYTES` of its log.
// - A run holds one result per step and one per unit of a step, and one restart count per unit.

import { closeSync, mkdirSync, openSync, readSync, fstatSync, writeFileSync, writeSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { dir as claimsDir, listing, machine } from "../claims/claims.ts";
import { goalVars } from "../cli/orient.ts";
import { type Checks, type Config, checkEnv, type Step, verifySteps } from "../goal/config.ts";
import { readLines } from "../jobs/log.ts";
import { type Lease, looperHome, SLOT_VAR, takeSlots } from "../jobs/slots.ts";
import { startJob } from "../jobs/spawn.ts";
import { KILLED_CODE, storeOf } from "../jobs/store.ts";
import { Build, type CargoTest, diagnostics, testId } from "./cargo.ts";
import { BINARY, Items, outcome, STEP } from "./items.ts";
import { Bounded } from "./output.ts";
import {
  changedSinceGreen,
  compare,
  type Digest,
  declared,
  explain,
  type Footprint,
  isEmpty,
  paths,
  peek,
  type Reason,
  reason as reasonOf,
  type Selection,
  Store,
  storeDir,
  Tree,
  type UnitDef,
  unitDef,
  wholeTree,
} from "./select.ts";

/** How long one unit runs before its process tree is killed, in seconds. */
export const VERIFY_TIMEOUT_S = 3600;
/** The lines of a red step's log printed. */
export const TAIL_LINES = 40;
/** The bytes at the end of a log that its tail is read from. */
export const TAIL_BYTES = 256 * 1024;
/** How many restarts of one unit a run makes before each further one waits for a quiet tree. */
export const MAX_RESTARTS = 3;
/** How long a tree goes with no new request before a unit restarted `MAX_RESTARTS` times runs again. */
export const QUIET_MS = 20_000;
/** How many selections a green run makes at its end before a tree that still changes ends it unverified. */
export const FINAL_SELECTIONS = 3;
/** The most files one line names. */
export const NAMED_FILES = 10;
/** How often a unit in flight asks the watch whether it was killed, in ms. */
const KILL_POLL_MS = 100;

/** What a verify prints with `--json`: each step in the order it ran, up to the first red one without `all`. */
export interface Verified {
  green: boolean;
  all: boolean;
  /** The files that still changed after the run's final selections, which left it unverified. */
  unverified: string[];
  steps: StepRun[];
}

/** How a step or a unit ended: `kept` when its last green still holds and it did not run. */
export type Mark = "green" | "red" | "kept";

/** One verify step. */
export interface StepRun {
  name: string;
  run: string[];
  result: Mark;
  /** Why it is red, in one line; empty otherwise. */
  why: string;
  duration_ms: number;
  /** Its full log, when it ran. */
  log: string | null;
  /** A step that is not `cargo test` is one unit, and a `cargo test` step one per test binary it built. */
  units: UnitRun[];
}

/** One unit of a step. */
export interface UnitRun {
  id: string;
  result: Mark;
  /** Why a change reached it, or null when it is kept or ran only for `all`. */
  reason: string | null;
  /** Why it is red, in one line; empty otherwise. */
  why: string;
  log: string | null;
  /** Whether it is red though no change reached it, so it joined the selection-miss list. */
  missed: boolean;
}

/** Where a run's lines go. */
export interface Out {
  line(text: string): void;
}

/** What a run learns of the requests that join it (`shared.ts`). */
export interface Watch {
  /** How many requests have come since the run began. */
  requests(): number;
  /** Names the unit that starts now, resting on `files` with their digests. */
  fly(files: [string, Digest][]): void;
  /** Whether a request found a file of the unit in flight changed. */
  killed(): boolean;
  /** Ends the unit in flight, and gives the files a request found changed under it. */
  land(): string[];
  /** Waits until `ms` pass with no new request. */
  quiet(ms: number): Promise<void>;
}

/** The watch of a run that nothing joins. */
export function lone(): Watch {
  return { requests: () => 0, fly: () => {}, killed: () => false, land: () => [], quiet: async () => {} };
}

/** What `runSteps` is asked. */
export interface Request {
  root: string;
  cfg: Config;
  all: boolean;
  out: Out;
  watch: Watch;
  items: Items;
}

/** Why a verify cannot run. */
export class VerifyRefused extends Error {}

/** `looper.toml`'s verify steps, refused when there are none or more than one `cargo test` step. */
export function steps(cfg: Config): Step[] {
  const all = verifySteps(cfg.verify);
  if (all.length === 0) throw new VerifyRefused("looper.toml has no `[verify]` steps");
  if (all.filter((s) => isCargoTest(s.run)).length > 1) {
    throw new VerifyRefused("looper.toml has more than one `cargo test` verify step; merge them into one");
  }
  return all;
}

/** Whether `argv` runs `cargo test`, with or without a `+toolchain`. */
export function isCargoTest(argv: readonly string[]): boolean {
  const [program, ...rest] = argv;
  if (program === undefined) return false;
  const stem = (program.split(/[\\/]/).pop() ?? "").replace(/\.[^.]*$/, "");
  return stem.toLowerCase() === "cargo" && rest.find((a) => !a.startsWith("+")) === "test";
}

/** The argv of a `cargo test` command that builds its binaries, and the arguments each binary runs with. */
export function splitCargo(argv: readonly string[]): [string[], string[]] {
  const at = argv.indexOf("--");
  return at < 0 ? [[...argv], []] : [argv.slice(0, at), argv.slice(at + 1)];
}

export function stepId(name: string): string {
  return `step:${name}`;
}

/** The unit that `step`, which is not a `cargo test` step, is. */
export function stepUnit(root: string, step: Step): UnitDef {
  return unitDef(stepId(step.name), `${JSON.stringify(step.run)}\n${JSON.stringify(step.inputs)}\n${root}`);
}

/** The unit that gates a `cargo test` step with `inputs`, or none for a step without them. */
export function gateOf(root: string, step: Step): UnitDef[] {
  return step.inputs.length > 0 ? [stepUnit(root, step)] : [];
}

/** The definition each unit of a `cargo test` step shares. */
export function cargoDefinition(step: Step): string {
  return JSON.stringify(step.run);
}

/** The units of a `cargo test` step: one per test binary its build made. */
export function cargoUnits(step: Step, build: Build): UnitDef[] {
  const definition = cargoDefinition(step);
  return build.tests.map((t) => unitDef(testId(t), definition));
}

/** The project's unit store. */
export function openStore(root: string): Store {
  return Store.open(storeDir(root));
}

/** The environment a project command of verify runs with: `[checks] env`, then the live goal's variables. */
export function projectEnv(root: string, cfg: Config): Record<string, string | undefined> {
  const env = checkEnv(cfg.checks);
  for (const [k, v] of Object.entries(goalVars(root, join(root, cfg.project.goals_dir)))) {
    for (const name of Object.keys(env).filter((n) => n.toUpperCase() === k)) delete env[name];
    if (v !== undefined) env[k] = v;
  }
  return env;
}

/** The first `NAMED_FILES` of `files`, and how many more there are. */
export function named(files: readonly string[]): string {
  const text = files.slice(0, NAMED_FILES).join(", ");
  return files.length > NAMED_FILES ? `${text} and ${files.length - NAMED_FILES} more` : text;
}

/** What every step of one run shares. */
interface Ctx {
  root: string;
  cfg: Config;
  env: Record<string, string | undefined>;
  logs: string;
  store: Store;
  all: boolean;
  out: Out;
  watch: Watch;
  items: Items;
}

/** `path` relative to the run's root with `/` between its parts, or as given outside it. */
function relOf(ctx: Ctx, path: string): string {
  const r = relative(ctx.root, path);
  return r.startsWith("..") ? path : r.replaceAll("\\", "/");
}

/**
 * Runs the verify steps, each only when a change reaches it, and stops after the first red step. With
 * `all` it runs every unit and every step, and records each selection miss. Throws `VerifyRefused`
 * for a config it cannot run.
 */
export async function runSteps(req: Request): Promise<Verified> {
  const list = steps(req.cfg);
  req.items.steps(list, isCargoTest);
  const store = openStore(req.root);
  store.retain(
    "step:",
    list.map((s) => stepId(s.name)),
  );
  const ctx: Ctx = { ...req, env: projectEnv(req.root, req.cfg), logs: join(req.root, req.cfg.project.scratch), store };
  const pass = new Pass(list.length);
  let seen = req.watch.requests();
  let finals = 0;
  let unverified: string[] = [];
  let i = 0;
  while (i < list.length || pass.green()) {
    if (i === list.length) {
      finals++;
      const found = pass.reached(ctx, list, i);
      if (found === null) break;
      if (finals >= FINAL_SELECTIONS) {
        unverified = stillChanged(found[1], ctx.out);
        break;
      }
      i = pass.restartAt(ctx, found[0], found[1]);
      continue;
    }
    if (pass.quiet !== null) {
      const [unit, n] = pass.quiet;
      pass.quiet = null;
      ctx.out.line(`verify: ${unit} restarted ${n} times; waiting for ${QUIET_MS / 1000} s with no new request`);
      await ctx.watch.quiet(QUIET_MS);
    }
    const step = list[i] as Step;
    const t0 = Date.now();
    const stepped = isCargoTest(step.run) ? await cargoStep(ctx, step, pass.known, i) : await plainStep(ctx, step);
    if ("interrupted" in stepped) {
      seen = ctx.watch.requests();
      if (stepped.interrupted !== null) pass.restarted(ctx, stepped.interrupted[0], stepped.interrupted[1]);
      const found = pass.reached(ctx, list, i);
      if (found !== null) i = pass.restartAt(ctx, found[0], found[1]);
      continue;
    }
    const ran = stepped.done;
    ran.duration_ms = Date.now() - t0;
    pass.done[i] = ran;
    if (ran.result === "red" && !ctx.all) break;
    i++;
    if (ctx.watch.requests() !== seen) {
      seen = ctx.watch.requests();
      const found = pass.reached(ctx, list, i);
      if (found !== null) i = pass.restartAt(ctx, found[0], found[1]);
    }
  }
  const done = pass.done.filter((s): s is StepRun => s !== null);
  const green = unverified.length === 0 && done.every((s) => s.result !== "red");
  ctx.items.done(green);
  return { green, all: ctx.all, unverified, steps: done };
}

/** The files of `reached` once the run's final selections still found them changed, which leave it unverified, and says so. */
function stillChanged(reached: Reached, out: Out): string[] {
  const files = [...new Set(reached.flatMap(([, f]) => f))].sort();
  out.line(`✗ verify: the tree still changed after ${FINAL_SELECTIONS} final selections, in ${named(files)}; the run is unverified`);
  return files;
}

/** The units of one step that new content reached, each with the files that changed since its last green. */
type Reached = [string, string[]][];

/** How a step's run ended: done, or cut short by new content, with the unit killed in flight and the files that changed under it. */
type Stepped = { done: StepRun } | { interrupted: [string, string[]] | null };

/** Where a run stands: each step's newest result in this pass, the units each cargo step built, and how many times each unit restarted. */
class Pass {
  done: (StepRun | null)[];
  known: UnitDef[][];
  readonly restarts = new Map<string, number>();
  /** The unit that restarted `MAX_RESTARTS` times, and how often, before the next step waits. */
  quiet: [string, number] | null = null;

  constructor(n: number) {
    this.done = Array.from({ length: n }, () => null);
    this.known = Array.from({ length: n }, () => []);
  }

  green(): boolean {
    return this.done.every((s) => s === null || s.result !== "red");
  }

  /** The first of the steps before `upto` that new content reaches, with each unit of it reached and the files that changed since its last green. */
  reached(ctx: Ctx, list: Step[], upto: number): [number, Reached] | null {
    const tree = Tree.list(ctx.root);
    for (let k = 0; k < upto; k++) {
      const step = list[k] as Step;
      const units = isCargoTest(step.run) ? [...(this.known[k] ?? []), ...gateOf(ctx.root, step)] : [stepUnit(ctx.root, step)];
      const reached: Reached = [];
      for (const [id, why] of peek(ctx.store, tree, units).run) {
        if (why.kind === "red" || why.kind === "owed") continue;
        const files = changedSinceGreen(ctx.store, tree, id);
        reached.push([id, files.length > 0 ? files : [explain(why)]]);
      }
      if (reached.length > 0) return [k, reached];
    }
    return null;
  }

  /** Restarts each of `reached`, a unit of step `j`, and returns `j`, the step the run goes on from. */
  restartAt(ctx: Ctx, j: number, reached: Reached): number {
    for (const [unit, files] of reached) this.restarted(ctx, unit, files);
    for (let k = j; k < this.done.length; k++) this.done[k] = null;
    return j;
  }

  /** Says that `unit` restarted for changes in `files`, and counts it. */
  restarted(ctx: Ctx, unit: string, files: string[]): void {
    ctx.out.line(`verify: restarted ${unit} for changes in ${named(files)}`);
    const n = (this.restarts.get(unit) ?? 0) + 1;
    this.restarts.set(unit, n);
    if (n >= MAX_RESTARTS) this.quiet = [unit, n];
  }
}

function newStep(step: Step, result: Mark, why: string): StepRun {
  return { name: step.name, run: [...step.run], result, why, duration_ms: 0, log: null, units: [] };
}

function keptUnit(id: string): UnitRun {
  return { id, result: "kept", reason: null, why: "", log: null, missed: false };
}

/** Digests every file `footprint` covers, or the whole tree for an empty one: what a unit's run starts from. */
function primed(tree: Tree, footprint: Footprint): [string, Digest][] {
  const f = isEmpty(footprint) ? wholeTree() : footprint;
  return paths(f, tree).map((p) => [p, tree.digest(p)]);
}

/** Runs a step that is one unit, when a change reaches it or `all` holds. A request that finds its files changed while it runs kills it. */
async function plainStep(ctx: Ctx, step: Step): Promise<Stepped> {
  const t0 = Date.now();
  let tree = Tree.list(ctx.root);
  const unit = stepUnit(ctx.root, step);
  const why = reasonOf(compare(ctx.store, tree, [unit]), unit.id);
  const reached = why !== undefined;
  if (!reached && !ctx.all) {
    ctx.out.line(`· ${step.name}: kept, nothing it reads changed`);
    ctx.items.kept(ctx.store, unit.id, null, step.name, STEP);
    const ran = newStep(step, "kept", "");
    ran.units.push(keptUnit(unit.id));
    return { done: ran };
  }
  const log = join(ctx.logs, `verify-${step.name}.log`);
  let first = true;
  let v: Verdict;
  let isVoid: boolean;
  for (;;) {
    const footprint = declared(tree, step.inputs);
    const files = primed(tree, footprint);
    const ran = await runUnit(ctx, unit.id, step.run, ctx.root, log, files, step.heavy);
    if ("killed" in ran) return { interrupted: [unit.id, ran.killed] };
    v = ran.done;
    const recorded = ctx.store.record(tree, unit, v.green ? "green" : "red", footprint);
    if (first && recorded.kind === "void") {
      ctx.out.line(`~ ${step.name}: its files changed while it ran, as a formatter's writes do; running it again`);
      first = false;
      tree = Tree.list(ctx.root);
      continue;
    }
    isVoid = voided(recorded, v);
    break;
  }
  const secs = ((Date.now() - t0) / 1000).toFixed(1);
  const shown = step.run.join(" ");
  const missed = !v.green && !reached && !isVoid;
  if (v.green) {
    ctx.out.line(`✓ ${step.name}: ${shown} (${secs}s)`);
  } else {
    ctx.out.line(`✗ ${step.name}: ${shown} (${secs}s): ${v.why}. Full log: ${log}`);
    if (missed) miss(ctx, unit.id);
    blame(ctx, tree, unit.id);
    printTail(log, ctx.out);
  }
  const done = newStep(step, v.green ? "green" : "red", v.why);
  done.log = relOf(ctx, log);
  ctx.items.end(unit.id, outcome(v.green), v.why, done.log);
  done.units.push({ id: unit.id, result: done.result, reason: reached ? explain(why as Reason) : null, why: v.why, log: done.log, missed });
  return { done };
}

/** Names the files of the red unit `id` that changed since its last green, each with the live session that claims it. */
function blame(ctx: Ctx, tree: Tree, id: string): void {
  const files = changedSinceGreen(ctx.store, tree, id);
  if (files.length === 0) return;
  let claims: ReturnType<typeof listing> = [];
  try {
    claims = listing(claimsDir(ctx.root), machine, Date.now());
  } catch {}
  const root = slashed(ctx.root).toLowerCase();
  const claimed = files.map((file) => {
    const by = claims.find((c) => c.live && slashed(c.tree).toLowerCase() === root && c.path.toLowerCase() === file.toLowerCase());
    return by ? `${file} (claimed by session ${by.session_id})` : file;
  });
  ctx.out.line(`  ${id}: changed since its last green: ${named(claimed)}`);
}

/** `path` with `/` between its parts and no `/` at its end. */
function slashed(path: string): string {
  return path.replaceAll("\\", "/").replace(/\/+$/, "");
}

/**
 * Runs a `cargo test` step, unless its gate holds: it has `inputs`, nothing they cover changed since its
 * last whole green run, and `all` is off. A run records the gate's result, and with `all` a red run the
 * gate did not reach is a selection miss.
 */
async function cargoStep(ctx: Ctx, step: Step, known: UnitDef[][], i: number): Promise<Stepped> {
  const gate = gateOf(ctx.root, step)[0];
  if (gate === undefined) return cargoRun(ctx, step, known, i);
  const tree = Tree.list(ctx.root);
  const reached = reasonOf(compare(ctx.store, tree, [gate]), gate.id) !== undefined;
  if (!reached && !ctx.all) {
    ctx.out.line(`· ${step.name}: kept, nothing it reads changed`);
    ctx.items.kept(ctx.store, gate.id, null, step.name, STEP);
    ctx.items.planned();
    const ran = newStep(step, "kept", "");
    ran.units.push(keptUnit(gate.id));
    return { done: ran };
  }
  const footprint = declared(tree, step.inputs);
  primed(tree, footprint);
  const stepped = await cargoRun(ctx, step, known, i);
  if ("done" in stepped) {
    const green = stepped.done.result !== "red";
    ctx.store.record(tree, gate, green ? "green" : "red", footprint);
    if (!green && !reached) miss(ctx, gate.id);
  }
  return stepped;
}

/**
 * Builds a `cargo test` step once and runs each reached test binary by its path, or with `all` every
 * one. The step is red when the build or any binary it ran is. Its binaries become `known[i]`.
 */
async function cargoRun(ctx: Ctx, step: Step, known: UnitDef[][], i: number): Promise<Stepped> {
  const t0 = Date.now();
  let seen = ctx.watch.requests();
  const tree = Tree.list(ctx.root);
  const whole = primed(tree, wholeTree());
  const [buildArgv, testArgs] = splitCargo(step.run);
  mkdirSync(ctx.logs, { recursive: true });
  const log = join(ctx.logs, `verify-${step.name}.log`);
  const stepItem = stepId(step.name);
  ctx.items.start(stepItem, null);
  const built = await build(ctx.root, ctx.cfg.checks, ctx.env, buildArgv, step.heavy ? (line) => said(ctx, stepItem, line) : null);
  let text = built.text;
  const shown = step.run.join(" ");
  if (typeof built.build === "string") return { done: buildFailed(ctx, step, log, text, built.build, t0) };
  const b = built.build;
  const units = cargoUnits(step, b);
  known[i] = units;
  const [selection, order] = planBinaries(ctx, tree, stepItem, b, units);

  const binLogs = join(ctx.logs, `verify-${step.name}`);
  const red: [string, Verdict][] = [];
  const runs: UnitRun[] = [];
  let ran = 0;
  for (const [test, unit] of order) {
    const why = reasonOf(selection, unit.id);
    const reached = why !== undefined;
    if (!reached && !ctx.all) {
      runs.push(keptUnit(unit.id));
      continue;
    }
    const came = ctx.watch.requests();
    if (came !== seen) {
      seen = came;
      if (tree.changed(whole).length > 0) {
        ctx.items.end(stepItem, "cancelled", "the tree changed since its build began", null);
        return { interrupted: null };
      }
    }
    ran++;
    const footprint = b.footprint(test, tree, ctx.root);
    const files = primed(tree, footprint);
    const unitLog = join(binLogs, `${test.package}.${test.kind}.${test.name}.log`);
    const r = await runUnit(ctx, unit.id, [test.executable, ...testArgs], test.packageDir, unitLog, files, step.heavy);
    if ("killed" in r) {
      ctx.items.end(stepItem, "cancelled", `${unit.id} restarted`, null);
      return { interrupted: [unit.id, r.killed] };
    }
    const v = r.done;
    const isVoid = voided(ctx.store.record(tree, unit, v.green ? "green" : "red", footprint), v);
    text += `${v.green ? "✓" : "✗"} ${unit.id} ${v.why}: ${v.log ?? ""}\n`;
    const shownLog = v.log === null ? null : relOf(ctx, v.log);
    ctx.items.end(unit.id, outcome(v.green), v.why, shownLog);
    runs.push({ id: unit.id, result: v.green ? "green" : "red", reason: reached ? explain(why) : null, why: v.why, log: shownLog, missed: !v.green && !reached && !isVoid });
    if (!v.green) red.push([unit.id, v]);
  }
  writeFileSync(log, text);

  const secs = ((Date.now() - t0) / 1000).toFixed(1);
  const firstRed = red[0];
  if (firstRed === undefined) {
    ctx.out.line(`✓ ${step.name}: ${shown}: ran ${ran} of ${units.length} test binaries, the rest kept (${secs}s)`);
    const done = newStep(step, ran === 0 ? "kept" : "green", "");
    done.log = relOf(ctx, log);
    done.units = runs;
    ctx.items.end(stepItem, "green", "", done.log);
    return { done };
  }
  const why = `${red.length} of the ${ran} test binaries it ran are red`;
  ctx.items.end(stepItem, "red", why, relOf(ctx, log));
  ctx.out.line(`✗ ${step.name}: ${shown} (${secs}s): ${why}. Full log: ${log}`);
  for (const [id, v] of red) {
    ctx.out.line(`  ✗ ${id}: ${v.why}. Log: ${v.log ?? ""}`);
    blame(ctx, tree, id);
  }
  for (const unit of runs.filter((u) => u.missed)) miss(ctx, unit.id);
  if (firstRed[1].log !== null) printTail(firstRed[1].log, ctx.out);
  const done = newStep(step, "red", why);
  done.log = relOf(ctx, log);
  done.units = runs;
  return { done };
}

/**
 * Selects among `units`, the units of `build`'s test binaries, after dropping the store's files of
 * binaries gone. Returns the selection and each binary with its unit in the order they run: those red
 * last time first, then the rest in build order. It adds an item for each under `stepItem`, kept when
 * the selection keeps it, and completes the plan.
 */
function planBinaries(ctx: Ctx, tree: Tree, stepItem: string, b: Build, units: UnitDef[]): [Selection, [CargoTest, UnitDef][]] {
  ctx.store.retain(
    "cargo:",
    units.map((u) => u.id),
  );
  const selection = compare(ctx.store, tree, units);
  const order = b.tests.map((t, k) => [t, units[k] as UnitDef] as [CargoTest, UnitDef]);
  const wasRed = (u: UnitDef) => ctx.store.unit(u.id)?.red === true;
  order.sort((x, y) => Number(wasRed(y[1])) - Number(wasRed(x[1])));
  for (const [, unit] of order) {
    if (reasonOf(selection, unit.id) !== undefined || ctx.all) ctx.items.add(unit.id, stepItem, unit.id, BINARY);
    else ctx.items.kept(ctx.store, unit.id, stepItem, unit.id, BINARY);
  }
  ctx.items.planned();
  return [selection, order];
}

/** Writes a cargo step's log `text` when its build failed for `why`, says so, and returns the red step. */
function buildFailed(ctx: Ctx, step: Step, log: string, text: string, why: string, t0: number): StepRun {
  writeFileSync(log, text);
  const secs = ((Date.now() - t0) / 1000).toFixed(1);
  ctx.out.line(`✗ ${step.name}: ${step.run.join(" ")} (${secs}s): ${why}. Full log: ${log}`);
  printTail(log, ctx.out);
  const ran = newStep(step, "red", why);
  ran.log = relOf(ctx, log);
  ctx.items.end(stepId(step.name), "red", why, ran.log);
  return ran;
}

/** Whether the store voided a green run because its files changed while it ran. A void run is red, and its verdict names the files. */
function voided(recorded: ReturnType<Store["record"]>, v: Verdict): boolean {
  if (recorded.kind !== "void") return false;
  v.green = false;
  v.why = `${recorded.changed.join(", ")} changed while it ran, so its result is void; run verify again`;
  return true;
}

/** Adds the red unit `id`, which no change reached, to the selection-miss list and says so. */
function miss(ctx: Ctx, id: string): void {
  ctx.store.recordMiss(id);
  ctx.out.line(`! ${id}: red, but no change reached it; added to the selection-miss list`);
}

/** A unit's result: green or why not, and its log. */
interface Verdict {
  green: boolean;
  why: string;
  log: string | null;
}

/** How a unit's run ended: judged, or killed because a request found these files changed under it. */
type Ran = { done: Verdict } | { killed: string[] };

/** Says a slot take's waiting line, and marks the item `item` waiting. */
function said(ctx: Ctx, item: string, line: string): void {
  ctx.out.line(line);
  ctx.items.waits(item);
}

/** Takes one heavy-job slot for `command`, unless this process runs under a holder. A wait goes to `onWait`. */
async function slot(root: string, env: Record<string, string | undefined>, command: string, onWait: (line: string) => void): Promise<Lease | null> {
  if ((env[SLOT_VAR] ?? "") !== "") return null;
  const asked = await takeSlots(looperHome(env), "one", { kind: "verify", project: root, command }, onWait);
  return "lease" in asked ? asked.lease : null;
}

/**
 * Runs `argv` from `cwd` as the unit whose item is `item`, logging to `log`. It is the unit in flight,
 * resting on `files`, until it ends. A request that finds those files changed kills its job tree, and
 * the item then ends `cancelled`. A `heavy` unit holds a heavy-job slot for its run.
 */
async function runUnit(ctx: Ctx, item: string, argv: string[], cwd: string, log: string, files: [string, Digest][], heavy: boolean): Promise<Ran> {
  const command = argv.join(" ");
  const lease = heavy ? await slot(ctx.root, ctx.env, command, (line) => said(ctx, item, line)) : null;
  try {
    const env = lease === null ? ctx.env : { ...ctx.env, [SLOT_VAR]: lease.id };
    ctx.watch.fly(files);
    let job: ReturnType<typeof startJob>;
    try {
      job = startJob(storeOf(ctx.root), { kind: "verify", argv, cwd, env, command, project: ctx.root, hide: ctx.cfg.checks.hide_windows });
    } catch (e) {
      ctx.watch.land();
      const why = `cannot start ${argv[0]}: ${(e as Error).message}`;
      mkdirSync(dirname(log), { recursive: true });
      writeFileSync(log, `${why}\n`);
      ctx.items.start(item, null);
      return { done: { green: false, why, log } };
    }
    await lease?.hold(job.id);
    ctx.items.start(item, job.id);
    let timedOut = false;
    let killed = false;
    const kill = () => {
      try {
        job.tree.kill(KILLED_CODE);
      } catch {}
    };
    const timer = setTimeout(() => {
      timedOut = true;
      kill();
    }, VERIFY_TIMEOUT_S * 1000);
    const poll = setInterval(() => {
      if (!killed && ctx.watch.killed()) {
        killed = true;
        kill();
      }
    }, KILL_POLL_MS);
    const code = await job.done.finally(() => {
      clearTimeout(timer);
      clearInterval(poll);
    });
    const changed = ctx.watch.land();
    copyLog(job.dir, log);
    if (killed) {
      ctx.items.end(item, "cancelled", `restarted for changes in ${named(changed)}`, null);
      return { killed: changed };
    }
    const why = timedOut ? `timed out after ${VERIFY_TIMEOUT_S}s` : code === 0 ? "" : `exited ${code}, want 0`;
    return { done: { green: why === "", why, log } };
  } finally {
    await lease?.release();
  }
}

/** Copies the log of the job in `dir` to `path`, one line at a time. */
function copyLog(dir: string, path: string): void {
  mkdirSync(dirname(path), { recursive: true });
  const fd = openSync(path, "w");
  try {
    readLines(dir, { all: true }, (line) => writeSync(fd, line));
  } finally {
    closeSync(fd);
  }
}

/** Prints the last `TAIL_LINES` lines of `log`, read from its last `TAIL_BYTES`. */
function printTail(log: string, out: Out): void {
  let text = "";
  try {
    const fd = openSync(log, "r");
    try {
      const size = fstatSync(fd).size;
      const n = Math.min(size, TAIL_BYTES);
      const buf = Buffer.alloc(n);
      readSync(fd, buf, 0, n, size - n);
      text = buf.toString("utf8");
    } finally {
      closeSync(fd);
    }
  } catch {
    return;
  }
  const lines = text.split(/\r?\n/);
  if (lines.at(-1) === "") lines.pop();
  for (const l of lines.slice(-TAIL_LINES)) out.line(l);
}

/** What a cargo command printed, kept within `Bounded`'s bound, and how it ended. */
interface Captured {
  /** Its exit code as text, with ` (timed out)` when its timeout killed it. */
  code: string;
  output: string;
}

/** Runs `argv` from `root` as a `verify` job for at most `checks.timeout`. With `onWait` it holds a heavy-job slot. */
async function capture(root: string, checks: Checks, env: Record<string, string | undefined>, argv: string[], onWait: ((line: string) => void) | null): Promise<Captured> {
  const command = argv.join(" ");
  const lease = onWait === null ? null : await slot(root, env, command, onWait);
  try {
    const kept = new Bounded();
    const onOutput = (chunk: Buffer) => kept.push(chunk);
    const jobEnv = lease === null ? env : { ...env, [SLOT_VAR]: lease.id };
    const job = startJob(storeOf(root), { kind: "verify", argv, cwd: root, env: jobEnv, command, project: root, onOutput, hide: checks.hide_windows });
    await lease?.hold(job.id);
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      try {
        job.tree.kill(KILLED_CODE);
      } catch {}
    }, checks.timeout * 1000);
    const code = await job.done.finally(() => clearTimeout(timer));
    return { code: `${code}${timedOut ? " (timed out)" : ""}`, output: kept.text() };
  } finally {
    await lease?.release();
  }
}

/** A cargo step's build: its log, and the build or why there is none. */
export interface Built {
  text: string;
  build: Build | string;
}

/**
 * Builds the binaries of `buildArgv`, a `cargo test` argv without its `--` part, without running them,
 * then reads `cargo metadata`, both from `root` with `env`, each for at most `checks.timeout`. With
 * `onWait` the build holds a heavy-job slot, and a wait for one is said to it. The log holds the build's
 * argv, its exit code, its compiler diagnostics and its other output, and a failed `cargo metadata` adds
 * its own argv, code and output.
 */
export async function build(root: string, checks: Checks, env: Record<string, string | undefined>, buildArgv: string[], onWait: ((line: string) => void) | null): Promise<Built> {
  const argv = [...buildArgv, "--no-run", "--message-format=json"];
  let compiled: Captured;
  try {
    compiled = await capture(root, checks, env, argv, onWait);
  } catch (e) {
    return { text: `$ ${argv.join(" ")}\n${(e as Error).message}\n`, build: "the build could not start" };
  }
  const lines = compiled.output.split(/\r?\n/);
  const other = lines.filter((l) => !l.startsWith("{")).join("\n");
  let text = `$ ${argv.join(" ")}\nexit ${compiled.code}\n${diagnostics(compiled.output).join("")}${other}`;
  if (compiled.code !== "0") return { text, build: "the build failed" };
  const metadataArgv = [buildArgv[0] as string, "metadata", "--format-version", "1"];
  const metadata = await capture(root, checks, env, metadataArgv, null);
  if (metadata.code !== "0") {
    text += `$ ${metadataArgv.join(" ")}\nexit ${metadata.code}\n${metadata.output}\n`;
    return { text, build: "`cargo metadata` failed" };
  }
  const json = metadata.output.split(/\r?\n/).find((l) => l.startsWith("{")) ?? "";
  try {
    return { text, build: Build.parse(compiled.output, json) };
  } catch (e) {
    return { text: `${text}${(e as Error).message}\n`, build: "`cargo metadata` printed what the kit cannot read" };
  }
}
