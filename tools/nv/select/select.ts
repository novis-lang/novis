// The selection: from what changed since a recorded tree to the atoms that have to run, and why.
//
//     changed = diff(recorded tree, working tree incl. untracked)
//            -> each .rs file: item diff -> reference-graph closure -> fn:, class:, card: keys
//            -> each build script whose inputs moved: the items that include what it generated
//            -> every path: file:, tree:, named:, and exists:/dir: for a path that came or went
//     run = atoms never recorded on this platform, last red, or owed from an earlier change
//         + atoms marked diverged: a recording run that ended differently from the judged run
//         + atoms whose definition changed
//         + atoms indexed under any changed key
//         + every atom, when a global file changed
//         + every proof program, when code only an optimized build compiles changed and has no twin
//
// Nothing outside `run` is started. `explain` says for one atom which key selected it, the path and
// item the key came from, and for a key reached through the reference graph the item that reached it.

import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { type FileItems, scanItems } from "../keys/scan.ts";
import { metadata, type Graph, closure as pkgClosure, testBinaries } from "../keys/graph.ts";
import { COVWS_TARGET, hostTriple } from "../lib/covws.ts";
import { abs, NOT_INPUTS, ROOT } from "../lib/paths.ts";
import { caseDef, caseFiles, caseId, currentDef, stillThere } from "./atoms.ts";
import { buildScripts, envReaders, generatedDigest, generatedIncludes, generatedMeta, isInput } from "./build.ts";
import { blobAt, type Change, changedBetween, changedPaths, commitOf, namedChanges, sinceOverlay, snapshot } from "./change.ts";
import { closure, diffFile, type ExtraDefines, type ItemChange, type Moved, type Origin, type Scope, Universe } from "./items.ts";
import { gitTexts, profileReader } from "./profile.ts";
import { ALL_CARDS, ALL_CLASSES, fileWild, kindOf, pathKeys, PROFILE_ONLY, testsKey, WILD } from "./keys.ts";
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
  /** A change that stands for everything: the store had no tree to compare with, and `view` is a scan
   * of every Rust file. */
  full?: boolean;
}

/** The workspace graph as the closure's scope: a file's package is the one whose directory holds it. */
export function graphScope(graph: Graph | null): Scope {
  if (!graph) return { pkgOf: () => null, sees: () => true };
  const dirs = [...graph.values()].map((p) => [p.dir, p.name] as const).sort((a, b) => b[0].length - a[0].length);
  const cache = new Map<string, string | null>();
  const deps = new Map<string, Set<string>>();
  return {
    pkgOf(file) {
      if (cache.has(file)) return cache.get(file)!;
      const hit = dirs.find(([d]) => d === "" || d === "." || file === d || file.startsWith(`${d}/`));
      const pkg = hit ? hit[1] : null;
      cache.set(file, pkg);
      return pkg;
    },
    sees(user, owner) {
      if (user === owner) return true;
      let seen = deps.get(user);
      if (!seen) {
        seen = pkgClosure(graph, user, true);
        deps.set(user, seen);
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
  if (global) emit(WILD, { path: global, how: "global" });

  const graph = opts.graph === undefined ? await metadata() : opts.graph;
  const scope = graphScope(graph);
  const stored = store.allItems();
  const rust = changes.filter((c) => isRust(c.path)).map((c) => c.path);
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

  // The tree as the closure reads it: the stored scan, with every changed file as it is now.
  let files = stored;
  if (files.size === 0) files = new Map(scanItems(await rustFiles(root), root).map((f) => [f.file, f]));
  const view = new Map(files);
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
  const profile = profileReader(gitTexts(since, until, root), view);
  for (const [k, o] of closure(itemChanges, universe, wideFiles, extra, profile)) emit(k, o);
  // Code of a file no item held was recorded as the whole file.
  for (const c of itemChanges) emit(fileWild(c.file), { path: c.file, item: c.id, how: c.how });
  // A test that did not exist is in no footprint: every test binary of its package runs.
  for (const c of itemChanges) {
    const pkg = c.how === "added" && c.item.test ? scope.pkgOf(c.file) : null;
    if (pkg) emit(testsKey(pkg), { path: c.file, item: c.id, how: c.how });
  }
  return {
    since,
    ...(until ? { until } : {}),
    changes,
    moved,
    global,
    rustFiles: rust.length,
    itemChanges: itemChanges.length,
    wideFiles,
    items: itemChanges,
    view,
    ...(tree ? { tree } : {}),
  };
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
  /** Known atoms whose definition is gone from disk. */
  gone: string[];
}

export interface QueryOptions {
  /** Atoms that exist now beyond the ones the store knows, with their definitions: new ones run. */
  discovered?: { id: string; def: string }[];
  /** The current definition of each atom whose definition is no file: a plan check's own atom. A known
   * atom whose recorded definition differs runs. */
  defs?: Map<string, string>;
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
  for (const a of atoms) {
    knownIds.add(a.id);
    if (!stillThere(a.id)) {
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
  for (const d of opts.discovered ?? []) {
    if (knownIds.has(d.id)) continue;
    known[kindOfAtom(d.id)] = (known[kindOfAtom(d.id)] ?? 0) + 1;
    add(d.id, "new");
  }
  // A footprint a diverged run recorded may be cut short, so the atom runs whatever changed.
  for (const id of store.divergences().keys()) if (knownIds.has(id) && stillThere(id)) add(id, "diverged");
  if (change.global || change.moved.size === 0) return { change, selected, known, gone };
  // Code only an optimized build compiles never ran on the build that records, so no footprint holds it.
  const profile = change.moved.get(PROFILE_ONLY);
  if (profile) {
    for (const a of atoms) if (a.kind === "proof" && stillThere(a.id)) add(a.id, "key", { key: PROFILE_ONLY, origin: profile });
    for (const d of opts.discovered ?? []) if (kindOfAtom(d.id) === "proof") add(d.id, "key", { key: PROFILE_ONLY, origin: profile });
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
    if (!stillThere(id)) continue;
    for (const key of keys) add(id, "key", { key, origin: originOf(key) });
  }
  return { change, selected, known, gone };
}

/** The atoms that exist now: every case file, every test binary cargo would build, and whatever the store
 * knows of the other kinds. A proof program or a `bun nv` check the store does not know yet is found by
 * the seed, which asks the roster and the plan. */
export async function discover(graph: Graph | null, root: string = ROOT): Promise<{ id: string; def: string }[]> {
  const out: { id: string; def: string }[] = caseFiles(root).map((p) => ({ id: caseId(p), def: caseDef(p, root) }));
  for (const pkg of graph?.keys() ?? []) for (const b of testBinaries(graph!, pkg)) out.push({ id: `test:${b.name}`, def: "" });
  return out;
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
  const where = o.item ? `${o.path}#${o.item}` : o.path;
  const how = o.how === "path" ? "the path changed" : o.how === "global" ? "a global file" : o.how === "wide" ? "the file does not parse; taken whole" : `item ${o.how}`;
  return `${where} (${how}${o.via ? `, through ${o.via}` : ""})`;
}
