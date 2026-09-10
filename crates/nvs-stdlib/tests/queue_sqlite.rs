//! `rule:concurrency/claiming-is-one-statement`'s claim on SQLite, and the
//! statements the worker runs beside it, against a real engine by two
//! connections at once.
//!
//! **This is the one queue suite that needs no server, and that is why it is
//! not `queue.rs` beside it.** That file's cases assert nothing at
//! all unless `NVS_DB_MATRIX_DRIVER` names a backend `python
//! tools/db-matrix.py` has started a container for, because a socket is the
//! only way to reach the four drivers that have one. SQLite is a file, so every
//! case here runs on every machine and under `python tools/verify.py` — the
//! statements below are *executed* on the default legs rather than read, which
//! is the one thing this backend can offer that the others cannot.
//!
//! Why the cases live in `nvs-stdlib` at all is `queue.rs`'s own module doc,
//! unchanged and for the same reason: the statement roster is
//! [`nvs_stdlib::queue`]'s and the thing that runs a statement is `nvs-db`'s
//! connection, and `rule:core-classes/db-crate-boundary` fixes which way that
//! edge points.
//!
//! **Every case runs two connections over one database.** A claim proven on one
//! connection is a claim whose whole failure mode was never exercised: the
//! deferred transaction `nvs_db::SqliteConn::begin` opens takes no lock until
//! its first statement, so a claim built on one reads a row and then asks to
//! upgrade a shared lock — which SQLite refuses without honouring the busy
//! timeout, and which nothing running alone can ever observe. That is what
//! `SqliteConn::begin_immediate` exists for, and what these cases hold it to.
//!
//! Every case owns a database name, because a `mode=memory&cache=shared` URI is
//! scoped to the process and cargo runs these on threads of one.

use nvs_config::tree::Database;
use nvs_db::{Driver, SqliteConn, SqliteTarget, SqliteValue};
use nvs_stdlib::queue;

/// The queue this file's rows are pushed onto, which is the key every statement
/// here is claimed and ordered by.
const QUEUE: &str = "sqlite-claims";

/// An epoch-millis instant a fixture's `run_at` and `created_at` sit at.
///
/// A constant rather than `now`, because every case here compares an instant
/// against a bound it also chose: what the claim reads is `run_at <= ?`, and a
/// fixture whose two sides both move cannot state which side it is testing.
const NOW: i64 = 1_767_225_600_000;

/// The visibility window these cases claim under, in milliseconds.
///
/// A claim's third bind is `now - window` and never `now`: the second arm reads
/// `state = 1 and claimed_at <= ?`, so a cutoff *at* the instant of the claim
/// makes a job the caller just took due again on the very next call. That is the
/// statement behaving exactly as `rule:concurrency/delivery-is-at-least-once`
/// says and a caller passing the wrong instant, which is why the bind has a name
/// here rather than being written out per case.
const WINDOW: i64 = 30_000;

/// A block naming one in-memory database that every handle in this process
/// naming it shares.
///
/// The `nvs-db` half of this arrangement is `crates/nvs-db/src/sqlite.rs`'s own
/// `shared` helper, and its doc owns why a URI is the spelling: two handles to
/// one database, and nothing left on disk for a failing case to leak.
fn block(name: &str) -> Database {
    Database {
        driver: Some(String::from("sqlite")),
        path: Some(format!("file:{name}?mode=memory&cache=shared")),
        ..Database::default()
    }
}

/// A connection to `block`'s database.
fn connect(block: &Database) -> SqliteConn {
    let target = SqliteTarget::resolve(block).expect("the block resolves to a path");
    nvs_db::sqlite::open(&target).expect("an in-memory database opens")
}

/// Two connections to one database, with § 2's two tables converged onto it.
///
/// The schema is [`queue::migration`]'s and never a copy: what these cases run
/// their statements against is the DDL `nvs queue migrate` would apply, so a
/// claim naming a column the schema does not have fails here rather than in
/// front of an operator.
fn two_connections(name: &str) -> (SqliteConn, SqliteConn) {
    let block = block(name);
    let first = connect(&block);
    let second = connect(&block);

    for step in queue::migration(Driver::Sqlite) {
        rows(&first, &step.sql, Vec::new());
    }

    (first, second)
}

/// One statement's rows, materialized, with the connection handed back.
///
/// A `SqliteRows` borrows the connection until it is dropped and leaves it
/// unable to start a second statement, and every case here runs a second one:
/// the claim is a pair.
fn rows(conn: &SqliteConn, sql: &str, params: Vec<SqliteValue>) -> Vec<Vec<SqliteValue>> {
    let mut answered = conn
        .query(sql, params)
        .unwrap_or_else(|refused| panic!("`{sql}` runs: {refused}"));
    let mut all = Vec::new();
    while let Some(row) = answered.next_row() {
        all.push(row);
    }
    all
}

/// One integer cell, or the panic naming what came back instead.
fn int(cell: &SqliteValue) -> i64 {
    match cell {
        SqliteValue::Int(read) => *read,
        other => panic!("this column is an integer and answered {other:?}"),
    }
}

/// One pending job on [`QUEUE`], due at `run_at`, with `attempts` already spent.
///
/// The insert is this file's own rather than [`queue::INSERT_MYSQL`]'s, because
/// it is the fixture and not the thing under test: what a case here asserts is
/// what a claim reads, and a claim reads the columns § 2's schema declares.
fn push(conn: &SqliteConn, run_at: i64, state: i64, attempts: i64, claimed_at: Option<i64>) -> i64 {
    rows(
        conn,
        "insert into nvs_jobs \
         (queue, script, args, state, attempts, max_attempts, backoff_ms, run_at, claimed_at, \
         dedupe_key, dedupe_pending, created_at, tag) \
         values (?, ?, ?, ?, ?, 3, 250, ?, ?, null, null, ?, null)",
        vec![
            SqliteValue::Text(String::from(QUEUE)),
            SqliteValue::Text(String::from("jobs/send.nvs")),
            SqliteValue::Text(String::from("[]")),
            SqliteValue::Int(state),
            SqliteValue::Int(attempts),
            SqliteValue::Int(run_at),
            claimed_at.map_or(SqliteValue::Null, SqliteValue::Int),
            SqliteValue::Int(run_at),
        ],
    );

    let read = rows(
        conn,
        "select id from nvs_jobs where queue = ? order by id desc limit 1",
        vec![SqliteValue::Text(String::from(QUEUE))],
    );
    int(&read.first().expect("the fixture landed a row")[0])
}

/// `rule:concurrency/claiming-is-one-statement`'s claim, taken whole: the pair
/// inside one immediate transaction, answering the row it claimed or nothing.
///
/// This is `crates/nvs-cli/src/worker.rs`'s `claimed_in_two` in the shape a test
/// can hold — the same two statements in the same order, keyed on what the first
/// one named.
fn claim(conn: &SqliteConn, now: i64, cutoff: i64) -> Option<Vec<SqliteValue>> {
    conn.begin_immediate().expect("the write lock, up front");
    let due = rows(
        conn,
        queue::CLAIM_SQLITE.first,
        vec![
            SqliteValue::Text(String::from(QUEUE)),
            SqliteValue::Int(now),
            SqliteValue::Int(cutoff),
        ],
    );

    let taken = due.into_iter().next();
    if let Some(job) = taken.as_ref() {
        rows(
            conn,
            queue::CLAIM_SQLITE.then,
            vec![SqliteValue::Int(now), SqliteValue::Int(int(&job[0]))],
        );
    }
    conn.commit().expect("the claim's transaction closes");
    taken
}

/// **The case that decides the mechanism.** One job, two connections, and the
/// claim run on both: the first takes it, the second is refused the write lock
/// outright and then finds nothing left to take.
///
/// Both halves matter, and neither is visible on one connection. The refusal is
/// § 4's mutual exclusion — inside an immediate transaction no second connection
/// is writing at all, which is why `CLAIM_SQLITE` needs no `skip locked` and has
/// nowhere to put one. The empty second claim is what makes the exclusion a
/// *claim* rather than a lock: the row the first connection took is no longer
/// due, so the second comes back with nothing rather than with the same job.
#[test]
fn two_connections_claiming_one_sqlite_file_never_come_back_with_the_same_job() {
    let (first, second) = two_connections("nvs-stdlib-queue-one-job-two-workers");
    let pushed = push(&first, NOW, 0, 0, None);

    first
        .begin_immediate()
        .expect("the first worker's transaction");
    let due = rows(
        &first,
        queue::CLAIM_SQLITE.first,
        vec![
            SqliteValue::Text(String::from(QUEUE)),
            SqliteValue::Int(NOW),
            SqliteValue::Int(NOW - WINDOW),
        ],
    );
    assert_eq!(due.len(), 1, "one job is due, and a claim takes one row");
    assert_eq!(int(&due[0][0]), pushed);

    let refused = second
        .begin_immediate()
        .expect_err("SQLite has one writer, and the first worker is it");
    let server = nvs_db::ServerError::of(&refused).expect("the refusal carried no kind");
    assert_eq!(
        server.kind,
        nvs_db::DbErrorKind::Deadlock,
        "§ 8's kind is what makes `{{retries: n}}` mean something here"
    );
    assert_eq!(second.depth(), 0, "a refused begin opened no transaction");

    rows(
        &first,
        queue::CLAIM_SQLITE.then,
        vec![SqliteValue::Int(NOW), SqliteValue::Int(pushed)],
    );
    first.commit().expect("the first worker's claim lands");

    let second_claim = claim(&second, NOW, NOW - WINDOW);
    assert!(
        second_claim.is_none(),
        "the job the first worker claimed is not due again until its window passes"
    );
}

/// `attempts` is answered as the value the row is *about to* have, which is
/// `CLAIM_MYSQL`'s reason and not PostgreSQL's: the `update` has not run when the
/// `select` answers, so a claim reading the stored count would stop § 6's ladder
/// one rung short of `max_attempts`.
///
/// Asserted against the column the second half then writes, so a claim that
/// answered `attempts + 1` and stored something else fails here as well.
#[test]
fn a_sqlite_claim_reads_the_attempt_it_is_about_to_have_and_not_the_one_before() {
    let (worker, reader) = two_connections("nvs-stdlib-queue-attempt-about-to-have");
    let pushed = push(&worker, NOW, 0, 2, None);

    let claimed = claim(&worker, NOW, NOW).expect("the job is due");
    assert_eq!(int(&claimed[0]), pushed);
    assert_eq!(
        int(&claimed[3]),
        3,
        "the fourth column is `attempts + 1`, read before the `update` that makes it true"
    );

    let stored = rows(
        &reader,
        "select attempts, state from nvs_jobs where id = ?",
        vec![SqliteValue::Int(pushed)],
    );
    assert_eq!(
        int(&stored[0][0]),
        3,
        "what the claim answered is what its second half wrote"
    );
    assert_eq!(int(&stored[0][1]), 1, "and the row is claimed");
}

/// `rule:concurrency/delivery-is-at-least-once`'s visibility timeout, as the
/// claim's second arm: a job already claimed is claimable again once its lease
/// is at or before the cutoff, and not one millisecond earlier.
///
/// Both sides of the bound, because a claim that spelled `<` for `<=` — or that
/// dropped the arm — reads plausibly against either half alone.
#[test]
fn a_sqlite_claim_takes_a_job_whose_visibility_window_has_passed() {
    let (worker, _reader) = two_connections("nvs-stdlib-queue-window-has-passed");
    let lease = NOW - WINDOW;
    let abandoned = push(&worker, NOW, 1, 1, Some(lease));

    let too_early = claim(&worker, NOW, lease - 1);
    assert!(
        too_early.is_none(),
        "a lease taken after the cutoff is a job still in flight"
    );

    let taken = claim(&worker, NOW, lease).expect("the lease is at the cutoff");
    assert_eq!(int(&taken[0]), abandoned);
    assert_eq!(
        int(&taken[3]),
        2,
        "the returning attempt is the one the retry ladder is about to count"
    );
}

/// `INSERT_SQLITE`'s pair, taken whole: the dedupe read and the insert inside one
/// immediate transaction, answering the job's id and whether the key was already
/// pending.
///
/// This is `push_in_two` one crate over in the shape a test can hold — the same
/// two statements in the same order, with `then` not running at all when `first`
/// answered a row. The new id is read back with a `select` rather than off the
/// connection's last insert, because what is under test is the text and not how
/// the caller learns the id.
fn push_in_two(conn: &SqliteConn, key: Option<&str>, run_at: i64) -> (i64, bool) {
    conn.begin_immediate().expect("the write lock, up front");

    if let Some(key) = key {
        let pending = rows(
            conn,
            queue::INSERT_SQLITE.first,
            vec![SqliteValue::Text(String::from(key))],
        );
        if let Some(job) = pending.first() {
            let already = int(&job[0]);
            conn.commit()
                .expect("the deduped push's transaction closes");
            return (already, true);
        }
    }

    let dedupe = key.map_or(SqliteValue::Null, |key| {
        SqliteValue::Text(String::from(key))
    });
    rows(
        conn,
        queue::INSERT_SQLITE.then,
        vec![
            SqliteValue::Text(String::from(QUEUE)),
            SqliteValue::Text(String::from("jobs/send.nvs")),
            SqliteValue::Text(String::from("[]")),
            SqliteValue::Int(0),
            SqliteValue::Int(3),
            SqliteValue::Int(250),
            SqliteValue::Int(run_at),
            dedupe.clone(),
            dedupe,
            SqliteValue::Int(run_at),
            SqliteValue::Null,
        ],
    );

    let landed = rows(
        conn,
        "select id from nvs_jobs where queue = ? order by id desc limit 1",
        vec![SqliteValue::Text(String::from(QUEUE))],
    );
    let id = int(&landed.first().expect("the insert landed a row")[0]);
    conn.commit().expect("the push's transaction closes");
    (id, false)
}

/// § 2's guarantee on this backend: at most one *pending* job per dedupe key, and
/// what enforces it is the pair rather than the unique index catching a second
/// insert.
///
/// The second push is run on the other connection, which is the half a
/// single-connection case could not state: two pushes of one key that never
/// overlap prove nothing about the read-then-insert this text is. Inside the
/// immediate transaction the second connection is not reading a snapshot the
/// first is still writing — it waits, and then reads the row the first one
/// committed.
///
/// Asserted as the same id and one row, not as a refusal: a deduped push is an
/// ordinary answer carrying the pending job's receipt, and a case asserting an
/// error would pin the opposite of what `Core\Queue::push` promises.
#[test]
fn a_sqlite_push_of_a_key_already_pending_answers_that_job_and_inserts_nothing() {
    let (first, second) = two_connections("nvs-stdlib-queue-dedupe-pending");

    let (landed, deduped) = push_in_two(&first, Some("welcome:7"), NOW);
    assert!(!deduped, "the first push of a key is an insert");

    let (answered, again) = push_in_two(&second, Some("welcome:7"), NOW);
    assert!(again, "the key is still pending, so the read answers it");
    assert_eq!(
        answered, landed,
        "a deduped push answers the pending job's own receipt"
    );

    let counted = rows(
        &first,
        "select count(*) from nvs_jobs where queue = ?",
        vec![SqliteValue::Text(String::from(QUEUE))],
    );
    assert_eq!(
        int(&counted[0][0]),
        1,
        "`then` does not run at all when `first` answered a row"
    );
}

/// The other side of the same bound: the key is free again once the job it was
/// on stops being pending, and it is `dedupe_pending` going null that frees it.
///
/// `CLAIM_SQLITE.then` is what nulls the column, so this case is the two
/// statements agreeing — a claim that stopped maintaining `dedupe_pending`, or an
/// insert that keyed its read on `dedupe_key` instead, both fail here while each
/// still reads plausibly on its own. A null collides with nothing under
/// `nvs_jobs_dedupe`, which is `rule:core-classes/queue-storage-is-a-table`'s
/// reason for the column existing at all.
#[test]
fn a_sqlite_push_reuses_a_dedupe_key_the_claim_has_released() {
    let (worker, pusher) = two_connections("nvs-stdlib-queue-dedupe-released");

    let (landed, _) = push_in_two(&pusher, Some("digest:daily"), NOW);
    let claimed = claim(&worker, NOW, NOW).expect("the pushed job is due");
    assert_eq!(int(&claimed[0]), landed);

    let (second, deduped) = push_in_two(&pusher, Some("digest:daily"), NOW);
    assert!(
        !deduped,
        "a claimed job holds no dedupe key, so the same key inserts again"
    );
    assert_ne!(second, landed, "the second push is its own row");
}

/// A push with no key runs `then` alone, and the row it lands carries no key
/// either — two nulls in the column pair, which is the state every other
/// statement reads as "this job was never deduped".
///
/// Two of them, because one null tells nothing about the unique index: what would
/// break a queue that ran keyless pushes through `dedupe_pending` is the *second*
/// row colliding with the first.
#[test]
fn a_sqlite_push_with_no_key_lands_every_row_it_is_given() {
    let (pusher, _reader) = two_connections("nvs-stdlib-queue-keyless-push");

    let (first, deduped) = push_in_two(&pusher, None, NOW);
    assert!(!deduped, "a keyless push reads nothing and inserts");
    let (second, _) = push_in_two(&pusher, None, NOW);
    assert_ne!(second, first, "a keyless push collides with nothing");

    let counted = rows(
        &pusher,
        "select count(*) from nvs_jobs where queue = ? and dedupe_pending is null",
        vec![SqliteValue::Text(String::from(QUEUE))],
    );
    assert_eq!(int(&counted[0][0]), 2);
}
