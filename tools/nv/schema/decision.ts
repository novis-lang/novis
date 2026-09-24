// A decision record's fields: `data/decisions/NNNN.json`. The reasoning is `docs/decisions/NNNN.md`,
// frozen on acceptance, and it holds prose and nothing else: its front matter and its bold field lines
// are rendered from this record. Which rules a decision created or amended is not written here, because
// the rules' `because` lists already say it.
//
// `summary` is the plain-language entry `docs/decisions.md` and the website print. A record that has
// none is left out of that page.

import { defineRecord, s } from "../lib/schema.ts";

export const decision = defineRecord({
  name: "decision",
  dir: "decisions",
  schema: s.object({
    /** The decision as a statement of what is now true: the H1 after `ADR NNNN — `. */
    title: s.string(),
    status: s.enum("accepted", "retired"),
    /** Markdown: what this decides, and what it leaves to another record. */
    scope: s.string(),
    dependsOn: s.array(s.ref("decision")),
    /** Markdown: the guard test or the measurement that holds it. */
    validatedBy: s.optional(s.string()),
    summary: s.optional(
      s.object({
        group: s.enum("foundations", "security", "language", "types", "syntax", "core", "tooling", "internal"),
        headline: s.string(),
        body: s.string(),
        /** Floats the entry to the top of its group. */
        pin: s.optional(s.boolean()),
      }),
    ),
  }),
  idOf: (sub) => (/^\d{4}\.json$/.test(sub) ? sub.slice(0, 4) : null),
});
