// Where the chain stands: its order, the goal the driver has installed, and the goals it has walked.
//
// The order is `data/chain.json`. The installed goal is the slug the driver's pointer names
// (`lib/state.ts`). A tree the driver has never run in has no pointer, and there the installed goal is
// the one whose prose has the H1 that `docs/agent/loop-goal.md` has. A goal has walked when it sits in front of the installed one, or when its record has no
// checks, which is how a goal the driver will not reach again is written. The union is taken so that a
// checkout that has never run the loop still counts the retired goals as walked.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "./paths.ts";
import { pointerSlug } from "./state.ts";
import { load } from "./store.ts";
import { chain as chainType } from "../schema/chain.ts";
import { goal as goalType } from "../schema/goal.ts";

export const LIVE_GOAL = "docs/agent/loop-goal.md";
const GOALS = "docs/agent/goals";

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

/** Every goal on the chain, in chain order. */
export function chainGoals(root: string = ROOT): ChainGoal[] {
  const order = load(chainType, root)[0]?.value ?? [];
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

/** The goal the driver's pointer names, else the one `docs/agent/loop-goal.md` is a copy of, else null. */
export function liveGoal(goals?: ChainGoal[], root: string = ROOT): ChainGoal | null {
  goals ??= chainGoals(root);
  const slug = pointerSlug(root);
  const pointed = slug === null ? undefined : goals.find((g) => g.slug === slug);
  if (pointed) return pointed;
  if (!existsSync(join(root, LIVE_GOAL))) return null;
  const head = h1Of(readFileSync(join(root, LIVE_GOAL), "utf8"));
  if (!head) return null;
  for (const g of goals) {
    if (g.md && existsSync(join(root, g.md)) && h1Of(readFileSync(join(root, g.md), "utf8")) === head) return g;
  }
  return null;
}

/** slug -> goal, for every goal the chain is already past. */
export function walkedGoals(goals: ChainGoal[] = chainGoals(), live: ChainGoal | null = liveGoal(goals)): Map<string, ChainGoal> {
  return new Map(goals.filter((g) => g.retired || (live !== null && g.num < live.num)).map((g) => [g.slug, g]));
}
