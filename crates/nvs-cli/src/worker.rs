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
//! ## What it spends
//!
//! One PostgreSQL connection per worker, opened once and held for the run, plus two statements per
//! idle turn — the roster and nothing, since a roster with no due work claims nothing. It is
//! `workers` connections against the deployment's `max_connections` and the operator wrote the
//! number; [ADR 0067] § 13's pool is deliberately not involved, because a pool exists to be handed
//! between requests and this connection belongs to one task for its whole life.
//!
//! ## Known gap
//!
//! **A claimed job is not run yet.** This is § 4's claim and nothing past it: the row goes to
//! `Claimed`, its `attempts` is incremented, and it stays there until § 4's visibility timeout
//! hands it back. Running the script the row names, reporting the attempt and § 6's dead-letter
//! move are the two slices after this one, and both are additions to [`turn`] rather than changes
//! to it.
//!
//! [ADR 0067]: ../../../docs/adr/0067-core-db.md
//! [ADR 0084]: ../../../docs/adr/0084-durable-background-jobs.md

use std::cell::Cell;
use std::io;
use std::rc::Rc;
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

/// The workers a run started, and the switch that stops them.
///
/// One flag for all of them rather than one each: they stop together, at the same moment and for
/// the same reason.
pub(crate) struct Workers {
    /// Set by the script's task when it has finished, read by every worker at the top of its turn.
    /// An [`Rc`] and a [`Cell`] because both ends are tasks on one core — there is no thread here
    /// to synchronize with.
    stop: Rc<Cell<bool>>,
}

impl Workers {
    /// Tells every worker this run started to finish its turn and return.
    pub(crate) fn stop(&self) {
        self.stop.set(true);
    }
}

/// Spawns `bounds.workers` worker tasks onto `sched`, each claiming out of `[db.<name>]`.
///
/// Nothing runs here — [`nvs_host::Scheduler::spawn`] only queues — so the caller is free to spawn
/// the script's own task afterwards and install the reactor after that.
pub(crate) fn start(
    sched: &mut nvs_host::Scheduler,
    bounds: &QueueBounds,
    block: &Database,
) -> Workers {
    let stop = Rc::new(Cell::new(false));
    for _ in 0..bounds.workers {
        let stop = Rc::clone(&stop);
        let name = bounds.connection.clone();
        // Cloned rather than borrowed because a task's body is `'static`, and cloned per worker
        // rather than shared because a `Database` is a handful of strings read once at connect.
        let block = block.clone();
        let visibility = bounds.visibility;
        sched.spawn(
            nvs_runtime::Ctx::stdout(),
            nvs_runtime::TaskRoot::Worker,
            move |_ctx| claim_until_stopped(&stop, &name, &block, visibility),
        );
    }
    Workers { stop }
}

/// One worker's whole life: open the connection, then take turns until the run ends.
///
/// A statement that fails ends the worker rather than being retried. A PostgreSQL connection is
/// only usable at a message boundary and a failed statement is not one, so the honest recovery is a
/// new connection — which is the next run's, since this one is by then within a few milliseconds of
/// its own end.
fn claim_until_stopped(stop: &Cell<bool>, name: &str, block: &Database, visibility: Duration) {
    let Some(mut conn) = open(name, block) else {
        return;
    };
    // Saturating rather than wrapping for a `visibility` no operator would write: the clamp makes
    // every job's claim eligible again immediately, which is a busy worker, where the wrap would
    // make it eligible never.
    let window = i64::try_from(visibility.as_millis()).unwrap_or(i64::MAX);
    while !stop.get() {
        match turn(&mut conn, window) {
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
fn turn(conn: &mut nvs_db::PgConn, window: i64) -> io::Result<bool> {
    let now = nvs_stdlib::queue::now_millis();
    let cutoff = now.saturating_sub(window);
    let mut claimed = false;
    for queue in roster(conn, now, cutoff)? {
        claimed |= claim(conn, &queue, now, cutoff)?;
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

/// One claim against one queue, answering whether a row came back.
///
/// Every row is drained before the answer is judged, exactly as `Core\Queue::push` drains its
/// `returning`: the connection has to be back at a message boundary before the next statement on it
/// starts. `limit 1` inside the statement is what makes that at most one row.
fn claim(conn: &mut nvs_db::PgConn, queue: &str, now: i64, cutoff: i64) -> io::Result<bool> {
    let sending = [
        Some(queue.as_bytes().to_vec()),
        Some(millis(now)),
        Some(millis(cutoff)),
    ];
    let bound: Vec<Option<&[u8]>> = sending.iter().map(|one| one.as_deref()).collect();
    let mut answered = conn.query(nvs_stdlib::queue::CLAIM, &bound)?;
    let mut took = false;
    while answered.next_row()?.is_some() {
        took = true;
    }
    Ok(took)
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
