//! The ingester: what moves sealed spool files into the index
//! (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`).
//!
//! # Read, insert, commit, then delete
//!
//! [`file`] reads one spool file whole, inserts each line in one transaction that also raises the
//! watermark in `meta`, commits, and only then deletes the file. A crash between the commit and the
//! delete leaves the file to be read again, and every row a line makes is inserted under the
//! line's dedupe key, `(writer, seq)`, ignoring a conflict, so the second read adds nothing. A
//! request line inserts its `requests` row first and the rest of what it records only when that
//! row is new, so its exception group's count is raised once.
//!
//! A line that is not a spool line is written to `unparseable` with its file and offset, and the
//! file still goes. What bounds that work per file, and what a file that cannot be read becomes, is
//! `rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`'s.
//!
//! # Which files, and how often
//!
//! [`ready`] lists the spool directory for every sealed file and every unsealed one untouched for
//! [`spool::UNSEALED_AFTER`], ordered by writer and then by file number, so one writer's lines
//! arrive in the order it wrote them. [`wait`] is the delay before the next listing: the seal age
//! while files arrive, and [`QUIET_WAIT`] once none has arrived for [`QUIET_AFTER`].
//!
//! **What it spends:** one spool file's bytes and its parsed lines per pass. A sealed file is at
//! most [`spool::SEAL_BYTES`] and one line past it.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use nvs_runtime::spool::{self, Body, BodyOf, EventKind, Line, Record};
use rusqlite::{Transaction, params};

use super::index::Index;

/// The delay between listings once no file has arrived for [`QUIET_AFTER`].
pub(crate) const QUIET_WAIT: Duration = Duration::from_secs(2);
/// How long without a file before the listing slows to [`QUIET_WAIT`].
pub(crate) const QUIET_AFTER: Duration = Duration::from_secs(60);
/// The most bytes of a bad line kept in `unparseable`.
const EXCERPT: usize = 256;

/// What one [`file`] call read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Read {
    /// The lines that were spool lines.
    pub(crate) lines: u64,
    /// The lines that were not.
    pub(crate) unparseable: u64,
}

/// The delay before the next listing, when the last file arrived at `last_arrival`.
#[must_use]
pub(crate) fn wait(last_arrival: Instant, now: Instant) -> Duration {
    if now.saturating_duration_since(last_arrival) < QUIET_AFTER {
        spool::SEAL_AFTER
    } else {
        QUIET_WAIT
    }
}

/// The files in `dir` ready to ingest at `now`, in the order to ingest them.
///
/// # Errors
///
/// When `dir` cannot be listed.
pub(crate) fn ready(dir: &Path, now: SystemTime) -> std::io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let Some((writer, number, extension)) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(named)
        else {
            continue;
        };
        let due = match extension {
            spool::SEALED => true,
            spool::OPEN => {
                entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .is_ok_and(|modified| {
                        now.duration_since(modified)
                            .is_ok_and(|age| age >= spool::UNSEALED_AFTER)
                    })
            }
            _ => false,
        };
        if due {
            found.push(((writer.to_owned(), number), path));
        }
    }
    found.sort();
    Ok(found.into_iter().map(|(_, path)| path).collect())
}

/// A spool file's name as its writer, its number and its extension.
fn named(name: &str) -> Option<(&str, u64, &str)> {
    let mut parts = name.rsplitn(3, '.');
    let extension = parts.next()?;
    let number = parts.next()?.parse().ok()?;
    let writer = parts.next()?;
    Some((writer, number, extension))
}

/// Ingests the spool file at `path` into `index` at `now_us`, and deletes it once the batch is
/// committed. See the module doc.
///
/// # Errors
///
/// When the file cannot be read, or the batch cannot be written. Either leaves the file in place.
pub(crate) fn file(index: &mut Index, path: &Path, now_us: u64) -> Result<Read, Failed> {
    let bytes = std::fs::read(path).map_err(Failed::Read)?;
    let source = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let tx = index.db.transaction().map_err(Failed::Index)?;
    let batch: i64 = tx
        .query_row(
            "UPDATE meta SET value = value + 1 WHERE key = 'watermark' RETURNING value",
            [],
            |row| row.get(0),
        )
        .map_err(Failed::Index)?;
    let mut read = Read::default();
    let mut offset = 0usize;
    for text in bytes.split(|&byte| byte == b'\n') {
        let at = offset;
        offset += text.len() + 1;
        if text.is_empty() {
            continue;
        }
        let parsed = std::str::from_utf8(text)
            .map_err(|err| err.to_string())
            .and_then(|text| Line::parse(text).map_err(|err| err.to_string()));
        let bad = match parsed {
            Ok(line) => match insert(&tx, &line, batch) {
                Ok(()) => None,
                Err(Insert::Bad(reason)) => Some(reason.to_owned()),
                Err(Insert::Index(err)) => return Err(Failed::Index(err)),
            },
            Err(reason) => Some(reason),
        };
        match bad {
            None => read.lines += 1,
            Some(reason) => {
                read.unparseable += 1;
                unparseable(&tx, batch, now_us, &source, at, &reason, text)
                    .map_err(Failed::Index)?;
            }
        }
    }
    tx.commit().map_err(Failed::Index)?;
    std::fs::remove_file(path).map_err(Failed::Delete)?;
    Ok(read)
}

/// Why a [`file`] call stopped.
#[derive(Debug)]
pub(crate) enum Failed {
    /// The file could not be read.
    Read(std::io::Error),
    /// The batch could not be written to the index.
    Index(rusqlite::Error),
    /// The batch was committed, and the file could not be deleted. The next pass reads it again
    /// and adds nothing.
    Delete(std::io::Error),
}

/// Why [`insert`] did not insert a line.
enum Insert {
    /// The index failed.
    Index(rusqlite::Error),
    /// The line is a spool line that is missing what its kind needs.
    Bad(&'static str),
}

impl From<rusqlite::Error> for Insert {
    fn from(err: rusqlite::Error) -> Self {
        Self::Index(err)
    }
}

/// Inserts the rows `line` makes, in batch `batch`.
fn insert(tx: &Transaction<'_>, line: &Line, batch: i64) -> Result<(), Insert> {
    let key = (line.writer.as_str(), int(line.seq));
    let at = int(line.at_us);
    let trace = line.trace.as_deref();
    match &line.record {
        Record::Request(request) => {
            let trace = trace.ok_or(Insert::Bad("a request line has no trace id"))?;
            let inserted = tx.execute(
                "INSERT OR IGNORE INTO requests \
                 (writer, seq, batch, trace, start_us, duration_us, method, route, path, status) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    key.0,
                    key.1,
                    batch,
                    trace,
                    int(request.start_us),
                    int(request.duration_us),
                    request.method,
                    request.route,
                    request.path,
                    request.status,
                ],
            )?;
            if inserted == 0 {
                return Ok(());
            }
            let id = tx.last_insert_rowid();
            if let Some(exception) = &request.exception {
                let start = int(request.start_us);
                let group: i64 = tx.query_row(
                    "INSERT INTO exception_groups (class, file, line, count, first_us, last_us) \
                     VALUES (?1, ?2, ?3, 1, ?4, ?4) \
                     ON CONFLICT (class, file, line) DO UPDATE SET count = count + 1, \
                     first_us = min(first_us, excluded.first_us), \
                     last_us = max(last_us, excluded.last_us) \
                     RETURNING id",
                    params![exception.class, exception.file, exception.line, start],
                    |row| row.get(0),
                )?;
                tx.execute(
                    "UPDATE requests SET exception_group = ?1 WHERE id = ?2",
                    params![group, id],
                )?;
                tx.execute(
                    "INSERT INTO exceptions \
                     (request, trace, class, message, file, line, exception_group) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        id,
                        trace,
                        exception.class,
                        exception.message,
                        exception.file,
                        exception.line,
                        group,
                    ],
                )?;
            }
            if request.input.is_some() || request.response.is_some() {
                let input = request.input.as_ref();
                let response = request.response.as_ref();
                let body = input.map(|input| &input.body);
                let out = response.map(|response| &response.body);
                tx.execute(
                    "INSERT INTO inputs (request, trace, query, headers, body, body_read, \
                     body_declared, body_cut, body_chunks, body_withheld, response_headers, \
                     response_body, response_read, response_cut, response_chunks, \
                     response_withheld) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                    params![
                        id,
                        trace,
                        input.map_or("", |input| input.query.as_str()),
                        json(&input.map(|input| &input.headers)),
                        body.and_then(Body::bytes),
                        body.is_some_and(|body| body.read),
                        body.and_then(|body| body.declared).map(int),
                        body.is_some_and(|body| body.cut),
                        body.and_then(|body| body.chunks).map(int),
                        body.is_some_and(|body| body.withheld),
                        response.map(|response| json(&response.headers)),
                        out.and_then(Body::bytes),
                        out.map(|body| body.read),
                        out.map(|body| body.cut),
                        out.and_then(|body| body.chunks).map(int),
                        out.map(|body| body.withheld),
                    ],
                )?;
            }
        }
        Record::Event(event) => {
            let trace = trace.ok_or(Insert::Bad("an event line has no trace id"))?;
            let query = event.query.as_ref();
            tx.execute(
                "INSERT OR IGNORE INTO events (writer, seq, batch, trace, at_us, kind, label, \
                 start_us, end_us, status, driver, sql, rows, site) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    key.0,
                    key.1,
                    batch,
                    trace,
                    at,
                    kind(event.event),
                    event.label,
                    int(event.start_us),
                    int(event.end_us),
                    event.status,
                    query.map(|query| query.driver.as_str()),
                    query.map(|query| query.sql.as_str()),
                    query.and_then(|query| query.rows).map(int),
                    query.and_then(|query| query.site),
                ],
            )?;
        }
        Record::Dump(dump) => {
            let trace = trace.ok_or(Insert::Bad("a dump line has no trace id"))?;
            tx.execute(
                "INSERT OR IGNORE INTO dumps \
                 (writer, seq, batch, trace, at_us, file, line, tree, elided) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    key.0,
                    key.1,
                    batch,
                    trace,
                    at,
                    dump.file,
                    dump.line,
                    json(&dump.tree),
                    dump.elided,
                ],
            )?;
        }
        Record::Log(log) => {
            tx.execute(
                "INSERT OR IGNORE INTO logs \
                 (writer, seq, batch, trace, at_us, level, message, fields, elided) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    key.0,
                    key.1,
                    batch,
                    trace,
                    at,
                    log.level,
                    log.message,
                    json(&log.fields),
                    log.elided,
                ],
            )?;
        }
        Record::Body(part) => {
            let trace = trace.ok_or(Insert::Bad("a body line has no trace id"))?;
            let bytes = Body {
                read: true,
                text: part.text.clone(),
                base64: part.base64.clone(),
                ..Body::default()
            }
            .bytes()
            .ok_or(Insert::Bad("a body line's base64 does not decode"))?;
            tx.execute(
                "INSERT OR IGNORE INTO body_parts \
                 (writer, seq, batch, trace, at_us, of, offset, bytes) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    key.0,
                    key.1,
                    batch,
                    trace,
                    at,
                    match part.of {
                        BodyOf::Request => "request",
                        BodyOf::Response => "response",
                    },
                    int(part.offset),
                    bytes,
                ],
            )?;
        }
        Record::Dropped(dropped) => {
            tx.execute(
                "INSERT OR IGNORE INTO dropped (writer, seq, batch, at_us, records, files) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    key.0,
                    key.1,
                    batch,
                    at,
                    int(dropped.records),
                    int(dropped.files)
                ],
            )?;
        }
    }
    Ok(())
}

/// Writes a bad line at `offset` of `source` into `unparseable`.
fn unparseable(
    tx: &Transaction<'_>,
    batch: i64,
    now_us: u64,
    source: &str,
    offset: usize,
    reason: &str,
    text: &[u8],
) -> rusqlite::Result<()> {
    let excerpt = String::from_utf8_lossy(&text[..text.len().min(EXCERPT)]);
    tx.execute(
        "INSERT OR IGNORE INTO unparseable (batch, at_us, source, offset, reason, excerpt) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            batch,
            int(now_us),
            source,
            i64::try_from(offset).unwrap_or(i64::MAX),
            reason,
            excerpt
        ],
    )
    .map(drop)
}

/// The name the index stores an event's kind under, as the spool spells it.
const fn kind(kind: EventKind) -> &'static str {
    match kind {
        EventKind::Call => "call",
        EventKind::Gc => "gc",
        EventKind::Spawn => "spawn",
        EventKind::Query => "query",
        EventKind::Http => "http",
    }
}

/// `value` as JSON text.
fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// `value` as SQLite's integer. A spool value past `i64::MAX` is no real time or count.
fn int(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Whether a file named `name` is one [`ready`] would list.
#[cfg(test)]
fn is_spool(name: &str) -> bool {
    named(name).is_some_and(|(_, _, ext)| ext == spool::SEALED || ext == spool::OPEN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_listing_slows_after_a_quiet_minute() {
        let start = Instant::now();
        assert_eq!(wait(start, start), spool::SEAL_AFTER);
        assert_eq!(
            wait(start, start + QUIET_AFTER - Duration::from_millis(1)),
            spool::SEAL_AFTER
        );
        assert_eq!(wait(start, start + QUIET_AFTER), QUIET_WAIT);
    }

    #[test]
    fn a_spool_file_name_is_its_writer_number_and_extension() {
        assert_eq!(
            named("4242-1700000000000-0.12.spool"),
            Some(("4242-1700000000000-0", 12, "spool"))
        );
        assert!(is_spool("4242-1-0.0.open"));
        assert!(!is_spool("index.sqlite"));
        assert!(!is_spool("notes.txt"));
    }
}
