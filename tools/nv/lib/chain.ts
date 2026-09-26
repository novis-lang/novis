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
// stage is carried once too. A walked test check the permanent suite already runs graduates
// (`graduatesTo`): the floor carries its crate's or its case tree's whole run once in its place, and
// `bun nv chain --check` holds every test and case it named to still being there. The fixtures a walked
// goal's checks run and its valgrind skips come with them, because a floor whose fixtures are missing
// fails before anything is built.
//
// **A side goal's plan is its record with main's carried floor**: the checks of every goal in front of
// the live one, and never the live goal's own, which are that goal's unfinished work. A process belongs to
// a side run when `SIDE_ENV` names a side goal whose prose is on disk.

import { existsSync, readdirSync } from "node:fs";
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

/** The `.nvst` trees `nv verify` runs whole, through the debug binary. */
export const CASE_TREES = ["conformance", "differential"];
/** The fields a test check may have and still graduate: none that asks more than the permanent suite does. */
const GRADUATING_FIELDS: Record<string, Set<string>> = {
  "cargo-named": new Set(["id", "kind", "stage", "name", "args", "tests"]),
  "nvs-suite": new Set(["id", "kind", "stage", "name", "args", "cases"]),
};

/**
 * The permanent-suite check a walked goal's test check graduates to, or null when it stays on the floor
 * as it is. A `cargo test -p <crate>` check, alone or narrowed by `--lib`, `--bin` or `--test`, graduates to
 * its crate's whole debug run, and an `nvs test` over a case tree or a directory in it graduates to that
 * whole tree: `nv verify` runs both, so the floor keeps one check per crate and per tree instead of one
 * per goal that named them. A release build, a feature, a flag past the target, a `minPassing` count or any
 * other field asks what the suite does not, and keeps the check.
 */
export function graduatesTo(c: GoalCheck): GoalCheck | null {
  const allowed = GRADUATING_FIELDS[c.kind];
  if (allowed === undefined || !Object.keys(c).every((k) => allowed.has(k))) return null;
  const args = c.args ?? [];
  if (args[0] !== "test") return null;
  if (c.kind === "cargo-named") {
    const pkg = args[1] === "-p" ? args[2] : undefined;
    if (pkg === undefined) return null;
    const rest = args.slice(3);
    const narrowed = rest.length === 0 || (rest.length === 1 && rest[0] === "--lib") || (rest.length === 2 && (rest[0] === "--bin" || rest[0] === "--test"));
    if (!narrowed) return null;
    return { id: `permanent-suite-${pkg}`, kind: "cargo-named", stage: c.stage, name: `the permanent suite: cargo test -p ${pkg}`, args: ["test", "-p", pkg], tests: [] };
  }
  const path = (args[1] ?? "").replaceAll("\\", "/");
  const tree = args.length === 2 ? CASE_TREES.find((t) => path === `tests/${t}` || path.startsWith(`tests/${t}/`)) : undefined;
  if (tree === undefined) return null;
  return { id: `permanent-suite-${tree}`, kind: "nvs-suite", stage: c.stage, name: `the permanent suite: nvs test tests/${tree}/`, args: ["test", `tests/${tree}/`] };
}

/** What a check asks, without its id and stage: two checks that ask the same are one check at the floor. */
function specOf(c: GoalCheck): string {
  const { id: _id, stage: _stage, ...rest } = c;
  return JSON.stringify(rest, Object.keys(rest).sort());
}

/**
 * `own` with every check of `walked` carried in as its floor, in chain order, and then the permanent-suite
 * checks the graduated ones stand for, one per crate and per case tree. The floor stage is the one `own`
 * titles `floor`, else `FLOOR_STAGE` when `own` has no stage by that number, else the stage in front of its
 * first. A carried check whose id `own` already uses for a different check is carried as `<id>--<its goal>`.
 */
export function withFloor(own: Goal, walked: { slug: string; goal: Goal }[]): Goal {
  const kept = own.stages.find((s) => s.title.toLowerCase().includes(FLOOR_TITLE));
  const floor = kept?.number ?? (own.stages.some((s) => s.number === FLOOR_STAGE) ? Math.min(...own.stages.map((s) => s.number)) - 1 : FLOOR_STAGE);
  const ids = new Set(own.checks.map((c) => c.id));
  const specs = new Set(own.checks.filter((c) => c.stage === floor).map(specOf));
  const carried: GoalCheck[] = [];
  const suite = new Map<string, GoalCheck>();
  const files = new Set(own.files);
  const skip = new Set(own.env.valgrind?.skip ?? []);
  for (const { slug, goal } of walked) {
    for (const c of goal.checks) {
      const to = graduatesTo(c);
      if (to !== null) {
        if (!suite.has(to.id)) suite.set(to.id, to);
        continue;
      }
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
  for (const c of suite.values()) {
    const spec = specOf(c);
    if (specs.has(spec) || ids.has(c.id)) continue;
    specs.add(spec);
    ids.add(c.id);
    carried.push({ ...c, stage: floor });
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

/** The side goal this process runs, or null in a chain run: `SIDE_ENV`'s slug, when a side goal's prose has it. */
export function sideGoal(root: string = ROOT): string | null {
  const slug = (process.env[SIDE_ENV] ?? "").trim();
  return SLUG.test(slug) && existsSync(join(root, GOALS, "side", `${slug}.md`)) ? slug : null;
}

/**
 * The plan a side run is checked against: side goal `slug`'s record with the floor carried in from every
 * goal in front of the live one. Its WSL leg builds into a target of its own, `<targetDir>-side-<slug>`,
 * because two runs sharing one would wait on each other's lock and rebuild each other's crates. Null when
 * the side goal has no record.
 */
export function sidePlan(slug: string, root: string = ROOT): Goal | null {
  const own = load(sideGoalType, root).find((g) => g.id === slug)?.value;
  if (own === undefined) return null;
  const records = new Map(load(goalType, root).map((g) => [g.id, g.value]));
  const { live, goals } = chainRecord(root);
  const at = live === null ? -1 : goals.indexOf(live);
  const walked = (at < 0 ? [] : goals.slice(0, at)).flatMap((s) => {
    const goal = records.get(s);
    return goal === undefined ? [] : [{ slug: s, goal }];
  });
  const plan = withFloor(own, walked);
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
