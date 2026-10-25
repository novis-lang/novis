// `bun nv affected`: what a change reaches, and exactly what verifying it runs. Every agent, hook and
// driver that verifies anything asks this command, or runs it with `--run`, rather than choosing a
// command itself.
//
//     bun nv affected                   the change: every uncommitted path, and every commit since
//                                       the session began when looper names its base
//     bun nv affected --since <rev>     the change: every path that differs between <rev> and the tree
//     bun nv affected --paths <p>...    the change: exactly these paths, against HEAD
//     bun nv affected --run             run the plan: `nv verify`, then the acceptance checks it names
//     bun nv affected --json            the plan as JSON, for a tool
//
// Both halves are the observed selection (`tools/nv/select/`), read over the change since the tree the
// store last recorded. The change the flags name is printed as the change a person asked about; what
// runs is what the store says differs from what it last recorded.
//
// **The verify half** is exactly what `nv verify` would run: each step, test binary and case of the two
// case trees whose footprint holds a key the change moved, or that is new, red or owed, printed with the
// path each came from. The other atoms the change reaches (proof programs, other case trees, the plan's
// own checks) are counted: verify marks them owed, and whoever runs them next pays.
//
// **The bench half** is every bench under `benches/members/` the change itself reaches: the change moved
// a key in its footprint or changed its own files, or a file every atom depends on (`reachedByChange` in
// `select/select.ts`). `--run` judges each one's growth on the release build and records its footprint
// after verify (`runBenches` in `proofs/select.ts`). A bench selected only for being owed, red or never
// recorded from before the change is counted, not judged: it stays as it is, and `bun nv benches
// --owed` pays it before a release, since the release gate's `proofs` job is red on a stale perf record.
//
// **The acceptance half** is the plan's checks the change reaches (`select/checks.ts`): the checks
// of the live plan, or of the side goal `NOVIS_SIDE_GOAL` names. A check reached through a key the change
// moved, or through its own record changing, is due, less the heavy ones: a check that builds or
// measures the release profile, fuzz, TSan, the database matrix and a check that is never memoized.
// Those are named as deferred: looper's sweep after the session runs the goal's checks. A check whose atoms
// were all picked only for being owed, red or never recorded before the change, none of them reached by
// the change itself, was red before the change and stays so until the goal is reached, so it is not due.
//
// `--run` runs `nv verify` when it has anything to run, then the benches the change reached, then sweeps the checks that were due. A test
// binary or case verify ran green is not picked again, so a check made of those is not started twice.
//
// Exits 0 when nothing reached is due, or when `--run` ran it green; 1 when something reached is due, or
// `--run` found a red; 2 on a bad argument or a change git cannot name.

import { type Check, isHeavy } from "../accept/accept.ts";
import { PlanSweep } from "../accept/runner.ts";
import { type Graph, metadata } from "../keys/graph.ts";
import { currentPlan } from "../lib/chain.ts";
import { passthrough, run as proc } from "../lib/proc.ts";
import { describe, reachedByChange, type Selected, touchesDef } from "../select/select.ts";
import { verifyPlan } from "./verify.ts";

export const summary = "what a change reaches and exactly what verifying it runs: nv affected [--since <rev> | --paths <p>...] [--run | --json]";

/** The commit looper's session started from, which looper sets for a step it runs in a loop session. */
const SESSION_BASE = "LOOPER_SESSION_BASE";
/** How many reasons a check's line names before it counts the rest. */
const SHOWN = 3;

export interface Change {
  /** What the change is measured from: a commit, or `HEAD`. */
  base: string;
  /** Why that base: the flag, or the default that chose it. */
  why: string;
  paths: string[];
}

export interface Reached {
  name: string;
  /** Why the change reaches it: the key and the path each picked atom came from, sorted. */
  by: string[];
}

export interface ReachedCheck extends Reached {
  id: string;
  /** Heavy: the loop runs it after the session, not this command. */
  deferred: boolean;
  /** Reached only through atoms owed or red from before the change. */
  before: boolean;
}

/** One atom `nv verify` runs, and where the key that selected it came from. */
export interface VerifyAtom {
  id: string;
  why: string;
}

export interface Plan {
  change: Change;
  verify: {
    /** The tree the store recorded, and how many paths differ from it. */
    since: string;
    paths: number;
    /** The steps verify starts, in order. */
    steps: string[];
    tests: VerifyAtom[];
    cases: VerifyAtom[];
    /** Atoms the change reaches that verify does not run, by kind: they stay owed. */
    others: Record<string, number>;
  };
  /** The benches the change itself reaches, which `--run` judges and records after verify. */
  benches: VerifyAtom[];
  /** How many benches are selected only for being owed, red or never recorded from before the change:
   * `bun nv benches --owed` pays them before a release. */
  benchesBefore: number;
  acceptance: { checks: ReachedCheck[] };
}

// ---- the change --------------------------------------------------------------------------------------

async function git(args: string[]): Promise<string | null> {
  const r = await proc(["git", ...args], { timeoutMs: 120_000 });
  return r.code === 0 ? r.stdout : null;
}

/** The loop's session base when `SESSION_BASE` names one and HEAD descends from it, else null. */
async function sessionBase(): Promise<string | null> {
  const base = (process.env[SESSION_BASE] ?? "").trim();
  if (!/^[0-9a-f]{7,40}$/.test(base)) return null;
  const r = await proc(["git", "merge-base", "--is-ancestor", base, "HEAD"], { timeoutMs: 60_000 });
  return r.code === 0 ? base : null;
}

/** Every path that differs between `base` and the working tree, untracked ones included. */
async function changedSince(base: string): Promise<string[] | null> {
  const [diff, others] = await Promise.all([git(["diff", "--name-only", "-z", "--no-renames", base, "--"]), git(["ls-files", "-z", "--others", "--exclude-standard"])]);
  if (diff === null || others === null) return null;
  return [...new Set([...diff.split("\0"), ...others.split("\0")].filter((p) => p !== ""))].sort();
}

/** The change the flags name, or the reason there is none. */
export async function changeOf(o: { since?: string; paths?: string[] }): Promise<Change | string> {
  if (o.paths !== undefined) {
    const paths = [...new Set(o.paths.map((p) => p.replace(/\\/g, "/").replace(/^\.\//, "").replace(/\/+$/, "")))].sort();
    return { base: "HEAD", why: "--paths", paths };
  }
  let base = o.since;
  let why = `--since ${o.since}`;
  if (base === undefined) {
    const session = await sessionBase();
    base = session ?? "HEAD";
    why = session ? "the loop's session base" : "uncommitted changes";
  }
  if ((await git(["rev-parse", "--verify", "--quiet", `${base}^{commit}`])) === null) return `\`${base}\` names no commit`;
  const paths = await changedSince(base);
  if (paths === null) return `git could not list what changed since ${base}`;
  return { base, why, paths };
}

// ---- the plan ----------------------------------------------------------------------------------------

/** Why the store picked an atom, for a person: its first key and where that came from, or its reason. */
export function reason(s: Selected): string {
  const first = s.keys[0];
  if (first) return describe(first.origin);
  return s.why === "new" ? "never recorded" : s.why === "def" ? "its definition changed" : s.why;
}

/** The reasons a reached check's picked atoms give, each once, and whether all of them are from before
 * the change: picked only for being owed, red, diverged or never recorded, none of them reached by the
 * change itself (`reachedByChange`, with `touched` from `touchesDef`). */
export function reasons(atoms: string[], picked: Map<string, Selected>, touched: (id: string) => boolean = () => false): { by: string[]; before: boolean } {
  const by = new Set<string>();
  let before = true;
  for (const a of atoms) {
    const s = picked.get(a);
    if (!s) continue;
    by.add(reason(s));
    if (reachedByChange(s, touched)) before = false;
  }
  return { by: [...by].sort(), before: before && by.size > 0 };
}

/** The selected benches split in two: the ones the change itself reaches, which `--run` judges, and how
 * many are selected only for being owed, red or never recorded from before the change, which `bun nv
 * debt` pays as floor debt. */
export function splitBenches(picked: Iterable<Selected>, touched: (id: string) => boolean): { judged: VerifyAtom[]; before: number } {
  const judged: VerifyAtom[] = [];
  let before = 0;
  for (const s of picked) {
    if (s.kind !== "bench") continue;
    if (reachedByChange(s, touched)) judged.push({ id: s.id, why: reason(s) });
    else before++;
  }
  judged.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  return { judged, before };
}

export async function plan(change: Change, graph: Graph | null): Promise<Plan> {
  const v = await verifyPlan(graph);
  const why = (id: string): string => {
    const s = v.sel.selected.get(id);
    return s ? reason(s) : "--no-cache";
  };
  const inVerify = new Set([...v.tests.map((n) => `test:${n}`), ...Object.values(v.cases).flatMap((cs) => cs.map((p) => `case:${p}`))]);
  if (v.steps.length > 0) for (const s of v.sel.selected.keys()) if (s.startsWith("step:")) inVerify.add(s);
  const others: Record<string, number> = {};
  for (const s of v.sel.selected.values()) if (!inVerify.has(s.id) && s.kind !== "step" && s.kind !== "bench") others[s.kind] = (others[s.kind] ?? 0) + 1;
  const benches = splitBenches(v.sel.selected.values(), touchesDef(v.change));
  const out: Plan = {
    change,
    verify: {
      since: v.change.full ? "" : v.change.since,
      paths: v.change.changes.length,
      steps: v.steps,
      tests: v.tests.map((n) => ({ id: `test:${n}`, why: why(`test:${n}`) })),
      cases: Object.values(v.cases).flatMap((cs) => cs.map((p) => ({ id: `case:${p}`, why: why(`case:${p}`) }))),
      others,
    },
    benches: benches.judged,
    benchesBefore: benches.before,
    acceptance: { checks: [] },
  };
  const found = currentPlan();
  if (found === null) return out;
  const checks = found.goal.checks as Check[];
  const labels = new Map(found.goal.stages.map((s) => [s.number, `${s.number} ${s.title}`]));
  const label = (n: number) => labels.get(n) ?? String(n);
  const sweep = await PlanSweep.open(checks, label, { full: false });
  try {
    const touched = touchesDef(sweep.sel.change);
    for (const c of checks) {
      if (!sweep.reached(c)) continue;
      const g = sweep.groups.get(c.id);
      const { by, before } = reasons(g?.atoms ?? [], sweep.sel.selected, touched);
      // A check that was red before the change stays red until the goal is reached.
      if (before) continue;
      out.acceptance.checks.push({ id: c.id, name: c.name ?? c.file ?? c.id, by: by.length > 0 ? by : [c.memoize === false ? "never memoized" : "what it names beyond its atoms is not in the store"], deferred: isHeavy(c), before });
    }
  } finally {
    sweep.discard();
  }
  return out;
}

/** The reached checks the sweep runs: not heavy. */
function due(p: Plan): ReachedCheck[] {
  return p.acceptance.checks.filter((x) => !x.deferred);
}

/** Whether anything the change reaches is due. */
function owes(p: Plan): boolean {
  return p.verify.steps.length > 0 || p.benches.length > 0 || due(p).length > 0;
}

// ---- output ------------------------------------------------------------------------------------------

function from(by: string[]): string {
  if (by.length === 0) return "";
  const more = by.length > SHOWN ? `, +${by.length - SHOWN} more` : "";
  return `  <- ${by.slice(0, SHOWN).join(", ")}${more}`;
}

function print(p: Plan, asked: string[] | undefined): void {
  const c = p.change;
  console.log(`affected: ${c.paths.length} changed path(s) against ${c.base === "HEAD" ? "HEAD" : c.base.slice(0, 12)} (${c.why})`);
  for (const path of c.paths.slice(0, 8)) console.log(`  ${path}`);
  if (c.paths.length > 8) console.log(`  +${c.paths.length - 8} more`);
  const same = (asked ?? []).filter((a) => !c.paths.some((q) => q === a || q.startsWith(`${a}/`)));
  if (same.length > 0) console.log(`  ${same.join(", ")}: the same as HEAD, so no change. Name a committed change with \`--since <rev>\`, such as \`--since HEAD~1\`.`);

  const v = p.verify;
  const since = v.since ? `the ${v.paths} path(s) changed since the store's recorded tree (${v.since.slice(0, 12)})` : "a store with no recorded tree, so everything";
  console.log(`\nverify -- \`bun nv verify\` runs, from ${since}:`);
  if (v.steps.length === 0) console.log("  nothing: the change reaches nothing a step reads");
  const cases = (tree: string) => v.cases.filter((x) => x.id.startsWith(`case:tests/${tree}/`));
  for (const s of v.steps) {
    const n = s === "test" ? v.tests.length : cases(s).length;
    const what = s === "test" ? ` ${n} binar${n === 1 ? "y" : "ies"}` : s === "conformance" ?` ${n} case(s)` : "";
    console.log(`  ${s.padEnd(12)}${what}`);
    const atoms = s === "test" ? v.tests : cases(s);
    for (const a of atoms.slice(0, 8)) console.log(`      ${a.id.slice(a.id.indexOf(":") + 1)}  <- ${a.why}`);
    if (atoms.length > 8) console.log(`      +${atoms.length - 8} more`);
  }
  const others = Object.entries(v.others);
  if (others.length > 0) console.log(`  (the change also reaches ${others.map(([k, n]) => `${n} ${k}`).join(", ")} that verify does not run; they stay owed until a run of each is green)`);

  console.log(`\nbenches -- \`bun nv affected --run\` judges the growth of ${p.benches.length} bench(es) and records each:`);
  for (const b of p.benches.slice(0, 8)) console.log(`  ${b.id.slice(b.id.indexOf(":") + 1)}  <- ${b.why}`);
  if (p.benches.length > 8) console.log(`  +${p.benches.length - 8} more`);
  if (p.benchesBefore > 0) console.log(`  ${p.benchesBefore} bench(es) owed, red or never recorded from before the change wait for a release: \`bun nv benches --owed\` pays them there`);

  const run = due(p);
  const deferred = p.acceptance.checks.filter((x) => x.deferred).length;
  console.log(`\nacceptance -- \`bun nv affected --run\` runs ${run.length} check(s):`);
  for (const x of run.slice(0, 20)) console.log(`  ${x.name}${from(x.by)}`);
  if (run.length > 20) console.log(`  +${run.length - 20} more`);
  if (deferred > 0) console.log(`  ${deferred} reached heavy check(s) wait for the loop: looper's sweep runs the release profile, fuzz, TSan and the database matrix`);


  if (owes(p)) console.log(`\nrun it: \`bun nv affected --run${c.why === "uncommitted changes" || c.why === "the loop's session base" ? "" : ` ${c.why}`}\``);
}

// ---- the command -------------------------------------------------------------------------------------

const USAGE = "usage: nv affected [--since <rev> | --paths <path>...] [--run | --json]";

export async function run(args: string[]): Promise<number> {
  let since: string | undefined;
  let paths: string[] | undefined;
  let doRun = false;
  let json = false;
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--run") doRun = true;
    else if (a === "--json") json = true;
    else if (a === "--since" && args[i + 1] !== undefined) since = args[++i];
    else if (a === "--paths") {
      paths = [];
      while (args[i + 1] !== undefined && !args[i + 1]!.startsWith("--")) paths.push(args[++i]!);
      if (paths.length === 0) {
        console.error(`nv affected: --paths needs at least one path\n${USAGE}`);
        return 2;
      }
    } else if (a === "-h" || a === "--help") {
      console.log(USAGE);
      return 0;
    } else {
      console.error(`nv affected: unknown argument ${a}\n${USAGE}`);
      return 2;
    }
  }
  if ((since !== undefined && paths !== undefined) || (doRun && json)) {
    console.error(`nv affected: --since and --paths are two ways to name one change, and --run prints no JSON\n${USAGE}`);
    return 2;
  }

  const flags = { ...(since !== undefined ? { since } : {}), ...(paths !== undefined ? { paths } : {}) };
  const change = await changeOf(flags);
  if (typeof change === "string") {
    console.error(`nv affected: ${change}`);
    return 2;
  }
  const graph = await metadata();
  if (graph === null) console.error("nv affected: `cargo metadata` failed, so no test binary is named");
  const p = await plan(change, graph);
  if (json) {
    console.log(JSON.stringify(p, null, 1));
    return owes(p) ? 1 : 0;
  }
  print(p, paths);
  if (!doRun) return owes(p) ? 1 : 0;

  // What was due is chosen before verify runs: verify moves the store's tree and marks what it did not run
  // owed, so a plan read after it would take the change's own checks and benches for debts from before.
  const ids = new Set(due(p).map((x) => x.id));
  const benches = new Set(p.benches.map((b) => b.id));
  if (p.verify.steps.length > 0) {
    console.log("");
    if ((await passthrough(["bun", "nv", "verify"], { timeoutMs: 3 * 60 * 60 * 1000 })) !== 0) return 1;
  }
  if (benches.size > 0) {
    console.log("");
    const { runBenches } = await import("../proofs/select.ts");
    const got = await runBenches((line) => console.log(`  ${line}`), benches);
    if (typeof got === "string") {
      console.error(`affected: ${got}`);
      return 1;
    }
    for (const r of got.red) console.log(`affected: RED bench ${r.path}\n${r.findings.map((f) => `    ${f}`).join("\n")}`);
    for (const [path, by] of got.skipped) console.log(`affected: skipped bench ${path}, proven by ${by.join(", ")}`);
    console.log(`affected: the change reached ${got.reached} bench(es): ${got.ran} judged and recorded, ${got.skipped.size} skipped, ${got.red.length} red`);
    if (got.red.length > 0) return 1;
  }
  console.log("");
  if (ids.size === 0) {
    console.log("affected: GREEN -- nothing the change reaches is due");
    return 0;
  }
  const { sweepIds } = await import("../accept/sweep.ts");
  const code = await sweepIds(ids);
  return code === 0 ? 0 : code === 2 ? 2 : 1;
}
