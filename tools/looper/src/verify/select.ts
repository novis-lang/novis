// The unit store and the selection over it: which verify units a change reaches. It ports
// `crates/looper-select/src/lib.rs` and `select.rs` from looper's SQLite store to the project's
// `.looper/select/` (decision 0027).
//
// # What it is
//
// - A **unit** is one thing a verify run can run and judge on its own: a cargo test binary, a bun test
//   file, a goal check or a verify step. The caller names it with an id and the text that defines it,
//   a `UnitDef`.
// - The **store** is the folder `.looper/select/` of the project's tree (`storeDir`). Each unit is one
//   state file, `units/<sha-1 of the id>.json`, holding its definition, its red mark, what it owes and
//   up to `GREEN_STATES` green states. A **green state** is the files a unit rested on at one green run,
//   with each file's content digest then. The store is a cache of past runs, never the work: deleting it
//   makes the next run select every unit.
// - A unit's `Footprint` is the files, directories and globs it rests on. A directory or a glob means
//   every file under it or matched by it, including one added later. An empty footprint means nothing
//   was attributed, and it widens to the whole tree.
// - A footprint comes from an observer or from a declaration. `cargo.ts` reads a test binary's
//   dep-info, `bun.ts` reads what a test file's preload recorded, and `declared` takes a unit's
//   `inputs`. A unit with neither rests on the whole tree.
// - `Store.record` writes a run's outcome. A green run adds a state from the digests of its footprint
//   and of every global file, and clears what it owed. A red run marks it red and keeps its states. A
//   run under a new definition drops the old definition's states.
// - A green run first digests its files again. When one changed while the unit ran, the result is
//   void: no state is added, the unit is owed, and the result names the files.
// - `select` compares each unit against the tree now. It keeps a unit any of whose states matches the
//   tree. It runs a unit that never ran here, was red or was redefined. It runs one whose every state
//   rests on a file whose digest changed, on a directory with a new file, or on global files that
//   changed, with the reason its newest state gives. A reached unit is owed until it runs green, so a
//   selection that is not followed by a run never loses it. A unit missing from the definition loses
//   its file. `compare` decides the same and deletes nothing, for a caller that learns its units a part
//   at a time and calls `Store.retain` for each part. `peek` decides as `compare` does and writes
//   nothing.
// - `changedSinceGreen` lists every file that differs from a unit's newest green state.
// - `Tree.prime` digests what a unit rests on before it runs, so its record holds what the run started
//   from.
// - The **selection-miss list** is the safety net. A caller that runs every unit calls
//   `Store.recordMiss` for each unit that went red when its selection had not reached it. It is the
//   state file `misses.json`.
// - A change is judged by content digest, never by `git diff`, so an uncommitted tree and a worktree
//   select the same way. A digest is SHA-256 over the file's bytes, cut to 128 bits. A selection costs
//   one `git ls-files` and one digest per file some unit rests on, each file read once.
//
// # The bounds it keeps
//
// - The store holds one file per unit of the current definition: `select` and `Store.retain` delete
//   the rest.
// - A unit keeps its newest `GREEN_STATES` states: `Store.record` drops the rest in the write that adds
//   one. A state on the same content as an older one replaces it. A state holds one entry per file it
//   rested on, and one per directory and glob in its footprint.
// - A `Tree` holds one path per file `git ls-files` lists and one digest per file it was asked about.
//   It lives for one verify run. A digest reads its file in `READ_CHUNK` pieces.
// - The selection-miss list keeps its newest `MISS_LIMIT` entries.
//
// # Known gaps
//
// - Two processes that write one unit's file at once race on it, and one write is lost. A lost green
//   state reruns the unit, and a lost owed mark can skip one. The verify run per tree is its one
//   writer.
// - The store is the tree's own, so a worktree does not share its main checkout's green states.
// - An observed file that exists but git ignores, such as one under `node_modules/` or `target/`, is
//   dropped from a footprint. A hand edit there that changes no lock file reaches no unit.
// - A unit's red mark and what it owes are one per unit, not one per tree.

import { createHash } from "node:crypto";
import { closeSync, openSync, readdirSync, readSync, statSync, unlinkSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import { isGlob, toRegex } from "../goal/glob.ts";
import { readState, writeState } from "../store/state.ts";

/** How many selection misses the store keeps: the newest ones. */
export const MISS_LIMIT = 100;
/** How many green states a unit keeps: the newest ones. */
export const GREEN_STATES = 8;
/** How many bytes a digest reads from its file at a time. */
export const READ_CHUNK = 64 * 1024;

/** A file's content digest as hex, or `null` when no file is there. */
export type Digest = string | null;

/** The file names that rerun every unit when they change: the manifests, the lock files and the toolchain files. */
const GLOBAL_NAMES = new Set(
  [
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain",
    "rust-toolchain.toml",
    "package.json",
    "bun.lock",
    "bun.lockb",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "looper.toml",
  ].map((n) => n.toLowerCase()),
);

/**
 * Whether a change to `path` reruns every unit. A `tsconfig*.json` and a `.cargo/config` count too,
 * because each changes how every file under it builds. Case is ignored, which can only widen.
 */
export function isGlobal(path: string): boolean {
  const lower = path.toLowerCase();
  const name = lower.slice(lower.lastIndexOf("/") + 1);
  return (
    GLOBAL_NAMES.has(name) ||
    (name.startsWith("tsconfig") && name.endsWith(".json")) ||
    lower === ".cargo/config.toml" ||
    lower === ".cargo/config" ||
    lower.endsWith("/.cargo/config.toml") ||
    lower.endsWith("/.cargo/config")
  );
}

/** `path` relative to the root in the store's form: forward slashes, no leading `./` and no trailing `/`. */
export function rel(path: string): string {
  let out = path.replaceAll("\\", "/");
  while (out.startsWith("./")) out = out.slice(2);
  return out.replace(/\/+$/, "");
}

/** `path` made absolute, resolved and with forward slashes, so two spellings of one path compare equal. */
function normal(path: string): string {
  return resolve(path).replaceAll("\\", "/").replace(/\/+$/, "");
}

/** Whether the glob `pattern` matches a path. A pattern that forms no regex matches every path, which can only widen. */
export function globMatcher(pattern: string): (path: string) => boolean {
  const re = toRegex(pattern);
  return (path) => re === undefined || re.test(path);
}

/** The digest of the file at `path`, `null` when no file is there. */
function digestFile(path: string): Digest {
  let fd: number;
  try {
    fd = openSync(path, "r");
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") return null;
    if (isDir(path)) return null;
    throw new Error(`Reading ${path} for its digest failed: ${(e as Error).message}`);
  }
  try {
    const hash = createHash("sha256");
    const buf = Buffer.alloc(READ_CHUNK);
    for (;;) {
      let n: number;
      try {
        n = readSync(fd, buf, 0, READ_CHUNK, null);
      } catch (e) {
        if ((e as NodeJS.ErrnoException).code === "EISDIR") return null;
        throw e;
      }
      if (n === 0) break;
      hash.update(buf.subarray(0, n));
    }
    return hash.digest("hex").slice(0, 32);
  } finally {
    closeSync(fd);
  }
}

function isDir(path: string): boolean {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

/**
 * The project's tree: the files `git ls-files` lists, tracked or untracked but not ignored, and the
 * digest of each file it was asked about. It reads each file at most once, so a digest is the file's
 * content when it was first asked about.
 */
export class Tree {
  readonly root: string;
  /** The root in `normal` form, which `relative` strips. */
  private readonly rootKey: string;
  private readonly listed: Set<string>;
  /** The listed files, sorted, so `under` finds a directory's files as one run. */
  private readonly sorted: string[];
  private readonly digests = new Map<string, Digest>();

  private constructor(root: string, files: string[]) {
    this.root = root;
    this.rootKey = normal(root);
    this.sorted = [...new Set(files)].sort();
    this.listed = new Set(this.sorted);
  }

  /** Lists the tree under `root`, which must be inside a git repository or worktree. */
  static list(root: string): Tree {
    const ran = Bun.spawnSync(["git", "-C", root, "ls-files", "-z", "--cached", "--others", "--exclude-standard"], {
      stdout: "pipe",
      stderr: "pipe",
    });
    if (ran.exitCode !== 0) {
      const stderr = ran.stderr.toString().trim();
      throw new Error(`\`git ls-files\` failed in ${root}: ${stderr}. Run the kit inside a git repository.`);
    }
    const files = ran.stdout
      .toString()
      .split("\0")
      .filter((p) => p !== "")
      .map(rel);
    return new Tree(root, files);
  }

  /** Whether `git ls-files` listed the file at the root-relative `path`. */
  contains(path: string): boolean {
    return this.listed.has(path);
  }

  /**
   * `path` relative to the root in the store's form, or `undefined` when it lies outside the root. A
   * relative `path` is taken from the current directory. On Windows case is ignored, as the file
   * system ignores it.
   */
  relative(path: string): string | undefined {
    const abs = normal(path);
    const head = abs.slice(0, this.rootKey.length);
    const same =
      process.platform === "win32" ? head.toLowerCase() === this.rootKey.toLowerCase() : head === this.rootKey;
    if (!same) return undefined;
    const rest = abs.slice(this.rootKey.length);
    if (rest === "") return "";
    return rest.startsWith("/") ? rest.slice(1) : undefined;
  }

  /** Every listed file, in path order. */
  files(): readonly string[] {
    return this.sorted;
  }

  /** The listed files under the directory `dir`, in path order. The empty path is the whole tree. */
  under(dir: string): string[] {
    if (dir === "") return this.sorted;
    const prefix = `${dir}/`;
    let lo = 0;
    let hi = this.sorted.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if ((this.sorted[mid] as string) < prefix) lo = mid + 1;
      else hi = mid;
    }
    const out: string[] = [];
    for (let i = lo; i < this.sorted.length && (this.sorted[i] as string).startsWith(prefix); i++) {
      out.push(this.sorted[i] as string);
    }
    return out;
  }

  /** The digest of the file at the root-relative `path`, `null` when no file is there. */
  digest(path: string): Digest {
    const known = this.digests.get(path);
    if (known !== undefined) return known;
    const digest = digestFile(join(this.root, path));
    this.digests.set(path, digest);
    return digest;
  }

  /**
   * Digests every file `footprint` covers, and the whole tree for an empty one. A caller primes the
   * tree before a run, so that `Store.record` after it holds what the run started from.
   */
  prime(footprint: Footprint): void {
    for (const path of paths(widen(footprint), this)) this.digest(path);
  }

  /**
   * The paths among `taken` whose content now differs from the digest given with each, in the order
   * given. It reads each file again from disk and keeps no digest it takes.
   */
  changed(taken: Iterable<[string, Digest]>): string[] {
    const out: string[] = [];
    for (const [path, digest] of taken) {
      if (digestFile(join(this.root, path)) !== digest) out.push(path);
    }
    return out;
  }

  /** Every listed global file with its digest, in path order. */
  globalDigests(): Map<string, Digest> {
    const out = new Map<string, Digest>();
    for (const path of this.sorted) if (isGlobal(path)) out.set(path, this.digest(path));
    return out;
  }
}

/** What a unit read, as paths relative to the project's root. */
export interface Footprint {
  files: string[];
  /** Each directory stands for every file under it, now and later. The empty path is the whole tree. */
  dirs: string[];
  /** Each glob stands for every file it matches, now and later, read as `goal/glob.ts` reads one. */
  globs: string[];
}

/** A footprint of `files` alone. */
export function footprintOf(files: string[] = [], dirs: string[] = [], globs: string[] = []): Footprint {
  return { files, dirs, globs };
}

export function wholeTree(): Footprint {
  return footprintOf([], [""]);
}

export function isEmpty(f: Footprint): boolean {
  return f.files.length === 0 && f.dirs.length === 0 && f.globs.length === 0;
}

/** `f`, or the whole tree when it is empty. */
function widen(f: Footprint): Footprint {
  return isEmpty(f) ? wholeTree() : f;
}

/**
 * The footprint a unit declares with its `inputs`, each a root-relative path or glob. A path the tree
 * lists is a file, a path with listed files under it is a directory, and a path with neither is a file
 * that is absent now. No inputs is the whole tree.
 */
export function declared(tree: Tree, inputs: string[]): Footprint {
  if (inputs.length === 0) return wholeTree();
  const out = footprintOf();
  for (const input of inputs) {
    const path = rel(input);
    if (isGlob(path)) out.globs.push(path);
    else if (!tree.contains(path) && tree.under(path).length > 0) out.dirs.push(path);
    else out.files.push(path);
  }
  return out;
}

/**
 * The footprint of what an observer saw a run read, as paths on disk: `files` it opened or probed and
 * `dirs` it listed. A path outside the root is dropped, and so is a file that exists but is not
 * listed, because git ignores it as a build output or an installed dependency. A file that does not
 * exist is kept, so creating it reaches the unit.
 */
export function observed(tree: Tree, files: Iterable<string>, dirs: Iterable<string>): Footprint {
  const out = footprintOf();
  const seen = new Set<string>();
  for (const path of files) {
    const r = tree.relative(isAbsolute(path) ? path : resolve(path));
    if (r === undefined) continue;
    const attributed = tree.contains(r) || !exists(path);
    if (attributed && !seen.has(r)) {
      seen.add(r);
      out.files.push(r);
    }
  }
  const listed = new Set<string>();
  for (const path of dirs) {
    const r = tree.relative(path);
    if (r !== undefined && tree.under(r).length > 0) listed.add(r);
  }
  out.dirs = [...listed].sort();
  return out;
}

function exists(path: string): boolean {
  try {
    statSync(path);
    return true;
  } catch {
    return false;
  }
}

/** Adds `other`'s files, directories and globs to `into`, each once. */
export function union(into: Footprint, other: Footprint): void {
  into.files = [...new Set([...into.files, ...other.files])].sort();
  into.dirs = [...new Set([...into.dirs, ...other.dirs])].sort();
  into.globs = [...new Set([...into.globs, ...other.globs])].sort();
}

/**
 * The root-relative path of every file the footprint covers in `tree`: its files, the listed files
 * under its directories and the listed files its globs match. An empty footprint covers none.
 */
export function paths(f: Footprint, tree: Tree): string[] {
  const out = f.files.map(rel);
  for (const dir of f.dirs) out.push(...tree.under(rel(dir)));
  for (const glob of f.globs) {
    const matches = globMatcher(rel(glob));
    out.push(...tree.files().filter(matches));
  }
  return out;
}

/** A unit as the current definition names it. */
export interface UnitDef {
  /** Stable across runs, such as `cargo:looper-store/test/store` or `check:select-unit`. */
  id: string;
  /** Everything that defines what the unit runs, such as its argv and its declared inputs. A change reruns it. */
  definition: string;
}

export function unitDef(id: string, definition: string): UnitDef {
  return { id, definition };
}

/** How a run of a unit ended. */
export type Outcome = "green" | "red";

/** What `Store.record` wrote: a green state, the red mark, or nothing green because these files changed while it ran. */
export type Recorded = { kind: "green" } | { kind: "red" } | { kind: "void"; changed: string[] };

/** One green state: the digest of each file it rested on, and the directories and globs it rested on. */
interface GreenState {
  /** A digest of the state's content, which two runs on the same content share. */
  key: string;
  at: number;
  /** Each non-global file it rested on, in path order, with its digest then. */
  files: [string, Digest][];
  /** Each global file listed then, in path order, with its digest then. */
  globals: [string, Digest][];
  dirs: string[];
  globs: string[];
}

/** A unit's state file. */
interface UnitFile {
  id: string;
  definition: string;
  red: boolean;
  /** Why a selection reached the unit, until it runs green. */
  owed: string | null;
  at: number;
  /** Oldest first. */
  states: GreenState[];
}

/** A unit's row as a reader sees it. */
export interface UnitRow {
  definition: string;
  red: boolean;
  owed: string | null;
  at: number;
}

/** A unit that went red in a run of every unit when the selection had not reached it. */
export interface Miss {
  unit: string;
  at: number;
}

/** The store folder of the project at `root`. */
export function storeDir(root: string): string {
  return join(root, ".looper", "select");
}

/** One project's unit store. */
export class Store {
  readonly dir: string;

  private constructor(dir: string) {
    this.dir = dir;
  }

  /** The store in the folder `dir`, which the first write creates. */
  static open(dir: string): Store {
    return new Store(dir);
  }

  private unitPath(id: string): string {
    return join(this.dir, "units", `${createHash("sha1").update(id).digest("hex")}.json`);
  }

  private read(id: string): UnitFile | undefined {
    let doc: UnitFile | undefined;
    try {
      doc = readState<UnitFile>(this.unitPath(id));
    } catch {
      return undefined;
    }
    return doc?.id === id && Array.isArray(doc.states) ? doc : undefined;
  }

  private write(doc: UnitFile): void {
    writeState(this.unitPath(doc.id), doc);
  }

  /** Every unit file in the store, in id order. */
  private all(): UnitFile[] {
    let names: string[];
    try {
      names = readdirSync(join(this.dir, "units"));
    } catch {
      return [];
    }
    const out: UnitFile[] = [];
    for (const name of names) {
      if (!name.endsWith(".json")) continue;
      try {
        const doc = readState<UnitFile>(join(this.dir, "units", name));
        if (doc && typeof doc.id === "string") out.push(doc);
      } catch {}
    }
    return out.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  }

  /** Sets why unit `id` is owed, when it has a file. */
  markOwed(id: string, why: string): void {
    const doc = this.read(id);
    if (doc) this.write({ ...doc, owed: why });
  }

  /**
   * Writes how a run of `unit` ended, and returns what it wrote.
   *
   * - A green run adds a state: the digests `tree` gives for its footprint and for every global file.
   *   A `Tree` keeps the digest it took first, so one primed before the run holds what the run started
   *   from.
   * - Before it writes, it digests each of those files again. When any differs, the run saw content
   *   that is gone: it writes no state and marks the unit owed.
   * - A state on the same content as an older one replaces it, and a unit keeps its newest
   *   `GREEN_STATES`.
   * - A red run marks the unit red and keeps its states. A run under a new definition drops every
   *   state of the old one.
   */
  record(tree: Tree, unit: UnitDef, outcome: Outcome, footprint: Footprint): Recorded {
    const red = outcome === "red";
    let state: GreenState | undefined;
    if (!red) {
      const f = widen(footprint);
      const files = new Map<string, Digest>();
      for (const path of paths(f, tree)) {
        if (!isGlobal(path) && !files.has(path)) files.set(path, tree.digest(path));
      }
      const globals = tree.globalDigests();
      const all = [...files, ...globals].sort(byPath);
      const changed = tree.changed(all);
      if (changed.length > 0) {
        this.markOwed(unit.id, `${changed.join(", ")} changed during its run`);
        return { kind: "void", changed };
      }
      const dirs = [...new Set(f.dirs.map(rel))].sort();
      const globs = [...new Set(f.globs.map(rel))].sort();
      const fileRows = [...files].sort(byPath);
      const globalRows = [...globals].sort(byPath);
      const key = createHash("sha256").update(JSON.stringify([fileRows, globalRows, dirs, globs])).digest("hex");
      state = { key, at: Date.now(), files: fileRows, globals: globalRows, dirs, globs };
    }
    const stored = this.read(unit.id);
    let states = stored && stored.definition === unit.definition ? stored.states : [];
    if (state) {
      const key = state.key;
      states = [...states.filter((s) => s.key !== key), state].slice(-GREEN_STATES);
    }
    this.write({
      id: unit.id,
      definition: unit.definition,
      red,
      owed: red ? (stored?.owed ?? null) : null,
      at: Date.now(),
      states,
    });
    return red ? { kind: "red" } : { kind: "green" };
  }

  /** How many green states the unit `id` keeps. */
  states(id: string): number {
    return this.read(id)?.states.length ?? 0;
  }

  /**
   * Deletes the file of every stored unit whose id starts with `scope` and is not among `ids`, and
   * returns those ids in id order. The empty scope is every unit.
   */
  retain(scope: string, ids: string[]): string[] {
    const keep = new Set(ids);
    const dropped: string[] = [];
    for (const doc of this.all()) {
      if (doc.id.startsWith(scope) && !keep.has(doc.id)) {
        try {
          unlinkSync(this.unitPath(doc.id));
        } catch {}
        dropped.push(doc.id);
      }
    }
    return dropped;
  }

  /** The row of the unit `id`, if it ever ran here and is still defined. */
  unit(id: string): UnitRow | undefined {
    const doc = this.read(id);
    return doc && { definition: doc.definition, red: doc.red, owed: doc.owed, at: doc.at };
  }

  /** The unit's newest green state, if it has one. */
  newest(id: string): GreenState | undefined {
    return this.read(id)?.states.at(-1);
  }

  /** The paths of the files the unit `id` rested on at its newest green, in path order. */
  files(id: string): string[] {
    const s = this.newest(id);
    return s ? [...s.files, ...s.globals].map(([p]) => p).sort() : [];
  }

  /** The directories the unit `id` rested on as a whole at its newest green, in path order. */
  dirs(id: string): string[] {
    return this.newest(id)?.dirs ?? [];
  }

  /** The globs the unit `id` declared at its newest green, in order. */
  globs(id: string): string[] {
    return this.newest(id)?.globs ?? [];
  }

  /** The id of every stored unit that starts with `scope`, in id order. */
  ids(scope: string): string[] {
    return this.all()
      .map((d) => d.id)
      .filter((id) => id.startsWith(scope));
  }

  /** Adds the unit `id` to the selection-miss list, and keeps its newest `MISS_LIMIT` entries. */
  recordMiss(id: string): void {
    const misses = [...this.misses(), { unit: id, at: Date.now() }].slice(-MISS_LIMIT);
    writeState(join(this.dir, "misses.json"), { misses });
  }

  /** The selection-miss list, oldest first. */
  misses(): Miss[] {
    try {
      const doc = readState<{ misses: Miss[] }>(join(this.dir, "misses.json"));
      return Array.isArray(doc?.misses) ? doc.misses : [];
    } catch {
      return [];
    }
  }

  /** The unit's whole file, for the selection. */
  doc(id: string): UnitFile | undefined {
    return this.read(id);
  }
}

function byPath(a: [string, Digest], b: [string, Digest]): number {
  return a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0;
}

/** Why a selection runs a unit. */
export type Reason =
  /** It never ran here. */
  | { kind: "new" }
  /** Its definition differs from the one its last run had. */
  | { kind: "redefined" }
  /** Its last run was red. */
  | { kind: "red" }
  /** A global file was changed, added or removed. */
  | { kind: "global"; path: string }
  /** A file it rested on was changed or removed. */
  | { kind: "changed"; path: string }
  /** A file is new under a directory it rests on, or new and matched by a glob it declared. */
  | { kind: "added"; path: string }
  /** An earlier selection reached it, for the reason given, and it has not run green since. */
  | { kind: "owed"; why: string };

/** The reason as a person reads it. */
export function explain(r: Reason): string {
  switch (r.kind) {
    case "new":
      return "it never ran here";
    case "redefined":
      return "its definition changed";
    case "red":
      return "its last run was red";
    case "global":
      return `the global file ${r.path} changed`;
    case "changed":
      return `${r.path} changed`;
    case "added":
      return `${r.path} is new where it rests`;
    case "owed":
      return `it has not run green since ${r.why}`;
  }
}

/** What a selection decided, each list in the order of the definition it was given. */
export interface Selection {
  /** The units to run, each with the first reason found. */
  run: [string, Reason][];
  /** The units with a green state that matches the tree. */
  kept: string[];
  /** The units the store held that the definition no longer names. Their files are gone. */
  dropped: string[];
}

/** The ids of the units a selection runs. */
export function ids(s: Selection): string[] {
  return s.run.map(([id]) => id);
}

/** Why the selection runs the unit `id`, or `undefined` when it does not. */
export function reason(s: Selection, id: string): Reason | undefined {
  return s.run.find(([u]) => u === id)?.[1];
}

/**
 * Decides which of `units` run against `tree`, when `units` is the whole definition. It deletes the
 * file of every stored unit that `units` does not name, and marks every reached unit owed until
 * `Store.record` records it green.
 */
export function select(store: Store, tree: Tree, units: UnitDef[]): Selection {
  const dropped = store.retain(
    "",
    units.map((u) => u.id),
  );
  return { ...compare(store, tree, units), dropped };
}

/** Decides as `select` does and marks every reached unit owed, but deletes no file. */
export function compare(store: Store, tree: Tree, units: UnitDef[]): Selection {
  return decide(store, tree, units, true);
}

/** Decides as `compare` does, and writes nothing: a reached unit is not marked owed. */
export function peek(store: Store, tree: Tree, units: UnitDef[]): Selection {
  return decide(store, tree, units, false);
}

/**
 * The files of `tree` that differ from the unit `id`'s newest green state, in path order: each file it
 * rested on whose digest changed or that is gone, and each file new under a directory or a glob it
 * rested on. A unit with no green state has none.
 */
export function changedSinceGreen(store: Store, tree: Tree, id: string): string[] {
  const state = store.newest(id);
  if (!state) return [];
  const rows = [...state.files, ...state.globals];
  const recorded = new Set(rows.map(([p]) => p));
  const changed = new Set<string>();
  for (const [path, digest] of rows) if (tree.digest(path) !== digest) changed.add(path);
  for (const dir of state.dirs) for (const p of tree.under(dir)) if (!recorded.has(p)) changed.add(p);
  for (const glob of state.globs) {
    const matches = globMatcher(glob);
    for (const p of tree.files()) if (matches(p) && !recorded.has(p)) changed.add(p);
  }
  return [...changed].sort();
}

function decide(store: Store, tree: Tree, units: UnitDef[], markOwed: boolean): Selection {
  const globals = tree.globalDigests();
  const selection: Selection = { run: [], kept: [], dropped: [] };
  for (const unit of units) {
    const doc = store.doc(unit.id);
    if (!doc) {
      selection.run.push([unit.id, { kind: "new" }]);
      continue;
    }
    const why: Reason | undefined =
      doc.definition !== unit.definition
        ? { kind: "redefined" }
        : doc.red
          ? { kind: "red" }
          : reach(tree, doc.states, globals);
    if (why) {
      if (markOwed && doc.owed === null) store.markOwed(unit.id, explain(why));
      selection.run.push([unit.id, why]);
    } else if (doc.owed !== null) {
      selection.run.push([unit.id, { kind: "owed", why: doc.owed }]);
    } else {
      selection.kept.push(unit.id);
    }
  }
  return selection;
}

/**
 * Why `tree` reaches a green unit with `states`, or `undefined` when any of them matches the tree. A
 * unit that matches none gets the reason its newest state gives. A unit with no state is new.
 */
function reach(tree: Tree, states: GreenState[], globals: Map<string, Digest>): Reason | undefined {
  let newest: Reason | undefined;
  for (let i = states.length - 1; i >= 0; i--) {
    const r = reachState(tree, states[i] as GreenState, globals);
    if (r === undefined) return undefined;
    newest ??= r;
  }
  return newest ?? { kind: "new" };
}

/**
 * The first change in `tree` from `state`: a global file first, then a file it read, then a file new
 * under a directory it rests on or matched by a glob it declared.
 */
function reachState(tree: Tree, state: GreenState, globals: Map<string, Digest>): Reason | undefined {
  const recorded = new Set([...state.files, ...state.globals].map(([p]) => p));
  for (const [path, digest] of state.globals) {
    if ((globals.get(path) ?? null) !== digest) return { kind: "global", path };
  }
  for (const path of globals.keys()) if (!recorded.has(path)) return { kind: "global", path };
  for (const [path, digest] of state.files) {
    if (tree.digest(path) !== digest) return { kind: "changed", path };
  }
  for (const dir of state.dirs) {
    const path = tree.under(dir).find((p) => !isGlobal(p) && !recorded.has(p));
    if (path !== undefined) return { kind: "added", path };
  }
  for (const glob of state.globs) {
    const matches = globMatcher(glob);
    const path = tree.files().find((p) => matches(p) && !isGlobal(p) && !recorded.has(p));
    if (path !== undefined) return { kind: "added", path };
  }
  return undefined;
}
