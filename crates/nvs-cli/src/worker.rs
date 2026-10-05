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
//! ## Why it is stopped rather than left running, and which end stops it
//!
//! [`nvs_host::run_until_idle`] returns when no task is left that could run again, and a worker
//! waiting for work always could — so a run with one would never end. [`Workers`] is the switch
//! that ends it, and which end holds it is the whole of the difference between the two binaries:
//! under `nvs run` the script's own task sets [`Workers::stop`] on its way out, and under
//! `nvs serve` there is no script to exit, so the same predicate reads the drain that command's
//! shutdown begins (`rule:concurrency/one-process-serves-requests-schedules-and-jobs`). A served
//! worker that ignored it would leave `run_until_idle` a task that is always parked, which is a
//! process nothing but a kill could stop.
//!
//! **Read at the top of a turn and nowhere inside one**, which is the semantics rather than an
//! economy: a drain means stop taking *new* work, and claiming a job is taking new work, so a job
//! already claimed is run and written back exactly as a request already accepted is answered. A
//! flag and a bit rather than a cancellation because every wait a stop can land in is either ended
//! by the stop or bounded and short. A worker waiting for work is woken by it (§ *What an idle
//! worker waits on*). The tail a stop pays otherwise is one statement's round trip while a claim
//! is in flight, and at worst one [`CONNECT_DEADLINE`] for a worker still shaking hands with a
//! server that is not answering — or one [`STATEMENT_DEADLINE`] for one that stopped answering
//! mid-statement, which is the only wait here a peer rather than this process decides the length of.
//!
//! ## What an idle worker waits on
//!
//! A worker asks for work the moment it starts and again the moment a job is written back, and
//! one that was answered with nothing waits before it asks again
//! (`rule:concurrency/a-push-wakes-an-idle-worker`). That wait is the whole cost of a `[queue]`
//! with nothing in it, so it is long: one [`IDLE_WAIT`], with no backoff and no key to set. Three
//! things end it early, and each is a wake and not a shorter poll:
//!
//! - **A push in this process.** `Core\Queue::push` rings [`Bell::process`] once the job is
//!   committed, from whichever core the request ran on, and every idle worker asks at once.
//! - **A stop.** [`Workers::stop`] rings the bell after it sets the flag, which is the script's
//!   exit under `nvs run` and a reload taking a worker away under `nvs serve`.
//! - **The drain**, which a served worker registers on for the length of the wait
//!   ([`nvs_host::wake_at_drain`]).
//!
//! Work nothing in this process announced is found by the bound alone, so it may start up to one
//! [`IDLE_WAIT`] late: a job another process pushed, a delayed job coming due, a retry, and a job
//! whose worker died before writing it back. One path serves every driver, because a ring is a
//! call this process makes after its own statement and never a message a database sends.
//!
//! ## What a job's grants are
//!
//! A claimed job is § 5's root isolate, and an isolate is reached through
//! `rule:security/capability-check-at-the-door`'s
//! spawn door like any other — [`nvs_runtime::script::resolve`] asks `script.spawn` with the job's
//! path as its scope. That question is asked of the *context*, and a worker's context is not the
//! script's, so each job is handed the snapshot the `[[app]]` blocks matching its own script fold
//! to, out of the publish in force when it is claimed ([`configure`]). Under `nvs serve` that is
//! whatever a reload published last, and under `nvs run` it is the boot's.
//!
//! That snapshot is the **ceiling** and not the answer.
//! `rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue` has the job run under what
//! the *enqueuing* context had narrowed itself to, and the claim answers with the `grants` and
//! `limits` columns that recorded it. [`run`] reads them back through
//! [`nvs_stdlib::queue::narrowing`] and hands the pair over as the same
//! [`nvs_runtime::host::Narrowing`] a `spawn script … with(…)` carries, so what applies it is
//! `nvs_config::Request::set` — the one reader that refuses a value wider than what is in force.
//! A job whose row recorded neither half narrows nothing and runs at the ceiling, which is where a
//! request that gave nothing up belongs.
//!
//! ## What it spends
//!
//! One connection per worker in whichever driver `[db.<name>]` names, opened once and held for the
//! run, plus one roster statement each time an idle worker asks and nothing beyond it, since a
//! roster with no due work claims nothing. An idle worker asks once per [`IDLE_WAIT`] and once
//! per ring, and holds a wake registration on the bell, and one on the drain under `nvs serve`,
//! for as long as it waits. A turn that *does* claim costs a transaction's worth of round trips on
//! MySQL, MariaDB and SQLite where it costs a single statement on PostgreSQL, which is
//! [`nvs_stdlib::queue::Split`]'s trade and not this module's: the claim's `select` and its
//! `update` are one moment or they are nothing, and a backend without the construct that makes them
//! one statement pays a transaction for the same property. On SQLite those round trips are calls
//! into a file this process opened rather than messages on a socket, which is the cheap end of that
//! trade rather than a new one. It is
//! `workers` connections against the deployment's `max_connections` and the operator wrote the
//! number; `rule:security/db-pool-reset-is-a-boundary`'s pool is deliberately not involved, because a pool exists to be handed
//! between requests and this connection belongs to one task for its whole life. A turn that claims
//! spends one isolate on top of that — its own arena and budget, sharing only the compiled unit,
//! which [`crate::script`]'s cache holds for the run so a queue draining ten jobs off one script
//! compiles it once.
//!
//! ## What `[queue] workers` buys on SQLite
//!
//! One worker's throughput, and a number above that buys waiting rather than parallelism. SQLite
//! has a single writer and every claim here is an immediate transaction
//! (`rule:concurrency/claiming-is-one-statement`), so a second worker against the same file waits
//! out the first one's write lock instead of proceeding beside it. That is a property of the
//! database rather than a defect of the queue, and nothing here shards, locks or opens a second
//! file to work around it.
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
//! **Every attempt's error reaches the dead-letter row through the row itself.** [`report`] writes
//! every attempt back — `Succeeded`, back to `Pending` on § 6's ladder, or out of `nvs_jobs` and
//! into `nvs_dead_jobs` where the job has used its last attempt — and each of the two failing
//! endings binds what [`nvs_stdlib::queue::dead_errors`] appends to the `errors` column the claim
//! answered with. A worker therefore keeps no history of its own, which is what makes the fleet
//! interchangeable: the array lives on the row between attempts, so the worker that dead-letters a
//! job is under no obligation to be one that saw it fail before.
//!

use std::cell::Cell;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nvs_config::queue::QueueBounds;
use nvs_config::tree::Database;
use nvs_runtime::host::{Woken, with_current};
use nvs_stdlib::queue::Bell;

/// How long a worker's own handshake may take.
///
/// Much shorter than [`crate::queue`]'s own connect deadline, and for the opposite reason: that
/// command exists to reach the server and has nothing else to do, while a worker is a passenger on
/// a run whose script may finish in milliseconds. An unreachable server costs the run this much and
/// then stops costing it anything.
const CONNECT_DEADLINE: Duration = Duration::from_secs(2);

/// How long a worker that found no due work waits before it asks again, when nothing wakes it.
///
/// The bound on how late work may start that nothing in this process announced, and the module
/// doc's *What an idle worker waits on* section lists what that is. It is also what an idle
/// `[queue]` costs: one roster statement per worker per wait. One fixed interval, so there is no
/// backoff whose current rung a job's delay would depend on and no `[queue]` key for it.
const IDLE_WAIT: Duration = Duration::from_secs(1);

/// How often [`Crew::keep`] looks for its last workers to have returned, once the drain has begun.
///
/// Short, because it is the tail of a shutdown and runs only during one.
const RETURN_POLL: Duration = Duration::from_millis(10);

/// How long one of a worker's exchanges with its server may take.
///
/// Filed per exchange and not once per connection, which is the whole of what
/// `nvs_db::Connection::set_deadline` bounds: a statement is a send and every row of the answer, and
/// the work *between* two statements — a claimed job's own isolate, which is unbounded by design —
/// is not part of either. A worker that filed none would park forever on a server that stopped
/// answering without closing the socket, and this task is stopped by a flag read between turns
/// rather than by a cancellation, so nothing else would ever end it.
///
/// Three orders of magnitude above a healthy claim's round trip, which is milliseconds against a
/// server on the same host or the same rack. What it costs is the other side of that: a worker
/// whose server went away mid-statement ends after this rather than at once, so a drain arriving in
/// that window waits this long.
const STATEMENT_DEADLINE: Duration = Duration::from_secs(10);

/// The root every worker task takes.
///
/// `rule:http-server/containment-does-not-end-at-the-helper`: a worker has no request beneath it to
/// charge a fault to, so a panic that reaches here retires the worker rather than being answered as
/// one request's `500`. Named rather than written inline at the [`start`] that spawns with it,
/// because the other spelling is three lines from where `nvs serve` arms these — the `[[schedule]]`
/// ticker holds `TaskRoot::Request`, a fire being a child of the loop that serves — and this is the
/// one line of that arming which looks right when it is wrong.
const ROOT: nvs_runtime::TaskRoot = nvs_runtime::TaskRoot::Worker;

/// The switch that stops every worker a binary started.
///
/// One switch for all of them rather than one each: they stop together, at the same moment and for
/// the same reason. It is [`Clone`] because both ends hold it — whatever says stop, and every
/// worker that reads it — and a clone is the same switch, not a second one.
///
/// **Which end says stop is the one thing the two binaries differ in.** [`Workers::new`] is
/// `nvs run`'s, the flag alone; [`Workers::draining`] is `nvs serve`'s, the same flag with the
/// drain beside it, because a served instance has no script whose exit could set one. [`start`] is
/// one function over both, so moving queue work between the two is an operational decision and
/// never a behavioural one (`rule:concurrency/who-runs-a-job-is-configuration`).
///
/// Built by the caller rather than by [`start`], because the two are spawned in the order the
/// module doc's *What a run pays* section fixes: the script's task first, and it needs this in its
/// body before any worker exists.
#[derive(Clone)]
pub(crate) struct Workers {
    /// An [`Rc`] and a [`Cell`] because both ends are tasks on one core — there is no thread here
    /// to synchronize with.
    stop: Rc<Cell<bool>>,
    /// The drain a served instance stops on, and `None` for a run that has none to read.
    ///
    /// A handle rather than the process's bit read here, because who may write that bit is
    /// [`nvs_server::Draining`]'s own decision and a worker is not one of them: what arrives is
    /// whichever handle the caller holds, which is the process's under `nvs serve` and a detached
    /// one under a test.
    draining: Option<nvs_server::Draining>,
    /// The bell a worker with nothing to claim waits on. Both constructors take the process's,
    /// which is the one `Core\Queue::push` rings, and [`Workers::stop`] rings whichever this is.
    bell: Bell,
    /// How long that worker waits when nothing wakes it: [`IDLE_WAIT`] from both constructors.
    idle: Duration,
}

impl Workers {
    /// A fresh switch, in the position every worker keeps running in.
    pub(crate) fn new() -> Self {
        Self {
            stop: Rc::new(Cell::new(false)),
            draining: None,
            bell: Bell::process(),
            idle: IDLE_WAIT,
        }
    }

    /// The same switch with a drain beside it, for a binary whose shutdown is a drain.
    pub(crate) fn draining(draining: nvs_server::Draining) -> Self {
        Self {
            draining: Some(draining),
            ..Self::new()
        }
    }

    /// Tells every worker this run started to finish its turn and return, and wakes the ones
    /// waiting for work so that none of them waits out [`IDLE_WAIT`] first.
    pub(crate) fn stop(&self) {
        // The flag first: a worker the ring wakes reads it at the top of its next turn.
        self.stop.set(true);
        self.bell.ring();
    }

    /// Whether a worker reading this may open another turn.
    ///
    /// Either end is enough and neither is asked again once a turn has begun, which is the module
    /// doc's *Read at the top of a turn* paragraph as one expression.
    fn stopping(&self) -> bool {
        self.stop.get()
            || self
                .draining
                .as_ref()
                .is_some_and(nvs_server::Draining::is_draining)
    }

    /// Waits for work after a turn that found none, reporting how the wait ended.
    ///
    /// `heard` is [`Bell::rings`] as the caller read it before that turn. A ring since then is a
    /// stop, or a job the turn may have asked too early to see, so the wait is not entered at all.
    /// Otherwise the task parks until the bell rings, the drain this switch reads begins, or
    /// [`Workers::idle`] has passed. Whichever of them ends it, the caller reads
    /// [`Workers::stopping`] and asks the database again: a ring says nothing about what the next
    /// turn will find.
    ///
    /// **A resume is not one of the three.** A wake is a hint
    /// (`rule:concurrency/the-parking-contract` rule 2), and a task can be resumed by a wake that
    /// was meant for an earlier wait — a blocking-pool answer the task read before it parked, whose
    /// wake the pool delivered afterwards. So the three are read again after every resume and the
    /// task parks again when none of them holds, against the deadline it started with. Ending the
    /// wait on a stray wake would cost the queue a statement
    /// `rule:concurrency/a-push-wakes-an-idle-worker` does not allow, and a stop that came a moment
    /// later would find the worker mid-turn.
    ///
    /// Both registrations are taken for this one wait, held across its re-parks, and let go when
    /// it ends, which costs the core no wake
    /// (`rule:concurrency/a-handle-given-up-by-its-task-wakes-nothing`).
    fn wait_for_work(&self, heard: u64) -> Woken {
        let Some(wake) = nvs_host::wake_this_task() else {
            // No task beneath the call, so nothing could deliver a wake and the clock is the only
            // thing this wait can end on.
            return pause(self.idle);
        };
        let deadline = Instant::now() + self.idle;
        let Some(_rung) = self.bell.wake_at_ring(heard, move || drop(wake.wake())) else {
            return Woken::Elapsed;
        };
        // A drain that has already begun fires its wake here, and that ends the first park below
        // at once.
        let _drained = self
            .draining
            .as_ref()
            .and_then(|draining| nvs_host::wake_at_drain(draining.bit()));
        loop {
            let woken = nvs_host::timer::wait_until(deadline);
            if woken == Woken::Cancelled
                || self.bell.rings() != heard
                || self.stopping()
                || Instant::now() >= deadline
            {
                return woken;
            }
        }
    }
}

/// Spawns `bounds.workers` worker tasks onto `sched`, each claiming out of `[db.<name>]`.
///
/// Nothing runs here — [`nvs_host::Scheduler::spawn`] only queues — so the caller is free to
/// install the reactor afterwards. `current` holds the configuration in force. Each job's context
/// is given its script's snapshot out of the publish it holds at the moment the job is claimed,
/// for the reason the module
/// doc's *What a job's grants are* section owns: a job's isolate is resolved against the context
/// that runs it. `bounds` names the connection and the count, and `visibility` is only the value a
/// turn falls back on.
///
/// `nvs run` starts its workers here, all on one switch, and never publishes a second snapshot, so
/// its holder keeps the one it was made with. `nvs serve` starts them through [`Crew`] instead,
/// one switch each, because a reload may stop some of them.
pub(crate) fn start(
    sched: &mut nvs_host::Scheduler,
    workers: &Workers,
    bounds: &QueueBounds,
    block: &Database,
    current: &Arc<nvs_config::Current>,
) {
    for _ in 0..bounds.workers {
        // The whole switch rather than its flag alone: which end stops these workers is carried
        // here rather than decided here, and [`Workers`]'s own doc owns the difference.
        let (ctx, body) = worker(workers.clone(), bounds, block, current);
        sched.spawn(ctx, ROOT, body);
    }
}

/// One worker's context and task body, claiming out of `[db.<bounds.connection>]` until `workers`
/// says stop.
///
/// The block is cloned rather than borrowed because a task's body is `'static`, and cloned per
/// worker rather than shared because a `Database` is a handful of strings read once at connect.
fn worker(
    workers: Workers,
    bounds: &QueueBounds,
    block: &Database,
    current: &Arc<nvs_config::Current>,
) -> (
    nvs_runtime::Ctx,
    impl FnOnce(&mut nvs_runtime::Ctx) + 'static,
) {
    let name = bounds.connection.clone();
    let block = block.clone();
    let visibility = bounds.visibility;
    let current = Arc::clone(current);
    let mut ctx = nvs_runtime::Ctx::stdout();
    ctx.set_config(current.load());
    (ctx, move |ctx: &mut nvs_runtime::Ctx| {
        claim_until_stopped(ctx, &workers, &name, &block, &current, visibility);
    })
}

/// The workers `nvs serve` runs, one [`Workers`] switch each, and the task that keeps them in step
/// with `[queue]` (`rule:config/reloadability-is-its-own-field`).
///
/// **A new count starts or stops workers, and a new connection replaces them all.** A worker that
/// is stopped finishes the job it holds and writes it back first, because its switch is read at
/// the top of a turn (the module doc's *Read at the top of a turn*). The workers that replace it
/// open their own connection and claim their first job from there. So for a moment the old
/// connection's last jobs and the new connection's first ones run side by side, and no claim is
/// ever left without a worker to report it.
///
/// **What it spends** is one task on the core that ticks, which wakes once per `every` and compares
/// two pointers, plus one switch per worker. The storage question the boot asks of a new
/// connection is asked by the reload before it publishes, off every core, so no worker on this
/// core waits on a catalog read.
pub(crate) struct Crew {
    /// The drain every worker also stops on.
    draining: nvs_server::Draining,
    /// The configuration in force, which each job runs under and which a reload replaces.
    current: Arc<nvs_config::Current>,
    /// The bounds and the block the running workers were started from, or `None` while none run.
    on: Option<(QueueBounds, Database)>,
    /// One switch per running worker, oldest first.
    members: Vec<Workers>,
    /// Cloned into every worker's task and dropped when that task returns, so its strong count
    /// less one is how many workers are still running, stopped ones included.
    alive: Rc<()>,
}

impl Crew {
    /// A crew with no worker, following `current`.
    pub(crate) fn new(draining: nvs_server::Draining, current: Arc<nvs_config::Current>) -> Self {
        Self {
            draining,
            current,
            on: None,
            members: Vec::new(),
            alive: Rc::new(()),
        }
    }

    /// How many workers are running and have not been told to stop.
    pub(crate) fn len(&self) -> usize {
        self.members.len()
    }

    /// Queues `bounds.workers` workers on `sched`, before the scheduler turns.
    pub(crate) fn arm(
        &mut self,
        sched: &mut nvs_host::Scheduler,
        bounds: &QueueBounds,
        block: &Database,
    ) {
        self.on = Some((bounds.clone(), block.clone()));
        for _ in 0..bounds.workers {
            let (ctx, body) = self.member();
            sched.spawn(ctx, ROOT, body);
        }
    }

    /// One more worker on the bounds and block in [`Crew::on`], with a switch of its own.
    fn member(
        &mut self,
    ) -> (
        nvs_runtime::Ctx,
        impl FnOnce(&mut nvs_runtime::Ctx) + 'static,
    ) {
        let (bounds, block) = self
            .on
            .as_ref()
            .expect("a member is only added while the crew has a connection");
        let switch = Workers::draining(self.draining.clone());
        self.members.push(switch.clone());
        let (ctx, body) = worker(switch, bounds, block, &self.current);
        let alive = Rc::clone(&self.alive);
        (ctx, move |ctx: &mut nvs_runtime::Ctx| {
            let _alive = alive;
            body(ctx);
        })
    }

    /// Brings the running workers to what `wanted` names, from inside the crew's own task.
    ///
    /// A different connection or block stops every running worker and starts `wanted`'s count on
    /// the new one. The same connection starts the workers the count gained or stops the newest
    /// ones it lost. `None` stops them all. A started worker is a child of the calling task,
    /// which is why [`Crew::keep`] outlives every worker it started.
    fn follow(&mut self, wanted: Option<(QueueBounds, Database)>) {
        let same = |(had, had_block): &(QueueBounds, Database),
                    (now, now_block): &(QueueBounds, Database)| {
            had.connection == now.connection && had_block == now_block
        };
        let moved = match (&self.on, &wanted) {
            (Some(had), Some(now)) => !same(had, now),
            (None, None) => false,
            _ => true,
        };
        let count = wanted.as_ref().map_or(0, |(bounds, _)| {
            usize::try_from(bounds.workers).unwrap_or(usize::MAX)
        });
        let keep = if moved {
            0
        } else {
            count.min(self.members.len())
        };
        let stopped = self.members.len() - keep;
        for switch in self.members.drain(keep..) {
            switch.stop();
        }
        if stopped > 0 {
            let (bounds, _) = self.on.as_ref().expect("a running worker has a connection");
            eprintln!(
                "note: {stopped} queue worker{} on `[db.{}]` stop{} after the current job",
                if stopped == 1 { "" } else { "s" },
                bounds.connection,
                if stopped == 1 { "s" } else { "" },
            );
        }
        self.on = wanted;
        let started = count - keep;
        for _ in 0..started {
            let (ctx, body) = self.member();
            if nvs_host::spawn_child(ctx, ROOT, body).is_none() {
                self.members.pop();
                eprintln!("warning: a queue worker was not started: no scheduler is turning here");
                return;
            }
        }
        if started > 0 {
            let (bounds, _) = self.on.as_ref().expect("a started worker has a connection");
            eprintln!(
                "note: {started} queue worker{} started on `[db.{}]`",
                if started == 1 { "" } else { "s" },
                bounds.connection,
            );
        }
    }

    /// The crew's task: every `every`, whether a reload published a new snapshot, and if it did,
    /// [`Crew::follow`] over what `wanted` resolves from it.
    ///
    /// A drain is read once per `every`, which bounds what it adds to a stop, as the schedule
    /// ticker's poll does. It then stops following and waits, one [`RETURN_POLL`] at a time, until
    /// every worker it ever started has returned. A worker it started is its child, and a child
    /// ends when its parent does, which would cut a job off in the middle.
    pub(crate) fn keep(
        mut self,
        every: Duration,
        wanted: impl Fn(&nvs_config::Config) -> Option<(QueueBounds, Database)>,
    ) {
        let mut seen = self.current.load();
        while !self.draining.is_draining() {
            if matches!(pause(every), Woken::Cancelled) {
                return;
            }
            let now = self.current.load();
            if Arc::ptr_eq(&now, &seen) {
                continue;
            }
            seen = now;
            self.follow(wanted(&seen.config));
        }
        while Rc::strong_count(&self.alive) > 1 {
            if matches!(pause(RETURN_POLL), Woken::Cancelled) {
                return;
            }
        }
    }
}

/// One worker's whole life: open the connection, then take turns until the run ends.
///
/// A statement that fails ends the worker rather than being retried. A connection is only usable at
/// a message boundary — on every driver — and a failed statement is not one, so the honest recovery
/// is a new connection, which is the next run's, since this one is by then within a few milliseconds
/// of its own end.
fn claim_until_stopped(
    ctx: &mut nvs_runtime::Ctx,
    workers: &Workers,
    name: &str,
    block: &Database,
    current: &nvs_config::Current,
    visibility: Duration,
) {
    // Asked before the connection is opened and not only at the top of a turn: this task is
    // spawned after the script's, so an ordinary CLI run has already finished by the time a worker
    // is first polled, and the module doc's *What a run pays* section is what that buys.
    if workers.stopping() {
        return;
    }
    let Some(mut conn) = open(name, block) else {
        return;
    };
    // A worker that stops on a failed statement says so, on the stream [`open`]'s own refusals use.
    // The failure this is written for is a claim naming a column the table has not got, which is
    // every deployment between a schema gaining one and `nvs queue migrate` being run: the worker
    // ends, the queue silently never drains, and a program polling `stats` sees a counter that
    // stays at zero with nothing anywhere to say why. Warning rather than fatal for
    // `rule:errors/escalation-ladder`'s reason — the run's own script is not this task's to end.
    if let Err(refused) = take_turns(workers, || turn(ctx, &mut conn, current, visibility)) {
        eprintln!("warning: the queue worker on `[db.{name}]` stopped: {refused}");
    }
}

/// The loop itself: a turn while [`Workers`] admits one, [`Workers::wait_for_work`] after a turn
/// that found nothing, and an end after a turn that failed, answering with what that turn refused
/// with.
///
/// Split from the connection above it because *when* the stop condition is read is the property
/// this has to keep — a drain arriving mid-claim must not cut the write-back short — and a turn a
/// case writes is what asserts an ordering over, where the whole of [`claim_until_stopped`] would
/// need a database standing up before it could be asked anything at all. The failure is handed back
/// rather than reported here for the same reason: this function is what a case drives, and a case
/// asserting that a refusal is not swallowed must be able to read it.
fn take_turns(workers: &Workers, mut turn: impl FnMut() -> io::Result<bool>) -> io::Result<()> {
    loop {
        // Read before the switch and before the claim, so a stop or a push that lands after
        // either has moved the count by the time this turn's wait is registered against it.
        let heard = workers.bell.rings();
        if workers.stopping() {
            return Ok(());
        }
        match turn() {
            // Something was claimed, so the roster may still hold more: turn again without
            // waiting, and the queue that answered drops out of the next roster by itself, because
            // a row this turn claimed is inside its visibility window.
            Ok(true) => {}
            Ok(false) => {
                if workers.wait_for_work(heard) == Woken::Cancelled {
                    return Ok(());
                }
            }
            Err(refused) => return Err(refused),
        }
    }
}

/// One turn: which queues have due work, then one claim against each.
///
/// The roster is asked first for the reason [`nvs_stdlib::queue::QUEUES_POSTGRES`] owns — § 2 names
/// no queues, so the table is the only place they are written down — at O(queues) a turn, which
/// that text's doc costs out. The two instants are
/// computed once here so that every claim in this turn judges due-ness against the same moment.
/// The visibility window is read from `current` at the start of the turn, so a reload that moves
/// `[queue] visibility` reaches the next turn. `boot` is the window used when the snapshot in
/// force resolves no queue.
fn turn(
    ctx: &mut nvs_runtime::Ctx,
    conn: &mut Wire,
    current: &nvs_config::Current,
    boot: Duration,
) -> io::Result<bool> {
    let now = nvs_stdlib::queue::now_millis();
    let cutoff = now.saturating_sub(window(&current.load(), boot));
    let mut claimed = false;
    conn.bound_next_exchange();
    for queue in roster(conn, now, cutoff)? {
        conn.bound_next_exchange();
        if let Some(job) = claim(conn, &queue, now, cutoff)? {
            // Run before the next queue is claimed against, rather than after the roster has been
            // walked: a claim this worker is holding is a job nothing else may take, so the
            // shortest time between the two is the one that costs a fleet the least. The write-back
            // rides with it for the same reason — the row is released by [`report`] and not by the
            // end of the turn.
            let failure = configure(ctx, current, &job)
                .err()
                .or_else(|| run(ctx, &job));
            // Filed again rather than once for the whole turn: the job above ran between the two
            // statements, and a clock that covered it would bound a write-back by how long
            // somebody else's code took.
            conn.bound_next_exchange();
            report(conn, &job, now, failure.as_ref())?;
            claimed = true;
        }
    }
    Ok(claimed)
}

/// Sets `ctx`'s configuration to the snapshot `job` runs under: the one the `[[app]]` blocks
/// matching the job's own script fold to (ADR 0271 § 2), out of the publish in force now, just
/// after the claim, and not the one this worker started with
/// (`rule:config/reloadability-is-its-own-field`).
///
/// # Errors
///
/// A script whose blocks do not fold is a refused attempt, for the reason [`run`] gives a script
/// that does not resolve: running it under the host's snapshot would hand it grants its own blocks
/// narrowed.
fn configure(
    ctx: &mut nvs_runtime::Ctx,
    current: &nvs_config::Current,
    job: &Job,
) -> Result<(), nvs_host::Failure> {
    match current.published().entry(
        std::path::Path::new(&job.script),
        &crate::config::LocalFiles,
    ) {
        Ok(snapshot) => {
            ctx.set_config(snapshot);
            Ok(())
        }
        Err(refused) => {
            eprintln!(
                "warning: the queued job `{}` was not run: {}",
                job.script, refused.message
            );
            Err(refusal(format!(
                "its configuration did not fold: {}",
                refused.message
            )))
        }
    }
}

/// `[queue] visibility` in milliseconds, as `snapshot` resolves it, or `boot` when it resolves no
/// queue.
///
/// A published snapshot was validated before it was published, so the fallback is reached only by a
/// tree whose `[queue]` block the reload removed. Saturating rather than wrapping for a
/// `visibility` no operator would write: the clamp makes every job's claim eligible again
/// immediately, which is a busy worker, where the wrap would make it eligible never.
fn window(snapshot: &nvs_config::Snapshot, boot: Duration) -> i64 {
    let visibility = nvs_config::queue::queue_for(&snapshot.config, &snapshot.origins)
        .ok()
        .flatten()
        .map_or(boot, |bounds| bounds.visibility);
    i64::try_from(visibility.as_millis()).unwrap_or(i64::MAX)
}

/// The queues holding work this worker could take, as [`nvs_stdlib::queue::QUEUES_POSTGRES`] and
/// its three siblings answer it, one text per dialect.
///
/// **The same values in the same order on every dialect**, and none of these texts is a
/// [`nvs_stdlib::queue::Split`] — so what the branch below is about is the walk over the answer and
/// never the parameters. What each arm builds for itself is the *binding*, because an instant is
/// the octets of its decimal text to a wire driver and the integer itself to SQLite. Where a
/// dialect *does* reorder its values, the caller reconciles it at the one site that already had to
/// branch: [`report`]'s retry.
fn roster(conn: &mut Wire, now: i64, cutoff: i64) -> io::Result<Vec<String>> {
    match conn.dialect() {
        Dialect::Postgres(postgres) => {
            let sending = [millis(now), millis(cutoff)];
            postgres_roster(postgres, &borrowed(&sending))
        }
        Dialect::Framed(mut framed) => {
            let sending = [millis(now), millis(cutoff)];
            framed_roster(&mut framed, &borrowed(&sending))
        }
        Dialect::SqlServer(tds) => {
            let sending = [millis(now), millis(cutoff)];
            tds_roster(tds, &borrowed(&sending))
        }
        Dialect::Sqlite(sqlite) => sqlite_roster(sqlite, now, cutoff),
    }
}

/// Owned octets as the borrowed, nullable slices a wire driver's `query` takes.
///
/// Nothing a worker sends is null — every value it binds is an id, an instant or a queue's own
/// name — so this is the shape of a parameter list rather than a decision about anything in one.
fn borrowed(values: &[Vec<u8>]) -> Vec<Option<&[u8]>> {
    values.iter().map(|one| Some(one.as_slice())).collect()
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

/// [`roster`] over TDS, which answers one statement's one column as the wire drivers above do.
///
/// **Not shared with [`framed_roster`] for [`framed_roster`]'s own reason**, one protocol further
/// along: a value is read here against the column definition it arrived under too, and
/// `nvs_db::tds::TdsScalar`'s rows are not `nvs_db::MySqlScalar`'s.
fn tds_roster(tds: &mut nvs_db::TdsConn, bound: &[Option<&[u8]>]) -> io::Result<Vec<String>> {
    let mut answered = tds.query(nvs_stdlib::queue::QUEUES_SQLSERVER, bound)?;
    // Described before the first row for [`postgres_roster`]'s reason, and needed whatever that
    // reason: `nvs_db::tds::scalar` reads a value against the definition it arrived under.
    let columns: Vec<nvs_db::tds::TdsColumn> = answered.columns().to_vec();
    let mut names = Vec::new();
    while let Some(row) = answered.next_row()? {
        let (Some(column), Some(body)) = (columns.first(), row.column(0)) else {
            continue;
        };
        if let nvs_db::tds::TdsScalar::Text(name) = nvs_db::tds::scalar(column, body)? {
            names.push(name.into_owned());
        }
    }
    Ok(names)
}

/// [`roster`] over SQLite, whose rows are in hand by the time the statement answers.
///
/// **No column definition to read a value against**, which is the one way this walk is shorter than
/// the two above it: SQLite stores a value as one of five storage classes whatever the column was
/// declared as, so a cell arrives as the class it is and [`nvs_db::SqliteValue`] is both halves of
/// what a column and its scalar are elsewhere.
fn sqlite_roster(sqlite: &nvs_db::SqliteConn, now: i64, cutoff: i64) -> io::Result<Vec<String>> {
    let mut answered = sqlite.query(
        nvs_stdlib::queue::QUEUES_SQLITE,
        vec![
            nvs_db::SqliteValue::Int(now),
            nvs_db::SqliteValue::Int(cutoff),
        ],
    )?;
    let mut names = Vec::new();
    while let Some(row) = answered.next_row() {
        if let Some(nvs_db::SqliteValue::Text(name)) = row.into_iter().next() {
            names.push(name);
        }
    }
    Ok(names)
}

/// What a worker reads off [`nvs_stdlib::queue::CLAIM_POSTGRES`]'s `returning` list, and what running one
/// and reporting it needs.
///
/// Every column of that list: what to run, what it runs under, and what § 6's ladder is judged
/// against, which [`report`] does the moment [`run`] returns.
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
    /// The `errors` column as it is stored: what the attempts before this one threw, or `None` for a
    /// job that has not failed yet. Never parsed here — it is handed back to
    /// [`nvs_stdlib::queue::dead_errors`], which is the one place an entry is appended, so this
    /// crate carries the array as the text the column holds.
    errors: Option<String>,
    /// The `grants` column as it is stored: the JSON array of capability names the context that
    /// enqueued the job had narrowed itself to, or `None` for a context that narrowed none. Held as
    /// the text the column carries for [`Self::errors`]'s reason — `nvs-stdlib` wrote it and
    /// `nvs_stdlib::queue::narrowing` is the one place it is read back into the shape the spawn
    /// door takes.
    grants: Option<String>,
    /// The `limits` column as it is stored: the JSON object of `[limits]` keys, each in the
    /// spelling `nvs.toml` writes the same directive in, or `None` for a context under no sub-cap
    /// at all. Read back beside the field above and by the same function.
    limits: Option<String>,
}

/// One claim against one queue, answering with the row it took.
///
/// **The same values in the same order on every dialect but one**: the queue, then now, then the
/// cutoff. [`nvs_stdlib::queue::CLAIM_MYSQL`] reads the queue in both of its arms, and a `?` cannot
/// be named twice, so that dialect is sent the queue again before the cutoff. The binding is each
/// arm's own, for [`roster`]'s reason.
fn claim(conn: &mut Wire, queue: &str, now: i64, cutoff: i64) -> io::Result<Option<Job>> {
    match conn.dialect() {
        Dialect::Postgres(postgres) => {
            let sending = wire_claim(queue, now, cutoff);
            postgres_claim(postgres, &borrowed(&sending))
        }
        Dialect::Framed(mut framed) => {
            let [queue, now_text, cutoff] = wire_claim(queue, now, cutoff);
            let sending = [queue.clone(), now_text, queue, cutoff];
            framed_claim(&mut framed, &borrowed(&sending), now)
        }
        Dialect::SqlServer(tds) => {
            let sending = wire_claim(queue, now, cutoff);
            tds_claim(tds, &borrowed(&sending))
        }
        Dialect::Sqlite(sqlite) => sqlite_claim(sqlite, queue, now, cutoff),
    }
}

/// A claim's three values as a wire driver binds them: the queue's own octets, then each instant as
/// the decimal text a `$n::bigint` and a `?` alike are sent as.
fn wire_claim(queue: &str, now: i64, cutoff: i64) -> [Vec<u8>; 3] {
    [queue.as_bytes().to_vec(), millis(now), millis(cutoff)]
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
        let [
            Some(script),
            Some(args),
            Some(errors),
            Some(grants),
            Some(limits),
        ] = [SCRIPT, ARGS, ERRORS, GRANTS, LIMITS].map(|at| columns.get(at))
        else {
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
        // Null here is the ordinary shape too — a job none of whose attempts has failed — and
        // `dead_errors` reads that as an array to start rather than one to append to.
        let errors = match errors.scalar(row.column(ERRORS)?)? {
            nvs_db::PgScalar::Text(errors) => Some(errors.into_owned()),
            _ => None,
        };
        // And a null in either of the last two is the enqueuing context that narrowed nothing,
        // which is a job held to the deployment's own ceiling rather than a row to drop.
        let grants = match grants.scalar(row.column(GRANTS)?)? {
            nvs_db::PgScalar::Text(grants) => Some(grants.into_owned()),
            _ => None,
        };
        let limits = match limits.scalar(row.column(LIMITS)?)? {
            nvs_db::PgScalar::Text(limits) => Some(limits.into_owned()),
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
            errors,
            grants,
            limits,
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

/// `backoff_ms`'s position in the same list.
const BACKOFF: usize = 5;

/// `errors`'s position in the same list.
const ERRORS: usize = 6;

/// `grants`'s position in the same list.
const GRANTS: usize = 7;

/// `limits`'s position in the same list, and the last of them — where a column a claim answers with
/// arrives, for the reason [`nvs_stdlib::queue::schema`]'s own column list gives.
const LIMITS: usize = 8;

/// [`claim`] as one statement in T-SQL: [`nvs_stdlib::queue::CLAIM_SQLSERVER`]'s updating CTE,
/// which takes the row and answers with it in one round trip as PostgreSQL's does.
///
/// **The same walk as [`postgres_claim`] and deliberately not shared with it**, for
/// [`framed_roster`]'s reason. Every row is drained before the answer is judged, for
/// [`postgres_claim`]'s reason, and `top 1` inside the statement is what makes that at most one
/// row.
fn tds_claim(tds: &mut nvs_db::TdsConn, bound: &[Option<&[u8]>]) -> io::Result<Option<Job>> {
    let mut answered = tds.query(nvs_stdlib::queue::CLAIM_SQLSERVER, bound)?;
    // Described before the first row for [`postgres_roster`]'s reason, and read against those
    // definitions for [`tds_roster`]'s.
    let columns: Vec<nvs_db::tds::TdsColumn> = answered.columns().to_vec();
    let mut took = None;
    while let Some(row) = answered.next_row()? {
        let [
            Some(script),
            Some(args),
            Some(errors),
            Some(grants),
            Some(limits),
        ] = [SCRIPT, ARGS, ERRORS, GRANTS, LIMITS].map(|at| columns.get(at))
        else {
            continue;
        };
        let [
            Some(script_body),
            Some(args_body),
            Some(errors_body),
            Some(grants_body),
            Some(limits_body),
        ] = [SCRIPT, ARGS, ERRORS, GRANTS, LIMITS].map(|at| row.column(at))
        else {
            continue;
        };
        let nvs_db::tds::TdsScalar::Text(script) = nvs_db::tds::scalar(script, script_body)? else {
            continue;
        };
        // A text column that is null is § 3's job with no payload, which [`postgres_claim`] reads
        // the same way and for the same reason: it is the ordinary shape of a job that needs none.
        let args = match nvs_db::tds::scalar(args, args_body)? {
            nvs_db::tds::TdsScalar::Text(args) => Some(args.into_owned()),
            _ => None,
        };
        // Null is the ordinary shape here too, for [`postgres_claim`]'s reason: a job none of whose
        // attempts has failed.
        let errors = match nvs_db::tds::scalar(errors, errors_body)? {
            nvs_db::tds::TdsScalar::Text(errors) => Some(errors.into_owned()),
            _ => None,
        };
        // And a null in either of the last two is the enqueuing context that narrowed nothing,
        // read the same way and for [`postgres_claim`]'s reason.
        let grants = match nvs_db::tds::scalar(grants, grants_body)? {
            nvs_db::tds::TdsScalar::Text(grants) => Some(grants.into_owned()),
            _ => None,
        };
        let limits = match nvs_db::tds::scalar(limits, limits_body)? {
            nvs_db::tds::TdsScalar::Text(limits) => Some(limits.into_owned()),
            _ => None,
        };
        // The columns the write-back judges against, every one of them `not null` in the migration
        // — [`postgres_claim`]'s own comment owns why a row missing one is dropped rather than run.
        let [Some(id), Some(attempts), Some(max_attempts), Some(backoff)] =
            [ID, ATTEMPTS, MAX_ATTEMPTS, BACKOFF].map(|at| columns.get(at))
        else {
            continue;
        };
        let [
            Some(id_body),
            Some(attempts_body),
            Some(max_body),
            Some(backoff_body),
        ] = [ID, ATTEMPTS, MAX_ATTEMPTS, BACKOFF].map(|at| row.column(at))
        else {
            continue;
        };
        let (
            nvs_db::tds::TdsScalar::Int(id),
            nvs_db::tds::TdsScalar::Int(attempts),
            nvs_db::tds::TdsScalar::Int(max_attempts),
            nvs_db::tds::TdsScalar::Int(backoff_ms),
        ) = (
            nvs_db::tds::scalar(id, id_body)?,
            nvs_db::tds::scalar(attempts, attempts_body)?,
            nvs_db::tds::scalar(max_attempts, max_body)?,
            nvs_db::tds::scalar(backoff, backoff_body)?,
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
            errors,
            grants,
            limits,
        });
    }
    Ok(took)
}

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
///
/// **The transaction is `read committed`**, which costs one more round trip on this protocol. Under
/// InnoDB's default `repeatable read`, each arm of the claim also locks the index record just past
/// its range, and `skip locked` waits on that record rather than stepping over it. Two workers
/// would then queue behind each other's claims. At `read committed` a row that does not match
/// keeps no lock.
fn framed_claim(
    framed: &mut Framed<'_>,
    bound: &[Option<&[u8]>],
    now: i64,
) -> io::Result<Option<Job>> {
    framed.begin(Some(nvs_db::Isolation::ReadCommitted), false)?;
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
/// The columns are at the ordinals the constants above name, on this dialect as on the others:
/// `all_three_dialects_answer_a_claim_with_the_same_columns` in `nvs-stdlib` is what holds every
/// `select` list to one set of positions, so nothing here is a second reading of § 4's list.
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
        if read.len() <= LIMITS {
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
        // And null `errors` is a job none of whose attempts has failed, read the same way.
        let errors = match &read[ERRORS] {
            nvs_db::MySqlScalar::Text(errors) => Some(errors.to_string()),
            _ => None,
        };
        // A null in either of the last two is the enqueuing context that narrowed nothing, which
        // is a job held to the deployment's own ceiling rather than a row to drop.
        let grants = match &read[GRANTS] {
            nvs_db::MySqlScalar::Text(grants) => Some(grants.to_string()),
            _ => None,
        };
        let limits = match &read[LIMITS] {
            nvs_db::MySqlScalar::Text(limits) => Some(limits.to_string()),
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
            errors,
            grants,
            limits,
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

/// [`claim`] as the pair SQLite spells, inside the immediate transaction that *is* the claim.
///
/// **The transaction is the mutual exclusion here and there is no locking clause to add**, which is
/// `rule:concurrency/claiming-is-one-statement`'s: this backend has one writer, so inside
/// `begin_immediate` no second connection is writing at all and two workers cannot come back with
/// one row. The one that arrives second waits for the write lock, and by the time it proceeds the
/// row the first took is no longer due.
///
/// **Immediate rather than the deferred transaction a bare `BEGIN` opens**, for the reason
/// `nvs_db::sqlite::SqliteConn::begin_immediate`'s own doc gives: a transaction that reads a row
/// and then writes it back asks to upgrade a shared lock, and SQLite refuses an upgrade without
/// honouring the busy timeout, so a deferred claim fails under exactly the concurrency it exists to
/// survive.
///
/// A refusal rolls back before it propagates, for [`framed_claim`]'s reason.
fn sqlite_claim(
    sqlite: &nvs_db::SqliteConn,
    queue: &str,
    now: i64,
    cutoff: i64,
) -> io::Result<Option<Job>> {
    sqlite.begin_immediate()?;
    match sqlite_claimed_in_two(sqlite, queue, now, cutoff) {
        Ok(took) => {
            sqlite.commit()?;
            Ok(took)
        }
        Err(refused) => {
            let _undone = sqlite.roll_back();
            Err(refused)
        }
    }
}

/// [`nvs_stdlib::queue::CLAIM_SQLITE`]'s row, then its `then` against the id that row named —
/// inside the transaction [`sqlite_claim`] opened.
///
/// The columns are at the ordinals the constants above name, on this dialect as on the others:
/// `all_three_dialects_answer_a_claim_with_the_same_columns` in `nvs-stdlib` is what holds every
/// `select` list to one set of positions, so nothing here is a second reading of § 4's list.
fn sqlite_claimed_in_two(
    sqlite: &nvs_db::SqliteConn,
    queue: &str,
    now: i64,
    cutoff: i64,
) -> io::Result<Option<Job>> {
    let split = nvs_stdlib::queue::CLAIM_SQLITE;
    let mut answered = sqlite.query(
        split.first,
        vec![
            nvs_db::SqliteValue::Text(queue.to_owned()),
            nvs_db::SqliteValue::Int(now),
            nvs_db::SqliteValue::Int(cutoff),
        ],
    )?;
    let mut took = None;
    while let Some(row) = answered.next_row() {
        // `limit 1`, so this guard is about the shape of the loop and not about a second row.
        if took.is_some() {
            continue;
        }
        // A row narrower than § 4's list is one no `Core\Queue::push` wrote, and it is dropped for
        // [`postgres_claim`]'s reason: it stays claimed until § 4's visibility timeout, which is
        // where a row this worker cannot make sense of belongs.
        if row.len() <= LIMITS {
            continue;
        }
        let nvs_db::SqliteValue::Text(script) = &row[SCRIPT] else {
            continue;
        };
        // A `text` cell that is null is § 3's job with no payload — the ordinary shape of a job
        // that needs none, so it is `None` rather than a skip.
        let args = match &row[ARGS] {
            nvs_db::SqliteValue::Text(args) => Some(args.clone()),
            _ => None,
        };
        // And a null `errors` cell is a job none of whose attempts has failed, read the same way.
        let errors = match &row[ERRORS] {
            nvs_db::SqliteValue::Text(errors) => Some(errors.clone()),
            _ => None,
        };
        // A null cell in either of the last two is the enqueuing context that narrowed nothing,
        // which is a job held to the deployment's own ceiling rather than a row to drop.
        let grants = match &row[GRANTS] {
            nvs_db::SqliteValue::Text(grants) => Some(grants.clone()),
            _ => None,
        };
        let limits = match &row[LIMITS] {
            nvs_db::SqliteValue::Text(limits) => Some(limits.clone()),
            _ => None,
        };
        let [
            Some(id),
            Some(attempts),
            Some(max_attempts),
            Some(backoff_ms),
        ] = [ID, ATTEMPTS, MAX_ATTEMPTS, BACKOFF].map(|at| sqlite_integer(&row[at]))
        else {
            continue;
        };
        took = Some(Job {
            id,
            script: script.clone(),
            args,
            attempts,
            max_attempts,
            backoff_ms,
            errors,
            grants,
            limits,
        });
    }
    // The rows have to have let the connection go before the `update` on it starts: a result set
    // holds this connection until it is dropped, which is § 4's one-statement-at-a-time rule as
    // this driver keeps it.
    drop(answered);
    let Some(job) = took else {
        return Ok(None);
    };
    // § 4's mark, keyed by the id the `select` named. The values are the `set` clause's and the
    // `where` clause's in that order, because a `?` is bound where it stands — the same reason
    // [`nvs_stdlib::queue::RETRY_MYSQL`] orders its own values differently.
    sqlite_apply(
        sqlite,
        split.then,
        vec![
            nvs_db::SqliteValue::Int(now),
            nvs_db::SqliteValue::Int(job.id),
        ],
    )?;
    Ok(Some(job))
}

/// One integer column of a claim as SQLite stored it.
///
/// [`integer`]'s twin, and shorter for [`sqlite_roster`]'s reason and one of its own: there is a
/// single integer storage class and it is signed, so there is no unsigned variant to accept beside
/// this one and no width for a column an operator widened to arrive as.
fn sqlite_integer(read: &nvs_db::SqliteValue) -> Option<i64> {
    match read {
        nvs_db::SqliteValue::Int(at) => Some(*at),
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
/// **Narrowed to what the row recorded, and read before anything else is spent.** The module doc's
/// *What a job's grants are* owns why the pair is the answer and the worker's own configuration
/// only the ceiling. A pair [`nvs_stdlib::queue::narrowing`] cannot read is a refused attempt
/// rather than a job run at that ceiling: running it there is exactly the widening the two columns
/// exist to prevent, and § 6's ladder bounds a refusal where nothing bounds an escalation.
///
/// A refusal is written to standard error rather than answered, because there is nobody to answer:
/// a worker has no caller. One line per refused job, and the answer is a failure either way — a job
/// whose script does not resolve is a failed attempt like any other, so § 6's ladder is what
/// bounds it rather than a second policy written here. It crosses as the same [`nvs_host::Failure`]
/// a throw does, under [`refusal`]'s class, so a dead-letter row records the two in one shape.
fn run(ctx: &mut nvs_runtime::Ctx, job: &Job) -> Option<nvs_host::Failure> {
    let Some(narrowing) =
        nvs_stdlib::queue::narrowing(job.grants.as_deref(), job.limits.as_deref())
    else {
        eprintln!(
            "warning: the queued job `{}` was not run: the grants and limits recorded with it are \
             not the shape a push writes",
            job.script
        );
        return Some(refusal(
            "the grants and limits recorded with it could not be read".to_string(),
        ));
    };
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
    match nvs_host::Isolate::new(program, args, nvs_host::Output::Capture)
        .narrowed_by(narrowing)
        .run(ctx)
    {
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
            // Every attempt's failure reaches the row's `errors` array as well, and this line is
            // still where one becomes visible without a query: a job that recovers on a later
            // attempt is never dead-lettered, so nothing but the log ever reports the attempts it
            // spent — and a queue whose failures are silent is the one thing § 6 exists to prevent.
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

/// The `errors` array an attempt leaves behind, whichever of [`report`]'s two failing endings writes
/// it.
///
/// **One function because the two endings owe the same value**, and because that is this crate's
/// whole half of `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`'s promise:
/// [`nvs_stdlib::queue::dead_errors`] appends an entry to an array it is *handed*, so what a worker
/// owes is handing it the column the claim answered with rather than starting a fresh one. A retry
/// that bound a fresh array would keep the count and lose the history, and the job would reach
/// `nvs_dead_jobs` carrying its last failure alone.
fn errors_after(job: &Job, held_at: i64, failure: &nvs_host::Failure) -> String {
    nvs_stdlib::queue::dead_errors(
        job.errors.as_deref(),
        held_at,
        &failure.class,
        &failure.message,
    )
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
            // The same two values in the same order once more: a marker carries its own number in
            // this dialect, so the lease keying is PostgreSQL's numbering rather than a position.
            Dialect::SqlServer(tds) => {
                tds_apply(tds, nvs_stdlib::queue::SUCCEEDED_SQLSERVER, &lease)
            }
            // The same two values again as the integers this driver binds, against the text the
            // arm above sends: what has to survive is the keying and not the spelling.
            Dialect::Sqlite(sqlite) => sqlite_apply(
                sqlite,
                nvs_stdlib::queue::SUCCEEDED_SQLITE,
                sqlite_lease(job.id, held_at),
            ),
        };
    };
    if job.attempts >= job.max_attempts {
        // § 6's floor: the attempt was the job's last, so the row moves rather than being armed
        // again — one statement, keyed on the same lease, which is where "never deleted by the
        // runtime" is actually kept. `>=` and not `==` because `[queue] max_attempts` is
        // configuration an operator can lower under a job that has already used more than the new
        // bound, and such a row is exhausted rather than owed an attempt it can no longer have.
        let errors = errors_after(job, held_at, failure);
        // The instant the attempt *ended*, read here rather than taken from the claim, for the
        // reason the retry's own `run_at` is: the attempt has just spent however long it spent.
        let failed_at = nvs_stdlib::queue::now_millis();
        let failed = millis(failed_at);
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
            Dialect::SqlServer(tds) => {
                tds_dead_letter_in_two(tds, &id, &held, failed.as_slice(), errors.as_bytes())
            }
            Dialect::Sqlite(sqlite) => {
                sqlite_dead_letter_in_two(sqlite, job.id, held_at, failed_at, &errors)
            }
        };
    }
    let due_at = nvs_stdlib::queue::retry_at(
        nvs_stdlib::queue::now_millis(),
        job.attempts,
        job.backoff_ms,
        job.id,
    );
    let due = millis(due_at);
    // The array this attempt leaves on the row, which is what makes the dead-letter row that
    // eventually carries it carry every attempt: the write-back that arms a job again is the only
    // place an attempt's failure becomes durable, since the next claim may be another worker's.
    let errors = errors_after(job, held_at, failure);
    match conn.dialect() {
        Dialect::Postgres(postgres) => apply(
            postgres,
            nvs_stdlib::queue::RETRY_POSTGRES,
            &[
                Some(id.as_slice()),
                Some(held.as_slice()),
                Some(due.as_slice()),
                Some(errors.as_bytes()),
            ],
        ),
        // **The one place the two dialects part on what is sent**, and this is the site
        // [`nvs_stdlib::queue::RETRY_MYSQL`]'s doc means when it says the caller is where the two
        // orders are reconciled: the `run_at` and the `errors` array it writes are in the `set`
        // clause, which is left of the `where`, and a `?` is bound by the position it occupies. A
        // worker sending PostgreSQL's order into that text would push every job's next attempt out
        // to its own id.
        Dialect::Framed(mut framed) => apply_framed(
            &mut framed,
            nvs_stdlib::queue::RETRY_MYSQL,
            &[
                Some(due.as_slice()),
                Some(errors.as_bytes()),
                Some(id.as_slice()),
                Some(held.as_slice()),
            ],
        ),
        // PostgreSQL's order and not the framed one, which that constant's doc owns: a `@pn` is
        // named where its value is wanted, so the new `run_at` is the third value here as it is
        // there, and the `set` clause standing left of the `where` forces nothing.
        Dialect::SqlServer(tds) => tds_apply(
            tds,
            nvs_stdlib::queue::RETRY_SQLSERVER,
            &[
                Some(id.as_slice()),
                Some(held.as_slice()),
                Some(due.as_slice()),
                Some(errors.as_bytes()),
            ],
        ),
        // The same values in the same order, forced by the same `set` clause: this is that
        // constant's own text rather than a transcription of it. The array crosses as text here,
        // where the two instants cross as the integers this driver binds.
        Dialect::Sqlite(sqlite) => sqlite_apply(
            sqlite,
            nvs_stdlib::queue::RETRY_SQLITE,
            vec![
                nvs_db::SqliteValue::Int(due_at),
                nvs_db::SqliteValue::Text(errors),
                nvs_db::SqliteValue::Int(job.id),
                nvs_db::SqliteValue::Int(held_at),
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

/// § 6's move in T-SQL: the copy, then the delete, inside one transaction.
///
/// [`dead_letter_in_two`]'s twin, and its doc owns the whole of why the pair is the shape — the
/// copy runs first because a `delete` cannot answer with what it removed, and both halves are keyed
/// on the lease. [`nvs_stdlib::queue::DEAD_LETTER_SQLSERVER`]'s own doc owns why the
/// `delete … output deleted.* into` this dialect could spell is refused.
///
/// The rollback is [`framed_claim`]'s, for its reason.
fn tds_dead_letter_in_two(
    tds: &mut nvs_db::TdsConn,
    id: &[u8],
    held: &[u8],
    failed: &[u8],
    errors: &[u8],
) -> io::Result<()> {
    let split = nvs_stdlib::queue::DEAD_LETTER_SQLSERVER;
    // The two values the copy adds stand before the two the lease is keyed on, exactly as in the
    // framed pair, because that is where they stand in the text.
    let copying: [Option<&[u8]>; 4] = [Some(failed), Some(errors), Some(id), Some(held)];
    let removing: [Option<&[u8]>; 2] = [Some(id), Some(held)];
    tds.begin(None, false)?;
    let moved =
        tds_apply(tds, split.first, &copying).and_then(|()| tds_apply(tds, split.then, &removing));
    match moved {
        Ok(()) => {
            tds.commit()?;
            Ok(())
        }
        Err(refused) => {
            let _undone = tds.roll_back();
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

/// [`apply`] over TDS.
///
/// The affected count is discarded for [`apply_framed`]'s reason, which is [`report`]'s.
fn tds_apply(tds: &mut nvs_db::TdsConn, sql: &str, bound: &[Option<&[u8]>]) -> io::Result<()> {
    tds.execute_many(sql, &[bound])?;
    Ok(())
}

/// [`apply`] over SQLite, whose `execute` and `query` are the same call.
///
/// The affected count is discarded for [`apply_framed`]'s reason, and here there is a second one:
/// on this driver the count a statement with no result set reports is what [`report`] would have to
/// read it out of anyway, and `report`'s doc owns why it does not judge it.
fn sqlite_apply(
    sqlite: &nvs_db::SqliteConn,
    sql: &str,
    bound: Vec<nvs_db::SqliteValue>,
) -> io::Result<()> {
    sqlite.execute_many(sql, vec![bound])?;
    Ok(())
}

/// A write-back's lease as SQLite binds it: the row's id and the `claimed_at` this worker's own
/// claim wrote, in the order every statement keyed on it names them.
fn sqlite_lease(id: i64, held_at: i64) -> Vec<nvs_db::SqliteValue> {
    vec![
        nvs_db::SqliteValue::Int(id),
        nvs_db::SqliteValue::Int(held_at),
    ]
}

/// § 6's move as SQLite runs it, which is [`nvs_stdlib::queue::DEAD_LETTER_SQLITE`] — MySQL's own
/// pair — inside the immediate transaction that makes the two one moment.
///
/// The copy runs first and both halves are keyed on the lease, for [`dead_letter_in_two`]'s
/// reasons: the columns have to be read while they still exist, and a worker that overran § 4's
/// visibility window must match no row in either half. The rollback is [`sqlite_claim`]'s, for its
/// reason.
fn sqlite_dead_letter_in_two(
    sqlite: &nvs_db::SqliteConn,
    id: i64,
    held_at: i64,
    failed_at: i64,
    errors: &str,
) -> io::Result<()> {
    let split = nvs_stdlib::queue::DEAD_LETTER_SQLITE;
    // The `insert … select` names the two values it adds to the copied row before the two the lease
    // is keyed on, because that is where they stand in the text.
    let mut copying = vec![
        nvs_db::SqliteValue::Int(failed_at),
        nvs_db::SqliteValue::Text(errors.to_owned()),
    ];
    copying.extend(sqlite_lease(id, held_at));
    sqlite.begin_immediate()?;
    let moved = sqlite_apply(sqlite, split.first, copying)
        .and_then(|()| sqlite_apply(sqlite, split.then, sqlite_lease(id, held_at)));
    match moved {
        Ok(()) => {
            sqlite.commit()?;
            Ok(())
        }
        Err(refused) => {
            let _undone = sqlite.roll_back();
            Err(refused)
        }
    }
}

/// An epoch-millisecond instant as the text a placeholder is sent as — a `$n::bigint` on one
/// driver, a `?` bound as text on the other, and the same octets either way.
fn millis(at: i64) -> Vec<u8> {
    at.to_string().into_bytes()
}

/// Parks this task for `length`, reporting how the wait ended.
///
/// With no host on the thread there is nothing to hand the core back to, so the wait is a blocking
/// one — `Core\Time::sleep`'s own reading, and unreachable here, since a worker is only ever a task.
fn pause(length: Duration) -> Woken {
    with_current(|host| host.sleep(length)).unwrap_or_else(|| {
        std::thread::sleep(length);
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
        nvs_db::Driver::SqlServer => open_as!(
            nvs_db::TdsTarget<'_>,
            nvs_db::TdsConn,
            nvs_db::tds::DEFAULT_PORT,
            Wire::SqlServer,
            name,
            block
        ),
        // Its own arm rather than one the macro writes: a SQLite block names a path this process
        // opens itself, so there is no host to resolve, no port to pick and no handshake to bound.
        nvs_db::Driver::Sqlite => sqlite_wire(name, block),
    }
}

/// [`open`]'s SQLite half, which resolves a path where every other arm resolves an address.
///
/// **Not [`open_as`], and what it is not is the whole difference between a file and a socket.**
/// Every arm that macro writes resolves a host, picks a port and connects under
/// [`CONNECT_DEADLINE`]; a SQLite block names a path this process opens, so there is no address to
/// fail to resolve and no handshake to time out. What survives of that shape is the two refusals
/// below, each a `warning:` that returns no worker rather than an `error:` that returns an exit
/// code.
fn sqlite_wire(name: &str, block: &Database) -> Option<Wire> {
    let target = match nvs_db::SqliteTarget::resolve(block) {
        Ok(target) => target,
        Err(refused) => {
            eprintln!(
                "warning: no queue worker started: {}",
                refused.refusal(name)
            );
            return None;
        }
    };
    match nvs_db::sqlite::open(&target) {
        Ok(sqlite) => Some(Wire::Sqlite(sqlite)),
        Err(err) => {
            eprintln!(
                "warning: no queue worker started: `[db.{name}]` at `{}` did not open: {err}",
                target.path.display()
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
/// Every driver `nvs_stdlib::queue::runs` answers for has an arm, so a `Wire` that exists is one
/// § 4's statements can run on, and a driver gaining a send path arrives here as a build failure
/// rather than as a refusal that has quietly stopped being true.
enum Wire {
    /// § 4's and § 6's statements as PostgreSQL's single texts.
    Postgres(nvs_db::PgConn),
    /// The same statements as MySQL's dialect, some of them
    /// [`nvs_stdlib::queue::Split`]s.
    MySql(nvs_db::MySqlConn),
    /// MariaDB, which runs every one of MySQL's texts unchanged over its own framing and its own
    /// authentication roster.
    MariaDb(nvs_db::MariaConn),
    /// § 4's and § 6's statements as T-SQL, whose claim is one statement and whose dead-letter move
    /// is a [`nvs_stdlib::queue::Split`].
    SqlServer(nvs_db::TdsConn),
    /// A file this process opened rather than a socket, running MySQL's own texts — the SQLite
    /// constants beside them in [`nvs_stdlib::queue`] are that dialect and not a copy of it.
    Sqlite(nvs_db::SqliteConn),
}

impl Wire {
    /// Gives the next exchange on this connection [`STATEMENT_DEADLINE`] to finish in.
    ///
    /// The four wire drivers file it on the socket they are waiting on. SQLite is a file this
    /// process opened, so there is no peer to wait for and nothing to bound — what a statement
    /// there can wait on is another connection's write lock, which is `sqlite3_busy_timeout`'s and
    /// `nvs_db::sqlite`'s to answer.
    fn bound_next_exchange(&mut self) {
        let at = Some(Instant::now() + STATEMENT_DEADLINE);
        match self {
            Wire::Postgres(postgres) => postgres.set_deadline(at),
            Wire::MySql(mysql) => mysql.set_deadline(at),
            Wire::MariaDb(maria) => maria.set_deadline(at),
            Wire::SqlServer(tds) => tds.set_deadline(at),
            Wire::Sqlite(_) => {}
        }
    }

    /// This connection borrowed as the dialect its statements are written in.
    fn dialect(&mut self) -> Dialect<'_> {
        match self {
            Wire::Postgres(postgres) => Dialect::Postgres(postgres),
            Wire::MySql(mysql) => Dialect::Framed(Framed::MySql(mysql)),
            Wire::MariaDb(maria) => Dialect::Framed(Framed::MariaDb(maria)),
            Wire::SqlServer(tds) => Dialect::SqlServer(tds),
            Wire::Sqlite(sqlite) => Dialect::Sqlite(sqlite),
        }
    }
}

/// A borrowed [`Wire`], narrowed to the dialects `nvs_stdlib::queue` writes statements in.
///
/// The same arms `Core\Queue`'s own members branch on, and for the same reason: § 2's schema and
/// §§ 4 and 6's statements are written per backend that can run them, so an arm here is a backend
/// with something to send.
///
/// **SQLite is its own arm rather than a third [`Framed`] driver**, and what keeps it out is a type
/// and not a dialect: [`nvs_db::SqliteConn::query`] takes owned values where a wire driver takes
/// already-encoded octets, so there is no borrow the two could share. The *text* is shared —
/// [`nvs_stdlib::queue::DEAD_LETTER_SQLITE`] is MySQL's own pair — which is exactly why the seam
/// falls here.
enum Dialect<'a> {
    /// [`nvs_stdlib::queue::CLAIM_POSTGRES`] and its siblings, each answering in one statement.
    Postgres(&'a mut nvs_db::PgConn),
    /// [`nvs_stdlib::queue::CLAIM_MYSQL`] and its siblings, some of them pairs inside one
    /// transaction — and MariaDB runs every one of them unchanged.
    Framed(Framed<'a>),
    /// [`nvs_stdlib::queue::CLAIM_SQLSERVER`] and its siblings: the claim is one statement as
    /// PostgreSQL's is, and § 6's move is a pair inside one transaction as the framed one is.
    SqlServer(&'a mut nvs_db::TdsConn),
    /// [`nvs_stdlib::queue::CLAIM_SQLITE`] and its siblings, every pair inside the immediate
    /// transaction `rule:concurrency/claiming-is-one-statement` makes the mutual exclusion out of.
    Sqlite(&'a mut nvs_db::SqliteConn),
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use nvs_stdlib::queue::Bell;

    /// The column list a split claim reads its row with.
    ///
    /// Both split dialects name it in the same place — between the `select` that opens the first
    /// statement and the table it reads — so this is one reader rather than one per dialect.
    fn selected(first: &str) -> &str {
        first
            .split_once("select ")
            .expect("a split claim reads the row first")
            .1
            .split_once(" from ")
            .expect("that read names the table it reads")
            .0
    }

    /// The names a `select` or `returning` list answers, in its own order.
    ///
    /// An ` as ` alias reads as the name it binds, because that is what a reader asking by position
    /// gets: `attempts + 1 as attempts` is `attempts` to everything downstream of it.
    fn answered(list: &str) -> Vec<&str> {
        list.split(',')
            .map(|column| {
                let column = column.trim();
                column.rsplit(" as ").next().unwrap_or(column).trim()
            })
            .collect()
    }

    /// The positions above are a claim's column list written down twice, and this is the second
    /// copy asserted against the first.
    ///
    /// Nothing else would notice them disagreeing: every column of the list is text or an integer,
    /// so a job whose `script` was read out of the `args` slot runs a file named by its own
    /// payload — and it type-checks, and the claim still answers as many values. Asked of every
    /// dialect, since [`nvs_stdlib::queue::CLAIM_MYSQL`] and [`nvs_stdlib::queue::CLAIM_SQLITE`]
    /// each carry the same list as a `select` and one of its entries is computed rather than named.
    #[test]
    fn the_worker_reads_args_and_script_at_the_positions_the_claim_statement_returns_them() {
        let postgres = nvs_stdlib::queue::CLAIM_POSTGRES
            .rsplit_once("returning ")
            .expect("the claim answers a `returning` list")
            .1;
        let mysql = selected(nvs_stdlib::queue::CLAIM_MYSQL.first);
        let sqlite = selected(nvs_stdlib::queue::CLAIM_SQLITE.first);
        let read_at = [
            (super::ID, "id"),
            (super::SCRIPT, "script"),
            (super::ARGS, "args"),
            (super::ATTEMPTS, "attempts"),
            (super::MAX_ATTEMPTS, "max_attempts"),
            (super::BACKOFF, "backoff_ms"),
            (super::ERRORS, "errors"),
            (super::GRANTS, "grants"),
            (super::LIMITS, "limits"),
        ];
        for (dialect, list) in [("postgres", postgres), ("mysql", mysql), ("sqlite", sqlite)] {
            let columns = answered(list);
            assert_eq!(
                columns.len(),
                read_at.len(),
                "{dialect}: the claim answers {columns:?}, and this file reads {} positions",
                read_at.len()
            );
            for (at, name) in read_at {
                assert_eq!(
                    columns[at], name,
                    "{dialect}: position {at} answers `{}` where this file reads `{name}`",
                    columns[at]
                );
            }
        }
    }

    /// `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`: the row that exhausts its
    /// attempts reaches the dead-letter table carrying what **every** attempt threw, and not the
    /// last one's failure alone.
    ///
    /// **Asserted over the whole ladder rather than over one call**, because this crate's half of
    /// that promise is the threading and not the appending — [`super::errors_after`]'s own doc owns
    /// the division. A case that called it once would pass equally well against a retry that bound
    /// a fresh array, which is precisely the shape that loses every attempt before the exhausting
    /// one.
    ///
    /// The array is threaded through a fresh [`super::Job`] each round, which is what a claim does
    /// to it in earnest: the value crosses the database between attempts, so each round reads the
    /// column the round before wrote and the last round's answer is what `report` hands to the
    /// dead-letter move.
    #[test]
    fn a_dead_lettered_row_carries_every_attempts_error() {
        let thrown = [
            (1_i64, "RuntimeError", "the endpoint refused"),
            (2, "Error", "it refused again"),
            (3, "LogicError", "and the third time the fault was ours"),
        ];
        let mut carried: Option<String> = None;
        for (attempt, class, message) in thrown {
            let job = super::Job {
                id: 7,
                script: "queue/failing.nvs".to_string(),
                args: None,
                attempts: attempt,
                max_attempts: 3,
                backoff_ms: 100,
                errors: carried,
                grants: None,
                limits: None,
            };
            carried = Some(super::errors_after(
                &job,
                1_700_000_000_000 + attempt,
                &nvs_host::Failure {
                    class: class.to_string(),
                    message: message.to_string(),
                },
            ));
        }
        let written = carried.expect("every attempt failed, so every round wrote the array");
        let parsed: serde_json::Value =
            serde_json::from_str(&written).expect("the column holds a JSON document");
        let entries = parsed
            .as_array()
            .expect("§ 6's `errors` is an array of entries");
        assert_eq!(
            entries.len(),
            thrown.len(),
            "the dead-letter row carries one entry per attempt, and this one carries {written}"
        );
        for (at, (attempt, class, message)) in thrown.into_iter().enumerate() {
            assert_eq!(
                (&entries[at]["class"], &entries[at]["message"]),
                (
                    &serde_json::Value::from(class),
                    &serde_json::Value::from(message)
                ),
                "the entries read in the order the attempts ran"
            );
            assert_eq!(
                entries[at]["at"],
                1_700_000_000_000_i64 + attempt,
                "an entry says when its own attempt started, which is the lease it was keyed on"
            );
        }
    }

    /// A job runs under the snapshot the `[[app]]` blocks matching its own script fold to
    /// (ADR 0271 § 2), out of the publish the worker's holder serves.
    ///
    /// The tree grants `script.spawn` over both scripts. The script whose block narrows
    /// `[app.limits] memory` gets that block's budget, and the resolve of the script whose block
    /// narrows `script.spawn` to the other directory is denied. A job run under the host's
    /// snapshot runs both at the global `512M`, and passes both doors.
    #[test]
    fn a_queued_job_runs_under_its_scripts_app_blocks() {
        let dir = nvs_repo::scratch("worker-job-app-blocks");
        let root = nvs_config::trust::canonical(&dir).expect("the case's directory is there");
        for script in ["jobs/nightly.nvs", "other/nightly.nvs"] {
            let path = root.join(script);
            std::fs::create_dir_all(path.parent().expect("a script in the case's directory"))
                .expect("a scratch directory of this case's own");
            std::fs::write(&path, b"<?php\n").expect("the case writes its own script");
        }
        let toml = root.join("nvs.toml");
        std::fs::write(
            &toml,
            format!(
                "[limits]\nmemory = '512M'\n\n[capabilities]\nscript.spawn = ['{}']\n\n\
                 [[app]]\nroot = 'jobs'\n\n[app.limits]\nmemory = '256M'\n\n\
                 [[app]]\nroot = 'other'\n\n[app.capabilities]\nscript.spawn = ['{}']\n",
                root.display(),
                root.join("jobs").display()
            ),
        )
        .expect("a tree of this case's own");
        let mut sources = nvs_diagnostics::SourceMap::new();
        let (host, blocks, _) = crate::config::boot_set(
            std::slice::from_ref(&toml),
            &mut sources,
            crate::config::Init::Never,
        )
        .expect("the tree this case wrote resolves");
        let current = nvs_config::Current::of(
            nvs_config::Published::new(host, blocks, &[], &crate::config::LocalFiles)
                .expect("the case's blocks fold"),
        );
        let job = |script: &str| super::Job {
            id: 1,
            script: root.join(script).to_string_lossy().into_owned(),
            args: None,
            attempts: 1,
            max_attempts: 1,
            backoff_ms: 100,
            errors: None,
            grants: None,
            limits: None,
        };

        let granted = job("jobs/nightly.nvs");
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        assert!(
            super::configure(&mut ctx, &current, &granted).is_ok(),
            "the job's script folds under the case's blocks"
        );
        // The block's ceiling less the tier-1 handler's reserve, which is bounded by a quarter of
        // the ceiling (`nvs_runtime::Ctx::refresh_limits`).
        let ceiling = 256 * 1024 * 1024;
        let held = ctx.memory_limit();
        assert!(
            held > 0 && held <= ceiling && ceiling - held <= ceiling / 4,
            "a job is charged against its script's `[app.limits] memory = '256M'`, and this one \
             was held to {held} byte(s)"
        );
        assert!(
            !matches!(
                nvs_runtime::script::resolve(&ctx, &granted.script),
                Err(nvs_runtime::script::ResolveError::Denied(_))
            ),
            "the job's script is under the deployment's `script.spawn` grant"
        );

        let narrowed = job("other/nightly.nvs");
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        assert!(
            super::configure(&mut ctx, &current, &narrowed).is_ok(),
            "the job's script folds under the case's blocks"
        );
        assert!(
            matches!(
                nvs_runtime::script::resolve(&ctx, &narrowed.script),
                Err(nvs_runtime::script::ResolveError::Denied(_))
            ),
            "a script whose own block narrows `script.spawn` away from it ran under the host's grant"
        );
    }

    /// `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them`: the row that exhausted its
    /// attempts arrives in the dead-letter table carrying its tag.
    ///
    /// **A move is three column lists that have to agree**, and the two dialects write them in two
    /// shapes, so this asserts the agreement rather than the presence of a word: a `tag` added to
    /// the insert list and not to the `select` beside it shifts every value after it into the wrong
    /// column, and the server takes it — the columns on either side of it are text as well. The
    /// trailing pair is what the move itself adds, and it is named, so a list that grew a fourth
    /// value fails here too.
    #[test]
    fn a_dead_letter_move_carries_the_tag_across_with_the_rest_of_the_row() {
        for (dialect, statement, from) in [
            ("postgres", nvs_stdlib::queue::DEAD_LETTER_POSTGRES, "moved"),
            (
                "mysql",
                nvs_stdlib::queue::DEAD_LETTER_MYSQL.first,
                "nvs_jobs",
            ),
        ] {
            let (columns, rest) = statement
                .split_once("insert into nvs_dead_jobs (")
                .expect("the move writes the dead-letter table")
                .1
                .split_once(") select ")
                .expect("it writes what it just read");
            let columns = answered(columns);
            let carried = answered(
                rest.split_once(&format!(" from {from}"))
                    .expect("the read names where the row is coming from")
                    .0,
            );
            assert_eq!(
                columns.len(),
                carried.len(),
                "{dialect}: {columns:?} is written from {carried:?}"
            );
            assert!(
                columns.contains(&"tag"),
                "{dialect}: the dead-letter row loses its tag, so a purge by tag cannot reach the \
                 work that failed"
            );
            let copied = columns.len() - 2;
            assert_eq!(
                columns[..copied],
                carried[..copied],
                "{dialect}: the two lists name the columns in two orders"
            );
            assert_eq!(
                &columns[copied..],
                &["failed_at", "errors"],
                "{dialect}: the move adds something other than when it failed and what it threw"
            );
            if dialect == "postgres" {
                let removed = answered(
                    statement
                        .split_once("returning ")
                        .expect("PostgreSQL's move reads the row out of its own delete")
                        .1
                        .split_once(") insert into")
                        .expect("the delete's list ends where the insert begins")
                        .0,
                );
                assert_eq!(
                    removed,
                    carried[..copied],
                    "the delete answers a list the insert does not write"
                );
            }
        }
    }

    /// A `[queue] connection` naming a SQLite block starts a worker, and the statements that worker
    /// will send are that dialect's.
    ///
    /// **The seam asserted is [`super::Wire::dialect`] and not the connection.** A block that opened
    /// and was then read as another dialect would send a placeholder spelling this backend refuses,
    /// and it would do so only once a job was actually due — a failure an operator meets in the work
    /// that did not happen rather than at boot. The path is absolute because a `-p nvs-cli` fixture
    /// has no `nvs.toml` for
    /// `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` to resolve a
    /// relative one against.
    #[test]
    fn a_sqlite_block_opens_a_queue_worker() {
        let dir = nvs_repo::scratch("worker-a-sqlite-block-opens-a-queue-worker");
        let block = nvs_config::tree::Database {
            driver: Some("sqlite".to_owned()),
            path: Some(dir.join("queue.db").display().to_string()),
            ..Default::default()
        };
        let mut wire = super::open("jobs", &block).expect("a SQLite block opens a worker");
        assert!(
            matches!(wire.dialect(), super::Dialect::Sqlite(_)),
            "the worker opened the block and reads it as some other dialect's statements"
        );
        // The connection holds the file until it is dropped, and on Windows an open file is one
        // nothing can remove.
        drop(wire);
        drop(dir);
    }

    /// A `[queue] connection` naming a SQL Server block starts a worker, and the statements that
    /// worker will send are T-SQL.
    ///
    /// **[`a_sqlite_block_opens_a_queue_worker`]'s claim over a socket**, and that is the whole of
    /// why this one asks the matrix for an endpoint where its twin makes a temporary file:
    /// [`super::open`] connects, so the only block that opens is one with a server behind it. A
    /// process that finds `NVS_DB_MATRIX_DRIVER` unset asserts nothing — [`nvs_db::matrix`]'s own
    /// rule, and `bun nv db-matrix` is what makes this assertion happen, which is why this
    /// crate is one of that command's suites.
    ///
    /// The seam asserted is [`super::Wire::dialect`] and not the connection, for the twin's reason:
    /// a block that opened and was then read as another dialect would send a placeholder spelling
    /// this backend refuses, and only once a job was actually due.
    #[test]
    fn a_worker_opens_a_sql_server_queue_block() {
        let Some(endpoint) = nvs_db::matrix::endpoint() else {
            return;
        };
        if endpoint.driver != nvs_db::Driver::SqlServer {
            return;
        }
        let nvs_db::matrix::Location::Server(server) = endpoint.location else {
            return;
        };
        let block = nvs_config::tree::Database {
            driver: Some("mssql".to_owned()),
            host: Some(server.host.clone()),
            port: Some(server.port),
            user: Some(server.user.clone()),
            password: Some(server.password.clone()),
            database: Some(server.database.clone()),
            tls_ca_file: Some(server.ca.display().to_string()),
            ..Default::default()
        };
        let mut wire = super::open("jobs", &block).expect("a SQL Server block opens a worker");
        assert!(
            matches!(wire.dialect(), super::Dialect::SqlServer(_)),
            "the worker opened the block and reads it as some other dialect's statements"
        );
    }

    /// A drain begun under a worker is read at the top of its next turn, and so there is no next
    /// turn.
    ///
    /// [`nvs_server::Draining::detached`] and never the process's bit: that one is a handle on a
    /// `OnceLock` nothing puts back, so a case beginning it would decide the answer for every other
    /// case in this binary, whichever order cargo ran them in.
    #[test]
    fn a_worker_handed_a_draining_handle_returns_at_the_top_of_its_next_turn() {
        let drain = nvs_server::Draining::detached();
        let workers = super::Workers::draining(drain.clone());
        let mut turns = 0_u32;
        super::take_turns(&workers, || {
            turns += 1;
            // The shutdown, arriving while this worker is holding the core.
            drain.begin();
            Ok(true)
        })
        .expect("a turn that claims answers with what it claimed");
        assert_eq!(
            turns, 1,
            "the worker took {turns} turns against a drain begun during the first, so what it \
             stops on is not read at the top of a turn"
        );
    }

    /// A drain that begins while a claim is in flight does not cut that turn short.
    ///
    /// `rule:concurrency/one-process-serves-requests-schedules-and-jobs`'s claimed job that runs to
    /// completion: what a drain stops is the *taking* of new work, so the predicate is read at the
    /// top of a turn and nowhere inside one. Asserted as the order of the steps a turn takes,
    /// because a turn that returned between them would leave the row claimed and invisible to every
    /// other worker until its visibility window ran out — which is a job delayed rather than a
    /// crash, and so is not otherwise noticed.
    #[test]
    fn a_worker_with_a_claim_in_flight_finishes_the_job_before_it_returns() {
        let drain = nvs_server::Draining::detached();
        let workers = super::Workers::draining(drain.clone());
        let mut steps = Vec::new();
        super::take_turns(&workers, || {
            steps.push("claimed");
            // With the row claimed and the job not yet written back, which is the moment this case
            // exists for.
            drain.begin();
            steps.push("ran");
            steps.push("reported");
            Ok(true)
        })
        .expect("a turn that claims answers with what it claimed");
        assert_eq!(
            steps,
            ["claimed", "ran", "reported"],
            "a turn holding a claim stopped part-way through it"
        );
    }

    /// `nvs run`'s workers still stop on the script's task and on nothing else.
    ///
    /// Both halves, because neither is the assertion alone: the loop keeps turning while the flag
    /// is unset, so nothing added beside it stops a run early, and it returns at the top of the
    /// turn after the one that set the flag, which is where it returned before.
    #[test]
    fn a_run_stops_its_workers_when_the_scripts_task_exits_exactly_as_before() {
        let workers = super::Workers::new();
        let mut turns = 0_u32;
        super::take_turns(&workers, || {
            turns += 1;
            // The script's own task, on its way out during this worker's third turn.
            if turns == 3 {
                workers.stop();
            }
            Ok(true)
        })
        .expect("a turn that claims answers with what it claimed");
        assert_eq!(
            turns, 3,
            "a run's worker took {turns} turns where the script's exit ends the third"
        );
    }

    /// A turn that failed is handed back, so [`super::claim_until_stopped`] has something to say on
    /// its way out.
    ///
    /// The failure this guards is the one that costs the most to diagnose: a claim naming a column
    /// the table has not got — a deployment whose `nvs queue migrate` has not been run since the
    /// schema grew — ends the worker, and a worker that ended without a word leaves a queue that
    /// never drains and a `stats` counter stuck at zero with nothing anywhere pointing at the
    /// database. Asserted here rather than on the message, because the loop is the half a case can
    /// drive without a server.
    #[test]
    fn a_turn_that_fails_is_handed_back_rather_than_swallowed() {
        let workers = super::Workers::new();
        let mut turns = 0_u32;
        let stopped = super::take_turns(&workers, || {
            turns += 1;
            Err(std::io::Error::other(
                "column \"errors\" does not exist at character 8",
            ))
        });
        let refused = stopped.expect_err("a worker stops on a failed turn and says what failed");
        assert_eq!(
            (turns, refused.to_string().contains("\"errors\"")),
            (1, true),
            "the worker ended on the first failing turn, and the refusal it ended on is the one the \
             server sent"
        );
    }

    /// One [`super::start`] over one [`super::Workers`], and the binaries differ in which end of it
    /// says stop.
    ///
    /// The shared half is what the signature carries — `start` takes a `&Workers` and neither
    /// binary has a second entry point to it. The differing half is this: handed the same two
    /// events, a run's switch is deaf to the drain it holds no handle on, and a served instance's
    /// stops on that drain without any script having exited.
    #[test]
    fn the_two_binaries_share_start_and_differ_in_one_predicate() {
        let drain = nvs_server::Draining::detached();
        let run = super::Workers::new();
        let served = super::Workers::draining(drain.clone());
        assert!(
            !run.stopping() && !served.stopping(),
            "a switch stopped its workers before anything asked it to"
        );
        drain.begin();
        assert!(
            !run.stopping(),
            "a run's workers stopped on a drain that is not theirs to read"
        );
        assert!(
            served.stopping(),
            "a served instance's workers went on claiming after the drain began"
        );
        run.stop();
        assert!(
            run.stopping(),
            "a run's workers went on claiming after the script's task exited"
        );
    }

    /// The root a worker's task takes is the one whose fault retires the worker, and not the one
    /// the `[[schedule]]` ticker beside it holds.
    ///
    /// Asserted through the consequence rather than by reading [`super::ROOT`] back, because the
    /// two roots differ in exactly one answer and that answer is
    /// `rule:http-server/containment-does-not-end-at-the-helper`'s whole reason for the choice: a
    /// fault under `TaskRoot::Request` is charged to the request it happened in, and a worker has
    /// no request beneath it for that to mean anything. Both are spawned here because the pair is
    /// the assertion — one root answering `true` says nothing about a copy of the ticker's line.
    #[test]
    fn an_armed_worker_holds_task_root_worker_and_not_the_tickers_request() {
        let mut sched = nvs_host::Scheduler::new();
        sched.spawn(nvs_runtime::Ctx::stdout(), super::ROOT, |_| {
            panic!("a worker's own fault, which no request is under");
        });
        sched.spawn(
            nvs_runtime::Ctx::stdout(),
            nvs_runtime::TaskRoot::Request,
            |_| panic!("a fire's fault, which belongs to the run it happened in"),
        );
        sched.run();
        // The run queue is FIFO, so these come back in the order they were spawned.
        let retired: Vec<bool> = sched
            .take_finished()
            .into_iter()
            .map(|done| {
                done.outcome
                    .expect_err("both tasks panicked")
                    .retires_worker()
            })
            .collect();
        assert_eq!(
            retired,
            [true, false],
            "a queue worker's fault is not retiring the worker, so it is being charged to a \
             request that does not exist"
        );
    }

    /// The queue the two end-to-end cases below push onto and claim from.
    ///
    /// It is also the `[db.<name>]` key their [`super::QueueBounds`] names, and the two are
    /// unrelated: a queue's name is a column, a connection's is a table of blocks. One word for
    /// both is what a `nvs.toml` that wrote neither would end up with, so nothing here depends on
    /// them differing.
    const QUEUE: &str = "jobs";

    /// `Core\Queue\State::Pending`'s ordinal — the state a row is in until a worker claims it.
    const PENDING: i64 = 0;

    /// `Core\Queue\State::Claimed`'s ordinal, which a row holds while the attempt is in flight.
    const CLAIMED: i64 = 1;

    /// `Core\Queue\State::Succeeded`'s ordinal, as [`nvs_stdlib::queue::SUCCEEDED_SQLITE`] writes
    /// it.
    const SUCCEEDED: i64 = 2;

    /// How long a case watches for a job to reach a state before it shuts the run down anyway.
    ///
    /// Generous, because it is not a bound anything asserts: what it exists for is a worker that
    /// never claims, which would otherwise leave a case waiting on a state no turn is going to
    /// write. Reaching it makes the assertion below fail with the state actually on the row rather
    /// than hanging the suite.
    const WATCHING_FOR: Duration = Duration::from_secs(30);

    /// A `[db.<name>]` block naming a SQLite file this case owns.
    ///
    /// A file rather than [`nvs_stdlib::queue`]'s own `mode=memory&cache=shared` fixture URI: the
    /// worker under test opens its *own* connection out of this block, and a case has to be able
    /// to say which database that is by handing it the same block a `nvs.toml` would. Absolute for
    /// the reason [`a_sqlite_block_opens_a_queue_worker`] is —
    /// `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` has no file to
    /// resolve against here. The file sits in a scratch directory of its own, because two runs of
    /// this test binary can overlap, and two runs sharing one file delete and claim each other's
    /// jobs. The case drops the guard once every connection to the file is closed.
    fn a_queue_file(case: &str) -> (nvs_config::tree::Database, nvs_repo::Scratch) {
        let dir = nvs_repo::scratch(&format!("worker-{case}"));
        let block = nvs_config::tree::Database {
            driver: Some("sqlite".to_owned()),
            path: Some(dir.join("queue.db").display().to_string()),
            ..Default::default()
        };
        (block, dir)
    }

    /// A second connection to `block`'s database, which is what a case seeds and reads through.
    fn opened(block: &nvs_config::tree::Database) -> nvs_db::SqliteConn {
        let target = nvs_db::SqliteTarget::resolve(block).expect("the block resolves to a path");
        nvs_db::sqlite::open(&target).expect("the queue's file opens")
    }

    /// [`opened`], with `rule:core-classes/queue-storage-is-a-table`'s two tables built on it.
    ///
    /// The DDL is [`nvs_stdlib::queue::migration`]'s and never a copy, exactly as
    /// `crates/nvs-stdlib/tests/queue_sqlite.rs`'s fixture takes it: what these cases run a real
    /// worker against is the schema `nvs queue migrate` applies, so a statement naming a column
    /// that schema does not have fails here rather than in front of an operator.
    fn converged(block: &nvs_config::tree::Database) -> nvs_db::SqliteConn {
        let conn = opened(block);
        for step in nvs_stdlib::queue::migration(nvs_db::Driver::Sqlite) {
            ran(&conn, &step.sql, Vec::new());
        }
        conn
    }

    /// One statement's rows, materialized, with the connection handed back at rest.
    ///
    /// A `SqliteRows` borrows the connection until it is dropped and leaves it unable to start a
    /// second statement, and every helper here runs another one after it.
    fn ran(
        conn: &nvs_db::SqliteConn,
        sql: &str,
        params: Vec<nvs_db::SqliteValue>,
    ) -> Vec<Vec<nvs_db::SqliteValue>> {
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
    fn int(cell: &nvs_db::SqliteValue) -> i64 {
        match cell {
            nvs_db::SqliteValue::Int(read) => *read,
            other => panic!("this column is an integer and answered {other:?}"),
        }
    }

    /// One pending job on [`QUEUE`], due now, naming `script` and carrying no argument.
    ///
    /// The insert is this module's own rather than [`nvs_stdlib::queue::INSERT_SQLITE`]'s, for the
    /// reason the SQLite suite's fixture gives for its copy: it is the enqueue and not the thing
    /// under test, and what these cases asserted would otherwise be two statements agreeing rather
    /// than a job running. Due *now* and not at a fixture instant, because the worker under test
    /// reads the real clock in [`super::turn`] and a row due later is one it correctly declines.
    fn pushed(conn: &nvs_db::SqliteConn, script: &str) -> i64 {
        let now = nvs_stdlib::queue::now_millis();
        ran(
            conn,
            "insert into nvs_jobs \
             (queue, script, args, state, attempts, max_attempts, backoff_ms, run_at, claimed_at, \
             dedupe_key, dedupe_pending, created_at, tag) \
             values (?, ?, null, 0, 0, 3, 250, ?, null, null, null, ?, null)",
            vec![
                nvs_db::SqliteValue::Text(String::from(QUEUE)),
                nvs_db::SqliteValue::Text(String::from(script)),
                nvs_db::SqliteValue::Int(now),
                nvs_db::SqliteValue::Int(now),
            ],
        );
        let read = ran(
            conn,
            "select id from nvs_jobs order by id desc limit 1",
            Vec::new(),
        );
        int(&read.first().expect("the enqueue landed a row")[0])
    }

    /// The `state` column of one job, or `None` once no row in `nvs_jobs` carries that id.
    ///
    /// `None` is the dead-letter outcome read from this side — § 6 moves the row into the other
    /// table — so a case waiting for a job to finish treats it as finished and fails on the state
    /// rather than waiting out [`WATCHING_FOR`].
    fn state(conn: &nvs_db::SqliteConn, id: i64) -> Option<i64> {
        let read = ran(
            conn,
            "select state from nvs_jobs where id = ?",
            vec![nvs_db::SqliteValue::Int(id)],
        );
        read.first().map(|row| int(&row[0]))
    }

    /// `[queue]` as an instance running `workers` of them against [`QUEUE`]'s connection.
    ///
    /// The bounds a served run resolved, rather than a default set: `visibility` has to outlast the
    /// whole case, since a window that expired mid-attempt would let the second worker take a job
    /// the first is still running and turn a case about one attempt into one about two.
    fn bounds(workers: u32) -> super::QueueBounds {
        super::QueueBounds {
            connection: String::from(QUEUE),
            workers,
            max_attempts: 3,
            visibility: Duration::from_secs(5 * 60),
        }
    }

    /// A script under the repository root, which `cargo test` does not run in. Its whole
    /// directory is recorded as read, as `crate::script`'s own test module records its fixtures.
    fn from_root(relative: &str) -> String {
        let (dir, file) = relative
            .rsplit_once('/')
            .expect("a script path names its directory");
        nvs_repo::path(dir)
            .join(file)
            .to_string_lossy()
            .into_owned()
    }

    /// `nvs serve`'s own loop over [`nvs_host::run_until_idle`], which is what a worker needs to be
    /// driven at all.
    ///
    /// A worker between turns is a parked task, and that call returns with `parked` non-zero the
    /// moment a poll wakes nothing — so a case that called it once would return with its workers
    /// still napping and assert against a queue nothing had claimed from. `serve.rs`'s own loop
    /// reads the same report the same way, and the end this shares with it is `parked == 0`: every
    /// worker has returned.
    fn served(sched: &mut nvs_host::Scheduler) {
        let installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("a reactor starts"));
        loop {
            match nvs_host::run_until_idle(sched) {
                Ok(report) if report.parked > 0 => {}
                Ok(_) => break,
                Err(error) => panic!("the scheduler stopped: {error}"),
            }
        }
        drop(installed);
    }

    /// The task that shuts the run down, spawned beside the workers: it waits for `id` to leave the
    /// states a worker holds it in, then begins the drain.
    ///
    /// **This is the operator's `SIGTERM` and it is also this case's clock.** A served instance has
    /// no script whose exit ends it, so without something beginning the drain these cases would run
    /// until the process was killed; and because the wait is bounded by [`WATCHING_FOR`], a worker
    /// that never claims ends the run too, leaving the assertion to fail on the state it can then
    /// read. `until` is the last state the case is willing to wait through, so a case wanting the
    /// drain to arrive *mid-attempt* passes [`PENDING`] alone.
    fn draining_once_the_job_leaves(
        sched: &mut nvs_host::Scheduler,
        block: &nvs_config::tree::Database,
        id: i64,
        until: &'static [i64],
        drain: nvs_server::Draining,
        began: &Rc<Cell<Option<Instant>>>,
    ) {
        let watching = opened(block);
        let began = Rc::clone(began);
        sched.spawn(
            nvs_runtime::Ctx::stdout(),
            nvs_runtime::TaskRoot::Request,
            move |_| {
                let deadline = Instant::now() + WATCHING_FOR;
                while state(&watching, id).is_some_and(|read| until.contains(&read))
                    && Instant::now() < deadline
                {
                    // A park and not a spin, so the core runs the workers between two reads.
                    super::pause(LOOKING_EVERY);
                }
                began.set(Some(Instant::now()));
                drain.begin();
            },
        );
    }

    /// **The case the whole goal is for**: one job enqueued against a SQLite `[queue]`, an instance
    /// serving it with `workers = 1`, and the row read back in `succeeded`.
    ///
    /// `rule:concurrency/one-process-serves-requests-schedules-and-jobs` end to end on the one
    /// backend that needs no server: [`super::start`] arms the worker the way `nvs serve` arms it,
    /// and everything between the enqueue and the state is the production path — the roster
    /// statement, `rule:concurrency/claiming-is-one-statement`'s claim, the isolate
    /// `rule:concurrency/a-job-runs-as-a-root-isolate` calls for, and § 6's write-back.
    ///
    /// **Asserted on the state and not on the worker having polled**, because every cheaper reading
    /// of this passes while the queue does nothing an operator wanted: a worker that claimed and
    /// then failed to run the job leaves the same trace as one that ran it, until the row says
    /// `succeeded`. The script is `examples/isolate/hello.nvs` — a program that returns rather than
    /// one that throws — so the outcome under test is the whole path's and not a failure's.
    #[test]
    fn a_job_pushed_to_a_server_with_one_worker_reaches_succeeded() {
        let (block, dir) = a_queue_file("a-job-reaches-succeeded");
        let seeded = converged(&block);
        let id = pushed(&seeded, &from_root("examples/isolate/hello.nvs"));

        let drain = nvs_server::Draining::detached();
        let workers = super::Workers::draining(drain.clone());
        let mut sched = nvs_host::Scheduler::new();
        // The grant is the *snapshot*'s and not a context's, because a worker builds its own
        // context out of the run's configuration — the module doc's *Why the grants are the run's
        // own* section — and `script.spawn` is denied by default at the resolve door.
        let current = Arc::new(nvs_config::Current::new(Arc::new(
            crate::script::granting_snapshot(),
        )));
        super::start(&mut sched, &workers, &bounds(1), &block, &current);
        let began = Rc::new(Cell::new(None));
        draining_once_the_job_leaves(&mut sched, &block, id, &[PENDING, CLAIMED], drain, &began);

        // The compiler is installed for the length of the run, exactly as both binaries install it:
        // a worker reaches it through `nvs_runtime::script::resolve`, and without one every job is
        // refused with no resolver rather than run.
        let compiler = crate::script::Compiler::default();
        nvs_runtime::script::scoped(&compiler, || served(&mut sched));

        assert_eq!(
            state(&seeded, id),
            Some(SUCCEEDED),
            "the job an instance with one worker was left alone with is not `succeeded`, so the \
             queue this process serves is not draining"
        );
        drop(seeded);
        drop(dir);
    }

    /// A served instance holding workers ends within a bound once the drain begins, and the bound
    /// is enforced from outside the run.
    ///
    /// **The half that cannot be asserted from inside.** A worker deaf to the drain is a task
    /// always parked, so `nvs_host::run_until_idle` never returns and a case driving it in this
    /// thread would hang the suite rather than fail — which is the failure this whole ordering
    /// exists to turn into a red test
    /// (`rule:concurrency/one-process-serves-requests-schedules-and-jobs`'s "a server nothing but
    /// killing the process could stop"). So the run gets a thread of its own and this one waits on
    /// a channel: a run that never ends arrives as a timeout naming it.
    ///
    /// **Two workers and a job in flight**, because neither is the assertion alone. The job is what
    /// proves the workers were live — a run whose connection never opened ends promptly for a
    /// reason that has nothing to do with the drain — and the second worker is what makes the end
    /// *every* worker's rather than one's. The drain arrives while the claim is in flight, so the
    /// tail measured is § 6's write-back finishing plus the drain waking the other worker out of
    /// its idle wait, which is what `[queue] workers`'s shutdown actually costs a deployment.
    #[test]
    fn a_served_process_with_workers_exits_within_its_deadline_once_the_drain_begins() {
        // Long enough that a slow machine never reaches it, and finite so a worker ignoring the
        // drain is reported instead of waited on.
        const ARRIVES_WITHIN: Duration = Duration::from_secs(60);
        // What the shutdown is allowed to cost once the drain has begun: one claim written back,
        // with room for a loaded machine. A worker waiting out its visibility window or its
        // connect deadline instead lands well outside it.
        const TAIL: Duration = Duration::from_secs(2);

        let (reached, arrived) = std::sync::mpsc::channel();
        // Every value below is built inside this thread: a scheduler, a reactor and a connection
        // are one thread's, and what crosses back is the one duration this case is about.
        std::thread::spawn(move || {
            let (block, dir) = a_queue_file("workers-exit-on-the-drain");
            let seeded = converged(&block);
            let id = pushed(&seeded, &from_root("examples/isolate/hello.nvs"));

            let drain = nvs_server::Draining::detached();
            let workers = super::Workers::draining(drain.clone());
            let mut sched = nvs_host::Scheduler::new();
            let current = Arc::new(nvs_config::Current::new(Arc::new(
                crate::script::granting_snapshot(),
            )));
            super::start(&mut sched, &workers, &bounds(2), &block, &current);
            let began = Rc::new(Cell::new(None));
            draining_once_the_job_leaves(&mut sched, &block, id, &[PENDING], drain, &began);

            let compiler = crate::script::Compiler::default();
            nvs_runtime::script::scoped(&compiler, || served(&mut sched));

            let tail = began
                .get()
                .expect("the drain began before the run ended")
                .elapsed();
            let landed = state(&seeded, id);
            drop(seeded);
            drop(dir);
            let _ = reached.send((tail, landed));
        });

        let (tail, landed) = arrived.recv_timeout(ARRIVES_WITHIN).expect(
            "a served instance holding queue workers did not end within its deadline after the \
             drain began, so a worker is not reading it and nothing but killing the process would \
             stop this server",
        );
        assert!(
            landed != Some(PENDING),
            "no worker claimed the job, so this run ended for a reason that is not the drain"
        );
        assert!(
            tail < TAIL,
            "the instance took {tail:?} to end after the drain began, where what it owes is one \
             write-back"
        );
    }

    /// How often a case's own task looks at the state it is waiting on.
    const LOOKING_EVERY: Duration = Duration::from_millis(2);

    /// An idle wait no case waits out. A worker holding it asks its queue again only when something
    /// wakes it, so what a case then reads is what the wake did and never what the clock did.
    const NEVER: Duration = Duration::from_secs(60 * 60);

    /// What the worker under one of the cases below has asked of its queue.
    #[derive(Default)]
    struct Asked {
        /// Turns begun. Each one is a roster statement, and a claim per queue the roster named.
        turns: Cell<u32>,
        /// Turns that found nothing, after each of which the worker waits for work.
        empty: Cell<u32>,
        /// Which turn first claimed a job, counted from 1.
        claimed_on: Cell<Option<u32>>,
    }

    /// A switch over a bell of its own, and that bell.
    ///
    /// The process's bell is rung by every switch in this binary that stops, so a case counting
    /// what one worker asked takes a bell nothing else holds.
    fn switch(draining: Option<nvs_server::Draining>, idle: Duration) -> (super::Workers, Bell) {
        let bell = Bell::detached();
        let workers = super::Workers {
            stop: Rc::new(Cell::new(false)),
            draining,
            bell: bell.clone(),
            idle,
        };
        (workers, bell)
    }

    /// The snapshot a worker's jobs run under in these cases, which grants `script.spawn`.
    fn granting() -> Arc<nvs_config::Current> {
        Arc::new(nvs_config::Current::new(Arc::new(
            crate::script::granting_snapshot(),
        )))
    }

    /// One worker over `block` on `sched`, with every turn it takes counted in `asked`.
    ///
    /// [`super::claim_until_stopped`]'s own body with the count taken around [`super::turn`], so
    /// what is counted is the turn both binaries run.
    fn counted_worker(
        sched: &mut nvs_host::Scheduler,
        workers: &super::Workers,
        block: &nvs_config::tree::Database,
        asked: &Rc<Asked>,
    ) {
        let workers = workers.clone();
        let block = block.clone();
        let asked = Rc::clone(asked);
        let current = granting();
        let mut ctx = nvs_runtime::Ctx::stdout();
        ctx.set_config(current.load());
        sched.spawn(ctx, super::ROOT, move |ctx| {
            let mut conn = super::open(QUEUE, &block).expect("the queue's file opens for a worker");
            super::take_turns(&workers, || {
                let turn = asked.turns.get() + 1;
                asked.turns.set(turn);
                let claimed = super::turn(ctx, &mut conn, &current, bounds(1).visibility)?;
                if !claimed {
                    asked.empty.set(asked.empty.get() + 1);
                } else if asked.claimed_on.get().is_none() {
                    asked.claimed_on.set(Some(turn));
                }
                Ok(claimed)
            })
            .expect("every statement a turn sends is answered");
        });
    }

    /// Parks the calling task until `done` answers `true`, or [`WATCHING_FOR`] has passed.
    fn until(mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + WATCHING_FOR;
        while !done() && Instant::now() < deadline {
            super::pause(LOOKING_EVERY);
        }
    }

    /// Runs `case` on a thread of its own and answers with what it returned.
    ///
    /// A worker nothing wakes is a run that never ends, so the run cannot be driven on the thread
    /// that asserts: the wait for its answer is what is bounded here, and a run past the bound
    /// fails naming the wake that never came.
    fn run_to_its_end<T: Send + 'static>(case: impl FnOnce() -> T + Send + 'static) -> T {
        let (reached, arrived) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = reached.send(case());
        });
        arrived.recv_timeout(Duration::from_secs(60)).expect(
            "a run holding an idle queue worker did not end, so nothing woke that worker out of \
             its idle wait",
        )
    }

    /// An idle worker asks its queue once when it starts and then once per [`super::IDLE_WAIT`],
    /// which is the whole of what a `[queue]` with nothing in it costs.
    ///
    /// **The bound is counted from the time the run really took**, so a slow machine moves the
    /// bound with it: a second turn needs one whole idle wait to have passed since the first
    /// found nothing, whatever else the machine was doing.
    #[test]
    fn an_idle_worker_asks_once_and_then_once_per_idle_wait() {
        // Many times the wait a worker that polled would take between two asks, and a fraction of
        // the one it takes.
        const WINDOW: Duration = Duration::from_millis(300);

        let (turns, watched) = run_to_its_end(|| {
            let (block, dir) = a_queue_file("an-idle-worker-asks-once");
            let seeded = converged(&block);
            let (workers, _bell) = switch(None, super::IDLE_WAIT);
            let asked = Rc::new(Asked::default());
            let mut sched = nvs_host::Scheduler::new();
            let started = Instant::now();
            counted_worker(&mut sched, &workers, &block, &asked);
            let watched = Rc::new(Cell::new(Duration::ZERO));
            {
                let (asked, watched) = (Rc::clone(&asked), Rc::clone(&watched));
                sched.spawn(
                    nvs_runtime::Ctx::stdout(),
                    nvs_runtime::TaskRoot::Request,
                    move |_| {
                        until(|| asked.empty.get() > 0);
                        super::pause(WINDOW);
                        watched.set(started.elapsed());
                        workers.stop();
                    },
                );
            }
            served(&mut sched);
            drop(seeded);
            drop(dir);
            (asked.turns.get(), watched.get())
        });

        assert_eq!(
            super::IDLE_WAIT,
            Duration::from_secs(1),
            "`rule:concurrency/a-push-wakes-an-idle-worker` gives the idle wait as one second"
        );
        let waits = watched.as_millis() / super::IDLE_WAIT.as_millis();
        let allowed = 1 + u32::try_from(waits).unwrap_or(u32::MAX);
        assert!(
            (1..=allowed).contains(&turns),
            "an idle worker asked its queue {turns} times in {watched:?}, where one ask at the \
             start and one per idle wait is at most {allowed}"
        );
    }

    /// The turns one idle worker took in a run that was stopped while the worker waited for work,
    /// under an idle wait no case could wait out.
    ///
    /// `served_instance` picks which end says stop: the drain `nvs serve` begins, or the switch
    /// `nvs run`'s script throws on its way out.
    fn turns_when_stopped_mid_wait(served_instance: bool) -> u32 {
        run_to_its_end(move || {
            let (block, dir) = a_queue_file(if served_instance {
                "a-drain-wakes-an-idle-worker"
            } else {
                "a-stop-wakes-an-idle-worker"
            });
            let seeded = converged(&block);
            let drain = served_instance.then(nvs_server::Draining::detached);
            let (workers, _bell) = switch(drain.clone(), NEVER);
            let asked = Rc::new(Asked::default());
            let mut sched = nvs_host::Scheduler::new();
            counted_worker(&mut sched, &workers, &block, &asked);
            {
                let asked = Rc::clone(&asked);
                sched.spawn(
                    nvs_runtime::Ctx::stdout(),
                    nvs_runtime::TaskRoot::Request,
                    move |_| {
                        until(|| asked.empty.get() > 0);
                        match drain {
                            Some(drain) => drain.begin(),
                            None => workers.stop(),
                        }
                    },
                );
            }
            served(&mut sched);
            drop(seeded);
            drop(dir);
            asked.turns.get()
        })
    }

    /// A script's exit does not wait out an idle worker's wait: the switch it throws wakes the
    /// worker, which reads the flag and returns without asking its queue again.
    #[test]
    fn a_runs_exit_wakes_an_idle_worker_and_it_returns_without_asking_again() {
        assert_eq!(
            turns_when_stopped_mid_wait(false),
            1,
            "a worker stopped while it waited for work asked its queue again before it returned"
        );
    }

    /// The same for `nvs serve`: the drain wakes a worker out of its idle wait, and the worker
    /// returns without asking its queue again.
    #[test]
    fn a_drain_wakes_an_idle_worker_and_it_returns_without_asking_again() {
        assert_eq!(
            turns_when_stopped_mid_wait(true),
            1,
            "a worker drained while it waited for work asked its queue again before it returned"
        );
    }

    /// One job enqueued while a worker waits for work, and what became of it.
    ///
    /// The job is written through a second connection once the worker's first turn has found
    /// nothing. `ring` says whether the bell is then rung, which is what `Core\Queue::push` does
    /// in this process and what a push from another process cannot do. The run is drained once the
    /// job has left the states a worker holds it in. The answer is the job's state and the turn
    /// that claimed it.
    fn enqueued_mid_wait(
        case: &'static str,
        idle: Duration,
        ring: bool,
    ) -> (Option<i64>, Option<u32>) {
        run_to_its_end(move || {
            let (block, dir) = a_queue_file(case);
            let seeded = converged(&block);
            let drain = nvs_server::Draining::detached();
            let (workers, bell) = switch(Some(drain.clone()), idle);
            let asked = Rc::new(Asked::default());
            let mut sched = nvs_host::Scheduler::new();
            counted_worker(&mut sched, &workers, &block, &asked);
            let job = Rc::new(Cell::new(None));
            {
                let (asked, job) = (Rc::clone(&asked), Rc::clone(&job));
                let pushing = opened(&block);
                sched.spawn(
                    nvs_runtime::Ctx::stdout(),
                    nvs_runtime::TaskRoot::Request,
                    move |_| {
                        until(|| asked.empty.get() > 0);
                        let id = pushed(&pushing, &from_root("examples/isolate/hello.nvs"));
                        job.set(Some(id));
                        if ring {
                            bell.ring();
                        }
                        until(|| !matches!(state(&pushing, id), Some(PENDING | CLAIMED)));
                        drain.begin();
                    },
                );
            }
            let compiler = crate::script::Compiler::default();
            nvs_runtime::script::scoped(&compiler, || served(&mut sched));
            let landed = job.get().and_then(|id| state(&seeded, id));
            drop(seeded);
            drop(dir);
            (landed, asked.claimed_on.get())
        })
    }

    /// A ring ends an idle worker's wait, and the job it announced runs.
    ///
    /// The worker's idle wait is one no case waits out, so a job that reached `succeeded` was
    /// claimed because of the ring and for no other reason. This is the worker's half of
    /// `rule:concurrency/a-push-wakes-an-idle-worker`; `Core\Queue::push` ringing after the commit
    /// is asserted beside `Core\Db`'s transaction in `nvs-stdlib`.
    #[test]
    fn a_ring_ends_an_idle_workers_wait_and_the_job_it_announced_runs() {
        let (landed, claimed_on) = enqueued_mid_wait("a-ring-wakes-an-idle-worker", NEVER, true);
        assert_eq!(
            (landed, claimed_on),
            (Some(SUCCEEDED), Some(2)),
            "a job enqueued and announced while a worker waited for work was not claimed by that \
             worker's next turn"
        );
    }

    /// A job nothing rang for is claimed when the idle wait ends, which is what a push from
    /// another process is to this one.
    ///
    /// The worker waits [`super::IDLE_WAIT`] here, as it does in both binaries, and nothing rings.
    /// The turn after the wait claims the job. It is the third turn and not the second only when
    /// the enqueue itself took longer than one idle wait to land.
    #[test]
    fn a_job_nothing_announced_is_claimed_when_the_idle_wait_ends() {
        let (landed, claimed_on) =
            enqueued_mid_wait("an-unannounced-job-is-claimed", super::IDLE_WAIT, false);
        assert_eq!(
            landed,
            Some(SUCCEEDED),
            "a job enqueued without a ring was never claimed, so a push from another process \
             would wait for a wake that process cannot send"
        );
        assert!(
            claimed_on.is_some_and(|turn| turn <= 3),
            "a job enqueued without a ring was claimed on turn {claimed_on:?}, where the turn \
             after one idle wait is the one that finds it"
        );
    }
}
