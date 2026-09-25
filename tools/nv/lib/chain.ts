// Where the chain stands: its order, the live goal, the goals it has walked, and the plan the driver
// runs for a goal.
//
// All of it is `data/chain.json`: `goals` is the order and `live` the goal the driver works on. A goal has
// walked when it sits in front of the live one, or when its record has no checks, which is how a goal the
// driver will not reach again is written.
//
// **A goal's plan is its record with a floor carried in.** The floor is every check of every walked goal,
// each once, under one stage whose title is `floor`: the stage the goal's own record keeps for it, or a
// stage added for it. A walked goal keeps its checks for exactly this reason, and a goal switch copies
// nothing. A check is carried once by id, and a second check whose spec is the same apart from its id and
// stage is carried once too, since a goal names the conformance tree at every stage that leans on it and at
// the floor those are one check. The fixtures a walked goal's checks run and its valgrind skips come with
// them, because a floor whose fixtures are missing fails before anything is built.

import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "./paths.ts";
import type { RecordType } from "./schema.ts";
import { load, write } from "./store.ts";
import { chain as chainType } from "../schema/chain.ts";
import { goal as goalType } from "../schema/goal.ts";

const GOALS = "docs/agent/goals";

/** A goal's record, as `data/goals/<slug>.json` holds it. */
export type Goal = typeof goalType extends RecordType<infer T> ? T : never;
type GoalCheck = Goal["checks"][number];

/** The stage a floor is carried under when the goal's record does not keep one, and its title. */
export const FLOOR_STAGE = 1;
export const FLOOR_TITLE = "floor";

/** A goal file's front matter, which sits above its H1. */
const FRONT = /^---\n[\s\S]*?\n---\n/;

export interface ChainGoal {
  slug: string;
  /** 1-based, its place in `data/chain.json`. */
  num: number;
  milestone: string | null;
  retired: boolean;
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
  return order.map((slug, i) => {
    const g = records.get(slug);
    return {
      slug,
      num: i + 1,
      milestone: g?.milestone ?? null,
      retired: !g || g.checks.length === 0,
      md: prose.get(slug) ?? null,
    };
  });
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

/** slug -> goal, for every goal the chain is already past. */
export function walkedGoals(goals: ChainGoal[] = chainGoals(), live: ChainGoal | null = liveGoal(goals)): Map<string, ChainGoal> {
  return new Map(goals.filter((g) => g.retired || (live !== null && g.num < live.num)).map((g) => [g.slug, g]));
}

/** What a check asks, without its id and stage: two checks that ask the same are one check at the floor. */
function specOf(c: GoalCheck): string {
  const { id: _id, stage: _stage, ...rest } = c;
  return JSON.stringify(rest, Object.keys(rest).sort());
}

/**
 * `own` with every check of `walked` carried in as its floor, in chain order. The floor stage is the
 * one `own` titles `floor`, else `FLOOR_STAGE` when `own` has no stage by that number, else the stage
 * in front of its first. A carried check whose id `own` already uses for a different check is carried
 * as `<id>--<its goal>`.
 */
export function withFloor(own: Goal, walked: { slug: string; goal: Goal }[]): Goal {
  const kept = own.stages.find((s) => s.title.toLowerCase().includes(FLOOR_TITLE));
  const floor = kept?.number ?? (own.stages.some((s) => s.number === FLOOR_STAGE) ? Math.min(...own.stages.map((s) => s.number)) - 1 : FLOOR_STAGE);
  const ids = new Set(own.checks.map((c) => c.id));
  const specs = new Set(own.checks.filter((c) => c.stage === floor).map(specOf));
  const carried: GoalCheck[] = [];
  const files = new Set(own.files);
  const skip = new Set(own.env.valgrind?.skip ?? []);
  for (const { slug, goal } of walked) {
    for (const c of goal.checks) {
      const spec = specOf(c);
      if (specs.has(spec)) continue;
      specs.add(spec);
      const id = ids.has(c.id) ? `${c.id}--${slug}` : c.id;
      if (ids.has(id)) continue;
      ids.add(id);
      carried.push({ ...c, id, stage: floor });
    }
    for (const f of goal.files) files.add(f);
    for (const f of goal.env.valgrind?.skip ?? []) skip.add(f);
  }
  const stages = kept ? own.stages : [{ number: floor, title: FLOOR_TITLE, summary: "Every check of every walked goal still passes." }, ...own.stages];
  const env = skip.size > 0 ? { ...own.env, valgrind: { skip: [...skip] } } : own.env;
  return { ...own, stages, checks: [...own.checks, ...carried], files: [...files], env };
}

/**
 * The plan the driver runs for goal `slug`: its record with the floor carried in from every goal in
 * front of it on the chain. Null when the goal has no record.
 */
export function goalPlan(slug: string, root: string = ROOT): Goal | null {
  const records = new Map(load(goalType, root).map((g) => [g.id, g.value]));
  const own = records.get(slug);
  if (own === undefined) return null;
  const order = chainRecord(root).goals;
  const at = order.indexOf(slug);
  const walked = (at < 0 ? [] : order.slice(0, at)).flatMap((s) => {
    const goal = records.get(s);
    return goal === undefined ? [] : [{ slug: s, goal }];
  });
  return withFloor(own, walked);
}
