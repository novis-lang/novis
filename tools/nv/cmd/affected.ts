// `bun nv affected`: what a change reaches, and exactly what verifying it runs. Every agent, hook and
// driver that verifies anything asks this command, or runs it with `--run`, rather than choosing a
// command itself.
//
//     bun nv affected                   the change: every uncommitted path, and in a loop session every
//                                       commit since the session began
//     bun nv affected --since <rev>     the change: every path that differs between <rev> and the tree
//     bun nv affected --paths <p>...    the change: exactly these paths, against HEAD
//     bun nv affected --run             run the plan: `nv verify`, then the acceptance checks it names
//     bun nv affected --json            the plan as JSON, for a tool
//
// **The verify half** is the observed selection (`tools/nv/select/`), exactly as `nv verify` makes it:
// the change since the tree the store last recorded, and each step, test binary and case of the two
// case trees whose footprint holds a key the change moved, or that is new, red or owed. It is printed
// with the path each came from. The other atoms the change reaches (proof programs, other case trees,
// the `bun nv` checks) are counted: verify marks them owed, and whoever runs them next pays.
//
// **The acceptance half** is still keyed by prediction until the sweep moves onto the store. The tree
// before the change is the working tree with every changed path put back as the base commit holds it,
// and each acceptance check and leg of the live plan (`keys/checks.ts`) is keyed over both trees. A
// check whose key moves is reached. A reached check is not run when the sweep's memo holds its current
// key, or, for a `cargo test -p` check, when every binary it names is green in the store and not
// selected, and each test it names passed in that run. The plan is the reached checks that are not
// green, less the heavy ones: a check that builds or measures the release profile, fuzz, TSan, the
// database matrix, the two Linux legs and a check that is never memoized. Those are named as deferred,
// and the loop's floor gate runs them. An acceptance check that is not green and not reached is owed
// from before the change. It is named as a count and never run here: the floor gate or `bun nv loop
// --settle` pays it, and the pre-push hook refuses a push until one has.
//
// `--run` runs `nv verify` when it has anything to run, then plans again, so a `cargo test -p` check the
// verify run answered is remembered green in the sweep's memo rather than run a second time, then
// sweeps the rest.
//
// Exits 0 when nothing reached is due, or when `--run` ran it green; 1 when something reached is due, or
// `--run` found a red; 2 on a bad argument or a change git cannot name.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { type Check as AcceptCheck, GreenMemo, isCarried, isHeavy } from "../driver/accept.ts";
import { type Unit, LEGS, checkName, loadRecords, units } from "../keys/checks.ts";
import { type Graph, metadata } from "../keys/graph.ts";
import { type Part, keyOf } from "../keys/key.ts";
import { partitionOf } from "../keys/partition.ts";
import { digest } from "../keys/scan.ts";
import { Tree } from "../keys/tree.ts";
import { goalPlan, liveGoal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { passthrough, run as proc } from "../lib/proc.ts";
import { ENV as WRITES_ENV } from "../lib/written.ts";
import { describe } from "../select/select.ts";
import { verifiedByStore, verifyPlan } from "./verify.ts";

export const summary = "what a change reaches and exactly what verifying it runs: nv affected [--since <rev> | --paths <p>...] [--run | --json]";

const GREEN = ".loop/accept-green.json";
const SESSION_BASE = ".loop/session-start.json";
/** How many changed paths a unit's line names before it counts the rest. */
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
  /** The changed paths in the unit's key whose digest moved, sorted. */
  by: string[];
  /** Already green over the tree as it stands, so not run. */
  green: boolean;
}

export interface ReachedCheck extends Reached {
  id: string;
  carried: boolean;
  /** Heavy: the floor gate runs it, not this command. */
  deferred: boolean;
  /** Green because `nv verify`'s record answers it, and the sweep's memo does not hold it yet. */
  byVerify: boolean;
  /** Its key over the tree as it stands. */
  key: string | null;
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
  acceptance: { checks: ReachedCheck[]; legs: Reached[]; owed: number };
}

// ---- the change --------------------------------------------------------------------------------------

async function git(args: string[]): Promise<string | null> {
  const r = await proc(["git", ...args], { timeoutMs: 120_000 });
  return r.code === 0 ? r.stdout : null;
}

/** The loop's session base when this is a loop session and HEAD descends from it, else null. The driver
 * names its session's write ledger on the session's environment, and nothing else does. */
async function sessionBase(): Promise<string | null> {
  if (!process.env[WRITES_ENV]) return null;
  let base: unknown;
  try {
    base = JSON.parse(readFileSync(join(ROOT, SESSION_BASE), "utf8"))?.base;
  } catch {
    return null;
  }
  if (typeof base !== "string" || !/^[0-9a-f]{7,40}$/.test(base)) return null;
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

/** The tree before `change`: each changed path as `base` holds it, or deleted where `base` has none. A
 * path whose bytes are the same at `base` is dropped from the change. A directory given with `--paths`
 * stands for every file under it. */
export function before(tree: Tree, change: Change): { tree: Tree; paths: string[] } {
  const paths = [...new Set(change.paths.flatMap((p) => (tree.has(p) ? [p] : tree.under(p).length > 0 ? tree.under(p) : [p])))];
  const edits: Record<string, string | null> = {};
  for (const p of paths) {
    const got = Bun.spawnSync(["git", "show", `${change.base}:${p}`], { cwd: ROOT, stdout: "pipe", stderr: "ignore" });
    const bytes = got.exitCode === 0 ? got.stdout : null;
    if ((bytes === null ? "absent" : digest(bytes)) === tree.raw(p)) continue;
    edits[p] = bytes === null ? null : new TextDecoder().decode(bytes);
  }
  return { tree: tree.edited(edits), paths: Object.keys(edits).sort() };
}

// ---- the plan ----------------------------------------------------------------------------------------

/** The changed paths a moved key holds: each moved part that is a changed path, each changed path
 * under a moved part over a directory or a partition, and each one a moved `**\/<name>` part names. */
function movedBy(pre: Part[], after: Part[], changed: Set<string>): string[] {
  const was = new Map(pre.map((p) => [p.label, p.digest]));
  const now = new Map(after.map((p) => [p.label, p.digest]));
  const out = new Set<string>();
  for (const label of new Set([...was.keys(), ...now.keys()])) {
    if (was.get(label) === now.get(label)) continue;
    if (changed.has(label)) {
      out.add(label);
      continue;
    }
    if (label.startsWith("**/")) {
      for (const p of changed) if (p === label.slice(3) || p.endsWith(label.slice(2))) out.add(p);
      continue;
    }
    const dir = label.replace(/^<[a-z]+>/, "").replace(/\/$/, "");
    for (const p of changed) if (dir === "." || p === dir || p.startsWith(`${dir}/`) || label === `<${partitionOf(p)}>`) out.add(p);
  }
  if (out.size === 0) for (const p of changed) out.add(p);
  return [...out].sort();
}

/** Null when the key did not move, else the changed paths that moved it. */
function reach(name: string, pre: Part[], after: Part[], changed: Set<string>): string[] | null {
  return keyOf(name, pre) === keyOf(name, after) ? null : movedBy(pre, after, changed);
}

export async function plan(change: Change, graph: Graph | null): Promise<Plan> {
  const tree = await Tree.read();
  const { tree: pre, paths } = before(tree, change);
  const changed = new Set(paths);
  const v = await verifyPlan(graph);
  const why = (id: string): string => {
    const s = v.sel.selected.get(id);
    if (!s) return "--no-cache";
    const first = s.keys[0];
    return first ? describe(first.origin) : s.why === "new" ? "never recorded" : s.why;
  };
  const inVerify = new Set([...v.tests.map((n) => `test:${n}`), ...Object.values(v.cases).flatMap((cs) => cs.map((p) => `case:${p}`))]);
  if (v.steps.length > 0) for (const s of v.sel.selected.keys()) if (s.startsWith("step:")) inVerify.add(s);
  const others: Record<string, number> = {};
  for (const s of v.sel.selected.values()) if (!inVerify.has(s.id) && s.kind !== "step") others[s.kind] = (others[s.kind] ?? 0) + 1;
  const out: Plan = {
    change: { ...change, paths },
    verify: {
      since: v.change.full ? "" : v.change.since,
      paths: v.change.changes.length,
      steps: v.steps,
      tests: v.tests.map((n) => ({ id: `test:${n}`, why: why(`test:${n}`) })),
      cases: Object.values(v.cases).flatMap((cs) => cs.map((p) => ({ id: `case:${p}`, why: why(`case:${p}`) }))),
      others,
    },
    acceptance: { checks: [], legs: [], owed: 0 },
  };
  if (graph === null) return out;

  const all = units(loadRecords(graph));
  const memo = GreenMemo.load(join(ROOT, GREEN));
  const live = liveGoal();
  const goal = live === null ? null : goalPlan(live.slug);
  const labels = new Map((goal?.stages ?? []).map((s) => [s.number, `${s.number} ${s.title}`]));
  const label = (n: number) => labels.get(n) ?? String(n);
  const partsOf = (u: Unit, t: Tree): Part[] | null => {
    try {
      return u.parts(t);
    } catch {
      return null;
    }
  };
  /** Whether `nv verify`'s record answers a `cargo test -p` check over the tree as it stands. */
  const verified = await verifiedByStore(graph);

  for (const u of all) {
    if (u.role === "binary") continue;
    const now = partsOf(u, tree);
    const was = partsOf(u, pre);
    const by = now === null || was === null ? paths : reach(u.name, was, now, changed);
    const key = now === null ? null : keyOf(u.name, now);
    if (u.role === "leg" && LEGS.includes(u.name)) {
      if (by !== null) out.acceptance.legs.push({ name: u.name, by, green: memo.answers({ id: u.name } as AcceptCheck, key) });
      continue;
    }
    if (u.check === undefined || typeof u.check.id !== "string") continue;
    const c = u.check as unknown as AcceptCheck;
    const inMemo = memo.answers(c, key);
    const byVerify = !inMemo && c.memoize !== false && verified(c);
    const carried = isCarried(label(c.stage));
    if (by !== null) out.acceptance.checks.push({ id: c.id, name: checkName(u.check), by, green: inMemo || byVerify, carried, deferred: isHeavy(c), byVerify, key });
    else if (!inMemo && !byVerify && carried && c.memoize !== false) out.acceptance.owed++;
  }
  tree.save();
  return out;
}

/** The reached checks the sweep runs: not green and not heavy. */
function due(p: Plan): ReachedCheck[] {
  return p.acceptance.checks.filter((x) => !x.green && !x.deferred);
}

/** Whether anything the change reaches is due. */
function owes(p: Plan): boolean {
  return p.verify.steps.length > 0 || due(p).length > 0;
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
  const cases = (tree: string) => v.cases.filter((c) => c.id.startsWith(`case:tests/${tree}/`));
  for (const s of v.steps) {
    const n = s === "test" ? v.tests.length : cases(s).length;
    const what = s === "test" ? ` ${n} binar${n === 1 ? "y" : "ies"}` : s === "conformance" || s === "differential" ? ` ${n} case(s)` : "";
    console.log(`  ${s.padEnd(12)}${what}`);
    const atoms = s === "test" ? v.tests : cases(s);
    for (const a of atoms.slice(0, 8)) console.log(`      ${a.id.slice(a.id.indexOf(":") + 1)}  <- ${a.why}`);
    if (atoms.length > 8) console.log(`      +${atoms.length - 8} more`);
  }
  const others = Object.entries(v.others);
  if (others.length > 0) console.log(`  (the change also reaches ${others.map(([k, n]) => `${n} ${k}`).join(", ")} that verify does not run; they stay owed until a run of each is green)`);

  const run = due(p);
  const deferred = p.acceptance.checks.filter((x) => !x.green && x.deferred).length + p.acceptance.legs.filter((l) => !l.green).length;
  console.log(`\nacceptance -- \`bun nv affected --run\` runs ${run.length} check(s):`);
  for (const x of run.slice(0, 20)) console.log(`  ${x.carried ? "floor" : "goal "} ${x.name}${from(x.by)}`);
  if (run.length > 20) console.log(`  +${run.length - 20} more`);
  const green = p.acceptance.checks.filter((x) => x.green);
  if (green.length > 0) console.log(`  (${green.length} reached check(s) are green over this tree already, ${green.filter((x) => x.byVerify).length} of them by \`nv verify\`'s test record)`);
  if (deferred > 0) console.log(`  ${deferred} reached heavy check(s) and leg(s) wait for the loop's floor gate: release profile, fuzz, TSan, the database matrix, the Linux legs`);
  if (p.acceptance.owed > 0) console.log(`  ${p.acceptance.owed} carried check(s) are owed from before this change; the floor gate or \`bun nv loop --settle\` at a push pays them`);

  if (owes(p)) console.log(`\nrun it: \`bun nv affected --run${c.why === "uncommitted changes" || c.why === "the loop's session base" ? "" : ` ${c.why}`}\``);
}

// ---- the command -------------------------------------------------------------------------------------

const USAGE = "usage: nv affected [--since <rev> | --paths <path>...] [--run | --json]";

/** Keeps each reached check `nv verify`'s record answers in the sweep's memo, so no sweep runs it again. */
function rememberVerified(p: Plan): void {
  const memo = GreenMemo.load(join(ROOT, GREEN));
  let kept = 0;
  for (const x of p.acceptance.checks) {
    if (!x.byVerify || x.key === null) continue;
    memo.remember({ id: x.id } as AcceptCheck, x.key);
    kept++;
  }
  if (kept === 0) return;
  const goal = liveGoal();
  const ids = new Set([...((goal === null ? [] : (goalPlan(goal.slug)?.checks ?? [])) as { id: string }[]).map((c) => c.id), ...LEGS]);
  memo.save(join(ROOT, GREEN), ids);
}

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
  if (graph === null) console.error("nv affected: `cargo metadata` failed, so no test binary or acceptance check is keyed");
  const p = await plan(change, graph);
  if (json) {
    console.log(JSON.stringify(p, null, 1));
    return owes(p) ? 1 : 0;
  }
  print(p, paths);
  if (!doRun) return owes(p) ? 1 : 0;

  const verifyDue = p.verify.steps.length > 0;
  let after = p;
  if (verifyDue) {
    console.log("");
    if ((await passthrough(["bun", "nv", "verify"], { timeoutMs: 3 * 60 * 60 * 1000 })) !== 0) return 1;
    // The verify run answered test binaries, and may have formatted files: plan again over the tree as it is.
    after = await plan(change, graph);
  }
  rememberVerified(after);
  const ids = new Set(due(after).map((x) => x.id));
  console.log("");
  if (ids.size === 0) {
    console.log("affected: GREEN -- nothing the change reaches is due");
    return 0;
  }
  const { sweepIds } = await import("./loop.ts");
  const code = await sweepIds(ids);
  return code === 0 ? 0 : code === 2 ? 2 : 1;
}
