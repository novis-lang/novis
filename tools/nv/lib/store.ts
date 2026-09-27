// The records' loader and their writer. This file holds the only code that writes a record, so every
// record on disk has one text for one value: keys in schema order (a map's keys sorted), two-space
// indent, LF line ends and one trailing newline. That is what lets git merge two edits to one record
// line by line, and what makes "load then write" change nothing.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { ROOT, rel } from "./paths.ts";
import { plainIdOf, SchemaError, type Issue, type RecordType } from "./schema.ts";

export interface Loaded<T> {
  type: RecordType<T>;
  id: string;
  /** Repo-relative. */
  path: string;
  /** The parsed JSON. It is a `T` only when `issues` is empty. */
  value: T;
  issues: Issue[];
}

export function dataDir(root: string = ROOT): string {
  return join(root, "data");
}

/** The repo-relative path of the record `id` of `type`. */
export function pathOf(type: RecordType<any>, id: string): string {
  return type.single ? `data/${type.dir}.json` : `data/${type.dir}/${id}${type.suffix ?? ".json"}`;
}

/** The id of the record at repo-relative `path` if it is one of `type`'s, else null. */
export function idAt(type: RecordType<any>, path: string): string | null {
  if (type.single) return path === `data/${type.dir}.json` ? type.name : null;
  const prefix = `data/${type.dir}/`;
  if (!path.startsWith(prefix)) return null;
  return (type.idOf ?? plainIdOf)(path.slice(prefix.length));
}

/** The text the writer puts on disk for `value`. `value` must already be valid. */
export function serialize<T>(type: RecordType<T>, value: T): string {
  return JSON.stringify(type.schema.ordered(value), null, 2) + "\n";
}

/** Parses and validates one record file. A file that is not JSON is one issue, not an exception. */
export function loadFile<T>(type: RecordType<T>, path: string, root: string = ROOT): Loaded<T> {
  const id = idAt(type, path);
  if (id === null) throw new Error(`${path} is not a ${type.name} record`);
  const text = readFileSync(join(root, path), "utf8");
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (e) {
    return { type, id, path, value: undefined as T, issues: [{ at: "", message: `is not JSON: ${(e as Error).message}` }] };
  }
  const issues = type.schema.validate(value);
  if (issues.length === 0 && serialize(type, value as T) !== text) {
    issues.push({ at: "", message: "is not in the writer's layout; `bun nv` rewrites it when it next writes this record" });
  }
  return { type, id, path, value: value as T, issues };
}

/** Every repo-relative `.json` path under `data/`, or under `data/<under>/` alone, sorted. */
export function recordFiles(root: string = ROOT, under = ""): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      const full = join(dir, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.name.endsWith(".json")) out.push(rel(full, root));
    }
  };
  walk(under ? join(dataDir(root), under) : dataDir(root));
  return out.sort();
}

/** Every record of `type` on disk. Only the type's own directory is listed, so what a command read is
 * the records it loaded and not the names of every other type's. */
export function load<T>(type: RecordType<T>, root: string = ROOT): Loaded<T>[] {
  if (type.single) {
    const path = pathOf(type, type.name);
    return existsSync(join(root, path)) ? [loadFile(type, path, root)] : [];
  }
  return recordFiles(root, type.dir)
    .filter((p) => idAt(type, p) !== null)
    .map((p) => loadFile(type, p, root));
}

/**
 * Writes the record `id` of `type`, after validating it. A value that fails its schema is never
 * written. Returns the repo-relative path, and whether the file's text changed.
 */
export function write<T>(type: RecordType<T>, id: string, value: T, root: string = ROOT): { path: string; changed: boolean } {
  const issues = type.schema.validate(value);
  if (issues.length > 0) throw new SchemaError(issues);
  const path = pathOf(type, id);
  if (idAt(type, path) !== (type.single ? type.name : id)) throw new Error(`${id} is not a valid ${type.name} id`);
  const full = join(root, path);
  const text = serialize(type, value);
  let before: string | null = null;
  try {
    before = readFileSync(full, "utf8");
  } catch {
    before = null;
  }
  if (before === text) return { path, changed: false };
  mkdirSync(dirname(full), { recursive: true });
  writeFileSync(full, text);
  return { path, changed: true };
}

/** Deletes the record `id` of `type`. Returns whether there was one. */
export function remove(type: RecordType<any>, id: string, root: string = ROOT): boolean {
  const full = join(root, pathOf(type, id));
  try {
    statSync(full);
  } catch {
    return false;
  }
  rmSync(full);
  return true;
}
