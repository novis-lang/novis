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
  retired: "retired: its record keeps no checks",
  side: "side goal, off the chain",
};

function stateOf(g: ChainGoal, live: ChainGoal | null): State {
  if (g.retired) return "retired";
  if (live === null) return "ahead";
  return g.num < live.num ? "walked" : g.num === live.num ? "live" : "ahead";
}

/** One goal's heading, its line of facts and its stages. */
function section(heading: string, record: Goal | undefined, state: State, prose: string | null): string[] {
  const facts = [STATE_WORD[state]];
  if (record?.milestone) facts.push(`milestone ${record.milestone}`);
  if (record?.position === "last") facts.push("pinned last");
  if (prose !== null) facts.push(`[prose](${prose})`);
  const out = ["", heading, "", facts.join(" · ")];
  const stages = [...(record?.stages ?? [])].sort((a, b) => a.number - b.number);
  if (stages.length > 0) {
    out.push("");
    for (const s of stages) out.push(`- **${s.number} · ${s.title}** — ${s.summary ?? "no summary yet"}`);
  }
  return out;
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
    "not write that stage lists only its own stages here, and the loop adds the floor when it runs the goal.",
  ];
  goals.forEach((g, i) => {
    const record = records.get(g.slug);
    const prose = g.md === null ? null : `${PROSE_FROM_PLAN}/${g.md.split("/").pop()}`;
    lines.push(...section(`## ${g.num}. \`${g.slug}\` — ${record?.title ?? "no record on disk"}`, record, states[i]!, prose));
  });
  if (sides.length > 0) {
    lines.push("", "# Side goals", "", "Run by hand in a worktree of their own, never by the loop.");
    for (const side of sides) {
      const rel = `${PROSE_FROM_PLAN}/side/${side.id}.md`;
      const prose = existsSync(join(root, "docs/agent", rel)) ? rel : null;
      lines.push(...section(`## \`${side.id}\` — ${side.value.title}`, side.value, "side", prose));
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
