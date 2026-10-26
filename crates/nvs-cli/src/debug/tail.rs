//! `nvs debug --tail <file>`: a log file read from a checkpoint, and never written
//! (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`).
//!
//! # The checkpoint
//!
//! A tailed file's `checkpoints` row is its path, its key, and the offset read up to. The key is
//! the file's identity (device and inode, or volume and file number) and a hash of its first line,
//! because a path misses a rotation, and an identity alone misses an identity reused and a file
//! truncated in place. A file is keyed once it has a first line, or [`EXCERPT`] bytes of one.
//!
//! When the key at the path is the checkpoint's, the file is read from the checkpoint, or from its
//! start when it is shorter than that. When it is not, the file was replaced and is read from its
//! start. Before that, `<file>.1`, the name a Novis log is rotated to, is read on from the
//! checkpoint when its key is the checkpoint's, so the lines written just before a rotation are
//! kept. The rows and the checkpoint are committed in one transaction, so a crash reads no line
//! twice.
//!
//! # A line
//!
//! A line is a record in the `json` log format. Its `ts`, `level`, `msg`, `trace_id` and `fields`
//! fill a `logs` row keyed by the file's key and the line's offset, so a line read twice is
//! inserted once. A line that is not such a record is an `unparseable` row, as a spool line is
//! (`rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`). Invalid UTF-8 is converted
//! lossily, and the row says `lossy`. A last line with no newline is read once it has one.
//!
//! **What it spends:** one [`READ_BUFFER`] and one line of at most
//! [`LINE_MAX`](nvs_runtime::spool::LINE_MAX) while a
//! tailed file is read. Nothing is held between passes.

use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, Read as _, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, Transaction, params};

use super::index::Index;
use super::ingest::{
    self, Bad, EXCERPT, Failed, Lines, READ_BUFFER, Raw, Read, TOO_LONG, int, unparseable,
};

/// The reason a tailed line that is not a log record is in `unparseable`.
const NOT_A_RECORD: &str = "the line is not a `json` log record";

/// Reads what is new in the log file at `path` into `index` at `now_us`, at most `budget` bytes.
///
/// # Errors
///
/// When the file cannot be opened or read, or the batch cannot be written.
pub(crate) fn pass(
    index: &mut Index,
    path: &Path,
    budget: u64,
    now_us: u64,
) -> Result<Read, Failed> {
    let Some(current) = Opened::open(path).map_err(Failed::Read)? else {
        return Ok(Read::default());
    };
    let checkpoint = path.display().to_string();
    let saved: Option<(String, i64)> = index
        .db
        .query_row(
            "SELECT source_key, offset FROM checkpoints WHERE path = ?1",
            [&checkpoint],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(Failed::Index)?;
    let saved = saved.map(|(key, offset)| (key, u64::try_from(offset).unwrap_or(0)));
    let mut read = Read::default();
    let from = match saved {
        Some((key, offset)) if key == current.key => {
            if offset <= current.len {
                offset
            } else {
                0
            }
        }
        Some((key, offset)) => {
            let rotated = rotated(path);
            if let Some(old) = Opened::open(&rotated)
                .ok()
                .flatten()
                .filter(|old| old.key == key && offset <= old.len)
            {
                let source = rotated.display().to_string();
                let (rest, done) = lines(index, &checkpoint, old, &source, offset, budget, now_us)?;
                read = rest;
                if !done {
                    return Ok(read);
                }
            }
            0
        }
        None => 0,
    };
    let left = budget.saturating_sub(read.bytes);
    if from == current.len || left == 0 {
        return Ok(read);
    }
    let (new, _) = lines(index, &checkpoint, current, &checkpoint, from, left, now_us)?;
    Ok(Read {
        lines: read.lines + new.lines,
        unparseable: read.unparseable + new.unparseable,
        bytes: read.bytes + new.bytes,
        next: None,
    })
}

/// The name a Novis log at `path` is rotated to.
fn rotated(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".1");
    PathBuf::from(name)
}

/// A tailed file, open, with its key and its length.
struct Opened {
    file: File,
    key: String,
    len: u64,
}

impl Opened {
    /// The file at `path`, or `None` while it has no first line to key it by.
    fn open(path: &Path) -> std::io::Result<Option<Self>> {
        let mut file = File::open(path)?;
        let len = file.metadata()?.len();
        let mut first = Vec::with_capacity(EXCERPT);
        (&mut file)
            .take(u64::try_from(EXCERPT).unwrap_or(u64::MAX))
            .read_to_end(&mut first)?;
        let first = match first.iter().position(|&byte| byte == b'\n') {
            Some(at) => &first[..at],
            None if first.len() == EXCERPT => &first[..],
            None => return Ok(None),
        };
        let hash = blake3::hash(first).to_hex();
        let hash = &hash.as_str()[..16];
        let key = match crate::reload::file_identity(&file) {
            Some((device, number)) => format!("{device:x}-{number:x}-{hash}"),
            None => hash.to_owned(),
        };
        Ok(Some(Self { file, key, len }))
    }
}

/// Reads `opened` from `from`, at most `budget` bytes, as lines of `source`, and moves the
/// checkpoint of `checkpoint` to the file's key and where the read stopped, in one transaction.
/// Answers what it read and whether it reached the end of the file.
fn lines(
    index: &mut Index,
    checkpoint: &str,
    mut opened: Opened,
    source: &str,
    from: u64,
    budget: u64,
    now_us: u64,
) -> Result<(Read, bool), Failed> {
    opened
        .file
        .seek(SeekFrom::Start(from))
        .map_err(Failed::Read)?;
    let reader = BufReader::with_capacity(READ_BUFFER, opened.file);
    let mut lines = Lines::new(reader, from, budget, false);
    let tx = index.db.transaction().map_err(Failed::Index)?;
    let batch = ingest::watermark(&tx).map_err(Failed::Index)?;
    let mut read = Read::default();
    while let Some(raw) = lines.next_line().map_err(Failed::Read)? {
        if line(&tx, batch, now_us, source, &opened.key, &raw).map_err(Failed::Index)? {
            read.lines += 1;
        } else {
            read.unparseable += 1;
        }
    }
    read.bytes = lines.offset() - from;
    tx.execute(
        "INSERT INTO checkpoints (path, source_key, offset) VALUES (?1, ?2, ?3) \
         ON CONFLICT (path) DO UPDATE SET source_key = excluded.source_key, \
         offset = excluded.offset",
        params![checkpoint, opened.key, int(lines.offset())],
    )
    .map_err(Failed::Index)?;
    tx.commit().map_err(Failed::Index)?;
    Ok((read, lines.at_end()))
}

/// Inserts the tailed line `raw` as a `logs` row, or as an `unparseable` one. Answers whether it
/// was a log record.
fn line(
    tx: &Transaction<'_>,
    batch: i64,
    now_us: u64,
    source: &str,
    key: &str,
    raw: &Raw,
) -> rusqlite::Result<bool> {
    let (text, lossy) = match std::str::from_utf8(&raw.bytes) {
        Ok(text) => (Cow::Borrowed(text), false),
        Err(_) => (String::from_utf8_lossy(&raw.bytes), true),
    };
    let record = if raw.cut() { None } else { Record::of(&text) };
    let Some(record) = record else {
        let reason = if raw.cut() { TOO_LONG } else { NOT_A_RECORD };
        let bad = Bad {
            source,
            source_key: key,
            reason,
            lossy,
        };
        unparseable(tx, batch, now_us, &bad, raw)?;
        return Ok(false);
    };
    tx.execute(
        "INSERT OR IGNORE INTO logs (source, source_key, offset, batch, trace, at_us, level, \
         message, fields, elided, lossy) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10)",
        params![
            source,
            key,
            int(raw.offset),
            batch,
            record.trace,
            record.at_us.unwrap_or_else(|| int(now_us)),
            record.level,
            record.message,
            record.fields,
            lossy,
        ],
    )?;
    Ok(true)
}

/// What a `json` log record gives a `logs` row.
struct Record {
    at_us: Option<i64>,
    level: String,
    message: String,
    trace: Option<String>,
    fields: String,
}

impl Record {
    /// The record `text` is, or `None` when it is not a JSON object with a `level`.
    fn of(text: &str) -> Option<Self> {
        let serde_json::Value::Object(record) = serde_json::from_str(text).ok()? else {
            return None;
        };
        let text = |key: &str| record.get(key).and_then(serde_json::Value::as_str);
        Some(Self {
            at_us: text("ts")
                .and_then(|ts| ts.parse::<jiff::Timestamp>().ok())
                .map(jiff::Timestamp::as_microsecond),
            level: text("level")?.to_owned(),
            message: text("msg").unwrap_or_default().to_owned(),
            trace: text("trace_id").map(str::to_owned),
            fields: record
                .get("fields")
                .filter(|fields| fields.is_object())
                .map_or_else(|| "{}".to_owned(), ToString::to_string),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_json_log_record_gives_its_time_level_message_trace_and_fields() {
        let record = Record::of(
            r#"{"ts":"2026-01-02T03:04:05.000006Z","level":"warn","msg":"slow","trace_id":"ab","fields":{"n":1}}"#,
        )
        .unwrap();
        assert_eq!(record.at_us, Some(1_767_323_045_000_006));
        assert_eq!(
            (record.level.as_str(), record.message.as_str()),
            ("warn", "slow")
        );
        assert_eq!(record.trace.as_deref(), Some("ab"));
        assert_eq!(record.fields, r#"{"n":1}"#);
        assert!(Record::of(r#"{"msg":"no level"}"#).is_none());
        assert!(Record::of("12:00 INFO text format").is_none());
    }
}
