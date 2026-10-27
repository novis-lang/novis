// The selection: from what changed since a recorded tree to the atoms that have to run, and why.
//
//     changed = diff(recorded tree, working tree incl. untracked)
//            -> each .rs file: item diff -> reference-graph walk -> fn:, class:, card: keys
//            -> each build script whose inputs moved: the items that include what it generated
//            -> every path: file:, config:, tree:, named:, ext:, and exists:/dir: for a path that came or went
//            -> a path of a built-in component's crate or WIT file: its component: key (`keys.ts`)
//            -> the repository's nvs.toml: config: and app: for the parts that changed (`config.ts`)
//     run = atoms never recorded on this platform, last red, or owed from an earlier change
//         + atoms marked diverged: a recording run that ended differently from the judged run
//         + atoms whose definition changed
//         + atoms indexed under any changed key
//         + every atom, when a global file changed, other than by adding a dependency the workspace
//           already resolves (`addsDependenciesOnly`)
//         + every proof program, when code only an optimized build compiles changed and has no twin
//
// A run counts the moment it is recorded, not when the run ends: an atom whose last run was recorded
// after the base was taken is stamped with the tree that run was taken at (`store.ts`), and the three
// lines above that read a change read, for that atom, the change since its own tree. One that last ran
// on the tree as it is now is selected only by its verdict, a divergence, or a definition the caller
// names. The change since a stamped tree is read as the change since the base is (`sinceStamps`): the
// stamped tree keeps its overlay's Rust items and marker scans, every other file is as its commit holds
// it, and a Rust file of its overlay with no items kept is taken whole. A tree that cannot be read
// again selects every atom stamped with it.
//
// Nothing outside `run` is started. `explain` says for one atom which key selected it, the path and
// item the key came from, and for a key reached through the reference graph the item that reached it.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { parse as parseToml } from "smol-toml";
import { type FileItems, type Item, scanItems } from "../keys/scan.ts";
import { landsIn, metadata, type Graph, dependencySet, testBinaries } from "../keys/graph.ts";
import { COVWS_TARGET, hostTriple } from "../lib/covws.ts";
import { abs, NOT_INPUTS, ROOT } from "../lib/paths.ts";
import { benchFiles, benchId, benchOf, caseFiles, caseId, currentDef, nvTestFiles, nvTestId, proofFiles, proofId, stillThere } from "./atoms.ts";
import { NV_TSC } from "./nvtests.ts";
import { buildScripts, envReaders, generatedDigest, generatedIncludes, generatedMeta, isInput } from "./build.ts";
import { ANCHOR_ANY, CALLS_ANY, COVERS_ANY, markerKeys, scannedFor } from "../proofs/markers.ts";
import { LEDGER, LEDGER_WHOLE, ledgerMoved, ledgerSigns, PERF_ANY } from "../proofs/ledger.ts";
import { anchorScan, isAnchorFile } from "../proofs/roster.ts";
import { blobAt, type Change, changedBetween, changedPaths, commitOf, diskDigest, namedChanges, sinceOverlay, snapshot, treeSig } from "./change.ts";
import { diffFile, referenceWalk, type ExtraDefines, type ItemChange, type Moved, type Origin, type Scope, Universe } from "./items.ts";
import { elsewhereOnly, gitTexts, platformOf, profileReader } from "./profile.ts";
import { ALL_CARDS, ALL_CLASSES, ALL_COMPONENTS, BUILTINS_READ, builtinComponents, componentKeys, configKey, fileWild, kindOf, pathKeys, pkgTestsKey, PROFILE_ONLY, ROOT_CONFIG, testsKey, WILD } from "./keys.ts";
import { blockPaths, configKeys } from "./config.ts";
import { type AtomKind, kindOfAtom, type Overlay, type RunTree, type SelectStore } from "./store.ts";

/** Files whose change selects every atom: the toolchain, the lock file, a manifest, and the two tools
 * every footprint's meaning rests on. An edit of the lock file or a member's manifest that only adds a
 * dependency the workspace already resolves is the exception (`globalOf`). */
export function isGlobal(path: string): boolean {
  return (
    path === "rust-toolchain.toml" ||
    path === "Cargo.lock" ||
    path === ".cargo/config.toml" ||
    (path.endsWith("/Cargo.toml") && !path.startsWith("fuzz/")) ||
    path === "Cargo.toml" ||
    path.startsWith("tools/nv-scan/") ||
    path.startsWith("tools/covwrap/")
  );
}

const DEP_TABLES = ["dependencies", "dev-dependencies", "build-dependencies"];

type Toml = Record<string, unknown>;
const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);
const table = (v: unknown): Toml => (v !== null && typeof v === "object" && !Array.isArray(v) ? (v as Toml) : {});

/** Whether dependency table `after` is `before` with entries added, each exactly `{ workspace = true }`:
 * a crate the workspace already resolves, with the features the root manifest gives it. */
function addsWorkspaceDeps(before: Toml, after: Toml): boolean {
  for (const [name, spec] of Object.entries(before)) if (!same(spec, after[name])) return false;
  for (const [name, spec] of Object.entries(after)) if (!(name in before) && !same(spec, { workspace: true })) return false;
  return true;
}

/** `doc` without its dependency tables, at the top and under each `[target.<cfg>]`. */
function withoutDeps(doc: Toml): Toml {
  const rest: Toml = { ...doc };
  for (const t of DEP_TABLES) delete rest[t];
  if (rest.target !== undefined) {
    rest.target = Object.fromEntries(Object.entries(table(rest.target)).map(([cfg, v]) => [cfg, Object.fromEntries(Object.entries(table(v)).filter(([k]) => !DEP_TABLES.includes(k)))]));
  }
  return rest;
}

/**
 * Whether a global file's edit from `before` to `after` only adds dependencies the workspace already
 * resolves, and so changes no compiled code that the item diff does not see: the code that uses such a
 * crate is a changed item of its own. A member's `Cargo.toml` qualifies when every other table is the
 * same and each new entry is `{ workspace = true }`; `Cargo.lock` when the same packages stand at the
 * same versions and sources, and a package's `dependencies` only grew. Every other edit, and every other
 * global file, selects every atom. What it costs: a crate the workspace held only as another crate's
 * dependency, with fewer features than the root manifest gives it, can compile differently, and nightly
 * CI is what runs that.
 */
export function addsDependenciesOnly(path: string, before: string, after: string): boolean {
  let a: Toml;
  let b: Toml;
  try {
    a = parseToml(before) as Toml;
    b = parseToml(after) as Toml;
  } catch {
    return false;
  }
  if (path === "Cargo.lock") {
    const key = (p: Toml) => `${p.name}@${p.version}@${p.source ?? ""}`;
    const was = new Map((Array.isArray(a.package) ? (a.package as Toml[]) : []).map((p) => [key(p), p]));
    const now = Array.isArray(b.package) ? (b.package as Toml[]) : [];
    if (now.length !== was.size || !same({ ...a, package: [] }, { ...b, package: [] })) return false;
    return now.every((p) => {
      const old = was.get(key(p));
      if (!old || !same({ ...old, dependencies: [] }, { ...p, dependencies: [] })) return false;
      const deps = new Set((p.dependencies as string[] | undefined) ?? []);
      return ((old.dependencies as string[] | undefined) ?? []).every((d) => deps.has(d));
    });
  }
  if (!path.endsWith("/Cargo.toml")) return false;
  if (!same(withoutDeps(a), withoutDeps(b))) return false;
  const scopes = (d: Toml) => [d, ...Object.values(table(d.target)).map(table)];
  const [was, now] = [scopes(a), scopes(b)];
  if (Object.keys(table(a.target)).join() !== Object.keys(table(b.target)).join()) return false;
  return was.every((s, i) => DEP_TABLES.every((t) => addsWorkspaceDeps(table(s[t]), table(now[i]![t]))));
}

/** The first global file of `changes` whose edit selects every atom, or null. A modified `Cargo.lock` or
 * member manifest is read on both sides (`addsDependenciesOnly`), unless the base side is a recorded
 * tree's overlay, whose text is not kept. */
async function globalOf(changes: Change[], since: string, until: string | null, overlay: Overlay, root: string): Promise<string | null> {
  const text = (b: Uint8Array | null) => (b === null ? null : new TextDecoder().decode(b));
  for (const c of changes) {
    if (!isGlobal(c.path)) continue;
    const readable = c.status === "modified" && !(c.path in overlay) && (c.path === "Cargo.lock" || (c.path.endsWith("/Cargo.toml") && c.path !== "Cargo.toml"));
    if (readable) {
      const before = text(await blobAt(since, c.path, root));
      const after = until ? text(await blobAt(until, c.path, root)) : existsSync(join(root, c.path)) ? readFileSync(join(root, c.path), "utf8") : null;
      if (before !== null && after !== null && addsDependenciesOnly(c.path, before, after)) continue;
    }
    return c.path;
  }
  return null;
}

/** What changed, as keys. */
export interface ChangeSet {
  since: string;
  /** The commit a replayed change ends at; absent for a change read from the working tree. */
  until?: string;
  changes: Change[];
  /** Every key that moved, with why. A key ending in `#` stands for every key of that file. */
  moved: Moved;
  /** The global file that changed, when one did. */
  global: string | null;
  rustFiles: number;
  itemChanges: number;
  wideFiles: string[];
  /** Each item that was added, removed or changed, and the ones a build script's output moved. */
  items: ItemChange[];
  /** Every Rust file's items as the changed side holds them: the stored scan with each changed file as
   * it is now. What a recorded run's coverage is mapped against. */
  view: Map<string, FileItems>;
  /** The tree the change was read at, for the store to record once a run over it is green; absent for
   * a change replayed from history. */
  tree?: { commit: string; overlay: Overlay };
  /** A change that stands for everything, and `view` is a scan of every Rust file: the store had no
   * tree to compare with, or the scanner itself changed. */
  full?: boolean;
  /** For each tree an atom is stamped with (`SelectStore.stamps`), by its row, the change since that
   * tree to this one: what `query` reads that atom against. A tree that could not be compared gives a
   * change to everything, and the tree this change was read at gives an empty one. A tree that is the
   * base has none, and its atoms are read against this change. Absent for a change not read since the
   * base, which reads every atom against itself. */
  stamped?: Map<number, ChangeSet>;
  /** The repository the change was read in; `ROOT` when absent. */
  root?: string;
}

/** The workspace graph as the reference walk's scope: a file's package is the one whose directory holds it, a
 * package's crate name is its library target's, and a file under its package's `tests/`, `benches/` or
 * `examples/`, or the root of a target of those kinds, is built only into that dev target. A path
 * dependency outside the workspace that a member names (`Package.linked`) is a package too, whose crate
 * name is its package name: a package sees it through its own dependency of that kind, its dev ones only
 * with `dev`, and through the normal and build ones of each package it depends on. Its own dependencies
 * are not read, so it sees every package. The crate of a built-in component (`components`,
 * `builtinComponents`'s set) that no package names as a path dependency is compiled by no build of the
 * workspace. */
export function graphScope(graph: Graph | null, components: ReadonlySet<string> = builtinComponents()): Scope {
  if (!graph) return { pkgOf: () => null, sees: () => true, modDir: () => null };
  const linked = new Map<string, string>();
  for (const p of graph.values()) for (const l of p.linked ?? []) if (!graph.has(l.name)) linked.set(l.name, l.dir);
  const linkedDirs = new Set(linked.values());
  const guests = [...components].map((c) => `extensions/${c}`).filter((d) => !linkedDirs.has(d));
  const dirOf = new Map([...[...graph.values()].map((p) => [p.name, p.dir] as const), ...linked]);
  const dirs = [...dirOf].map(([name, dir]) => [dir, name] as const).sort((a, b) => b[0].length - a[0].length);
  const roots = new Set([...graph.values()].flatMap((p) => p.targets.map((t) => t.src)));
  const devRoots = new Set([...graph.values()].flatMap((p) => p.targets.filter((t) => ["test", "bench", "example"].includes(t.kind)).map((t) => t.src)));
  const crates = new Map([...linked.keys()].map((name) => [name.replace(/-/g, "_"), name]));
  for (const p of graph.values()) crates.set((p.targets.find((t) => t.kind === "lib")?.name ?? p.name).replace(/-/g, "_"), p.name);
  const cache = new Map<string, string | null>();
  const deps = new Map<string, Set<string>>();
  const pkgOf = (file: string): string | null => {
    if (cache.has(file)) return cache.get(file)!;
    const hit = dirs.find(([d]) => d === "" || d === "." || file === d || file.startsWith(`${d}/`));
    const pkg = hit ? hit[1] : null;
    cache.set(file, pkg);
    return pkg;
  };
  return {
    crateNamed(ident, user) {
      return graph.get(user)?.renames?.get(ident) ?? crates.get(ident) ?? null;
    },
    devFile(file) {
      if (devRoots.has(file)) return true;
      const pkg = pkgOf(file);
      const dir = pkg === null ? "" : dirOf.get(pkg)!;
      const rest = dir === "" || dir === "." ? file : file.slice(dir.length + 1);
      return /^(tests|benches|examples)\//.test(rest);
    },
    modDir(file) {
      if (roots.has(file) || file.endsWith("/mod.rs")) return file.slice(0, file.lastIndexOf("/") + 1);
      return `${file.replace(/\.rs$/, "")}/`;
    },
    pkgOf,
    sees(user, owner, dev = true) {
      if (user === owner || linked.has(user)) return true;
      const key = `${dev ? "dev" : "ship"}\0${user}`;
      let seen = deps.get(key);
      if (!seen) {
        seen = dependencySet(graph, user, dev);
        for (const l of graph.get(user)?.linked ?? []) if (dev || l.kind !== "dev") seen.add(l.name);
        for (const d of [...seen]) for (const l of graph.get(d)?.linked ?? []) if (l.kind !== "dev") seen.add(l.name);
        deps.set(key, seen);
      }
      return seen.has(owner);
    },
    unlinked(file) {
      if (pkgOf(file) !== null) return null;
      return guests.find((d) => file.startsWith(`${d}/`)) ?? null;
    },
  };
}

const isRust = (path: string) => path.endsWith(".rs") && !NOT_INPUTS.has(path.split("/")[0]!);

/** The files of the base scan that hold a class table, a class item or a card: the batch every scan
 * needs beside the files it reads, since a class is attributed across the files of one batch. */
function classBatch(items: Map<string, FileItems>): string[] {
  const out: string[] = [];
  for (const f of items.values()) if (f.items.some((i) => i.class || i.rows || i.cards)) out.push(f.file);
  return out;
}

/** `files` scanned as they are at commit `rev`, from a scratch tree under `.agent-tmp/`. The files each
 * one embeds are fetched too, in a second pass, so an embedding folds the bytes it had then. */
export async function scanAt(rev: string, files: string[], root: string = ROOT): Promise<Map<string, FileItems>> {
  const tree = join(root, ".agent-tmp", "select-rev", rev.slice(0, 12));
  rmSync(tree, { recursive: true, force: true });
  const put = async (paths: string[]) => {
    for (const p of paths) {
      const bytes = await blobAt(rev, p, root);
      if (bytes === null) continue;
      const full = join(tree, p);
      mkdirSync(dirname(full), { recursive: true });
      writeFileSync(full, bytes);
    }
  };
  try {
    await put(files);
    let scanned = scanItems(files, tree);
    const embeds = [...new Set(scanned.flatMap((f) => f.items.flatMap((i) => i.includes ?? [])))].filter((p) => !existsSync(join(tree, p)));
    if (embeds.length > 0) {
      await put(embeds);
      scanned = scanItems(files, tree);
    }
    return new Map(scanned.filter((f) => f.raw !== "").map((f) => [f.file, f]));
  } finally {
    rmSync(tree, { recursive: true, force: true });
  }
}

/**
 * The keys `changes` move in the repository's `nvs.toml`: the parts an edit of it changed, from its text
 * at `since` and at `until` or on disk. The text a recorded overlay stands for is not kept, so a change
 * read from one moves the file whole. A block that an edit moved, or whose path a deleted file was or
 * sat under, is keyed on a path that must still be there: one that is not stops every program that
 * reads the file, so the file moves whole.
 */
async function rootConfigKeys(overlay: Overlay, changes: Change[], since: string, until: string | null, fromBase: boolean, root: string): Promise<string[]> {
  const edited = changes.some((c) => c.path === ROOT_CONFIG);
  const gone = changes.filter((c) => c.status === "deleted").map((c) => c.path.toLowerCase());
  if (!edited && gone.length === 0) return [];
  const text = (bytes: Uint8Array | null) => (bytes === null ? null : new TextDecoder().decode(bytes));
  const onDisk = join(root, ROOT_CONFIG);
  const after = until ? text(await blobAt(until, ROOT_CONFIG, root)) : existsSync(onDisk) ? readFileSync(onDisk, "utf8") : null;
  let keys: string[] = [];
  if (edited) {
    const before = fromBase && ROOT_CONFIG in overlay ? null : text(await blobAt(since, ROOT_CONFIG, root));
    keys = configKeys(ROOT_CONFIG, before, after, root);
  }
  const blocks = after === null ? null : blockPaths(ROOT_CONFIG, after, root);
  if (blocks === null) return keys;
  const whole = configKey(ROOT_CONFIG);
  for (const [key, path] of blocks) {
    const lower = path.toLowerCase();
    const named = keys.includes(key) || gone.some((g) => g === lower || g.startsWith(`${lower}/`));
    if (named && !keys.includes(whole) && !(await pathAt(until, path, root))) return [whole, ...keys];
  }
  return keys;
}

/** Whether `path`, repo-relative or absolute, is there at commit `until`, or on disk when there is none.
 * A path outside the tree is looked for on disk either way. */
async function pathAt(until: string | null, path: string, root: string): Promise<boolean> {
  if (path === ".") return true;
  if (path.startsWith("/") || /^[A-Za-z]:\//.test(path)) return existsSync(path);
  if (!until) return existsSync(join(root, path));
  const p = Bun.spawn(["git", "cat-file", "-e", `${until}:${path}`], { cwd: root, stdout: "ignore", stderr: "ignore" });
  return (await p.exited) === 0;
}

export interface ChangeOptions {
  /** The commit to compare the working tree with; the store's base when omitted. */
  since?: string;
  /** A commit to take as the changed side instead of the working tree: a change replayed from history.
   * What a build script generated is not known for it, and every atom whose definition files it touches
   * counts as redefined. */
  until?: string;
  /** Paths to take as changed instead of asking git. */
  paths?: string[];
  graph?: Graph | null;
  root?: string;
  /** A tree an atom's last run was taken at, to read the change since in place of the base's, with
   * every Rust file's items now (the change since the base's `view`) and the tree as it is now. */
  from?: { tree: RunTree; view: Map<string, FileItems>; now: { commit: string; overlay: Overlay } };
}

/** A change that moves nothing, read at the tree `view` is of. */
function nothing(view: Map<string, FileItems>): ChangeSet {
  return { since: "", changes: [], moved: new Map(), global: null, rustFiles: 0, itemChanges: 0, wideFiles: [], items: [], view };
}

/** A change that moves everything, for the reason `why`. */
function everything(view: Map<string, FileItems>, why: string): ChangeSet {
  return { ...nothing(view), moved: new Map([[WILD, { path: why, how: "global" }]]), global: why };
}

/** The keys that moved between `since` and the working tree. Read since the base, it also reads the
 * change since each tree an atom is stamped with (`ChangeSet.stamped`). */
export async function computeChange(store: SelectStore, opts: ChangeOptions = {}): Promise<ChangeSet> {
  const root = opts.root ?? ROOT;
  const from = opts.from;
  const base = from ? from.tree.commit : store.base();
  const sinceRev = opts.since ?? base;
  if (!sinceRev) throw new Error("the store has no recorded tree: run `bun nv select --seed` first, or name one with --since");
  const since = await commitOf(sinceRev, root);
  const until = opts.until ? await commitOf(opts.until, root) : null;
  // The tree is taken first, so a file written while the change is read is a change again next time.
  const tree = until ? undefined : from ? from.now : await snapshot(root);
  const fromBase = opts.since === undefined && !until && !opts.paths;
  const overlay = from ? from.tree.overlay : store.overlay();
  let changes = until ? await changedBetween(since, until, root) : opts.paths ? await namedChanges(opts.paths, since, root) : await changedPaths(since, root);
  if (fromBase) changes = sinceOverlay(changes, overlay, root);
  const moved: Moved = new Map();
  const emit = (key: string, origin: Origin) => {
    if (!moved.has(key)) moved.set(key, origin);
  };
  const global = await globalOf(changes, since, until, fromBase || from ? overlay : {}, root);
  for (const c of changes) for (const k of pathKeys(c.path, c.status !== "modified")) emit(k, { path: c.path, how: "path" });
  const components = builtinComponents(root);
  for (const c of changes) for (const k of componentKeys(c.path, components)) emit(k, { path: c.path, how: "path" });
  for (const k of await rootConfigKeys(overlay, changes, since, until, fromBase, root)) emit(k, { path: ROOT_CONFIG, how: "path" });
  if (global) emit(WILD, { path: global, how: "global" });
  const markers = from ? (from.tree.markers as Record<string, Scanned>) : fromBase ? storedMarkers(store) : {};
  for (const [k, origin] of await scanKeys(changes, since, until, fromBase ? overlay : {}, markers, root)) emit(k, origin);

  const graph = opts.graph === undefined ? await metadata() : opts.graph;
  const scope = graphScope(graph, components);
  const stored = from ? from.view : store.allItems();
  // Every Rust file's items as the store last scanned them, or every Rust file as it is now when the
  // store holds no scan.
  const files = stored.size > 0 ? stored : new Map(scanItems(await rustFiles(root), root).map((f) => [f.file, f]));
  // Each changed Rust file, and each Rust file that embeds a changed file: its embedding item folds
  // the embedded bytes into its digest, so reading it again moves that item like an edit of it.
  const edited = changes.filter((c) => isRust(c.path)).map((c) => c.path);
  const rust = [...new Set([...edited, ...embedders(files.values(), changes.map((c) => c.path))])];
  // The base side: the store's own scan when it was taken at `since`, else the files as they were then.
  // The scan is of the recorded tree, overlay and all, so it stands for the commit alone only when the
  // overlay is empty.
  let before: Map<string, FileItems>;
  // Since a stamped tree: each file of its overlay as that tree keeps it, and every other file as its
  // commit holds it. A Rust file of the overlay the tree keeps no items for is taken whole.
  const unknown: string[] = [];
  const scanIsSince = !from && base !== null && (fromBase || (since === (await commitOf(base, root)) && Object.keys(store.overlay()).length === 0));
  if (from) {
    before = new Map();
    const kept = new Map(from.tree.items.map((f) => [f.file, f]));
    const atCommit: string[] = [];
    for (const f of rust) {
      if (!(f in overlay)) {
        if (changes.find((c) => c.path === f)?.status !== "added") atCommit.push(f);
      } else if (overlay[f] !== null) {
        const items = kept.get(f);
        if (items) before.set(f, items);
        else unknown.push(f);
      }
    }
    if (atCommit.length > 0) for (const [f, items] of await scanAt(since, [...new Set([...atCommit, ...classBatch(stored)])], root)) if (atCommit.includes(f)) before.set(f, items);
  } else if (scanIsSince && stored.size > 0) {
    before = new Map(rust.flatMap((f) => (stored.has(f) ? [[f, stored.get(f)!] as const] : [])));
    const missing = rust.filter((f) => !stored.has(f) && changes.find((c) => c.path === f)?.status !== "added");
    if (missing.length > 0) for (const [f, items] of await scanAt(since, missing, root)) before.set(f, items);
  } else {
    before = rust.length > 0 ? await scanAt(since, [...new Set([...rust, ...classBatch(stored)])], root) : new Map();
    for (const f of [...before.keys()]) if (!rust.includes(f)) before.delete(f);
  }
  // The head side, scanned with the class batch.
  const present = until ? rust.filter((f) => changes.find((c) => c.path === f)?.status !== "deleted") : rust.filter((f) => existsSync(join(root, f)));
  const batch = [...new Set([...present, ...classBatch(stored.size > 0 ? stored : new Map())])];
  const after = until
    ? new Map([...(await scanAt(until, batch, root))].filter(([f]) => present.includes(f)))
    : new Map(scanItems(batch, root).filter((f) => present.includes(f.file)).map((f) => [f.file, f]));

  const itemChanges: ItemChange[] = [];
  const wideFiles: string[] = [...unknown];
  for (const f of rust) {
    const d = diffFile(before.get(f) ?? null, after.get(f) ?? null);
    if (d.wide) wideFiles.push(f);
    itemChanges.push(...d.changes);
  }

  // The tree as the reference walk reads it: the stored scan, with every changed file as it is now. A change
  // to the scanner leaves every stored scan in its old shape, so the whole tree is scanned again and
  // the store keeps that scan once the run is recorded.
  const rescan = !until && global !== null && global.startsWith("tools/nv-scan/");
  const view = rescan ? new Map(scanItems(await rustFiles(root), root).map((f) => [f.file, f])) : new Map(files);
  for (const f of rust) {
    const now = after.get(f);
    if (now) view.set(f, now);
    else view.delete(f);
  }

  // What the build scripts made: a generated file that moved moves the item including it.
  const extra: ExtraDefines = new Map();
  const pkgDirs = new Map([...(graph?.values() ?? [])].map((p) => [p.name, p.dir]));
  const scripts = buildScripts(abs(`${COVWS_TARGET}/${hostTriple()}/debug/build`, root), pkgDirs);
  const newest = (paths: Change[]) => Math.max(0, ...paths.map((c) => (existsSync(join(root, c.path)) ? Bun.file(join(root, c.path)).lastModified : Date.now())));
  for (const g of until ? [] : generatedIncludes(view.values(), (f) => scope.pkgOf(f), root)) {
    const script = scripts.get(g.pkg);
    const inputsMoved = script ? changes.filter((c) => isInput(c.path, script.inputs)) : [];
    const recorded = store.meta(generatedMeta(store.platform, g));
    const now = generatedDigest(script, g.name);
    // The digests kept are the base's, so since a stamped tree any input that moved counts.
    const stale = inputsMoved.length > 0 && (from !== undefined || !script || script.ranAt < newest(inputsMoved));
    if (stale || recorded === null || recorded !== now) {
      const item = view.get(g.file)?.items.find((i) => i.id === g.id);
      if (item) itemChanges.push({ file: g.file, id: g.id, how: "changed", item });
      if (script) {
        const out = scanItems([g.name], join(script.dir, "out"))[0];
        if (out) extra.set(`${g.file}#${g.id}`, [...new Set(out.items.flatMap((i) => i.defines))]);
      }
    }
  }
  for (const script of scripts.values()) {
    const inputsMoved = changes.filter((c) => isInput(c.path, script.inputs));
    if (inputsMoved.length === 0) continue;
    const own = [...view.values()].filter((f) => scope.pkgOf(f.file) === script.pkg);
    for (const r of envReaders(own, script.envs, root)) {
      const item = view.get(r.file)?.items.find((i) => i.id === r.id);
      if (item) itemChanges.push({ file: r.file, id: r.id, how: "changed", item });
    }
  }

  const universe = new Universe(view, scope);
  const texts = gitTexts(since, until, root);
  const profile = profileReader(texts, view);
  const platform = profileReader(texts, view, elsewhereOnly(platformOf(process.platform)));
  for (const [k, o] of referenceWalk(itemChanges, universe, wideFiles, extra, profile, platform)) emit(k, o);
  // Code of a file no item held was recorded as the whole file.
  for (const c of itemChanges) emit(fileWild(c.file), { path: c.file, item: c.id, how: c.how });
  // A test that did not exist is in no footprint: every test binary it is compiled into runs.
  for (const c of itemChanges) {
    const pkg = c.how === "added" && isTest(c.item) ? scope.pkgOf(c.file) : null;
    if (pkg && graph) for (const name of landsIn(graph, pkg, c.file)) emit(testsKey(name), { path: c.file, item: c.id, how: c.how });
  }
  const stamped = fromBase && !from && tree ? await sinceStamps(store, tree, view, graph, root) : undefined;
  return {
    since,
    ...(until ? { until } : {}),
    changes,
    moved,
    global,
    ...(rescan ? { full: true } : {}),
    rustFiles: rust.length,
    itemChanges: itemChanges.length,
    wideFiles,
    items: itemChanges,
    view,
    ...(tree ? { tree } : {}),
    ...(stamped ? { stamped } : {}),
    root,
  };
}

/**
 * The change since each tree an atom is stamped with, to `now`, whose items `view` holds. The tree
 * `now` is moves nothing and the base's tree is left out. A tree whose row is gone, or whose change
 * cannot be read (its commit is no longer in the repository), moves everything.
 */
async function sinceStamps(store: SelectStore, now: { commit: string; overlay: Overlay }, view: Map<string, FileItems>, graph: Graph | null, root: string): Promise<Map<number, ChangeSet>> {
  const out = new Map<number, ChangeSet>();
  const trees = store.stampTrees();
  if (trees.size === 0) return out;
  const nowSig = treeSig(now);
  const base = store.base();
  const baseSig = base === null ? null : treeSig({ commit: base, overlay: store.overlay() });
  for (const [n, t] of trees) {
    if (t === null) out.set(n, everything(view, "the tree its last run was taken at is not kept"));
    else if (t.sig === nowSig) out.set(n, nothing(view));
    else if (t.sig !== baseSig) {
      try {
        out.set(n, await computeChange(store, { graph, root, from: { tree: t.tree, view, now } }));
      } catch (e) {
        out.set(n, everything(view, `the tree its last run was taken at could not be compared: ${(e as Error).message.split("\n")[0]}`));
      }
    }
  }
  return out;
}

/** What `trees` keeps for the tree `tree`, whose Rust items `view` holds. */
export function runTree(tree: { commit: string; overlay: Overlay }, view: Map<string, FileItems>, root: string = ROOT): RunTree {
  const items = Object.keys(tree.overlay).flatMap((p) => (tree.overlay[p] !== null && view.has(p) ? [view.get(p)!] : []));
  return { commit: tree.commit, overlay: tree.overlay, items, markers: overlayMarkers(tree.overlay, root) };
}

/** Whether item `i` can be a test a binary runs: a test-only function, or a test-only macro call that
 * may expand to tests. A `use`, a type or a constant only a test sees runs nothing of its own. */
export function isTest(i: Item): boolean {
  return i.test && (i.kind === "fn" || i.kind === "macro");
}

/** The meta key holding, for each path of the recorded overlay the proof scans read, the keys its
 * recorded text named (`markerKeys`), and for the perf ledger the digest of each feature's rows. */
const markersMeta = (platform: string) => `markers:${platform}`;

/** What the proof scans read of one file's text (`text` null for a file that is not there): the
 * `covers:`, `calls:` and `anchor:` keys it names, and the signature of its class-name consts. For
 * the perf ledger it is `rows`, the digest of each feature's rows (`ledgerSigns`). */
interface Scanned {
  keys: string[];
  consts: string;
  rows?: Record<string, string>;
}

function scanned(path: string, text: string | null): Scanned {
  if (path === LEDGER) return { keys: [], consts: "", rows: ledgerSigns(text) };
  const anchor = isAnchorFile(path) ? anchorScan(path, text ?? "") : { keys: [], consts: "" };
  return { keys: [...(text === null ? [] : markerKeys(path, text)), ...(text === null ? [] : anchor.keys)], consts: anchor.consts };
}

const readByScans = (path: string) => {
  const s = scannedFor(path);
  return s.markers || s.calls || isAnchorFile(path) || path === LEDGER;
};

/** Records, beside the overlay just recorded as the store's tree, what the proof scans read of each of
 * its paths (`Scanned`), so the next change knows what that text named. A path whose bytes moved since
 * the overlay was taken is left out, and its earlier text counts as unknown. */
export function recordOverlayMarkers(store: SelectStore, overlay: Overlay, root: string = ROOT): void {
  store.setMeta(markersMeta(store.platform), JSON.stringify(overlayMarkers(overlay, root)));
}

/** What the proof scans read of each path of `overlay` whose bytes on disk are still the ones it names. */
function overlayMarkers(overlay: Overlay, root: string): Record<string, Scanned> {
  const out: Record<string, Scanned> = {};
  for (const [path, digest] of Object.entries(overlay)) {
    if (!readByScans(path)) continue;
    if (digest === null) out[path] = scanned(path, null);
    else if (diskDigest(path, root) === digest) out[path] = scanned(path, readFileSync(join(root, path), "utf8"));
  }
  return out;
}

/** What the proof scans read of the base's overlay paths, as `recordOverlayMarkers` kept it. */
function storedMarkers(store: SelectStore): Record<string, Scanned> {
  try {
    return JSON.parse(store.meta(markersMeta(store.platform)) ?? "{}") as Record<string, Scanned>;
  } catch {
    return {};
  }
}

/** The `covers:`, `calls:`, `anchor:` and `perf:` keys `changes` move: what each changed file the proof
 * scans read names in its text before and after. A path of the recorded tree's `overlay` was as
 * `recorded` says, and one it leaves out is not known. A file whose earlier text is not known moves every
 * key of each kind it is scanned for, and a file whose class-name consts changed moves every `anchor:`
 * key. A change to the perf ledger moves `LEDGER_WHOLE` and the `perf:` key of each feature whose rows
 * differ, or every `perf:` key when its earlier text is not known. */
async function scanKeys(changes: Change[], since: string, until: string | null, overlay: Overlay, recorded: Record<string, Scanned>, root: string): Promise<Map<string, Origin>> {
  const out = new Map<string, Origin>();
  const text = (b: Uint8Array | null) => (b === null ? null : new TextDecoder().decode(b));
  for (const c of changes) {
    if (!readByScans(c.path)) continue;
    const s = scannedFor(c.path);
    const origin: Origin = { path: c.path, how: "path" };
    const before: Scanned | null = c.path in overlay ? (recorded[c.path] ?? null) : scanned(c.path, text(await blobAt(since, c.path, root)));
    const now = until ? text(await blobAt(until, c.path, root)) : existsSync(join(root, c.path)) ? readFileSync(join(root, c.path), "utf8") : null;
    const after = scanned(c.path, now);
    if (c.path === LEDGER) {
      out.set(LEDGER_WHOLE, origin);
      if (before?.rows === undefined) out.set(PERF_ANY, origin);
      else for (const k of ledgerMoved(before.rows, after.rows ?? {})) if (!out.has(k)) out.set(k, origin);
      continue;
    }
    if (before === null) {
      if (s.markers) out.set(COVERS_ANY, origin);
      if (s.calls) out.set(CALLS_ANY, origin);
    }
    if (isAnchorFile(c.path) && (before === null || before.consts !== after.consts)) out.set(ANCHOR_ANY, origin);
    for (const k of [...(before?.keys ?? []), ...after.keys]) if (!out.has(k)) out.set(k, origin);
  }
  return out;
}

/** The Rust files of `files` that hold an item embedding one of `paths` with `include_str!`,
 * `include_bytes!` or `include!`. The scan lists what an item embeds relative to the repository
 * root, the way a change names a path. */
export function embedders(files: Iterable<FileItems>, paths: Iterable<string>): string[] {
  const changed = new Set(paths);
  const out: string[] = [];
  for (const f of files) if (f.items.some((i) => i.includes?.some((p) => changed.has(p)))) out.push(f.file);
  return out;
}

/** Every Rust file of the tree, tracked or not. */
export async function rustFiles(root: string = ROOT): Promise<string[]> {
  const r = Bun.spawnSync(["git", "ls-files", "-co", "--exclude-standard", "-z", "--", "*.rs"], { cwd: root, stdout: "pipe" });
  return r.stdout
    .toString()
    .split("\0")
    .filter((p) => p && isRust(p) && existsSync(join(root, p)));
}

export type Why = "new" | "red" | "owed" | "diverged" | "def" | "key" | "global";

export interface Selected {
  id: string;
  kind: AtomKind;
  why: Why;
  /** For `key`, `owed`, `red` and `diverged`: each changed key the footprint holds, with where it came
   * from. */
  keys: { key: string; origin: Origin }[];
  /** For `owed`: the changed keys the atom was owed for, null or absent when it was owed for another
   * reason. */
  owed?: string[] | null;
}

/** The changed keys a selected atom executes, or null when it runs for a reason no key gives: what a
 * bench is skipped by when all of them are proven, and what an atom owed by this run keeps. */
export function changedItems(s: Selected): string[] | null {
  if (s.why === "key") return [...new Set(s.keys.map((k) => k.key))];
  if (s.why === "owed" && s.owed) return [...new Set([...s.owed, ...s.keys.map((k) => k.key)])];
  return null;
}

/**
 * Which atoms `change` may have given a new definition: each case, proof, test and bench one of its
 * paths is a file of, and each proof and bench beside a changed `nvs.toml`.
 */
export function touchesDef(change: ChangeSet): (id: string) => boolean {
  const ids = new Set<string>();
  for (const c of change.changes) {
    ids.add(`case:${c.path}`);
    ids.add(`nvtest:${c.path}`);
    ids.add(`proof:${c.path.replace(/\.(out|in)$/, ".nvs")}`);
    const bench = benchOf(c.path);
    if (bench) ids.add(`bench:${bench}`);
  }
  const tomlDirs = new Set(change.changes.filter((c) => c.path.endsWith("/nvs.toml")).map((c) => dirname(c.path)));
  return (id) => ids.has(id) || ((id.startsWith("proof:") || id.startsWith("bench:")) && tomlDirs.has(dirname(id.slice(id.indexOf(":") + 1))));
}

/**
 * Whether the change itself reaches a selected atom: it moved a key the atom's footprint holds, changed
 * the atom's definition (`touched`, from `touchesDef`), or changed a file every atom depends on. An atom
 * selected only because it was owed, red, diverged or never recorded before the change is not reached
 * by it.
 */
export function reachedByChange(s: Selected, touched: (id: string) => boolean): boolean {
  return s.why === "key" || s.why === "def" || s.why === "global" || s.keys.length > 0 || touched(s.id);
}

export interface Selection {
  change: ChangeSet;
  selected: Map<string, Selected>;
  /** Atoms known on this platform, by kind. */
  known: Record<string, number>;
  /** Known atoms that no longer exist: their file is gone from disk, or their kind is listed in full
   * (`QueryOptions.complete`) and the listing lacks them. The run that moves the tree forgets them. */
  gone: string[];
  /** Each atom read against the change since its own tree (`ChangeSet.stamped`), with the tree's row:
   * the run that moves the tree clears the stamp of each one it did not select. */
  stamps?: Map<string, number>;
}

export interface QueryOptions {
  /** Atoms that exist now: each one the store does not know runs as new. */
  discovered?: string[];
  /** The kinds `discovered` lists in full: a known atom of one of them that it does not list is gone. A
   * kind named by a file is gone when its file is, whatever this says. */
  complete?: AtomKind[];
  /** The current definition of each atom whose definition is no file: a plan check's own atom. A known
   * atom whose recorded definition differs runs. */
  defs?: Map<string, string>;
  /** The test binaries an atom that is no test binary runs (`<package> <kind> <target>`): a heavy check's
   * twin. It narrows a footprint that holds only its package's `tests:` key to those binaries. */
  ran?: Map<string, string[]>;
}

/**
 * The atoms `change` selects. An atom stamped with a tree is read against the change since that tree
 * (`ChangeSet.stamped`), so one that last ran on the tree as it is now is selected only by its verdict,
 * a divergence or a definition the caller names. Every run recorded through `store` from here on is
 * taken as run on the tree `change` was read at (`SelectStore.runAt`).
 */
export function query(store: SelectStore, change: ChangeSet, opts: QueryOptions = {}): Selection {
  if (change.tree) {
    const tree = change.tree;
    store.runAt(treeSig(tree), () => runTree(tree, change.view, change.root ?? ROOT));
  }
  const selected = new Map<string, Selected>();
  const add = (id: string, why: Why, key?: { key: string; origin: Origin }) => {
    const was = selected.get(id);
    if (was) {
      if (key && was.why !== "def" && was.why !== "new") was.keys.push(key);
      return;
    }
    selected.set(id, { id, kind: kindOfAtom(id), why, keys: key ? [key] : [] });
  };
  const atoms = store.atoms();
  const known: Record<string, number> = {};
  const gone: string[] = [];
  const knownIds = new Set<string>();
  // Each stamped atom is read against the change since the tree its last run was taken at.
  const stamps = change.stamped ? store.stamps() : new Map<string, number>();
  const changeOf = (id: string): ChangeSet => {
    const n = stamps.get(id);
    return (n !== undefined && change.stamped!.get(n)) || change;
  };
  // A definition is files, so only an atom whose files are among the changed paths can have a new one.
  const touchers = new Map<ChangeSet, (id: string) => boolean>();
  const touched = (c: ChangeSet, id: string) => (touchers.get(c) ?? touchers.set(c, touchesDef(c)).get(c)!)(id);
  const complete = new Set(opts.complete ?? []);
  const listed = new Set(complete.size > 0 ? (opts.discovered ?? []) : []);
  const present = (id: string, kind: AtomKind) => stillThere(id) && (!complete.has(kind) || listed.has(id));
  for (const a of atoms) {
    knownIds.add(a.id);
    if (!present(a.id, a.kind)) {
      gone.push(a.id);
      continue;
    }
    known[a.kind] = (known[a.kind] ?? 0) + 1;
    const c = changeOf(a.id);
    if (c.global) add(a.id, "global", { key: WILD, origin: c.moved.get(WILD)! });
    else if (a.keys === 0) add(a.id, "new");
    else if (a.verdict === "red") add(a.id, "red");
    else if (a.verdict === "owed") {
      add(a.id, "owed");
      selected.get(a.id)!.owed = a.owedItems;
    }
    else if (opts.defs?.has(a.id)) {
      if (opts.defs.get(a.id) !== a.def) add(a.id, "def");
    } else if (touched(c, a.id)) {
      const def = c.until ? null : currentDef(a.id);
      if (c.until || (def !== null && def !== a.def)) add(a.id, "def");
    }
  }
  for (const id of opts.discovered ?? []) {
    if (knownIds.has(id)) continue;
    knownIds.add(id);
    known[kindOfAtom(id)] = (known[kindOfAtom(id)] ?? 0) + 1;
    add(id, "new");
  }
  const goneIds = new Set(gone);
  // A footprint a diverged run recorded may be cut short, so the atom runs whatever changed.
  for (const id of store.divergences().keys()) if (knownIds.has(id) && !goneIds.has(id) && stillThere(id)) add(id, "diverged");
  for (const c of new Set([change, ...(change.stamped?.values() ?? [])])) {
    byKeys(store, c, (id) => changeOf(id) === c && !goneIds.has(id) && stillThere(id), atoms, add, opts);
  }
  return { change, selected, known, gone, stamps };
}

/** The prefix of every `fn:` key of the crate that holds the built-in components and loads them. */
export const BUILTIN_LOADER = "fn:crates/nvs-ext/src/";

/** Selects each atom `accept` takes whose footprint holds a key `change` moved. */
function byKeys(
  store: SelectStore,
  change: ChangeSet,
  accept: (id: string) => boolean,
  atoms: { id: string; kind: AtomKind }[],
  add: (id: string, why: Why, key?: { key: string; origin: Origin }) => void,
  opts: QueryOptions,
): void {
  if (change.global || change.moved.size === 0) return;
  // Code only an optimized build compiles never ran on the build that records, so no footprint holds it.
  const profile = change.moved.get(PROFILE_ONLY);
  if (profile) {
    for (const a of atoms) if (a.kind === "proof" && accept(a.id)) add(a.id, "key", { key: PROFILE_ONLY, origin: profile });
    for (const id of opts.discovered ?? []) if (kindOfAtom(id) === "proof" && accept(id)) add(id, "key", { key: PROFILE_ONLY, origin: profile });
  }

  const exact = new Map<string, number>();
  const lookup = (key: string) => {
    for (const [k, n] of store.keyIdsOf([key])) exact.set(k, n);
  };
  let anyClass = false;
  let anyCard = false;
  for (const key of change.moved.keys()) {
    if (key.endsWith("#")) {
      for (const [k, n] of store.keysWithPrefix(key)) exact.set(k, n);
      continue;
    }
    if (key === ALL_CLASSES) {
      for (const [k, n] of store.keysWithPrefix("class:")) exact.set(k, n);
      anyClass = true;
      continue;
    }
    if (key === ALL_CARDS) {
      for (const [k, n] of store.keysWithPrefix("card:")) exact.set(k, n);
      anyCard = true;
      continue;
    }
    if (key === ALL_COMPONENTS) {
      for (const [k, n] of store.keysWithPrefix("component:")) exact.set(k, n);
      continue;
    }
    if (key.startsWith("class:")) anyClass = true;
    if (key.startsWith("card:")) anyCard = true;
    if (key.startsWith("component:")) lookup(ALL_COMPONENTS);
    lookup(key);
  }
  // A footprint that read every class, every card or every component is under any one of them; one
  // that could not be attributed at all is under any change.
  if (anyClass) lookup(ALL_CLASSES);
  if (anyCard) lookup(ALL_CARDS);
  lookup(WILD);
  const originOf = (key: string): Origin => {
    const direct = change.moved.get(key);
    if (direct) return direct;
    if (key === ALL_CLASSES) return [...change.moved].find(([k]) => k.startsWith("class:"))![1];
    if (key === ALL_CARDS) return [...change.moved].find(([k]) => k.startsWith("card:"))![1];
    if (key === ALL_COMPONENTS) return [...change.moved].find(([k]) => k.startsWith("component:"))![1];
    if (key === WILD) return change.moved.values().next().value!;
    if (key.startsWith("class:")) return change.moved.get(ALL_CLASSES)!;
    if (key.startsWith("card:")) return change.moved.get(ALL_CARDS)!;
    if (key.startsWith("component:")) return change.moved.get(ALL_COMPONENTS)!;
    const prefix = key.startsWith("fn:") ? key.slice(0, key.indexOf("#") + 1) : "";
    return change.moved.get(prefix) ?? { path: "", how: "path" };
  };
  for (const [id, keys] of store.atomsUnderIds(exact)) {
    if (!accept(id)) continue;
    for (const key of keys) add(id, "key", { key, origin: originOf(key) });
  }
  // A footprint recorded before `component:` keys existed holds none: one that ran the built-in loader
  // and holds no `builtins:read` is read as holding them all, until its atom is recorded again.
  const component = [...change.moved].find(([k]) => k.startsWith("component:"));
  if (component) {
    const named = store.atomsUnder([BUILTINS_READ]);
    for (const id of store.atomsUnderIds(store.keysWithPrefix(BUILTIN_LOADER)).keys()) {
      if (accept(id) && !named.has(id)) add(id, "key", { key: component[0], origin: component[1] });
    }
  }
  // A footprint holding its package's one `tests:` key is under an added test only when a binary it runs
  // is one the test is compiled into. Which binaries that is: a test atom is one, a caller may know more
  // (`QueryOptions.ran`), and a footprint that holds a binary's own key has named them all itself.
  const pkgs = new Map<string, Origin>();
  for (const [key, origin] of change.moved) {
    const slash = key.indexOf("/");
    if (key.startsWith("tests:") && slash > 0 && !pkgs.has(key.slice(6, slash))) pkgs.set(key.slice(6, slash), origin);
  }
  for (const [pkg, origin] of pkgs) {
    for (const id of store.atomsUnder([pkgTestsKey(pkg)]).keys()) {
      if (!accept(id)) continue;
      const ran = kindOfAtom(id) === "test" ? [id.slice(5)] : opts.ran?.get(id);
      if (ran === undefined && store.footprint(id).some((k) => k.startsWith(`${pkgTestsKey(pkg)}/`))) continue;
      const hit = ran === undefined ? pkgTestsKey(pkg) : ran.map(testsKey).find((k) => change.moved.has(k));
      if (hit !== undefined) add(id, "key", { key: hit, origin: change.moved.get(hit) ?? origin });
    }
  }
}

/** What `discover` found: every atom the tree names now, and the kinds it lists in full. */
export interface Discovery {
  atoms: string[];
  /** The kinds whose every atom is in `atoms`, which `QueryOptions.complete` takes. */
  complete: AtomKind[];
}

/**
 * The atoms that exist now, read from directory listings and the cargo graph the caller already has:
 * every case file, every proof program (`proofFiles`), every bench (`benchFiles`), every tools test file
 * and the tools' `tsc`, and every test binary cargo would build. No file is read. A plan check's own atom is the plan's, which the
 * sweep adds. The test binaries are listed in full when there is a graph, so a known one cargo no longer
 * builds is gone.
 */
export function discover(graph: Graph | null, root: string = ROOT): Discovery {
  const atoms = [...caseFiles(root).map(caseId), ...proofFiles(root).map(proofId), ...benchFiles(root).map(benchId), ...nvTestFiles(root).map(nvTestId), NV_TSC];
  for (const pkg of graph?.keys() ?? []) for (const b of testBinaries(graph!, pkg)) atoms.push(`test:${b.name}`);
  return { atoms, complete: graph ? ["test"] : [] };
}

/** Counts of a selection: atoms per kind and reason, and the changed keys per kind. */
export function counts(sel: Selection): {
  selected: Record<string, number>;
  byWhy: Record<string, Record<string, number>>;
  known: Record<string, number>;
  movedKeys: Record<string, number>;
  changes: number;
  rustFiles: number;
  itemChanges: number;
} {
  const selected: Record<string, number> = {};
  const byWhy: Record<string, Record<string, number>> = {};
  for (const s of sel.selected.values()) {
    selected[s.kind] = (selected[s.kind] ?? 0) + 1;
    const w = (byWhy[s.why] ??= {});
    w[s.kind] = (w[s.kind] ?? 0) + 1;
  }
  const movedKeys: Record<string, number> = {};
  for (const k of sel.change.moved.keys()) movedKeys[kindOf(k)] = (movedKeys[kindOf(k)] ?? 0) + 1;
  return { selected, byWhy, known: sel.known, movedKeys, changes: sel.change.changes.length, rustFiles: sel.change.rustFiles, itemChanges: sel.change.itemChanges };
}

/** Why `id` was selected or not, as lines for a person. */
export function explain(store: SelectStore, sel: Selection, id: string): string[] {
  const out: string[] = [];
  const atom = store.atom(id);
  const s = sel.selected.get(id);
  if (!atom && !s) return [`${id}: not an atom this store knows on ${store.platform}`];
  if (s) {
    const whys: Record<Why, string> = {
      new: "it has no footprint on this platform yet",
      red: "its last run was red",
      owed: "an earlier change reached it and nothing has run it since",
      diverged: "its last recording run ended differently from the run it was judged on, so its footprint is not trusted",
      def: "its definition changed",
      key: "its footprint holds a key the change moved",
      global: "a file every atom depends on changed",
    };
    out.push(`${id}: selected, because ${whys[s.why]}`);
    for (const { key, origin } of s.keys.slice(0, 20)) out.push(`  ${key}  <- ${describe(origin)}`);
    if (s.keys.length > 20) out.push(`  ... and ${s.keys.length - 20} more key(s)`);
    return out;
  }
  const n = sel.stamps?.get(id);
  const own = n === undefined ? undefined : sel.change.stamped?.get(n);
  if (own && own.changes.length === 0 && own.moved.size === 0) return [`${id}: not selected; its last run was ${atom!.verdict} on the tree as it is now`];
  out.push(`${id}: not selected; its footprint holds ${atom!.keys} key(s) and none of them moved${own ? " since the tree its last run was taken at" : ""}`);
  const fp = store.footprint(id);
  const byFile = new Map<string, number>();
  for (const k of fp) if (k.startsWith("fn:")) byFile.set(k.slice(3, k.indexOf("#")), (byFile.get(k.slice(3, k.indexOf("#"))) ?? 0) + 1);
  for (const c of (own ?? sel.change).changes) {
    const held = byFile.get(c.path);
    if (held) out.push(`  ${c.path} changed; this atom ran ${held} item(s) of it, none of the ones that changed`);
    else if (isRust(c.path)) out.push(`  ${c.path} changed; this atom ran none of its code`);
    else out.push(`  ${c.path} changed; this atom did not read it`);
  }
  return out;
}

export function describe(o: Origin): string {
  const where = o.item ? `${o.path}#${o.item}` : o.path;
  const how = o.how === "path" ? "the path changed" : o.how === "global" ? "a global file" : o.how === "wide" ? "the file does not parse; taken whole" : `item ${o.how}`;
  return `${where} (${how}${o.via ? `, through ${o.via}` : ""})`;
}
