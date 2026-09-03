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

/// The ordinal `Core\Queue\State::Pending` is, as `MIGRATION_POSTGRES`'s `state` column
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
        for step in queue::MIGRATION_POSTGRES {
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

/// § 3's `insert into orders …`: the application table the two transactional
/// cases write beside the job, created once for this binary and emptied of
/// `queue`'s rows.
///
/// **The whole point of the table is that it is not the queue's.** § 3's
/// property is that a job and the write that caused it commit together, so a
/// case holding only the job would still pass against a `push` that quietly
/// enqueued on a connection of its own — the one thing § 3 exists to forbid.
/// It carries a `queue` column for the reason every other row here does: it is
/// what one case's rows are told apart from another's by.
///
/// The `Once` is [`schema`]'s and for [`schema`]'s reason, and the `delete` is
/// [`clear`]'s half for the one table § 2 does not own.
fn orders(server: &Server, conn: &mut PgConn, queue: &str) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let mut own = open(server);
        apply(
            &mut own,
            "create table if not exists nvs_stdlib_tests_orders \
             (id bigserial primary key, queue text not null)",
            &[],
        );
    });
    apply(
        conn,
        "delete from nvs_stdlib_tests_orders where queue = $1::text",
        &[Some(queue.as_bytes())],
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

/// One pending job on `queue`, due at `at`, answering the id `INSERT_POSTGRES` returned.
///
/// `max_attempts` is the case's whole lever over § 6: a job pushed with `1` is
/// exhausted by its first claim, and one pushed with more is owed another.
fn push(conn: &mut PgConn, queue: &str, at: i64, max_attempts: &str) -> String {
    let at = millis(at);
    one(
        conn,
        queue::INSERT_POSTGRES,
        &[
            // No dedupe key: `INSERT_POSTGRES`'s `existing` arm is empty for a null
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
        queue::CLAIM_POSTGRES,
        &[
            Some(queue.as_bytes()),
            Some(now.as_slice()),
            Some(cutoff.as_slice()),
        ],
    )
}

/// How many jobs and how many orders `queue` has, read in one statement.
///
/// **One read rather than two**, because § 3's property is about the pair: "one
/// exists without the other" is a state two statements can each miss, since
/// whatever happened between them is a moment neither one looked at.
fn landed(conn: &mut PgConn, queue: &str) -> (String, String) {
    let mut answered = rows(
        conn,
        "select (select count(*) from nvs_jobs where queue = $1::text), \
                (select count(*) from nvs_stdlib_tests_orders where queue = $1::text)",
        &[Some(queue.as_bytes())],
    );
    assert_eq!(answered.len(), 1, "a count answers with one row");
    let mut answered = answered.remove(0);
    let orders = answered.remove(1).expect("a count is not null");
    let jobs = answered.remove(0).expect("a count is not null");
    (jobs, orders)
}

/// An epoch-millisecond instant as the text a `$n::bigint` placeholder is sent
/// as — `nvs-cli`'s worker binds every instant this way.
fn millis(at: i64) -> Vec<u8> {
    at.to_string().into_bytes()
}

/// `id`, `attempts` and `max_attempts`'s places in `CLAIM_POSTGRES`'s `returning` list,
/// which that constant's doc owns.
const ID: usize = 0;
const ATTEMPTS: usize = 3;
const MAX_ATTEMPTS: usize = 4;

/// § 6: a job whose last attempt threw leaves `nvs_jobs` for `nvs_dead_jobs`
/// carrying what it threw, rather than being deleted or left claimed forever.
///
/// The three statements are the ones a worker runs and in the order it runs
/// them, so what is asserted is the roster agreeing with itself against a real
/// server: `INSERT_POSTGRES`'s `returning` names the row `CLAIM_POSTGRES` then takes, and the
/// lease `CLAIM_POSTGRES` writes is what `DEAD_LETTER_POSTGRES` is keyed on. `max_attempts = 1`
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
        queue::DEAD_LETTER_POSTGRES,
        &[
            Some(id.as_bytes()),
            // The lease, which is what `CLAIM_POSTGRES` wrote to `claimed_at`.
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
    // Two jobs, due in the order `CLAIM_POSTGRES`'s `order by run_at, id` takes them.
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

/// § 3: a job pushed inside a transaction becomes durable with the write that
/// caused it, in the one moment that transaction commits.
///
/// **The order row is what makes this a test of § 3 rather than of `INSERT_POSTGRES`.**
/// The property is that there is no window in which the job exists without the
/// write or the write without the job, so a case holding only the job would
/// pass just as well against a design that enqueued over a connection of its
/// own — which is the design § 3 rejects. Both rows go in over the one
/// connection the transaction is open on, and both are counted after it closes.
///
/// The `push` is [`queue::INSERT_POSTGRES`] and nothing about it changes inside a
/// transaction: § 3's enlistment is not a mode the statement is issued in but
/// the plain consequence of running it on a connection that is already in one,
/// which is why the design has no outbox in it.
#[test]
fn an_enqueue_commits_with_the_write_that_made_it() {
    const QUEUE: &str = "nvs-stdlib-tests-commit";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);
    orders(&server, &mut conn, QUEUE);

    let now = queue::now_millis();
    assert_eq!(
        landed(&mut conn, QUEUE),
        ("0".to_owned(), "0".to_owned()),
        "the case starts where an earlier run of it started"
    );

    conn.begin(None, false)
        .expect("the server opened a transaction");
    assert_eq!(conn.depth(), 1, "the connection is inside one transaction");

    let order = one(
        &mut conn,
        "insert into nvs_stdlib_tests_orders (queue) values ($1::text) returning id",
        &[Some(QUEUE.as_bytes())],
    );
    let id = push(&mut conn, QUEUE, now, "3");

    conn.commit().expect("the server closed the transaction");
    assert_eq!(conn.depth(), 0, "the transaction is over");

    assert_eq!(
        landed(&mut conn, QUEUE),
        ("1".to_owned(), "1".to_owned()),
        "the job and the write that caused it are both durable"
    );
    let job = rows(
        &mut conn,
        "select id, state, attempts from nvs_jobs where queue = $1::text",
        &[Some(QUEUE.as_bytes())],
    );
    assert_eq!(
        job[0][0].as_deref(),
        Some(id.as_str()),
        "the durable job is the row `INSERT_POSTGRES` answered with inside the transaction"
    );
    assert_eq!(
        job[0][1].as_deref(),
        Some(std::str::from_utf8(PENDING).expect("an ordinal is ASCII")),
        "it is claimable, so a worker that starts now runs it"
    );
    assert_eq!(
        job[0][2].as_deref(),
        Some("0"),
        "nothing has attempted it yet"
    );
    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_stdlib_tests_orders where id = $1::bigint",
            &[Some(order.as_bytes())],
        ),
        "1",
        "the order the job was pushed for is the one that committed"
    );
}

/// § 3, from the other side: the same push under a `ROLLBACK` leaves no job at
/// all, so the job was never enqueued rather than enqueued and then compensated.
///
/// **The bound is asserted from both sides for the reason the visibility one
/// is**: a `push` that failed for any reason of its own would satisfy a case
/// that only ever rolled back, and one that enqueued outside the transaction
/// would satisfy a case that only ever committed. Neither half alone says
/// anything about the window between them.
///
/// The id is the tell that this is a rollback and not a failure. `INSERT_POSTGRES`
/// answered with one inside the transaction — a sequence does not roll back, so
/// the number was really allocated and really handed out — and what is gone
/// afterwards is the row, which is the only thing § 3 ever promised.
#[test]
fn a_rolled_back_write_leaves_no_job() {
    const QUEUE: &str = "nvs-stdlib-tests-rollback";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);
    orders(&server, &mut conn, QUEUE);

    let now = queue::now_millis();

    conn.begin(None, false)
        .expect("the server opened a transaction");
    apply(
        &mut conn,
        "insert into nvs_stdlib_tests_orders (queue) values ($1::text)",
        &[Some(QUEUE.as_bytes())],
    );
    let id = push(&mut conn, QUEUE, now, "3");
    assert_eq!(
        landed(&mut conn, QUEUE),
        ("1".to_owned(), "1".to_owned()),
        "both rows are there for the transaction that wrote them"
    );

    conn.roll_back().expect("the server undid the transaction");
    assert_eq!(conn.depth(), 0, "the transaction is over");

    assert_eq!(
        landed(&mut conn, QUEUE),
        ("0".to_owned(), "0".to_owned()),
        "neither the job nor the write that caused it survived"
    );
    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_jobs where id = $1::bigint",
            &[Some(id.as_bytes())],
        ),
        "0",
        "the id `INSERT_POSTGRES` answered with names nothing, so no worker can ever claim it"
    );
}

/// § 6: a failed attempt arms the job for a later one, the ladder it climbs is
/// finite, and two jobs failing in the same moment do not come back in the same
/// one.
///
/// **The ladder's arithmetic is `a_retry_is_exponential_jittered_and_capped`'s
/// and is not asserted again here.** [`queue::retry_at`] is a pure function and
/// that unit test walks twelve rungs of it. What this case can say and that one
/// cannot is that the delay is a wait the *server* enforces:
/// [`queue::RETRY_POSTGRES`] writes it to `run_at`, and a claim a millisecond
/// earlier answers with nothing at all.
///
/// **Bounded is asserted by counting the claims rather than by reading the last
/// row.** Two jobs at two attempts each is four claims and then a queue that
/// answers nothing however far ahead the worker asks — a job armed once too
/// often would answer a fifth, and a case reading only the row in front of it
/// would not notice. The jitter is asserted as the thing § 6 wants it for: the
/// two delays *differ*, which is what stops a hundred jobs retrying an endpoint
/// that came back from landing on it together.
#[test]
fn retries_are_bounded_and_backoff_is_jittered() {
    const QUEUE: &str = "nvs-stdlib-tests-retry";
    /// The `backoff_ms` [`push`] writes, so the first rung is this file's to
    /// compute as well as the server's.
    const BACKOFF: i64 = 1_000;
    /// Two attempts each — the smallest number with a retry in it.
    const CAP: &str = "2";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let now = queue::now_millis();
    let pushed = [
        push(&mut conn, QUEUE, now, CAP),
        push(&mut conn, QUEUE, now, CAP),
    ];
    let mut claims = 0;

    // Each job's first attempt, which throws. `RETRY` is the write-back that
    // arms it for the next one at the instant `retry_at` put it on.
    let mut due = Vec::new();
    for expected in &pushed {
        let claimed = claim(&mut conn, QUEUE, now, now);
        assert_eq!(claimed.len(), 1, "a job due now is claimable");
        claims += 1;
        assert_eq!(
            claimed[0][ID].as_deref(),
            Some(expected.as_str()),
            "the claim took the jobs in the order they were pushed"
        );
        assert_eq!(claimed[0][ATTEMPTS].as_deref(), Some("1"));
        assert_eq!(
            claimed[0][MAX_ATTEMPTS].as_deref(),
            Some(CAP),
            "one attempt of two, so this job is owed another"
        );

        let id: i64 = expected
            .parse()
            .expect("`INSERT_POSTGRES` answered with an id");
        let at = queue::retry_at(now, 1, BACKOFF, id);
        assert_eq!(
            apply(
                &mut conn,
                queue::RETRY_POSTGRES,
                &[
                    Some(expected.as_bytes()),
                    // The lease this claim wrote, which is what the write-back
                    // is keyed on.
                    Some(millis(now).as_slice()),
                    Some(millis(at).as_slice()),
                ],
            ),
            1,
            "the write-back matched the claim's own lease"
        );
        due.push(at);
    }

    assert_ne!(
        due[0], due[1],
        "two jobs that failed in one moment are armed for two different ones"
    );
    for at in &due {
        assert!(
            (now + BACKOFF / 2..=now + BACKOFF).contains(at),
            "the first rung is one base, jittered inside its own top half"
        );
    }

    let armed = rows(
        &mut conn,
        "select id, state, attempts, run_at, claimed_at from nvs_jobs \
         where queue = $1::text order by id",
        &[Some(QUEUE.as_bytes())],
    );
    assert_eq!(armed.len(), 2, "both jobs are back in the queue, not gone");
    for (row, (id, at)) in armed.iter().zip(pushed.iter().zip(due.iter())) {
        assert_eq!(row[0].as_deref(), Some(id.as_str()));
        assert_eq!(
            row[1].as_deref(),
            Some(std::str::from_utf8(PENDING).expect("an ordinal is ASCII")),
            "the job is pending again rather than held by the worker that failed it"
        );
        assert_eq!(
            row[2].as_deref(),
            Some("1"),
            "the attempt it spent is still counted against it"
        );
        assert_eq!(
            row[3].as_deref(),
            Some(at.to_string().as_str()),
            "the server holds the delay `retry_at` computed, to the millisecond"
        );
        assert_eq!(
            row[4], None,
            "nothing holds a job that is waiting out its backoff"
        );
    }

    // Not yet: the backoff is a wait the claim enforces, not a number written
    // beside a row that is claimable anyway.
    let soonest = *due.iter().min().expect("both jobs were armed");
    assert!(
        claim(&mut conn, QUEUE, soonest - 1, soonest - 1).is_empty(),
        "a job inside its backoff is claimed by nobody"
    );

    // Each job's second attempt, which is the last one it has. The jitter is
    // why the order here is nobody's to predict, so the ids are collected and
    // compared as a set rather than one at a time.
    let latest = *due.iter().max().expect("both jobs were armed");
    let mut exhausted = Vec::new();
    for _ in 0..pushed.len() {
        let claimed = claim(&mut conn, QUEUE, latest, latest);
        assert_eq!(
            claimed.len(),
            1,
            "the armed job came back once its delay was out"
        );
        claims += 1;
        let row = &claimed[0];
        assert_eq!(
            row[ATTEMPTS], row[MAX_ATTEMPTS],
            "the attempt just counted was this job's last, which is what `report` branches on"
        );
        let id = row[ID]
            .as_deref()
            .expect("a claimed row has an id")
            .to_owned();
        assert_eq!(
            apply(
                &mut conn,
                queue::DEAD_LETTER_POSTGRES,
                &[
                    Some(id.as_bytes()),
                    Some(millis(latest).as_slice()),
                    Some(millis(latest + 1).as_slice()),
                    Some(
                        queue::dead_errors(latest, "IOError", "the endpoint is still down")
                            .as_bytes()
                    ),
                ],
            ),
            1,
            "the exhausted job moved rather than being armed again"
        );
        exhausted.push(id);
    }
    exhausted.sort();
    let mut expected = pushed.to_vec();
    expected.sort();
    assert_eq!(
        exhausted, expected,
        "the two jobs the second round claimed are the two that were pushed"
    );

    // The ladder ends, stated as the queue having nothing left however far
    // ahead the worker asks rather than as the state of the last row seen.
    let far = latest + BACKOFF * 64;
    assert!(
        claim(&mut conn, QUEUE, far, far).is_empty(),
        "an exhausted job is not armed again, so a job's retries are its `max_attempts` and no more"
    );
    assert_eq!(
        claims, 4,
        "two jobs at two attempts each, counted rather than read off the last row"
    );
    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_dead_jobs where queue = $1::text and attempts = 2",
            &[Some(QUEUE.as_bytes())],
        ),
        "2",
        "both jobs are dead-lettered having spent every attempt they were given"
    );
}
