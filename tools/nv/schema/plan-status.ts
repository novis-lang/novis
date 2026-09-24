// The implementation plan's status block: `data/plan/status.json`, seven Markdown fields and no others.
// The plan renders it at its head, and a session's wrap overwrites a field whole.

import { defineRecord, s } from "../lib/schema.ts";

export const planStatus = defineRecord({
  name: "plan_status",
  dir: "plan/status",
  single: true,
  schema: s.object({
    status: s.string(),
    done: s.string(),
    onDisk: s.string(),
    toolchain: s.string(),
    adrSlicesLanded: s.string(),
    openNow: s.string(),
    blocking: s.string(),
  }),
});
