// The contract every key is held to: `data/impact-probes.json`, one synthetic edit per probe and the
// units it must and must not move. `bun nv impact --probe` applies each; `tools/nv/cmd/impact.ts` says
// how an edit and a pattern are read.

import { defineRecord, s } from "../lib/schema.ts";

export const impactProbes = defineRecord({
  name: "impact_probes",
  dir: "impact-probes",
  single: true,
  schema: s.object({
    probes: s.array(
      s.object({
        name: s.slug(),
        /** The edit, in words, and what the two lists stand for. */
        what: s.string(),
        edit: s.object({
          path: s.string(),
          after: s.optional(s.array(s.string())),
          insert: s.optional(s.string()),
          create: s.optional(s.string()),
        }),
        /** Patterns over `<role>: <name>`: each unit one matches must move. Never empty. */
        rerun: s.array(s.string()),
        /** Patterns over `<role>: <name>`: each unit one matches, and no `rerun` one does, must not. */
        keep: s.array(s.string()),
      }),
    ),
  }),
});
