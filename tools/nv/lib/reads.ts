// What a `bun nv` process reads of the tree, recorded so the key of the check that ran it names what it
// read. `tools/nv/keys/checks.ts` builds the key of every `bun nv` check from its last record.
//
// When `NV_READS_LOG` names a file, `main.ts` calls `install` before it loads the command. From then on
// every tools module that imports `node:fs` or `node:fs/promises` gets `reads-fs.ts` or
// `reads-fsp.ts` instead, which note each path a call reads, lists or tests and then make the call.
// `Bun.file` and `Bun.spawn` are wrapped the same way. At exit the process appends one JSON line to the
// log, which also names every tools module Bun's registry holds by then (`readModules`). A `bun nv` process it starts inherits the variable and appends its own line, so one log holds
// every process one check ran, and `readLog` is their union.
//
// A module loaded before `install` is never rewritten, so this module imports `paths.ts` alone, which
// reads nothing. A path outside the repository, or under a directory in `NOT_INPUTS`, is no input
// and is not noted.

import { appendFileSync, readFileSync } from "node:fs";
import { isAbsolute, join, relative, resolve } from "node:path";
import { NOT_INPUTS, ROOT } from "./paths.ts";

export const ENV = "NV_READS_LOG";

/** What one check's processes read, each list sorted: files read whole, paths tested for existence,
 * directories listed, and every program started, as its argument list. */
export interface Reads {
  files: string[];
  exists: string[];
  dirs: string[];
  spawns: string[][];
}

export type Kind = "files" | "exists" | "dirs";

const noted: Record<Kind, Set<string>> = { files: new Set(), exists: new Set(), dirs: new Set() };
const spawned: string[][] = [];

/** `p` as a repo-relative path, or null when it is no input. */
function inTree(p: unknown): string | null {
  let s: string;
  if (typeof p === "string") s = p;
  else if (p instanceof URL) s = p.protocol === "file:" ? decodeURIComponent(p.pathname.replace(/^\/([A-Za-z]:)/, "$1")) : "";
  else if (p instanceof Uint8Array) s = new TextDecoder().decode(p);
  else return null;
  if (s === "") return null;
  const r = relative(ROOT, isAbsolute(s) ? s : resolve(process.cwd(), s)).replace(/\\/g, "/");
  if (r.startsWith("..") || isAbsolute(r)) return null;
  if (r === "") return ".";
  return r.split("/").some((part) => NOT_INPUTS.has(part)) ? null : r;
}

/** Notes that this process read, listed or tested `p`. */
export function note(kind: Kind, p: unknown): void {
  const r = inTree(p);
  if (r !== null) noted[kind].add(r);
}

/** Notes a program this process started. */
export function noteSpawn(cmd: unknown): void {
  const argv = Array.isArray(cmd) ? cmd : cmd && typeof cmd === "object" && Array.isArray((cmd as { cmd?: unknown }).cmd) ? (cmd as { cmd: unknown[] }).cmd : null;
  if (argv) spawned.push(argv.map(String));
}

/** A wrapper of `f` that notes its first argument as `kind` before it calls `f`. */
export function reading<F extends (...args: never[]) => unknown>(kind: Kind, f: F): F {
  return function (this: unknown, ...args: Parameters<F>) {
    note(kind, args[0]);
    return f.apply(this, args);
  } as F;
}

const SHIMS = [join(import.meta.dir, "reads-fs.ts"), join(import.meta.dir, "reads-fsp.ts")];
const SPECIFIER = /(\bfrom\s*|\bimport\s*\(\s*|\bimport\s+)(["'])(?:node:)?fs(\/promises)?\2/g;

/** Starts recording into `log`: every tools module loaded from here on is rewritten, and the process
 * appends its reads to `log` when it exits. */
export function install(log: string): void {
  const [fsShim, fspShim] = SHIMS.map((s) => JSON.stringify(s.replace(/\\/g, "/")));
  Bun.plugin({
    name: "nv-reads",
    setup(build) {
      build.onLoad({ filter: /[\\/]tools[\\/]nv[\\/].+\.ts$/ }, (args) => {
        const text = readFileSync(args.path, "utf8");
        if (SHIMS.some((s) => resolve(s) === resolve(args.path))) return { contents: text, loader: "ts" };
        return { contents: text.replace(SPECIFIER, (_m, head: string, _q: string, promises?: string) => `${head}${promises ? fspShim : fsShim}`), loader: "ts" };
      });
    },
  });
  const file = Bun.file;
  (Bun as { file: unknown }).file = function (this: unknown, p: unknown, ...rest: unknown[]) {
    note("files", p);
    return (file as (...a: unknown[]) => unknown).call(Bun, p, ...rest);
  };
  for (const name of ["spawn", "spawnSync"] as const) {
    const f = Bun[name] as (...a: unknown[]) => unknown;
    (Bun as Record<string, unknown>)[name] = function (this: unknown, cmd: unknown, ...rest: unknown[]) {
      noteSpawn(cmd);
      return f.call(Bun, cmd, ...rest);
    };
  }
  process.on("exit", () => {
    const modules = Object.keys(require.cache)
      .map((m) => inTree(m))
      .filter((m): m is string => m !== null && m.endsWith(".ts"))
      .sort();
    const line = { files: [...noted.files].sort(), exists: [...noted.exists].sort(), dirs: [...noted.dirs].sort(), spawns: spawned, modules };
    try {
      appendFileSync(log, `${JSON.stringify(line)}\n`);
    } catch {
      // A record that cannot be written leaves the check keyed on everything, which is never unsafe.
    }
  });
}

/** The TypeScript modules every process of the log at `path` had loaded when it exited, repo-relative
 * and sorted: Bun's module registry, read at exit, so a module loaded by a dynamic import counts only
 * when it was loaded. Empty when the log holds none. */
export function readModules(path: string): string[] {
  const out = new Set<string>();
  let text = "";
  try {
    text = readFileSync(path, "utf8");
  } catch {
    return [];
  }
  for (const line of text.split("\n")) {
    if (!line.trim()) continue;
    try {
      for (const m of (JSON.parse(line) as { modules?: string[] }).modules ?? []) out.add(m);
    } catch {
      return [];
    }
  }
  return [...out].sort();
}

/** The union of every line in the log at `path`, or null when it holds none. */
export function readLog(path: string): Reads | null {
  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch {
    return null;
  }
  const out = { files: new Set<string>(), exists: new Set<string>(), dirs: new Set<string>(), spawns: new Map<string, string[]>() };
  let lines = 0;
  for (const line of text.split("\n")) {
    if (!line.trim()) continue;
    let got: Partial<Reads>;
    try {
      got = JSON.parse(line);
    } catch {
      return null;
    }
    lines++;
    for (const k of ["files", "exists", "dirs"] as const) for (const p of got[k] ?? []) out[k].add(p);
    for (const argv of got.spawns ?? []) out.spawns.set(JSON.stringify(argv), argv);
  }
  if (lines === 0) return null;
  return { files: [...out.files].sort(), exists: [...out.exists].sort(), dirs: [...out.dirs].sort(), spawns: [...out.spawns.values()] };
}
