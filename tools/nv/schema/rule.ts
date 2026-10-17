// A rule: `data/rules/<topic>/<slug>.json`, whose id `<topic>/<slug>` is the `rule:` citation token.
// Its prose, reasons included, is `docs/rules/<topic>/<slug>.md`, which opens on the sentence
// `ground-rules.md` quotes.
//
// A `designed` rule names the goal that will ship it, by slug, as a gap names its owner. A `shipped`
// rule names none. `bun nv owners` classifies the owner.

import { defineRecord, s } from "../lib/schema.ts";

export const rule = defineRecord({
  name: "rule",
  dir: "rules",
  schema: s.object({
    title: s.string(),
    status: s.enum("shipped", "designed"),
    seeAlso: s.array(s.ref("rule")),
    /** Repo-rooted paths of what fails when the rule is broken. */
    guardedBy: s.array(s.string()),
    /** The slug of a goal file under `docs/goals/` that ships this designed rule. */
    goal: s.optional(s.slug()),
  }),
  idOf: (sub) => (/^[a-z0-9-]+\/[a-z0-9-]+\.json$/.test(sub) ? sub.slice(0, -".json".length) : null),
  checks: [
    {
      name: "a rule is in its topic's list",
      sql: `SELECT path, id || ' is not in its topic''s rules' AS detail FROM rule r
            WHERE NOT EXISTS (SELECT 1 FROM topic__rules t WHERE t.value = r.id AND t.owner = substr(r.id, 1, instr(r.id, '/') - 1))`,
    },
    {
      name: "a shipped rule names no owner",
      sql: `SELECT path, id || ' is shipped and names goal ' || goal AS detail
            FROM rule WHERE status = 'shipped' AND goal IS NOT NULL`,
    },
  ],
});
