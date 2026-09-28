// The selection: from what changed since a recorded tree to the atoms that have to run, and why.
//
//     changed = diff(recorded tree, working tree incl. untracked)
//            -> each .rs file: item diff -> reference-graph closure -> fn:, class:, card: keys
//            -> each build script whose inputs moved: the items that include what it generated
//            -> every path: file:, config:, tree:, named:, ext:, and exists:/dir: for a path that came or went
//            -> the repository's nvs.toml: config: and app: for the parts that changed (`config.ts`)
//     run = atoms never recorded on this platform, last red, or owed from an earlier change
//         + atoms marked diverged: a recording run that ended differently from the judged run
//         + atoms whose definition changed
//         + atoms indexed under any changed key
//         + every atom, when a global file changed
//         + every proof program, when code only an optimized build compiles changed and has no twin
//
// Nothing outside `run` is started. `explain` says for one atom which key selected it, the path and
// item the key came from, and for a key reached through the reference graph the item that reached it.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { type FileItems, type Item, scanItems } from "../keys/scan.ts";
import { landsIn, metadata, type Graph, closure as pkgClosure, testBinaries } from "../keys/graph.ts";
import { COVWS_TARGET, hostTriple } from "../lib/covws.ts";
import { abs, NOT_INPUTS, ROOT } from "../lib/paths.ts";
import { caseFiles, caseId, currentDef, nvTestFiles, nvTestId, proofFiles, proofId, stillThere } from "./atoms.ts";
import { NV_TSC } from "./nvtests.ts";
import { buildScripts, envReaders, generatedDigest, generatedIncludes, generatedMeta, isInput } from "./build.ts";
import { ANCHOR_ANY, CALLS_ANY, COVERS_ANY, markerKeys, scannedFor } from "../proofs/markers.ts";
import { LEDGER, LEDGER_WHOLE, ledgerMoved, ledgerSigns, PERF_ANY } from "../proofs/ledger.ts";
import { anchorScan, isAnchorFile } from "../proofs/roster.ts";
import { blobAt, type Change, changedBetween, changedPaths, commitOf, diskDigest, namedChanges, sinceOverlay, snapshot } from "./change.ts";
import { closure, diffFile, type ExtraDefines, type ItemChange, type Moved, type Origin, type Scope, Universe } from "./items.ts";
import { elsewhereOnly, gitTexts, platformOf, profileReader } from "./profile.ts";
import { ALL_CARDS, ALL_CLASSES, configKey, fileWild, kindOf, pathKeys, pkgTestsKey, PROFILE_ONLY, ROOT_CONFIG, testsKey, WILD } from "./keys.ts";
import { blockPaths, configKeys } from "./config.ts";
import { type AtomKind, kindOfAtom, type Overlay, type SelectStore } from "./store.ts";

/** Files whose change selects every atom: the toolchain, the lock file, a manifest, and the two tools
 * every footprint's meaning rests on. */
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
}

/** The workspace graph as the closure's scope: a file's package is the one whose directory holds it, a
 * package's crate name is its library target's, and a file under its package's `tests/`, `benches/` or
 * `examples/`, or the root of a target of those kinds, is built only into that dev target. */
export function graphScope(graph: Graph | null): Scope {
  if (!graph) return { pkgOf: () => null, sees: () => true, modDir: () => null };
  const dirs = [...graph.values()].map((p) => [p.dir, p.name] as const).sort((a, b) => b[0].length - a[0].length);
  const roots = new Set([...graph.values()].flatMap((p) => p.targets.map((t) => t.src)));
  const devRoots = new Set([...graph.values()].flatMap((p) => p.targets.filter((t) => ["test", "bench", "example"].includes(t.kind)).map((t) => t.src)));
  const crates = new Map([...graph.values()].map((p) => [(p.targets.find((t) => t.kind === "lib")?.name ?? p.name).replace(/-/g, "_"), p.name]));
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
      const dir = pkg === null ? "" : graph.get(pkg)!.dir;
      const rest = dir === "" || dir === "." ? file : file.slice(dir.length + 1);
      return /^(tests|benches|examples)\//.test(rest);
    },
    modDir(file) {
      if (roots.has(file) || file.endsWith("/mod.rs")) return file.slice(0, file.lastIndexOf("/") + 1);
      return `${file.replace(/\.rs$/, "")}/`;
    },
    pkgOf,
    sees(user, owner, dev = true) {
      if (user === owner) return true;
      const key = `${dev ? "dev" : "ship"}\0${user}`;
      let seen = deps.get(key);
      if (!seen) {
        seen = pkgClosure(graph, user, dev);
        deps.set(key, seen);
      }
      return seen.has(owner);
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
async function rootConfigKeys(store: SelectStore, changes: Change[], since: string, until: string | null, fromBase: boolean, root: string): Promise<string[]> {
  const edited = changes.some((c) => c.path === ROOT_CONFIG);
  const gone = changes.filter((c) => c.status === "deleted").map((c) => c.path.toLowerCase());
  if (!edited && gone.length === 0) return [];
  const text = (bytes: Uint8Array | null) => (bytes === null ? null : new TextDecoder().decode(bytes));
  const onDisk = join(root, ROOT_CONFIG);
  const after = until ? text(await blobAt(until, ROOT_CONFIG, root)) : existsSync(onDisk) ? readFileSync(onDisk, "utf8") : null;
  let keys: string[] = [];
  if (edited) {
    const before = fromBase && ROOT_CONFIG in store.overlay() ? null : text(await blobAt(since, ROOT_CONFIG, root));
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
}

/** The keys that moved between `since` and the working tree. */
export async function computeChange(store: SelectStore, opts: ChangeOptions = {}): Promise<ChangeSet> {
  const root = opts.root ?? ROOT;
  const base = store.base();
  const sinceRev = opts.since ?? base;
  if (!sinceRev) throw new Error("the store has no recorded tree: run `bun nv select --seed` first, or name one with --since");
  const since = await commitOf(sinceRev, root);
  const until = opts.until ? await commitOf(opts.until, root) : null;
  // The tree is taken first, so a file written while the change is read is a change again next time.
  const tree = until ? undefined : await snapshot(root);
  const fromBase = opts.since === undefined && !until && !opts.paths;
  let changes = until ? await changedBetween(since, until, root) : opts.paths ? await namedChanges(opts.paths, since, root) : await changedPaths(since, root);
  if (fromBase) changes = sinceOverlay(changes, store.overlay(), root);
  const moved: Moved = new Map();
  const emit = (key: string, origin: Origin) => {
    if (!moved.has(key)) moved.set(key, origin);
  };
  const global = changes.find((c) => isGlobal(c.path))?.path ?? null;
  for (const c of changes) for (const k of pathKeys(c.path, c.status !== "modified")) emit(k, { path: c.path, how: "path" });
  for (const k of await rootConfigKeys(store, changes, since, until, fromBase, root)) emit(k, { path: ROOT_CONFIG, how: "path" });
  if (global) emit(WILD, { path: global, how: "global" });
  for (const [k, origin] of await scanKeys(store, changes, since, until, fromBase, root)) emit(k, origin);

  const graph = opts.graph === undefined ? await metadata() : opts.graph;
  const scope = graphScope(graph);
  const stored = store.allItems();
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
  const scanIsSince = base !== null && (fromBase || (since === (await commitOf(base, root)) && Object.keys(store.overlay()).length === 0));
  if (scanIsSince && stored.size > 0) {
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
  const wideFiles: string[] = [];
  for (const f of rust) {
    const d = diffFile(before.get(f) ?? null, after.get(f) ?? null);
    if (d.wide) wideFiles.push(f);
    itemChanges.push(...d.changes);
  }

  // The tree as the closure reads it: the stored scan, with every changed file as it is now. A change
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
    const stale = inputsMoved.length > 0 && (!script || script.ranAt < newest(inputsMoved));
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
  for (const [k, o] of closure(itemChanges, universe, wideFiles, extra, profile, platform)) emit(k, o);
  // Code of a file no item held was recorded as the whole file.
  for (const c of itemChanges) emit(fileWild(c.file), { path: c.file, item: c.id, how: c.how });
  // A test that did not exist is in no footprint: every test binary it is compiled into runs.
  for (const c of itemChanges) {
    const pkg = c.how === "added" && isTest(c.item) ? scope.pkgOf(c.file) : null;
    if (pkg && graph) for (const name of landsIn(graph, pkg, c.file)) emit(testsKey(name), { path: c.file, item: c.id, how: c.how });
  }
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
  };
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
  const out: Record<string, Scanned> = {};
  for (const [path, digest] of Object.entries(overlay)) {
    if (!readByScans(path)) continue;
    if (digest === null) out[path] = scanned(path, null);
    else if (diskDigest(path, root) === digest) out[path] = scanned(path, readFileSync(join(root, path), "utf8"));
  }
  store.setMeta(markersMeta(store.platform), JSON.stringify(out));
}

/** The `covers:`, `calls:`, `anchor:` and `perf:` keys `changes` move: what each changed file the proof
 * scans read names in its text before and after. A file whose earlier text is not known moves every
 * key of each kind it is scanned for, and a file whose class-name consts changed moves every `anchor:`
 * key. A change to the perf ledger moves `LEDGER_WHOLE` and the `perf:` key of each feature whose rows
 * differ, or every `perf:` key when its earlier text is not known. */
async function scanKeys(store: SelectStore, changes: Change[], since: string, until: string | null, fromBase: boolean, root: string): Promise<Map<string, Origin>> {
  const out = new Map<string, Origin>();
  const overlay = fromBase ? store.overlay() : {};
  let recorded: Record<string, Scanned> = {};
  try {
    recorded = fromBase ? JSON.parse(store.meta(markersMeta(store.platform)) ?? "{}") : {};
  } catch {
    recorded = {};
  }
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
  /** For `key`: each changed key the footprint holds, with where it came from. */
  keys: { key: string; origin: Origin }[];
}

export interface Selection {
  change: ChangeSet;
  selected: Map<string, Selected>;
  /** Atoms known on this platform, by kind. */
  known: Record<string, number>;
  /** Known atoms that no longer exist: their file is gone from disk, or their kind is listed in full
   * (`QueryOptions.complete`) and the listing lacks them. The run that moves the tree forgets them. */
  gone: string[];
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

/** The atoms `change` selects. */
export function query(store: SelectStore, change: ChangeSet, opts: QueryOptions = {}): Selection {
  const selected = new Map<string, Selected>();
  const add = (id: string, why: Why, key?: { key: string; origin: Origin }) => {
    const was = selected.get(id);
    if (was) {
      if (key && was.why === "key") was.keys.push(key);
      return;
    }
    selected.set(id, { id, kind: kindOfAtom(id), why, keys: key ? [key] : [] });
  };
  const atoms = store.atoms();
  const known: Record<string, number> = {};
  const gone: string[] = [];
  const knownIds = new Set<string>();
  // A definition is files, so only an atom whose files are among the changed paths can have a new one.
  const defTouched = new Set<string>();
  for (const c of change.changes) {
    defTouched.add(`case:${c.path}`);
    defTouched.add(`nvtest:${c.path}`);
    defTouched.add(`proof:${c.path.replace(/\.(out|in)$/, ".nvs")}`);
  }
  const tomlDirs = new Set(change.changes.filter((c) => c.path.endsWith("/nvs.toml")).map((c) => dirname(c.path)));
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
    if (change.global) add(a.id, "global", { key: WILD, origin: change.moved.get(WILD)! });
    else if (a.keys === 0) add(a.id, "new");
    else if (a.verdict === "red") add(a.id, "red");
    else if (a.verdict === "owed") add(a.id, "owed");
    else if (opts.defs?.has(a.id)) {
      if (opts.defs.get(a.id) !== a.def) add(a.id, "def");
    } else if (defTouched.has(a.id) || (a.kind === "proof" && tomlDirs.has(dirname(a.id.slice(6))))) {
      const def = change.until ? null : currentDef(a.id);
      if (change.until || (def !== null && def !== a.def)) add(a.id, "def");
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
  if (change.global || change.moved.size === 0) return { change, selected, known, gone };
  // Code only an optimized build compiles never ran on the build that records, so no footprint holds it.
  const profile = change.moved.get(PROFILE_ONLY);
  if (profile) {
    for (const a of atoms) if (a.kind === "proof" && stillThere(a.id)) add(a.id, "key", { key: PROFILE_ONLY, origin: profile });
    for (const id of opts.discovered ?? []) if (kindOfAtom(id) === "proof") add(id, "key", { key: PROFILE_ONLY, origin: profile });
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
    if (key.startsWith("class:")) anyClass = true;
    if (key.startsWith("card:")) anyCard = true;
    lookup(key);
  }
  // A footprint that read every class, or every card, is under any one of them; one that could not be
  // attributed at all is under any change.
  if (anyClass) lookup(ALL_CLASSES);
  if (anyCard) lookup(ALL_CARDS);
  lookup(WILD);
  const originOf = (key: string): Origin => {
    const direct = change.moved.get(key);
    if (direct) return direct;
    if (key === ALL_CLASSES) return [...change.moved].find(([k]) => k.startsWith("class:"))![1];
    if (key === ALL_CARDS) return [...change.moved].find(([k]) => k.startsWith("card:"))![1];
    if (key === WILD) return change.moved.values().next().value!;
    if (key.startsWith("class:")) return change.moved.get(ALL_CLASSES)!;
    if (key.startsWith("card:")) return change.moved.get(ALL_CARDS)!;
    const prefix = key.startsWith("fn:") ? key.slice(0, key.indexOf("#") + 1) : "";
    return change.moved.get(prefix) ?? { path: "", how: "path" };
  };
  for (const [id, keys] of store.atomsUnderIds(exact)) {
    if (goneIds.has(id) || !stillThere(id)) continue;
    for (const key of keys) add(id, "key", { key, origin: originOf(key) });
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
      if (goneIds.has(id) || !stillThere(id)) continue;
      const ran = kindOfAtom(id) === "test" ? [id.slice(5)] : opts.ran?.get(id);
      if (ran === undefined && store.footprint(id).some((k) => k.startsWith(`${pkgTestsKey(pkg)}/`))) continue;
      const hit = ran === undefined ? pkgTestsKey(pkg) : ran.map(testsKey).find((k) => change.moved.has(k));
      if (hit !== undefined) add(id, "key", { key: hit, origin: change.moved.get(hit) ?? origin });
    }
  }
  return { change, selected, known, gone };
}

/** What `discover` found: every atom the tree names now, and the kinds it lists in full. */
export interface Discovery {
  atoms: string[];
  /** The kinds whose every atom is in `atoms`, which `QueryOptions.complete` takes. */
  complete: AtomKind[];
}

/**
 * The atoms that exist now, read from directory listings and the cargo graph the caller already has:
 * every case file, every proof program (`proofFiles`), every tools test file and the tools' `tsc`, and
 * every test binary cargo would build. No file is read. A plan check's own atom is the plan's, which the
 * sweep adds. The test binaries are listed in full when there is a graph, so a known one cargo no longer
 * builds is gone.
 */
export function discover(graph: Graph | null, root: string = ROOT): Discovery {
  const atoms = [...caseFiles(root).map(caseId), ...proofFiles(root).map(proofId), ...nvTestFiles(root).map(nvTestId), NV_TSC];
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
  out.push(`${id}: not selected; its footprint holds ${atom!.keys} key(s) and none of them moved`);
  const fp = store.footprint(id);
  const byFile = new Map<string, number>();
  for (const k of fp) if (k.startsWith("fn:")) byFile.set(k.slice(3, k.indexOf("#")), (byFile.get(k.slice(3, k.indexOf("#"))) ?? 0) + 1);
  for (const c of sel.change.changes) {
    const held = byFile.get(c.path);
    if (held) out.push(`  ${c.path} changed; this atom ran ${held} item(s) of it, none of the ones that changed`);
    else if (isRust(c.path)) out.push(`  ${c.path} changed; this atom ran none of its code`);
    else out.push(`  ${c.path} changed; this atom did not read it`);
  }
  return out;
}

export function describe(o: Origin): string {
  if (o.how === "cadence") return "the safety net's cadence: every heavy check runs on this goal end";
  const where = o.item ? `${o.path}#${o.item}` : o.path;
  const how = o.how === "path" ? "the path changed" : o.how === "global" ? "a global file" : o.how === "wide" ? "the file does not parse; taken whole" : `item ${o.how}`;
  return `${where} (${how}${o.via ? `, through ${o.via}` : ""})`;
}
