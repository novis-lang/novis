// What the feature proofs are excused from, and what still owes its help. `rule:testing/feature-proofs`
// is what a feature owes; `data/proofs/policy.json` is each single excuse with its reason as the value,
// and `data/proofs/help-backlog.json` is the features the help proof is not yet asked of.

import { defineRecord, s } from "../lib/schema.ts";

const reason = s.optional(s.string());

export const proofPolicy = defineRecord({
  name: "proof_policy",
  dir: "proofs/policy",
  single: true,
  schema: s.object({
    report: s.object({
      /** How far past its peers a figure is before the report names it. */
      outlierFactor: s.number(),
      /** Per declared complexity, the largest scaling ratio a bench may show. */
      ceiling: s.map(s.number()),
    }),
    /** Per feature id, per proof it is excused from, the reason. */
    skip: s.map(
      s.object({
        tests: reason,
        examples: reason,
        perf: reason,
        hostile: reason,
        about: reason,
        help: reason,
      }),
    ),
  }),
});

export const helpBacklog = defineRecord({
  name: "help_backlog",
  dir: "proofs/help-backlog",
  single: true,
  schema: s.object({ features: s.array(s.string()) }),
});
