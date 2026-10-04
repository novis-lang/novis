// Where the chain stands: its order, the live goal, the goals in front of it, and the plan the driver
// runs for a goal.
//
// All of it is `data/chain.json`: `goals` is the order and `live` the goal the driver works on. The goal
// switch deletes the goal it leaves (`leaveGoal`), so a goal in front of the live one is a goal walked
// before that rule, and `walkedGoals` names them.
//
// **A goal's plan is its record and nothing else** (`rule:tooling/the-chain-names-its-live-goal`). No
// check of another goal is carried into it: the permanent suites and `bun nv verify` protect finished
// work. A side goal's plan is its own record as well, with a WSL target of its own. A process belongs to
// a side run when `SIDE_ENV` names a side goal whose prose is on disk.

import { existsSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "./paths.ts";
import type { RecordType } from "./schema.ts";
import { load, write } from "./store.ts";
import { chain as chainType } from "../schema/chain.ts";
import { goal as goalType, sideGoal as sideGoalType } from "../schema/goal.ts";

const GOALS = "docs/agent/goals";

/** Set to a side goal's slug in every process of a side run: the driver, its turns and their sessions. */
export const SIDE_ENV = "NOVIS_SIDE_GOAL";

/** A slug as a goal file names it. */
const SLUG = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

/** A goal's record, as `data/goals/<slug>.json` holds it. */
export type Goal = typeof goalType extends RecordType<infer T> ? T : never;
type GoalCheck = Goal["checks"][number];

/** A goal file's front matter, which sits above its H1. */
const FRONT = /^---\n[\s\S]*?\n---\n/;

export interface ChainGoal {
  slug: string;
  /** 1-based, its place in `data/chain.json`. */
  num: number;
  milestone: string | null;
  /** The goal's prose, repo-relative, or null when none is on disk. */
  md: string | null;
}

/** A goal file's H1: its first line once the front matter and the blank lines after it are gone. */
export function h1Of(text: string): string {
  return text.replace(/\r\n/g, "\n").replace(FRONT, "").replace(/^\n+/, "").split("\n", 1)[0]!.trim();
}

/** slug -> its prose file, `<slug>.md` in the goals directory. */
function proseFiles(root: string): Map<string, string> {
  const out = new Map<string, string>();
  const dir = join(root, GOALS);
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    const m = /^([a-z0-9]+(?:-[a-z0-9]+)*)\.md$/.exec(name);
    if (m) out.set(m[1]!, `${GOALS}/${name}`);
  }
  return out;
}

/** `data/chain.json`, or an empty chain when it is absent. */
function chainRecord(root: string): { id: string; live: string | null; goals: string[] } {
  const got = load(chainType, root)[0];
  return { id: got?.id ?? "chain", live: got?.value.live ?? null, goals: got?.value.goals ?? [] };
}

/** Every goal on the chain, in chain order. */
export function chainGoals(root: string = ROOT): ChainGoal[] {
  const order = chainRecord(root).goals;
  const records = new Map(load(goalType, root).map((g) => [g.id, g.value]));
  const prose = proseFiles(root);
  return order.map((slug, i) => ({
    slug,
    num: i + 1,
    milestone: records.get(slug)?.milestone ?? null,
    md: prose.get(slug) ?? null,
  }));
}

/** The goal `data/chain.json` names live, or null when it names none on the chain. */
export function liveGoal(goals?: ChainGoal[], root: string = ROOT): ChainGoal | null {
  goals ??= chainGoals(root);
  const slug = chainRecord(root).live;
  return goals.find((g) => g.slug === slug) ?? null;
}

/** Makes `slug` the live goal. The caller commits `data/chain.json`, which is the whole of a goal switch on disk. */
export function setLive(slug: string, root: string = ROOT): string {
  const c = chainRecord(root);
  if (!c.goals.includes(slug)) throw new Error(`${slug} is not on the chain`);
  return write(chainType, c.id, { live: slug, goals: c.goals }, root).path;
}

/** A goal's three files, repo-relative: its prose, its record and its handoff record. */
export function goalFiles(slug: string): string[] {
  return [`${GOALS}/${slug}.md`, `data/goals/${slug}.json`, `data/goals/${slug}.handoff.json`];
}

/**
 * The goal switch on disk, once goal `from` is reached: its three files are deleted, its slug leaves
 * `goals`, and `to` becomes `live`. Returns every path it changed, repo-relative, for the caller to
 * commit. Throws when either goal is not on the chain, and changes nothing then.
 */
export function leaveGoal(from: string, to: string, root: string = ROOT): string[] {
  const c = chainRecord(root);
  for (const slug of [from, to]) if (!c.goals.includes(slug)) throw new Error(`${slug} is not on the chain`);
  const gone = goalFiles(from).filter((p) => existsSync(join(root, p)));
  for (const p of gone) rmSync(join(root, p));
  const chain = write(chainType, c.id, { live: to, goals: c.goals.filter((s) => s !== from) }, root).path;
  return [chain, ...gone];
}

/** slug -> goal, for every goal in front of the live one. None while no goal is live. */
export function walkedGoals(goals: ChainGoal[] = chainGoals(), live: ChainGoal | null = liveGoal(goals)): Map<string, ChainGoal> {
  return new Map(goals.filter((g) => live !== null && g.num < live.num).map((g) => [g.slug, g]));
}

/** The plan the driver runs for goal `slug`: its record, unchanged. Null when the goal has no record. */
export function goalPlan(slug: string, root: string = ROOT): Goal | null {
  return load(goalType, root).find((g) => g.id === slug)?.value ?? null;
}

/** The flags that narrow a `bun nv proofs` run to part of the roster. */
const ROSTER_SCOPES = ["--group", "--only", "--id"];

/** What a `bun nv proofs` check passes after the command's name, or null for any other check. */
function proofsArgs(c: GoalCheck): string[] | null {
  const argv = c.argv ?? [];
  return argv[0] === "bun" && argv[1] === "nv" && argv[2] === "proofs" ? argv.slice(3) : null;
}

/**
 * Every goal of `ahead`, the goals the run has still to reach in chain order, whose whole-roster gate
 * cannot pass where it stands, each with the goals behind it that are why. `bun nv proofs --gate` with no
 * scope is red while any roster group owes a proof, and a goal whose check names a group with `--group` is
 * that group's proofs still to write, so the gate is reached only behind it.
 */
export function gatesInFrontOfGroups(ahead: { slug: string; goal: Goal }[]): { slug: string; behind: string[] }[] {
  const has = (goal: Goal, test: (args: string[]) => boolean) =>
    goal.checks.some((c) => {
      const args = proofsArgs(c);
      return args !== null && test(args);
    });
  const out: { slug: string; behind: string[] }[] = [];
  ahead.forEach(({ slug, goal }, i) => {
    if (!has(goal, (a) => a.includes("--gate") && !ROSTER_SCOPES.some((s) => a.includes(s)))) return;
    const behind = ahead.slice(i + 1).filter((h) => has(h.goal, (a) => a.includes("--group"))).map((h) => h.slug);
    if (behind.length > 0) out.push({ slug, behind });
  });
  return out;
}

/** The side goal this process runs, or null in a chain run: `SIDE_ENV`'s slug, when a side goal's prose has it. */
export function sideGoal(root: string = ROOT): string | null {
  const slug = (process.env[SIDE_ENV] ?? "").trim();
  return SLUG.test(slug) && existsSync(join(root, GOALS, "side", `${slug}.md`)) ? slug : null;
}

/**
 * The plan a side run is checked against: side goal `slug`'s record. Its WSL leg builds into a target of
 * its own, `<targetDir>-side-<slug>`, because two runs sharing one would wait on each other's lock and
 * rebuild each other's crates. Null when the side goal has no record.
 */
export function sidePlan(slug: string, root: string = ROOT): Goal | null {
  const plan = load(sideGoalType, root).find((g) => g.id === slug)?.value;
  if (plan === undefined) return null;
  const wsl = plan.env.wsl;
  return wsl?.targetDir ? { ...plan, env: { ...plan.env, wsl: { ...wsl, targetDir: `${wsl.targetDir}-side-${slug}` } } } : plan;
}

/**
 * The plan this process works against: the side goal `SIDE_ENV` names, with `sidePlan`'s plan, or else the
 * live goal, with `goalPlan`'s. Null when neither has a record, or no goal is live.
 */
export function currentPlan(root: string = ROOT): { slug: string; goal: Goal } | null {
  const side = sideGoal(root);
  if (side !== null) {
    const goal = sidePlan(side, root);
    return goal === null ? null : { slug: side, goal };
  }
  const live = chainRecord(root).live;
  const goal = live === null ? null : goalPlan(live, root);
  return goal === null || live === null ? null : { slug: live, goal };
}
