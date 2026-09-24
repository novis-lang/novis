// A goal's handoff: where its work stands, the group of slices the next session takes, and what waits
// behind them. It sits beside its goal as `<slug>.handoff.json`, and its id is the goal's slug.

import { defineRecord, s } from "../lib/schema.ts";

export const handoffShape = s.object({
  /** Markdown: where the work stands now. */
  state: s.string(),
  next: s.object({
    /** The goal stage the group is in, which picks that stage's `context`; null for none. */
    stage: s.nullable(s.int()),
    title: s.string(),
    /** The file set the group's slices share. */
    files: s.array(s.string()),
    /** Markdown after the file set on the group's lead line, when there is any. */
    note: s.optional(s.string()),
    items: s.array(s.object({ done: s.boolean(), text: s.string() })),
  }),
  backlog: s.array(s.string()),
});

/** `<slug>.handoff.json` directly under the type's directory. */
function handoffIdOf(sub: string): string | null {
  if (sub.includes("/") || !sub.endsWith(".handoff.json")) return null;
  return sub.slice(0, -".handoff.json".length);
}

function ownedBy(t: string, goalTable: string): { name: string; sql: string } {
  return {
    name: "a handoff belongs to a goal",
    sql: `SELECT path, 'no goal ' || id || ' for this handoff' AS detail FROM ${t}
          WHERE id NOT IN (SELECT id FROM ${goalTable})`,
  };
}

export const handoff = defineRecord({
  name: "handoff",
  dir: "goals",
  suffix: ".handoff.json",
  schema: handoffShape,
  idOf: handoffIdOf,
  checks: [ownedBy("handoff", "goal")],
});

export const sideHandoff = defineRecord({
  name: "side_handoff",
  dir: "goals/side",
  suffix: ".handoff.json",
  schema: handoffShape,
  idOf: handoffIdOf,
  checks: [ownedBy("side_handoff", "side_goal")],
});
