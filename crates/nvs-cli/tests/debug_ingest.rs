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

use nvs_runtime::spool::{LINE_MAX, Line, Record, RunOf, RunOutcome};
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
            Record::Run(run) => run.start_us = start_us,
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
        "copies",
        "dropped",
        "mails",
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

/// A request's memory, handler and sizes reach its row, and a line written before they existed
/// reads with each of them null.
#[test]
fn a_requests_memory_handler_and_sizes_reach_its_row_and_an_older_line_reads() {
    type Facts = (
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<i64>,
    );
    let folder = Folder::new();
    let start = now_us();
    let with = copy("9-1-0", "00000000000000000000000000000009", start);
    let mut without = copy("9-2-0", "0000000000000000000000000000000a", start + 10_000);
    for line in &mut without {
        if let Record::Request(request) = &mut line.record {
            request.memory_peak = None;
            request.memory_limit = None;
            request.handler = None;
            request.bytes_in = None;
            request.bytes_out = None;
        }
    }
    folder.write("9-1-0.0.spool", &with);
    folder.write("9-2-0.0.spool", &without);
    folder.ingest();
    let db = folder.index();
    let facts = |trace: &str| -> Facts {
        db.query_row(
            "SELECT memory_peak, memory_limit, handler, bytes_in, bytes_out FROM requests \
             WHERE trace = ?1",
            [trace],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("the request has its row")
    };
    assert_eq!(
        facts("00000000000000000000000000000009"),
        (
            Some(1_048_576),
            Some(134_217_728),
            Some("Shop\\Orders::create".to_owned()),
            Some(34),
            Some(3),
        ),
        "the sample's figures round-trip"
    );
    assert_eq!(
        facts("0000000000000000000000000000000a"),
        (None, None, None, None, None),
        "a line without them reads"
    );
}

/// The sample's `cache` event line, rewritten as a `kind` event with `outcome`, as `writer`'s
/// line `seq` under `trace`.
fn event_with(
    lines: &[Line],
    seq: u64,
    kind: nvs_runtime::spool::EventKind,
    outcome: nvs_runtime::spool::Outcome,
) -> Line {
    let mut line = lines
        .iter()
        .find(|line| {
            matches!(&line.record, Record::Event(event)
                if event.event == nvs_runtime::spool::EventKind::Cache)
        })
        .expect("the sample has a cache event")
        .clone();
    line.seq = seq;
    if let Record::Event(event) = &mut line.record {
        event.event = kind;
        event.label = format!("{kind:?} member");
        event.outcome = outcome;
    }
    line
}

/// `rule:testing/the-debug-stream-reaches-the-service-through-the-disk`'s event outcome: a
/// cache, HTTP, file and process event's outcome reaches its row's `outcome` as JSON, and an
/// event with none has a null `outcome`.
#[test]
fn each_kinds_outcome_reaches_its_events_row_and_an_event_without_one_has_none() {
    use nvs_runtime::spool::{EventKind, Outcome};
    let folder = Folder::new();
    let trace = "0000000000000000000000000000000d";
    let mut lines = copy("13-1-0", trace, now_us());
    let next = u64::try_from(lines.len()).expect("few lines") + 100;
    lines.extend([
        event_with(
            &lines,
            next,
            EventKind::Http,
            Outcome {
                http_status: Some(503),
                bytes_in: Some(17),
                bytes_out: Some(4),
                ..Outcome::default()
            },
        ),
        event_with(
            &lines,
            next + 1,
            EventKind::File,
            Outcome {
                bytes: Some(2_048),
                ..Outcome::default()
            },
        ),
        event_with(
            &lines,
            next + 2,
            EventKind::Spawn,
            Outcome {
                command: Some("convert".to_owned()),
                exit: Some(2),
                ..Outcome::default()
            },
        ),
        event_with(&lines, next + 3, EventKind::Session, Outcome::default()),
    ]);
    folder.write("13-1-0.0.spool", &lines);
    folder.ingest();
    let db = folder.index();
    let outcome = |kind: &str| -> Option<serde_json::Value> {
        db.query_row(
            "SELECT outcome FROM events WHERE trace = ?1 AND kind = ?2",
            [trace, kind],
            |row| row.get::<_, Option<String>>(0),
        )
        .expect("the event has its row")
        .map(|text| serde_json::from_str(&text).expect("the outcome is JSON"))
    };
    assert_eq!(
        outcome("cache"),
        Some(serde_json::json!({"hit": true, "tier": "memory"}))
    );
    assert_eq!(
        outcome("http"),
        Some(serde_json::json!({"http_status": 503, "bytes_in": 17, "bytes_out": 4}))
    );
    assert_eq!(outcome("file"), Some(serde_json::json!({"bytes": 2048})));
    assert_eq!(
        outcome("spawn"),
        Some(serde_json::json!({"command": "convert", "exit": 2}))
    );
    assert_eq!(
        outcome("session"),
        None,
        "an event without an outcome has none"
    );
}

/// `rule:testing/a-recorded-exception-keeps-its-frames-and-never-a-value`: an exception's frames
/// and chain reach its row, a line without them reads with none, and `nvs debug to-test` prints
/// the same test for both.
#[test]
fn an_exceptions_frames_and_chain_reach_its_row_and_to_test_is_unchanged() {
    let folder = Folder::new();
    let start = now_us();
    let (traced, bare) = (
        "0000000000000000000000000000000b",
        "0000000000000000000000000000000c",
    );
    let with = copy("11-1-0", traced, start);
    let mut without = copy("11-2-0", bare, start + 10_000);
    for line in &mut without {
        if let Record::Request(request) = &mut line.record
            && let Some(exception) = &mut request.exception
        {
            exception.stack = nvs_runtime::spool::Stack::default();
        }
    }
    folder.write("11-1-0.0.spool", &with);
    folder.write("11-2-0.0.spool", &without);
    folder.ingest();
    let db = folder.index();
    let stack = |trace: &str| -> (Option<String>, bool, Option<String>, bool) {
        db.query_row(
            "SELECT e.frames, e.frames_cut, e.previous, e.previous_cut FROM exceptions e \
             JOIN requests r ON r.id = e.request WHERE r.trace = ?1",
            [trace],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("the exception has its row")
    };
    let (frames, frames_cut, previous, previous_cut) = stack(traced);
    let frames = frames.expect("the sample's frames are kept");
    assert!(frames.contains("Shop\\\\Stock::take"), "{frames}");
    assert!(
        previous
            .expect("the sample's chain is kept")
            .contains("no row for item 7")
    );
    assert!(!frames_cut && !previous_cut);
    assert_eq!(
        stack(bare),
        (None, false, None, false),
        "a line without them reads"
    );

    let printed = |trace: &str| {
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .current_dir(&*folder.data)
            .arg("--data")
            .arg(&*folder.data)
            .arg("--no-init")
            .args(["debug", "to-test", trace])
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        assert!(
            out.status.success(),
            "`nvs debug to-test` succeeds: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).replace(trace, "TRACE")
    };
    assert_eq!(
        printed(traced),
        printed(bare),
        "the frames do not change the test"
    );
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

/// A `run` line of each kind makes its `runs` row, and a failed run's exception is a row that
/// names the run and no request.
#[test]
fn each_kind_of_run_reaches_its_row_and_a_failed_run_keeps_its_exception() {
    let folder = Folder::new();
    let start = now_us();
    let mut lines = Vec::new();
    for (n, of) in [RunOf::Schedule, RunOf::Queue, RunOf::Command]
        .into_iter()
        .enumerate()
    {
        let n = u64::try_from(n).expect("few kinds");
        let mut copied = copy("5-1-0", &format!("{:032x}", n + 1), start + n * 10_000);
        copied.retain(|line| matches!(line.record, Record::Run(_)));
        for line in &mut copied {
            line.seq += n * 100;
            if let Record::Run(run) = &mut line.record {
                run.of = of;
                run.name = format!("{} job", of.as_str());
                if of != RunOf::Queue {
                    run.outcome = RunOutcome::Ok;
                    run.exception = None;
                }
            }
        }
        lines.extend(copied);
    }
    folder.write("5-1-0.0.spool", &lines);
    folder.ingest();
    let db = folder.index();
    // `of`, `name`, `outcome`, the memory peak and limit, and whether it has an exception group.
    type RunRow = (String, String, String, Option<i64>, Option<i64>, bool);
    let rows: Vec<RunRow> = db
        .prepare(
            "SELECT of, name, outcome, memory_peak, memory_limit, exception_group IS NOT NULL \
             FROM runs ORDER BY start_us",
        )
        .expect("the query prepares")
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .expect("the query runs")
        .collect::<Result<_, _>>()
        .expect("each row reads");
    let row = |of: &str, outcome: &str, failed: bool| {
        (
            of.to_owned(),
            format!("{of} job"),
            outcome.to_owned(),
            Some(524_288),
            Some(67_108_864),
            failed,
        )
    };
    assert_eq!(
        rows,
        vec![
            row("schedule", "ok", false),
            row("queue", "failed", true),
            row("command", "ok", false),
        ]
    );
    let (request, class, frames): (Option<i64>, String, Option<String>) = db
        .query_row(
            "SELECT e.request, e.class, e.frames FROM exceptions e \
             JOIN runs r ON r.id = e.run WHERE r.of = 'queue'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("the failed run's exception has its row");
    assert_eq!(request, None, "a run's exception names no request");
    assert_eq!(class, "Shop\\MailDown");
    assert!(
        frames.is_some_and(|frames| frames.contains("jobs/receipt.nvs")),
        "its frames are kept"
    );
}

#[test]
fn ten_throws_from_one_site_make_one_group_of_ten() {
    let folder = Folder::new();
    let start = now_us();
    let mut lines = Vec::new();
    for n in 0..10u64 {
        let mut copied = copy("3-1-0", &format!("{n:032x}"), start + n * 10_000);
        // The sample's run throws from a site of its own, so it is left out here.
        copied.retain(|line| !matches!(line.record, Record::Run(_)));
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

/// A `copies` line of `trace`, written by `writer` as its line `seq`, with one count per site.
fn copies_line(writer: &str, seq: u64, trace: &str, sites: &[(u32, &str, u64, u64)]) -> Line {
    Line {
        writer: writer.to_owned(),
        seq,
        trace: Some(trace.to_owned()),
        at_us: now_us(),
        record: Record::Copies(nvs_runtime::spool::Copies {
            span: Some("00f067aa0ba902b7".to_owned()),
            sites: sites
                .iter()
                .map(
                    |&(line, reason, copies, units)| nvs_runtime::spool::CopyCount {
                        file: "shop.nvs".to_owned(),
                        line,
                        hash: "ab".repeat(32),
                        reason: reason.to_owned(),
                        copies,
                        units,
                    },
                )
                .collect(),
            elided: false,
        }),
    }
}

#[test]
fn a_recorded_request_counts_its_copies_by_site() {
    let folder = Folder::new();
    let first = "00000000000000000000000000000011";
    let second = "00000000000000000000000000000012";
    let lines = [
        copies_line(
            "4-1-0",
            0,
            first,
            &[(7, "still_used", 3, 300), (12, "slot_kept", 1, 40)],
        ),
        copies_line("4-1-0", 1, second, &[(7, "still_used", 1, 100)]),
    ];
    folder.write("4-1-0.0.spool", &lines);
    folder.ingest();
    let db = folder.index();
    let rows: Vec<(String, i64, String, i64, i64)> = db
        .prepare("SELECT trace, line, reason, copies, units FROM copies ORDER BY trace, part")
        .expect("the copies table exists")
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .expect("the query runs")
        .collect::<Result<_, _>>()
        .expect("each row reads");
    assert_eq!(
        rows,
        vec![
            (first.to_owned(), 7, "still_used".to_owned(), 3, 300),
            (first.to_owned(), 12, "slot_kept".to_owned(), 1, 40),
            (second.to_owned(), 7, "still_used".to_owned(), 1, 100),
        ],
        "each site of each request is its own row"
    );

    // The same file again adds no row.
    folder.write("4-1-0.0.spool", &lines);
    folder.ingest();
    assert_eq!(count(&db, "SELECT count(*) FROM copies"), 3);
    assert_eq!(
        count(
            &db,
            &format!(
                "SELECT sum(copies) FROM copies WHERE hash = '{}' AND line = 7",
                "ab".repeat(32)
            )
        ),
        4,
        "the site index finds both requests' copies at one line"
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

/// `[debug] index_max` bounds the rows that no request or run names too, such as the events of a
/// command run that is still going. One time order covers them and the requests, so past the
/// bound the oldest rows go first and the newer requests are kept. A bound under the empty schema
/// deletes every row, whatever owns it.
#[test]
fn index_max_deletes_the_oldest_rows_whether_a_request_names_them_or_not() {
    const UNOWNED: u64 = 20_000;
    let folder = Folder::new();
    let now = now_us();
    let start = |n: u64| now - 1_000_000_000 + n * 1_000_000;
    let mut lines = Vec::new();
    for n in 0..500 {
        let mut one = copy("6-1-0", &format!("{n:032x}"), start(n));
        for line in &mut one {
            line.seq += n * 100;
        }
        lines.extend(one);
    }
    folder.write("6-1-0.0.spool", &lines);
    folder.ingest();
    let requests_alone = bytes_in_use(&folder.index());

    let events: Vec<Line> = sample()
        .into_iter()
        .filter(|line| matches!(line.record, Record::Event(_)))
        .collect();
    let unowned_at = |n: u64| now - 3_000_000_000 + n * 10_000;
    let lines: Vec<Line> = (0..UNOWNED)
        .map(|n| {
            let mut line = events[usize::try_from(n).expect("a few") % events.len()].clone();
            line.writer = "7-1-0".to_owned();
            line.seq = n;
            line.trace = Some("dd".repeat(16));
            line.at_us = unowned_at(n);
            if let Record::Event(event) = &mut line.record {
                event.start_us = line.at_us;
                event.end_us = line.at_us + 300;
            }
            line
        })
        .collect();
    folder.write("7-1-0.0.spool", &lines);
    folder.ingest();
    let full = bytes_in_use(&folder.index());
    let unowned = "SELECT count(*) FROM events WHERE trace = 'dddddddddddddddddddddddddddddddd'";
    assert_eq!(count(&folder.index(), unowned), int(lines.len()));

    let config = folder.data.join("small.toml");
    let bound = (requests_alone + full) / 2;
    std::fs::write(&config, format!("[debug]\nindex_max = \"{bound}\"\n"))
        .expect("the data folder is writable");
    folder.ingest_with(&[OsStr::new("--config"), config.as_os_str()]);
    let db = folder.index();
    assert!(
        bytes_in_use(&db) <= bound,
        "the pages in use fit under `index_max`"
    );
    assert_eq!(
        count(&db, "SELECT count(*) FROM requests"),
        500,
        "the requests are newer than every unowned row, so all of them are kept"
    );
    let left = count(&db, unowned);
    assert!(
        left > 0 && left < int(lines.len()),
        "some unowned rows are deleted and some kept: {left} left"
    );
    assert_eq!(
        count(
            &db,
            &format!(
                "SELECT min(at_us) FROM events WHERE trace = '{}'",
                "dd".repeat(16)
            )
        ),
        i64::try_from(unowned_at(UNOWNED - u64::try_from(left).expect("a count")))
            .expect("the time fits"),
        "the unowned rows kept are the newest"
    );
    drop(db);

    std::fs::write(&config, "[debug]\nindex_max = \"1K\"\n").expect("the data folder is writable");
    folder.ingest_with(&[OsStr::new("--config"), config.as_os_str()]);
    let db = folder.index();
    assert_eq!(count(&db, "SELECT count(*) FROM requests"), 0);
    assert_eq!(
        count(&db, "SELECT count(*) FROM events"),
        0,
        "a bound under the empty schema deletes the unowned rows too"
    );
}

/// The sample's run line alone, under `trace`, from `start_us` for `duration_us`, written at its
/// end by `writer` as line `seq`.
fn run_line(writer: &str, seq: u64, trace: &str, start_us: u64, duration_us: u64) -> Line {
    let mut line = sample()
        .into_iter()
        .find(|line| matches!(line.record, Record::Run(_)))
        .expect("the sample has a run line");
    line.writer = writer.to_owned();
    line.seq = seq;
    line.trace = Some(trace.to_owned());
    line.at_us = start_us + duration_us;
    if let Record::Run(run) = &mut line.record {
        run.start_us = start_us;
        run.duration_us = duration_us;
    }
    line
}

/// `count` of the sample's events under `trace`, written by `writer` from line `seq`, the `n`th at
/// `at(n)`.
fn events_at(
    writer: &str,
    seq: u64,
    trace: &str,
    count: u64,
    at: impl Fn(u64) -> u64,
) -> Vec<Line> {
    let events: Vec<Line> = sample()
        .into_iter()
        .filter(|line| matches!(line.record, Record::Event(_)))
        .collect();
    (0..count)
        .map(|n| {
            let mut line = events[usize::try_from(n).expect("a few") % events.len()].clone();
            line.writer = writer.to_owned();
            line.seq = seq + n;
            line.trace = Some(trace.to_owned());
            line.at_us = at(n);
            if let Record::Event(event) = &mut line.record {
                event.start_us = line.at_us;
                event.end_us = line.at_us + 300;
            }
            line
        })
        .collect()
}

/// `[debug] index_age` deletes the rows of a finished run that are older than it by their own
/// time, and keeps the run's newer rows and its run row while its end is within it. The newest
/// time it deleted is the index's cut.
#[test]
fn index_age_deletes_a_finished_runs_old_rows_and_keeps_its_new_ones() {
    let folder = Folder::new();
    let now = now_us();
    let trace = "ee".repeat(16);
    let start = now - 10 * DAY_US;
    let at = |n: u64| {
        if n < 5 {
            start + n * 1_000_000
        } else {
            now - 2 * DAY_US + n * 1_000_000
        }
    };
    let mut lines = events_at("8-1-0", 0, &trace, 10, at);
    lines.push(run_line("8-1-0", 100, &trace, start, 9 * DAY_US));
    folder.write("8-1-0.0.spool", &lines);
    folder.ingest();
    let db = folder.index();
    let times: Vec<i64> = db
        .prepare("SELECT at_us FROM events WHERE trace = ?1 ORDER BY at_us")
        .expect("the query prepares")
        .query_map([&trace], |row| row.get(0))
        .expect("the query runs")
        .collect::<Result<_, _>>()
        .expect("each row reads");
    let kept: Vec<i64> = (5..10)
        .map(|n| i64::try_from(at(n)).expect("the time fits"))
        .collect();
    assert_eq!(times, kept, "the run's rows past index_age are deleted");
    assert_eq!(
        count(&db, "SELECT count(*) FROM runs"),
        1,
        "the run row stays while its end is within index_age"
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM exceptions WHERE run IN (SELECT id FROM runs)"
        ),
        1,
        "the run keeps its exception"
    );
    assert_eq!(
        count(&db, "SELECT value FROM meta WHERE key = 'cut_us'"),
        i64::try_from(at(4)).expect("the time fits"),
        "the cut is the newest time deleted"
    );
}

/// `[debug] index_max` deletes a long run's rows by their own time. Its oldest rows are older than
/// every request, so they go first, and every request and the run row stay. The run row is timed
/// by its end, which is newer than every request.
#[test]
fn index_max_deletes_a_long_runs_oldest_rows_before_a_newer_request() {
    const ROWS: u64 = 20_000;
    let folder = Folder::new();
    let now = now_us();
    let start = |n: u64| now - 1_000_000_000 + n * 1_000_000;
    let mut lines = Vec::new();
    for n in 0..500 {
        let mut one = copy("6-1-0", &format!("{n:032x}"), start(n));
        for line in &mut one {
            line.seq += n * 100;
        }
        lines.extend(one);
    }
    folder.write("6-1-0.0.spool", &lines);
    folder.ingest();
    let requests_alone = bytes_in_use(&folder.index());

    let trace = "ff".repeat(16);
    let run_start = now - 3_000_000_000;
    let at = |n: u64| run_start + 1_000 + n * 10_000;
    let mut lines = events_at("7-1-0", 0, &trace, ROWS, at);
    lines.push(run_line("7-1-0", ROWS, &trace, run_start, 2_990_000_000));
    folder.write("7-1-0.0.spool", &lines);
    folder.ingest();
    let full = bytes_in_use(&folder.index());
    let rows = format!("SELECT count(*) FROM events WHERE trace = '{trace}'");
    assert_eq!(
        count(&folder.index(), &rows),
        int(usize::try_from(ROWS).expect("a few"))
    );

    let config = folder.data.join("small.toml");
    let bound = (requests_alone + full) / 2;
    std::fs::write(&config, format!("[debug]\nindex_max = \"{bound}\"\n"))
        .expect("the data folder is writable");
    folder.ingest_with(&[OsStr::new("--config"), config.as_os_str()]);
    let db = folder.index();
    assert!(
        bytes_in_use(&db) <= bound,
        "the pages in use fit under `index_max`"
    );
    assert_eq!(
        count(&db, "SELECT count(*) FROM requests"),
        500,
        "the requests are newer than the run's oldest rows, so all of them are kept"
    );
    assert_eq!(
        count(
            &db,
            &format!("SELECT count(*) FROM runs WHERE trace = '{trace}'")
        ),
        1,
        "the run row's end is newer than every request, so it is kept"
    );
    let left = count(&db, &rows);
    assert!(
        left > 0 && left < int(usize::try_from(ROWS).expect("a few")),
        "some of the run's rows are deleted and some kept: {left} left"
    );
    assert_eq!(
        count(
            &db,
            &format!("SELECT min(at_us) FROM events WHERE trace = '{trace}'")
        ),
        i64::try_from(at(ROWS - u64::try_from(left).expect("a count"))).expect("the time fits"),
        "the run's rows kept are its newest"
    );
}

/// `[debug] index_age` deletes a run row once its end is past it, with its exception, and keeps a
/// run that started as early while its end is within it.
#[test]
fn a_run_row_goes_once_its_end_is_past_index_age() {
    let folder = Folder::new();
    let now = now_us();
    let ended = format!("{:032x}", 0xa1);
    let going = format!("{:032x}", 0xa2);
    let start = now - 10 * DAY_US;
    let lines = vec![
        run_line("9-1-0", 0, &ended, start, 2 * DAY_US),
        run_line("9-1-0", 1, &going, start, 4 * DAY_US),
    ];
    folder.write("9-1-0.0.spool", &lines);
    folder.ingest();
    let db = folder.index();
    let traces: Vec<String> = db
        .prepare("SELECT trace FROM runs")
        .expect("the query prepares")
        .query_map([], |row| row.get(0))
        .expect("the query runs")
        .collect::<Result<_, _>>()
        .expect("each row reads");
    assert_eq!(
        traces,
        std::slice::from_ref(&going),
        "only the run that ended within index_age is kept"
    );
    let exceptions = |trace: &str| {
        count(
            &db,
            &format!("SELECT count(*) FROM exceptions WHERE trace = '{trace}'"),
        )
    };
    assert_eq!(
        exceptions(&ended),
        0,
        "the deleted run's exception goes with it"
    );
    assert_eq!(exceptions(&going), 1, "the kept run keeps its exception");
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
        count(
            &db,
            &format!("SELECT count(*) FROM exceptions WHERE trace <> '{new}'")
        ),
        0,
        "only the new trace's exceptions are left"
    );
    assert_eq!(
        count(&db, "SELECT count(*) FROM runs"),
        1,
        "the old trace's run is forgotten with it"
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
