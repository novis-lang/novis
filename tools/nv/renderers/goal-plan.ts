// The goal plan: every goal on the chain in the order the loop walks it, each with its stages, and then
// the side goals, which are off the chain. It is rendered from `data/chain.json` and the goal records, and
// the tools that change those through their own code write it again at once: `bun nv chain`'s edits, the
// loop's goal switch and every `nv session --wrap`. A hand edit to a record is caught by `nv render --check`.

import { existsSync } from "node:fs";
import { join } from "node:path";
import { type ChainGoal, type Goal, chainGoals, liveGoal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { apply, markdown, type Output, type Renderer } from "../lib/render.ts";
import { load } from "../lib/store.ts";
import { goal as goalType, sideGoal } from "../schema/goal.ts";

export const GOAL_PLAN = "docs/agent/goal-plan.md";

/** Where a goal's prose sits, relative to the plan's own directory. */
const PROSE_FROM_PLAN = "goals";

type State = "walked" | "live" | "ahead" | "retired" | "side";

const STATE_WORD: Record<State, string> = {
  walked: "walked",
  live: "**live**",
  ahead: "ahead",
  retired: "retired",
  side: "side",
};

function stateOf(g: ChainGoal, live: ChainGoal | null): State {
  if (g.retired) return "retired";
  if (live === null) return "ahead";
  return g.num < live.num ? "walked" : g.num === live.num ? "live" : "ahead";
}

/** `text` safe inside one table cell. */
const cell = (text: string) => text.replace(/\|/g, "\\|").replace(/\s*\n\s*/g, " ");

/** One goal's table cells after its position: the slug linked to its prose, title, state, milestone and stage titles. */
function row(slug: string, record: Goal | undefined, state: State, prose: string | null): string {
  const name = prose === null ? `\`${slug}\`` : `[\`${slug}\`](${prose})`;
  const stages = [...(record?.stages ?? [])].sort((a, b) => a.number - b.number).map((s) => `**${s.number}** ${cell(s.title)}`);
  const pinned = record?.position === "last" ? ", pinned last" : "";
  return `${name} | ${cell(record?.title ?? "no record on disk")} | ${STATE_WORD[state]}${pinned} | ${record?.milestone ?? ""} | ${stages.join(" · ")} |`;
}

export function renderGoalPlan(root: string = ROOT): Output {
  const goals = chainGoals(root);
  const live = liveGoal(goals, root);
  const records = new Map(load(goalType, root).map((g) => [g.id, g.value]));
  const sides = load(sideGoal, root).sort((a, b) => a.id.localeCompare(b.id));
  const states = goals.map((g) => stateOf(g, live));
  const count = (s: State) => states.filter((x) => x === s).length;
  const where = live === null ? "no goal is live" : `\`${live.slug}\` is live at ${live.num} of ${goals.length}`;

  const lines = [
    "# The goal plan",
    "",
    "Every goal on the chain, in the order the loop walks them, with the stages of each, and then the side",
    "goals, which are off the chain. `bun nv render` writes this file from `data/chain.json` and",
    "`data/goals/`: change those, never this file.",
    "",
    `The chain holds ${goals.length} goals. ${count("walked") + count("retired")} are walked (${count("retired")} of them retired), ${where}, and ${count("ahead")} are ahead.`,
    "",
    "A goal's stage 1 is its floor: every check of every goal in front of it still passes. A record that does",
    "not write that stage lists only its own stages here, and the loop adds the floor when it runs the goal. A",
    "retired goal's record keeps no checks and no stages, so its prose is where to read it.",
    "",
    "| # | Goal | What it builds | State | Milestone | Stages |",
    "|--:|---|---|---|---|---|",
  ];
  goals.forEach((g, i) => {
    const prose = g.md === null ? null : `${PROSE_FROM_PLAN}/${g.md.split("/").pop()}`;
    lines.push(`| ${g.num} | ${row(g.slug, records.get(g.slug), states[i]!, prose)}`);
  });
  if (sides.length > 0) {
    lines.push("", "## Side goals", "", "Each runs in a worktree of its own, started by hand with `bun nv loop --side <slug>`.", "");
    lines.push("| Goal | What it builds | State | Milestone | Stages |", "|---|---|---|---|---|");
    for (const side of sides) {
      const rel = `${PROSE_FROM_PLAN}/side/${side.id}.md`;
      const prose = existsSync(join(root, "docs/agent", rel)) ? rel : null;
      lines.push(`| ${row(side.id, side.value, "side", prose)}`);
    }
  }
  return { path: GOAL_PLAN, text: markdown(lines.join("\n")) };
}

/** Writes the goal plan when the records give a different one: its path when it did, else "". */
export function writeGoalPlan(root: string = ROOT): string {
  return apply([renderGoalPlan(root)], { check: false, root }).stale.length > 0 ? GOAL_PLAN : "";
}

export const goalPlan: Renderer = {
  name: "goal-plan",
  render: (root) => [renderGoalPlan(root)],
};
