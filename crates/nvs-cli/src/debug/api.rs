//! The viewer's JSON API over the index
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//!
//! **One read connection, and every list is a page.** The listener's site holds one [`Api`], and
//! every connection thread takes its connection in turn: a page is one indexed query, so the wait
//! is short, and the index has one reader beside its writer. The index is in write-ahead mode, so
//! that reader never waits on the ingester and the ingester never waits on it.
//!
//! **The watermark says when, the cursor says where.** `/api/events` pushes the last committed
//! batch. `GET /api/requests` reads the watermark and the page in one read transaction, so the
//! number it gives is the batch the page is complete up to. The newest page has no cursor;
//! `before=<id>` is the page older than a row the viewer has, and `since=<batch>` is what arrived
//! after a watermark the viewer had. A request's id only rises (the index module's *The viewer's
//! cursor*), so neither moves a row the viewer already shows.
//!
//! **One request is its trace id and span id.** `GET /api/request?trace=&span=` gives the row,
//! its uncaught exception, and its trace events, dumps and log lines in time order. The records
//! are keyed by trace id alone, so when an inbound `traceparent` gives several requests one trace
//! id, a request's records are those its writer wrote between the previous request of that trace
//! and its own `request` line. Two such requests in flight at once on one thread cannot be told
//! apart, and each shows both.
//!
//! **A profile is downloaded by its row.** A request's answer lists the files its
//! `Core\Debug::startProfile` calls named, and `GET /api/profile?id=` sends the file one row names,
//! as an attachment of at most [`PROFILE_MAX`] bytes. The page names a row and never a path, so it
//! reads only a file a program wrote under its `debug.profile` grant.
//!
//! **A request's coverage is its own path.** A request's answer says whether its server counted
//! the lines it ran, and [`super::source`] serves those counts and the files they name.
//!
//! **The stream, the search and the settings are their own modules.** `GET /api/stream` is
//! [`super::stream`], `GET /api/search` is [`super::search`], `GET /api/exceptions` is
//! [`super::exceptions`], and `GET` and `PUT /api/settings` are [`super::settings`]. The first
//! three read through this connection.
//!
//! **Discarding the index closes this connection first.** `POST /api/index/discard` drops the
//! connection, asks the ingest loop to discard and create the file, and waits for it, all while it
//! holds the connection's lock. A file another handle has open cannot be deleted on Windows, and no
//! API request opens the old file again in between. The next API request opens the new one.
//!
//! **What it spends:** one SQLite connection and its page cache for the service, and one page of
//! at most [`PAGE_MAX`] rows per API request while it is written out. One request's answer is at
//! most [`EVENTS_MAX`] events, [`DUMPS_MAX`] dumps, [`LOGS_MAX`] log lines and [`PROFILES_MAX`]
//! profiles, and `cut` says which list stopped there. A profile download holds its file in memory
//! while it is sent.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError, mpsc};
use std::time::Duration;

use hyper::StatusCode;
use rusqlite::{Connection, OpenFlags, OptionalExtension as _, ToSql, params};
use serde::Serialize;
use serde_json::value::RawValue;

use super::Signal;

/// The rows in a page when the viewer names no `limit`.
pub(super) const PAGE_DEFAULT: i64 = 100;

/// How long `POST /api/index/discard` waits for the ingest loop. The loop may be in a pass, which
/// reads at most 16 MiB per source.
const REBUILD_WAIT: Duration = Duration::from_secs(60);

/// The most rows in one page.
pub(crate) const PAGE_MAX: i64 = 500;

/// The most profiles in one request's answer.
const PROFILES_MAX: i64 = 100;

/// The largest profile `GET /api/profile` sends. A larger one is opened from its path.
const PROFILE_MAX: u64 = 256 << 20;

/// The most trace events in one request's answer.
const EVENTS_MAX: i64 = 5_000;

/// The most dumps in one request's answer. A dump's line can be the spool's longest.
const DUMPS_MAX: i64 = 200;

/// The most log lines in one request's answer.
const LOGS_MAX: i64 = 1_000;

/// The index, opened for reading, and the way to the ingest loop.
pub(crate) struct Api {
    path: PathBuf,
    /// `None` while the index is discarded, and until the next API request opens it again.
    db: Mutex<Option<Connection>>,
    signal: mpsc::Sender<Signal>,
    /// The folder `GET /api/source` reads under ([`super::source::root`]).
    root: Option<PathBuf>,
}

/// One page of `GET /api/requests`.
#[derive(Serialize)]
struct Page {
    /// The last batch this page is complete up to.
    watermark: i64,
    /// The requests, newest first.
    requests: Vec<Row>,
    /// The `before` that gives the next older page, or `None` when this page has the oldest.
    older: Option<i64>,
}

/// One request in the list: the columns the rule names for the main view.
#[derive(Serialize)]
pub(super) struct Row {
    pub(super) id: i64,
    batch: i64,
    pub(super) trace: String,
    pub(super) span: Option<String>,
    pub(super) start_us: i64,
    duration_us: i64,
    method: String,
    route: Option<String>,
    path: String,
    status: i64,
    queries: i64,
    dumps: i64,
    logs: i64,
    exception: Option<Threw>,
}

/// The exception a request did not catch: its class and the site of its `throw`.
#[derive(Serialize)]
struct Threw {
    class: String,
    file: String,
    line: i64,
}

/// `GET /api/request`: one request and everything it recorded, each list in time order.
#[derive(Serialize)]
struct Detail {
    /// The last batch this answer is complete up to.
    watermark: i64,
    request: Row,
    /// The exception it did not catch, with its message.
    exception: Option<Exception>,
    events: Vec<Event>,
    dumps: Vec<Dump>,
    logs: Vec<Log>,
    profiles: Vec<ProfileFile>,
    /// The lines it ran, when its server counted them: `GET /api/coverage?id=` reads them.
    coverage: Option<Covered>,
    /// Which lists stopped at their most rows.
    cut: Cut,
}

/// A profile file the request's `Core\Debug::startProfile` named.
#[derive(Serialize)]
struct ProfileFile {
    id: i64,
    at_us: i64,
    path: String,
}

/// That a request has coverage, and the row that holds it.
#[derive(Serialize)]
struct Covered {
    id: i64,
    /// How many files it ran lines of.
    files: i64,
    /// Whether the spool line was too long, so no line is listed.
    elided: bool,
}

/// An uncaught exception: its class, its message and the site of its `throw`.
#[derive(Serialize)]
struct Exception {
    class: String,
    message: String,
    file: String,
    line: i64,
}

/// One trace event: a call, a query, a spawn, a garbage-collection pause or an outbound call.
#[derive(Serialize)]
struct Event {
    id: i64,
    kind: String,
    label: String,
    start_us: i64,
    end_us: i64,
    /// A call's checked-return status.
    status: Option<i64>,
    query: Option<EventQuery>,
}

/// A `query` event's driver, SQL, rows and call site.
#[derive(Serialize)]
struct EventQuery {
    driver: String,
    sql: String,
    rows: Option<i64>,
    site: Option<i64>,
}

/// One `Core\Debug::dump`: its site and its value tree in the JSON rendering.
#[derive(Serialize)]
struct Dump {
    id: i64,
    at_us: i64,
    file: String,
    line: i64,
    tree: Box<RawValue>,
    elided: bool,
}

/// One log line the request wrote.
#[derive(Serialize)]
struct Log {
    id: i64,
    at_us: i64,
    level: String,
    message: String,
    fields: Box<RawValue>,
    elided: bool,
    lossy: bool,
}

/// Whether each list of a [`Detail`] stopped at its most rows.
#[derive(Default, Serialize)]
struct Cut {
    events: bool,
    dumps: bool,
    logs: bool,
    profiles: bool,
}

/// What `GET /api/request` was asked for: a trace id, and the span id when the trace has several.
#[derive(Debug)]
struct Key {
    trace: String,
    span: Option<String>,
}

/// Why `GET /api/request` names no one request.
enum Missing {
    None,
    Several,
}

/// What `GET /api/requests` was asked for.
#[derive(Default)]
struct Filter {
    before: Option<i64>,
    since: Option<i64>,
    trace: Option<String>,
    span: Option<String>,
    limit: Option<i64>,
}

impl Api {
    /// Opens the index at `path` for reading. The ingester has created it already, and `signal`
    /// reaches its loop.
    ///
    /// # Errors
    ///
    /// When SQLite cannot open the file.
    pub(crate) fn open(path: &Path, signal: mpsc::Sender<Signal>) -> rusqlite::Result<Self> {
        let db = connect(path)?;
        Ok(Self {
            path: path.to_owned(),
            db: Mutex::new(Some(db)),
            signal,
            root: super::source::root(),
        })
    }

    /// The folder `GET /api/source` reads under, or `None` when it cannot be read.
    pub(super) fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Runs `read` on the connection, opening it first when a discard closed it.
    pub(super) fn read<T>(
        &self,
        read: impl FnOnce(&mut Connection) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let mut db = self.db.lock().unwrap_or_else(PoisonError::into_inner);
        if db.is_none() {
            *db = Some(connect(&self.path)?);
        }
        match db.as_mut() {
            Some(db) => read(db),
            None => Err(rusqlite::Error::InvalidQuery),
        }
    }

    /// `GET /api/requests` with `query`: the status and the JSON body.
    pub(crate) fn requests(&self, query: Option<&str>) -> (StatusCode, Vec<u8>) {
        let filter = match Filter::parse(query.unwrap_or_default()) {
            Ok(filter) => filter,
            Err(message) => return error(StatusCode::BAD_REQUEST, &message),
        };
        answer(self.read(|db| page(db, &filter)))
    }

    /// `GET /api/request` with `query`: the status and the JSON body.
    pub(crate) fn request(&self, query: Option<&str>) -> (StatusCode, Vec<u8>) {
        let key = match Key::parse(query.unwrap_or_default()) {
            Ok(key) => key,
            Err(message) => return error(StatusCode::BAD_REQUEST, &message),
        };
        match self.read(|db| detail(db, &key)) {
            Ok(Ok(detail)) => answer(Ok(detail)),
            Ok(Err(Missing::None)) => error(
                StatusCode::NOT_FOUND,
                "No request in the index has this trace id and span id.",
            ),
            Ok(Err(Missing::Several)) => error(
                StatusCode::BAD_REQUEST,
                "Several requests have this trace id. Add `span` to choose one.",
            ),
            Err(err) => answer::<()>(Err(err)),
        }
    }

    /// `GET /api/profile?id=`: the file name and the bytes of the profile the row names, or the
    /// status and JSON body of an error.
    pub(crate) fn profile(
        &self,
        query: Option<&str>,
    ) -> Result<(String, Vec<u8>), (StatusCode, Vec<u8>)> {
        let id = match query.unwrap_or_default().split_once('=') {
            Some(("id", value)) if !value.contains('&') => number("id", value),
            _ => Err("`id` is the one key, and it is required.".to_owned()),
        }
        .map_err(|message| error(StatusCode::BAD_REQUEST, &message))?;
        let path: Option<String> = self
            .read(|db| {
                db.query_row("SELECT path FROM profiles WHERE id = ?1", [id], |row| {
                    row.get(0)
                })
                .optional()
            })
            .map_err(|err| answer::<()>(Err(err)))?;
        let Some(path) = path else {
            return Err(error(
                StatusCode::NOT_FOUND,
                "No profile in the index has this id.",
            ));
        };
        let unreadable = |err: std::io::Error| {
            error(
                StatusCode::NOT_FOUND,
                &format!("The profile `{path}` cannot be read: {err}"),
            )
        };
        let size = std::fs::metadata(&path).map_err(unreadable)?.len();
        if size > PROFILE_MAX {
            return Err(error(
                StatusCode::PAYLOAD_TOO_LARGE,
                &format!("The profile `{path}` is larger than 256 MiB. Open it from disk."),
            ));
        }
        let bytes = std::fs::read(&path).map_err(unreadable)?;
        let name = Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok((name, bytes))
    }

    /// `POST /api/index/discard`: the module doc's *Discarding the index closes this connection
    /// first*. The body is `{"watermark": <the new index's first batch>}`.
    pub(crate) fn discard(&self) -> (StatusCode, Vec<u8>) {
        let mut db = self.db.lock().unwrap_or_else(PoisonError::into_inner);
        *db = None;
        let (reply, done) = mpsc::channel();
        if self.signal.send(Signal::Discard(reply)).is_err() {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "The service is stopping, so the index cannot be discarded.",
            );
        }
        match done.recv_timeout(REBUILD_WAIT) {
            Ok(Ok(watermark)) => (
                StatusCode::OK,
                serde_json::json!({ "watermark": watermark })
                    .to_string()
                    .into_bytes(),
            ),
            Ok(Err(message)) => error(StatusCode::SERVICE_UNAVAILABLE, &message),
            Err(_) => error(
                StatusCode::SERVICE_UNAVAILABLE,
                "The service did not discard the index in time.",
            ),
        }
    }
}

/// A read-only connection to the index at `path`.
fn connect(path: &Path) -> rusqlite::Result<Connection> {
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    db.busy_timeout(Duration::from_secs(1))?;
    Ok(db)
}

/// The status and JSON body of a read: `200` with the value, or `503` when the index failed.
pub(super) fn answer<T: Serialize>(read: rusqlite::Result<T>) -> (StatusCode, Vec<u8>) {
    match read {
        Ok(value) => match serde_json::to_vec(&value) {
            Ok(body) => (StatusCode::OK, body),
            Err(err) => error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
        },
        Err(err) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("The index cannot be read: {err}"),
        ),
    }
}

impl Key {
    /// Reads `trace`, which is required, and `span`. Any other key is an error.
    fn parse(query: &str) -> Result<Self, String> {
        let (mut trace, mut span) = (None, None);
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "trace" => trace = Some(hex(key, value, 32)?),
                "span" => span = Some(hex(key, value, 16)?),
                _ => return Err(format!("`{key}` is not a key of one request.")),
            }
        }
        let trace = trace.ok_or("`trace` is required.")?;
        Ok(Self { trace, span })
    }
}

impl Filter {
    /// Reads `before`, `since`, `trace`, `span` and `limit`. Any other key is an error, so a
    /// viewer that sends one finds out.
    fn parse(query: &str) -> Result<Self, String> {
        let mut filter = Self::default();
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "before" => filter.before = Some(number(key, value)?),
                "since" => filter.since = Some(number(key, value)?),
                "limit" => {
                    let limit = number(key, value)?;
                    if !(1..=PAGE_MAX).contains(&limit) {
                        return Err(format!("`limit` must be from 1 to {PAGE_MAX}."));
                    }
                    filter.limit = Some(limit);
                }
                "trace" => filter.trace = Some(hex(key, value, 32)?),
                "span" => filter.span = Some(hex(key, value, 16)?),
                _ => return Err(format!("`{key}` is not a filter of this list.")),
            }
        }
        Ok(filter)
    }
}

/// `value` as a whole number that is not negative.
pub(super) fn number(key: &str, value: &str) -> Result<i64, String> {
    value
        .parse::<i64>()
        .ok()
        .filter(|number| *number >= 0)
        .ok_or_else(|| format!("`{key}` must be a whole number."))
}

/// `value` when it is `digits` lower-case hex digits.
pub(super) fn hex(key: &str, value: &str, digits: usize) -> Result<String, String> {
    if value.len() == digits
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        Ok(value.to_owned())
    } else {
        Err(format!("`{key}` must be {digits} lower-case hex digits."))
    }
}

/// The page `filter` names, read with the watermark in one transaction.
fn page(db: &mut Connection, filter: &Filter) -> rusqlite::Result<Page> {
    let limit = filter.limit.unwrap_or(PAGE_DEFAULT);
    let mut wheres = Vec::new();
    let mut values: Vec<&dyn ToSql> = Vec::new();
    if let Some(before) = &filter.before {
        wheres.push("r.id < ?");
        values.push(before);
    }
    if let Some(since) = &filter.since {
        wheres.push("r.batch > ?");
        values.push(since);
    }
    if let Some(trace) = &filter.trace {
        wheres.push("r.trace = ?");
        values.push(trace);
    }
    if let Some(span) = &filter.span {
        wheres.push("r.span = ?");
        values.push(span);
    }
    // One row past the page says whether an older page exists.
    let fetched = limit + 1;
    values.push(&fetched);
    let sql = format!(
        "SELECT {ROW} FROM requests r LEFT JOIN exception_groups g ON g.id = r.exception_group \
         {} ORDER BY r.id DESC LIMIT ?",
        if wheres.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", wheres.join(" AND "))
        }
    );
    let tx = db.transaction()?;
    let watermark = watermark(&tx)?;
    let mut requests = {
        let mut statement = tx.prepare_cached(&sql)?;
        statement
            .query_map(values.as_slice(), row)?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    tx.commit()?;
    let full = usize::try_from(limit).unwrap_or(usize::MAX);
    let older = (requests.len() > full).then(|| {
        requests.truncate(full);
        requests.last().map_or(0, |row| row.id)
    });
    Ok(Page {
        watermark,
        requests,
        older,
    })
}

/// The columns [`row`] reads, over `requests r` joined to its `exception_groups g`.
pub(super) const ROW: &str = "r.id, r.batch, r.trace, r.span, r.start_us, r.duration_us, r.method, r.route, \
                   r.path, r.status, r.queries, r.dumps, r.logs, g.class, g.file, g.line";

/// One [`Row`] from the columns [`ROW`] names, in its order.
pub(super) fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    let class: Option<String> = row.get(13)?;
    Ok(Row {
        id: row.get(0)?,
        batch: row.get(1)?,
        trace: row.get(2)?,
        span: row.get(3)?,
        start_us: row.get(4)?,
        duration_us: row.get(5)?,
        method: row.get(6)?,
        route: row.get(7)?,
        path: row.get(8)?,
        status: row.get(9)?,
        queries: row.get(10)?,
        dumps: row.get(11)?,
        logs: row.get(12)?,
        exception: match class {
            Some(class) => Some(Threw {
                class,
                file: row.get(14)?,
                line: row.get(15)?,
            }),
            None => None,
        },
    })
}

/// The last committed batch.
pub(super) fn watermark(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<i64> {
    tx.query_row(
        "SELECT value FROM meta WHERE key = 'watermark'",
        [],
        |row| row.get(0),
    )
}

/// The request `key` names and everything it recorded, read with the watermark in one
/// transaction, or why the key names no one request.
fn detail(db: &mut Connection, key: &Key) -> rusqlite::Result<Result<Detail, Missing>> {
    let tx = db.transaction()?;
    let watermark = watermark(&tx)?;
    let mut found = {
        let mut statement = tx.prepare_cached(&format!(
            "SELECT {ROW}, r.writer, r.seq FROM requests r \
             LEFT JOIN exception_groups g ON g.id = r.exception_group \
             WHERE r.trace = ?1 AND (?2 IS NULL OR r.span = ?2) ORDER BY r.id DESC LIMIT 2"
        ))?;
        statement
            .query_map(params![key.trace, key.span], |found| {
                Ok((
                    row(found)?,
                    found.get::<_, String>(16)?,
                    found.get::<_, i64>(17)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    if found.is_empty() {
        return Ok(Err(Missing::None));
    }
    if found.len() > 1 && key.span.is_none() {
        return Ok(Err(Missing::Several));
    }
    let (request, writer, seq) = found.swap_remove(0);
    let exception = tx
        .query_row(
            "SELECT class, message, file, line FROM exceptions WHERE request = ?1",
            [request.id],
            |row| {
                Ok(Exception {
                    class: row.get(0)?,
                    message: row.get(1)?,
                    file: row.get(2)?,
                    line: row.get(3)?,
                })
            },
        )
        .optional()?;
    let shared: i64 = tx.query_row(
        "SELECT count(*) FROM requests WHERE trace = ?1",
        [&key.trace],
        |row| row.get(0),
    )?;
    // A trace id that names one request owns every record of the trace. When several share it,
    // a request's records are the ones its writer wrote after the previous request of that trace
    // and before this one's own line, and a tailed log line is the one inside its time.
    let (writer, after) = if shared > 1 {
        let after: Option<i64> = tx.query_row(
            "SELECT max(seq) FROM requests WHERE trace = ?1 AND writer = ?2 AND seq < ?3",
            params![key.trace, writer, seq],
            |row| row.get(0),
        )?;
        (Some(writer), after.unwrap_or(-1))
    } else {
        (None, 0)
    };
    let (before, from_us, to_us) = (
        seq,
        request.start_us,
        request.start_us + request.duration_us,
    );
    let mut cut = Cut::default();
    let events = {
        let mut statement = tx.prepare_cached(
            "SELECT id, kind, label, start_us, end_us, status, driver, sql, rows, site FROM events \
             WHERE trace = ?1 AND (?2 IS NULL OR (writer = ?2 AND seq > ?3 AND seq < ?4)) \
             ORDER BY start_us, id LIMIT ?5",
        )?;
        let mut events = statement
            .query_map(
                params![key.trace, writer, after, before, EVENTS_MAX + 1],
                |row| {
                    let driver: Option<String> = row.get(6)?;
                    Ok(Event {
                        id: row.get(0)?,
                        kind: row.get(1)?,
                        label: row.get(2)?,
                        start_us: row.get(3)?,
                        end_us: row.get(4)?,
                        status: row.get(5)?,
                        query: match driver {
                            Some(driver) => Some(EventQuery {
                                driver,
                                sql: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                                rows: row.get(8)?,
                                site: row.get(9)?,
                            }),
                            None => None,
                        },
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        cut.events = keep(&mut events, EVENTS_MAX);
        events
    };
    let dumps = {
        let mut statement = tx.prepare_cached(
            "SELECT id, at_us, file, line, tree, elided FROM dumps \
             WHERE trace = ?1 AND (?2 IS NULL OR (writer = ?2 AND seq > ?3 AND seq < ?4)) \
             ORDER BY at_us, id LIMIT ?5",
        )?;
        let mut dumps = statement
            .query_map(
                params![key.trace, writer, after, before, DUMPS_MAX + 1],
                |row| {
                    Ok(Dump {
                        id: row.get(0)?,
                        at_us: row.get(1)?,
                        file: row.get(2)?,
                        line: row.get(3)?,
                        tree: tree(&row.get::<_, String>(4)?),
                        elided: row.get(5)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        cut.dumps = keep(&mut dumps, DUMPS_MAX);
        dumps
    };
    let logs = {
        let mut statement = tx.prepare_cached(
            "SELECT id, at_us, level, message, fields, elided, lossy FROM logs \
             WHERE trace = ?1 AND (?2 IS NULL OR (writer = ?2 AND seq > ?3 AND seq < ?4) \
             OR (writer IS NULL AND at_us BETWEEN ?5 AND ?6)) \
             ORDER BY at_us, id LIMIT ?7",
        )?;
        let mut logs = statement
            .query_map(
                params![
                    key.trace,
                    writer,
                    after,
                    before,
                    from_us,
                    to_us,
                    LOGS_MAX + 1
                ],
                |row| {
                    Ok(Log {
                        id: row.get(0)?,
                        at_us: row.get(1)?,
                        level: row.get(2)?,
                        message: row.get(3)?,
                        fields: tree(&row.get::<_, String>(4)?),
                        elided: row.get(5)?,
                        lossy: row.get(6)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        cut.logs = keep(&mut logs, LOGS_MAX);
        logs
    };
    let profiles = {
        let mut statement = tx.prepare_cached(
            "SELECT id, at_us, path FROM profiles \
             WHERE trace = ?1 AND (?2 IS NULL OR (writer = ?2 AND seq > ?3 AND seq < ?4)) \
             ORDER BY at_us, id LIMIT ?5",
        )?;
        let mut profiles = statement
            .query_map(
                params![key.trace, writer, after, before, PROFILES_MAX + 1],
                |row| {
                    Ok(ProfileFile {
                        id: row.get(0)?,
                        at_us: row.get(1)?,
                        path: row.get(2)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        cut.profiles = keep(&mut profiles, PROFILES_MAX);
        profiles
    };
    // The coverage line carries the span id, and a streamed answer writes it after its `request`
    // line, so it is found by span rather than by the writer's sequence.
    let coverage = tx
        .query_row(
            "SELECT id, (SELECT count(*) FROM json_each(files)), elided FROM coverage              WHERE trace = ?1 AND (?2 IS NULL OR span = ?2) ORDER BY id LIMIT 1",
            params![key.trace, request.span],
            |row| {
                Ok(Covered {
                    id: row.get(0)?,
                    files: row.get(1)?,
                    elided: row.get(2)?,
                })
            },
        )
        .optional()?;
    tx.commit()?;
    Ok(Ok(Detail {
        watermark,
        request,
        exception,
        events,
        dumps,
        logs,
        profiles,
        coverage,
        cut,
    }))
}

/// Truncates `rows`, read with one row past `max`, to `max`, and says whether that cut any.
fn keep<T>(rows: &mut Vec<T>, max: i64) -> bool {
    let max = usize::try_from(max).unwrap_or(usize::MAX);
    let over = rows.len() > max;
    rows.truncate(max);
    over
}

/// A value tree the index has as JSON text, written into the answer as it is. A tree the ingester
/// wrote is always JSON, so text that is not is given as a string.
pub(super) fn tree(text: &str) -> Box<RawValue> {
    match serde_json::from_str::<&RawValue>(text) {
        Ok(raw) => raw.to_owned(),
        Err(_) => {
            serde_json::value::to_raw_value(text).unwrap_or_else(|_| RawValue::NULL.to_owned())
        }
    }
}

/// A JSON error body with `message`.
pub(super) fn error(status: StatusCode, message: &str) -> (StatusCode, Vec<u8>) {
    let body = serde_json::json!({ "error": message });
    (status, body.to_string().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filter_reads_its_keys_and_names_a_wrong_one() {
        let filter =
            Filter::parse("before=40&since=3&limit=20&trace=4bf92f3577b34da6a3ce929d0e0e4736")
                .expect("a valid filter");
        assert_eq!(filter.before, Some(40));
        assert_eq!(filter.since, Some(3));
        assert_eq!(filter.limit, Some(20));
        assert!(filter.trace.is_some());
        assert!(Filter::parse("").is_ok());
        assert!(Filter::parse("limit=0").is_err());
        assert!(Filter::parse("limit=501").is_err());
        assert!(Filter::parse("before=-1").is_err());
        assert!(Filter::parse("span=00F067AA0BA902B7").is_err());
        assert!(Filter::parse("order=asc").is_err());
    }

    #[test]
    fn a_request_key_needs_its_trace_id() {
        let key = Key::parse("trace=4bf92f3577b34da6a3ce929d0e0e4736&span=00f067aa0ba902b7")
            .expect("a valid key");
        assert_eq!(key.span.as_deref(), Some("00f067aa0ba902b7"));
        assert!(Key::parse("span=00f067aa0ba902b7").is_err());
        assert!(Key::parse("trace=4bf92f3577b34da6a3ce929d0e0e4736&limit=3").is_err());
    }
}
