// What every importer shares: the shape of what it returns, and the few readers of the legacy text
// formats more than one of them needs. An importer reads legacy homes and never writes; `nv import`
// decides what happens to the records it returns.

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import type { RecordType } from "../lib/schema.ts";

/** One record built from a legacy home, before it is validated or written. */
export interface Imported {
  type: RecordType<any>;
  id: string;
  value: unknown;
  /** The repo-relative legacy file it was read from. */
  from: string;
}

/** A legacy file, or a part of one, that did not become a record as it stands, and why. */
export interface Unread {
  path: string;
  reason: string;
}

export interface ImportResult {
  records: Imported[];
  unread: Unread[];
  /** How many legacy files were read. */
  files: number;
}

export interface Importer {
  name: string;
  read(root: string): ImportResult;
}

/** The text of the repo-relative `path`, with LF line ends. */
export function text(root: string, path: string): string {
  return readFileSync(join(root, path), "utf8").replace(/\r\n?/g, "\n");
}

export function exists(root: string, path: string): boolean {
  return existsSync(join(root, path));
}

/** The names in the repo-relative directory `dir`, sorted, or none when it does not exist. */
export function list(root: string, dir: string): string[] {
  try {
    return readdirSync(join(root, dir)).sort();
  } catch {
    return [];
  }
}

/** Every repo-relative file under `dir` whose name ends with `ext`, sorted. */
export function walk(root: string, dir: string, ext: string): string[] {
  const out: string[] = [];
  const go = (sub: string) => {
    for (const e of readdirSync(join(root, sub), { withFileTypes: true })) {
      const path = `${sub}/${e.name}`;
      if (e.isDirectory()) go(path);
      else if (e.name.endsWith(ext)) out.push(path);
    }
  };
  if (exists(root, dir)) go(dir);
  return out.sort();
}

/**
 * A leading `---` block of `key: value` lines, each value the raw rest of its line, and the text
 * after it. `null` when the text does not open on one. Nested YAML is not read here: the one legacy
 * home that nests, a decision's `changes:`, is read by its own importer.
 */
export function frontMatter(src: string): { fields: Map<string, string>; body: string } | null {
  const m = /^---\n([\s\S]*?)\n---\n/.exec(src);
  if (!m) return null;
  const fields = new Map<string, string>();
  for (const line of m[1]!.split("\n")) {
    const kv = /^([A-Za-z][\w-]*):(?: (.*))?$/.exec(line);
    if (kv) fields.set(kv[1]!, kv[2] ?? "");
  }
  return { fields, body: src.slice(m[0].length) };
}

/**
 * The keys of `value` that `known` does not list, as one `Unread` for `path` naming them. A legacy
 * field no record carries is dropped by the import, and this is what makes the drop visible.
 */
export function extraKeys(path: string, what: string, value: object, known: readonly string[]): Unread[] {
  const extra = Object.keys(value).filter((k) => !known.includes(k));
  return extra.length === 0 ? [] : [{ path, reason: `${what} carries ${extra.join(", ")}, which no record holds` }];
}
