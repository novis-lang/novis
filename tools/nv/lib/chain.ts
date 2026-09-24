// Where the chain stands: its order, the goal the driver has installed, and the goals it has walked.
//
// The order is `data/chain.json`. The installed goal is the one whose prose has the H1 that
// `docs/agent/loop-goal.md` has, since the switch copies that file verbatim and a record carries no
// flag for it. A goal has walked when it sits in front of the installed one, or when its record has no
// checks, which is how a goal the driver will not reach again is written. The union is taken so that a
// checkout that has never run the loop still counts the retired goals as walked.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "./paths.ts";
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

/** slug -> its prose file, named `<slug>.md` or `N-<slug>.md`, in the goals directory or its `dossier/`. */
function proseFiles(): Map<string, string> {
  const out = new Map<string, string>();
  for (const sub of [`${GOALS}/dossier`, GOALS]) {
    const dir = join(ROOT, sub);
    if (!existsSync(dir)) continue;
    for (const name of readdirSync(dir)) {
      const m = /^(?:\d+-)?([a-z0-9]+(?:-[a-z0-9]+)*)\.md$/.exec(name);
      if (m && name !== "README.md") out.set(m[1]!, `${sub}/${name}`);
    }
  }
  return out;
}

/** Every goal on the chain, in chain order. */
export function chainGoals(): ChainGoal[] {
  const order = load(chainType)[0]?.value ?? [];
  const records = new Map(load(goalType).map((g) => [g.id, g.value]));
  const prose = proseFiles();
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

/** The goal `docs/agent/loop-goal.md` is a copy of, or null. */
export function liveGoal(goals: ChainGoal[] = chainGoals()): ChainGoal | null {
  if (!existsSync(join(ROOT, LIVE_GOAL))) return null;
  const head = h1Of(readFileSync(join(ROOT, LIVE_GOAL), "utf8"));
  if (!head) return null;
  for (const g of goals) {
    if (g.md && existsSync(join(ROOT, g.md)) && h1Of(readFileSync(join(ROOT, g.md), "utf8")) === head) return g;
  }
  return null;
}

/** slug -> goal, for every goal the chain is already past. */
export function walkedGoals(goals: ChainGoal[] = chainGoals(), live: ChainGoal | null = liveGoal(goals)): Map<string, ChainGoal> {
  return new Map(goals.filter((g) => g.retired || (live !== null && g.num < live.num)).map((g) => [g.slug, g]));
}
