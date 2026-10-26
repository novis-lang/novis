//! The debug index: one SQLite file, `debug/index.sqlite`, that `nvs debug`'s ingester writes and
//! the viewer reads (`rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time`).
//!
//! # A cache, opened in its fastest mode
//!
//! The index holds what the spool recorded and nothing else, so losing it loses history and never
//! correctness. [`Index::open`] puts it in write-ahead mode, so the one writer and the viewer's
//! readers do not block each other, with synchronous writes off. A file whose `meta` row `schema`
//! is not [`SCHEMA`], and a file SQLite cannot read, is discarded and created again: what is still
//! in the spool is ingested into the new one.
//!
//! # What the triggers keep
//!
//! The searchable text of a row is written and deleted by triggers on its table, so no insert or
//! delete in the ingester can forget it. A search row's rowid is the source row's id times eight
//! plus its table's code (`requests` 1, `events` 2, `dumps` 3, `logs` 4, `exceptions` 5), so it
//! names the table and the row, and deleting one is a lookup by rowid rather than a scan of the
//! full-text table. A request's dump, log and query
//! counts are kept by triggers too: a request line counts the rows already in, and each row that
//! arrives after it adds one.
//!
//! # The viewer's cursor
//!
//! A request's id is `AUTOINCREMENT`, so it only rises, and in the order the batches were
//! committed: [`super::api`] pages the list on it, and an id the viewer holds never names a newer
//! request after the deletion window emptied the table.
//!
//! # The deletion window
//!
//! [`Index::forget`] deletes the requests that started before `[debug] index_age`, then the oldest
//! requests while the pages in use are past `[debug] index_max`, each with every row that names its
//! trace id, a thousand requests per transaction so a reader never waits on a long write. Rows that
//! name no recorded request are deleted by their own time past `index_age`, and an exception group
//! once its last seen is.
//!
//! # A discard keeps the numbers rising
//!
//! [`Index::rebuild`] deletes the file and creates it again, and the new file's watermark and
//! request ids go on from the old file's, so a cursor or a watermark the viewer holds never names
//! a newer row. A file too damaged to read them starts both at the watermark the loop last
//! published.
//!
//! **What it spends:** SQLite's page cache in the service process. On disk, at most `index_max`
//! of pages in use; a page freed by a deletion is reused, so the file stays near its largest size.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

/// The schema this build writes. A file with any other is a cache miss.
pub(crate) const SCHEMA: i64 = 5;

/// How many requests one deletion transaction removes.
const FORGET_BATCH: i64 = 1000;

/// The schema, written into a new file.
const CREATE: &str = r"
CREATE TABLE meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL) WITHOUT ROWID;

CREATE TABLE exception_groups (
    id INTEGER PRIMARY KEY,
    class TEXT NOT NULL,
    file TEXT NOT NULL,
    line INTEGER NOT NULL,
    count INTEGER NOT NULL,
    first_us INTEGER NOT NULL,
    last_us INTEGER NOT NULL
);
CREATE UNIQUE INDEX exception_groups_site ON exception_groups (class, file, line);
CREATE INDEX exception_groups_last ON exception_groups (last_us);

CREATE TABLE requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    writer TEXT NOT NULL,
    seq INTEGER NOT NULL,
    batch INTEGER NOT NULL,
    trace TEXT NOT NULL,
    span TEXT,
    start_us INTEGER NOT NULL,
    duration_us INTEGER NOT NULL,
    method TEXT NOT NULL,
    route TEXT,
    path TEXT NOT NULL,
    status INTEGER NOT NULL,
    queries INTEGER NOT NULL DEFAULT 0,
    dumps INTEGER NOT NULL DEFAULT 0,
    logs INTEGER NOT NULL DEFAULT 0,
    exception_group INTEGER
);
CREATE UNIQUE INDEX requests_key ON requests (writer, seq);
CREATE INDEX requests_trace ON requests (trace);
CREATE INDEX requests_start ON requests (start_us);
CREATE INDEX requests_batch ON requests (batch);

CREATE TABLE inputs (
    request INTEGER PRIMARY KEY,
    trace TEXT NOT NULL,
    query TEXT NOT NULL,
    headers TEXT NOT NULL,
    body BLOB,
    body_read INTEGER NOT NULL,
    body_declared INTEGER,
    body_cut INTEGER NOT NULL,
    body_chunks INTEGER,
    body_withheld INTEGER NOT NULL,
    response_headers TEXT,
    response_body BLOB,
    response_read INTEGER,
    response_cut INTEGER,
    response_chunks INTEGER,
    response_withheld INTEGER
);
CREATE INDEX inputs_trace ON inputs (trace);

CREATE TABLE body_parts (
    id INTEGER PRIMARY KEY,
    writer TEXT NOT NULL,
    seq INTEGER NOT NULL,
    batch INTEGER NOT NULL,
    trace TEXT NOT NULL,
    at_us INTEGER NOT NULL,
    of TEXT NOT NULL,
    offset INTEGER NOT NULL,
    bytes BLOB NOT NULL
);
CREATE UNIQUE INDEX body_parts_key ON body_parts (writer, seq);
CREATE INDEX body_parts_trace ON body_parts (trace, of, offset);

CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    writer TEXT NOT NULL,
    seq INTEGER NOT NULL,
    batch INTEGER NOT NULL,
    trace TEXT NOT NULL,
    at_us INTEGER NOT NULL,
    kind TEXT NOT NULL,
    label TEXT NOT NULL,
    start_us INTEGER NOT NULL,
    end_us INTEGER NOT NULL,
    status INTEGER,
    driver TEXT,
    sql TEXT,
    rows INTEGER,
    site INTEGER
);
CREATE UNIQUE INDEX events_key ON events (writer, seq);
CREATE INDEX events_trace ON events (trace);
CREATE INDEX events_at ON events (at_us);

CREATE TABLE dumps (
    id INTEGER PRIMARY KEY,
    writer TEXT NOT NULL,
    seq INTEGER NOT NULL,
    batch INTEGER NOT NULL,
    trace TEXT NOT NULL,
    at_us INTEGER NOT NULL,
    file TEXT NOT NULL,
    line INTEGER NOT NULL,
    tree TEXT NOT NULL,
    elided INTEGER NOT NULL
);
CREATE UNIQUE INDEX dumps_key ON dumps (writer, seq);
CREATE INDEX dumps_trace ON dumps (trace);
CREATE INDEX dumps_at ON dumps (at_us);

CREATE TABLE profiles (
    id INTEGER PRIMARY KEY,
    writer TEXT NOT NULL,
    seq INTEGER NOT NULL,
    batch INTEGER NOT NULL,
    trace TEXT NOT NULL,
    at_us INTEGER NOT NULL,
    path TEXT NOT NULL
);
CREATE UNIQUE INDEX profiles_key ON profiles (writer, seq);
CREATE INDEX profiles_trace ON profiles (trace);
CREATE INDEX profiles_at ON profiles (at_us);

CREATE TABLE coverage (
    id INTEGER PRIMARY KEY,
    writer TEXT NOT NULL,
    seq INTEGER NOT NULL,
    batch INTEGER NOT NULL,
    trace TEXT NOT NULL,
    span TEXT,
    at_us INTEGER NOT NULL,
    files TEXT NOT NULL,
    elided INTEGER NOT NULL
);
CREATE UNIQUE INDEX coverage_key ON coverage (writer, seq);
CREATE INDEX coverage_trace ON coverage (trace);
CREATE INDEX coverage_at ON coverage (at_us);

CREATE TABLE logs (
    id INTEGER PRIMARY KEY,
    writer TEXT,
    seq INTEGER,
    source TEXT,
    source_key TEXT,
    offset INTEGER,
    batch INTEGER NOT NULL,
    trace TEXT,
    at_us INTEGER NOT NULL,
    level TEXT NOT NULL,
    message TEXT NOT NULL,
    fields TEXT NOT NULL,
    elided INTEGER NOT NULL,
    lossy INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX logs_key ON logs (writer, seq);
CREATE UNIQUE INDEX logs_tailed ON logs (source_key, offset);
CREATE INDEX logs_trace ON logs (trace);
CREATE INDEX logs_at ON logs (at_us);

CREATE TABLE exceptions (
    id INTEGER PRIMARY KEY,
    request INTEGER NOT NULL,
    trace TEXT NOT NULL,
    class TEXT NOT NULL,
    message TEXT NOT NULL,
    file TEXT NOT NULL,
    line INTEGER NOT NULL,
    exception_group INTEGER NOT NULL
);
CREATE INDEX exceptions_trace ON exceptions (trace);
CREATE INDEX exceptions_group ON exceptions (exception_group);

CREATE TABLE dropped (
    id INTEGER PRIMARY KEY,
    writer TEXT NOT NULL,
    seq INTEGER NOT NULL,
    batch INTEGER NOT NULL,
    at_us INTEGER NOT NULL,
    records INTEGER NOT NULL,
    files INTEGER NOT NULL
);
CREATE UNIQUE INDEX dropped_key ON dropped (writer, seq);
CREATE INDEX dropped_at ON dropped (at_us);

CREATE TABLE unparseable (
    id INTEGER PRIMARY KEY,
    batch INTEGER NOT NULL,
    at_us INTEGER NOT NULL,
    source TEXT NOT NULL,
    source_key TEXT NOT NULL,
    offset INTEGER NOT NULL,
    length INTEGER NOT NULL,
    reason TEXT NOT NULL,
    excerpt TEXT NOT NULL,
    lossy INTEGER NOT NULL
);
CREATE UNIQUE INDEX unparseable_key ON unparseable (source_key, offset);
CREATE INDEX unparseable_at ON unparseable (at_us);

CREATE TABLE quarantine (
    path TEXT PRIMARY KEY,
    at_us INTEGER NOT NULL,
    retry_us INTEGER NOT NULL,
    failures INTEGER NOT NULL,
    reason TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE checkpoints (
    path TEXT PRIMARY KEY,
    source_key TEXT NOT NULL,
    offset INTEGER NOT NULL
) WITHOUT ROWID;

CREATE VIRTUAL TABLE search USING fts5 (text);

CREATE TRIGGER requests_search_in AFTER INSERT ON requests BEGIN
    INSERT INTO search (rowid, text)
        VALUES (NEW.id * 8 + 1, NEW.path || ' ' || coalesce(NEW.route, ''));
    UPDATE requests SET
        queries = (SELECT count(*) FROM events WHERE trace = NEW.trace AND kind = 'query'),
        dumps = (SELECT count(*) FROM dumps WHERE trace = NEW.trace),
        logs = (SELECT count(*) FROM logs WHERE trace = NEW.trace)
    WHERE id = NEW.id;
END;
CREATE TRIGGER requests_search_out AFTER DELETE ON requests BEGIN
    DELETE FROM search WHERE rowid = OLD.id * 8 + 1;
END;

CREATE TRIGGER events_search_in AFTER INSERT ON events WHEN NEW.sql IS NOT NULL BEGIN
    INSERT INTO search (rowid, text) VALUES (NEW.id * 8 + 2, NEW.sql);
END;
CREATE TRIGGER events_count AFTER INSERT ON events WHEN NEW.kind = 'query' BEGIN
    UPDATE requests SET queries = queries + 1 WHERE trace = NEW.trace;
END;
CREATE TRIGGER events_search_out AFTER DELETE ON events WHEN OLD.sql IS NOT NULL BEGIN
    DELETE FROM search WHERE rowid = OLD.id * 8 + 2;
END;

CREATE TRIGGER dumps_search_in AFTER INSERT ON dumps BEGIN
    INSERT INTO search (rowid, text) VALUES (NEW.id * 8 + 3, NEW.tree);
    UPDATE requests SET dumps = dumps + 1 WHERE trace = NEW.trace;
END;
CREATE TRIGGER dumps_search_out AFTER DELETE ON dumps BEGIN
    DELETE FROM search WHERE rowid = OLD.id * 8 + 3;
END;

CREATE TRIGGER logs_search_in AFTER INSERT ON logs BEGIN
    INSERT INTO search (rowid, text) VALUES (NEW.id * 8 + 4, NEW.message);
    UPDATE requests SET logs = logs + 1 WHERE NEW.trace IS NOT NULL AND trace = NEW.trace;
END;
CREATE TRIGGER logs_search_out AFTER DELETE ON logs BEGIN
    DELETE FROM search WHERE rowid = OLD.id * 8 + 4;
END;

CREATE TRIGGER exceptions_search_in AFTER INSERT ON exceptions BEGIN
    INSERT INTO search (rowid, text) VALUES (NEW.id * 8 + 5, NEW.class || ' ' || NEW.message);
END;
CREATE TRIGGER exceptions_search_out AFTER DELETE ON exceptions BEGIN
    DELETE FROM search WHERE rowid = OLD.id * 8 + 5;
END;
";

/// The tables whose rows name a trace id, deleted with their request.
const BY_TRACE: [&str; 8] = [
    "inputs",
    "body_parts",
    "events",
    "dumps",
    "profiles",
    "coverage",
    "logs",
    "exceptions",
];

/// The tables whose rows carry their own time, deleted past `index_age` when no request names them.
/// A `quarantine` row is not one: it stays while its file fails, and goes once the file is read or
/// is gone.
const BY_TIME: [&str; 8] = [
    "body_parts",
    "events",
    "dumps",
    "profiles",
    "coverage",
    "logs",
    "dropped",
    "unparseable",
];

/// The open index. See the module doc.
pub(crate) struct Index {
    pub(crate) db: Connection,
}

/// The two bounds of `[debug] index_age` and `[debug] index_max`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Window {
    /// The oldest record kept, in microseconds.
    pub(crate) age_us: u64,
    /// The most bytes of pages in use.
    pub(crate) max_bytes: u64,
}

/// What [`Index::forget`] deleted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Forgotten {
    /// The requests deleted, with every row that names them.
    pub(crate) requests: u64,
}

impl Index {
    /// Opens the index at `path`, creating it, and discarding a file with another schema or one
    /// SQLite cannot read.
    ///
    /// # Errors
    ///
    /// When the file cannot be created or written even after it was discarded.
    pub(crate) fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Ok(index) = Self::open_existing(path) {
            return Ok(index);
        }
        discard(path);
        let db = Connection::open(path)?;
        configure(&db)?;
        db.execute_batch(CREATE)?;
        db.execute(
            "INSERT INTO meta (key, value) VALUES ('schema', ?1), ('watermark', 0)",
            [SCHEMA],
        )?;
        Ok(Self { db })
    }

    /// The index at `path` when it is one this build reads.
    fn open_existing(path: &Path) -> rusqlite::Result<Self> {
        let db = Connection::open(path)?;
        configure(&db)?;
        let schema: Option<i64> = db
            .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
                row.get(0)
            })
            .optional()?;
        if schema == Some(SCHEMA) {
            Ok(Self { db })
        } else {
            Err(rusqlite::Error::InvalidQuery)
        }
    }

    /// Deletes the file at `path`, which this index has open, and creates it again empty. The
    /// next watermark and request id go on from this file's (the module doc's *A discard keeps the
    /// numbers rising*). Answers the new watermark.
    ///
    /// # Errors
    ///
    /// When the file cannot be deleted, because another process has it open, or cannot be created.
    /// The old file stays open in the first case, and an empty in-memory database stands in for
    /// it in the second, so every write fails and backs off until a discard succeeds.
    pub(crate) fn rebuild(&mut self, path: &Path) -> Result<i64, String> {
        let watermark = self.watermark().unwrap_or(0);
        let requests: i64 = self
            .db
            .query_row(
                "SELECT seq FROM sqlite_sequence WHERE name = 'requests'",
                [],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or(0);
        let closed = Connection::open_in_memory().map_err(|err| err.to_string())?;
        drop(std::mem::replace(&mut self.db, closed));
        discard(path);
        if path.exists() {
            self.db = Self::open(path).map_err(|err| err.to_string())?.db;
            return Err(format!(
                "`{}` cannot be deleted. Another program may have it open.",
                path.display()
            ));
        }
        let fresh = Self::open(path).map_err(|err| format!("`{}`: {err}", path.display()))?;
        let next = watermark + 1;
        fresh
            .db
            .execute("UPDATE meta SET value = ?1 WHERE key = 'watermark'", [next])
            .and_then(|_| {
                fresh.db.execute(
                    "INSERT INTO sqlite_sequence (name, seq) VALUES ('requests', ?1)",
                    [requests],
                )
            })
            .map_err(|err| err.to_string())?;
        self.db = fresh.db;
        Ok(next)
    }

    /// The number of the last committed batch, which the viewer is pushed
    /// (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`).
    pub(crate) fn watermark(&self) -> rusqlite::Result<i64> {
        self.db.query_row(
            "SELECT value FROM meta WHERE key = 'watermark'",
            [],
            |row| row.get(0),
        )
    }

    /// Deletes what is past `window` at `now_us`, in transactions of [`FORGET_BATCH`] requests.
    pub(crate) fn forget(&mut self, window: Window, now_us: u64) -> rusqlite::Result<Forgotten> {
        let cutoff = i64::try_from(now_us.saturating_sub(window.age_us)).unwrap_or(i64::MAX);
        let mut forgotten = Forgotten::default();
        loop {
            let deleted = self.forget_batch(cutoff)?;
            forgotten.requests += deleted;
            if deleted == 0 {
                break;
            }
        }
        while self.bytes_in_use()? > window.max_bytes {
            let deleted = self.forget_batch(i64::MAX)?;
            forgotten.requests += deleted;
            if deleted == 0 {
                break;
            }
        }
        let tx = self.db.transaction()?;
        for table in BY_TIME {
            let unclaimed = if has_trace(table) {
                " AND (trace IS NULL OR trace NOT IN (SELECT trace FROM requests))"
            } else {
                ""
            };
            tx.execute(
                &format!("DELETE FROM {table} WHERE at_us < ?1{unclaimed}"),
                [cutoff],
            )?;
        }
        tx.execute("DELETE FROM exception_groups WHERE last_us < ?1", [cutoff])?;
        tx.commit()?;
        Ok(forgotten)
    }

    /// Deletes the [`FORGET_BATCH`] oldest requests that started before `cutoff`, with every row
    /// that names their trace ids, in one transaction. Answers how many requests it deleted.
    fn forget_batch(&mut self, cutoff: i64) -> rusqlite::Result<u64> {
        let tx = self.db.transaction()?;
        tx.execute_batch("CREATE TEMP TABLE IF NOT EXISTS doomed (id INTEGER, trace TEXT)")?;
        tx.execute("DELETE FROM temp.doomed", [])?;
        let picked = tx.execute(
            "INSERT INTO temp.doomed (id, trace) \
             SELECT id, trace FROM requests WHERE start_us < ?1 ORDER BY start_us LIMIT ?2",
            params![cutoff, FORGET_BATCH],
        )?;
        if picked == 0 {
            return Ok(0);
        }
        for table in BY_TRACE {
            tx.execute(
                &format!("DELETE FROM {table} WHERE trace IN (SELECT trace FROM temp.doomed)"),
                [],
            )?;
        }
        tx.execute(
            "DELETE FROM requests WHERE id IN (SELECT id FROM temp.doomed)",
            [],
        )?;
        tx.execute("DELETE FROM temp.doomed", [])?;
        tx.commit()?;
        Ok(u64::try_from(picked).unwrap_or(u64::MAX))
    }

    /// The bytes of the pages in use: every page but the free ones.
    fn bytes_in_use(&self) -> rusqlite::Result<u64> {
        let pragma = |name: &str| -> rusqlite::Result<u64> {
            self.db
                .query_row(&format!("PRAGMA {name}"), [], |row| row.get::<_, i64>(0))
                .map(|value| u64::try_from(value).unwrap_or(0))
        };
        let pages = pragma("page_count")?.saturating_sub(pragma("freelist_count")?);
        Ok(pages.saturating_mul(pragma("page_size")?))
    }
}

/// Whether `table` has a `trace` column.
fn has_trace(table: &str) -> bool {
    !matches!(table, "dropped" | "unparseable")
}

/// Write-ahead mode with synchronous writes off: the module doc's *A cache, opened in its fastest
/// mode*.
fn configure(db: &Connection) -> rusqlite::Result<()> {
    db.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get::<_, String>(0))?;
    db.pragma_update(None, "synchronous", "OFF")?;
    Ok(())
}

/// Removes the index at `path` and its write-ahead files. A file that is not there is not an error.
fn discard(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        let _ = std::fs::remove_file(PathBuf::from(name));
    }
}
