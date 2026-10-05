// The spec chapters' tables, which the tools read. `bun nv render` writes them from the tables in
// `docs/spec/01-core-library.md` and `docs/spec/02-php-migration.md` (`renderers/spec-tables.ts`), and
// the chapters' prose stays there.
//
// The core library's tables differ by section, so each is kept whole: its section, its column names
// and its cells as Markdown. The migration table has one shape, one row per PHP name.

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

export const specPhpMigration = defineRecord({
  name: "spec_php_migration",
  dir: "spec/php-migration",
  single: true,
  schema: s.object({
    rows: s.array(
      s.object({
        /** The heading the row sits under, as written. */
        section: s.string(),
        /** The PHP built-in, exactly as PHP spells it. */
        php: s.string(),
        outcome: s.enum("member", "language", "dropped", "open"),
        /** Markdown: the member, the construct, or the reason and the rewrite. */
        novis: s.string(),
      }),
    ),
  }),
  checks: [
    {
      name: "a PHP name has one row",
      sql: `SELECT min(path) AS path, php || ' has ' || count(*) || ' rows' AS detail
            FROM spec_php_migration__rows GROUP BY php HAVING count(*) > 1`,
    },
  ],
});
