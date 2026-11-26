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
//! The same two triggers keep what the request list draws for each row, so no page aggregates
//! events: the summed durations of the request's events by kind (`db_us` for `query`, then
//! `http_us`, `cache_us`, `session_us`, `file_us`, `gc_us` and `spawn_us`), and `repeats`, the
//! most times one statement text ran at one call site in it, the repeat
//! `rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site` defines. A writer writes a
//! request's line after its events, so the request trigger counts them once. A query that arrives
//! after its request line counts its own statement at its site, on the `events_trace` index, and a
//! query whose request is not in yet costs nothing. Spends: eight integers per request row.
//!
//! A request row also keeps what its line recorded of the program: the memory peak and limit,
//! the handler's name and the body bytes read and written, each null where the line has none.
//! Spends: four integers and one name per request row.
//!
//! A `runs` row is a scheduled job, a queue job or a command run
//! (`rule:testing/a-run-is-recorded-like-a-request`). The same three triggers keep its counts and
//! summed durations as they keep a request's, each with one more indexed update by trace id, and
//! the deletion window forgets it as it forgets a request. An `exceptions` row names its request
//! or its run. Spends: per run row, what a request row spends, less its path, route and sizes.
//!
//! An `events` row keeps the event's outcome, such as a cache read's hit and tier or an outbound
//! call's status and sizes, as one JSON text, null where the line has none. The viewer draws it
//! and never filters or sums on it, so it is one column and not one per field. Spends: per
//! recorded event, at most a few short fields.
//!
//! An `exceptions` row keeps the uncaught exception's frames and its `previous` chain as two
//! JSON texts, each null where the line has none, and whether either was cut. The viewer draws
//! them and never filters on them, so they are not searched. Spends: per recorded exception, at
//! most 64 frames and 8 links of names, paths and lines.
//!
//! # The viewer's cursor
//!
//! A request's id is `AUTOINCREMENT`, so it only rises, and in the order the batches were
//! committed: [`super::api`] pages the list on it, and an id the viewer holds never names a newer
//! request after the deletion window emptied the table. It costs one `sqlite_sequence` update per
//! ingested request, off the request path. The viewer reads through one connection behind a mutex,
//! which each page's bounded query holds briefly.
//!
//! # The deletion window
//!
//! [`Index::forget`] first deletes what is past `[debug] index_age`: each request and run that
//! started before it, with every row that names its trace id, then each row no request or run
//! names by its own time, and an exception group once its last seen is. A row no owner names is
//! one of a command run still going, whose run line is written when it ends, or one whose owner
//! never arrived.
//!
//! Then, while the pages in use are past `[debug] index_max`, it deletes the oldest records in one
//! time order, whether a request or run owns them or not. Each step compares the oldest request,
//! the oldest run and each table's oldest row no owner names, and deletes one batch of the oldest
//! of them up to the next oldest: a thousand requests or runs with their rows, or a thousand
//! rows of one table, so a reader never waits on a long write. The order holds to within one
//! batch of requests or runs, because an owner's rows are not before its start. Looking up a
//! table's oldest row no owner names reads only the rows older than the oldest owner, so a step
//! never reads the rows the owners keep.
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
//! The `events_site` index, which the editor's recorded lens looks a call site up on
//! (`nvs_lsp::recorded`), is one entry per event inside that cap. The `copies_site` index, which
//! looks up the copies recorded requests made at one copy site, is one entry per site a recorded
//! request copied at, inside the same cap. The `requests_route` index, which the Routes view and
//! the list's method and route filters read, is one entry per request. The `body_parts_at` index,
//! which the deletion window finds a body part no request names on, is one entry per recorded
//! body part, inside the same cap.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

/// The schema this build writes. A file with any other is a cache miss.
pub(crate) const SCHEMA: i64 = 16;

/// How many requests one deletion transaction removes.
const FORGET_BATCH: i64 = 1000;

/// How many rows that no request or run names one deletion transaction removes from one table.
const FORGET_ROWS: i64 = 1000;

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
    exception_group INTEGER,
    db_us INTEGER NOT NULL DEFAULT 0,
    http_us INTEGER NOT NULL DEFAULT 0,
    cache_us INTEGER NOT NULL DEFAULT 0,
    session_us INTEGER NOT NULL DEFAULT 0,
    file_us INTEGER NOT NULL DEFAULT 0,
    gc_us INTEGER NOT NULL DEFAULT 0,
    spawn_us INTEGER NOT NULL DEFAULT 0,
    repeats INTEGER NOT NULL DEFAULT 0,
    memory_peak INTEGER,
    memory_limit INTEGER,
    handler TEXT,
    bytes_in INTEGER,
    bytes_out INTEGER
);
CREATE UNIQUE INDEX requests_key ON requests (writer, seq);
CREATE INDEX requests_trace ON requests (trace);
CREATE INDEX requests_start ON requests (start_us);
CREATE INDEX requests_batch ON requests (batch);
CREATE INDEX requests_route ON requests (method, route, start_us);

CREATE TABLE runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    writer TEXT NOT NULL,
    seq INTEGER NOT NULL,
    batch INTEGER NOT NULL,
    trace TEXT NOT NULL,
    of TEXT NOT NULL,
    name TEXT NOT NULL,
    start_us INTEGER NOT NULL,
    duration_us INTEGER NOT NULL,
    outcome TEXT NOT NULL,
    queries INTEGER NOT NULL DEFAULT 0,
    dumps INTEGER NOT NULL DEFAULT 0,
    logs INTEGER NOT NULL DEFAULT 0,
    exception_group INTEGER,
    db_us INTEGER NOT NULL DEFAULT 0,
    http_us INTEGER NOT NULL DEFAULT 0,
    cache_us INTEGER NOT NULL DEFAULT 0,
    session_us INTEGER NOT NULL DEFAULT 0,
    file_us INTEGER NOT NULL DEFAULT 0,
    gc_us INTEGER NOT NULL DEFAULT 0,
    spawn_us INTEGER NOT NULL DEFAULT 0,
    repeats INTEGER NOT NULL DEFAULT 0,
    memory_peak INTEGER,
    memory_limit INTEGER
);
CREATE UNIQUE INDEX runs_key ON runs (writer, seq);
CREATE INDEX runs_trace ON runs (trace);
CREATE INDEX runs_start ON runs (start_us);
CREATE INDEX runs_name ON runs (of, name, start_us);

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
CREATE INDEX body_parts_at ON body_parts (at_us);

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
    target TEXT,
    site_file TEXT,
    site_line INTEGER,
    site_hash TEXT,
    outcome TEXT
);
CREATE UNIQUE INDEX events_key ON events (writer, seq);
CREATE INDEX events_trace ON events (trace);
CREATE INDEX events_at ON events (at_us);
CREATE INDEX events_site ON events (site_hash, site_line);

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
CREATE TABLE mails (
    id INTEGER PRIMARY KEY,
    writer TEXT NOT NULL,
    seq INTEGER NOT NULL,
    batch INTEGER NOT NULL,
    trace TEXT NOT NULL,
    at_us INTEGER NOT NULL,
    endpoint TEXT NOT NULL,
    subject TEXT NOT NULL,
    recipients TEXT NOT NULL,
    message TEXT NOT NULL
);
CREATE UNIQUE INDEX mails_key ON mails (writer, seq);
CREATE INDEX mails_trace ON mails (trace);
CREATE INDEX mails_at ON mails (at_us);

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

CREATE TABLE copies (
    id INTEGER PRIMARY KEY,
    writer TEXT NOT NULL,
    seq INTEGER NOT NULL,
    part INTEGER NOT NULL,
    batch INTEGER NOT NULL,
    trace TEXT NOT NULL,
    span TEXT,
    at_us INTEGER NOT NULL,
    file TEXT NOT NULL,
    line INTEGER NOT NULL,
    hash TEXT NOT NULL,
    reason TEXT NOT NULL,
    copies INTEGER NOT NULL,
    units INTEGER NOT NULL
);
CREATE UNIQUE INDEX copies_key ON copies (writer, seq, part);
CREATE INDEX copies_trace ON copies (trace);
CREATE INDEX copies_at ON copies (at_us);
CREATE INDEX copies_site ON copies (hash, line);

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
    request INTEGER,
    run INTEGER,
    trace TEXT NOT NULL,
    class TEXT NOT NULL,
    message TEXT NOT NULL,
    file TEXT NOT NULL,
    line INTEGER NOT NULL,
    exception_group INTEGER NOT NULL,
    frames TEXT,
    frames_cut INTEGER NOT NULL DEFAULT 0,
    previous TEXT,
    previous_cut INTEGER NOT NULL DEFAULT 0
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
        logs = (SELECT count(*) FROM logs WHERE trace = NEW.trace),
        (db_us, http_us, cache_us, session_us, file_us, gc_us, spawn_us) = (
            SELECT
                coalesce(sum(CASE kind WHEN 'query' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'http' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'cache' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'session' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'file' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'gc' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'spawn' THEN max(end_us - start_us, 0) END), 0)
            FROM events WHERE trace = NEW.trace
        ),
        repeats = (
            SELECT coalesce(max(n), 0) FROM (
                SELECT count(*) AS n FROM events WHERE trace = NEW.trace AND kind = 'query'
                GROUP BY site_file, site_line, sql
            )
        )
    WHERE id = NEW.id;
END;
CREATE TRIGGER runs_count AFTER INSERT ON runs BEGIN
    UPDATE runs SET
        queries = (SELECT count(*) FROM events WHERE trace = NEW.trace AND kind = 'query'),
        dumps = (SELECT count(*) FROM dumps WHERE trace = NEW.trace),
        logs = (SELECT count(*) FROM logs WHERE trace = NEW.trace),
        (db_us, http_us, cache_us, session_us, file_us, gc_us, spawn_us) = (
            SELECT
                coalesce(sum(CASE kind WHEN 'query' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'http' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'cache' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'session' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'file' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'gc' THEN max(end_us - start_us, 0) END), 0),
                coalesce(sum(CASE kind WHEN 'spawn' THEN max(end_us - start_us, 0) END), 0)
            FROM events WHERE trace = NEW.trace
        ),
        repeats = (
            SELECT coalesce(max(n), 0) FROM (
                SELECT count(*) AS n FROM events WHERE trace = NEW.trace AND kind = 'query'
                GROUP BY site_file, site_line, sql
            )
        )
    WHERE id = NEW.id;
END;
CREATE TRIGGER requests_search_out AFTER DELETE ON requests BEGIN
    DELETE FROM search WHERE rowid = OLD.id * 8 + 1;
END;

CREATE TRIGGER events_search_in AFTER INSERT ON events WHEN NEW.sql IS NOT NULL BEGIN
    INSERT INTO search (rowid, text) VALUES (NEW.id * 8 + 2, NEW.sql);
END;
CREATE TRIGGER events_count AFTER INSERT ON events WHEN NEW.kind <> 'call' BEGIN
    UPDATE requests SET
        queries = queries + (NEW.kind = 'query'),
        db_us = db_us + CASE NEW.kind WHEN 'query' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        http_us = http_us + CASE NEW.kind WHEN 'http' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        cache_us = cache_us + CASE NEW.kind WHEN 'cache' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        session_us = session_us + CASE NEW.kind WHEN 'session' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        file_us = file_us + CASE NEW.kind WHEN 'file' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        gc_us = gc_us + CASE NEW.kind WHEN 'gc' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        spawn_us = spawn_us + CASE NEW.kind WHEN 'spawn' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        repeats = CASE WHEN NEW.kind = 'query' THEN max(repeats, (
            SELECT count(*) FROM events
            WHERE trace = NEW.trace AND kind = 'query' AND site_file IS NEW.site_file
                AND site_line IS NEW.site_line AND sql IS NEW.sql
        )) ELSE repeats END
    WHERE trace = NEW.trace;
    UPDATE runs SET
        queries = queries + (NEW.kind = 'query'),
        db_us = db_us + CASE NEW.kind WHEN 'query' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        http_us = http_us + CASE NEW.kind WHEN 'http' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        cache_us = cache_us + CASE NEW.kind WHEN 'cache' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        session_us = session_us + CASE NEW.kind WHEN 'session' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        file_us = file_us + CASE NEW.kind WHEN 'file' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        gc_us = gc_us + CASE NEW.kind WHEN 'gc' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        spawn_us = spawn_us + CASE NEW.kind WHEN 'spawn' THEN max(NEW.end_us - NEW.start_us, 0) ELSE 0 END,
        repeats = CASE WHEN NEW.kind = 'query' THEN max(repeats, (
            SELECT count(*) FROM events
            WHERE trace = NEW.trace AND kind = 'query' AND site_file IS NEW.site_file
                AND site_line IS NEW.site_line AND sql IS NEW.sql
        )) ELSE repeats END
    WHERE trace = NEW.trace;
END;
CREATE TRIGGER events_search_out AFTER DELETE ON events WHEN OLD.sql IS NOT NULL BEGIN
    DELETE FROM search WHERE rowid = OLD.id * 8 + 2;
END;

CREATE TRIGGER dumps_search_in AFTER INSERT ON dumps BEGIN
    INSERT INTO search (rowid, text) VALUES (NEW.id * 8 + 3, NEW.tree);
    UPDATE requests SET dumps = dumps + 1 WHERE trace = NEW.trace;
    UPDATE runs SET dumps = dumps + 1 WHERE trace = NEW.trace;
END;
CREATE TRIGGER dumps_search_out AFTER DELETE ON dumps BEGIN
    DELETE FROM search WHERE rowid = OLD.id * 8 + 3;
END;

CREATE TRIGGER logs_search_in AFTER INSERT ON logs BEGIN
    INSERT INTO search (rowid, text) VALUES (NEW.id * 8 + 4, NEW.message);
    UPDATE requests SET logs = logs + 1 WHERE NEW.trace IS NOT NULL AND trace = NEW.trace;
    UPDATE runs SET logs = logs + 1 WHERE NEW.trace IS NOT NULL AND trace = NEW.trace;
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

/// The tables whose rows each own a trace id and start at a time: a request or a run. Each is
/// forgotten by its start, with every row of [`BY_TRACE`] that names its trace id.
const OWNERS: [&str; 2] = ["requests", "runs"];

/// The tables whose rows name a trace id, deleted with their request or run.
const BY_TRACE: [&str; 10] = [
    "inputs",
    "body_parts",
    "events",
    "dumps",
    "mails",
    "profiles",
    "coverage",
    "copies",
    "logs",
    "exceptions",
];

/// The tables whose rows carry their own time, deleted by it when no request or run names them.
/// A `quarantine` row is not one: it stays while its file fails, and goes once the file is read or
/// is gone.
const BY_TIME: [&str; 10] = [
    "body_parts",
    "events",
    "dumps",
    "mails",
    "profiles",
    "coverage",
    "copies",
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
        let sequence = |name: &str| -> i64 {
            self.db
                .query_row(
                    "SELECT seq FROM sqlite_sequence WHERE name = ?1",
                    [name],
                    |row| row.get(0),
                )
                .optional()
                .ok()
                .flatten()
                .unwrap_or(0)
        };
        let (requests, runs) = (sequence("requests"), sequence("runs"));
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
                    "INSERT INTO sqlite_sequence (name, seq) VALUES ('requests', ?1), ('runs', ?2)",
                    [requests, runs],
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

    /// Deletes what is past `window` at `now_us`: the module doc's *The deletion window*.
    pub(crate) fn forget(&mut self, window: Window, now_us: u64) -> rusqlite::Result<Forgotten> {
        let cutoff = i64::try_from(now_us.saturating_sub(window.age_us)).unwrap_or(i64::MAX);
        let mut forgotten = Forgotten::default();
        for owner in OWNERS {
            loop {
                let deleted = self.forget_batch(owner, cutoff)?;
                if owner == "requests" {
                    forgotten.requests += deleted;
                }
                if deleted == 0 {
                    break;
                }
            }
        }
        for table in BY_TIME {
            while self.forget_rows(table, cutoff)? > 0 {}
        }
        self.db
            .execute("DELETE FROM exception_groups WHERE last_us < ?1", [cutoff])?;
        while self.bytes_in_use()? > window.max_bytes {
            let deleted = match self.oldest()? {
                Some((Oldest::Owner(owner), until)) => {
                    let deleted = self.forget_batch(owner, until)?;
                    if owner == "requests" {
                        forgotten.requests += deleted;
                    }
                    deleted
                }
                Some((Oldest::Unowned(table), until)) => self.forget_rows(table, until)?,
                None => 0,
            };
            if deleted == 0 {
                break;
            }
        }
        Ok(forgotten)
    }

    /// The kind of record whose oldest row is the oldest of all, and the bound one batch of it is
    /// deleted below: the next oldest kind's time plus one, so a tie still deletes. `None` when
    /// the index has no record left to delete.
    ///
    /// A table's oldest row that no owner names is looked up only below the oldest owner's start,
    /// so the lookup reads the rows older than every owner and never the rows the owners keep.
    fn oldest(&self) -> rusqlite::Result<Option<(Oldest, i64)>> {
        let mut heads: Vec<(i64, Oldest)> = Vec::with_capacity(OWNERS.len() + BY_TIME.len());
        for owner in OWNERS {
            let start: Option<i64> =
                self.db
                    .query_row(&format!("SELECT min(start_us) FROM {owner}"), [], |row| {
                        row.get(0)
                    })?;
            if let Some(start) = start {
                heads.push((start, Oldest::Owner(owner)));
            }
        }
        let below = heads
            .iter()
            .map(|(at, _)| *at)
            .min()
            .map_or(i64::MAX, |at| at.saturating_add(1));
        for table in BY_TIME {
            let at: Option<i64> = self
                .db
                .query_row(
                    &format!(
                        "SELECT at_us FROM {table} WHERE at_us < ?1{} ORDER BY at_us LIMIT 1",
                        unowned(table)
                    ),
                    [below],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(at) = at {
                heads.push((at, Oldest::Unowned(table)));
            }
        }
        heads.sort_unstable_by_key(|(at, _)| *at);
        let mut heads = heads.into_iter();
        Ok(heads.next().map(|(_, oldest)| {
            let until = heads
                .next()
                .map_or(i64::MAX, |(at, _)| at.saturating_add(1));
            (oldest, until)
        }))
    }

    /// Deletes the [`FORGET_ROWS`] oldest rows of `table`, one of [`BY_TIME`], that are older than
    /// `until` and that no request or run names, in one transaction. Returns how many it deleted.
    fn forget_rows(&mut self, table: &str, until: i64) -> rusqlite::Result<u64> {
        let deleted = self.db.execute(
            &format!(
                "DELETE FROM {table} WHERE rowid IN (SELECT rowid FROM {table} \
                 WHERE at_us < ?1{} ORDER BY at_us LIMIT ?2)",
                unowned(table)
            ),
            params![until, FORGET_ROWS],
        )?;
        Ok(u64::try_from(deleted).unwrap_or(u64::MAX))
    }

    /// Deletes the [`FORGET_BATCH`] oldest rows of `owner`, one of [`OWNERS`], that started before
    /// `cutoff`, with every row that names their trace ids, in one transaction. Answers how many
    /// it deleted.
    fn forget_batch(&mut self, owner: &str, cutoff: i64) -> rusqlite::Result<u64> {
        let tx = self.db.transaction()?;
        tx.execute_batch("CREATE TEMP TABLE IF NOT EXISTS doomed (id INTEGER, trace TEXT)")?;
        tx.execute("DELETE FROM temp.doomed", [])?;
        let picked = tx.execute(
            &format!(
                "INSERT INTO temp.doomed (id, trace) \
                 SELECT id, trace FROM {owner} WHERE start_us < ?1 ORDER BY start_us LIMIT ?2"
            ),
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
            &format!("DELETE FROM {owner} WHERE id IN (SELECT id FROM temp.doomed)"),
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

/// What [`Index::oldest`] found oldest.
#[derive(Clone, Copy, Debug)]
enum Oldest {
    /// The oldest request or run, in a table of [`OWNERS`].
    Owner(&'static str),
    /// The oldest row that no request or run names, in a table of [`BY_TIME`].
    Unowned(&'static str),
}

/// Whether `table` has a `trace` column.
fn has_trace(table: &str) -> bool {
    !matches!(table, "dropped" | "unparseable")
}

/// The condition that no request or run names a row of `table`, one of [`BY_TIME`], as an
/// ` AND …` to append to a `WHERE`. A table without a `trace` column has no owner, so it adds
/// nothing. Each test is one lookup on the owner's `trace` index.
fn unowned(table: &str) -> String {
    if has_trace(table) {
        format!(
            " AND NOT EXISTS (SELECT 1 FROM requests WHERE requests.trace = {table}.trace) \
             AND NOT EXISTS (SELECT 1 FROM runs WHERE runs.trace = {table}.trace)"
        )
    } else {
        String::new()
    }
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
