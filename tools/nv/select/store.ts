// The observed-selection store: `.cache/select.sqlite`, one `bun:sqlite` database in WAL mode.
//
// It lives beside `.cache/nv.sqlite`, never inside it: the index deletes its own file whenever its layout
// changes, and losing the footprints would mean seeding every atom again.
//
// | table | holds |
// |---|---|
// | `meta` | `schema`, the tree the store last recorded (`base:<platform>`), the `covws` build, and each generated file's digest |
// | `atoms` | one row per atom and platform: its kind, its definition's digest, its last verdict and run, and its own key list |
// | `keys` | every key any footprint holds, numbered, with the digest it had when last recorded |
// | `footprint` | `(key, atom)`: the reverse index a selection reads, keyed on the key |
// | `items` | each Rust file's items as `nv-scan --items` read them at the base tree, per platform |
// | `verdicts` | a slot's digest and verdict, for the memos this store replaces |
//
// An atom is `<kind>:<name>`: `case:tests/conformance/a.nvst`, `proof:docs/examples/.../01-x.nvs`,
// `test:nvs-cli test footprint`, `nv:<check id>`, `nvtest:<file>` for one `bun test` file of the tools,
// `step:<name>` for a step of `bun nv verify`, `check:<check id>` for another plan check, and
// `heavy:<check id>` for one of the heavy set or a Linux leg. A
// footprint is per platform, because what an atom does differs between Windows and the Linux leg.
//
// The recorded tree is a commit and an overlay: `base:<platform>` names the commit, and
// `overlay:<platform>` holds, for every path that differed from it when the tree was recorded, the
// digest of its bytes then, or null for a path that was absent. So a run over uncommitted work records
// that work, and reverting it is a change.
//
// An atom's own key list is kept as a blob of key numbers beside the reverse index, so a new run can
// replace or widen one atom's footprint by point writes rather than a scan of the whole index.

import { Database } from "bun:sqlite";
import { mkdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import type { FileItems } from "../keys/scan.ts";
import { CACHE } from "../lib/paths.ts";

export const SCHEMA = "1";

/** The store's file. */
export const STORE = join(CACHE, "select.sqlite");

export type AtomKind = "case" | "proof" | "test" | "nv" | "nvtest" | "step" | "check" | "heavy";
export const ATOM_KINDS: AtomKind[] = ["case", "proof", "test", "nv", "nvtest", "step", "check", "heavy"];

/**
 * `owed` is an atom a change reached that the run which recorded the change did not run: the base
 * moved past the change, so the atom stays selected by its verdict until a run of it is green. A red
 * atom stays `red` when it is owed as well.
 */
export type Verdict = "green" | "red" | "owed" | "";

export interface AtomRow {
  id: string;
  kind: AtomKind;
  def: string;
  verdict: Verdict;
  /** Milliseconds since the epoch of the last recorded run, 0 for none. */
  lastRun: number;
  /** How many keys its footprint holds, 0 for an atom never recorded. */
  keys: number;
}

/** `win32` or `linux` and so on, as `process.platform` names it: the platform a footprint belongs to. */
export function currentPlatform(): string {
  return process.platform;
}

/** The kind an atom id names. */
export function kindOfAtom(id: string): AtomKind {
  return id.slice(0, id.indexOf(":")) as AtomKind;
}

const DDL = `
  CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS atoms (
    n INTEGER PRIMARY KEY,
    id TEXT NOT NULL,
    platform TEXT NOT NULL,
    kind TEXT NOT NULL,
    def TEXT NOT NULL DEFAULT '',
    verdict TEXT NOT NULL DEFAULT '',
    last_run INTEGER NOT NULL DEFAULT 0,
    nkeys INTEGER NOT NULL DEFAULT 0,
    keys BLOB,
    UNIQUE (id, platform)
  );
  CREATE TABLE IF NOT EXISTS keys (n INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE, digest TEXT NOT NULL DEFAULT '');
  CREATE TABLE IF NOT EXISTS footprint (key INTEGER NOT NULL, atom INTEGER NOT NULL, PRIMARY KEY (key, atom)) WITHOUT ROWID;
  CREATE TABLE IF NOT EXISTS items (platform TEXT NOT NULL, file TEXT NOT NULL, raw TEXT NOT NULL, json TEXT NOT NULL, PRIMARY KEY (platform, file));
  CREATE TABLE IF NOT EXISTS verdicts (slot TEXT PRIMARY KEY, digest TEXT NOT NULL, verdict TEXT NOT NULL DEFAULT '', at INTEGER NOT NULL);
`;

/** A footprint as it is recorded: each key with the digest it had, `""` where a key has none. */
export type Keyed = Map<string, string>;

/** Each path that differed from the base commit when the tree was recorded: its digest, or null. */
export type Overlay = Record<string, string | null>;

export class SelectStore {
  readonly db: Database;
  readonly platform: string;
  private keyIds = new Map<string, number>();
  private keyDigests = new Map<string, string>();

  constructor(
    readonly file: string = STORE,
    platform: string = currentPlatform(),
  ) {
    this.platform = platform;
    if (file !== ":memory:") mkdirSync(dirname(file), { recursive: true });
    this.db = new Database(file);
    // A sweep's own process and the `bun nv` processes it starts write the store at once, so a writer
    // waits for another's transaction rather than failing on it.
    this.db.exec("PRAGMA busy_timeout = 60000");
    this.db.exec("PRAGMA journal_mode = WAL");
    this.db.exec("PRAGMA synchronous = NORMAL");
    const schema = this.tryMeta("schema");
    if (schema !== null && schema !== SCHEMA) {
      // A store of another layout is read by nothing here; dropping it costs a reseed, never a wrong answer.
      for (const t of ["meta", "atoms", "keys", "footprint", "items", "verdicts"]) this.db.exec(`DROP TABLE IF EXISTS ${t}`);
    }
    this.db.exec(DDL);
    this.setMeta("schema", SCHEMA);
  }

  close(): void {
    this.db.close();
  }

  /** Writes a copy of the whole store to `path`, which must not exist. */
  copyTo(path: string): void {
    this.db.query("VACUUM INTO ?").run(path);
  }

  private tryMeta(key: string): string | null {
    try {
      const row = this.db.query("SELECT value FROM meta WHERE key = ?").get(key) as { value: string } | null;
      return row?.value ?? null;
    } catch {
      return null;
    }
  }

  meta(key: string): string | null {
    return this.tryMeta(key);
  }

  setMeta(key: string, value: string): void {
    this.db.query("INSERT INTO meta (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value").run(key, value);
  }

  /** The commit whose tree this platform's footprints and items were last recorded against. */
  base(): string | null {
    return this.meta(`base:${this.platform}`);
  }

  /** Records the tree: the commit, and every path that differed from it with its digest then. */
  setBase(commit: string, overlay: Overlay = {}): void {
    this.transaction(() => {
      this.setMeta(`base:${this.platform}`, commit);
      this.setMeta(`overlay:${this.platform}`, JSON.stringify(overlay));
    });
  }

  /** The recorded tree's overlay; empty for a store that recorded a commit alone. */
  overlay(): Overlay {
    const raw = this.meta(`overlay:${this.platform}`);
    if (!raw) return {};
    try {
      const got = JSON.parse(raw) as unknown;
      return got && typeof got === "object" && !Array.isArray(got) ? (got as Overlay) : {};
    } catch {
      return {};
    }
  }

  transaction<T>(body: () => T): T {
    return this.db.transaction(body)();
  }

  // ---- keys ----------------------------------------------------------------------------------------

  /** The number of `key`, made when it is new. */
  keyId(key: string, digest = ""): number {
    const known = this.keyIds.get(key);
    if (known !== undefined && (digest === "" || this.keyDigests.get(key) === digest)) return known;
    const row = this.db
      .query("INSERT INTO keys (key, digest) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET digest = CASE WHEN excluded.digest = '' THEN keys.digest ELSE excluded.digest END RETURNING n")
      .get(key, digest) as { n: number };
    this.keyIds.set(key, row.n);
    if (digest !== "") this.keyDigests.set(key, digest);
    return row.n;
  }

  /** The numbers of the keys that exist of `keys`; a key no footprint ever held has none. */
  keyIdsOf(keys: Iterable<string>): Map<string, number> {
    const out = new Map<string, number>();
    const q = this.db.query("SELECT n FROM keys WHERE key = ?");
    for (const k of keys) {
      const row = q.get(k) as { n: number } | null;
      if (row) out.set(k, row.n);
    }
    return out;
  }

  /** Every key that starts with `prefix`, with its number. */
  keysWithPrefix(prefix: string): Map<string, number> {
    const out = new Map<string, number>();
    const rows = this.db.query("SELECT key, n FROM keys WHERE key >= ? AND key < ?").all(prefix, `${prefix}\u{10FFFF}`) as { key: string; n: number }[];
    for (const r of rows) out.set(r.key, r.n);
    return out;
  }

  keyDigest(key: string): string | null {
    const row = this.db.query("SELECT digest FROM keys WHERE key = ?").get(key) as { digest: string } | null;
    return row?.digest ?? null;
  }

  // ---- atoms ---------------------------------------------------------------------------------------

  private row(r: { id: string; kind: string; def: string; verdict: string; last_run: number; nkeys: number }): AtomRow {
    return { id: r.id, kind: r.kind as AtomKind, def: r.def, verdict: r.verdict as Verdict, lastRun: r.last_run, keys: r.nkeys };
  }

  atom(id: string): AtomRow | null {
    const r = this.db.query("SELECT id, kind, def, verdict, last_run, nkeys FROM atoms WHERE id = ? AND platform = ?").get(id, this.platform) as any;
    return r ? this.row(r) : null;
  }

  atoms(kind?: AtomKind): AtomRow[] {
    const rows = (
      kind
        ? this.db.query("SELECT id, kind, def, verdict, last_run, nkeys FROM atoms WHERE platform = ? AND kind = ? ORDER BY id").all(this.platform, kind)
        : this.db.query("SELECT id, kind, def, verdict, last_run, nkeys FROM atoms WHERE platform = ? ORDER BY id").all(this.platform)
    ) as any[];
    return rows.map((r) => this.row(r));
  }

  private atomN(id: string): number | null {
    const r = this.db.query("SELECT n FROM atoms WHERE id = ? AND platform = ?").get(id, this.platform) as { n: number } | null;
    return r?.n ?? null;
  }

  /** Makes the atom known, with no footprint, when it is new. */
  ensureAtom(id: string, def = ""): void {
    this.db.query("INSERT INTO atoms (id, platform, kind, def) VALUES (?, ?, ?, ?) ON CONFLICT (id, platform) DO NOTHING").run(id, this.platform, kindOfAtom(id), def);
  }

  /** The keys of an atom's footprint, sorted. */
  footprint(id: string): string[] {
    const r = this.db.query("SELECT keys FROM atoms WHERE id = ? AND platform = ?").get(id, this.platform) as { keys: Uint8Array | null } | null;
    if (!r?.keys) return [];
    const ids = unpack(r.keys);
    const q = this.db.query("SELECT key FROM keys WHERE n = ?");
    return ids.map((n) => (q.get(n) as { key: string }).key).sort();
  }

  /** For each atom on this platform, of `kind` when given, the keys of its footprint that start with
   * `prefix`, sorted. An atom holding none is absent. Read from the reverse index, key by key. */
  keysUnder(prefix: string, kind?: AtomKind): Map<string, string[]> {
    const out = new Map<string, string[]>();
    const rows = this.db
      .query("SELECT k.key AS key, a.id AS id FROM keys k JOIN footprint f ON f.key = k.n JOIN atoms a ON a.n = f.atom WHERE k.key >= ? AND k.key < ? AND a.platform = ?" + (kind ? " AND a.kind = ?" : ""))
      .all(...([prefix, `${prefix}\u{10FFFF}`, this.platform, ...(kind ? [kind] : [])] as string[])) as { key: string; id: string }[];
    for (const r of rows) (out.get(r.id) ?? out.set(r.id, []).get(r.id)!).push(r.key);
    for (const keys of out.values()) keys.sort();
    return out;
  }

  /**
   * Records one run of `id`. A run whose definition differs from the one recorded starts the footprint
   * afresh; any other run widens it by what this run used, since a footprint is the union of the runs
   * since the definition last changed. A red run still widens it: a wider footprint only selects more.
   */
  recordRun(id: string, run: { def: string; verdict: Verdict; keys: Keyed; at?: number }): void {
    this.transaction(() => {
      this.ensureAtom(id, run.def);
      const n = this.atomN(id)!;
      const was = this.db.query("SELECT def, keys FROM atoms WHERE n = ?").get(n) as { def: string; keys: Uint8Array | null };
      const old = was.keys ? unpack(was.keys) : [];
      const fresh = was.def !== run.def;
      const ids = new Set<number>(fresh ? [] : old);
      for (const [k, d] of run.keys) ids.add(this.keyId(k, d));
      const del = this.db.query("DELETE FROM footprint WHERE key = ? AND atom = ?");
      const ins = this.db.query("INSERT OR IGNORE INTO footprint (key, atom) VALUES (?, ?)");
      if (fresh) for (const k of old) if (!ids.has(k)) del.run(k, n);
      const oldSet = new Set(old);
      for (const k of ids) if (fresh || !oldSet.has(k)) ins.run(k, n);
      const sorted = [...ids].sort((a, b) => a - b);
      this.db
        .query("UPDATE atoms SET def = ?, verdict = ?, last_run = ?, nkeys = ?, keys = ? WHERE n = ?")
        .run(run.def, run.verdict, run.at ?? Date.now(), sorted.length, pack(sorted), n);
    });
  }

  /** Sets an atom's verdict without touching its footprint. */
  setVerdict(id: string, verdict: Verdict): void {
    this.db.query("UPDATE atoms SET verdict = ?, last_run = ? WHERE id = ? AND platform = ?").run(verdict, Date.now(), id, this.platform);
  }

  /** Marks each known atom of `ids` owed, keeping a red one red; returns how many it marked. An atom
   * the store does not know has no footprint, and is selected as new without a mark. */
  owe(ids: Iterable<string>): number {
    let n = 0;
    const q = this.db.query("UPDATE atoms SET verdict = 'owed' WHERE id = ? AND platform = ? AND verdict != 'red'");
    this.transaction(() => {
      for (const id of ids) n += q.run(id, this.platform).changes;
    });
    return n;
  }

  /** Forgets an atom and its footprint. */
  removeAtom(id: string): void {
    this.transaction(() => {
      const n = this.atomN(id);
      if (n === null) return;
      const r = this.db.query("SELECT keys FROM atoms WHERE n = ?").get(n) as { keys: Uint8Array | null };
      const del = this.db.query("DELETE FROM footprint WHERE key = ? AND atom = ?");
      for (const k of r.keys ? unpack(r.keys) : []) del.run(k, n);
      this.db.query("DELETE FROM atoms WHERE n = ?").run(n);
    });
  }

  /** Takes `keys` out of every footprint on this platform: items that no longer exist. */
  prune(keys: Iterable<string>): number {
    let removed = 0;
    this.transaction(() => {
      const ids = this.keyIdsOf(keys);
      const touched = new Map<number, Set<number>>();
      for (const kn of ids.values()) {
        const rows = this.db.query("SELECT f.atom AS atom FROM footprint f JOIN atoms a ON a.n = f.atom WHERE f.key = ? AND a.platform = ?").all(kn, this.platform) as { atom: number }[];
        for (const r of rows) {
          (touched.get(r.atom) ?? touched.set(r.atom, new Set()).get(r.atom)!).add(kn);
          this.db.query("DELETE FROM footprint WHERE key = ? AND atom = ?").run(kn, r.atom);
          removed++;
        }
      }
      for (const [atom, gone] of touched) {
        const r = this.db.query("SELECT keys FROM atoms WHERE n = ?").get(atom) as { keys: Uint8Array | null };
        const left = (r.keys ? unpack(r.keys) : []).filter((k) => !gone.has(k));
        this.db.query("UPDATE atoms SET keys = ?, nkeys = ? WHERE n = ?").run(pack(left), left.length, atom);
      }
    });
    return removed;
  }

  /**
   * The reverse index: for each atom on this platform whose footprint holds one of `keys`, the keys of
   * `keys` it holds.
   */
  atomsUnder(keys: Iterable<string>): Map<string, string[]> {
    return this.atomsUnderIds(this.keyIdsOf(keys));
  }

  /** As `atomsUnder`, for keys already numbered. */
  atomsUnderIds(ids: Map<string, number>): Map<string, string[]> {
    const out = new Map<string, string[]>();
    const q = this.db.query("SELECT a.id AS id FROM footprint f JOIN atoms a ON a.n = f.atom WHERE f.key = ? AND a.platform = ?");
    for (const [key, n] of ids) {
      for (const r of q.all(n, this.platform) as { id: string }[]) (out.get(r.id) ?? out.set(r.id, []).get(r.id)!).push(key);
    }
    return out;
  }

  // ---- items ---------------------------------------------------------------------------------------

  setItems(files: FileItems[]): void {
    const q = this.db.query("INSERT INTO items (platform, file, raw, json) VALUES (?, ?, ?, ?) ON CONFLICT (platform, file) DO UPDATE SET raw = excluded.raw, json = excluded.json");
    this.transaction(() => {
      for (const f of files) q.run(this.platform, f.file, f.raw, JSON.stringify(f));
    });
  }

  /** Replaces this platform's whole scan. */
  replaceItems(files: FileItems[]): void {
    this.transaction(() => {
      this.db.query("DELETE FROM items WHERE platform = ?").run(this.platform);
      this.setItems(files);
    });
  }

  removeItems(files: string[]): void {
    const q = this.db.query("DELETE FROM items WHERE platform = ? AND file = ?");
    this.transaction(() => {
      for (const f of files) q.run(this.platform, f);
    });
  }

  items(file: string): FileItems | null {
    const r = this.db.query("SELECT json FROM items WHERE platform = ? AND file = ?").get(this.platform, file) as { json: string } | null;
    return r ? (JSON.parse(r.json) as FileItems) : null;
  }

  allItems(): Map<string, FileItems> {
    const out = new Map<string, FileItems>();
    for (const r of this.db.query("SELECT file, json FROM items WHERE platform = ?").all(this.platform) as { file: string; json: string }[]) {
      out.set(r.file, JSON.parse(r.json) as FileItems);
    }
    return out;
  }

  itemFiles(): string[] {
    return (this.db.query("SELECT file FROM items WHERE platform = ? ORDER BY file").all(this.platform) as { file: string }[]).map((r) => r.file);
  }

  // ---- verdicts ------------------------------------------------------------------------------------

  verdict(slot: string): { digest: string; verdict: string; at: number } | null {
    return (this.db.query("SELECT digest, verdict, at FROM verdicts WHERE slot = ?").get(slot) as any) ?? null;
  }

  /** Every slot that starts with `prefix`, with what it holds. */
  verdictsWithPrefix(prefix: string): Map<string, { digest: string; verdict: string }> {
    const rows = this.db.query("SELECT slot, digest, verdict FROM verdicts WHERE slot >= ? AND slot < ?").all(prefix, `${prefix}\u{10FFFF}`) as { slot: string; digest: string; verdict: string }[];
    return new Map(rows.map((r) => [r.slot, { digest: r.digest, verdict: r.verdict }]));
  }

  putVerdict(slot: string, digest: string, verdict: string): void {
    this.db
      .query("INSERT INTO verdicts (slot, digest, verdict, at) VALUES (?, ?, ?, ?) ON CONFLICT (slot) DO UPDATE SET digest = excluded.digest, verdict = excluded.verdict, at = excluded.at")
      .run(slot, digest, verdict, Date.now());
  }

  // ---- divergences ---------------------------------------------------------------------------------

  /** Marks `id` as an atom whose recording run ended differently from the run it was judged on, with
   * why. Its footprint may be cut short, so a selection picks it every time until the mark is cleared. */
  markDiverged(id: string, why: string): void {
    this.putVerdict(divergedSlot(this.platform, id), "", why);
  }

  /** Clears `id`'s divergence mark: its recording run ended as its judged run did. */
  clearDiverged(id: string): void {
    this.db.query("DELETE FROM verdicts WHERE slot = ?").run(divergedSlot(this.platform, id));
  }

  /** Every atom marked diverged on this platform, with why. */
  divergences(): Map<string, string> {
    const prefix = divergedSlot(this.platform, "");
    return new Map([...this.verdictsWithPrefix(prefix)].map(([slot, v]) => [slot.slice(prefix.length), v.verdict]));
  }

  // ---- counts --------------------------------------------------------------------------------------

  stats(): { atoms: Record<string, number>; recorded: Record<string, number>; red: number; owed: number; keys: number; entries: number; bytes: number } {
    const atoms: Record<string, number> = {};
    const recorded: Record<string, number> = {};
    for (const r of this.db.query("SELECT kind, COUNT(*) AS c, SUM(nkeys > 0) AS rec FROM atoms WHERE platform = ? GROUP BY kind").all(this.platform) as any[]) {
      atoms[r.kind] = r.c;
      recorded[r.kind] = r.rec ?? 0;
    }
    const red = (this.db.query("SELECT COUNT(*) AS c FROM atoms WHERE platform = ? AND verdict = 'red'").get(this.platform) as { c: number }).c;
    const owed = (this.db.query("SELECT COUNT(*) AS c FROM atoms WHERE platform = ? AND verdict = 'owed'").get(this.platform) as { c: number }).c;
    const keys = (this.db.query("SELECT COUNT(*) AS c FROM keys").get() as { c: number }).c;
    const entries = (this.db.query("SELECT COALESCE(SUM(nkeys), 0) AS c FROM atoms WHERE platform = ?").get(this.platform) as { c: number }).c;
    let bytes = 0;
    if (this.file !== ":memory:") {
      for (const f of [this.file, `${this.file}-wal`]) {
        try {
          bytes += statSync(f).size;
        } catch {
          // No WAL file is no bytes.
        }
      }
    }
    return { atoms, recorded, red, owed, keys, entries, bytes };
  }
}

/** The paths each recorded test binary asked `nvs_repo` for, by `<package> <kind> <target>`, from its
 * footprint's `tree:` keys; a binary the store has not recorded is absent. */
export function testReads(store: SelectStore): Map<string, string[]> {
  const out = new Map<string, string[]>();
  for (const a of store.atoms("test")) if (a.keys > 0) out.set(a.id.slice(5), []);
  for (const [id, keys] of store.keysUnder("tree:", "test")) out.set(id.slice(5), keys.map((k) => k.slice(5)));
  return out;
}

/** The slot an atom's divergence mark is kept in. */
const divergedSlot = (platform: string, id: string) => `diverged:${platform}:${id}`;

/** The slot a proof group's own paths are kept in: its example and attack directories and its bench
 * file, which a `proofs: <group>` unit keys on. */
export const proofReadsSlot = (group: string) => `proof-reads:${group}`;

/** Sorted key numbers as a blob: each gap from the one before as an unsigned LEB128 varint. */
export function pack(sorted: number[]): Uint8Array {
  const out: number[] = [];
  let prev = 0;
  for (const n of sorted) {
    let gap = n - prev;
    prev = n;
    do {
      let byte = gap & 0x7f;
      gap = Math.floor(gap / 128);
      if (gap > 0) byte |= 0x80;
      out.push(byte);
    } while (gap > 0);
  }
  return Uint8Array.from(out);
}

export function unpack(blob: Uint8Array): number[] {
  const out: number[] = [];
  let prev = 0;
  let gap = 0;
  let shift = 1;
  for (const byte of blob) {
    gap += (byte & 0x7f) * shift;
    if (byte & 0x80) shift *= 128;
    else {
      prev += gap;
      out.push(prev);
      gap = 0;
      shift = 1;
    }
  }
  return out;
}
