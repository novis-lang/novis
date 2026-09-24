// `bun nv loop --list`: the live goal's acceptance plan, read from `docs/agent/loop-goal.toml` while the
// Python driver runs it and from the goal's record under `data/goals/` once that file is gone. One
// line per check, `[<stage number> <stage title>]`, its kind, its name and what it runs, then
// `list: N check(s) match` last. The live goal is the one `docs/agent/loop-goal.md` is a copy of, and its
// record carries the floor as its own first stage, so the plan printed is the whole plan the driver runs.
//
// Three filters narrow it, and a check is printed when it matches every one given:
//   - `--stage <label>` — the stage's number, or the start of its label `8 the driver` or of its title;
//   - `--name <text>` — text inside the check's name, case-insensitive;
//   - `--feature <id>` — a proofs check over that feature: its name is `proofs: <id>` or `dossier: <id>`,
//     with or without a `(1/3)` part, or its `argv` passes `<id>` to `--group` or `--id`.
// A check with no name of its own is named by its `file`, so `--name` reaches a fixture's check too.
//
// The driver itself is not written yet, so `--list` is the only form this command takes.
//
// Exits 0 when the plan is read, whether or not a check matches, and 2 on a bad argument or when there
// is no live goal to read.

import { existsSync } from "node:fs";
import { join } from "node:path";
import { chainGoals, liveGoal } from "../lib/chain.ts";
import { installedGoal } from "../import/goals.ts";
import type { Unread } from "../import/lib.ts";
import { ROOT } from "../lib/paths.ts";
import type { RecordType } from "../lib/schema.ts";
import { loadFile } from "../lib/store.ts";
import { goal as goalType } from "../schema/goal.ts";

export const summary = "the live goal's acceptance plan: nv loop --list [--stage <label>] [--name <text>] [--feature <id>]";

/** The Python driver's acceptance list, the plan until the cutover deletes it. */
const LEGACY = "docs/agent/loop-goal.toml";

type Goal = typeof goalType extends RecordType<infer T> ? T : never;

const USAGE ="bun nv loop --list [--stage <label>] [--name <text>] [--feature <id>]";

interface Check {
  id: string;
  kind: string;
  stage: number;
  name?: string;
  argv?: string[];
  args?: string[];
  cwd?: string;
  cases?: string[];
  file?: string;
  setup?: boolean;
  memoize?: boolean;
  overlap?: boolean;
}

interface Filters {
  stage?: string;
  name?: string;
  feature?: string;
}

function parse(args: string[]): Filters | null {
  if (args[0] !== "--list") return null;
  const out: Filters = {};
  for (let i = 1; i < args.length; i += 2) {
    const flag = args[i]!;
    const value = args[i + 1];
    if (value === undefined) return null;
    if (flag === "--stage") out.stage = value;
    else if (flag === "--name") out.name = value;
    else if (flag === "--feature") out.feature = value;
    else return null;
  }
  return out;
}

/** The check's name as the driver prints it: its own, or its fixture's run line. */
function nameOf(c: Check): string {
  return c.name ?? `run ${[...(c.args ?? []), c.file ?? ""].join(" ")}`.trimEnd();
}

function stageMatches(label: string, number: number, want: string): boolean {
  // A bare number names that stage alone, so `1` never reaches stages 10 to 12 through the prefix.
  const w = want.trim().toLowerCase();
  if (/^\d+$/.test(w)) return w === String(number);
  const l = label.toLowerCase();
  return l.startsWith(w) || l.slice(String(number).length + 1).startsWith(w);
}

function featureMatches(c: Check, id: string): boolean {
  const named = /^(?:proofs|dossier): (.+?)(?: \(\d+\/\d+\))?$/.exec(c.name ?? "");
  if (named?.[1] === id) return true;
  const argv = c.argv ?? [];
  return argv.some((a, i) => (a === "--group" || a === "--id") && argv[i + 1] === id);
}

/** One check as `loop.py --list` printed it: what a session reads to decide whether a `want` is producible. */
function line(c: Check, label: string): string {
  const head = `  [${label}] ${c.kind.padEnd(11)}`;
  if (c.file !== undefined) {
    // A check's own `args` go ahead of the fixture, which is the order the driver runs them in.
    return `${head} nvs ${["run", ...(c.args ?? []), c.file].join(" ")}`;
  }
  if (c.kind === "command") {
    const note = c.setup ? "  (setup: runs before the fixtures)" : c.overlap ? "  (overlap: runs beside the sweep)" : c.memoize ? "  (memoized against the tree)" : "";
    return `${head} ${nameOf(c)}: ${(c.argv ?? []).join(" ")} in ${c.cwd ?? "."}${note}`;
  }
  const driver = c.kind === "nvs-suite" ? "nvs" : "cargo";
  return `${head} ${nameOf(c)}: ${driver} ${(c.args ?? []).join(" ")}`;
}

/**
 * The live goal's plan, or why it cannot be read. While the Python driver runs, `docs/agent/loop-goal.toml`
 * is the list it runs and the sessions edit, so the plan is imported from it the way `nv import` would;
 * once that file is gone, the record under `data/goals/` is the plan.
 */
function planOf(slug: string, md: string): Goal | string {
  if (existsSync(join(ROOT, LEGACY))) {
    const unread: Unread[] = [];
    const value = installedGoal(ROOT, slug, md, unread);
    const issues = value === null ? [] : goalType.schema.validate(value);
    if (value === null || unread.length > 0 || issues.length > 0) {
      const why = unread[0] ? `${unread[0].path}: ${unread[0].reason}` : `it fails the goal schema: ${JSON.stringify(issues[0])}`;
      return `${LEGACY} does not import: ${why}`;
    }
    return value as Goal;
  }
  const path = `data/goals/${slug}.json`;
  const rec = loadFile(goalType, path);
  return rec.issues.length > 0 ? `${path} fails its schema; \`bun nv check\` names how` : rec.value;
}

function list(filters: Filters): number {
  const goals = chainGoals();
  const live = liveGoal(goals);
  if (live === null) {
    console.error("nv loop: docs/agent/loop-goal.md is a copy of no goal on the chain, so there is no plan to list");
    return 2;
  }
  const goal = planOf(live.slug, live.md!);
  if (typeof goal === "string") {
    console.error(`nv loop: ${goal}`);
    return 2;
  }
  const path = existsSync(join(ROOT, LEGACY)) ? LEGACY : `data/goals/${live.slug}.json`;
  const labels = new Map(goal.stages.map((s) => [s.number, `${s.number} ${s.title}`]));
  const labelOf = (n: number) => labels.get(n) ?? String(n);
  if (filters.stage !== undefined && !goal.stages.some((s) => stageMatches(labelOf(s.number), s.number, filters.stage!))) {
    console.error(`nv loop: no stage of \`${live.slug}\` is \`${filters.stage}\`; its stages are ${[...labels.values()].map((l) => `\`${l}\``).join(", ")}`);
    return 2;
  }

  const checks = goal.checks as Check[];
  const shown = checks.filter(
    (c) =>
      (filters.stage === undefined || stageMatches(labelOf(c.stage), c.stage, filters.stage)) &&
      (filters.name === undefined || nameOf(c).toLowerCase().includes(filters.name.toLowerCase())) &&
      (filters.feature === undefined || featureMatches(c, filters.feature)),
  );

  console.log(`${path}: ${checks.length} checks, ${goal.files.length} fixtures, goal \`${live.slug}\` (${live.num} of ${goals.length})`);
  for (const c of shown) {
    console.log(line(c, labelOf(c.stage)));
    // Named cases are the half of a suite check a session acts on: the ones not yet on disk are the worklist.
    const cases = c.cases ?? [];
    if (cases.length > 0) {
      const absent = cases.filter((p) => !existsSync(join(ROOT, p)));
      console.log(`      ${cases.length} named case(s), ${absent.length} not written yet`);
      for (const p of absent) console.log(`        - ${p}`);
    }
  }
  const narrowed = filters.stage !== undefined || filters.name !== undefined || filters.feature !== undefined;
  if (!narrowed) console.log(`  valgrind sweep over every fixture except: ${[...(goal.env.valgrind?.skip ?? [])].sort().join(", ") || "nothing"}`);
  console.log(`list: ${shown.length} ${shown.length === 1 ? "check matches" : "checks match"}`);
  return 0;
}

export async function run(args: string[]): Promise<number> {
  const filters = parse(args);
  if (filters === null) {
    console.error(`usage: ${USAGE}`);
    return 2;
  }
  return list(filters);
}
