//! The in-process worker [ADR 0084] § 2's `[queue] workers` starts: a task beside the script's, on
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
//! *shared* is the schema — [`nvs_stdlib::queue::QUEUES`] and [`nvs_stdlib::queue::CLAIM`] are read
//! from the module that owns § 2's tables, so a worker decides nothing about them.
//!
//! [`nvs_runtime::TaskRoot::Worker`] and not `Request`, per
//! [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md) § 2:
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
//! [ADR 0118](../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md) § 2's
//! spawn door like any other — [`nvs_runtime::script::resolve`] asks `script.spawn` with the job's
//! path as its scope. That question is asked of the *context*, and a worker's context is not the
//! script's, so each one is handed the same configuration snapshot the run resolved at boot. § 5's
//! "grants narrowed from those recorded at enqueue" is the narrower rule and § 2's schema has no
//! column to record them in yet; what is here is the deployment's own configuration, which is the
//! ceiling that narrowing would sit under.
//!
//! ## What it spends
//!
//! One PostgreSQL connection per worker, opened once and held for the run, plus two statements per
//! idle turn — the roster and nothing, since a roster with no due work claims nothing. It is
//! `workers` connections against the deployment's `max_connections` and the operator wrote the
//! number; [ADR 0067] § 13's pool is deliberately not involved, because a pool exists to be handed
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
//! polled — so under the other order every `nvs run` of every program waited out one PostgreSQL
//! handshake before it could exit. Measured on this repository's own configuration: 16.7 ms
//! against 8.9 ms for the same empty program, which is what stage 1's `a warm-cache CLI start
//! stays under 10ms` was failing on. [`Workers`] is therefore created before either task and read
//! here before [`open`], not only at the top of a turn.
//!
//! ## Known gap
//!
//! **The attempt is not reported.** [`run`] is § 5's isolate and nothing past it: whatever the job
//! answered is dropped, so the row stays `Claimed` until § 4's visibility timeout hands it back —
//! including the row of a job that succeeded. § 6's write-back, its retry ladder and its
//! dead-letter move are the two slices after this one, and both are additions beside [`run`]
//! rather than changes to it.
//!
//! [ADR 0067]: ../../../docs/adr/0067-core-db.md
//! [ADR 0084]: ../../../docs/adr/0084-durable-background-jobs.md

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
/// Much shorter than [`crate::queue`]'s ten seconds, and for the opposite reason: that command
/// exists to reach the server and has nothing else to do, while a worker is a passenger on a run
/// whose script may finish in milliseconds. An unreachable server costs the run this much and then
/// stops costing it anything.
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
/// A statement that fails ends the worker rather than being retried. A PostgreSQL connection is
/// only usable at a message boundary and a failed statement is not one, so the honest recovery is a
/// new connection — which is the next run's, since this one is by then within a few milliseconds of
/// its own end.
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
/// The roster is asked first for the reason [`nvs_stdlib::queue::QUEUES`] owns — § 2 names no
/// queues, so the table is the only place they are written down — and the two instants are computed
/// once here so that every claim in this turn judges due-ness against the same moment.
fn turn(ctx: &mut nvs_runtime::Ctx, conn: &mut nvs_db::PgConn, window: i64) -> io::Result<bool> {
    let now = nvs_stdlib::queue::now_millis();
    let cutoff = now.saturating_sub(window);
    let mut claimed = false;
    for queue in roster(conn, now, cutoff)? {
        if let Some(job) = claim(conn, &queue, now, cutoff)? {
            // Run before the next queue is claimed against, rather than after the roster has been
            // walked: a claim this worker is holding is a job nothing else may take, so the
            // shortest time between the two is the one that costs a fleet the least.
            run(ctx, &job);
            claimed = true;
        }
    }
    Ok(claimed)
}

/// The queues holding work this worker could take, as [`nvs_stdlib::queue::QUEUES`] answers it.
fn roster(conn: &mut nvs_db::PgConn, now: i64, cutoff: i64) -> io::Result<Vec<String>> {
    let sending = [Some(millis(now)), Some(millis(cutoff))];
    let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();
    let mut answered = conn.query(nvs_stdlib::queue::QUEUES, &bound)?;
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

/// What a worker reads off [`nvs_stdlib::queue::CLAIM`]'s `returning` list, and what running one
/// needs.
///
/// Two columns of the six for now — the four the retry ladder judges against are the next slice's,
/// and a field nothing reads is a field whose decode nothing checks.
struct Job {
    /// The file § 1 says a job names. `spawn script`'s own spelling, resolved the same way.
    script: String,
    /// The `args` column as it is stored: the document `Core\Queue::push` encoded, or `None` for a
    /// job pushed without one. Decoded at the last moment, in [`run`], so a job whose script is
    /// refused never pays for it.
    args: Option<String>,
}

/// One claim against one queue, answering with the row it took.
///
/// Every row is drained before the answer is judged, exactly as `Core\Queue::push` drains its
/// `returning`: the connection has to be back at a message boundary before the next statement on it
/// starts — and before the isolate [`turn`] then runs, which is a whole program's worth of time for
/// a half-read result to sit through. `limit 1` inside the statement is what makes that at most one
/// row, so the last row read is the only one.
fn claim(conn: &mut nvs_db::PgConn, queue: &str, now: i64, cutoff: i64) -> io::Result<Option<Job>> {
    let sending = [
        Some(queue.as_bytes().to_vec()),
        Some(millis(now)),
        Some(millis(cutoff)),
    ];
    let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();
    let mut answered = conn.query(nvs_stdlib::queue::CLAIM, &bound)?;
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
        took = Some(Job {
            script: script.into_owned(),
            args,
        });
    }
    Ok(took)
}

/// `script`'s position in [`nvs_stdlib::queue::CLAIM`]'s `returning` list, which that constant's
/// doc calls what running a job needs.
const SCRIPT: usize = 1;

/// `args`'s position in the same list.
const ARGS: usize = 2;

/// Runs one claimed job as ADR 0084 § 5's root isolate: its own arena, its own budget, sharing only
/// compiled code.
///
/// **The same `Isolate` a `spawn script` builds, through the same door**, which is § 5's "there is
/// no second execution path" taken literally: a job is resolved by
/// [`nvs_runtime::script::resolve`], so `script.spawn` is asked of this worker's context with the
/// job's path as its scope, and it runs on [`nvs_host::Output::Capture`] — the default, and here
/// the only honest one, since a job's `echo` landing in the middle of what the run's own script is
/// writing is exactly the mixing that option exists to prevent.
///
/// A refusal is written to standard error rather than answered, because there is nobody to answer:
/// a worker has no caller. One line per refused job, and the job stays claimed either way — the
/// module doc's *Known gap* owns what the next slice writes back.
fn run(ctx: &mut nvs_runtime::Ctx, job: &Job) {
    let program = match nvs_runtime::script::resolve(ctx, &job.script) {
        Ok(program) => program,
        Err(refused) => {
            eprintln!(
                "warning: the queued job `{}` was not run: {refused}",
                job.script
            );
            return;
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
    if let Err(refused) = nvs_host::Isolate::new(program, args, nvs_host::Output::Capture).run(ctx)
    {
        // The argument refusing to cross, which is the graph copy's answer and not the job's — a
        // payload from JSON is a tree of scalars, arrays and strings, so this is unreachable for a
        // row this deployment wrote and is reported rather than asserted.
        eprintln!(
            "warning: the queued job `{}` was not run: its payload could not cross: {refused}",
            job.script
        );
    }
}

/// An epoch-millisecond instant as the text a `$n::bigint` placeholder is sent as.
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

/// The worker's connection, or a note on standard error and no worker at all.
///
/// **Reported rather than swallowed**, because a queue that never claims is the failure mode
/// `nvs_config::queue`'s module doc calls the expensive one: an operator learns it from the work
/// that did not happen, days later. One line per run and never per turn — a worker that cannot open
/// its connection returns instead of retrying, since the run it is a passenger on is measured in
/// milliseconds and a second attempt would land inside the same outage.
fn open(name: &str, block: &Database) -> Option<nvs_db::PgConn> {
    let target = match nvs_db::PgTarget::resolve(block) {
        Ok(target) => target,
        Err(refused) => {
            eprintln!(
                "warning: no queue worker started: {}",
                refused.refusal(name)
            );
            return None;
        }
    };
    let Some(address) = crate::queue::address_of(target.host, block.port) else {
        eprintln!(
            "warning: no queue worker started: `[db.{name}]` names the host `{}`, which resolves \
             to no address",
            target.host
        );
        return None;
    };
    match nvs_db::PgConn::connect(address, &target, Some(Instant::now() + CONNECT_DEADLINE)) {
        Ok(conn) => Some(conn),
        Err(err) => {
            eprintln!(
                "warning: no queue worker started: `[db.{name}]` at {address} did not open: {err}"
            );
            None
        }
    }
}
