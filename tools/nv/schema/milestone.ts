// A milestone: `data/plan/milestones/<id>.json`, one row of the plan's milestone table, with its scope
// in `docs/plan/<id>.md`. A goal names the milestone it carries, and which goals carry a milestone is
// read off the goals rather than written here.
//
// The plan holds only the milestones still ahead: a finished one's record and scope file are deleted.
// A milestone a goal carries is `open`. One no goal carries is `ongoing`, or `open` with a `backlog`
// place, its position behind the chain.

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
    /** Loop-days, as written: `~8`, `measurement-bound`. */
    loopDays: s.string(),
    state: s.enum("open", "ongoing"),
    backlog: s.optional(s.int()),
  }),
});
