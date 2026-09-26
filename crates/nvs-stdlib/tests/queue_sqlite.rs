//! `rule:concurrency/claiming-is-one-statement`'s claim on SQLite, and the
//! statements the worker runs beside it, against a real engine by two
//! connections at once.
//!
//! **This is the one queue suite that needs no server, and that is why it is
//! not `queue.rs` beside it.** That file's cases assert nothing at
//! all unless `NVS_DB_MATRIX_DRIVER` names a backend
//! `bun nv db-matrix` has started a container for, because a socket is the
//! only way to reach the four drivers that have one. SQLite is a file, so every
//! case here runs on every machine and under `bun nv verify` — the
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
    push_on(conn, QUEUE, run_at, state, attempts, claimed_at)
}

/// [`push`] onto a queue the case names, for the one statement whose answer is
/// about the *table* rather than about a row.
///
/// [`queue::QUEUES_SQLITE`] answers `select distinct queue`, so the case that
/// runs it needs rows on more than one name and a database holding nothing else
/// — and a database per case is what this file already gives it.
fn push_on(
    conn: &SqliteConn,
    queue: &str,
    run_at: i64,
    state: i64,
    attempts: i64,
    claimed_at: Option<i64>,
) -> i64 {
    rows(
        conn,
        "insert into nvs_jobs \
         (queue, script, args, state, attempts, max_attempts, backoff_ms, run_at, claimed_at, \
         dedupe_key, dedupe_pending, created_at, tag) \
         values (?, ?, ?, ?, ?, 3, 250, ?, ?, null, null, ?, null)",
        vec![
            SqliteValue::Text(String::from(queue)),
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
        vec![SqliteValue::Text(String::from(queue))],
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
            // The narrowing pair, after the tag: a fixture pushing by hand
            // stands in for a context that narrowed nothing, which is the null
            // `queue::schema` declares both columns for.
            SqliteValue::Null,
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
// covers: Core\Queue::push
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

/// One statement's affected count, which is what a member answers where
/// PostgreSQL reads a `returning` row.
///
/// `sqlite3_changes` is read off the finished statement, so the rows are
/// materialized here for [`rows`]'s reason and the count is taken from the same
/// handle rather than from a `select changes()` after it.
fn affected(conn: &SqliteConn, sql: &str, params: Vec<SqliteValue>) -> u64 {
    conn.query(sql, params)
        .unwrap_or_else(|refused| panic!("`{sql}` runs: {refused}"))
        .affected()
}

/// `DEAD_LETTER_SQLITE`'s pair, taken whole: the copy into § 2's second table and
/// the delete that follows it, both keyed on the lease the claim wrote.
///
/// `crates/nvs-cli/src/worker.rs` runs these two in this order, and what holds
/// them as one moment here is the immediate transaction rather than anything in
/// the text.
fn dead_letter(conn: &SqliteConn, id: i64, lease: i64) {
    conn.begin_immediate().expect("the write lock, up front");
    rows(
        conn,
        queue::DEAD_LETTER_SQLITE.first,
        vec![
            SqliteValue::Int(lease),
            SqliteValue::Text(queue::dead_errors(
                None,
                lease,
                "LogicError",
                "the last attempt threw",
            )),
            SqliteValue::Int(id),
            SqliteValue::Int(lease),
        ],
    );
    rows(
        conn,
        queue::DEAD_LETTER_SQLITE.then,
        vec![SqliteValue::Int(id), SqliteValue::Int(lease)],
    );
    conn.commit().expect("the move's transaction closes");
}

/// `STATUS_SQLITE`'s answer for one receipt, or `None` when no row in either
/// table carries it.
fn status(conn: &SqliteConn, id: i64) -> Option<i64> {
    let read = rows(
        conn,
        queue::STATUS_SQLITE,
        vec![
            SqliteValue::Int(id),
            SqliteValue::Text(String::from(QUEUE)),
            SqliteValue::Int(id),
            SqliteValue::Text(String::from(QUEUE)),
        ],
    );
    read.first().map(|row| int(&row[0]))
}

/// `STATUS_SQLITE` is MySQL's text and this is the case that says so by running
/// it: one receipt read at three states, the last of them from the other table.
///
/// The `union all` under one `limit` is the construct worth executing rather than
/// reading — it is the same statement in both dialects only if the `limit` binds
/// to the compound and not to its second arm, and a statement that answered the
/// dead-letter arm first would read plausibly against any single state.
// covers: Core\Queue::status
#[test]
fn a_sqlite_status_reads_the_live_state_and_then_the_dead_letter_ordinal() {
    let (worker, reader) = two_connections("nvs-stdlib-queue-status-both-arms");
    let pushed = push(&worker, NOW, 0, 0, None);

    assert_eq!(
        status(&reader, pushed),
        Some(0),
        "a pushed job answers `State::Pending`'s ordinal"
    );
    let claimed = claim(&worker, NOW, NOW).expect("the pushed job is due");
    assert_eq!(int(&claimed[0]), pushed);
    assert_eq!(
        status(&reader, pushed),
        Some(1),
        "a claim writes the state the same receipt then reads back"
    );

    dead_letter(&worker, pushed, NOW);
    assert_eq!(
        status(&reader, pushed),
        Some(3),
        "§ 6 moved the row, and the second arm is what keeps the receipt answerable"
    );
    assert_eq!(
        status(&reader, pushed + 1),
        None,
        "a receipt naming no row in either table answers nothing rather than a state"
    );
}

/// `CANCEL_SQLITE`'s answer for one receipt: the affected count, which is what
/// this backend has in place of a `returning` row.
fn cancel(conn: &SqliteConn, id: i64) -> u64 {
    affected(
        conn,
        queue::CANCEL_SQLITE,
        vec![SqliteValue::Int(id), SqliteValue::Text(String::from(QUEUE))],
    )
}

/// The member's whole semantics are `and state = 0`, and this is that bound
/// asserted on both sides on a real engine: a pending job is cancelled, and a
/// claimed one and an already-cancelled one both change nothing.
///
/// The count is the assertion rather than the row's state alone, because
/// `Counted::touched` is what tells a caller *this call* is what cancelled it —
/// a statement matching no row is an ordinary answer here and not an error, so a
/// text that dropped the state arm would still leave the column reading `4`.
///
/// The key is pushed through `INSERT_SQLITE` so the last assertion can be about
/// `dedupe_pending`: cancel releases a key with the same `null` the claim writes,
/// which is § 2's guarantee holding across a state no worker reached.
// covers: Core\Queue::cancel
#[test]
fn a_sqlite_cancel_takes_a_pending_job_and_changes_nothing_else() {
    let (worker, canceller) = two_connections("nvs-stdlib-queue-cancel-only-pending");
    let (landed, _) = push_in_two(&canceller, Some("welcome:9"), NOW);

    assert_eq!(
        cancel(&canceller, landed),
        1,
        "a pending job is cancellable, and the count is what says this call did it"
    );
    assert_eq!(
        cancel(&canceller, landed),
        0,
        "a cancelled job is no longer pending, so the same call changes nothing"
    );

    let claimed = push(&worker, NOW, 1, 1, Some(NOW));
    assert_eq!(
        cancel(&canceller, claimed),
        0,
        "a claimed job is not cancellable at all, and the `where` is what refuses it"
    );

    let stored = rows(
        &worker,
        "select state from nvs_jobs where id = ?",
        vec![SqliteValue::Int(landed)],
    );
    assert_eq!(
        int(&stored[0][0]),
        4,
        "the row the count claimed carries `State::Cancelled`'s ordinal"
    );

    let (again, deduped) = push_in_two(&canceller, Some("welcome:9"), NOW);
    assert!(
        !deduped,
        "a cancel releases the dedupe key with the same `dedupe_pending = null` a claim writes"
    );
    assert_ne!(again, landed, "so the same key pushes a second row");
}

/// `COUNTS_SQLITE`'s five counters, as the integers a caller decodes.
///
/// [`int`] is the assertion and not a convenience: what a `cast(… as signed)`
/// answers on this backend is the whole question the alias rests on, so a column
/// that came back a text or a real fails here by name.
fn counts(conn: &SqliteConn) -> [i64; 5] {
    let read = rows(
        conn,
        queue::COUNTS_SQLITE,
        vec![
            SqliteValue::Text(String::from(QUEUE)),
            SqliteValue::Text(String::from(QUEUE)),
            SqliteValue::Text(String::from(QUEUE)),
        ],
    );
    let row = read
        .first()
        .expect("an aggregate with no `group by` is exactly one row");
    [
        int(&row[0]),
        int(&row[1]),
        int(&row[2]),
        int(&row[3]),
        int(&row[4]),
    ]
}

/// `stats` answers one row of five integers on this backend too, and the empty
/// queue is the half that decides it: `count` over no rows is `0`, and the
/// `coalesce` is what keeps a `sum` over no rows from answering `null` where § 6
/// means zero.
///
/// The `cast(… as signed)` is MySQL's and inert here, which is a claim only a run
/// can carry — SQLite reads a type name it does not have by its affinity rules,
/// and `COUNTS_SQLITE`'s doc owns why that leaves the sum an integer.
///
/// Asserted across the dead-letter move, because the last two counters are
/// subqueries over the *other* table: counters that all read `nvs_jobs` would
/// answer plausibly until a job was lost, which is the one moment an operator
/// reads `stats` for. The move is also where the two attempt sums prove they are
/// disjoint — the buried job's attempts leave one and arrive in the other, and
/// their total does not move.
// covers: Core\Queue::stats
#[test]
fn sqlite_stats_answer_five_integers_over_an_empty_queue_and_a_worked_one() {
    let (worker, reader) = two_connections("nvs-stdlib-queue-stats-five-counters");

    assert_eq!(
        counts(&reader),
        [0, 0, 0, 0, 0],
        "an empty queue answers zero five times, and neither sum is a null"
    );

    push(&worker, NOW, 0, 5, None);
    let claimed = push(&worker, NOW, 1, 2, Some(NOW));
    assert_eq!(
        counts(&reader),
        [1, 1, 7, 0, 0],
        "one waiting, one in flight, the attempts of both, and a dead-letter table with neither a \
         row nor an attempt in it"
    );

    dead_letter(&worker, claimed, NOW);
    assert_eq!(
        counts(&reader),
        [1, 0, 5, 1, 2],
        "the depth comes from the other table, and the two attempts the buried job used follow the \
         row out of this one rather than being lost between them"
    );
}

/// `attempts` is summed over every row of the queue in `nvs_jobs`, whatever its
/// state: a finished or cancelled job keeps the attempts it used until `purge`
/// or `delete` removes its row. The `where queue = ?` is the aggregate's own
/// and not the subqueries', so a row on another queue adds nothing even when
/// it is the only row with attempts to add.
// covers: Core\Queue\Stats::attempts
#[test]
fn sqlite_attempts_sum_every_state_of_one_queue_and_nothing_of_another() {
    let (worker, reader) = two_connections("nvs-stdlib-queue-stats-attempts");
    push_on(&worker, "another-queue", NOW, 0, 1000, None);
    assert_eq!(
        counts(&reader)[2],
        0,
        "the other queue's attempts are not this queue's"
    );

    for (state, attempts) in [(0, 1), (1, 2), (2, 4), (3, 8), (4, 16)] {
        let claimed_at = (state == 1).then_some(NOW);
        push(&worker, NOW, state, attempts, claimed_at);
    }
    assert_eq!(
        counts(&reader)[2],
        31,
        "each of the five states adds its own power of two, so a state left out is named by the \
         bit missing from the sum"
    );
}

/// `claimed` is the rows of the queue whose `state` is `1`, and nothing about
/// `claimed_at` enters it: a claim long past `[queue] visibility` is still a
/// claim until the next worker's second arm takes it again, so it is counted
/// here and not under `pending`. A claimed row on another queue and a
/// dead-lettered one add nothing.
// covers: Core\Queue\Stats::claimed
#[test]
fn sqlite_claimed_counts_state_one_of_one_queue_however_old_the_claim() {
    let (worker, reader) = two_connections("nvs-stdlib-queue-stats-claimed");
    push_on(&worker, "another-queue", NOW, 1, 1, Some(NOW));
    assert_eq!(
        counts(&reader)[1],
        0,
        "the other queue's claim is not this queue's"
    );

    for state in 0..5 {
        let claimed_at = (state == 1).then_some(NOW);
        push(&worker, NOW, state, 1, claimed_at);
    }
    let expired = push(&worker, NOW, 1, 1, Some(0));
    assert_eq!(
        counts(&reader)[..2],
        [1, 2],
        "one row waits, and both the live claim and the expired one are claimed"
    );

    dead_letter(&worker, expired, 0);
    assert_eq!(
        counts(&reader)[1],
        1,
        "a claim that is dead-lettered leaves the count with its row"
    );
}

/// `deadAttempts` is summed over the queue's rows of `nvs_dead_jobs` alone: the
/// subquery carries its own `where queue = ?`, so a buried job of another queue
/// adds nothing, and a live row's attempts stay in `attempts` until the
/// dead-letter move carries them across.
// covers: Core\Queue\Stats::deadAttempts
#[test]
fn sqlite_dead_attempts_sum_the_buried_rows_of_one_queue_and_nothing_live() {
    let (worker, reader) = two_connections("nvs-stdlib-queue-stats-dead-attempts");
    let elsewhere = push_on(&worker, "another-queue", NOW, 1, 1000, Some(NOW));
    dead_letter(&worker, elsewhere, NOW);
    push(&worker, NOW, 0, 8, None);
    assert_eq!(
        counts(&reader)[4],
        0,
        "the other queue's buried attempts and this queue's live ones are not this counter's"
    );

    for attempts in [1, 2, 4] {
        let id = push(&worker, NOW, 1, attempts, Some(NOW));
        dead_letter(&worker, id, NOW);
    }
    let read = counts(&reader);
    assert_eq!(
        (read[2], read[3], read[4]),
        (8, 3, 7),
        "each buried job adds its own power of two here and nothing to `attempts`, so a row left \
         out is named by the bit missing from the sum"
    );
}

/// `pending` is the rows of the queue whose `state` is `0`, and nothing about
/// `run_at` enters it: a job pushed for later and a job a retry put back to wait
/// out its backoff are pending exactly like one due now, although no claim can
/// take either yet. A waiting row on another queue adds nothing.
// covers: Core\Queue\Stats::pending
#[test]
fn sqlite_pending_counts_state_zero_of_one_queue_whatever_its_run_at() {
    let (worker, reader) = two_connections("nvs-stdlib-queue-stats-pending");
    push_on(&worker, "another-queue", NOW, 0, 0, None);
    assert_eq!(
        counts(&reader)[0],
        0,
        "the other queue's waiting job is not this queue's"
    );

    push(&worker, NOW, 0, 0, None);
    push(&worker, i64::MAX, 0, 0, None);
    push(&worker, NOW + 60_000, 0, 2, None);
    for state in 1..5 {
        let claimed_at = (state == 1).then_some(NOW);
        push(&worker, NOW, state, 1, claimed_at);
    }
    assert_eq!(
        counts(&reader)[..2],
        [3, 1],
        "the job due now, the one due at the end of time and the one between two attempts all \
         wait, and no other state does"
    );
}

/// `deadLettered` is the queue's rows of `nvs_dead_jobs`, counted by a subquery
/// with its own `where queue = ?`: a live row in any state adds nothing however
/// many attempts it has used, a buried row of another queue adds nothing, and
/// the dead-letter move takes a row out of the live counters as it adds it here.
// covers: Core\Queue\Stats::deadLettered
#[test]
fn sqlite_dead_lettered_counts_the_buried_rows_of_one_queue_and_nothing_live() {
    let (worker, reader) = two_connections("nvs-stdlib-queue-stats-dead-lettered");
    let elsewhere = push_on(&worker, "another-queue", NOW, 1, 5, Some(NOW));
    dead_letter(&worker, elsewhere, NOW);
    for state in 0..5 {
        let claimed_at = (state == 1).then_some(NOW);
        push(&worker, NOW, state, 5, claimed_at);
    }
    assert_eq!(
        counts(&reader)[3],
        0,
        "the other queue's buried job and this queue's live ones are not this counter's"
    );

    for _ in 0..3 {
        let id = push(&worker, NOW, 1, 5, Some(NOW));
        dead_letter(&worker, id, NOW);
    }
    assert_eq!(
        counts(&reader)[..4],
        [1, 1, 25, 3],
        "the three buried jobs are counted here and left the claimed count with their rows"
    );
}

/// `DELETE_SQLITE`'s pair, taken whole: both arms inside one immediate
/// transaction, and the member's `bool` is either of them having removed a row.
///
/// `then` runs whatever `first` answered, which is what makes this the one
/// `Split` here that is not keyed on its first statement's row — the constant's
/// own doc owns why, and why the transaction is still what the pair means.
fn delete_in_two(conn: &SqliteConn, id: i64) -> bool {
    conn.begin_immediate().expect("the write lock, up front");
    let receipt = vec![SqliteValue::Int(id), SqliteValue::Text(String::from(QUEUE))];
    let gone = affected(conn, queue::DELETE_SQLITE.first, receipt.clone());
    let buried = affected(conn, queue::DELETE_SQLITE.then, receipt);
    conn.commit().expect("the removal's transaction closes");
    assert!(
        gone + buried <= 1,
        "§ 6 moves a job, so a receipt is in one of § 2's tables and never in both"
    );
    gone + buried > 0
}

/// The rows of § 2's two tables that still carry a receipt.
fn present(conn: &SqliteConn, id: i64) -> (i64, i64) {
    let read = rows(
        conn,
        "select (select count(*) from nvs_jobs where id = ?), \
         (select count(*) from nvs_dead_jobs where id = ?)",
        vec![SqliteValue::Int(id), SqliteValue::Int(id)],
    );
    (int(&read[0][0]), int(&read[0][1]))
}

/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s `delete` on the
/// one backend where it is two statements: a finished job goes from either table,
/// and a claimed one goes from neither.
///
/// Executed rather than read because this member is the stage's only construct —
/// two arms standing in for a data-modifying CTE — and a pair that removed from
/// the wrong table, or that let the second arm answer for the first, reads
/// plausibly in either text alone.
// covers: Core\Queue::delete
#[test]
fn a_sqlite_delete_removes_a_receipt_from_either_table_and_a_claimed_job_from_neither() {
    let (worker, caller) = two_connections("nvs-stdlib-queue-delete-two-arms");

    let finished = push(&worker, NOW, 2, 1, None);
    assert!(
        delete_in_two(&caller, finished),
        "a succeeded job is removable, and the count is what says this call removed it"
    );
    assert_eq!(present(&caller, finished), (0, 0));
    assert!(
        !delete_in_two(&caller, finished),
        "a receipt naming no row in either table removes nothing"
    );

    let claimed = push(&worker, NOW, 1, 1, Some(NOW));
    assert!(
        !delete_in_two(&caller, claimed),
        "a claimed job is not removable at all, and `state <> 1` is what refuses it"
    );
    assert_eq!(
        present(&caller, claimed),
        (1, 0),
        "the row a worker holds a lease on is still there"
    );

    let lost = push(&worker, NOW, 0, 1, None);
    // The cutoff is a window back rather than `NOW`, because the claimed row above is still here:
    // at `NOW` the claim's second arm would find its lease due and take that job instead.
    let taken = claim(&worker, NOW, NOW - WINDOW).expect("the pending job is due");
    assert_eq!(int(&taken[0]), lost);
    dead_letter(&worker, lost, NOW);
    assert_eq!(present(&caller, lost), (0, 1), "§ 6 moved it");
    assert!(
        delete_in_two(&caller, lost),
        "the second arm is what keeps a receipt removable across that move"
    );
    assert_eq!(present(&caller, lost), (0, 0));
}

/// `PURGE_SQLITE`'s selection: the state set the call named, narrowed by an
/// optional tag and an optional age, and never more rows than the bound.
fn purge(
    conn: &SqliteConn,
    state: Option<i64>,
    tag: Option<&str>,
    before: Option<i64>,
    limit: i64,
) -> u64 {
    let state = state.map_or(SqliteValue::Null, SqliteValue::Int);
    let tag = tag.map_or(SqliteValue::Null, |tag| {
        SqliteValue::Text(String::from(tag))
    });
    let before = before.map_or(SqliteValue::Null, SqliteValue::Int);
    affected(
        conn,
        queue::PURGE_SQLITE,
        vec![
            SqliteValue::Text(String::from(QUEUE)),
            state.clone(),
            state,
            tag.clone(),
            tag,
            before.clone(),
            before,
            SqliteValue::Int(limit),
        ],
    )
}

/// The ids left in one of § 2's tables, oldest first.
fn remaining(conn: &SqliteConn, table: &str) -> Vec<i64> {
    rows(
        conn,
        &format!("select id from {table} where queue = ? order by id"),
        vec![SqliteValue::Text(String::from(QUEUE))],
    )
    .iter()
    .map(|row| int(&row[0]))
    .collect()
}

/// `purge` over the jobs table, which on this backend carries its bound in a
/// subquery because `delete … limit` needs a build option nothing here sets.
///
/// Every arm of the text is asked something a broken one would answer wrongly:
/// the age selects the row on the far side of the cutoff and not the one just
/// inside it, the bound removes exactly one row and takes the oldest, the
/// default set is what has finished, `State::Pending` is opt-in, and
/// `State::Claimed` is refused by the text even when a call names it.
// covers: Core\Queue::purge
#[test]
fn a_sqlite_purge_selects_the_finished_set_and_never_more_rows_than_its_bound() {
    let (worker, sweeper) = two_connections("nvs-stdlib-queue-purge-jobs");

    let oldest = push(&worker, NOW - 5_000, 2, 1, None);
    let cancelled = push(&worker, NOW - 4_000, 4, 1, None);
    let pending = push(&worker, NOW, 0, 0, None);
    let claimed = push(&worker, NOW, 1, 1, Some(NOW));
    let succeeded = push(&worker, NOW, 2, 1, None);

    assert_eq!(
        purge(&sweeper, None, None, Some(NOW - 4_500), 10),
        1,
        "the age is asked of `created_at`, so only the row on the far side of the cutoff goes"
    );
    assert_eq!(
        remaining(&sweeper, "nvs_jobs"),
        vec![cancelled, pending, claimed, succeeded],
        "and the row just inside it stays"
    );
    assert_eq!(
        oldest,
        cancelled - 1,
        "the fixture's ids run in push order, which is what `order by id` sweeps in"
    );

    assert_eq!(
        purge(&sweeper, Some(1), None, None, 10),
        0,
        "`state <> 1` refuses a claimed job in the text as well as at the call"
    );
    assert_eq!(
        purge(&sweeper, None, None, None, 1),
        1,
        "the bound is a subquery here and it is the same bound"
    );
    assert_eq!(
        remaining(&sweeper, "nvs_jobs"),
        vec![pending, claimed, succeeded],
        "and the row it took is the oldest of the set, not wherever the scan started"
    );

    assert_eq!(
        purge(&sweeper, None, None, None, 10),
        1,
        "the default set is what has finished, which leaves the pending job where it is"
    );
    assert_eq!(
        purge(&sweeper, Some(0), None, None, 10),
        1,
        "`State::Pending` goes only when a call names it"
    );
    assert_eq!(
        remaining(&sweeper, "nvs_jobs"),
        vec![claimed],
        "and the one state with no removal in it is what is left"
    );
}

/// `PURGE_DEAD_SQLITE`'s selection over the other table: no state, the same tag
/// and age, and the same bound.
fn purge_dead(conn: &SqliteConn, tag: Option<&str>, before: Option<i64>, limit: i64) -> u64 {
    let tag = tag.map_or(SqliteValue::Null, |tag| {
        SqliteValue::Text(String::from(tag))
    });
    let before = before.map_or(SqliteValue::Null, SqliteValue::Int);
    affected(
        conn,
        queue::PURGE_DEAD_SQLITE,
        vec![
            SqliteValue::Text(String::from(QUEUE)),
            tag.clone(),
            tag,
            before.clone(),
            before,
            SqliteValue::Int(limit),
        ],
    )
}

/// One dead-lettered row, tagged as the enqueue would have tagged it.
///
/// The tag is written onto the job before the move rather than into the dead
/// table afterwards, because that is § 6's own claim: a group tagged at enqueue
/// is still that group once its work was lost, and the move's `select` is what
/// has to carry the column for a purge to be able to ask about it.
fn buried(conn: &SqliteConn, created_at: i64, tag: Option<&str>) -> i64 {
    let id = push(conn, created_at, 0, 1, None);
    if let Some(tag) = tag {
        rows(
            conn,
            "update nvs_jobs set tag = ? where id = ?",
            vec![SqliteValue::Text(String::from(tag)), SqliteValue::Int(id)],
        );
    }
    let claimed = claim(conn, NOW, NOW).expect("the fixture's job is due");
    assert_eq!(int(&claimed[0]), id, "and it is the one just pushed");
    dead_letter(conn, id, NOW);
    id
}

/// `purge` over the dead-letter table: the record that work was lost, swept by
/// the two options § 6 moves the columns for and by the same bound.
///
/// A tag that names no row is the first assertion, because a filter that fell
/// through to *everything* would pass every other one here.
#[test]
fn a_sqlite_purge_of_the_dead_letter_table_asks_the_tag_and_the_age_of_that_table() {
    let (worker, sweeper) = two_connections("nvs-stdlib-queue-purge-dead");

    let old_tagged = buried(&worker, NOW - 5_000, Some("batch:7"));
    let old_plain = buried(&worker, NOW - 5_000, None);
    let new_tagged = buried(&worker, NOW, Some("batch:7"));

    assert_eq!(
        purge_dead(&sweeper, Some("batch:9"), None, 10),
        0,
        "a tag naming no row removes nothing at all"
    );
    assert_eq!(
        purge_dead(&sweeper, Some("batch:7"), Some(NOW - 4_500), 10),
        1,
        "the two options narrow together, so only the old row of that group goes"
    );
    assert_eq!(
        remaining(&sweeper, "nvs_dead_jobs"),
        vec![old_plain, new_tagged]
    );

    assert_eq!(
        purge_dead(&sweeper, None, None, 1),
        1,
        "the bound is this text's as well, in `PURGE_SQLITE`'s subquery shape"
    );
    assert_eq!(
        remaining(&sweeper, "nvs_dead_jobs"),
        vec![new_tagged],
        "and it took the oldest of the table"
    );

    assert_eq!(purge_dead(&sweeper, None, None, 10), 1);
    assert_eq!(
        remaining(&sweeper, "nvs_dead_jobs"),
        Vec::<i64>::new(),
        "a sweep naming no option is still bounded, and this one reached the end"
    );
    assert_eq!(
        old_tagged + 1,
        old_plain,
        "the fixture's ids run in push order, which is what `order by id` sweeps in"
    );
}

/// § 6's move, executed: an exhausted job leaves `nvs_jobs` for `nvs_dead_jobs`
/// carrying the columns § 2's second table declares, rather than being discarded
/// or left claimed forever.
///
/// **The copy runs before the delete on this backend**, which is
/// [`queue::DEAD_LETTER_MYSQL`]'s order and not PostgreSQL's, so the columns are
/// read while they still exist; what holds the two halves as one moment is the
/// immediate transaction rather than anything in the text.
///
/// **Asserted from both sides**, because § 6's property is not that the row
/// appears in the other table but that it is not discarded: a move that deleted
/// without copying would satisfy a case that only looked at where the job went.
/// The fixture spends two of three attempts and the claim spends the third, so
/// the row reaches the move with nothing left to try — which is the condition
/// `crates/nvs-cli/src/worker.rs` branches on, stated here rather than assumed.
#[test]
fn a_sqlite_dead_letter_move_carries_the_row_whole_into_the_other_table() {
    let (worker, reader) = two_connections("nvs-stdlib-queue-dead-letter");

    let id = push(&worker, NOW, 0, 2, None);
    let claimed = claim(&worker, NOW, NOW - WINDOW).expect("the pushed job is due");
    assert_eq!(
        (int(&claimed[0]), int(&claimed[3]), int(&claimed[4])),
        (id, 3, 3),
        "the claim spent the job's last attempt, which is what makes the move the legal one"
    );

    dead_letter(&worker, id, NOW);

    assert_eq!(
        present(&reader, id),
        (0, 1),
        "the receipt is in the other table, and in exactly one of the two"
    );
    assert_eq!(
        rows(
            &reader,
            "select id, queue, script, args, attempts, max_attempts, backoff_ms, run_at, \
             created_at, failed_at, errors from nvs_dead_jobs where id = ?",
            vec![SqliteValue::Int(id)],
        ),
        vec![vec![
            SqliteValue::Int(id),
            SqliteValue::Text(String::from(QUEUE)),
            SqliteValue::Text(String::from("jobs/send.nvs")),
            SqliteValue::Text(String::from("[]")),
            SqliteValue::Int(3),
            SqliteValue::Int(3),
            SqliteValue::Int(250),
            SqliteValue::Int(NOW),
            SqliteValue::Int(NOW),
            SqliteValue::Int(NOW),
            SqliteValue::Text(queue::dead_errors(
                None,
                NOW,
                "LogicError",
                "the last attempt threw",
            )),
        ]],
        "carrying its own columns and the array § 6 asks for"
    );
}

/// The `dedupe_pending` one job carries, or `None` where the column is null.
///
/// Read as the column rather than as a count, because both of its values are an
/// assertion here: the key that is on the row and the null that is not are the
/// two halves § 2's guarantee is made of.
fn pending_key(conn: &SqliteConn, id: i64) -> Option<String> {
    let read = rows(
        conn,
        "select dedupe_pending from nvs_jobs where id = ?",
        vec![SqliteValue::Int(id)],
    );
    match read.first().map(|row| &row[0]) {
        Some(SqliteValue::Text(key)) => Some(key.clone()),
        None | Some(SqliteValue::Null) => None,
        Some(other) => panic!("`dedupe_pending` is a text column and answered {other:?}"),
    }
}

/// § 6's ladder putting a job back: the row returns to `Pending` at the instant
/// the caller chose, and the dedupe key the claim took off it comes back with it.
///
/// **`dedupe_pending = dedupe_key` is the half nothing else here states.**
/// `rule:core-classes/queue-storage-is-a-table` admits at most one *pending* job
/// per key, so a retry that left the column null would hand a queue two pending
/// rows on one key — and every statement in the roster would still read as legal,
/// because the unique index reads two nulls as distinct. It is asserted through a
/// second push as well as off the column, so what is pinned is the guarantee and
/// not one write.
///
/// The lease keying is the other half, and
/// [`a_sqlite_worker_that_overran_its_visibility_window_reports_nothing`] holds
/// it from the side where it refuses.
#[test]
fn a_sqlite_retry_puts_the_dedupe_key_back_on_the_row_it_released() {
    let (worker, pusher) = two_connections("nvs-stdlib-queue-retry-key");

    let (id, _) = push_in_two(&pusher, Some("digest:daily"), NOW);
    let claimed = claim(&worker, NOW, NOW - WINDOW).expect("the pushed job is due");
    assert_eq!(int(&claimed[0]), id);
    assert_eq!(
        pending_key(&pusher, id),
        None,
        "the claim released the key, which is the state the retry has to undo"
    );

    let due_again = NOW + 250;
    assert_eq!(
        affected(
            &worker,
            queue::RETRY_SQLITE,
            vec![
                SqliteValue::Int(due_again),
                SqliteValue::Text(queue::dead_errors(
                    None,
                    NOW,
                    "LogicError",
                    "the attempt threw",
                )),
                SqliteValue::Int(id),
                SqliteValue::Int(NOW),
            ],
        ),
        1,
        "the worker still holds the lease it claimed with, so its retry lands"
    );
    assert_eq!(
        pending_key(&pusher, id),
        Some(String::from("digest:daily")),
        "and the key the job was pushed under is on the row again"
    );

    let (answered, deduped) = push_in_two(&pusher, Some("digest:daily"), due_again);
    assert!(
        deduped,
        "so the key is pending again and a second push reads it"
    );
    assert_eq!(
        answered, id,
        "answering the retried job's own receipt rather than landing a second row"
    );
}

/// § 2's roster, sorted: the queue names a worker idling at `now` would claim
/// against.
///
/// `select distinct` names no order, and what a case asks of it is a set, so the
/// comparison is made against a sorted vector rather than against whichever walk
/// the planner de-duplicated with.
fn roster(conn: &SqliteConn, now: i64, cutoff: i64) -> Vec<String> {
    let mut named: Vec<String> = rows(
        conn,
        queue::QUEUES_SQLITE,
        vec![SqliteValue::Int(now), SqliteValue::Int(cutoff)],
    )
    .iter()
    .map(|row| match &row[0] {
        SqliteValue::Text(name) => name.clone(),
        other => panic!("`queue` is a text column and answered {other:?}"),
    })
    .collect();
    named.sort();
    named
}

/// § 2's unanswered question — which queues hold work — asked of the table,
/// because the config block names none.
///
/// **Both arms, and both of their bounds on either side.** A pending job due one
/// millisecond later is not work yet, a claimed job whose lease is newer than the
/// cutoff is somebody else's work, and the lease sitting exactly on the cutoff is
/// abandoned rather than held. A statement that spelled `<` for `<=`, or that
/// dropped either arm, reads plausibly against any single one of those rows.
///
/// The whole answer is asserted rather than membership in it, which is what a
/// database per case buys and what `crates/nvs-stdlib/tests/queue.rs` cannot do
/// on a leg whose cases share one server.
#[test]
fn a_sqlite_roster_names_only_the_queues_holding_due_work() {
    let (worker, _reader) = two_connections("nvs-stdlib-queue-roster");

    push_on(&worker, "roster-due", NOW, 0, 0, None);
    push_on(&worker, "roster-later", NOW + 1, 0, 0, None);
    push_on(&worker, "roster-held", NOW, 1, 1, Some(NOW));
    push_on(&worker, "roster-abandoned", NOW, 1, 1, Some(NOW - WINDOW));

    assert_eq!(
        roster(&worker, NOW, NOW - WINDOW),
        vec![String::from("roster-abandoned"), String::from("roster-due"),],
        "the due pending row and the expired lease, and neither the job due later \
         nor the lease still inside its window"
    );
}

/// `rule:concurrency/delivery-is-at-least-once`'s visibility timeout from the
/// end that loses: a worker that overran the window reports into a row that is no
/// longer its own, and every write-back it can make matches nothing.
///
/// **All three write-backs, because they are one keying and not three.**
/// [`queue::SUCCEEDED_SQLITE`], [`queue::RETRY_SQLITE`] and both halves of
/// [`queue::DEAD_LETTER_SQLITE`] match `id` and `claimed_at` together, so a text
/// that dropped the lease from its `where` would finish, postpone or bury the
/// attempt that replaced this one — and would read plausibly on its own, because
/// the id is still the right id.
///
/// Nothing here mocks an expiry. The row really is re-claimed by the other
/// connection, which is what makes the first worker's lease stale, and the last
/// assertion is the same statement landing for the worker that does hold it: a
/// case asserting only the refusals would pass against a roster that had stopped
/// matching anything at all.
#[test]
fn a_sqlite_worker_that_overran_its_visibility_window_reports_nothing() {
    let (slow, fast) = two_connections("nvs-stdlib-queue-overrun");

    let id = push(&slow, NOW, 0, 0, None);
    let first = claim(&slow, NOW, NOW - WINDOW).expect("the pushed job is due");
    assert_eq!(int(&first[0]), id);

    let retaken = NOW + WINDOW;
    let second = claim(&fast, retaken, retaken - WINDOW).expect("the lease has expired");
    assert_eq!(
        int(&second[0]),
        id,
        "one row, claimed twice, and the second claim is the lease that counts"
    );

    assert_eq!(
        affected(
            &slow,
            queue::SUCCEEDED_SQLITE,
            vec![SqliteValue::Int(id), SqliteValue::Int(NOW)],
        ),
        0,
        "the overrun worker cannot finish an attempt it no longer owns"
    );
    assert_eq!(
        affected(
            &slow,
            queue::RETRY_SQLITE,
            vec![
                SqliteValue::Int(retaken + 250),
                SqliteValue::Text(queue::dead_errors(
                    None,
                    NOW,
                    "LogicError",
                    "the attempt threw",
                )),
                SqliteValue::Int(id),
                SqliteValue::Int(NOW),
            ],
        ),
        0,
        "nor put it back, which would move a running job's `run_at` out from under it"
    );
    dead_letter(&slow, id, NOW);
    assert_eq!(
        present(&slow, id),
        (1, 0),
        "nor bury it: both halves of the move are keyed on the lease, so the copy \
         and the delete refuse together"
    );

    assert_eq!(
        affected(
            &fast,
            queue::SUCCEEDED_SQLITE,
            vec![SqliteValue::Int(id), SqliteValue::Int(retaken)],
        ),
        1,
        "and the worker that does hold the lease reports exactly once"
    );
}
