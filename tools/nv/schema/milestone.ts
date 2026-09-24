// A milestone: `data/plan/milestones/<id>.json`, one row of the plan's milestone table, with its scope
// in `docs/plan/<id>.md` when it has one. A goal names the milestone it carries, and which goals carry a
// milestone is read off the goals rather than written here.
//
// A milestone no goal carries yet is `backlog`, and `backlog` is its place behind the chain.

import { defineRecord, s } from "../lib/schema.ts";

export const milestone = defineRecord({
  name: "milestone",
  dir: "plan/milestones",
  schema: s.object({
    title: s.string(),
    /** The table's order. */
    order: s.int(),
    /** The effort the plan estimated, as written: `~6 weeks`. */
    estimate: s.optional(s.string()),
    /** Loop-days, as written: `0.3`, `~2.5`, `measurement-bound`. */
    loopDays: s.string(),
    state: s.enum("done", "open", "ongoing"),
    backlog: s.optional(s.int()),
  }),
});
