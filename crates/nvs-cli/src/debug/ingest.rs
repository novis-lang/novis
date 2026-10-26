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
//! # A bad line, and a bad file
//!
//! `rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`. [`Lines`] reads a file one
//! line at a time and keeps at most [`spool::LINE_MAX`] bytes of a line: a longer one is skipped to
//! its newline and arrives cut. A cut line, a line that is not UTF-8, and a line that is not a spool
//! line are each written to `unparseable` with the file, the offset, the whole length and the first
//! [`EXCERPT`] bytes, converted lossily with `lossy` set where they are not UTF-8. The writer writes
//! UTF-8 alone, so a spool line that is not UTF-8 is damaged and is never inserted as a record.
//!
//! One call reads at most the budget it is given, [`PASS_BYTES`] per source per pass, and passes
//! the budget only to finish the line it is in. A file the budget stopped in is committed as far as
//! it was read and is not deleted, and [`Read::next`] is where the next pass goes on.
//!
//! A file that cannot be opened, read or deleted is the caller's to quarantine with [`quarantine`],
//! under a [`Backoff`], and [`release`] lists it no more. An index error deletes no file.
//!
//! # Which files, and how often
//!
//! [`ready`] lists the spool directory for every sealed file and every unsealed one untouched for
//! [`spool::UNSEALED_AFTER`], ordered by writer and then by file number, so one writer's lines
//! arrive in the order it wrote them. [`wait`] is the delay before the next listing: the seal age
//! while files arrive, and [`QUIET_WAIT`] once none has arrived for [`QUIET_AFTER`].
//!
//! **What it spends:** one [`READ_BUFFER`] and one line of at most [`spool::LINE_MAX`] while a
//! file is read, and the open transaction's pages, which SQLite spills to disk past its cache.

use std::borrow::Cow;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
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
pub(crate) const EXCERPT: usize = 4 << 10;
/// The most bytes one source gives one pass.
pub(crate) const PASS_BYTES: u64 = 16 << 20;
/// The read buffer of a file being ingested.
pub(crate) const READ_BUFFER: usize = 64 << 10;
/// The first wait after a failure. Each next one doubles, up to [`BACKOFF_MAX`].
pub(crate) const BACKOFF_FIRST: Duration = Duration::from_secs(1);
/// The longest wait after a failure.
pub(crate) const BACKOFF_MAX: Duration = Duration::from_secs(300);

/// What one [`file`] call read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Read {
    /// The lines that were records.
    pub(crate) lines: u64,
    /// The lines that were not.
    pub(crate) unparseable: u64,
    /// The bytes read.
    pub(crate) bytes: u64,
    /// Where the next pass goes on, when the budget stopped before the end of the file.
    pub(crate) next: Option<u64>,
}

/// When a source that failed is tried again: [`BACKOFF_FIRST`] after its first failure, doubling
/// with each next one up to [`BACKOFF_MAX`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Backoff {
    /// The failures in a row, the first one counted as one.
    pub(crate) failures: u32,
    /// The wait before the next try.
    pub(crate) delay: Duration,
    /// The time of the next try.
    pub(crate) until: Instant,
}

impl Backoff {
    /// The backoff after a failure at `now`, when `previous` was the one before it.
    #[must_use]
    pub(crate) fn after(previous: Option<Self>, now: Instant) -> Self {
        let failures = previous.map_or(1, |previous| previous.failures.saturating_add(1));
        let doublings = (failures - 1).min(16);
        let delay = BACKOFF_FIRST
            .saturating_mul(1 << doublings)
            .min(BACKOFF_MAX);
        Self {
            failures,
            delay,
            until: now + delay,
        }
    }
}

/// One line [`Lines`] read.
pub(crate) struct Raw {
    /// Where the line starts in its file.
    pub(crate) offset: u64,
    /// Its first [`spool::LINE_MAX`] bytes, with no newline.
    pub(crate) bytes: Vec<u8>,
    /// Its whole length, with no newline.
    pub(crate) length: u64,
}

impl Raw {
    /// Whether the line is longer than [`spool::LINE_MAX`], so [`Self::bytes`] is its start.
    #[must_use]
    pub(crate) fn cut(&self) -> bool {
        self.length > len(spool::LINE_MAX)
    }
}

/// The lines of a file from one offset, up to a budget. See the module doc.
pub(crate) struct Lines<R> {
    reader: R,
    offset: u64,
    end: u64,
    eof: bool,
    unended: bool,
}

impl<R: BufRead> Lines<R> {
    /// The lines `reader` gives, when it is at byte `offset` of its file, until `budget` bytes are
    /// read. With `unended`, a last line with no newline is a line; without it, that line is left
    /// for a later read, unless it is already past [`spool::LINE_MAX`].
    pub(crate) fn new(reader: R, offset: u64, budget: u64, unended: bool) -> Self {
        Self {
            reader,
            offset,
            end: offset.saturating_add(budget),
            eof: false,
            unended,
        }
    }

    /// The offset after the last line returned.
    pub(crate) fn offset(&self) -> u64 {
        self.offset
    }

    /// Whether the end of the file was reached.
    pub(crate) fn at_end(&self) -> bool {
        self.eof
    }

    /// The next line that is not empty, or `None` at the end of the file or of the budget.
    ///
    /// # Errors
    ///
    /// When the file cannot be read.
    pub(crate) fn next_line(&mut self) -> std::io::Result<Option<Raw>> {
        while !self.eof && self.offset < self.end {
            let mut bytes = Vec::new();
            let mut length = 0;
            let mut ended = false;
            loop {
                let buf = self.reader.fill_buf()?;
                if buf.is_empty() {
                    self.eof = true;
                    break;
                }
                let newline = buf.iter().position(|&byte| byte == b'\n');
                let line = &buf[..newline.unwrap_or(buf.len())];
                let room = spool::LINE_MAX.saturating_sub(bytes.len());
                bytes.extend_from_slice(&line[..line.len().min(room)]);
                length += len(line.len());
                let take = newline.map_or(buf.len(), |at| at + 1);
                self.reader.consume(take);
                if newline.is_some() {
                    ended = true;
                    break;
                }
            }
            let raw = Raw {
                offset: self.offset,
                bytes,
                length,
            };
            if !ended && (length == 0 || !(self.unended || raw.cut())) {
                return Ok(None);
            }
            self.offset += length + u64::from(ended);
            if length > 0 {
                return Ok(Some(raw));
            }
        }
        Ok(None)
    }
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

/// Ingests the spool file at `path` from byte `from` into `index` at `now_us`, reading at most
/// `budget` bytes, and deletes it once the batch that reached its end is committed. See the module
/// doc.
///
/// # Errors
///
/// When the file cannot be read, or the batch cannot be written. Either leaves the file in place.
pub(crate) fn file(
    index: &mut Index,
    path: &Path,
    from: u64,
    budget: u64,
    now_us: u64,
) -> Result<Read, Failed> {
    let mut opened = File::open(path).map_err(Failed::Read)?;
    if from > 0 {
        opened.seek(SeekFrom::Start(from)).map_err(Failed::Read)?;
    }
    let source = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let mut lines = Lines::new(
        BufReader::with_capacity(READ_BUFFER, opened),
        from,
        budget,
        true,
    );
    let tx = index.db.transaction().map_err(Failed::Index)?;
    let batch = watermark(&tx).map_err(Failed::Index)?;
    let mut read = Read::default();
    while let Some(raw) = lines.next_line().map_err(Failed::Read)? {
        let bad = if raw.cut() {
            Some((Cow::Borrowed(TOO_LONG), false))
        } else {
            match std::str::from_utf8(&raw.bytes) {
                Err(_) => Some((Cow::Borrowed("the line is not UTF-8"), true)),
                Ok(text) => match Line::parse(text) {
                    Ok(line) => match insert(&tx, &line, batch) {
                        Ok(()) => None,
                        Err(Insert::Bad(reason)) => Some((Cow::Borrowed(reason), false)),
                        Err(Insert::Index(err)) => return Err(Failed::Index(err)),
                    },
                    Err(err) => Some((Cow::Owned(err.to_string()), false)),
                },
            }
        };
        match bad {
            None => read.lines += 1,
            Some((reason, lossy)) => {
                read.unparseable += 1;
                let bad = Bad {
                    source: &source,
                    source_key: &source,
                    reason: &reason,
                    lossy,
                };
                unparseable(&tx, batch, now_us, &bad, &raw).map_err(Failed::Index)?;
            }
        }
    }
    read.bytes = lines.offset() - from;
    let done = lines.at_end();
    tx.commit().map_err(Failed::Index)?;
    if done {
        match std::fs::remove_file(path) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                return Err(Failed::Delete(err));
            }
            _ => {}
        }
    } else {
        read.next = Some(lines.offset());
    }
    Ok(read)
}

/// The reason a line longer than [`spool::LINE_MAX`] is in `unparseable`.
pub(crate) const TOO_LONG: &str = "the line is longer than 4 MiB";

/// Raises the watermark in `meta`, and answers the batch number it is now.
pub(crate) fn watermark(tx: &Transaction<'_>) -> rusqlite::Result<i64> {
    tx.query_row(
        "UPDATE meta SET value = value + 1 WHERE key = 'watermark' RETURNING value",
        [],
        |row| row.get(0),
    )
}

/// Lists `path` in `quarantine` at `now_us`: why it failed, how often in a row, and when it is
/// tried again.
///
/// # Errors
///
/// When the index cannot be written.
pub(crate) fn quarantine(
    index: &Index,
    path: &Path,
    reason: &str,
    backoff: Backoff,
    now_us: u64,
) -> rusqlite::Result<()> {
    let delay = u64::try_from(backoff.delay.as_micros()).unwrap_or(u64::MAX);
    index
        .db
        .execute(
            "INSERT INTO quarantine (path, at_us, retry_us, failures, reason) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT (path) DO UPDATE SET retry_us = excluded.retry_us, \
             failures = excluded.failures, reason = excluded.reason",
            params![
                path.display().to_string(),
                int(now_us),
                int(now_us.saturating_add(delay)),
                backoff.failures,
                reason
            ],
        )
        .map(drop)
}

/// Lists `path` in `quarantine` no more.
///
/// # Errors
///
/// When the index cannot be written.
pub(crate) fn release(index: &Index, path: &Path) -> rusqlite::Result<()> {
    index
        .db
        .execute(
            "DELETE FROM quarantine WHERE path = ?1",
            [path.display().to_string()],
        )
        .map(drop)
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

/// What [`unparseable`] writes beside a line: where it came from and why it is bad.
pub(crate) struct Bad<'a> {
    /// The file, as the viewer shows it.
    pub(crate) source: &'a str,
    /// The file, as the dedupe key names it.
    pub(crate) source_key: &'a str,
    /// Why the line is not a record.
    pub(crate) reason: &'a str,
    /// Whether the line is not UTF-8, so its excerpt was converted lossily.
    pub(crate) lossy: bool,
}

/// Writes the bad line `raw` into `unparseable`, with its first [`EXCERPT`] bytes.
///
/// # Errors
///
/// When the index cannot be written.
pub(crate) fn unparseable(
    tx: &Transaction<'_>,
    batch: i64,
    now_us: u64,
    bad: &Bad<'_>,
    raw: &Raw,
) -> rusqlite::Result<()> {
    let mut end = raw.bytes.len().min(EXCERPT);
    while end > 0 && end < raw.bytes.len() && raw.bytes[end] & 0xC0 == 0x80 {
        end -= 1;
    }
    let excerpt = String::from_utf8_lossy(&raw.bytes[..end]);
    tx.execute(
        "INSERT OR IGNORE INTO unparseable \
         (batch, at_us, source, source_key, offset, length, reason, excerpt, lossy) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            batch,
            int(now_us),
            bad.source,
            bad.source_key,
            int(raw.offset),
            int(raw.length),
            bad.reason,
            excerpt,
            bad.lossy,
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
pub(crate) fn int(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// A length as a file offset.
fn len(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
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

    /// Every line `bytes` gives from `offset` within `budget`, as `(offset, length, cut)`, and the
    /// offset and end the reader stopped at.
    fn lines_of(bytes: &[u8], budget: u64, unended: bool) -> (Vec<(u64, u64, bool)>, u64, bool) {
        let mut lines = Lines::new(bytes, 0, budget, unended);
        let mut found = Vec::new();
        while let Some(raw) = lines.next_line().unwrap() {
            assert!(raw.bytes.len() <= spool::LINE_MAX);
            found.push((raw.offset, raw.length, raw.cut()));
        }
        (found, lines.offset(), lines.at_end())
    }

    #[test]
    fn a_line_past_the_cap_is_cut_and_the_next_line_is_read_whole() {
        let mut bytes = vec![b'x'; spool::LINE_MAX + 10];
        bytes.extend_from_slice(b"\n\nok\n");
        let long = len(spool::LINE_MAX + 10);
        let (found, offset, end) = lines_of(&bytes, u64::MAX, true);
        assert_eq!(found, vec![(0, long, true), (long + 2, 2, false)]);
        assert_eq!((offset, end), (len(bytes.len()), true));
    }

    #[test]
    fn an_unended_line_waits_unless_the_file_is_sealed_or_it_is_past_the_cap() {
        let (found, offset, _) = lines_of(b"one\ntw", u64::MAX, false);
        assert_eq!((found, offset), (vec![(0, 3, false)], 4));
        let (found, offset, _) = lines_of(b"one\ntw", u64::MAX, true);
        assert_eq!((found, offset), (vec![(0, 3, false), (4, 2, false)], 6));
        let huge = vec![b'x'; spool::LINE_MAX + 1];
        let (found, _, _) = lines_of(&huge, u64::MAX, false);
        assert_eq!(found, vec![(0, len(huge.len()), true)]);
    }

    #[test]
    fn the_budget_stops_at_the_end_of_the_line_it_is_in() {
        let (found, offset, end) = lines_of(b"aaaa\nbb\ncc\n", 6, true);
        assert_eq!(found, vec![(0, 4, false), (5, 2, false)]);
        assert_eq!((offset, end), (8, false));
    }

    #[test]
    fn a_backoff_doubles_from_one_second_to_five_minutes() {
        let now = Instant::now();
        let mut backoff = None;
        let mut delays = Vec::new();
        for _ in 0..12 {
            let next = Backoff::after(backoff, now);
            delays.push(next.delay.as_secs());
            backoff = Some(next);
        }
        assert_eq!(delays, [1, 2, 4, 8, 16, 32, 64, 128, 256, 300, 300, 300]);
    }
}
