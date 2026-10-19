// The kit's copy in a project (decision 0027): sync writes the kit that runs into the project's
// `tools/looper/`, with a manifest of the kit's version and a hash per file, and a shim for each shell
// at the root. Lint reads the manifest to refuse a copy edited by hand or one that differs from the kit.
//
// The copy holds the kit's `main.ts`, its `package.json`, which carries its version, every file of
// `src/` but the tests, and the templates bundled in one `templates.json`. So the copy reads its own
// templates, and runs as `bun tools/looper/main.ts <command>` with no `bun install`.
//
// The manifest `tools/looper/looper-manifest.json` maps each file the copy writes, from the project
// root and with `/`, to the SHA-256 of its text with each `\r\n` read as `\n`. So a checkout that turns
// line endings keeps its hashes. The manifest names the shims too, but not itself.
//
// `copyPlan` compares a project with the kit at `kit` and returns one `Change` per file to write or
// remove, the manifest last. `copyIssues` is the lint rule:
//
// - A project with no manifest never took the copy, and is not checked.
// - A file the manifest names that is missing or hashes otherwise, and a file under `tools/looper/`
//   it does not name, are edits by hand.
// - With no edit by hand, a manifest that differs from the kit's own is a copy behind the kit.
//
// Both hold one copy of the kit's text, a fixed set bounded by the kit's size.

import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import type { Issue } from "../goal/lint.ts";
import { BUNDLE, templateEntries } from "./template.ts";

/** Where the copy stands in a project. */
export const COPY = "tools/looper";
/** The copy's manifest, named so no one takes it for the project's own. */
export const MANIFEST = `${COPY}/looper-manifest.json`;

/** The shims at the project's root, each running the copy with its arguments: Git Bash's, then cmd's. */
export const SHIMS: readonly (readonly [string, string])[] = [
  ["loop", '#!/bin/sh\n# Runs this project\'s copy of looper\'s kit. Sync writes this file.\nexec bun "$(dirname "$0")/tools/looper/main.ts" "$@"\n'],
  ["loop.cmd", '@echo off\r\nrem Runs this project\'s copy of looper\'s kit. Sync writes this file.\r\nbun "%~dp0tools\\looper\\main.ts" %*\r\n'],
];

/** The kit's version and the hash of each file the copy writes, by its path from the project root. */
export interface Manifest {
  version: string;
  files: Record<string, string>;
}

/** One file of the copy to write, or to remove when `after` is `undefined`. */
export interface CopyChange {
  path: string;
  before: string | undefined;
  after: string | undefined;
}

/** `text` with each `\r\n` read as `\n`, as the copy compares and hashes it. */
function lf(text: string): string {
  return text.replaceAll("\r\n", "\n");
}

/** The SHA-256 of `text` with its line endings read as `\n`, in hex. */
export function hash(text: string): string {
  return createHash("sha256").update(lf(text)).digest("hex");
}

/** Every file below `dir`, as `[its path with prefix and /, its text]`, sorted by path. */
function walk(dir: string, prefix: string, keep: (rel: string) => boolean): [string, string][] {
  const out: [string, string][] = [];
  const visit = (at: string, rel: string) => {
    for (const name of readdirSync(at)) {
      const path = join(at, name);
      if (statSync(path).isDirectory()) visit(path, `${rel}${name}/`);
      else if (keep(`${rel}${name}`)) out.push([`${prefix}${rel}${name}`, readFileSync(path, "utf8")]);
    }
  };
  visit(dir, "");
  return out.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
}

/** The version in the `package.json` of the kit at `kit`. */
export function kitVersion(kit: string): string {
  return (JSON.parse(readFileSync(join(kit, "package.json"), "utf8")) as { version: string }).version;
}

/** Every file the copy of the kit at `kit` writes into a project, by its path there, the shims last. */
export function kitFiles(kit: string): [string, string][] {
  const out: [string, string][] = [];
  for (const name of ["main.ts", "package.json"]) out.push([`${COPY}/${name}`, readFileSync(join(kit, name), "utf8")]);
  out.push(...walk(join(kit, "src"), `${COPY}/src/`, (rel) => !rel.endsWith(".test.ts")));
  out.push([`${COPY}/${BUNDLE}`, `${JSON.stringify(templateEntries(kit), null, 1)}\n`]);
  out.push(...SHIMS.map(([path, text]) => [path, text] as [string, string]));
  return out;
}

/** The manifest of `files` for a kit of `version`. */
export function manifestOf(version: string, files: [string, string][]): Manifest {
  const hashes: Record<string, string> = {};
  for (const [path, text] of files) hashes[path] = hash(text);
  return { version, files: hashes };
}

/** The manifest's text as sync writes it. */
function manifestText(m: Manifest): string {
  return `${JSON.stringify(m, null, 2)}\n`;
}

/** The file at `path`, or `undefined` when it does not exist. */
function readOrUndefined(path: string): string | undefined {
  try {
    return readFileSync(path, "utf8");
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") return undefined;
    throw e;
  }
}

/** The project's manifest at `root`, `undefined` without one. It throws on a manifest it cannot read. */
export function readManifest(root: string): Manifest | undefined {
  const text = readOrUndefined(join(root, MANIFEST));
  if (text === undefined) return undefined;
  let m: Manifest;
  try {
    m = JSON.parse(text);
  } catch (e) {
    throw new Error(`${MANIFEST}: not JSON: ${(e as Error).message}`);
  }
  if (typeof m?.version !== "string" || typeof m.files !== "object" || m.files === null) {
    throw new Error(`${MANIFEST}: it needs a \`version\` and a \`files\` map`);
  }
  return m;
}

/**
 * Every change that makes the project at `root` hold the copy of the kit at `kit`: each file whose text
 * differs, each file the old manifest names and the kit no longer has, and the manifest last.
 */
export function copyPlan(root: string, kit: string): CopyChange[] {
  const files = kitFiles(kit);
  const changes: CopyChange[] = [];
  for (const [path, after] of files) {
    const before = readOrUndefined(join(root, path));
    if (before === undefined || lf(before) !== lf(after)) changes.push({ path, before, after });
  }
  let old: Manifest | undefined;
  try {
    old = readManifest(root);
  } catch {
    // A manifest that cannot be read names no file to remove, and sync rewrites it.
  }
  const kept = new Set(files.map(([path]) => path));
  for (const path of Object.keys(old?.files ?? {}).sort()) {
    if (kept.has(path)) continue;
    const before = readOrUndefined(join(root, path));
    if (before !== undefined) changes.push({ path, before, after: undefined });
  }
  const after = manifestText(manifestOf(kitVersion(kit), files));
  const before = readOrUndefined(join(root, MANIFEST));
  if (before === undefined || lf(before) !== after) changes.push({ path: MANIFEST, before, after });
  return changes;
}

/** Whether `a` and `b` name the same files with the same hashes. */
function sameFiles(a: Record<string, string>, b: Record<string, string>): boolean {
  const keys = Object.keys(a);
  return keys.length === Object.keys(b).length && keys.every((k) => b[k] === a[k]);
}

/** What lint finds wrong with the copy in the project at `root`, compared with the kit at `kit`. */
export function copyIssues(root: string, kit: string): Issue[] {
  const issue = (file: string, message: string): Issue => ({ file, line: undefined, message });
  let have: Manifest | undefined;
  try {
    have = readManifest(root);
  } catch (e) {
    return [issue(MANIFEST, `${(e as Error).message}: run sync`)];
  }
  if (have === undefined) return [];
  const out: Issue[] = [];
  for (const [path, want] of Object.entries(have.files).sort(([a], [b]) => (a < b ? -1 : 1))) {
    const text = readOrUndefined(join(root, path));
    if (text === undefined) out.push(issue(path, "the kit's copy lacks this file: run sync"));
    else if (hash(text) !== want) out.push(issue(path, "the kit's copy was edited by hand: change the kit in looper, then run sync"));
  }
  for (const [path] of walk(join(root, COPY), `${COPY}/`, () => true)) {
    if (path !== MANIFEST && !Object.hasOwn(have.files, path)) {
      out.push(issue(path, "the kit's copy holds a file the kit lacks: move it out of tools/looper/, or run sync"));
    }
  }
  if (out.length > 0) return out;
  const own = manifestOf(kitVersion(kit), kitFiles(kit));
  if (have.version !== own.version || !sameFiles(have.files, own.files)) {
    out.push(issue(MANIFEST, `the kit's copy is v${have.version}, and differs from the kit's v${own.version}: run sync`));
  }
  return out;
}
