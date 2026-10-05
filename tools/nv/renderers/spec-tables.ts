// The spec chapters' tables, rendered as records. The chapters are where the tables are written, and the
// records are what the tools read. `docs/spec/01-core-library.md` renders `data/spec/core-members.json`:
// every table in it, whole, under the headings it sits beneath, which is what `bun nv gaps`,
// `bun nv migration` and the website's Core reference (`website-core.ts`) each pick their own rows out
// of. `docs/spec/02-php-migration.md` renders `data/spec/php-migration.json`: one row per line of a
// `PHP | Outcome | Novis` table, which is what `bun nv reference` and `bun nv migration` read. Every
// other table in that chapter is its legend, and stays prose.
//
// A cell is kept as written, trimmed, with its `\|` escapes intact, so a reader can write it back
// unchanged. A section is the heading text without its `#`s. A row whose cell count is not its table's
// column count, and a line of a migration table that is not a `` | `name` | outcome | novis | `` row,
// fail the render naming the line.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { Output, Renderer } from "../lib/render.ts";
import type { RecordType } from "../lib/schema.ts";
import { pathOf, serialize } from "../lib/store.ts";
import { specCoreMembers, specPhpMigration } from "../schema/spec.ts";

export const CORE = "docs/spec/01-core-library.md";
export const MIGRATION = "docs/spec/02-php-migration.md";
const HEADING = /^(#{1,6})\s+(.*?)\s*$/;
/** A Markdown table's separator line, `|---|:--:|`. */
export const SEPARATOR = /^\|(\s*:?-+:?\s*\|)+\s*$/;
const MIGRATION_ROW = /^\|\s*`([^`]+)`\s*\|\s*([a-z]+)\s*\|(.*)\|\s*$/;

/** The cells of one table line: split on every `|` that is not escaped, each trimmed. */
export function cells(line: string): string[] {
  const inner = line.trim().replace(/^\|/, "").replace(/(?<!\\)\|$/, "");
  return inner.split(/(?<!\\)\|/).map((c) => c.trim());
}

export interface Table {
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

function text(root: string, path: string): string {
  return readFileSync(join(root, path), "utf8").replace(/\r\n?/g, "\n");
}

/** The `spec_core_members` record's value, read from the core library chapter. */
export function coreMembers(root: string): { tables: { section: string[]; columns: string[]; rows: string[][] }[] } {
  const core = tables(text(root, CORE));
  for (const t of core) {
    const bad = t.rows.find((r) => r.cells.length !== t.columns.length);
    if (bad) throw new Error(`${CORE}:${bad.line} has ${bad.cells.length} cell(s) under ${t.columns.length} column(s)`);
  }
  return { tables: core.map((t) => ({ section: t.section, columns: t.columns, rows: t.rows.map((r) => r.cells) })) };
}

/** The `spec_php_migration` record's value, read from the PHP migration chapter. */
export function phpMigration(root: string): { rows: { section: string; php: string; outcome: string; novis: string }[] } {
  const rows: { section: string; php: string; outcome: string; novis: string }[] = [];
  const src = text(root, MIGRATION);
  const lines = src.split("\n");
  for (const t of tables(src)) {
    if (t.columns.join("|") !== "PHP|Outcome|Novis") continue;
    const section = t.section[t.section.length - 1] ?? "";
    for (const r of t.rows) {
      const m = MIGRATION_ROW.exec(lines[r.line - 1]!);
      if (!m) throw new Error(`${MIGRATION}:${r.line} is no \`| \`name\` | outcome | novis |\` row`);
      rows.push({ section, php: m[1]!, outcome: m[2]!, novis: m[3]!.trim() });
    }
  }
  return { rows };
}

/** The record file of the single-record `type` holding `value`, which its schema must accept. */
function record<T>(type: RecordType<T>, value: unknown, from: string): Output {
  const issues = type.schema.validate(value);
  if (issues.length > 0) throw new Error(`${from}: ${issues.map((i) => `${i.at || "(value)"}: ${i.message}`).join("; ")}`);
  return { path: pathOf(type, type.name), text: serialize(type, value as T) };
}

export const specTables: Renderer = {
  name: "spec-tables",
  render: (root: string): Output[] => [record(specCoreMembers, coreMembers(root), CORE), record(specPhpMigration, phpMigration(root), MIGRATION)],
};
