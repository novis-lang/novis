// The core library chapter's tables, which the tools read. `bun nv render` writes them from the tables
// in `docs/spec/01-core-library.md` (`renderers/spec-tables.ts`), and the chapter's prose stays there.
//
// The tables differ by section, so each is kept whole: its section, its column names and its cells as
// Markdown.

import { defineRecord, s } from "../lib/schema.ts";

export const specCoreMembers = defineRecord({
  name: "spec_core_members",
  dir: "spec/core-members",
  single: true,
  schema: s.object({
    tables: s.array(
      s.object({
        /** The headings the table sits under, outermost first, as written. */
        section: s.array(s.string()),
        columns: s.array(s.string()),
        rows: s.array(s.array(s.string())),
      }),
    ),
  }),
});
