// Where the server writes the `Core` stub tree, and how the setting that names it reaches the server.
//
// A jump to a `Core` name opens a generated declaration file
// (`rule:ide/the-stub-tree-is-where-core-is-declared`). The server writes that tree wherever
// `nvs.stubs.dir` names, and this client names a directory of its own storage when the user named
// none: beside the copies `binary.ts` keeps, one directory per server version, so an upgrade never
// opens a stale file and nothing is ever written into a workspace. Both functions here are pure, which
// is what lets the headless tier hold them without an editor.

import { join } from "node:path";

/** The directory this client names for server `version`: `<storage>/stubs/<version>`. */
export function stubsDirectory(storage: string, version: string): string {
  return join(storage, "stubs", version.trim() === "" ? "unversioned" : version);
}

/**
 * The `nvs` section as the server receives it: `section` as the user holds it, with `stubs.dir`
 * filled from `fallback` where the user set none. A directory the user wrote is kept exactly, and no
 * other key is touched — the server reads the section nested, as the editor holds it.
 */
export function withStubs(section: unknown, fallback: string): Record<string, unknown> {
  const held = isRecord(section) ? section : {};
  const stubs = isRecord(held.stubs) ? held.stubs : {};
  const written = typeof stubs.dir === "string" && stubs.dir.trim() !== "" ? stubs.dir : fallback;
  return { ...held, stubs: { ...stubs, dir: written } };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
