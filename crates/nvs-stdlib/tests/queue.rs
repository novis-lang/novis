//! [ADR 0084](../../../docs/adr/0084-durable-background-jobs.md)'s queue
//! statements, run against a real PostgreSQL server rather than read.
//!
//! **These cases live in this crate and not in `nvs-db`, and one edge's
//! direction is the whole reason.** What they run is [`nvs_stdlib::queue`]'s
//! statement roster — § 2's schema has one home and that is it — and the only
//! thing that can run a statement is `nvs-db`'s `PgConn`. ADR 0132 § 1 fixes
//! which way that edge points: `nvs-stdlib` depends on `nvs-db` and never the
//! reverse, so a `crates/nvs-db/tests/queue.rs` would need a `use
//! nvs_stdlib::…` that closes a cycle in the workspace, and the alternative —
//! a copy of the statement text kept beside such a test — would assert over
//! the copy rather than over the statement a `push` actually issues. A test
//! target here sees both crates and needs neither compromise. `tools/db-matrix.py`'s
//! `SUITES` is what points this file at a server as well as `nvs-db`'s own.
//!
//! The skip rule is `nvs-db`'s, because it is the same harness: a process that
//! finds `NVS_DB_MATRIX_DRIVER` unset asserts nothing at all, so `python
//! tools/verify.py` is green on a machine with no containers and `python
//! tools/db-matrix.py` is what makes these assertions happen.
//! [`nvs_db::matrix`]'s module doc owns that rule and why a field that is *set
//! but unusable* panics instead.
//!
//! **Every case owns a queue name and shares nothing else.** Cargo runs them on
//! threads of one process against one database, and a queue is exactly the key
//! every statement here is claimed, counted and ordered by, so a name each is
//! the whole of the isolation — no case reads a row another wrote, and each
//! clears its own before it pushes so a re-run starts where the first run did.
//! The schema itself is the one thing they do share, and [`schema`] says why
//! that needs a `Once` rather than a call each.

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Once;
use std::time::{Duration, Instant};

use nvs_db::matrix::{self, Location, Server};
use nvs_db::{Driver, PgConn, PgTarget};
use nvs_stdlib::queue;

/// How long the whole of one handshake here may take.
///
/// The bound `crates/nvs-db/tests/handshake.rs` uses, for its reason: the
/// server is a container the harness may have started moments ago, and a
/// matrix leg that hangs reports nothing at all.
const DEADLINE: Duration = Duration::from_secs(10);

/// The ordinal `Core\Queue\State::Pending` is, as `MIGRATION`'s `state` column
/// holds it — ADR 0010's enums are their ordinal at runtime, so the enum and
/// the column are one representation rather than two.
const PENDING: &[u8] = b"0";

/// This process's PostgreSQL server, or `None` because nothing pointed it at
/// one.
///
/// Two shapes of `None` and neither is a failure: no harness at all, and a leg
/// testing one of the other four drivers, which runs every test in this file
/// including these.
fn postgres() -> Option<Server> {
    let endpoint = matrix::endpoint()?;
    if endpoint.driver != Driver::Postgres {
        return None;
    }
    let Location::Server(server) = endpoint.location else {
        unreachable!("SQLite is the only driver reached by path, and this is not it")
    };
    Some(server)
}

/// A connection to `server`, as a request that found the pool empty opens one.
fn open(server: &Server) -> PgConn {
    let addr: SocketAddr = (server.host.as_str(), server.port)
        .to_socket_addrs()
        .expect("the matrix host is an address")
        .next()
        .expect("the matrix host resolves to somewhere");
    let target = PgTarget {
        host: &server.host,
        user: &server.user,
        password: &server.password,
        database: &server.database,
        tls_ca_file: Some(server.ca.as_path()),
        time_zone: 0,
        statement_cache: 8,
    };

    PgConn::connect(addr, &target, Some(Instant::now() + DEADLINE))
        .expect("the matrix server accepts a handshake verified against its own anchor")
}

/// § 2's schema, applied once for this whole binary however many cases run.
///
/// **A `Once` rather than a step every case pays**, and not for the time:
/// `create table if not exists` is idempotent against a schema that is already
/// there and *not* safe against a second session running it in the same
/// moment, which answers a uniqueness error on the catalog rather than a
/// no-op. Cargo runs these cases on threads of one process, so that race is the
/// ordinary case here and not a rare one.
fn schema(server: &Server) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let mut conn = open(server);
        for step in queue::MIGRATION {
            apply(&mut conn, step.sql, &[]);
        }
    });
}

/// Whatever an earlier run left on `queue`, in both tables.
fn clear(conn: &mut PgConn, queue: &str) {
    let name = queue.as_bytes();
    apply(
        conn,
        "delete from nvs_jobs where queue = $1::text",
        &[Some(name)],
    );
    apply(
        conn,
        "delete from nvs_dead_jobs where queue = $1::text",
        &[Some(name)],
    );
}

/// Every row `sql` answers with, each column as the text PostgreSQL sent and
/// `None` for SQL `NULL`.
///
/// It drains to the end of the stream whatever it found, for the reason
/// `Core\Queue::push` does: the connection has to be back at a message boundary
/// before the next statement on it starts.
fn rows(conn: &mut PgConn, sql: &str, bound: &[Option<&[u8]>]) -> Vec<Vec<Option<String>>> {
    let mut answered = conn
        .query(sql, bound)
        .expect("the server ran the statement");
    // Taken before the first row, because a `PgRows` lends its columns and its
    // rows out of one borrow.
    let width = answered.columns().len();
    let mut all = Vec::new();
    while let Some(row) = answered
        .next_row()
        .expect("a row, or the end of the stream")
    {
        let mut one = Vec::with_capacity(width);
        for at in 0..width {
            one.push(
                row.column(at)
                    .expect("the row has that column")
                    .map(|body| {
                        String::from_utf8(body.to_vec()).expect("PostgreSQL's text format is UTF-8")
                    }),
            );
        }
        all.push(one);
    }
    all
}

/// One row's first column, as the text it arrived as.
fn one(conn: &mut PgConn, sql: &str, bound: &[Option<&[u8]>]) -> String {
    let mut answered = rows(conn, sql, bound);
    assert_eq!(answered.len(), 1, "`{sql}` answered with one row");
    answered
        .remove(0)
        .remove(0)
        .expect("the column is not null")
}

/// A statement run for its effect, answering the count it affected.
fn apply(conn: &mut PgConn, sql: &str, bound: &[Option<&[u8]>]) -> u64 {
    conn.execute_many(sql, &[bound])
        .expect("the server ran the statement")
}

/// One pending job on `queue`, due at `at`, answering the id `INSERT` returned.
///
/// `max_attempts` is the case's whole lever over § 6: a job pushed with `1` is
/// exhausted by its first claim, and one pushed with more is owed another.
fn push(conn: &mut PgConn, queue: &str, at: i64, max_attempts: &str) -> String {
    let at = millis(at);
    one(
        conn,
        queue::INSERT,
        &[
            // No dedupe key: `INSERT`'s `existing` arm is empty for a null
            // `$1`, which is the ordinary push.
            None,
            Some(queue.as_bytes()),
            Some(b"scripts/receipt.nvs"),
            Some(br#"{"order":7}"#),
            Some(PENDING),
            Some(max_attempts.as_bytes()),
            Some(b"1000"),
            Some(at.as_slice()),
            Some(at.as_slice()),
        ],
    )
}

/// One claim against `queue`, taken at `now`, returning jobs whose lease was
/// taken at or before `cutoff` as well as the ones nothing holds.
fn claim(conn: &mut PgConn, queue: &str, now: i64, cutoff: i64) -> Vec<Vec<Option<String>>> {
    let (now, cutoff) = (millis(now), millis(cutoff));
    rows(
        conn,
        queue::CLAIM,
        &[
            Some(queue.as_bytes()),
            Some(now.as_slice()),
            Some(cutoff.as_slice()),
        ],
    )
}

/// An epoch-millisecond instant as the text a `$n::bigint` placeholder is sent
/// as — `nvs-cli`'s worker binds every instant this way.
fn millis(at: i64) -> Vec<u8> {
    at.to_string().into_bytes()
}

/// `id`, `attempts` and `max_attempts`'s places in `CLAIM`'s `returning` list,
/// which that constant's doc owns.
const ID: usize = 0;
const ATTEMPTS: usize = 3;
const MAX_ATTEMPTS: usize = 4;

/// § 6: a job whose last attempt threw leaves `nvs_jobs` for `nvs_dead_jobs`
/// carrying what it threw, rather than being deleted or left claimed forever.
///
/// The three statements are the ones a worker runs and in the order it runs
/// them, so what is asserted is the roster agreeing with itself against a real
/// server: `INSERT`'s `returning` names the row `CLAIM` then takes, and the
/// lease `CLAIM` writes is what `DEAD_LETTER` is keyed on. `max_attempts = 1`
/// is what makes one claim an exhausted job — the returned `attempts` and
/// `max_attempts` say so, which is the condition `nvs-cli`'s `report` branches
/// on, asserted here so the case states why the move is the legal one rather
/// than arriving at it.
///
/// **The move is asserted from both sides.** § 6's claim is not that the row
/// appears in the dead-letter table but that it is *not discarded*: one
/// data-modifying CTE deletes it and inserts it, so a job left in neither table
/// would satisfy a case that only looked at where it went.
#[test]
fn an_exhausted_job_reaches_the_dead_letter_table_and_is_not_discarded() {
    const QUEUE: &str = "nvs-stdlib-tests-dead";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let now = queue::now_millis();
    let id = push(&mut conn, QUEUE, now, "1");

    let claimed = claim(&mut conn, QUEUE, now, now);
    assert_eq!(claimed.len(), 1, "the claim took the job that was pushed");
    let claimed = &claimed[0];
    assert_eq!(
        claimed[ID].as_deref(),
        Some(id.as_str()),
        "the row claimed is the row pushed"
    );
    assert_eq!(
        claimed[ATTEMPTS].as_deref(),
        Some("1"),
        "the claim counted this attempt"
    );
    assert_eq!(
        claimed[MAX_ATTEMPTS].as_deref(),
        Some("1"),
        "the attempt just counted was the job's last, so the job is exhausted"
    );

    let failed = now + 5;
    let errors = queue::dead_errors(now, "IOError", "the receipt service refused the order");
    let moved = apply(
        &mut conn,
        queue::DEAD_LETTER,
        &[
            Some(id.as_bytes()),
            // The lease, which is what `CLAIM` wrote to `claimed_at`.
            Some(millis(now).as_slice()),
            Some(millis(failed).as_slice()),
            Some(errors.as_bytes()),
        ],
    );
    assert_eq!(moved, 1, "one job moved");

    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_jobs where id = $1::bigint",
            &[Some(id.as_bytes())],
        ),
        "0",
        "the job left the jobs table rather than staying claimed"
    );

    let dead = rows(
        &mut conn,
        "select id, queue, errors, failed_at from nvs_dead_jobs where id = $1::bigint",
        &[Some(id.as_bytes())],
    );
    assert_eq!(
        dead.len(),
        1,
        "the job is in the dead-letter table, and so in one of the two rather than in neither"
    );
    let dead = &dead[0];
    assert_eq!(dead[0].as_deref(), Some(id.as_str()), "the job kept its id");
    assert_eq!(dead[1].as_deref(), Some(QUEUE), "the job kept its queue");
    assert_eq!(
        dead[2].as_deref(),
        Some(errors.as_str()),
        "the row carries the errors array whole, quoting and all"
    );
    assert_eq!(
        dead[3].as_deref(),
        Some(failed.to_string().as_str()),
        "the row says when the attempt ended, which is not when it started"
    );

    // The entry's own shape, read the way a reader of the column reads it: one
    // attempt, and what it threw as data rather than as text to grep.
    let entry: serde_json::Value = serde_json::from_str(dead[2].as_deref().expect("not null"))
        .expect("`errors` is a JSON document");
    let entry = entry.as_array().expect("`errors` is an array");
    assert_eq!(
        entry.len(),
        1,
        "one entry, which is the depth this schema pays for"
    );
    assert_eq!(entry[0]["class"], "IOError");
    assert_eq!(entry[0]["message"], "the receipt service refused the order");
    assert_eq!(entry[0]["at"], now);
}

/// § 4: a job whose worker went away is claimable again once its lease is older
/// than the visibility bound, and not one moment before.
///
/// **The bound is asserted on both sides**, because either half alone passes
/// against a statement that has stopped asking: a claim whose `cutoff` is older
/// than the lease answers nothing at all — the job is *held*, which is what
/// makes delivery at-least-once rather than at-most-one-worker-at-a-time — and
/// the same claim a millisecond past it answers with the same job. That second
/// claim is a second attempt and the column says so, which is what ties the
/// timeout to § 6's ladder: an abandoned job spends attempts exactly as a
/// throwing one does, so a worker that keeps dying cannot retry a job forever.
#[test]
fn a_visibility_timeout_returns_an_abandoned_job_to_the_queue() {
    const QUEUE: &str = "nvs-stdlib-tests-visibility";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let now = queue::now_millis();
    let id = push(&mut conn, QUEUE, now, "3");

    let first = claim(&mut conn, QUEUE, now, now);
    assert_eq!(first.len(), 1, "a pending job due now is claimable");
    assert_eq!(first[0][ID].as_deref(), Some(id.as_str()));
    assert_eq!(first[0][ATTEMPTS].as_deref(), Some("1"));

    // Held: the lease was taken at `now`, and this claim is willing to take
    // over only a lease from a millisecond earlier.
    let held = claim(&mut conn, QUEUE, now + 1, now - 1);
    assert!(
        held.is_empty(),
        "a job inside its visibility window is claimed by nobody else"
    );

    // Abandoned: the same claim, willing to take over a lease that old.
    let again = claim(&mut conn, QUEUE, now + 2, now);
    assert_eq!(
        again.len(),
        1,
        "a lease older than the bound returns the job to the queue"
    );
    assert_eq!(
        again[0][ID].as_deref(),
        Some(id.as_str()),
        "the job returned is the abandoned one and not a second row"
    );
    assert_eq!(
        again[0][ATTEMPTS].as_deref(),
        Some("2"),
        "the retaken job spent an attempt, so an abandoning worker cannot loop forever"
    );
    assert_eq!(
        one(
            &mut conn,
            "select claimed_at from nvs_jobs where id = $1::bigint",
            &[Some(id.as_bytes())],
        ),
        (now + 2).to_string(),
        "the lease moved to the claim that took it over"
    );
}

/// § 4: two workers claiming at once take different jobs, because the claim
/// skips a row another transaction holds instead of queueing behind it.
///
/// **This is the property `for update skip locked` exists for, and the only way
/// to see it is a lock that outlives a statement.** Each claim is autocommitted
/// on its own, so a second connection can only meet the first one's row lock
/// while the first is inside an open transaction — which is what this case
/// opens by hand. Without `skip locked` the second claim would *block* on the
/// held row rather than answer wrongly, so the case cannot assert its way to a
/// verdict: `statement_timeout` is what turns that hang into a failure, and it
/// is the reason this file's one hand-written `set` exists. A matrix leg that
/// hangs reports nothing at all.
///
/// The name says *every backend that has it* and this asserts one, because
/// PostgreSQL is the only driver with a statement path at all; the second
/// backend brings its own claim statement and answers here beside this one.
#[test]
fn claiming_is_skip_locked_shaped_on_every_backend_that_has_it() {
    const QUEUE: &str = "nvs-stdlib-tests-skip-locked";
    /// Long enough that a loaded container is not mistaken for a lock, short
    /// enough that a blocked claim is a failure rather than a hung leg.
    const BLOCKED: &str = "set statement_timeout = 3000";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut first = open(&server);
    clear(&mut first, QUEUE);

    let now = queue::now_millis();
    // Two jobs, due in the order `CLAIM`'s `order by run_at, id` takes them.
    let older = push(&mut first, QUEUE, now, "3");
    let newer = push(&mut first, QUEUE, now + 1, "3");

    first
        .begin(None, false)
        .expect("the server opened a transaction");
    let held = claim(&mut first, QUEUE, now + 2, now);
    assert_eq!(
        held.len(),
        1,
        "the first worker claimed inside its transaction"
    );
    assert_eq!(
        held[0][ID].as_deref(),
        Some(older.as_str()),
        "the claim took the oldest due job"
    );

    let mut second = open(&server);
    apply(&mut second, BLOCKED, &[]);
    let took = claim(&mut second, QUEUE, now + 2, now);
    assert_eq!(
        took.len(),
        1,
        "the second worker was answered rather than left waiting on a row it cannot have"
    );
    assert_eq!(
        took[0][ID].as_deref(),
        Some(newer.as_str()),
        "it skipped the locked job and took the next due one, which is the whole of `skip locked`"
    );

    first.commit().expect("the server closed the transaction");

    // Both jobs are claimed and neither was claimed twice, which is the
    // property stated as a count rather than read off the two rows above.
    assert_eq!(
        one(
            &mut second,
            "select count(*) from nvs_jobs where queue = $1::text and state = 1 and attempts = 1",
            &[Some(QUEUE.as_bytes())],
        ),
        "2",
        "two jobs, one attempt each, and no job claimed by both workers"
    );
}
