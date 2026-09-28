// The keys an edit of the repository's own `nvs.toml` moves.
//
// Every program run from the repository root reads that file, and dossier goals add an `[[app]]` block to
// it for nearly every example they write. A program therefore records it by part (`nvs_footprint`'s
// `config` and `app` lines): `config:nvs.toml` for the global tables, and `app:<path>` for its entry file
// and each directory above it, the paths a block that applies to it can be keyed on. An edit moves
// `config:nvs.toml` when anything outside the `[[app]]` blocks changed, and `app:<path>` with `app:*` for
// each block keyed on `<path>` that was added, removed or changed. A block is found by the path it is
// keyed on, resolved against the file's directory, so moving a block within the file, or writing its path
// another way, moves nothing: which blocks apply to an entry file, and in which order, depends on their
// paths and never on their place in the file. Each side is read twice, parsed and as lines, and a part
// either reading finds changed moves (`textParts` says why).
//
// What an edit can do to a program no block of it applies to is make the roster fail to resolve, which
// stops every program alike. The commonest way is a block keyed on a path that is not there, so a block
// an edit moved, or whose path a deleted file was or sat under, is looked for, and the file moves whole
// when it is gone (`rootConfigKeys` in `select.ts`). Every other way, an unknown directive in a block or
// a limit above the global ceiling, the Rust test that resolves this file whole finds: it holds
// `tree:nvs.toml` and the path of every block, so it runs on every edit and every block path that goes.
//
// Anything this cannot read moves `config:nvs.toml`, which every reader holds: a side that is not there
// or not known, text that does not parse, an `app` that is not a list of tables, and a block keyed on
// both paths, on neither, or on a path another block has, or with no one line that writes its key.

import { posix } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { appKey, configKey, EVERY_APP, repoPath } from "./keys.ts";

/** One side of the file, read: its global tables as one canonical text, and each block's canonical text
 * by the key of the path it is keyed on. */
interface Parts {
  global: string;
  apps: Map<string, string>;
}

/** `value` as JSON with every table's keys sorted, so two readings of one TOML value are one text. */
function canonical(value: unknown): string {
  if (value instanceof Date) return JSON.stringify(value.toISOString());
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value !== null && typeof value === "object") {
    const entries = Object.keys(value as Record<string, unknown>)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${canonical((value as Record<string, unknown>)[k])}`);
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value);
}

const isTable = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === "object" && !Array.isArray(v) && !(v instanceof Date);

/** The path a block written in `file` names, resolved against the file's directory as the configuration
 * resolves it: repo-relative, or absolute when it is outside the tree or under a directory that is never
 * an input. */
function blockPath(file: string, written: string, root: string): string {
  const p = written.replace(/\\/g, "/");
  const absolute = p.startsWith("/") || /^[A-Za-z]:\//.test(p);
  const joined = absolute ? p : posix.join(root.replace(/\\/g, "/"), posix.dirname(file), p);
  return repoPath(joined, root) ?? posix.normalize(joined);
}

/** The `app:` key of the path a block written in `file` names. */
const blockKey = (file: string, written: string, root: string) => {
  const p = blockPath(file, written, root);
  return appKey(p.startsWith("/") || /^[A-Za-z]:\//.test(p) ? null : p);
};

/** Each block of `file`'s text by its `app:` key, with the path it is keyed on (`blockPath`); null when
 * the text cannot be read by part. */
export function blockPaths(file: string, text: string, root: string = ROOT): Map<string, string> | null {
  let table: unknown;
  try {
    table = Bun.TOML.parse(text);
  } catch {
    return null;
  }
  if (!isTable(table) || (table.app !== undefined && !Array.isArray(table.app))) return null;
  const out = new Map<string, string>();
  for (const block of (table.app ?? []) as unknown[]) {
    if (!isTable(block)) return null;
    const written = typeof block.root === "string" ? block.root : typeof block.entry === "string" ? block.entry : null;
    if (written === null) return null;
    out.set(blockKey(file, written, root), blockPath(file, written, root));
  }
  return out;
}

/** One side of `file`, or null when it cannot be read by part. */
function parts(file: string, text: string, root: string): Parts | null {
  let table: unknown;
  try {
    table = Bun.TOML.parse(text);
  } catch {
    return null;
  }
  if (!isTable(table)) return null;
  const { app, ...rest } = table;
  const apps = new Map<string, string>();
  if (app !== undefined) {
    if (!Array.isArray(app)) return null;
    for (const block of app) {
      if (!isTable(block)) return null;
      const { root: dir, entry } = block;
      const written = typeof dir === "string" && entry === undefined ? dir : typeof entry === "string" && dir === undefined ? entry : null;
      if (written === null) return null;
      const key = blockKey(file, written, root);
      if (apps.has(key)) return null;
      const { root: _root, entry: _entry, ...directives } = block;
      apps.set(key, canonical(directives));
    }
  }
  return { global: canonical(rest), apps };
}

/** A header that starts a table of the `[[app]]` block above it. */
const APP_TABLE = /^\[\s*app\s*\./;
/** The line that keys a block, and the path it writes. */
const KEY_LINE = /^(root|entry)\s*=\s*(?:"([^"]*)"|'([^']*)')\s*(?:#.*)?$/;

/**
 * One side of `file` read as text rather than parsed, or null when a block has no key line or two blocks
 * share one: each part's lines without blank and comment lines, sorted. `Bun.TOML` reads some text the
 * configuration refuses, and a parser that read two different texts as one value would move no key, so
 * the lines are compared as well and a part whose lines changed moves too. A line this puts in the wrong
 * part moves that part instead, which selects more and never less.
 */
function textParts(file: string, text: string, root: string): Parts | null {
  const global: string[] = [];
  const blocks: string[][] = [];
  let block: string[] | null = null;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) continue;
    if (/^\[\[\s*app\s*\]\]/.test(line)) {
      block = [];
      blocks.push(block);
    } else if (line.startsWith("[") && !(block !== null && APP_TABLE.test(line))) {
      block = null;
      global.push(line);
    } else (block ?? global).push(line);
  }
  const apps = new Map<string, string>();
  for (const lines of blocks) {
    const keyed = lines.map((l) => KEY_LINE.exec(l)).filter((m) => m !== null);
    if (keyed.length !== 1) return null;
    const key = blockKey(file, keyed[0]![2] ?? keyed[0]![3]!, root);
    if (apps.has(key)) return null;
    apps.set(key, lines.filter((l) => !KEY_LINE.test(l)).sort().join("\n"));
  }
  return { global: global.sort().join("\n"), apps };
}

/** The keys of the parts that differ between two readings of one file. */
function moved(file: string, was: Parts, now: Parts): Set<string> {
  const keys = new Set<string>();
  if (was.global !== now.global) keys.add(configKey(file));
  for (const k of new Set([...was.apps.keys(), ...now.apps.keys()])) if (was.apps.get(k) !== now.apps.get(k)) keys.add(k);
  return keys;
}

/**
 * The keys an edit of the configuration `file` moves, from its text before and after; null for a side
 * that is not there or whose text is not known. `file` is repo-relative.
 */
export function configKeys(file: string, before: string | null, after: string | null, root: string = ROOT): string[] {
  const whole = [configKey(file)];
  if (before === null || after === null) return whole;
  const readings = [parts, textParts].map((read) => [read(file, before, root), read(file, after, root)] as const);
  if (readings.some(([was, now]) => was === null || now === null)) return whole;
  const keys = new Set(readings.flatMap(([was, now]) => [...moved(file, was!, now!)]));
  const blocks = [...keys].filter((k) => k !== configKey(file)).sort();
  return [...(keys.has(configKey(file)) ? whole : []), ...(blocks.length > 0 ? [EVERY_APP, ...blocks] : [])];
}
