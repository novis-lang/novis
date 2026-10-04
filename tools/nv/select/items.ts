// From changed Rust files to changed keys: the item diff and the reference-graph walk.
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
// - an item whose tokens moved only in its links to cards (`bare`, compared by `cardsOnly`) moves the
//   `card:` keys of the classes it or its moved rows name, or `card:*` when it names none, and no
//   `class:` key; a `use` whose card imports alone changed is paired across its new id (`pairUses`)
//   and moves only what names those card types. Such an item is still reached like any other when
//   something it names moved beside it;
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
// depend on the workspace's crates. Code another package ships sees the packages its build depends on,
// and its test code (`test`, or a file of a test, bench or example target) its dev-dependencies too.
// There it reaches an item only with evidence it names that item and not one of its own spelled the
// same (`Universe.canName`): its tokens spell a crate name that leads to the item, or a `use` of its
// file binds the name, a glob or a `_` from such a crate, or from its own modules or an external crate
// once some item of its package brings the name in from one. A crate leads to the item when it is the
// item's own or one built against it that binds the name or a glob, since that may re-export it. A
// `macro_rules!` is scoped by where it is defined and exported, not by `scope`, and keeps the package
// rule alone, as do a macro invocation and what it generates, whose tokens may spell a path.
//
// An item compiled only for tests (`test`) is never a card or a class row a program reads: its cards,
// classes and rows are ignored and it moves what names it, like any other item. A test that was added
// is in no footprint yet; `select.ts` moves the `tests:` key of each test binary it is compiled into.
//
// An item only an optimized build compiles (`profile.ts`) never ran on the debug build every footprint
// is recorded on. It moves its twin as well, the item of its file with the same id but for the `#N`
// ordinal that a debug build compiles; with no twin it moves `PROFILE_ONLY`, which selects every proof
// program.

import type { FileItems, Item, ItemRow } from "../keys/scan.ts";
import { ALL_CARDS, ALL_CLASSES, cardKey, classKey, fnKey, itemPrefix, PLATFORM_ONLY, PROFILE_ONLY } from "./keys.ts";
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

/** Why a key moved: the path and item it came from, and for a reached item the item that reached it.
 * `cadence` is no change at all: the loop's safety net picks every heavy check on its cadence. */
export interface Origin {
  path: string;
  item?: string;
  how: How | "path" | "global" | "build" | "wide" | "cadence";
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
  return { changes: pairUses(changes), wide: false };
}

/** An item's digest without its links to cards. */
const plain = (i: Item) => i.bare ?? i.digest;

/** Whether `item` differs from `was` in its links to cards alone: the tokens moved, the tokens without
 * those links did not, and neither did the classes it names. */
export function cardsOnly(was: Item | undefined, item: Item): boolean {
  if (!was || was.digest === item.digest || plain(was) !== plain(item)) return false;
  const classes = (i: Item) => JSON.stringify([i.class ?? [], (i.rows ?? []).map((r) => r.classes)]);
  return classes(was) === classes(item);
}

/** A `use` whose card imports changed binds other names, so its id moved and the diff reads it as one
 * `use` removed and another added. Paired by what they import besides the cards, in the same module,
 * the two are one `use` that changed. A pair that is not the only one of its kind is left apart. */
function pairUses(changes: ItemChange[]): ItemChange[] {
  const carded = (c: ItemChange) => c.item.kind === "use" && (c.item.cardTypes?.length ?? 0) > 0;
  const key = (c: ItemChange) => `${c.id.slice(0, c.id.lastIndexOf("use:"))}\0${c.item.test}\0${plain(c.item)}`;
  const removed = new Map<string, ItemChange[]>();
  const added = new Map<string, ItemChange[]>();
  for (const c of changes) {
    if (c.item.kind !== "use" || (c.how !== "removed" && c.how !== "added")) continue;
    const side = c.how === "removed" ? removed : added;
    (side.get(key(c)) ?? side.set(key(c), []).get(key(c))!).push(c);
  }
  const paired = new Map<ItemChange, ItemChange | null>();
  for (const [k, gone] of removed) {
    const came = added.get(k);
    if (gone.length !== 1 || came?.length !== 1 || !(carded(gone[0]!) || carded(came[0]!))) continue;
    paired.set(came[0]!, { file: came[0]!.file, id: came[0]!.id, how: "changed", item: came[0]!.item, was: gone[0]!.item });
    paired.set(gone[0]!, null);
  }
  if (paired.size === 0) return changes;
  return changes.flatMap((c) => {
    const p = paired.get(c);
    return p === undefined ? [c] : p === null ? [] : [p];
  });
}

/** Which packages can name an item of which file. */
export interface Scope {
  /** The package a file belongs to, or null for a file of no package. */
  pkgOf(file: string): string | null;
  /** Whether code of `user` can name items of `owner`: the same package, or one it depends on. With
   * `dev` false its own dev-dependencies are left out, which is what its shipped code is built against. */
  sees(user: string, owner: string, dev?: boolean): boolean;
  /** The package a crate name (`nvs_test`) is in code of `user`, or null for a name that is no
   * workspace crate. Absent when crate names are not known, and then a package reaches every item of
   * the packages it sees by name alone. */
  crateNamed?(ident: string, user: string): string | null;
  /** Whether `file` is built only into a test, bench or example target of its package. */
  devFile?(file: string): boolean;
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

/** The items of the tree as the reference walk reads them, indexed by the names they spell and by parent. */
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

  private uses = new Map<string, Item[]>();
  private pkgFiles: Map<string | null, FileItems[]> | null = null;
  private memo = new Map<string, boolean>();

  private remember(key: string, f: () => boolean): boolean {
    const hit = this.memo.get(key);
    if (hit !== undefined) return hit;
    // A cycle of re-exports ends as "no" while it is being worked out.
    this.memo.set(key, false);
    const v = f();
    this.memo.set(key, v);
    return v;
  }

  private filesOf(pkg: string): FileItems[] {
    if (!this.pkgFiles) {
      this.pkgFiles = new Map();
      for (const f of this.files.values()) {
        const p = this.scope.pkgOf(f.file);
        (this.pkgFiles.get(p) ?? this.pkgFiles.set(p, []).get(p)!).push(f);
      }
    }
    return this.pkgFiles.get(pkg) ?? [];
  }

  private usesOf(file: string): Item[] {
    let u = this.uses.get(file);
    if (!u) this.uses.set(file, (u = (this.files.get(file)?.items ?? []).filter((i) => i.kind === "use")));
    return u;
  }

  /** Whether crate name `ident` leads code of `user` to `names` of `owner`: true for `owner` itself, and
   * for another crate built against `owner` that binds one of `names` or a glob in a `use` of its own,
   * since that may re-export it; false for any other workspace crate; null for a name that is no
   * workspace crate, which is a module of `user` or an external crate. */
  private viaCrate(ident: string, user: string, owner: string, names: string[], key: string): boolean | null {
    const pkg = this.scope.crateNamed!(ident, user);
    if (pkg === null || pkg === user) return null;
    if (pkg === owner) return true;
    if (!this.scope.sees(pkg, owner, false)) return false;
    return this.remember(`rebinds\0${pkg}\0${key}`, () =>
      this.filesOf(pkg).some((f) => f.items.some((i) => !i.test && i.kind === "use" && i.defines.some((d) => d === "*" || names.includes(d)))),
    );
  }

  /** Whether an item of `user` spells a crate name that leads to `owner` (`viaCrate`) beside one of
   * `names`, as a `use` binding one of them, a glob, a `_` or a name of its own does: the evidence that
   * a `use` of `user` rooted in its own modules can bind them. */
  private pkgImports(user: string, owner: string, names: string[], key: string): boolean {
    return this.remember(`pkg\0${user}\0${key}`, () =>
      this.filesOf(user).some((f) =>
        f.items.some(
          (i) =>
            (i.defines.some((d) => d === "*" || d === "_" || names.includes(d)) || i.refs.some((r) => names.includes(r))) &&
            i.refs.some((r) => this.viaCrate(r, user, owner, names, key) === true),
        ),
      ),
    );
  }

  /**
   * Whether item `p` of package `user` can name what item `key` of another package `owner` binds as
   * `names`. It can when its own tokens spell a crate name that leads there (a qualified path), or when a
   * `use` of its file binds one of `names`, a glob or a `_`: from a crate name that leads there, or from
   * a module of `user` or an external crate, when some item of `user` brings one of `names` in from a
   * crate that leads there. A glob from a crate that does not lead there binds none of them.
   */
  canName(p: Placed, user: string, owner: string, names: string[], key: string): boolean {
    if (!this.scope.crateNamed) return true;
    if (p.item.refs.some((r) => this.viaCrate(r, user, owner, names, key) === true)) return true;
    return this.remember(`file\0${p.file}\0${key}`, () => {
      for (const u of this.usesOf(p.file)) {
        if (!u.defines.some((d) => d === "*" || d === "_" || names.includes(d))) continue;
        let crate = false;
        for (const r of u.refs) {
          const v = this.viaCrate(r, user, owner, names, key);
          if (v === null) continue;
          crate = true;
          if (v) return true;
        }
        if (!crate && this.pkgImports(user, owner, names, key)) return true;
      }
      return false;
    });
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
 * The keys `changes` move, closed over the reference graph. `wideFiles` are files taken whole,
 * `profile` says which items only an optimized build compiles, and `platform` which items only a Linux
 * build does: each one the walk reaches, test code included, moves `PLATFORM_ONLY`.
 */
export function referenceWalk(
  changes: ItemChange[],
  universe: Universe,
  wideFiles: string[] = [],
  extra: ExtraDefines = new Map(),
  profile: ProfileOnly = NO_PROFILE,
  platform: ProfileOnly = NO_PROFILE,
): Moved {
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
    const macro = from.item.kind === "macro-rules";
    const key = `${from.file}#${from.item.id}`;
    for (const name of names) {
      if (name === "*" || name === "") continue;
      for (const p of universe.naming(name)) {
        if (p.file === from.file && p.item.id === from.item.id) continue;
        if (sameFile && p.file !== from.file) continue;
        if (!sameFile && owner !== null && p.file !== from.file) {
          const user = universe.scope.pkgOf(p.file);
          const dev = p.item.test || (universe.scope.devFile?.(p.file) ?? true);
          if (user === null ? !p.file.startsWith("fuzz/") : !universe.scope.sees(user, owner, dev)) continue;
          const scope = macro ? undefined : from.item.scope;
          if (scope !== undefined && user !== owner) continue;
          const dir = scope === "private" ? universe.scope.modDir(from.file) : null;
          if (dir !== null && !p.file.startsWith(dir)) continue;
          // A macro's tokens may spell a path the reader cannot see, so an invocation and what it
          // generates keep the package rule, and so does a `macro_rules!`, which is named unqualified.
          const generated = p.item.kind === "macro" || (p.item.parent?.includes("!") ?? false);
          if (user !== null && user !== owner && !macro && !generated && !universe.canName(p, user, owner, names, key)) continue;
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
    if (platform(e.file, e.item, e.how === "removed" ? "base" : "head")) emit(PLATFORM_ONLY, origin);
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
    if (shipped && e.how === "changed" && cardsOnly(e.was, e.item)) {
      // Only its links to cards moved, and a program reads no card: a `use` moves what names the card
      // types it imports, and anything else the cards of the classes it or its moved rows name.
      if (e.item.kind === "use") reach(e, [...new Set([...(e.was?.cardTypes ?? []), ...(e.item.cardTypes ?? [])])], true);
      else {
        const classes = e.item.rows?.length ? rowClasses(e.was?.rows, e.item.rows) : new Set(e.item.class ?? []);
        for (const c of classes) emit(cardKey(c), origin);
        if (classes.size === 0) emit(ALL_CARDS, origin);
      }
      // What its tokens name may still have moved, as an import that changed beside the link does, and
      // then it is reached like any other item.
      seen.delete(id);
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
