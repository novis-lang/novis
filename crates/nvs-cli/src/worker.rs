//! The in-process worker `rule:core-classes/queue-storage-is-a-table`'s `[queue] workers` starts: a task beside the script's, on
//! the scheduler this run already turns.
//!
//! ## Why a task, and why this crate
//!
//! A worker waits on a socket and then waits for work, and both waits have to hand the core back.
//! `nvs_host`'s parking stream is what does that — but only from inside a task; off one it blocks
//! the calling thread (`nvs_host::net`'s § *Off a core, it blocks*). `nvs queue migrate` opens its
//! connection on the main thread for exactly the opposite reason ([`crate::queue`]'s module doc): a
//! command has nothing to yield *to*. A worker does — the program's own task, which parks on every
//! `Core\Time::sleep` and every statement it runs — so a worker that blocked the thread would stop
//! the program it is claiming for, and the fixture that waits for a claim would wait forever.
//!
//! That is also why this lives in `nvs-cli` rather than beside `Core\Queue` in `nvs-stdlib`:
//! [`nvs_host::Scheduler`] is created here, and a task is a thing only its owner can spawn. What is
//! *shared* is the schema — [`nvs_stdlib::queue::QUEUES_POSTGRES`],
//! [`nvs_stdlib::queue::CLAIM_POSTGRES`] and the MySQL dialect written beside each of them are read
//! from the module that owns § 2's tables, so a worker decides nothing about them and nothing about
//! which dialect a driver gets.
//!
//! [`nvs_runtime::TaskRoot::Worker`] and not `Request`, per
//! `rule:http-server/containment-does-not-end-at-the-helper`:
//! there is no request beneath a worker to charge a panic to.
//!
//! ## Why it is stopped rather than left running
//!
//! [`nvs_host::run_until_idle`] returns when nothing is runnable, and a worker polling for work is
//! always runnable — so a CLI run with one would never end. The script's own task therefore sets
//! [`Workers::stop`] on its way out and each worker reads it at the top of its turn. A flag rather
//! than a cancellation because every wait here is bounded and short: the tail a run pays after its
//! script returns is one [`IDLE_TURN`] in the ordinary case, one statement's round trip while a
//! claim is in flight, and at worst one [`CONNECT_DEADLINE`] for a worker still shaking hands with
//! a server that is not answering.
//!
//! ## Why the grants are the run's own
//!
//! A claimed job is § 5's root isolate, and an isolate is reached through
//! `rule:security/capability-check-at-the-door`'s
//! spawn door like any other — [`nvs_runtime::script::resolve`] asks `script.spawn` with the job's
//! path as its scope. That question is asked of the *context*, and a worker's context is not the
//! script's, so each one is handed the same configuration snapshot the run resolved at boot. § 5's
//! "grants narrowed from those recorded at enqueue" is the narrower rule and § 2's schema has no
//! column to record them in yet; what is here is the deployment's own configuration, which is the
//! ceiling that narrowing would sit under.
//!
//! ## What it spends
//!
//! One connection per worker in whichever driver `[db.<name>]` names, opened once and held for the
//! run, plus the roster statement each idle turn and nothing beyond it, since a roster with no due
//! work claims nothing. A turn that *does* claim costs a transaction's worth of round trips on
//! MySQL and MariaDB where it costs a single statement on PostgreSQL, which is
//! [`nvs_stdlib::queue::Split`]'s trade and not this module's: the claim's `select` and its
//! `update` are one moment or they are nothing, and a backend without the construct that makes them
//! one statement pays a transaction for the same property. It is
//! `workers` connections against the deployment's `max_connections` and the operator wrote the
//! number; `rule:security/db-pool-reset-is-a-boundary`'s pool is deliberately not involved, because a pool exists to be handed
//! between requests and this connection belongs to one task for its whole life. A turn that claims
//! spends one isolate on top of that — its own arena and budget, sharing only the compiled unit,
//! which [`crate::script`]'s cache holds for the run so a queue draining ten jobs off one script
//! compiles it once.
//!
//! ## What a run pays for a worker it never gives a turn to
//!
//! Nothing, and that is why `main` spawns the workers *after* the script's own task rather than
//! before it. A worker's first act is a database handshake, a scheduler's run queue is FIFO, and a
//! CLI program that never parks has already finished by the time anything spawned after it is
//! polled — so under the other order every `nvs run` of every program waits out one PostgreSQL
//! handshake before it can exit, which on this repository's own configuration dominates an empty
//! program's whole start and puts it the wrong side of stage 1's `a warm-cache CLI start stays
//! under 10ms`. [`Workers`] is therefore created before either task and read here before
//! [`open`], not only at the top of a turn.
//!
//! ## Known gap
//!
//! **A dead-lettered row carries the last attempt's error and no earlier one's.** [`report`] writes
//! every attempt back — `Succeeded`, back to `Pending` on § 6's ladder, or out of `nvs_jobs` and
//! into `nvs_dead_jobs` where the job has used its last attempt — but § 2's jobs table has nowhere
//! to keep what an earlier attempt threw, so the `errors` array § 6 asks for is one entry deep and
//! every attempt before the last is visible only on this worker's standard error.
//! [`nvs_stdlib::queue::schema`]'s own doc owns that decision and what a deeper array would cost.
//!

use std::cell::Cell;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nvs_config::queue::QueueBounds;
use nvs_config::tree::Database;
use nvs_runtime::host::{Woken, with_current};

/// How long a worker's own handshake may take.
///
/// Much shorter than [`crate::queue`]'s own connect deadline, and for the opposite reason: that
/// command exists to reach the server and has nothing else to do, while a worker is a passenger on
/// a run whose script may finish in milliseconds. An unreachable server costs the run this much and
/// then stops costing it anything.
const CONNECT_DEADLINE: Duration = Duration::from_secs(2);

/// How long a worker waits after a turn that found no due work.
///
/// Short because the wait is what the program's own polling is waiting on, and cheap because it is
/// a park rather than a spin — the core runs the script while a worker holds this.
const IDLE_TURN: Duration = Duration::from_millis(10);

/// The switch that stops every worker a run started.
///
/// One flag for all of them rather than one each: they stop together, at the same moment and for
/// the same reason. It is [`Clone`] because both ends hold it — the script's task sets it on its
/// way out and every worker reads it — and a clone is the same flag, not a second one.
///
/// Built by the caller rather than by [`start`], because the two are spawned in the order the
/// module doc's *What a run pays* section fixes: the script's task first, and it needs this in its
/// body before any worker exists.
#[derive(Clone)]
pub(crate) struct Workers {
    /// An [`Rc`] and a [`Cell`] because both ends are tasks on one core — there is no thread here
    /// to synchronize with.
    stop: Rc<Cell<bool>>,
}

impl Workers {
    /// A fresh switch, in the position every worker keeps running in.
    pub(crate) fn new() -> Self {
        Self {
            stop: Rc::new(Cell::new(false)),
        }
    }

    /// Tells every worker this run started to finish its turn and return.
    pub(crate) fn stop(&self) {
        self.stop.set(true);
    }
}

/// Spawns `bounds.workers` worker tasks onto `sched`, each claiming out of `[db.<name>]`.
///
/// Nothing runs here — [`nvs_host::Scheduler::spawn`] only queues — so the caller is free to
/// install the reactor afterwards. `snapshot` is the configuration the run resolved at boot, and
/// each worker's context is given it for the reason the module doc's *Why the grants* section
/// owns: a job's isolate is resolved against the context that runs it.
pub(crate) fn start(
    sched: &mut nvs_host::Scheduler,
    workers: &Workers,
    bounds: &QueueBounds,
    block: &Database,
    snapshot: &Arc<nvs_config::Snapshot>,
) {
    for _ in 0..bounds.workers {
        let stop = Rc::clone(&workers.stop);
        let name = bounds.connection.clone();
        // Cloned rather than borrowed because a task's body is `'static`, and cloned per worker
        // rather than shared because a `Database` is a handful of strings read once at connect.
        let block = block.clone();
        let visibility = bounds.visibility;
        let mut ctx = nvs_runtime::Ctx::stdout();
        ctx.set_config(Arc::clone(snapshot));
        sched.spawn(ctx, nvs_runtime::TaskRoot::Worker, move |ctx| {
            claim_until_stopped(ctx, &stop, &name, &block, visibility);
        });
    }
}

/// One worker's whole life: open the connection, then take turns until the run ends.
///
/// A statement that fails ends the worker rather than being retried. A connection is only usable at
/// a message boundary — on either driver — and a failed statement is not one, so the honest recovery
/// is a new connection, which is the next run's, since this one is by then within a few milliseconds
/// of its own end.
fn claim_until_stopped(
    ctx: &mut nvs_runtime::Ctx,
    stop: &Cell<bool>,
    name: &str,
    block: &Database,
    visibility: Duration,
) {
    // Asked before the connection is opened and not only at the top of a turn: this task is
    // spawned after the script's, so an ordinary CLI run has already finished by the time a worker
    // is first polled, and the module doc's *What a run pays* section is what that buys.
    if stop.get() {
        return;
    }
    let Some(mut conn) = open(name, block) else {
        return;
    };
    // Saturating rather than wrapping for a `visibility` no operator would write: the clamp makes
    // every job's claim eligible again immediately, which is a busy worker, where the wrap would
    // make it eligible never.
    let window = i64::try_from(visibility.as_millis()).unwrap_or(i64::MAX);
    while !stop.get() {
        match turn(ctx, &mut conn, window) {
            // Something was claimed, so the roster may still hold more: turn again without
            // waiting, and the queue that answered drops out of the next roster by itself, because
            // a row this turn claimed is inside its visibility window.
            Ok(true) => {}
            Ok(false) => {
                if nap() == Woken::Cancelled {
                    return;
                }
            }
            Err(_) => return,
        }
    }
}

/// One turn: which queues have due work, then one claim against each.
///
/// The roster is asked first for the reason [`nvs_stdlib::queue::QUEUES_POSTGRES`] owns — § 2 names
/// no queues, so the table is the only place they are written down — and the two instants are
/// computed once here so that every claim in this turn judges due-ness against the same moment.
fn turn(ctx: &mut nvs_runtime::Ctx, conn: &mut Wire, window: i64) -> io::Result<bool> {
    let now = nvs_stdlib::queue::now_millis();
    let cutoff = now.saturating_sub(window);
    let mut claimed = false;
    for queue in roster(conn, now, cutoff)? {
        if let Some(job) = claim(conn, &queue, now, cutoff)? {
            // Run before the next queue is claimed against, rather than after the roster has been
            // walked: a claim this worker is holding is a job nothing else may take, so the
            // shortest time between the two is the one that costs a fleet the least. The write-back
            // rides with it for the same reason — the row is released by [`report`] and not by the
            // end of the turn.
            let failure = run(ctx, &job);
            report(conn, &job, now, failure.as_ref())?;
            claimed = true;
        }
    }
    Ok(claimed)
}

/// The queues holding work this worker could take, as [`nvs_stdlib::queue::QUEUES_POSTGRES`] and
/// [`nvs_stdlib::queue::QUEUES_MYSQL`] answer it.
///
/// **One binding for both dialects**, because both texts name the same values in the same
/// order and neither is a [`nvs_stdlib::queue::Split`] — so what the branch below is about is the
/// walk over the answer and never the parameters. Where a dialect *does* reorder its values, the
/// caller reconciles it at the one site that already had to branch: [`report`]'s retry.
fn roster(conn: &mut Wire, now: i64, cutoff: i64) -> io::Result<Vec<String>> {
    let sending = [Some(millis(now)), Some(millis(cutoff))];
    let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();
    match conn.dialect() {
        Dialect::Postgres(postgres) => postgres_roster(postgres, &bound),
        Dialect::Framed(mut framed) => framed_roster(&mut framed, &bound),
    }
}

/// [`roster`] over the extended-query protocol: one statement, and its one column.
fn postgres_roster(
    postgres: &mut nvs_db::PgConn,
    bound: &[Option<&[u8]>],
) -> io::Result<Vec<String>> {
    let mut answered = postgres.query(nvs_stdlib::queue::QUEUES_POSTGRES, bound)?;
    // Taken before the first row because a `PgRows` lends its columns and its rows out of one
    // borrow, which is how `Core\Queue::status` reads its own single column.
    let columns: Vec<nvs_db::PgColumn> = answered.columns().to_vec();
    let mut names = Vec::new();
    while let Some(row) = answered.next_row()? {
        let Some(column) = columns.first() else {
            continue;
        };
        if let nvs_db::PgScalar::Text(name) = column.scalar(row.column(0)?)? {
            names.push(name.into_owned());
        }
    }
    Ok(names)
}

/// [`roster`] over MySQL's framing, which MariaDB runs unchanged.
///
/// **The same walk as [`postgres_roster`] and deliberately not shared with it**, for the reason
/// `Core\Db`'s own two readers give: a value is read here against the column definition it arrived
/// under, where there it is a body the column decodes, and the two `Scalar` enums are two sets of
/// rows. A trait over that would be more abstraction than the concrete lines it would stand for.
fn framed_roster(framed: &mut Framed<'_>, bound: &[Option<&[u8]>]) -> io::Result<Vec<String>> {
    let mut answered = framed.query(nvs_stdlib::queue::QUEUES_MYSQL, bound)?;
    // Described before the first row for [`postgres_roster`]'s reason, and needed whatever that
    // reason: `nvs_db::mysql::scalar` reads a value against the definition it arrived under.
    let columns = answered.columns().to_vec();
    let mut names = Vec::new();
    while let Some(row) = answered.next_row()? {
        let (Some(column), Some(body)) = (columns.first(), row.value(0)) else {
            continue;
        };
        if let nvs_db::MySqlScalar::Text(name) = nvs_db::mysql::scalar(column, body)? {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

/// What a worker reads off [`nvs_stdlib::queue::CLAIM_POSTGRES`]'s `returning` list, and what running one
/// and reporting it needs.
///
/// Every column of that list: what to run, and what § 6's ladder is judged against, which
/// [`report`] does the moment [`run`] returns.
struct Job {
    /// The primary key, which is what the write-back names the row by.
    id: i64,
    /// The file § 1 says a job names. `spawn script`'s own spelling, resolved the same way.
    script: String,
    /// The `args` column as it is stored: the document `Core\Queue::push` encoded, or `None` for a
    /// job pushed without one. Decoded at the last moment, in [`run`], so a job whose script is
    /// refused never pays for it.
    args: Option<String>,
    /// Attempts made *including this one* — [`nvs_stdlib::queue::CLAIM_POSTGRES`] increments the column in
    /// the same statement it returns it from, so a job being run for the first time reads `1`.
    attempts: i64,
    /// § 6's bound on the above, as `Core\Queue::push` recorded it from `{maxAttempts: …}` or from
    /// `[queue] max_attempts`.
    max_attempts: i64,
    /// The base delay of § 6's ladder for this job, in milliseconds.
    ///
    /// `i64` for every integer here, whatever width the DDL gave the column: they arrive as
    /// [`nvs_db::PgScalar::Int`], which is one variant for `smallint`, `integer` and `bigint`
    /// alike, so narrowing here would be a conversion this crate has no use for.
    backoff_ms: i64,
}

/// One claim against one queue, answering with the row it took.
///
/// **The same values in the same order for both dialects**, as [`roster`]'s are:
/// [`nvs_stdlib::queue::CLAIM_MYSQL`]'s `select` names the queue and the two instants exactly where
/// [`nvs_stdlib::queue::CLAIM_POSTGRES`] names them, so the branch is over how the answer is read
/// and how many statements it took, never over what was sent.
fn claim(conn: &mut Wire, queue: &str, now: i64, cutoff: i64) -> io::Result<Option<Job>> {
    let sending = [
        Some(queue.as_bytes().to_vec()),
        Some(millis(now)),
        Some(millis(cutoff)),
    ];
    let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();
    match conn.dialect() {
        Dialect::Postgres(postgres) => postgres_claim(postgres, &bound),
        Dialect::Framed(mut framed) => framed_claim(&mut framed, &bound, now),
    }
}

/// [`claim`] as one statement: [`nvs_stdlib::queue::CLAIM_POSTGRES`]'s data-modifying CTE, which
/// takes the row and answers with it in one round trip.
///
/// Every row is drained before the answer is judged, exactly as `Core\Queue::push` drains its
/// `returning`: the connection has to be back at a message boundary before the next statement on it
/// starts — and before the isolate [`turn`] then runs, which is a whole program's worth of time for
/// a half-read result to sit through. `limit 1` inside the statement is what makes that at most one
/// row, so the last row read is the only one.
fn postgres_claim(conn: &mut nvs_db::PgConn, bound: &[Option<&[u8]>]) -> io::Result<Option<Job>> {
    let mut answered = conn.query(nvs_stdlib::queue::CLAIM_POSTGRES, bound)?;
    // Taken before the first row for [`roster`]'s reason: a `PgRows` lends its columns and its rows
    // out of one borrow.
    let columns: Vec<nvs_db::PgColumn> = answered.columns().to_vec();
    let mut took = None;
    while let Some(row) = answered.next_row()? {
        let (Some(script), Some(args)) = (columns.get(SCRIPT), columns.get(ARGS)) else {
            continue;
        };
        let nvs_db::PgScalar::Text(script) = script.scalar(row.column(SCRIPT)?)? else {
            continue;
        };
        // A `text` column that is null is § 3's job with no payload, which is the ordinary shape of
        // a job that needs none — not a malformed row, so it is `None` rather than a skip.
        let args = match args.scalar(row.column(ARGS)?)? {
            nvs_db::PgScalar::Text(args) => Some(args.into_owned()),
            _ => None,
        };
        // The columns the write-back judges against, and every one of them is `not null` in the
        // migration: a row missing any of them is one no `Core\Queue::push` wrote, so the claim is
        // dropped rather than run on a guess. It stays claimed until § 4's visibility timeout, which
        // is where a row this worker cannot make sense of belongs.
        let [Some(id), Some(attempts), Some(max_attempts), Some(backoff)] =
            [ID, ATTEMPTS, MAX_ATTEMPTS, BACKOFF].map(|at| columns.get(at))
        else {
            continue;
        };
        let (
            nvs_db::PgScalar::Int(id),
            nvs_db::PgScalar::Int(attempts),
            nvs_db::PgScalar::Int(max_attempts),
            nvs_db::PgScalar::Int(backoff_ms),
        ) = (
            id.scalar(row.column(ID)?)?,
            attempts.scalar(row.column(ATTEMPTS)?)?,
            max_attempts.scalar(row.column(MAX_ATTEMPTS)?)?,
            backoff.scalar(row.column(BACKOFF)?)?,
        )
        else {
            continue;
        };
        took = Some(Job {
            id,
            script: script.into_owned(),
            args,
            attempts,
            max_attempts,
            backoff_ms,
        });
    }
    Ok(took)
}

/// `id`'s position in [`nvs_stdlib::queue::CLAIM_POSTGRES`]'s `returning` list, which that constant's doc
/// calls what running a job needs.
const ID: usize = 0;

/// `script`'s position in the same list.
const SCRIPT: usize = 1;

/// `args`'s position in the same list.
const ARGS: usize = 2;

/// `attempts`'s position in the same list.
const ATTEMPTS: usize = 3;

/// `max_attempts`'s position in the same list.
const MAX_ATTEMPTS: usize = 4;

/// `backoff_ms`'s position in the same list, and the last of them.
const BACKOFF: usize = 5;

/// [`claim`] as the pair MySQL and MariaDB spell it, inside one transaction.
///
/// **The transaction is what the claim *means* here** and not a convenience —
/// [`nvs_stdlib::queue::Split`]'s own doc owns that, and the short version is that the
/// `select … for update skip locked` holds the row's lock only until something commits, so a pair
/// that committed in between would have offered the row to a second worker before the `update`
/// marked it taken.
///
/// **A refusal rolls back before it propagates**, best effort and never reported over the refusal
/// the caller is about to hear: a failed statement ends this worker either way, but an open
/// transaction on the way out would hold the row's lock for as long as the connection lived, and
/// the connection is held for the whole run.
fn framed_claim(
    framed: &mut Framed<'_>,
    bound: &[Option<&[u8]>],
    now: i64,
) -> io::Result<Option<Job>> {
    framed.begin(None, false)?;
    match claimed_in_two(framed, bound, now) {
        Ok(took) => {
            framed.commit()?;
            Ok(took)
        }
        Err(refused) => {
            let _undone = framed.roll_back();
            Err(refused)
        }
    }
}

/// [`nvs_stdlib::queue::Split::first`]'s row, then `then` against the id it named — inside the
/// transaction [`framed_claim`] opened.
///
/// The columns are at the ordinals the constants above name, on this dialect as on the other:
/// `both_dialects_answer_a_claim_with_the_same_columns` in `nvs-stdlib` is what holds the two
/// `select` lists to one set of positions, so nothing here is a second reading of § 4's list.
fn claimed_in_two(
    framed: &mut Framed<'_>,
    bound: &[Option<&[u8]>],
    now: i64,
) -> io::Result<Option<Job>> {
    let split = nvs_stdlib::queue::CLAIM_MYSQL;
    let mut answered = framed.query(split.first, bound)?;
    // Described before the first row, for [`framed_roster`]'s reason.
    let columns = answered.columns().to_vec();
    let mut took = None;
    while let Some(row) = answered.next_row()? {
        // `limit 1`, so this guard is about the shape of the loop and not about a second row —
        // every row is still read, because draining is what ends a statement on this driver.
        if took.is_some() {
            continue;
        }
        let mut read = Vec::with_capacity(columns.len());
        for (at, column) in columns.iter().enumerate() {
            let Some(body) = row.value(at) else {
                break;
            };
            read.push(nvs_db::mysql::scalar(column, body)?);
        }
        // A row narrower than § 4's list is one no `Core\Queue::push` wrote, and [`postgres_claim`]
        // drops such a row for the reason its own guards give: it stays claimed until § 4's
        // visibility timeout, which is where a row this worker cannot make sense of belongs.
        if read.len() <= BACKOFF {
            continue;
        }
        let nvs_db::MySqlScalar::Text(script) = &read[SCRIPT] else {
            continue;
        };
        // A `text` column that is null is § 3's job with no payload — the ordinary shape of a job
        // that needs none, so it is `None` rather than a skip.
        let args = match &read[ARGS] {
            nvs_db::MySqlScalar::Text(args) => Some(args.to_string()),
            _ => None,
        };
        let [
            Some(id),
            Some(attempts),
            Some(max_attempts),
            Some(backoff_ms),
        ] = [ID, ATTEMPTS, MAX_ATTEMPTS, BACKOFF].map(|at| integer(&read[at]))
        else {
            continue;
        };
        took = Some(Job {
            id,
            script: script.to_string(),
            args,
            attempts,
            max_attempts,
            backoff_ms,
        });
    }
    // The rows have to have let the connection go before the `update` on it starts, which is the
    // same message-boundary rule [`postgres_claim`] drains for.
    drop(answered);
    let Some(job) = took else {
        return Ok(None);
    };
    // § 4's mark, keyed by the id the `select` named and still holding its lock. The values are the
    // `set` clause's and the `where` clause's in that order, because a `?` is bound where it
    // stands — the same reason [`nvs_stdlib::queue::RETRY_MYSQL`] orders its own values
    // differently.
    let claimed = millis(now);
    let id = job.id.to_string().into_bytes();
    let marking: [Option<&[u8]>; 2] = [Some(claimed.as_slice()), Some(id.as_slice())];
    framed.execute_many(split.then, &[&marking])?;
    Ok(Some(job))
}

/// One integer column of a claim, whichever width and sign the server described it as.
///
/// § 2's schema declares each of them `bigint`, so [`nvs_db::MySqlScalar::Int`] is what a table
/// `nvs queue migrate` created answers with. `UInt` is accepted beside it because a column an
/// operator widened to `bigint unsigned` is still a count, and a claim refused over the sign of a
/// column nothing else is wrong with would strand the job rather than report anything.
fn integer(read: &nvs_db::MySqlScalar<'_>) -> Option<i64> {
    match read {
        nvs_db::MySqlScalar::Int(at) => Some(*at),
        nvs_db::MySqlScalar::UInt(at) => i64::try_from(*at).ok(),
        _ => None,
    }
}

/// Runs one claimed job as `rule:concurrency/a-job-runs-as-a-root-isolate`'s root isolate: its own arena, its own budget, sharing only
/// compiled code, answering with what the attempt threw — or `None`, which is the attempt
/// [`report`] writes back as `Succeeded`.
///
/// **The same `Isolate` a `spawn script` builds, through the same door**, which is § 5's "there is
/// no second execution path" taken literally: a job is resolved by
/// [`nvs_runtime::script::resolve`], so `script.spawn` is asked of this worker's context with the
/// job's path as its scope, and it runs on [`nvs_host::Output::Capture`] — the default, and here
/// the only honest one, since a job's `echo` landing in the middle of what the run's own script is
/// writing is exactly the mixing that option exists to prevent.
///
/// A refusal is written to standard error rather than answered, because there is nobody to answer:
/// a worker has no caller. One line per refused job, and the answer is a failure either way — a job
/// whose script does not resolve is a failed attempt like any other, so § 6's ladder is what
/// bounds it rather than a second policy written here. It crosses as the same [`nvs_host::Failure`]
/// a throw does, under [`refusal`]'s class, so a dead-letter row records the two in one shape.
fn run(ctx: &mut nvs_runtime::Ctx, job: &Job) -> Option<nvs_host::Failure> {
    let program = match nvs_runtime::script::resolve(ctx, &job.script) {
        Ok(program) => program,
        Err(refused) => {
            eprintln!(
                "warning: the queued job `{}` was not run: {refused}",
                job.script
            );
            return Some(refusal(format!("the script did not resolve: {refused}")));
        }
    };
    // Ownership: `payload` hands over one reference and `Isolate::new` consumes exactly one, so
    // nothing here releases anything — and the decode is after the resolve so that a job whose
    // script does not compile never builds a value to release.
    let args = job
        .args
        .as_deref()
        .and_then(nvs_stdlib::queue::payload)
        .unwrap_or_else(nvs_runtime::Value::null);
    match nvs_host::Isolate::new(program, args, nvs_host::Output::Capture).run(ctx) {
        // `ok` and not "it returned": an isolate whose program threw, or that was torn down over a
        // budget, answers here exactly as one that returned — which is § 6's "a job exceeding its
        // memory, CPU or time budget is a failed attempt, reported as that rather than as an
        // out-of-memory". So the flag the isolate already computed is the whole judgement, and
        // there is no second reading of the completion beside it.
        Ok(mut completion) => {
            // A job has no caller, so nothing downstream ever reads its answer —
            // and [`nvs_host::Completion`]'s `value` is a reference *copied into
            // this frame's ownership*, not a borrow of the arena that has already
            // gone. Dropping it without releasing it leaks the whole returned
            // graph, once per job that returns one, which grows with jobs served
            // rather than with jobs in flight: AGENTS.md's priority 5 calls that a
            // leak and not a footprint. Released before the verdict is read
            // because the obligation is the same on both arms.
            completion.discard_value();
            if completion.ok {
                return None;
            }
            // `error` is present exactly when `ok` is false — [`nvs_host::Completion`]'s own doc —
            // so the fallback is unreachable for a completion this host produced. Written rather
            // than asserted, because an exhausted job still has to reach `nvs_dead_jobs` carrying
            // something a reader can act on if it ever is reached.
            let failure = completion.error.unwrap_or_else(|| {
                refusal("the attempt ended without returning and named no failure".to_string())
            });
            // Only the *last* attempt's error reaches the dead-letter row, so for every attempt
            // before it this line is the only place the failure is visible at all — and a queue
            // whose failures are silent is the one thing § 6 exists to prevent.
            eprintln!(
                "warning: the queued job `{}` threw {}: {}",
                job.script, failure.class, failure.message
            );
            Some(failure)
        }
        Err(refused) => {
            // The argument refusing to cross, which is the graph copy's answer and not the job's —
            // a payload from JSON is a tree of scalars, arrays and strings, so this is unreachable
            // for a row this deployment wrote and is reported rather than asserted.
            eprintln!(
                "warning: the queued job `{}` was not run: its payload could not cross: {refused}",
                job.script
            );
            Some(refusal(format!("its payload could not cross: {refused}")))
        }
    }
}

/// A failure that was not a throw, in the shape [`report`] records a thrown one in.
///
/// `Error` is [`nvs_host::Failure`]'s own name for the class of a failure the child did not throw,
/// so nothing here invents a spelling: a refusal and a throw differ in that field and in nothing
/// else, which is what lets § 6's `errors` array hold either without a second entry shape.
fn refusal(message: String) -> nvs_host::Failure {
    nvs_host::Failure {
        class: "Error".to_string(),
        message,
    }
}

/// `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`'s write-back: the row the claim took, told what the attempt did.
///
/// **A branch per outcome, and one lease.** An attempt that returned is `Succeeded`; one that
/// failed with attempts still to come is armed for the next on § 6's ladder; one that failed on the
/// job's last attempt is moved into `nvs_dead_jobs` carrying what it threw. Each is one statement
/// out of [`nvs_stdlib::queue`] — the move a pair inside one transaction where the dialect has no
/// data-modifying CTE to say it in one — and that module owns each of them and every column they
/// name.
///
/// **Keyed on the lease `held_at`**, which is the `claimed_at` this worker's own claim wrote —
/// [`nvs_stdlib::queue::SUCCEEDED_POSTGRES`]'s doc owns why, and it is why this takes the turn's
/// instant rather than reading the clock again. A statement that matches no row is the ordinary
/// shape of a worker that overran § 4's visibility window, not an error, so the affected count is
/// deliberately not judged: another worker owns the job by then and has its own attempt to report.
///
/// The retry's own `run_at` is computed against a *fresh* instant, because the attempt has just
/// spent however long it spent: a backoff measured from the claim would already be part-elapsed,
/// and for a job that ran longer than its own base delay it would be wholly elapsed, which is
/// § 6's ladder collapsed to a busy loop.
fn report(
    conn: &mut Wire,
    job: &Job,
    held_at: i64,
    failure: Option<&nvs_host::Failure>,
) -> io::Result<()> {
    let id = job.id.to_string().into_bytes();
    let held = millis(held_at);
    let Some(failure) = failure else {
        let lease: [Option<&[u8]>; 2] = [Some(id.as_slice()), Some(held.as_slice())];
        return match conn.dialect() {
            Dialect::Postgres(postgres) => {
                apply(postgres, nvs_stdlib::queue::SUCCEEDED_POSTGRES, &lease)
            }
            // The same values in the same order, which that constant's doc calls the lease
            // keying surviving the transcription intact.
            Dialect::Framed(mut framed) => {
                apply_framed(&mut framed, nvs_stdlib::queue::SUCCEEDED_MYSQL, &lease)
            }
        };
    };
    if job.attempts >= job.max_attempts {
        // § 6's floor: the attempt was the job's last, so the row moves rather than being armed
        // again — one statement, keyed on the same lease, which is where "never deleted by the
        // runtime" is actually kept. `>=` and not `==` because `[queue] max_attempts` is
        // configuration an operator can lower under a job that has already used more than the new
        // bound, and such a row is exhausted rather than owed an attempt it can no longer have.
        let errors = nvs_stdlib::queue::dead_errors(held_at, &failure.class, &failure.message);
        // The instant the attempt *ended*, read here rather than taken from the claim, for the
        // reason the retry's own `run_at` is: the attempt has just spent however long it spent.
        let failed = millis(nvs_stdlib::queue::now_millis());
        return match conn.dialect() {
            Dialect::Postgres(postgres) => apply(
                postgres,
                nvs_stdlib::queue::DEAD_LETTER_POSTGRES,
                &[
                    Some(id.as_slice()),
                    Some(held.as_slice()),
                    Some(failed.as_slice()),
                    Some(errors.as_bytes()),
                ],
            ),
            Dialect::Framed(mut framed) => dead_letter_in_two(
                &mut framed,
                &id,
                &held,
                failed.as_slice(),
                errors.as_bytes(),
            ),
        };
    }
    let due = millis(nvs_stdlib::queue::retry_at(
        nvs_stdlib::queue::now_millis(),
        job.attempts,
        job.backoff_ms,
        job.id,
    ));
    match conn.dialect() {
        Dialect::Postgres(postgres) => apply(
            postgres,
            nvs_stdlib::queue::RETRY_POSTGRES,
            &[
                Some(id.as_slice()),
                Some(held.as_slice()),
                Some(due.as_slice()),
            ],
        ),
        // **The one place the two dialects part on what is sent**, and this is the site
        // [`nvs_stdlib::queue::RETRY_MYSQL`]'s doc means when it says the caller is where the two
        // orders are reconciled: the `run_at` it writes is in the `set` clause, which is left of
        // the `where`, and a `?` is bound by the position it occupies. A worker sending
        // PostgreSQL's order into that text would push every job's next attempt out to its own id.
        Dialect::Framed(mut framed) => apply_framed(
            &mut framed,
            nvs_stdlib::queue::RETRY_MYSQL,
            &[
                Some(due.as_slice()),
                Some(id.as_slice()),
                Some(held.as_slice()),
            ],
        ),
    }
}

/// § 6's move as MySQL and MariaDB spell it: the copy, then the delete, inside one transaction.
///
/// **The copy runs first and that is not this function's choice** —
/// [`nvs_stdlib::queue::DEAD_LETTER_MYSQL`]'s doc owns it: the columns have to be read while they
/// still exist, since a `delete` here cannot answer with what it removed. § 6's property is
/// unchanged, the row being in both tables or in neither, and here it is the transaction that
/// holds what PostgreSQL's single statement held by construction.
///
/// The rollback is [`framed_claim`]'s, for its reason.
fn dead_letter_in_two(
    framed: &mut Framed<'_>,
    id: &[u8],
    held: &[u8],
    failed: &[u8],
    errors: &[u8],
) -> io::Result<()> {
    let split = nvs_stdlib::queue::DEAD_LETTER_MYSQL;
    // The `insert … select` names the two values it adds to the copied row before the two the
    // lease is keyed on, because that is where they stand in the text.
    let copying: [Option<&[u8]>; 4] = [Some(failed), Some(errors), Some(id), Some(held)];
    let removing: [Option<&[u8]>; 2] = [Some(id), Some(held)];
    framed.begin(None, false)?;
    let moved = apply_framed(framed, split.first, &copying)
        .and_then(|()| apply_framed(framed, split.then, &removing));
    match moved {
        Ok(()) => {
            framed.commit()?;
            Ok(())
        }
        Err(refused) => {
            let _undone = framed.roll_back();
            Err(refused)
        }
    }
}

/// One statement that answers with no rows, run for its effect.
///
/// [`nvs_db::PgConn::execute_many`] with a single set is what a driver spells that as — there is no
/// second door for a one-set write — and it leaves the connection at a message boundary, which is
/// what the next claim on it needs.
fn apply(conn: &mut nvs_db::PgConn, sql: &str, bound: &[Option<&[u8]>]) -> io::Result<()> {
    conn.execute_many(sql, &[bound])?;
    Ok(())
}

/// [`apply`] over MySQL's framing, which MariaDB runs unchanged.
///
/// The affected count is discarded on both drivers and for one reason, which [`report`]'s own doc
/// owns: a write-back that matches no row is the ordinary shape of a worker that overran § 4's
/// visibility window, not an error.
fn apply_framed(framed: &mut Framed<'_>, sql: &str, bound: &[Option<&[u8]>]) -> io::Result<()> {
    framed.execute_many(sql, &[bound])?;
    Ok(())
}

/// An epoch-millisecond instant as the text a placeholder is sent as — a `$n::bigint` on one
/// driver, a `?` bound as text on the other, and the same octets either way.
fn millis(at: i64) -> Vec<u8> {
    at.to_string().into_bytes()
}

/// Parks this worker for [`IDLE_TURN`], reporting how the wait ended.
///
/// With no host on the thread there is nothing to hand the core back to, so the wait is a blocking
/// one — `Core\Time::sleep`'s own reading, and unreachable here, since a worker is only ever a task.
fn nap() -> Woken {
    with_current(|host| host.sleep(IDLE_TURN)).unwrap_or_else(|| {
        std::thread::sleep(IDLE_TURN);
        Woken::Elapsed
    })
}

/// One driver's half of [`open`]: resolve the block as that driver's target, open it, and hand back
/// the arm of [`Wire`] it belongs to.
///
/// **A macro for [`crate::queue`]'s `open_and_apply` reason** — the arms differ in names and
/// not in shape, and `rule:core-classes/db-drivers-are-an-enum` makes the drivers an enum rather than a trait, so there is no type parameter to write this as
/// a generic function over. It is not that macro because every refusal here is a `warning:` that
/// returns no worker where that one is an `error:` that returns an exit code.
macro_rules! open_as {
    ($target:ty, $conn:ty, $port:path, $wire:path, $name:expr, $block:expr) => {{
        let target = match <$target>::resolve($block) {
            Ok(target) => target,
            Err(refused) => {
                eprintln!(
                    "warning: no queue worker started: {}",
                    refused.refusal($name)
                );
                return None;
            }
        };
        let Some(address) = crate::queue::address_of(target.host, $block.port, $port) else {
            eprintln!(
                "warning: no queue worker started: `[db.{}]` names the host `{}`, which resolves \
                 to no address",
                $name, target.host
            );
            return None;
        };
        match <$conn>::connect(address, &target, Some(Instant::now() + CONNECT_DEADLINE)) {
            Ok(conn) => Some($wire(conn)),
            Err(err) => {
                eprintln!(
                    "warning: no queue worker started: `[db.{}]` at {address} did not open: {err}",
                    $name
                );
                None
            }
        }
    }};
}

/// The worker's connection, or a note on standard error and no worker at all.
///
/// **Reported rather than swallowed**, because a queue that never claims is the failure mode
/// `nvs_config::queue`'s module doc calls the expensive one: an operator learns it from the work
/// that did not happen, days later. One line per run and never per turn — a worker that cannot open
/// its connection returns instead of retrying, since the run it is a passenger on is measured in
/// milliseconds and a second attempt would land inside the same outage.
///
/// **The driver is read before anything is resolved**, where `nvs queue migrate` reads it to pick a
/// migration list: a `[db.<name>]` block belongs to exactly one backend, and a target resolver
/// asked about a block written for another one refuses over the `driver` field rather than over
/// anything a worker could act on.
fn open(name: &str, block: &Database) -> Option<Wire> {
    let Some(written) = block.driver.as_deref() else {
        eprintln!(
            "warning: no queue worker started: `[db.{name}]` names no `driver`, so it is not \
             openable at all"
        );
        return None;
    };
    let Some(driver) = nvs_db::Driver::from_config_name(written) else {
        eprintln!(
            "warning: no queue worker started: `[db.{name}]` names the `{written}` driver, which \
             Novis has no backend for"
        );
        return None;
    };
    match driver {
        nvs_db::Driver::Postgres => open_as!(
            nvs_db::PgTarget<'_>,
            nvs_db::PgConn,
            nvs_db::pg::DEFAULT_PORT,
            Wire::Postgres,
            name,
            block
        ),
        nvs_db::Driver::MySql => open_as!(
            nvs_db::MySqlTarget<'_>,
            nvs_db::MySqlConn,
            nvs_db::mysql::DEFAULT_PORT,
            Wire::MySql,
            name,
            block
        ),
        nvs_db::Driver::MariaDb => open_as!(
            nvs_db::MariaTarget<'_>,
            nvs_db::MariaConn,
            nvs_db::maria::DEFAULT_PORT,
            Wire::MariaDb,
            name,
            block
        ),
        // Spelled rather than left to a `_`, exactly as [`crate::queue`]'s applying half spells the
        // same ones: a driver *gaining* a send path arrives here as a build failure instead of as a
        // refusal that has stopped being true.
        nvs_db::Driver::SqlServer | nvs_db::Driver::Sqlite => {
            eprintln!(
                "warning: no queue worker started: `[db.{name}]` names the {} driver, which runs no \
                 statement at all yet — `Core\\Db`'s own known gaps are the list",
                driver.display_name()
            );
            None
        }
    }
}

/// The worker's connection, in the driver `[db.<name>]` named.
///
/// **Owned, with an arm per driver, where [`Dialect`] is borrowed and narrower.** A worker holds
/// its connection for the whole run — the module doc's *What it spends* section owns why it is not
/// out of `rule:security/db-pool-reset-is-a-boundary`'s pool — so there has to be a value that *is* the connection, and it has one arm
/// per driver this can open. What every statement below then branches on is the dialect instead,
/// and [`Wire::dialect`] is the one place a driver narrows to one.
///
/// The drivers with no send path are not arms: [`open`] refuses them before anything is
/// connected, so a `Wire` that exists is one § 4's statements can run on.
enum Wire {
    /// § 4's and § 6's statements as PostgreSQL's single texts.
    Postgres(nvs_db::PgConn),
    /// The same statements as MySQL's dialect, some of them
    /// [`nvs_stdlib::queue::Split`]s.
    MySql(nvs_db::MySqlConn),
    /// MariaDB, which runs every one of MySQL's texts unchanged over its own framing and its own
    /// authentication roster.
    MariaDb(nvs_db::MariaConn),
}

impl Wire {
    /// This connection borrowed as the dialect its statements are written in.
    fn dialect(&mut self) -> Dialect<'_> {
        match self {
            Wire::Postgres(postgres) => Dialect::Postgres(postgres),
            Wire::MySql(mysql) => Dialect::Framed(Framed::MySql(mysql)),
            Wire::MariaDb(maria) => Dialect::Framed(Framed::MariaDb(maria)),
        }
    }
}

/// A borrowed [`Wire`], narrowed to the two dialects `nvs_stdlib::queue` writes.
///
/// The same two arms `Core\Queue`'s own members branch on, and for the same reason: § 2's schema
/// has two migration lists and §§ 4 and 6's statements two spellings, so a third arm here would be
/// a driver with nothing to send.
enum Dialect<'a> {
    /// [`nvs_stdlib::queue::CLAIM_POSTGRES`] and its siblings, each answering in one statement.
    Postgres(&'a mut nvs_db::PgConn),
    /// [`nvs_stdlib::queue::CLAIM_MYSQL`] and its siblings, some of them pairs inside one
    /// transaction — and MariaDB runs every one of them unchanged.
    Framed(Framed<'a>),
}

/// The two drivers that share one dialect and one send path, borrowed as one.
///
/// **MariaDB is its own driver above the framing, not inside it** — its own target, its own
/// authentication roster, its own § 8 code table — and what it does not duplicate is the wire:
/// `nvs_db::MariaConn::query` delegates into the same `nvs_db::mysql::start_statement` and hands
/// back the same [`nvs_db::MySqlRows`]. So the only difference this module can observe between the
/// two is the *type of the borrow*, and without this every framed reader below would grow a second
/// arm copying the first line for line.
///
/// `nvs-stdlib` has the same enum for the same reason and it is `pub(crate)` there, so this is not
/// a duplicate that could have been shared: `rule:core-classes/db-crate-boundary`'s crate graph puts the two on opposite
/// sides of a boundary, and what is shared is the statements they send.
enum Framed<'a> {
    /// `rule:core-classes/db-one-api`'s two round trips as MySQL frames them.
    MySql(&'a mut nvs_db::MySqlConn),
    /// The same two, framed as MariaDB.
    MariaDb(&'a mut nvs_db::MariaConn),
}

impl Framed<'_> {
    /// One statement and the rows it answers with, as the driver's own `query`.
    fn query(&mut self, sql: &str, bound: &[Option<&[u8]>]) -> io::Result<nvs_db::MySqlRows<'_>> {
        match self {
            Framed::MySql(mysql) => mysql.query(sql, bound),
            Framed::MariaDb(maria) => maria.query(sql, bound),
        }
    }

    /// One statement run for its effect, as the driver's own `execute_many`.
    fn execute_many(&mut self, sql: &str, sets: &[&[Option<&[u8]>]]) -> io::Result<u64> {
        match self {
            Framed::MySql(mysql) => mysql.execute_many(sql, sets),
            Framed::MariaDb(maria) => maria.execute_many(sql, sets),
        }
    }

    /// `rule:core-classes/db-transactions`'s `START TRANSACTION`, which is what a [`nvs_stdlib::queue::Split`] means.
    fn begin(
        &mut self,
        isolation: Option<nvs_db::Isolation>,
        read_only: bool,
    ) -> io::Result<nvs_db::QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.begin(isolation, read_only),
            Framed::MariaDb(maria) => maria.begin(isolation, read_only),
        }
    }

    /// § 7's `COMMIT`, which is where a split pair becomes one moment.
    fn commit(&mut self) -> io::Result<nvs_db::QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.commit(),
            Framed::MariaDb(maria) => maria.commit(),
        }
    }

    /// § 7's `ROLLBACK`, run best-effort on the way out of a refused pair.
    fn roll_back(&mut self) -> io::Result<nvs_db::QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.roll_back(),
            Framed::MariaDb(maria) => maria.roll_back(),
        }
    }
}
