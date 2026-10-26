//! `nvs debug --once` over the committed sample spool
//! (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`,
//! `rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time`).
//!
//! Each case writes copies of `fixtures/debug/one-of-each.spool` into a fresh data folder's spool,
//! with each copy's writer, trace id and times rewritten, runs one pass, and reads the index back.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use nvs_runtime::spool::{Line, Record};
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

fn sample() -> Vec<Line> {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "debug",
        "one-of-each.spool",
    ]
    .iter()
    .collect();
    std::fs::read_to_string(path)
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
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .current_dir(&*self.data)
            .arg("--data")
            .arg(&*self.data)
            .arg("--no-init")
            .arg("debug")
            .arg("--once")
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        assert!(
            out.status.success(),
            "`nvs debug --once` succeeds: {}",
            String::from_utf8_lossy(&out.stderr)
        );
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
