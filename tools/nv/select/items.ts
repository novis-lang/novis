// From changed Rust files to changed keys: the item diff and the reference-graph closure.
//
// A file is compared item by item (`nv-scan --items`): an item whose id is new was added, one whose id
// is gone was removed, and one whose digest moved was changed. Doc comments are outside the digest, so
// editing one is no change. A file that does not parse on either side is taken whole: every atom that
// ran any of its code is selected, and so is everything that reads a class or a card.
//
// Every changed item moves its own `fn:` key, which is what coverage recorded. What coverage cannot see
// is closed over the names items spell (`refs`) and bind (`defines`):
//
// - a `fn` stops there: its callers ran it, so their atoms hold its key already;
// - a card (`cards`) moves `card:` keys and nothing else, so a card edit re-runs no program;
// - a class table (`rows`) moves the `class:` key of each row whose digest moved, compared by position;
//   a table reached through an item that names a class adds nothing, since the rows naming that item
//   are its class's, and one reached through any other name moves every row's class, since which row
//   named it is not known;
// - an item that names a class (`class`) moves that `class:` key and the items of its own file that
//   name it: programs reach a row by lookup, and code of the same module may read the const directly;
// - a `macro_rules!` moves every item that invokes it;
// - an `impl` block's or a macro invocation's own tokens move every item inside it (`parent`);
// - a `use` moves the items of its own file that name what it binds;
// - any other item (a const, a static, a struct, an enum, a type alias) moves every item that names
//   it, in its own package and in the packages that depend on it, and so on until a `fn` is reached.
//
// An item is reached only from where Rust lets its name resolve to the item that moved. A `private`
// item (`scope`) is named from its own module's files alone, and a `crate` item from its own package;
// another file's item of the same name is another item. A file of no package (vendored code, a tool
// with a workspace of its own) names no item of the workspace, except under `fuzz/`, whose targets
// depend on the workspace's crates. A `macro_rules!` is scoped by where it is defined and exported, not
// by `scope`, and keeps the package rule alone.
//
// An item compiled only for tests (`test`) is never a card or a class row a program reads: its cards,
// classes and rows are ignored and it moves what names it, like any other item. A test that was added
// is in no footprint yet; `select.ts` moves its package's `tests:` key for it.
//
// An item only an optimized build compiles (`profile.ts`) never ran on the debug build every footprint
// is recorded on. It moves its twin as well, the item of its file with the same id but for the `#N`
// ordinal that a debug build compiles; with no twin it moves `PROFILE_ONLY`, which selects every proof
// program.

import type { FileItems, Item, ItemRow } from "../keys/scan.ts";
import { ALL_CARDS, ALL_CLASSES, cardKey, classKey, fnKey, itemPrefix, PROFILE_ONLY } from "./keys.ts";
import { baseId, NO_PROFILE, type ProfileOnly, shipsIn } from "./profile.ts";

export type How = "added" | "removed" | "changed" | "reached";

export interface ItemChange {
  file: string;
  id: string;
  how: How;
  item: Item;
  /** The item as it was, for a changed or removed one. */
  was?: Item;
}

/** Why a key moved: the path and item it came from, and for a reached item the item that reached it. */
export interface Origin {
  path: string;
  item?: string;
  how: How | "path" | "global" | "build" | "wide";
  via?: string;
}

/** Keys that moved, each with the first reason found. A key ending in `#` is a prefix: every key of that
 * file. */
export type Moved = Map<string, Origin>;

/** The item-level difference between one file's two scans. `wide` is set when either side did not parse,
 * and then the whole file is taken. */
export function diffFile(base: FileItems | null, head: FileItems | null): { changes: ItemChange[]; wide: boolean } {
  const file = (head ?? base)!.file;
  if ((base && !base.parsed && base.raw !== "") || (head && !head.parsed && head.raw !== "")) return { changes: [], wide: true };
  const before = new Map((base?.items ?? []).map((i) => [i.id, i]));
  const after = new Map((head?.items ?? []).map((i) => [i.id, i]));
  const changes: ItemChange[] = [];
  for (const [id, item] of after) {
    const was = before.get(id);
    if (!was) changes.push({ file, id, how: "added", item });
    else if (was.digest !== item.digest || JSON.stringify(was.rows ?? []) !== JSON.stringify(item.rows ?? [])) changes.push({ file, id, how: "changed", item, was });
  }
  for (const [id, was] of before) if (!after.has(id)) changes.push({ file, id, how: "removed", item: was, was });
  return { changes, wide: false };
}

/** Which packages can name an item of which file. */
export interface Scope {
  /** The package a file belongs to, or null for a file of no package. */
  pkgOf(file: string): string | null;
  /** Whether code of `user` can name items of `owner`: the same package, or one that depends on it. */
  sees(user: string, owner: string): boolean;
  /** The prefix every file of `file`'s module and the modules inside it starts with: `a/b/` for
   * `a/b.rs`, and the file's own directory for a crate root or a `mod.rs`. Null when the crate roots
   * are not known, and then a private item is reached as a `crate` one. */
  modDir(file: string): string | null;
}

/** Every package sees every other: the scope a caller without the workspace graph uses. */
export const OPEN_SCOPE: Scope = { pkgOf: () => null, sees: () => true, modDir: () => null };

interface Placed {
  file: string;
  item: Item;
}

/** The items of the tree as the closure reads them, indexed by the names they spell and by parent. */
export class Universe {
  private byName = new Map<string, Placed[]>();
  private byParent = new Map<string, Placed[]>();

  constructor(
    readonly files: Map<string, FileItems>,
    readonly scope: Scope = OPEN_SCOPE,
  ) {
    for (const f of files.values()) {
      for (const item of f.items) {
        const placed = { file: f.file, item };
        for (const r of new Set(item.refs)) (this.byName.get(r) ?? this.byName.set(r, []).get(r)!).push(placed);
        if (item.parent) {
          const k = `${f.file}#${item.parent}`;
          (this.byParent.get(k) ?? this.byParent.set(k, []).get(k)!).push(placed);
        }
      }
    }
  }

  naming(name: string): Placed[] {
    return this.byName.get(name) ?? [];
  }

  children(file: string, id: string): Placed[] {
    return this.byParent.get(`${file}#${id}`) ?? [];
  }

  /** The other items of `file` whose id is `id` but for nv-scan's `#N` ordinal. */
  twins(file: string, id: string): Item[] {
    const base = baseId(id);
    return (this.files.get(file)?.items ?? []).filter((i) => i.id !== id && baseId(i.id) === base);
  }
}

/** The classes of the rows that differ between two versions of a class table: each row one side has and
 * the other lacks, so a row added or taken out moves that row alone. When the rows both sides share
 * are in another order, every position whose row moved counts, since a table's order can be what a
 * reader indexes by. */
export function rowClasses(before: ItemRow[] | undefined, after: ItemRow[] | undefined): Set<string> {
  const out = new Set<string>();
  const a = before ?? [];
  const b = after ?? [];
  const count = (rows: ItemRow[]) => {
    const m = new Map<string, number>();
    for (const r of rows) m.set(r.digest, (m.get(r.digest) ?? 0) + 1);
    return m;
  };
  const inA = count(a);
  const inB = count(b);
  const only = (rows: ItemRow[], other: Map<string, number>) => {
    const left = new Map(other);
    const shared: string[] = [];
    for (const r of rows) {
      const n = left.get(r.digest) ?? 0;
      if (n > 0) {
        left.set(r.digest, n - 1);
        shared.push(r.digest);
      } else for (const c of r.classes) out.add(c);
    }
    return shared;
  };
  const sharedA = only(a, inB);
  const sharedB = only(b, inA);
  if (sharedA.join("\n") === sharedB.join("\n")) return out;
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    if (a[i]?.digest === b[i]?.digest) continue;
    for (const c of a[i]?.classes ?? []) out.add(c);
    for (const c of b[i]?.classes ?? []) out.add(c);
  }
  return out;
}

/** Extra names an item binds that its own tokens do not show: the items of a generated file it
 * includes. Keyed by `<file>#<id>`. */
export type ExtraDefines = Map<string, string[]>;

/**
 * The keys `changes` move, closed over the reference graph. `wideFiles` are files taken whole, and
 * `profile` says which items only an optimized build compiles.
 */
export function closure(changes: ItemChange[], universe: Universe, wideFiles: string[] = [], extra: ExtraDefines = new Map(), profile: ProfileOnly = NO_PROFILE): Moved {
  const moved: Moved = new Map();
  const emit = (key: string, origin: Origin) => {
    if (!moved.has(key)) moved.set(key, origin);
  };
  for (const file of wideFiles) {
    const origin: Origin = { path: file, how: "wide" };
    emit(itemPrefix(file), origin);
    emit(ALL_CLASSES, origin);
    emit(ALL_CARDS, origin);
  }
  const seen = new Set<string>();
  const queue: { file: string; item: Item; was?: Item; how: How; via?: string; viaClasses?: string[] }[] = [];
  for (const c of changes) queue.push({ file: c.file, item: c.item, ...(c.was ? { was: c.was } : {}), how: c.how });
  const reach = (from: { file: string; item: Item }, names: string[], sameFile: boolean) => {
    const viaClasses = from.item.class ?? [];
    const owner = universe.scope.pkgOf(from.file);
    for (const name of names) {
      if (name === "*" || name === "") continue;
      for (const p of universe.naming(name)) {
        if (p.file === from.file && p.item.id === from.item.id) continue;
        if (sameFile && p.file !== from.file) continue;
        if (!sameFile && owner !== null && p.file !== from.file) {
          const user = universe.scope.pkgOf(p.file);
          if (user === null ? !p.file.startsWith("fuzz/") : !universe.scope.sees(user, owner)) continue;
          const scope = from.item.kind === "macro-rules" ? undefined : from.item.scope;
          if (scope !== undefined && user !== owner) continue;
          const dir = scope === "private" ? universe.scope.modDir(from.file) : null;
          if (dir !== null && !p.file.startsWith(dir)) continue;
        }
        queue.push({ file: p.file, item: p.item, how: "reached", via: `${from.file}#${from.item.id}`, viaClasses });
      }
    }
  };
  while (queue.length > 0) {
    const e = queue.shift()!;
    const id = `${e.file}#${e.item.id}`;
    if (seen.has(id)) continue;
    seen.add(id);
    const origin: Origin = { path: e.file, item: e.item.id, how: e.how, ...(e.via ? { via: e.via } : {}) };
    emit(fnKey(e.file, e.item.id), origin);
    const shipped = !e.item.test;
    if (shipped && shipsIn(e.file) && profile(e.file, e.item, e.how === "removed" ? "base" : "head")) {
      const twins = universe.twins(e.file, e.item.id).filter((t) => !profile(e.file, t, "head"));
      for (const t of twins) queue.push({ file: e.file, item: t, how: "reached", via: id });
      if (twins.length === 0) emit(PROFILE_ONLY, origin);
    }
    if (shipped && (e.item.cards?.length || e.was?.cards?.length)) {
      for (const c of [...(e.item.cards ?? []), ...(e.was?.cards ?? [])]) emit(cardKey(c), origin);
      continue;
    }
    if (shipped && (e.item.rows?.length || e.was?.rows?.length)) {
      if (e.how === "reached") {
        // Reached through an item that names a class, the rows naming it are that class's, which moved
        // already. Reached through anything else, which row names it is not known.
        if (!e.viaClasses?.length) for (const r of e.item.rows ?? []) for (const c of r.classes) emit(classKey(c), origin);
        continue;
      }
      const classes = e.how === "changed" ? rowClasses(e.was?.rows, e.item.rows) : rowClasses(undefined, e.item.rows);
      for (const c of classes) emit(classKey(c), origin);
      // A table whose tokens moved outside every row changed something every row shares.
      if (e.how === "changed" && classes.size === 0) emit(ALL_CLASSES, origin);
      continue;
    }
    if (shipped && (e.item.class?.length || e.was?.class?.length)) {
      for (const c of [...(e.item.class ?? []), ...(e.was?.class ?? [])]) emit(classKey(c), origin);
      reach(e, e.item.defines, true);
      continue;
    }
    for (const child of universe.children(e.file, e.item.id)) queue.push({ file: child.file, item: child.item, how: "reached", via: id });
    const defines = [...e.item.defines, ...(extra.get(id) ?? [])];
    switch (e.item.kind) {
      case "fn":
        break;
      case "macro-rules":
        reach(e, [e.item.id.replace(/!.*$/, "").split("::").pop()!], false);
        break;
      case "use":
        reach(e, defines, true);
        break;
      case "impl-rest":
        break;
      default:
        reach(e, defines, false);
    }
  }
  return moved;
}
