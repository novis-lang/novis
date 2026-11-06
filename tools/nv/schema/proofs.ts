// What the feature proofs are excused from, and what still owes its help. `rule:testing/feature-proofs`
// is what a feature owes; `data/proofs/policy.json` is each kind's override of that and the perf
// report's bounds, each file under `data/proofs/skip/` is one feature's excuses with their reasons,
// `data/proofs/help-backlog.json` is the features the help proof is not yet asked of, and
// `data/proofs/intro-backlog.json` is the members their class introduction does not yet have to name.

import { defineRecord, s } from "../lib/schema.ts";

const reason = s.optional(s.string());
const count = s.optional(s.int());
const owed = s.optional(s.boolean());

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
    /**
     * Per kind, or `all` for every kind, what it owes over `POLICY` in `tools/nv/proofs/collect.ts`. `all`
     * applies first and a kind's own entry over it. A proof that is off for every kind by default, such
     * as `about` or `comments`, is switched on here.
     */
    owes: s.optional(
      s.map(
        s.object({
          tests: count,
          rust: count,
          examples: count,
          perf: owed,
          hostile: count,
          about: owed,
          help: owed,
          comments: owed,
          intro: owed,
        }),
      ),
    ),
  }),
});

/**
 * One feature's excuses: per proof it is excused from, the reason. The file is the feature's roster
 * path, the one its examples, attacks and bench share, so `Core\Cache::shared` is
 * `data/proofs/skip/core/Cache/shared.json`. A path does not give its id back, so `feature` names it.
 */
export const proofSkip = defineRecord({
  name: "proof_skip",
  dir: "proofs/skip",
  schema: s.object({
    feature: s.string(),
    tests: reason,
    examples: reason,
    perf: reason,
    hostile: reason,
    about: reason,
    help: reason,
    intro: reason,
  }),
  checks: [
    {
      name: "a feature has one skip file",
      sql: `SELECT path, feature || ' is skipped by another file too' AS detail FROM proof_skip s
            WHERE EXISTS (SELECT 1 FROM proof_skip o WHERE o.feature = s.feature AND o.path <> s.path)`,
    },
  ],
});

export const helpBacklog = defineRecord({
  name: "help_backlog",
  dir: "proofs/help-backlog",
  single: true,
  schema: s.object({ features: s.array(s.string()) }),
});

export const introBacklog = defineRecord({
  name: "intro_backlog",
  dir: "proofs/intro-backlog",
  single: true,
  schema: s.object({ features: s.array(s.string()) }),
});
