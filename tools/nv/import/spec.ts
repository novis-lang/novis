// The spec chapters' tables. `docs/spec/01-core-library.md` becomes `data/spec/core-members.json`:
// every table in it, whole, under the headings it sits beneath, which is what `bun nv gaps`,
// `bun nv migration` and the website's Core reference (`tools/nv/renderers/website-core.ts`)
// each pick their own rows out of.
// `docs/spec/02-php-migration.md` becomes `data/spec/php-migration.json`: one row per line of a
// `PHP | Outcome | Novis` table, which is what `bun nv reference` and `bun nv migration` read. Every other table in that chapter is its legend, and stays prose.
//
// A cell is kept as written, trimmed, with its `\|` escapes intact, so a renderer writes it back
// unchanged. A section is the heading text without its `#`s.

import { specCoreMembers, specPhpMigration } from "../schema/spec.ts";
import { text, type Importer, type ImportResult, type Unread } from "./lib.ts";

const CORE = "docs/spec/01-core-library.md";
const MIGRATION = "docs/spec/02-php-migration.md";
const HEADING = /^(#{1,6})\s+(.*?)\s*$/;
const SEPARATOR = /^\|(\s*:?-+:?\s*\|)+\s*$/;
const MIGRATION_ROW = /^\|\s*`([^`]+)`\s*\|\s*([a-z]+)\s*\|(.*)\|\s*$/;

/** The cells of one table line: split on every `|` that is not escaped, each trimmed. */
export function cells(line: string): string[] {
  const inner = line.trim().replace(/^\|/, "").replace(/(?<!\\)\|$/, "");
  return inner.split(/(?<!\\)\|/).map((c) => c.trim());
}

interface Table {
  section: string[];
  columns: string[];
  /** Each row with the 1-based line it was read from. */
  rows: { line: number; cells: string[] }[];
}

/** Every table in `src`: a `|` line followed by a separator line, and the `|` lines after them. */
export function tables(src: string): Table[] {
  const lines = src.split("\n");
  const stack: string[] = [];
  const out: Table[] = [];
  let fence = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    if (/^\s*(```|~~~)/.test(line)) fence = !fence;
    if (fence) continue;
    const h = HEADING.exec(line);
    if (h) {
      stack.length = h[1]!.length - 1;
      stack[h[1]!.length - 1] = h[2]!;
      continue;
    }
    if (!line.startsWith("|") || !SEPARATOR.test(lines[i + 1] ?? "")) continue;
    const table: Table = { section: stack.filter((s) => s !== undefined), columns: cells(line), rows: [] };
    i += 2;
    for (; i < lines.length && lines[i]!.startsWith("|"); i++) table.rows.push({ line: i + 1, cells: cells(lines[i]!) });
    i--;
    out.push(table);
  }
  return out;
}

function misshapen(path: string, t: Table): Unread[] {
  return t.rows
    .filter((r) => r.cells.length !== t.columns.length)
    .map((r) => ({ path, reason: `line ${r.line} has ${r.cells.length} cell(s) under ${t.columns.length} column(s)` }));
}

export const spec: Importer = {
  name: "spec",
  read(root) {
    const out: ImportResult = { records: [], unread: [], files: 2 };

    const core = tables(text(root, CORE));
    for (const t of core) out.unread.push(...misshapen(CORE, t));
    out.records.push({
      type: specCoreMembers,
      id: specCoreMembers.name,
      value: { tables: core.map((t) => ({ section: t.section, columns: t.columns, rows: t.rows.map((r) => r.cells) })) },
      from: CORE,
    });

    const rows: { section: string; php: string; outcome: string; novis: string }[] = [];
    const src = text(root, MIGRATION);
    const lines = src.split("\n");
    for (const t of tables(src)) {
      if (t.columns.join("|") !== "PHP|Outcome|Novis") continue;
      const section = t.section[t.section.length - 1] ?? "";
      for (const r of t.rows) {
        const m = MIGRATION_ROW.exec(lines[r.line - 1]!);
        if (!m) {
          out.unread.push({ path: MIGRATION, reason: `line ${r.line} is no \`| \`name\` | outcome | novis |\` row` });
          continue;
        }
        rows.push({ section, php: m[1]!, outcome: m[2]!, novis: m[3]!.trim() });
      }
    }
    out.records.push({ type: specPhpMigration, id: specPhpMigration.name, value: { rows }, from: MIGRATION });
    return out;
  },
};
