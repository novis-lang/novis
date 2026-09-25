// Where things are. Every path the tools print or store is repo-relative with forward slashes, so a
// record, an index row and a finding read the same on Windows and on Linux.

import { join, relative, resolve, sep } from "node:path";

/** The repository root: two directories above this library. */
export const ROOT = resolve(import.meta.dir, "..", "..", "..");

/** The records. Everything under it is JSON that only `store` writes. */
export const DATA = join(ROOT, "data");

/** Runtime state that is never committed: the index, and the tests' scratch trees. */
export const CACHE = join(ROOT, ".cache");

/** Directories that are never an input to a key wherever they appear: build output, caches and
 * machine-local state, every one of them git-ignored. */
export const NOT_INPUTS = new Set([".git", "target", ".loop", ".agent-tmp", "node_modules", "out", ".vscode-test", "__pycache__"]);

/** A repo-relative, forward-slash path for `abs`. */
export function rel(abs: string, root: string = ROOT): string {
  return relative(root, abs).split(sep).join("/");
}

/** The absolute path of a repo-relative `path`, whichever slash it was written with. */
export function abs(path: string, root: string = ROOT): string {
  return resolve(root, ...path.split("/"));
}
