// The driver's runtime state: `.loop/state.sqlite`, git-ignored and never committed. It holds the
// chain pointer, the slug of the goal the run has installed. A goal switch writes the pointer and
// copies no file, so the slug is the whole answer to "which goal is live".
//
// Until the cutover, the Python driver's `.loop/chain.json` is the pointer instead. It names the goal
// by the number in its file name, `N-<slug>.md`, and `legacyPointer` turns that number into the slug.
// The cutover's `adoptLegacyPointer` writes that slug into `STATE`, and from then on `cutOver` is true.

import { Database } from "bun:sqlite";
import { existsSync, mkdirSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { ROOT } from "./paths.ts";

export const STATE = ".loop/state.sqlite";
/** The Python driver's pointer, `{"goal": N}`, read until the cutover moves it into `STATE`. */
export const LEGACY_POINTER = ".loop/chain.json";

const POINTER_TABLE = "CREATE TABLE IF NOT EXISTS pointer (id INTEGER PRIMARY KEY CHECK (id = 1), goal TEXT NOT NULL)";

/** The slug `STATE` points at, or null when the file, its table or its row is absent. */
export function readPointer(root: string = ROOT): string | null {
  const file = join(root, STATE);
  if (!existsSync(file)) return null;
  let db: Database | undefined;
  try {
    db = new Database(file, { readonly: true });
    const row = db.query("SELECT goal FROM pointer WHERE id = 1").get() as { goal: string } | null;
    return row?.goal || null;
  } catch {
    return null;
  } finally {
    db?.close();
  }
}

/** Points `STATE` at `slug`, creating the file and its table when they are absent. */
export function writePointer(slug: string, root: string = ROOT): void {
  const file = join(root, STATE);
  mkdirSync(dirname(file), { recursive: true });
  const db = new Database(file, { create: true });
  try {
    db.run(POINTER_TABLE);
    db.query("INSERT INTO pointer (id, goal) VALUES (1, ?1) ON CONFLICT (id) DO UPDATE SET goal = excluded.goal").run(slug);
  } finally {
    db.close();
  }
}

/** The slug of the goal `LEGACY_POINTER` names by number, or null when it names none on disk. */
export function legacyPointer(root: string = ROOT): string | null {
  const file = join(root, LEGACY_POINTER);
  if (!existsSync(file)) return null;
  let num: unknown;
  try {
    num = (JSON.parse(readFileSync(file, "utf8")) as { goal?: unknown }).goal;
  } catch {
    return null;
  }
  if (typeof num !== "number" || !Number.isInteger(num) || num <= 0) return null;
  for (const sub of ["docs/agent/goals/dossier", "docs/agent/goals"]) {
    const dir = join(root, sub);
    if (!existsSync(dir)) continue;
    for (const name of readdirSync(dir)) {
      const m = /^(\d+)-([a-z0-9]+(?:-[a-z0-9]+)*)\.md$/.exec(name);
      if (m && Number(m[1]) === num) return m[2]!;
    }
  }
  return null;
}

/** The slug the driver has installed: `STATE`'s pointer, else `LEGACY_POINTER`'s, else null. */
export function pointerSlug(root: string = ROOT): string | null {
  return readPointer(root) ?? legacyPointer(root);
}

/**
 * Whether the run belongs to `bun nv loop`: `STATE` holds a pointer. Before the cutover only the Python
 * driver runs, and nothing but `adoptLegacyPointer` writes the pointer, so its presence is the switch
 * every tool that still serves the Python driver reads.
 */
export function cutOver(root: string = ROOT): boolean {
  return readPointer(root) !== null;
}

/**
 * The cutover's pointer step: `LEGACY_POINTER`'s goal becomes `STATE`'s pointer, and the slug is returned.
 * It reads the numbered goal file names, so it runs before the cutover renames them. Null, with nothing
 * written, when the legacy pointer names no goal on disk.
 */
export function adoptLegacyPointer(root: string = ROOT): string | null {
  const slug = legacyPointer(root);
  if (slug) writePointer(slug, root);
  return slug;
}
