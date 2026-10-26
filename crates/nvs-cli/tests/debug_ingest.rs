//! `nvs debug --once` over the committed sample spool and log file
//! (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`,
//! `rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time`,
//! `rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`).
//!
//! Each case writes copies of `fixtures/debug/one-of-each.spool` into a fresh data folder's spool,
//! with each copy's writer, trace id and times rewritten, runs one pass, and reads the index back.
//!
//! The bad-line cases write a line past the cap, a line that is not a spool line and a line that is
//! not UTF-8, and the quarantine case a directory named like a spool file, which no runner's OS
//! opens and reads as one. The `--tail` cases copy `fixtures/debug/a-json-log.log` into the data
//! folder, outside the spool, and append to it, rotate it and truncate it between passes. Every
//! time in the committed bad-line spool and log file is in 2100, so the deletion window keeps
//! their rows.

use std::ffi::OsStr;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use nvs_runtime::spool::{LINE_MAX, Line, Record};
use rusqlite::Connection;

/// The tables whose rows name a trace id.
const TRACED: [&str; 6] = [
    "requests",
    "inputs",
    "body_parts",
    "events",
    "dumps",
    "logs",
];

const DAY_US: u64 = 24 * 60 * 60 * 1_000_000;

/// The trace id of the one record in `a-json-log.log` that has one.
const LOG_TRACE: &str = "4bf92f3577b34da6a3ce929d0e0e4736";

/// `2100-01-01T00:00:00Z` in Unix microseconds.
const YEAR_2100_US: i64 = 4_102_444_800_000_000;

fn fixture(name: &str) -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "debug",
        name,
    ]
    .iter()
    .collect()
}

fn sample() -> Vec<Line> {
    std::fs::read_to_string(fixture("one-of-each.spool"))
        .expect("the sample spool is committed")
        .lines()
        .map(|text| Line::parse(text).expect("every sample line parses"))
        .collect()
}

fn now_us() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the clock is past 1970")
            .as_micros(),
    )
    .expect("the time fits")
}

/// The sample, written by `writer` under `trace`, starting at `start_us`.
fn copy(writer: &str, trace: &str, start_us: u64) -> Vec<Line> {
    let mut lines = sample();
    for (n, line) in lines.iter_mut().enumerate() {
        line.writer = writer.to_owned();
        line.at_us = start_us + u64::try_from(n).expect("few lines") * 1_000;
        if line.trace.is_some() {
            line.trace = Some(trace.to_owned());
        }
        match &mut line.record {
            Record::Request(request) => request.start_us = start_us,
            Record::Event(event) => {
                event.start_us = start_us + 100;
                event.end_us = start_us + 400;
            }
            _ => {}
        }
    }
    lines
}

/// A data folder whose spool `nvs debug` has created.
struct Folder {
    data: nvs_repo::Scratch,
}

impl Folder {
    fn new() -> Self {
        let folder = Self {
            data: nvs_repo::scratch_private("nvsdata"),
        };
        folder.ingest();
        folder
    }

    fn spool(&self) -> PathBuf {
        self.data.join("debug").join("spool")
    }

    /// Writes `lines` as the sealed file `<writer>.<n>.spool`.
    fn write(&self, name: &str, lines: &[Line]) {
        let text: String = lines
            .iter()
            .map(|line| serde_json::to_string(line).expect("a line serializes") + "\n")
            .collect();
        std::fs::write(self.spool().join(name), text).expect("the spool is writable");
    }

    /// One `nvs debug --once` pass.
    fn ingest(&self) {
        self.ingest_with(&[]);
    }

    /// One `nvs debug --once` pass, with `extra` after `--once`.
    fn ingest_with(&self, extra: &[&OsStr]) {
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .current_dir(&*self.data)
            .arg("--data")
            .arg(&*self.data)
            .arg("--no-init")
            .arg("debug")
            .arg("--once")
            .args(extra)
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        assert!(
            out.status.success(),
            "`nvs debug --once` succeeds: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// One pass that also reads the log file at `path`.
    fn tail(&self, path: &Path) {
        self.ingest_with(&[OsStr::new("--tail"), path.as_os_str()]);
    }

    /// The path of a log file named `name`, in the data folder and outside the spool.
    fn log_path(&self, name: &str) -> PathBuf {
        let dir = self.data.join("logs");
        std::fs::create_dir_all(&dir).expect("the data folder is writable");
        dir.join(name)
    }

    fn index(&self) -> Connection {
        Connection::open(self.data.join("debug").join("index.sqlite"))
            .expect("the pass created the index")
    }

    fn files_left(&self) -> usize {
        std::fs::read_dir(self.spool())
            .expect("the spool lists")
            .count()
    }
}

fn count(db: &Connection, sql: &str) -> i64 {
    db.query_row(sql, [], |row| row.get(0))
        .unwrap_or_else(|err| panic!("{sql}: {err}"))
}

fn counts(db: &Connection) -> Vec<(String, i64)> {
    [
        "requests",
        "inputs",
        "body_parts",
        "events",
        "dumps",
        "logs",
        "exceptions",
        "exception_groups",
        "coverage",
        "dropped",
        "search",
        "unparseable",
    ]
    .iter()
    .map(|table| {
        (
            (*table).to_owned(),
            count(db, &format!("SELECT count(*) FROM {table}")),
        )
    })
    .collect()
}

fn path_of(data: &Path) -> String {
    data.display().to_string()
}

fn int(n: usize) -> i64 {
    i64::try_from(n).expect("the length fits")
}

/// Where each line of `bytes` starts.
fn line_starts(bytes: &[u8]) -> Vec<i64> {
    let mut starts = vec![0];
    for (at, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' && at + 1 < bytes.len() {
            starts.push(int(at + 1));
        }
    }
    starts
}

/// A `json` log record of `msg` at `2100-01-01T00:01:00Z`, with its newline.
fn record(msg: &str) -> String {
    format!(
        "{{\"ts\":\"2100-01-01T00:01:00Z\",\"level\":\"info\",\"msg\":\"{msg}\",\"fields\":{{}}}}\n"
    )
}

fn append(path: &Path, bytes: &[u8]) {
    std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("the log file opens")
        .write_all(bytes)
        .expect("the log file is writable");
}

/// The name the index gives the tailed file at `path`: its absolute path.
fn source_of(path: &Path) -> String {
    std::path::absolute(path)
        .expect("the path is absolute")
        .display()
        .to_string()
}

/// The `logs` rows of `source`, in the order they were inserted: offset, time, message and trace.
fn tailed(db: &Connection, source: &str) -> Vec<(i64, i64, String, Option<String>)> {
    db.prepare("SELECT offset, at_us, message, trace FROM logs WHERE source = ?1 ORDER BY id")
        .expect("the logs table exists")
        .query_map([source], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("the query runs")
        .collect::<Result<_, _>>()
        .expect("each row reads")
}

fn messages(rows: &[(i64, i64, String, Option<String>)]) -> Vec<&str> {
    rows.iter().map(|row| row.2.as_str()).collect()
}

#[test]
fn a_second_service_on_one_data_folder_exits_while_the_lock_is_held() {
    let folder = Folder::new();
    let lock = std::fs::File::options()
        .write(true)
        .open(folder.data.join("debug").join("service.lock"))
        .expect("the first pass created the lock file");
    lock.try_lock().expect("no service is running");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .current_dir(&*folder.data)
        .arg("--data")
        .arg(&*folder.data)
        .arg("--no-init")
        .arg("debug")
        .arg("--once")
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(!out.status.success(), "the second service does not run");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("already running"),
        "it says why: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    drop(lock);
    folder.ingest();
}

#[test]
fn a_crash_between_commit_and_delete_ingests_nothing_twice() {
    let folder = Folder::new();
    let lines = copy("1-1-0", "00000000000000000000000000000001", now_us());
    folder.write("1-1-0.0.spool", &lines);
    folder.ingest();
    assert_eq!(
        folder.files_left(),
        0,
        "the file is deleted after its commit"
    );
    let db = folder.index();
    let first = counts(&db);
    assert!(
        first
            .iter()
            .all(|(table, n)| *n > 0 || table == "unparseable"),
        "every kind of line made its row: {first:?} in {}",
        path_of(&folder.data)
    );

    // The same file again, as a crash after the commit and before the delete leaves it.
    folder.write("1-1-0.0.spool", &lines);
    folder.ingest();
    assert_eq!(counts(&db), first, "the second read adds no row");
    assert_eq!(
        count(&db, "SELECT count FROM exception_groups"),
        1,
        "the group counts the exception once"
    );
    assert_eq!(
        count(&db, "SELECT value FROM meta WHERE key = 'watermark'"),
        2,
        "each committed batch raises the watermark"
    );
    let row: (i64, i64, i64) = db
        .query_row("SELECT queries, dumps, logs FROM requests", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .expect("one request");
    assert_eq!(row, (1, 1, 1), "the request counts its query, dump and log");
}

#[test]
fn a_search_finds_a_dumps_text() {
    let folder = Folder::new();
    let mut lines = copy("2-1-0", "00000000000000000000000000000002", now_us());
    for line in &mut lines {
        if let Record::Dump(dump) = &mut line.record {
            dump.tree = serde_json::json!({"class": "Shop\\Item", "fields": {"name": "marmalade"}});
        }
    }
    folder.write("2-1-0.0.spool", &lines);
    folder.ingest();
    let db = folder.index();
    let found: Vec<i64> = db
        .prepare("SELECT rowid FROM search WHERE search MATCH 'marmalade'")
        .expect("the search table exists")
        .query_map([], |row| row.get(0))
        .expect("the search runs")
        .collect::<Result<_, _>>()
        .expect("each row reads");
    let dump = count(&db, "SELECT id FROM dumps");
    assert_eq!(found, vec![dump * 8 + 3], "one row, naming the dump");
}

#[test]
fn ten_throws_from_one_site_make_one_group_of_ten() {
    let folder = Folder::new();
    let start = now_us();
    let mut lines = Vec::new();
    for n in 0..10u64 {
        let mut copied = copy("3-1-0", &format!("{n:032x}"), start + n * 10_000);
        for line in &mut copied {
            line.seq += n * 100;
            if let Record::Request(request) = &mut line.record
                && let Some(exception) = &mut request.exception
            {
                exception.message = format!("item {n} is out of stock");
            }
        }
        lines.extend(copied);
    }
    folder.write("3-1-0.0.spool", &lines);
    folder.ingest();
    let db = folder.index();
    let (groups, total, first, last): (i64, i64, i64, i64) = db
        .query_row(
            "SELECT count(*), sum(count), min(first_us), max(last_us) FROM exception_groups",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("the groups read");
    assert_eq!((groups, total), (1, 10), "one class and site, ten throws");
    assert_eq!(
        (first, last),
        (
            i64::try_from(start).expect("fits"),
            i64::try_from(start + 9 * 10_000).expect("fits")
        )
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM requests WHERE exception_group = (SELECT id FROM exception_groups)"
        ),
        10
    );
}

/// The bytes of the pages `db` has in use, as `Index::forget` measures them.
fn bytes_in_use(db: &Connection) -> i64 {
    (count(db, "PRAGMA page_count") - count(db, "PRAGMA freelist_count"))
        * count(db, "PRAGMA page_size")
}

/// `[debug] index_max` deletes the oldest requests first, each with every row that names it, until
/// the pages in use fit, and a bound smaller than the empty schema deletes every request and still
/// ends. A deletion transaction takes 1000 requests, so the index holds 2500, which needs more than
/// one.
#[test]
fn index_max_deletes_the_oldest_requests_until_the_index_fits() {
    let folder = Folder::new();
    let now = now_us();
    let start = |n: u64| now - 2_500_000_000 + n * 1_000_000;
    let mut lines = Vec::new();
    for n in 0..2500 {
        let mut one = copy("6-1-0", &format!("{n:032x}"), start(n));
        for line in &mut one {
            line.seq += n * 100;
        }
        lines.extend(one);
    }
    folder.write("6-1-0.0.spool", &lines);
    folder.ingest();
    let full = bytes_in_use(&folder.index());
    assert_eq!(
        count(&folder.index(), "SELECT count(*) FROM requests"),
        2500
    );

    let config = folder.data.join("small.toml");
    let bound = full / 4 * 3;
    std::fs::write(&config, format!("[debug]\nindex_max = \"{bound}\"\n"))
        .expect("the data folder is writable");
    folder.ingest_with(&[OsStr::new("--config"), config.as_os_str()]);
    let db = folder.index();
    let left = count(&db, "SELECT count(*) FROM requests");
    assert!(
        left > 0 && left < 2500,
        "some requests are deleted and some kept: {left} left"
    );
    assert!(
        bytes_in_use(&db) <= bound,
        "the pages in use fit under `index_max`"
    );
    let kept_from = 2500 - u64::try_from(left).expect("a count");
    assert_eq!(
        count(&db, "SELECT min(start_us) FROM requests"),
        i64::try_from(start(kept_from)).expect("the time fits"),
        "the requests kept are the newest"
    );
    for table in TRACED {
        assert_eq!(
            count(
                &db,
                &format!(
                    "SELECT count(*) FROM {table} \
                     WHERE trace IS NOT NULL AND trace NOT IN (SELECT trace FROM requests)"
                )
            ),
            0,
            "`{table}` keeps no row of a deleted request"
        );
    }
    drop(db);

    std::fs::write(&config, "[debug]\nindex_max = \"1K\"\n").expect("the data folder is writable");
    folder.ingest_with(&[OsStr::new("--config"), config.as_os_str()]);
    assert_eq!(
        count(&folder.index(), "SELECT count(*) FROM requests"),
        0,
        "a bound under the empty schema deletes every request"
    );
}

#[test]
fn the_deletion_window_removes_every_row_of_a_request() {
    let folder = Folder::new();
    let now = now_us();
    let old = "000000000000000000000000000000aa";
    let new = "000000000000000000000000000000bb";
    let mut lines = copy("4-1-0", old, now - 30 * DAY_US);
    let mut recent = copy("4-1-0", new, now);
    for line in &mut recent {
        line.seq += 100;
    }
    lines.extend(recent);
    folder.write("4-1-0.0.spool", &lines);
    folder.ingest();
    let db = folder.index();
    for table in TRACED {
        let left = |trace: &str| {
            count(
                &db,
                &format!("SELECT count(*) FROM {table} WHERE trace = '{trace}'"),
            )
        };
        assert_eq!(left(old), 0, "`{table}` keeps no row of the old request");
        assert!(left(new) > 0, "`{table}` keeps the new request's rows");
    }
    assert_eq!(
        count(&db, "SELECT count(*) FROM exceptions"),
        1,
        "only the new request's exception is left"
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM search WHERE rowid % 8 = 1 \
             AND rowid / 8 NOT IN (SELECT id FROM requests)"
        ),
        0,
        "a deleted request leaves no search row"
    );
    assert_eq!(
        count(&db, "SELECT count FROM exception_groups"),
        2,
        "the group still counts the deleted request"
    );
}

#[test]
fn a_file_that_is_one_enormous_line_is_one_unparseable_row() {
    let folder = Folder::new();
    let length = LINE_MAX + (1 << 20);
    std::fs::write(folder.spool().join("5-1-0.0.spool"), vec![b'x'; length])
        .expect("the spool is writable");
    folder.ingest();
    let db = folder.index();
    assert_eq!(count(&db, "SELECT count(*) FROM unparseable"), 1);
    let row: (String, i64, i64, String, i64, i64) = db
        .query_row(
            "SELECT source, offset, length, reason, length(excerpt), lossy FROM unparseable",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("one row");
    assert_eq!(
        row,
        (
            "5-1-0.0.spool".to_owned(),
            0,
            int(length),
            "the line is longer than 4 MiB".to_owned(),
            4096,
            0
        ),
        "the row has the whole length and the first 4 KiB"
    );
    assert_eq!(folder.files_left(), 0, "the file is deleted");
}

#[test]
fn a_bad_line_between_two_good_ones_is_kept_and_the_good_ones_are_inserted() {
    let folder = Folder::new();
    let mut bytes = std::fs::read(fixture("a-bad-line-between-two-good.spool"))
        .expect("the bad-line spool is committed");
    bytes.extend_from_slice(b"\xff\xfe\n");
    let starts = line_starts(&bytes);
    assert_eq!(
        starts.len(),
        4,
        "a dump, a line that is not a spool line, a log, and a line that is not UTF-8"
    );
    std::fs::write(folder.spool().join("6-1-0.0.spool"), &bytes).expect("the spool is writable");
    folder.ingest();
    let db = folder.index();
    assert_eq!(
        count(&db, "SELECT count(*) FROM dumps WHERE seq = 3"),
        1,
        "the dump before the bad line is in `dumps`"
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM logs WHERE seq = 4 AND message = 'stock is empty'"
        ),
        1,
        "the log after the bad line is in `logs`"
    );
    let bad: Vec<(i64, i64, String, i64)> = db
        .prepare(
            "SELECT offset, length, reason, lossy FROM unparseable \
             WHERE source = '6-1-0.0.spool' ORDER BY offset",
        )
        .expect("the unparseable table exists")
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("the query runs")
        .collect::<Result<_, _>>()
        .expect("each row reads");
    let not_json = int("this line is not a spool line".len());
    assert_eq!(
        bad.iter()
            .map(|(offset, length, _, lossy)| (*offset, *length, *lossy))
            .collect::<Vec<_>>(),
        vec![(starts[1], not_json, 0), (starts[3], 2, 1)],
        "each bad line is a row at its offset, and the one that is not UTF-8 is lossy: {bad:?}"
    );
    assert_eq!(bad[1].2, "the line is not UTF-8");
    assert_eq!(folder.files_left(), 0, "the file is deleted");
}

#[test]
fn an_unreadable_spool_file_is_quarantined_while_the_others_are_read() {
    let folder = Folder::new();
    // A directory with a spool file's name: opening or reading it fails on every OS.
    let bad = folder.spool().join("bad-writer.1.spool");
    std::fs::create_dir(&bad).expect("the spool is writable");
    let trace = "00000000000000000000000000000007";
    folder.write("7-1-0.0.spool", &copy("7-1-0", trace, now_us()));
    folder.ingest();
    let db = folder.index();
    let rows: Vec<(String, i64, i64, i64)> = db
        .prepare("SELECT path, at_us, retry_us, failures FROM quarantine")
        .expect("the quarantine table exists")
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("the query runs")
        .collect::<Result<_, _>>()
        .expect("each row reads");
    assert_eq!(rows.len(), 1, "one file is quarantined: {rows:?}");
    let (path, at_us, retry_us, failures) = &rows[0];
    assert_eq!(
        Path::new(path).file_name(),
        Some(OsStr::new("bad-writer.1.spool")),
        "the row names the directory"
    );
    assert_eq!(*failures, 1);
    assert_eq!(
        retry_us - at_us,
        1_000_000,
        "the first retry is a second later"
    );
    assert_eq!(
        count(
            &db,
            &format!("SELECT count(*) FROM requests WHERE trace = '{trace}'")
        ),
        1,
        "the good file's request is in the index"
    );
    assert!(
        !folder.spool().join("7-1-0.0.spool").exists(),
        "the good file is deleted"
    );
    assert_eq!(folder.files_left(), 1, "only the directory is left");

    std::fs::remove_dir(&bad).expect("the directory is removed");
    folder.ingest();
    assert_eq!(
        count(&db, "SELECT count(*) FROM quarantine"),
        0,
        "a file that is gone leaves quarantine"
    );
}

#[test]
fn a_tailed_log_file_is_read_line_by_line_and_never_written() {
    let folder = Folder::new();
    let path = folder.log_path("shop.log");
    let log = std::fs::read(fixture("a-json-log.log")).expect("the log file is committed");
    std::fs::write(&path, &log).expect("the data folder is writable");
    folder.tail(&path);
    let db = folder.index();
    let source = source_of(&path);
    let rows = tailed(&db, &source);
    assert_eq!(
        rows.iter().map(|row| row.0).collect::<Vec<_>>(),
        line_starts(&log),
        "one row per line, at the line's offset: {rows:?}"
    );
    assert_eq!(
        rows.iter().map(|row| row.3.as_deref()).collect::<Vec<_>>(),
        vec![None, Some(LOG_TRACE), None],
        "the record with a `trace_id` has its trace"
    );
    assert_eq!(
        rows[0].1,
        YEAR_2100_US + 1_000_000,
        "a row's time is its record's `ts`"
    );
    assert_eq!(
        std::fs::read(&path).expect("the log file reads"),
        log,
        "the tailed file is never written"
    );

    folder.tail(&path);
    assert_eq!(
        tailed(&db, &source).len(),
        3,
        "an unchanged file adds no row"
    );

    let fourth = record("Shop stopped");
    let (text, newline) = fourth.split_at(fourth.len() - 1);
    append(&path, text.as_bytes());
    folder.tail(&path);
    assert_eq!(
        tailed(&db, &source).len(),
        3,
        "a last line with no newline waits"
    );
    append(&path, newline.as_bytes());
    folder.tail(&path);
    let rows = tailed(&db, &source);
    assert_eq!(
        messages(&rows),
        vec![
            "Shop started on example.com",
            "stock is low",
            "payment failed",
            "Shop stopped"
        ],
        "the line is read once it has its newline"
    );
    assert_eq!(rows[3].0, int(log.len()));
    assert_eq!(
        count(&db, "SELECT offset FROM checkpoints"),
        int(log.len() + fourth.len()),
        "the checkpoint is the end of the file"
    );
}

#[test]
fn a_tailed_log_file_is_read_across_a_rotation_and_a_truncation() {
    let folder = Folder::new();
    let path = folder.log_path("shop.log");
    let log = std::fs::read(fixture("a-json-log.log")).expect("the log file is committed");
    std::fs::write(&path, &log).expect("the data folder is writable");
    folder.tail(&path);
    let db = folder.index();
    let source = source_of(&path);
    assert_eq!(tailed(&db, &source).len(), 3);

    // A line written just before the rotation, then the rotation to `shop.log.1`.
    let last = record("order 8 shipped");
    append(&path, last.as_bytes());
    let mut rotated = path.clone().into_os_string();
    rotated.push(".1");
    let rotated = PathBuf::from(rotated);
    std::fs::rename(&path, &rotated).expect("the log file is renamed");
    let first = record("Shop restarted");
    let second = record("order 9 shipped");
    std::fs::write(&path, format!("{first}{second}")).expect("the data folder is writable");
    folder.tail(&path);
    let old = tailed(&db, &source_of(&rotated));
    assert_eq!(
        old.iter()
            .map(|row| (row.0, row.2.as_str()))
            .collect::<Vec<_>>(),
        vec![(int(log.len()), "order 8 shipped")],
        "the rest of the rotated file is read from the checkpoint, under its own name"
    );
    let rows = tailed(&db, &source);
    assert_eq!(
        rows[3..]
            .iter()
            .map(|row| (row.0, row.2.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "Shop restarted"), (int(first.len()), "order 9 shipped")],
        "the new file is read from its start: {rows:?}"
    );
    let mut expected = log.clone();
    expected.extend_from_slice(last.as_bytes());
    assert_eq!(
        std::fs::read(&rotated).expect("the rotated file reads"),
        expected,
        "the rotated file is never written"
    );

    // The file truncated in place, with a shorter content and a new first line.
    std::fs::File::create(&path)
        .expect("the log file truncates")
        .write_all(record("Shop up").as_bytes())
        .expect("the log file is writable");
    folder.tail(&path);
    let rows = tailed(&db, &source);
    assert_eq!(rows.len(), 6, "one row more: {rows:?}");
    assert_eq!(
        (rows[5].0, rows[5].2.as_str()),
        (0, "Shop up"),
        "the truncated file is read from its start"
    );
}
