// A topic of the rulebook: `data/rules/<topic>.json`, one chapter `docs/rules/<topic>.md` renders. Its
// `rules` list is the order the chapter prints them in, by full id.

import { defineRecord, s } from "../lib/schema.ts";

export const topic = defineRecord({
  name: "topic",
  dir: "rules",
  schema: s.object({
    title: s.string(),
    order: s.int(),
    rules: s.array(s.ref("rule")),
  }),
  idOf: (sub) => (/^[a-z0-9-]+\.json$/.test(sub) ? sub.slice(0, -".json".length) : null),
  checks: [
    {
      name: "a topic lists only its own rules",
      sql: `SELECT path, value || ' is listed under ' || owner AS detail FROM topic__rules
            WHERE substr(value, 1, instr(value, '/') - 1) <> owner`,
    },
  ],
});
