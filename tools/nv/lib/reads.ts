// What a `bun nv` process reads of the tree, recorded so the footprint of the atom that ran it names what
// it read: `select/seed.ts` `nvKeys` turns a log into keys for a `bun nv` check, a verify step and a
// tools test file.
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
//
// One process can answer several checks at once, as a batched `nv proofs --verify` does for each of its
// groups. What it reads inside `inPart` is noted under those parts alone, and `readLog` with a part
// gives what everything outside any part read together with what that part read. Without a part it gives
// the union of everything, as a check that runs the process alone reads it.
//
// A command's bookkeeping is kept out the same way. `unrecorded` keeps out what it reads, and
// `loadUnrecorded` keeps out the modules it loads: every module Bun's registry gains while it runs is
// left out of the modules the command's line names, so a change to the selection engine that
// `nv proofs --verify` loads to skip unreached programs does not reach every proofs check. A
// `bun test` process names every module it loaded (`reads-preload.ts`), since a test's verdict reads
// whatever its code runs.

import { AsyncLocalStorage } from "node:async_hooks";
import { appendFileSync, readFileSync } from "node:fs";
import { isAbsolute, join, relative, resolve } from "node:path";
import { NOT_INPUTS, ROOT } from "./paths.ts";

export const ENV = "NV_READS_LOG";

/** What one check's processes read, each list sorted: files read whole, paths tested for existence,
 * directories listed, every program started, as its argument list, and the keys a reader named itself
 * for what it read `unrecorded`. */
export interface Reads {
  files: string[];
  exists: string[];
  dirs: string[];
  spawns: string[][];
  keys?: string[];
}

export type Kind = "files" | "exists" | "dirs";

/** What one process, or one part of it, noted. */
interface Noted {
  files: Set<string>;
  exists: Set<string>;
  dirs: Set<string>;
  spawns: string[][];
  keys: Set<string>;
}

const fresh = (): Noted => ({ files: new Set(), exists: new Set(), dirs: new Set(), spawns: [], keys: new Set() });
const common = fresh();
const parts = new Map<string, Noted>();
/** Set inside `unrecorded`, and carried across every `await` of what it runs. */
const quiet = new AsyncLocalStorage<true>();
/** Set inside `inPart`, and carried the same way. */
const inside = new AsyncLocalStorage<readonly string[]>();

/** Where a note made here goes: the parts of the enclosing `inPart`, or the process as a whole. */
function targets(): Noted[] {
  const labels = inside.getStore();
  if (labels === undefined || labels.length === 0) return [common];
  return labels.map((l) => parts.get(l) ?? parts.set(l, fresh()).get(l)!);
}

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
  if (quiet.getStore()) return;
  const r = inTree(p);
  if (r !== null) for (const t of targets()) t[kind].add(r);
}

/** Runs `f` with nothing it reads, lists, tests or starts noted. The caller names what it read with
 * `noteKey` instead, as a key the change moves exactly when that answer can change, or names nothing for
 * work that is no input to its verdict. The scope is asynchronous: when `f` returns a promise, every
 * continuation of the work `f` started stays unrecorded, and work that runs beside it outside `f` is
 * noted as before. */
export function unrecorded<T>(f: () => T): T {
  return quiet.run(true, f);
}

/** Every module `loadUnrecorded` brought into Bun's registry, as `require.cache` names it. */
const unnoted = new Set<string>();
/** Set by `install` for a `bun test` process, whose line names every module it loaded. */
let everyModule = false;

/**
 * Runs `load`, which imports code that is no input to this process's verdict, `unrecorded`, and leaves
 * every module Bun's registry gains while it runs out of the modules `flush` names. A module already
 * loaded stays named, so one that the verdict's own code imports statically is kept whatever `load`
 * imports. What is left out is decided by when a module first loads, so `load` imports only what the
 * command's bookkeeping alone uses: a module it loads first and that code judging the verdict imports
 * later, dynamically, is not named. Nothing else of the process should load a module while `load`
 * runs; the window is the promise `load` returns.
 */
export async function loadUnrecorded<T>(load: () => Promise<T>): Promise<T> {
  const before = new Set(Object.keys(require.cache));
  try {
    return await unrecorded(load);
  } finally {
    for (const m of Object.keys(require.cache)) if (!before.has(m)) unnoted.add(m);
  }
}

/** Runs `f` with everything it reads, lists, tests, starts and names noted under each of `labels`
 * instead of the process as a whole. The scope is asynchronous, as `unrecorded`'s is, and inside
 * `unrecorded` nothing is noted at all. An empty `labels` notes for the process as a whole. */
export function inPart<T>(labels: readonly string[], f: () => T): T {
  return inside.run(labels, f);
}

/** Notes that this process depends on `key`, a key a change moves by what the text it read names. */
export function noteKey(key: string): void {
  for (const t of targets()) t.keys.add(key);
}

/** Notes a program this process started. One started in a directory that is no input (a test's scratch
 * tree under `.cache`, say) reads nothing of the tree through its working directory, and is not noted;
 * nor is one started inside `unrecorded`. */
export function noteSpawn(cmd: unknown, opts?: unknown): void {
  if (quiet.getStore()) return;
  const argv = Array.isArray(cmd) ? cmd : cmd && typeof cmd === "object" && Array.isArray((cmd as { cmd?: unknown }).cmd) ? (cmd as { cmd: unknown[] }).cmd : null;
  const options = (Array.isArray(cmd) ? opts : cmd) as { cwd?: unknown } | undefined;
  const cwd = options && typeof options === "object" ? options.cwd : undefined;
  if (typeof cwd === "string" && inTree(cwd) === null) return;
  if (argv) for (const t of targets()) t.spawns.push(argv.map(String));
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
 * appends its reads to `log` when it exits. With `tests`, the process is a `bun test` run, and its line
 * names every module it loaded, `loadUnrecorded`'s too. */
export function install(log: string, tests = false): void {
  everyModule = tests;
  // The shim's path goes in with the quote the specifier had, so a specifier spelled inside another
  // string literal leaves that literal whole.
  const [fsShim, fspShim] = SHIMS.map((s) => s.replace(/\\/g, "/"));
  Bun.plugin({
    name: "nv-reads",
    setup(build) {
      build.onLoad({ filter: /[\\/]tools[\\/]nv[\\/].+\.ts$/ }, (args) => {
        const text = readFileSync(args.path, "utf8");
        if (SHIMS.some((s) => resolve(s) === resolve(args.path))) return { contents: text, loader: "ts" };
        return { contents: text.replace(SPECIFIER, (_m, head: string, q: string, promises?: string) => `${head}${q}${promises ? fspShim : fsShim}${q}`), loader: "ts" };
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
      noteSpawn(cmd, rest[0]);
      return f.call(Bun, cmd, ...rest);
    };
  }
  process.on("exit", () => flush(log));
}

let flushed = false;

const listed = (n: Noted): Reads => ({ files: [...n.files].sort(), exists: [...n.exists].sort(), dirs: [...n.dirs].sort(), spawns: n.spawns, keys: [...n.keys].sort() });

/** Appends this process's line to `log`, once: what it read, listed, tested and started, what each part
 * read besides under `parts`, and every tools module Bun's registry holds by then other than those
 * `loadUnrecorded` loaded. The exit handler calls it; `bun test` fires no exit handler, so
 * `reads-preload.ts` calls it after the last test. */
export function flush(log: string): void {
  if (flushed) return;
  flushed = true;
  const modules = Object.keys(require.cache)
    .filter((m) => everyModule || !unnoted.has(m))
    .map((m) => inTree(m))
    .filter((m): m is string => m !== null && m.endsWith(".ts"))
    .sort();
  const line: Reads & { modules: string[]; parts?: Record<string, Reads> } = { ...listed(common), modules };
  if (parts.size > 0) line.parts = Object.fromEntries([...parts].map(([l, n]) => [l, listed(n)]));
  try {
    appendFileSync(log, `${JSON.stringify(line)}\n`);
  } catch {
    // A record that cannot be written leaves the check keyed on everything, which is never unsafe.
  }
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

/** The union of every line in the log at `path`, or null when it holds none. With `part`, a line's parts
 * other than `part` are left out; without it, every part is in the union. */
export function readLog(path: string, part?: string): Reads | null {
  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch {
    return null;
  }
  const out = { files: new Set<string>(), exists: new Set<string>(), dirs: new Set<string>(), keys: new Set<string>(), spawns: new Map<string, string[]>() };
  let lines = 0;
  for (const line of text.split("\n")) {
    if (!line.trim()) continue;
    let got: Partial<Reads> & { parts?: Record<string, Partial<Reads>> };
    try {
      got = JSON.parse(line);
    } catch {
      return null;
    }
    lines++;
    const parts = Object.entries(got.parts ?? {}).filter(([l]) => part === undefined || l === part);
    for (const one of [got, ...parts.map(([, r]) => r)]) {
      for (const k of ["files", "exists", "dirs", "keys"] as const) for (const p of one[k] ?? []) out[k].add(p);
      for (const argv of one.spawns ?? []) out.spawns.set(JSON.stringify(argv), argv);
    }
  }
  if (lines === 0) return null;
  return { files: [...out.files].sort(), exists: [...out.exists].sort(), dirs: [...out.dirs].sort(), spawns: [...out.spawns.values()], keys: [...out.keys].sort() };
}
