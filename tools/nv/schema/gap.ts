// A known gap: `data/gaps/<crate>/<slug>.json`, something a shipped feature still owes, named by a slug
// that never changes and owned by exactly one goal or milestone.

import { defineRecord, s } from "../lib/schema.ts";

export const gap = defineRecord({
  name: "gap",
  dir: "gaps",
  schema: s.object({
    /** The repo-rooted file whose code owes it. */
    module: s.string(),
    /** Markdown: the gap as one claim. */
    title: s.string(),
    /** Markdown: why it is open and what closes it. */
    text: s.string(),
    goal: s.optional(s.ref("goal")),
    milestone: s.optional(s.ref("milestone")),
  }),
  checks: [
    {
      name: "a gap has exactly one owner",
      sql: `SELECT path, 'names ' || CASE WHEN goal IS NULL THEN 'no owner' ELSE 'both a goal and a milestone' END AS detail
            FROM gap WHERE (goal IS NULL) = (milestone IS NULL)`,
    },
  ],
});
