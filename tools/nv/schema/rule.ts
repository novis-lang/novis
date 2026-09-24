// A rule: `data/rules/<topic>/<slug>.json`, whose id `<topic>/<slug>` is the `rule:` citation token.
// Its prose is `docs/rules/<topic>/<slug>.md`, which opens on the sentence `ground-rules.md` quotes.
//
// `because` lists the decisions that created it and then amended it, in that order, and a decision's
// `changes` is derived from these lists rather than written twice.

import { defineRecord, s } from "../lib/schema.ts";

export const rule = defineRecord({
  name: "rule",
  dir: "rules",
  schema: s.object({
    title: s.string(),
    status: s.enum("shipped", "designed"),
    because: s.array(s.ref("decision")),
    /** The sentence after "PHP …", for a rule whose behaviour differs from PHP's. */
    divergesFromPhp: s.optional(s.string()),
    seeAlso: s.array(s.ref("rule")),
    /** Repo-rooted paths of what fails when the rule is broken. */
    guardedBy: s.array(s.string()),
  }),
  idOf: (sub) => (/^[a-z0-9-]+\/[a-z0-9-]+\.json$/.test(sub) ? sub.slice(0, -".json".length) : null),
  checks: [
    {
      name: "a rule was decided somewhere",
      sql: `SELECT path, id || ' names no decision in because' AS detail FROM rule r
            WHERE NOT EXISTS (SELECT 1 FROM rule__because b WHERE b.owner = r.id)`,
    },
    {
      name: "a rule is in its topic's list",
      sql: `SELECT path, id || ' is not in its topic''s rules' AS detail FROM rule r
            WHERE NOT EXISTS (SELECT 1 FROM topic__rules t WHERE t.value = r.id AND t.owner = substr(r.id, 1, instr(r.id, '/') - 1))`,
    },
  ],
});
