//! Many SQLite writers on one file at once, through the lock wait a statement with no deadline of
//! its own is given (`rule:core-classes/db-statement-members`), and the journal mode a file opens in.
//!
//! This is the attack a `.nvs` case cannot write: the harness runs one request at a time, and
//! the failure only shows when many connections meet the file's write lock together. Every
//! connection files its wait through `Connection::set_statement_deadline`, as `Core\Db`'s
//! statement path does with the request's remaining `wall_time`.

use std::path::Path;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use nvs_config::tree::Database;
use nvs_db::{Connection, DbErrorKind, ServerError, SqliteTarget, SqliteValue};

/// How many connections insert at once.
const WRITERS: usize = 50;

/// [`WRITERS`], as the count SQLite gives back.
const WRITTEN: i64 = 50;

/// A `sqlite` block on `path`, with `journal_mode` as written.
fn block(path: &Path, journal_mode: Option<&str>) -> Database {
    Database {
        driver: Some(String::from("sqlite")),
        path: Some(path.to_string_lossy().into_owned()),
        journal_mode: journal_mode.map(String::from),
        ..Database::default()
    }
}

/// A connection to `block`'s file.
fn connect(block: &Database) -> Connection {
    let target = SqliteTarget::resolve(block).expect("the block resolves to a path");
    Connection::Sqlite(nvs_db::sqlite::open(&target).expect("the file opens"))
}

/// Runs `sql`, and gives its rows or SQLite's error.
fn run(
    conn: &Connection,
    sql: &str,
    params: Vec<SqliteValue>,
) -> std::io::Result<Vec<Vec<SqliteValue>>> {
    let Connection::Sqlite(sqlite) = conn else {
        unreachable!("every connection here is SQLite");
    };
    let mut answered = sqlite.query(sql, params)?;
    let mut all = Vec::new();
    while let Some(row) = answered.next_row() {
        all.push(row);
    }
    Ok(all)
}

/// The file's journal mode, as SQLite reports it.
fn journal_mode(conn: &Connection) -> String {
    match run(conn, "PRAGMA journal_mode", Vec::new())
        .expect("the pragma reads")
        .as_slice()
    {
        [row] => match row.as_slice() {
            [SqliteValue::Text(mode)] => mode.to_ascii_lowercase(),
            other => panic!("`PRAGMA journal_mode` gave {other:?}"),
        },
        other => panic!("`PRAGMA journal_mode` gave {} rows", other.len()),
    }
}

/// The smallest amount this thread's CPU clock moves by, measured on this
/// machine. Some platforms add CPU time in whole scheduler ticks, so a reading
/// can be up to one step behind the time the thread really spent.
///
/// The thread spins while it reads, so its clock always moves. The wall-clock
/// bound only fails a clock that never moves.
fn clock_step(clock: &nvs_host::cpuclock::ThreadClock) -> Duration {
    let give_up_at = Instant::now() + Duration::from_secs(10);
    let read = || {
        assert!(
            Instant::now() < give_up_at,
            "this thread's CPU clock did not move in 10 s"
        );
        clock.burned().expect("this thread's clock reads")
    };
    let first = read();
    let mut from = read();
    while from == first {
        from = read();
    }
    let mut to = read();
    while to == from {
        to = read();
    }
    to - from
}

#[test]
fn fifty_writers_at_once_all_insert() {
    let scratch = nvs_repo::scratch("sqlite-contention");
    let block = Arc::new(block(&scratch.join("app.sqlite"), None));
    run(
        &connect(&block),
        "create table notes (id integer primary key, body text not null)",
        Vec::new(),
    )
    .expect("the table is created");

    let start = Arc::new(Barrier::new(WRITERS));
    let writers: Vec<_> = (0..WRITERS)
        .map(|n| {
            let block = Arc::clone(&block);
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                let mut conn = connect(&block);
                conn.set_statement_deadline(
                    None,
                    Some(Instant::now() + Duration::from_secs(30)),
                    None,
                )
                .expect("the lock wait files");
                start.wait();
                run(
                    &conn,
                    "insert into notes (body) values (?)",
                    vec![SqliteValue::Text(format!("note {n}"))],
                )
                .map(drop)
            })
        })
        .collect();

    let failed: Vec<String> = writers
        .into_iter()
        .filter_map(|writer| writer.join().expect("a writer panicked").err())
        .map(|refused| refused.to_string())
        .collect();
    assert!(
        failed.is_empty(),
        "{} of {WRITERS} inserts failed: {failed:?}",
        failed.len()
    );

    let counted =
        run(&connect(&block), "select count(*) from notes", Vec::new()).expect("the count reads");
    assert_eq!(counted, vec![vec![SqliteValue::Int(WRITTEN)]]);
}

#[test]
fn a_lock_held_past_the_wait_fails_as_deadlock() {
    let scratch = nvs_repo::scratch("sqlite-lock-held");
    let block = block(&scratch.join("app.sqlite"), None);
    let holder = connect(&block);
    run(&holder, "create table notes (body text)", Vec::new()).expect("the table is created");
    let Connection::Sqlite(holding) = &holder else {
        unreachable!("the connection was opened as SQLite");
    };
    holding
        .begin_immediate()
        .expect("the holder takes the write lock");

    let mut writer = connect(&block);
    let waited = Duration::from_millis(200);
    let started = Instant::now();
    writer
        .set_statement_deadline(None, Some(started + waited), None)
        .expect("the lock wait files");
    let refused = run(&writer, "insert into notes values ('late')", Vec::new())
        .expect_err("the lock is still held when the wait ends");

    let server = ServerError::of(&refused).expect("a busy lock carries a kind");
    assert_eq!(server.kind, DbErrorKind::Deadlock);
    assert!(
        started.elapsed() >= waited / 2,
        "the writer gave up after {:?}, before its wait of {waited:?}",
        started.elapsed()
    );
    holding.commit().expect("the holder commits");
}

#[test]
fn an_endless_statement_stops_near_the_cpu_time_its_request_has_left() {
    // Where the platform has no thread clock there is no CPU ceiling to stop
    // at, and this statement would never end.
    let Some(clock) = nvs_host::cpuclock::ThreadClock::current() else {
        return;
    };
    let scratch = nvs_repo::scratch("sqlite-cpu-time");
    let mut conn = connect(&block(&scratch.join("app.sqlite"), None));

    // What this machine needs, in this run, for the work a stop may run past
    // its budget: a statement's start and end, and more than one interrupt
    // check's worth of steps. A thousand rows are many thousand steps. The
    // wall clock is read, and a thread's CPU time is never more than its wall time.
    let reference = Instant::now();
    run(
        &conn,
        "with recursive c(x) as (select 1 union all select x + 1 from c where x < 1000) \
         select count(*) from c",
        Vec::new(),
    )
    .expect("a bounded statement runs to its end");
    let reference = reference.elapsed();
    let step = clock_step(&clock);

    let budget = Duration::from_millis(300);
    let started = Instant::now();
    conn.set_statement_deadline(None, Some(started + Duration::from_secs(30)), Some(budget))
        .expect("the bounds file");
    // The statement runs on this thread when no reactor is installed, so this
    // thread's clock is the one its budget is checked against.
    let burned_before = clock.burned().expect("this thread's clock reads");
    let refused = run(
        &conn,
        "with recursive c(x) as (select 1 union all select x + 1 from c) \
         select count(*) from c",
        Vec::new(),
    )
    .expect_err("a statement with no end stops at its CPU budget");
    let charged = clock
        .burned()
        .expect("this thread's clock reads")
        .saturating_sub(burned_before);

    let server = ServerError::of(&refused).expect("an interrupt carries a kind");
    assert_eq!(server.kind, DbErrorKind::Timeout);
    let took = started.elapsed();
    assert!(
        took >= budget / 2,
        "the statement stopped after {took:?}, before its budget of {budget:?}"
    );
    assert!(
        charged >= budget,
        "the statement was charged {charged:?} of CPU time, less than its budget of {budget:?}"
    );
    // Past its budget, the statement runs to the next interrupt check and then
    // ends. Each of the three clock readings around that can lag by one step.
    let allowed = budget + reference + step * 3;
    assert!(
        charged <= allowed,
        "the statement was charged {charged:?} of CPU time on a budget of {budget:?}; the \
         budget, a bounded statement's {reference:?} and three clock steps of {step:?} \
         allow {allowed:?}"
    );
    // The bound is the statement's: the next one runs to its end.
    conn.set_statement_deadline(None, Some(Instant::now() + Duration::from_secs(30)), None)
        .expect("the bounds lift");
    assert_eq!(
        run(&conn, "select 1", Vec::new()).expect("the connection is idle again"),
        vec![vec![SqliteValue::Int(1)]]
    );
}

#[test]
fn a_new_file_opens_in_wal_unless_its_block_says_delete() {
    let scratch = nvs_repo::scratch("sqlite-journal");

    let wal_path = scratch.join("wal.sqlite");
    let wal = connect(&block(&wal_path, None));
    run(&wal, "create table notes (body text)", Vec::new()).expect("the table is created");
    assert_eq!(journal_mode(&wal), "wal");
    assert!(
        scratch.join("wal.sqlite-wal").exists(),
        "a WAL file keeps a `-wal` file beside it"
    );

    let delete_path = scratch.join("delete.sqlite");
    let delete = connect(&block(&delete_path, Some("delete")));
    run(&delete, "create table notes (body text)", Vec::new()).expect("the table is created");
    run(&delete, "insert into notes values ('kept')", Vec::new()).expect("the row is written");
    assert_eq!(journal_mode(&delete), "delete");
    assert!(
        !scratch.join("delete.sqlite-wal").exists(),
        "`journal_mode = \"delete\"` left a `-wal` file"
    );
}
