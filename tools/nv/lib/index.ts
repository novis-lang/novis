// The index: every record and every prose file's headings and citations, in one SQLite database at
// `.cache/nv.sqlite`. It is derived and never committed; deleting it costs one full rebuild.
//
// Each record type is a table named for it: `id`, `path`, the record's whole `json`, and one column
// per scalar field. A field that is a list becomes a child table, `<type>__<field>`, with the owner's
// id, the item's position, and the item's scalar fields. A reference (`s.ref`) is declared as a
// foreign key wherever it lands. The keys are declared but not enforced while files are read, because
// files arrive in no particular order; `check` asks SQLite for every row that breaks one, and that is
// the one place a dangling reference is found.
//
// A rebuild is incremental. A file whose size and modification time match its last read is skipped
// without being read, and one whose text hashes the same is skipped after it is read. A change to any
// declaration the tables are built from drops the whole database and builds it again.

import { Database, type Statement } from "bun:sqlite";
import { mkdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { CACHE, ROOT } from "./paths.ts";
import { citations, headings } from "./prose.ts";
import {
  ArraySchema,
  BooleanSchema,
  EnumSchema,
  NullableSchema,
  NumberSchema,
  ObjectSchema,
  OptionalSchema,
  RefSchema,
  StringSchema,
  type RecordType,
  type Schema,
} from "./schema.ts";
import { idAt, loadFile, recordFiles } from "./store.ts";

/** Bumped when the layout below changes in a way the declarations do not show. */
const LAYOUT_VERSION = 1;

/** The prose the index scans by default: every Markdown file under `docs/` and at the root. */
export const DEFAULT_PROSE = ["docs/**/*.md", "*.md"];

export interface IndexOptions {
  root?: string;
  types: RecordType<any>[];
  /** Repo-relative glob patterns of the prose files to scan. */
  prose?: string[];
  /** The database file. `:memory:` builds one that lives as long as the object. */
  file?: string;
}

export interface Finding {
  /** Repo-relative path of the record or prose file the finding is about. */
  path: string;
  message: string;
}

export interface RefreshStats {
  read: number;
  unchanged: number;
  removed: number;
}

interface Column {
  name: string;
  sql: "TEXT" | "INTEGER" | "REAL";
  ref?: string;
  boolean?: boolean;
}

interface Child {
  table: string;
  field: string | null;
  /** `value` for a list of scalars; the item's own fields for a list of objects. */
  columns: Column[];
  objects: boolean;
}

interface Layout {
  type: RecordType<any>;
  table: string;
  columns: Column[];
  children: Child[];
}

const MAIN_SYSTEM = new Set(["id", "path", "json"]);
const CHILD_SYSTEM = new Set(["owner", "ord", "path", "json"]);

function unwrap(schema: Schema<any>): Schema<any> {
  let s = schema;
  while (s instanceof OptionalSchema || s instanceof NullableSchema) s = s.inner;
  return s;
}

function column(name: string, schema: Schema<any>): Column | null {
  const s = unwrap(schema);
  if (s instanceof RefSchema) return { name, sql: "TEXT", ref: s.target };
  if (s instanceof StringSchema || s instanceof EnumSchema) return { name, sql: "TEXT" };
  if (s instanceof NumberSchema) return { name, sql: s.integer ? "INTEGER" : "REAL" };
  if (s instanceof BooleanSchema) return { name, sql: "INTEGER", boolean: true };
  return null;
}

function child(table: string, field: string | null, schema: Schema<any>): Child | null {
  const s = unwrap(schema);
  if (!(s instanceof ArraySchema)) return null;
  const name = `${table}__${field ?? "items"}`;
  const item = unwrap(s.item);
  if (item instanceof ObjectSchema) {
    const columns = Object.entries(item.shape as Record<string, Schema<any>>)
      .filter(([k]) => !CHILD_SYSTEM.has(k))
      .map(([k, f]) => column(k, f))
      .filter((c): c is Column => c !== null);
    return { table: name, field, columns, objects: true };
  }
  const value = column("value", item);
  return value ? { table: name, field, columns: [value], objects: false } : null;
}

function layoutOf(type: RecordType<any>): Layout {
  const root = unwrap(type.schema);
  const columns: Column[] = [];
  const children: Child[] = [];
  if (root instanceof ObjectSchema) {
    for (const [k, f] of Object.entries(root.shape as Record<string, Schema<any>>)) {
      const c = MAIN_SYSTEM.has(k) ? null : column(k, f);
      if (c) columns.push(c);
      const ch = child(type.name, k, f);
      if (ch) children.push(ch);
    }
  } else {
    const ch = child(type.name, null, root);
    if (ch) children.push(ch);
  }
  return { type, table: type.name, columns, children };
}

const q = (name: string) => `"${name.replace(/"/g, '""')}"`;

function columnSql(c: Column): string {
  return `${q(c.name)} ${c.sql}${c.ref ? ` REFERENCES ${q(c.ref)}(id)` : ""}`;
}

function bind(c: Column, v: unknown): string | number | null {
  if (v === undefined || v === null) return null;
  if (c.boolean) return v ? 1 : 0;
  return typeof v === "string" || typeof v === "number" ? v : JSON.stringify(v);
}

export class Index {
  readonly db: Database;
  readonly root: string;
  readonly file: string;
  private readonly types: RecordType<any>[];
  private readonly prose: string[];
  private readonly layouts: Layout[];
  private readonly statements = new Map<string, Statement>();

  constructor(opts: IndexOptions) {
    this.root = opts.root ?? ROOT;
    this.types = opts.types;
    this.prose = opts.prose ?? DEFAULT_PROSE;
    this.file = opts.file ?? join(opts.root ? join(opts.root, ".cache") : CACHE, "nv.sqlite");
    this.layouts = this.types.map(layoutOf);
    const names = new Set(this.types.map((t) => t.name));
    for (const l of this.layouts) {
      for (const c of [...l.columns, ...l.children.flatMap((ch) => ch.columns)]) {
        if (c.ref && !names.has(c.ref)) throw new Error(`${l.table}.${c.name} references ${c.ref}, which is no record type`);
      }
    }
    const fingerprint = String(
      Bun.hash(
        JSON.stringify({
          v: LAYOUT_VERSION,
          prose: this.prose,
          types: this.types.map((t) => ({
            name: t.name,
            dir: t.dir,
            single: t.single ?? false,
            schema: t.schema.jsonSchema(),
            views: t.views ?? {},
            checks: t.checks ?? [],
            idOf: t.idOf?.toString() ?? null,
          })),
        }),
      ),
    );
    if (this.file !== ":memory:") mkdirSync(dirname(this.file), { recursive: true });
    this.db = this.openFresh(fingerprint);
  }

  private openFresh(fingerprint: string): Database {
    if (this.file !== ":memory:") {
      try {
        const old = new Database(this.file);
        let same = false;
        try {
          const read = old.prepare("SELECT value FROM meta WHERE key = 'fingerprint'");
          const row = read.get() as { value: string } | null;
          read.finalize();
          same = row?.value === fingerprint;
        } catch {
          same = false;
        }
        if (same) return old;
        old.close();
        rmSync(this.file, { force: true });
      } catch {
        rmSync(this.file, { force: true });
      }
    }
    const db = new Database(this.file);
    db.exec("PRAGMA journal_mode = WAL");
    db.exec(`
      CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
      CREATE TABLE files (path TEXT PRIMARY KEY, kind TEXT NOT NULL, size INTEGER NOT NULL, mtime REAL NOT NULL, hash TEXT NOT NULL);
      CREATE TABLE issues (path TEXT NOT NULL, at TEXT NOT NULL, message TEXT NOT NULL);
      CREATE TABLE headings (path TEXT NOT NULL, line INTEGER NOT NULL, level INTEGER NOT NULL, text TEXT NOT NULL);
      CREATE TABLE citations (path TEXT NOT NULL, line INTEGER NOT NULL, kind TEXT NOT NULL, target TEXT NOT NULL);
      CREATE INDEX citations_target ON citations (kind, target);
    `);
    for (const l of this.layouts) {
      const cols = ["id TEXT PRIMARY KEY", "path TEXT NOT NULL", "json TEXT NOT NULL", ...l.columns.map(columnSql)];
      db.exec(`CREATE TABLE ${q(l.table)} (${cols.join(", ")})`);
      for (const ch of l.children) {
        const ccols = [
          `owner TEXT NOT NULL REFERENCES ${q(l.table)}(id)`,
          "ord INTEGER NOT NULL",
          "path TEXT NOT NULL",
          ...(ch.objects ? ["json TEXT NOT NULL"] : []),
          ...ch.columns.map(columnSql),
        ];
        db.exec(`CREATE TABLE ${q(ch.table)} (${ccols.join(", ")}, PRIMARY KEY (owner, ord))`);
      }
    }
    if (this.layouts.length > 0) {
      const union = this.layouts.map((l) => `SELECT '${l.table}' AS type, id, path FROM ${q(l.table)}`).join(" UNION ALL ");
      db.exec(`CREATE VIEW records AS ${union}`);
    }
    for (const t of this.types) {
      for (const [name, sql] of Object.entries(t.views ?? {})) db.exec(`CREATE VIEW ${q(name)} AS ${sql}`);
    }
    const mark = db.prepare("INSERT INTO meta (key, value) VALUES ('fingerprint', ?)");
    mark.run(fingerprint);
    mark.finalize();
    return db;
  }

  /** Brings the index up to date with the files on disk. */
  refresh(): RefreshStats {
    const want = new Map<string, RecordType<any> | "prose" | "orphan">();
    for (const path of recordFiles(this.root)) {
      want.set(path, this.types.find((t) => idAt(t, path) !== null) ?? "orphan");
    }
    for (const pattern of this.prose) {
      for (const p of new Bun.Glob(pattern).scanSync({ cwd: this.root, onlyFiles: true })) {
        const path = p.replace(/\\/g, "/");
        if (!path.includes("node_modules/") && !want.has(path)) want.set(path, "prose");
      }
    }
    const known = new Map<string, { size: number; mtime: number; hash: string }>();
    for (const row of this.stmt("SELECT path, size, mtime, hash FROM files").all() as {
      path: string;
      size: number;
      mtime: number;
      hash: string;
    }[]) {
      known.set(row.path, row);
    }
    const stats: RefreshStats = { read: 0, unchanged: 0, removed: 0 };
    const upsert = this.stmt("INSERT OR REPLACE INTO files (path, kind, size, mtime, hash) VALUES (?, ?, ?, ?, ?)");
    // `BEGIN` and `COMMIT` by hand: `db.transaction()` keeps prepared statements that outlive
    // `close()`, and on Windows those hold the file open, so a changed declaration could not
    // delete it.
    this.db.exec("BEGIN");
    try {
      for (const path of known.keys()) {
        if (!want.has(path)) {
          this.forget(path);
          this.stmt("DELETE FROM files WHERE path = ?").run(path);
          stats.removed++;
        }
      }
      for (const [path, kind] of want) {
        const st = statSync(join(this.root, path));
        const before = known.get(path);
        if (before && before.size === st.size && before.mtime === st.mtimeMs) {
          stats.unchanged++;
          continue;
        }
        const text = readFileSync(join(this.root, path), "utf8");
        const hash = String(Bun.hash(text));
        const label = typeof kind === "string" ? kind : `record:${kind.name}`;
        if (before && before.hash === hash) {
          upsert.run(path, label, st.size, st.mtimeMs, hash);
          stats.unchanged++;
          continue;
        }
        this.forget(path);
        if (kind === "prose") this.readProse(path, text);
        else if (kind === "orphan") this.issue(path, "", "is under data/ but no record type claims it");
        else this.readRecord(kind, path);
        upsert.run(path, label, st.size, st.mtimeMs, hash);
        stats.read++;
      }
      this.db.exec("COMMIT");
    } catch (e) {
      this.db.exec("ROLLBACK");
      throw e;
    }
    return stats;
  }

  private issue(path: string, at: string, message: string): void {
    this.stmt("INSERT INTO issues (path, at, message) VALUES (?, ?, ?)").run(path, at, message);
  }

  private forget(path: string): void {
    for (const l of this.layouts) {
      for (const ch of l.children) this.stmt(`DELETE FROM ${q(ch.table)} WHERE path = ?`).run(path);
      this.stmt(`DELETE FROM ${q(l.table)} WHERE path = ?`).run(path);
    }
    for (const t of ["issues", "headings", "citations"]) this.stmt(`DELETE FROM ${t} WHERE path = ?`).run(path);
  }

  private readProse(path: string, text: string): void {
    const h = this.stmt("INSERT INTO headings (path, line, level, text) VALUES (?, ?, ?, ?)");
    for (const x of headings(text)) h.run(path, x.line, x.level, x.text);
    const c = this.stmt("INSERT INTO citations (path, line, kind, target) VALUES (?, ?, ?, ?)");
    for (const x of citations(text)) c.run(path, x.line, x.kind, x.target);
  }

  private readRecord(type: RecordType<any>, path: string): void {
    const loaded = loadFile(type, path, this.root);
    for (const i of loaded.issues) this.issue(path, i.at, i.message);
    if (loaded.value === undefined) return;
    const valid = type.schema.is(loaded.value);
    const l = this.layouts.find((x) => x.type === type)!;
    const value = loaded.value as Record<string, unknown>;
    const cols = ["id", "path", "json", ...l.columns.map((c) => c.name)];
    const vals = [loaded.id, path, JSON.stringify(loaded.value), ...l.columns.map((c) => (valid ? bind(c, value[c.name]) : null))];
    try {
      this.stmt(`INSERT INTO ${q(l.table)} (${cols.map(q).join(", ")}) VALUES (${cols.map(() => "?").join(", ")})`).run(...vals);
    } catch (e) {
      this.issue(path, "", `could not be indexed: ${(e as Error).message}`);
      return;
    }
    if (!valid) return;
    for (const ch of l.children) {
      const items = (ch.field === null ? loaded.value : value[ch.field]) as unknown[] | undefined | null;
      if (!Array.isArray(items)) continue;
      const ccols = ["owner", "ord", "path", ...(ch.objects ? ["json"] : []), ...ch.columns.map((c) => c.name)];
      const insert = this.stmt(
        `INSERT INTO ${q(ch.table)} (${ccols.map(q).join(", ")}) VALUES (${ccols.map(() => "?").join(", ")})`,
      );
      items.forEach((item, ord) => {
        const fields = ch.objects
          ? ch.columns.map((c) => bind(c, (item as Record<string, unknown>)[c.name]))
          : [bind(ch.columns[0]!, item)];
        insert.run(loaded.id, ord, path, ...(ch.objects ? [JSON.stringify(item)] : []), ...fields);
      });
    }
  }

  /** Every schema, foreign-key and invariant finding, sorted by path. Call `refresh` first. */
  check(): Finding[] {
    const out: Finding[] = [];
    for (const row of this.stmt("SELECT path, at, message FROM issues").all() as { path: string; at: string; message: string }[]) {
      out.push({ path: row.path, message: row.at ? `${row.at} ${row.message}` : row.message });
    }
    const broken = this.stmt("PRAGMA foreign_key_check").all() as { table: string; rowid: number; parent: string; fkid: number }[];
    for (const b of broken) {
      const fks = this.stmt(`PRAGMA foreign_key_list(${q(b.table)})`).all() as { id: number; from: string }[];
      const from = fks.find((f) => f.id === b.fkid)?.from ?? "?";
      const row = this.stmt(`SELECT path, ${q(from)} AS value FROM ${q(b.table)} WHERE rowid = ?`).get(b.rowid) as {
        path: string;
        value: unknown;
      } | null;
      out.push({
        path: row?.path ?? b.table,
        message: `${b.table}.${from} names ${JSON.stringify(row?.value)}, which is no ${b.parent} record`,
      });
    }
    for (const t of this.types) {
      for (const c of t.checks ?? []) {
        for (const row of this.stmt(c.sql).all() as { path: string; detail: string }[]) {
          out.push({ path: row.path, message: `${c.name}: ${row.detail}` });
        }
      }
    }
    return out.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : a.message < b.message ? -1 : 1));
  }

  /** Runs `sql` with the database in query-only mode, so nothing it does can write. */
  query(sql: string, params: (string | number | null)[] = []): Record<string, unknown>[] {
    this.db.exec("PRAGMA query_only = ON");
    try {
      return this.stmt(sql).all(...params) as Record<string, unknown>[];
    } finally {
      this.db.exec("PRAGMA query_only = OFF");
    }
  }

  /**
   * The prepared statement for `sql`, prepared once per index. The cache is this class's rather than
   * `db.query`'s because `close` has to finalize every statement: on Windows one left unfinalized
   * keeps the database file open after `close`, and the file can then be neither deleted nor rebuilt.
   */
  private stmt(sql: string): Statement {
    let st = this.statements.get(sql);
    if (!st) {
      st = this.db.prepare(sql);
      this.statements.set(sql, st);
    }
    return st;
  }

  close(): void {
    for (const st of this.statements.values()) st.finalize();
    this.statements.clear();
    this.db.close();
  }
}
