//! Worker cores: where a child placed `on: "worker"` is started, so that it
//! runs on a core other than the one that spawned it.
//!
//! `rule:concurrency/on-worker-runs-the-child-on-another-core` is this module's
//! specification and its one home: *started* is the whole of the placement,
//! because a task never migrates
//! (`rule:concurrency/a-wake-never-moves-a-task`), and what crosses in both
//! directions is a copy. ADR 0184 §§ 2–5 is the reasoning behind each half of
//! the mechanism below.
//!
//! # The blocking pool, turned around
//!
//! [`crate::blocking::run`] takes work that cannot be waited on *off* a core and
//! brings its answer back. A placement is the same handoff pointing the other
//! way, and it is deliberately the same shape: on the parent's core, take a
//! [`RemoteWake`] for the running task, put the work and a slot for its answer
//! into another core's [`Inbox`], and ring that core's bell. On the far core,
//! run the work **as a task** — so it may park, spawn children and reach a
//! reactor of its own, which is the whole difference from a pool thread — put
//! the outcome in the slot, and drop the handle, which ends the parent's park.
//! Back on the parent's core, take the answer out of the slot, or park again:
//! the slot is the record and the wake only ends the wait.
//!
//! # Posting and collecting are two calls
//!
//! [`post`] does the first half and answers a [`Posted`]: the child is on its
//! way to a core, and nothing here has parked. [`Posted::collect`] does the
//! second. The split is [`nvs_runtime::host::Host::start_isolate`]'s
//! **eagerness**, and it is what makes `spawn` and `await` two constructs rather
//! than one blocking call wearing two names — a parent that posts three children
//! and then collects three has three of them running at once, on as many cores
//! as [`WorkerCores::pick`] had to give it. [`place`] is the two halves back to
//! back, for a caller that wants the answer and nothing in between.
//!
//! **[`destination`] splits the first half again**, for a caller that has to
//! spend something to build the work. Securing a core is one question and
//! queueing the work is another, and a spawn has to ask them in that order
//! because encoding its argument consumes it — [`Destination`]'s own doc is that
//! reason's home.
//!
//! **Off a core, [`post`] hands the function back** rather than answering a
//! placement nothing would ever collect: there is no core to give up and no task
//! to park, so [`place`] calls the function where it stands. That is the answer
//! [`crate::blocking::run`] and [`crate::timer::park_until`] both give for the
//! same state.
//!
//! # The inbox is a task, not a hook inside the reactor
//!
//! A core's inbox is drained by one long-lived task on that core — its
//! *receptionist* — woken through the reactor's own cross-thread queue rather
//! than by a second mechanism beside it. Two things decide this. A [`Reactor`]
//! is `!Send`, knows only [`TaskId`]s, and has no [`Ctx`] to start anything
//! with, so draining from inside one would mean handing it a task API it
//! deliberately does not have. And a receptionist that is parked is a parked
//! task, so [`crate::reactor::run_until_idle`] is the core's whole body,
//! unchanged: the loop blocks in the poll exactly as it does for a core serving
//! sockets, and a core with nothing placed on it is a core asleep in `poll`.
//!
//! The **bell** is what makes that repeatable. A [`RemoteWake`] is one-shot, so
//! the receptionist arms one in [`Inbox::bell`] before each drain and whichever
//! core writes next takes it and fires it. Arming happens *before* the drain, so
//! a placement written between a drain and the park it precedes finds a bell to
//! ring; a placement that finds none was written before a drain that has not
//! happened yet, and that drain sees it. A ring for a receptionist that is
//! already running is the ordinary spurious wake every park site here tolerates.
//!
//! # A core that is already running can offer itself
//!
//! Everything above describes a core this module started. A core that already
//! has a thread, a scheduler and a reactor — a serving core — needs none of
//! that built for it a second time, and ADR 0184 § 5 is why it may not have it:
//! those threads are pinned, and a second thread per core for placement work
//! would oversubscribe exactly the cores the server's own latency depends on.
//! So [`register_this_core`] puts an inbox in the set and spawns the identical
//! receptionist on the caller's scheduler, and [`WorkerCores`] holds a member
//! with no thread of its own to join. A set that can reach one never starts one,
//! and the core asking is never the core answered — a child is started on a core
//! *other than* its parent's, and a registered core is in the set it is asking.
//!
//! # A placed child is still its parent's child
//!
//! ADR 0184 § 4: it dies with its parent, and the parent's call does not return
//! while it is still running. Cancellation travels the way the start did —
//! [`Placed::cancelled`] is set on the parent's core, the bell is rung, and the
//! receptionist's next sweep cancels the child on its own core. The
//! acknowledgement comes back through the same slot and the same handle, because
//! [`Answering`] answers from its `Drop`: a child torn down mid-park has never
//! written an answer, and what its parent reads is [`Answer::Stopped`].
//!
//! **A parent being force-unwound is the one case that may not wait**, which is
//! the carve-out [`nvs_runtime::host::Running::abandon`] already states: a stack
//! the scheduler is unwinding may not park. [`Awaited`]'s `Drop` therefore
//! flags the cancellation and rings, and does not wait for the acknowledgement.
//! A parent standing on Novis frames is not unwindable, is resumed with
//! `Resume::Cancelled` instead, and does wait — which is the path a `spawn
//! script` cancellation actually takes.
//!
//! # What it spends
//!
//! `rule:programs/memory-priority` asks for this out loud. At most one scheduler
//! thread per core, started the first time work is placed and never afterwards
//! reclaimed: O(cores), never O(requests served), and a process that places
//! nothing has none — the same shape, and the same reason, as a worker with no
//! blocking work having no pool threads. A **registered** core costs no thread
//! at all, only an inbox and one long-lived task on a core that was running
//! anyway, so a deployment that places nothing pays nothing for having offered.
//! Per placement in flight, charged to the
//! tree that asked for it: one boxed job, one answer slot, one [`RemoteWake`],
//! and on the far core one task and its pooled stack ([`crate::stack`]). The
//! argument and the answer are copied at every node in both directions, which is
//! the cost the placement buys and the reason `on: "here"` exists.
//!
//! # Known gaps
//!
//! `nvs serve` does not call [`register_this_core`] yet, so a placement there
//! still reaches one of the lazily started cores rather than the sibling serving
//! core ADR 0184 § 5 decides on. The seam and the destination set are here; what
//! is missing is the one call where a serving core starts, which is
//! `crates/nvs-cli/src/serve.rs`'s to make and goal `worker-placement` owns.
//! Until it does, `rule:concurrency/on-worker-runs-the-child-on-another-core`'s
//! last paragraph is written as what runs.
//!
//! What reaches this from above is [`crate::placed`], and it reaches it for both
//! of `spawn script`'s entry forms: a core here is started under the process's
//! published resolver, so a path is compiled on the core that runs it exactly as
//! a method is prepared there. That module's *Both entry forms cross* is the one
//! home of what each form needs, and nothing about either is a question for the
//! transport here.
//! — owner: worker-placement

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use nvs_runtime::{Ctx, OutputSink, TaskPanic, TaskRoot};

use crate::Worker;
use crate::affinity::cpus;
use crate::reactor::{self, Reactor, RemoteWake, run_until_idle};
use crate::scheduler::{
    Scheduler, TaskId, Waiting, cancel_task, current_task, spawn_child, suspend_current,
};

/// What a placement answered with.
///
/// Three states rather than an `Option`, because a child that panicked on
/// another core and a child that was stopped there are different facts about the
/// same placement, and a parent that collapsed them would report a cancellation
/// as a failure. A panic is a **value** by the time it is here — the far core
/// contained it at its own task root, which is `rule:security/isolate-shares-nothing`'s
/// failure-is-a-value applied to a boundary that is also a thread.
#[derive(Debug)]
pub enum Answer<T> {
    /// The job ran to its end on the far core and this is what it returned.
    Value(T),
    /// It panicked there. The message crossed; the payload did not.
    Panicked(TaskPanic),
    /// The child stopped without answering — the cancellation this placement
    /// sent, or a core that was closed while it held the work.
    Stopped,
}

/// How many cores this process may start for placements.
///
/// **The core count**, which is
/// `rule:concurrency/on-worker-runs-the-child-on-another-core`'s number: a
/// placement runs compute rather than waiting on something, so a second thread
/// per core would oversubscribe exactly the cores the work is trying to use —
/// which is the whole difference from [`crate::blocking::bound`]'s factor of
/// two. One when the platform will not say how many CPUs there are, matching
/// [`cpus`]'s own answer for that case.
#[must_use]
pub fn bound() -> usize {
    cpus().len().max(1)
}

/// One placement, as both cores see it.
///
/// Behind an `Arc` shared by the parent that is waiting and the receptionist
/// that started the child: the parent writes [`Placed::cancelled`], the
/// receptionist writes [`Placed::child`], and the child's own guard writes
/// [`Placed::finished`]. Every field is a flag or an id because that is all a
/// cancellation needs to cross — the answer itself travels in the slot, not
/// here.
#[derive(Debug, Default)]
struct Placed {
    /// The child's id on the far core, written by the receptionist that started
    /// it and read by the sweep that cancels it. `None` until then, which is the
    /// window a cancellation arriving before the start falls into — the sweep
    /// that follows the start closes it.
    child: Mutex<Option<TaskId>>,
    /// Set on the parent's core when the parent is cancelled, read by the
    /// receptionist's sweep. Relaxed on both sides: the bell that follows it is
    /// the ordering, and a sweep that ran a moment early is a sweep that runs
    /// again.
    cancelled: AtomicBool,
    /// Set by [`Answering`]'s `Drop`, however the child ended. What the sweep
    /// prunes on, so a finished placement stops being swept for cancellation.
    finished: AtomicBool,
}

impl Placed {
    /// Whether the parent has asked for this child to stop.
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Whether the child has ended, however it ended.
    fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }
}

/// One placement on its way to a core: what to run, and the record both cores
/// keep of it.
struct Start {
    placed: Arc<Placed>,
    /// The work itself, with its own answer slot and wake captured inside it —
    /// the same shape [`crate::blocking::run`] hands its pool, and for the same
    /// reason: dropping this job answers the placement rather than losing it.
    job: Box<dyn FnOnce() + Send + 'static>,
}

/// One core's inbox: the work other cores have written for it, and the bell that
/// ends the wait of the task which drains it.
///
/// Shared across the thread boundary and therefore the only state in this module
/// two cores touch. The queue's lock is held for a push or a `drain` and never
/// across a syscall or a park; the bell's is held for a `take` or a store.
struct Inbox {
    /// Written by any core, drained by this one's receptionist.
    queue: Mutex<VecDeque<Start>>,
    /// This core's receptionist's own wake, armed before each drain and taken by
    /// whichever core writes next. `None` means one is already in flight, which
    /// is the module doc's *bell*: the drain it will cause has not happened yet,
    /// so it will see whatever was just written.
    bell: Mutex<Option<RemoteWake>>,
    /// How many placements this core has been given and not yet finished. The
    /// idle test [`WorkerCores::pick`] starts a core on, which is
    /// [`crate::blocking::BlockingPool::submit`]'s test with a count of work
    /// rather than of threads.
    live: Arc<AtomicUsize>,
    /// Set when the set that owns this core is going away. A core that sees it
    /// stops receiving; a placement that sees it is refused rather than queued
    /// on a core that will never drain it.
    closed: AtomicBool,
}

impl std::fmt::Debug for Inbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hand-written: a `Start` carries a closure and has no `Debug`. What is
        // printed is the state a reader of one of these actually asks about —
        // how much work is here and whether the core is still taking any.
        f.debug_struct("Inbox")
            .field("queued", &lock(&self.queue).len())
            .field("live", &self.live())
            .field("closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

impl Inbox {
    /// An empty inbox for a core that has not started yet.
    fn new() -> Arc<Self> {
        Arc::new(Self {
            queue: Mutex::new(VecDeque::new()),
            bell: Mutex::new(None),
            live: Arc::new(AtomicUsize::new(0)),
            closed: AtomicBool::new(false),
        })
    }

    /// How many placements this core is holding.
    fn live(&self) -> usize {
        self.live.load(Ordering::Relaxed)
    }

    /// Whether this core has stopped receiving.
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    /// Queues `start` for this core and rings its bell, or hands it back if the
    /// core has stopped receiving.
    ///
    /// The refusal is not an error path the caller has to be clever about: a
    /// returned [`Start`] dropped on the floor answers its placement
    /// [`Answer::Stopped`] through [`Answering`]'s `Drop`, which is what the
    /// parent is parked to read.
    fn post(&self, start: Start) -> Option<Start> {
        let mut queue = lock(&self.queue);
        if self.is_closed() {
            return Some(start);
        }
        queue.push_back(start);
        drop(queue);
        self.ring();
        None
    }

    /// Ends the receptionist's park, if it is the one holding the bell.
    ///
    /// A failed poke loses the wake's promptness rather than the wake —
    /// [`RemoteWake::wake`]'s own account of it — and a bell that is already in
    /// flight needs no second one.
    fn ring(&self) {
        if let Some(wake) = lock(&self.bell).take() {
            let _ = wake.wake();
        }
    }

    /// Everything written since the last drain.
    fn take(&self) -> Vec<Start> {
        lock(&self.queue).drain(..).collect()
    }

    /// Stops this core receiving and answers everything still queued.
    ///
    /// The jobs are dropped rather than run, which is the whole of the
    /// answering: each one's [`Answering`] fires from its own `Drop`, so no
    /// parent is left parked on a core that is going away.
    fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
        drop(self.take());
        self.ring();
    }
}

/// The far core's half of a placement: what the child owes its parent however it
/// ends.
///
/// Captured inside the job, so there is no path out of the child — a return, a
/// contained panic, a forced unwind for a cancellation — that does not go
/// through this type's `Drop`. That is what makes
/// `rule:concurrency/nothing-is-still-running-when-a-call-returns` hold across
/// the thread: the parent's park ends exactly when the child has stopped.
struct Answering<T> {
    /// Where the answer goes. Shared with the parent, and outliving the parent's
    /// *frame* rather than its task: a parent force-unwound while this is in
    /// flight leaves the `Arc` holding the slot alone.
    slot: Arc<Mutex<Option<Answer<T>>>>,
    /// The end of the parent's park, fired from this type's `Drop` after the
    /// slot has been written and the lock released.
    wake: Option<RemoteWake>,
    placed: Arc<Placed>,
    live: Arc<AtomicUsize>,
}

impl<T> Answering<T> {
    /// Records what the child answered.
    ///
    /// The wake is deliberately **not** fired here: it goes out from `Drop`,
    /// once, whether or not this was ever called, so the two endings have one
    /// path and a child that never answered cannot leave a parent parked.
    fn answered(&mut self, answer: Answer<T>) {
        *lock(&self.slot) = Some(answer);
    }
}

impl<T> Drop for Answering<T> {
    fn drop(&mut self) {
        {
            let mut slot = lock(&self.slot);
            if slot.is_none() {
                // Nothing was ever written, so this child was torn down — the
                // cancellation the parent sent, or the core closing under it.
                *slot = Some(Answer::Stopped);
            }
        }
        // Before the wake below and after the slot above: the parent may be
        // running the moment the poke lands, and a placement still counted as
        // live would keep `pick` from handing this core its next piece of work.
        self.placed.finished.store(true, Ordering::Relaxed);
        self.live.fetch_sub(1, Ordering::Relaxed);
        // Last, and from the handle's own `Drop`: the answer is in the slot and
        // its lock is released before the parent can be resumed to read it.
        drop(self.wake.take());
    }
}

/// The parent's half of a placement, for as long as it is waiting.
///
/// Its `Drop` is the force-unwind path and nothing else: a parent whose stack
/// the scheduler is tearing down cannot park for the acknowledgement, so it
/// sends the cancellation and lets the far core's teardown finish on its own
/// time. The module doc owns why that is the one case that may not wait.
struct Awaited {
    placed: Arc<Placed>,
    inbox: Arc<Inbox>,
}

impl Awaited {
    /// Asks the far core to stop this child, once.
    ///
    /// Idempotent by construction: the flag is already set on a second call and
    /// the ring finds no bell, so a parent resumed with a cancellation and then
    /// unwound anyway pays one poke rather than two.
    fn cancel(&self) {
        if self.placed.is_finished() {
            return;
        }
        self.placed.cancelled.store(true, Ordering::Relaxed);
        self.inbox.ring();
    }
}

impl Drop for Awaited {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// A placement that has been posted and not collected yet: a child already on
/// its way to another core, and the whole of what its parent holds.
///
/// [`post`] answers one of these without parking, [`Posted::collect`] parks for
/// what the child answered, and [`Posted::finished`] asks whether that park
/// would wait at all. Dropping one instead is legal and is not a leak: the drop
/// is [`Awaited`]'s, which cancels the child and lets the far core tear it down
/// on its own time — [`nvs_runtime::host::Running`]'s own reading of a spawn
/// nobody awaited, with the answer discarded rather than crossing.
pub struct Posted<T> {
    /// Where the far core leaves the answer. The record; the wake that ends the
    /// park only says there is something to re-read.
    slot: Arc<Mutex<Option<Answer<T>>>>,
    /// The cancellation half, which is also this handle's `Drop`.
    awaited: Awaited,
}

impl<T> std::fmt::Debug for Posted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hand-written so that a `T` with no `Debug` of its own still leaves the
        // handle printable, and what is printed is what a reader asks of one:
        // whether there is anything left to wait for.
        f.debug_struct("Posted")
            .field("finished", &self.finished())
            .finish_non_exhaustive()
    }
}

impl<T> Posted<T> {
    /// Whether the child has ended, asked **without waiting** for it.
    ///
    /// [`Answering`]'s `Drop` is what sets it, so it is true for a child that
    /// ran to its end, one that panicked on the far core and one torn down
    /// half-way — every state in which [`Posted::collect`] has nothing to park
    /// for. `false` says only that the child had not ended when it was asked;
    /// what says to ask again is the wake, never a re-read on a loop.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.awaited.placed.is_finished()
    }

    /// Parks the calling task until the child's answer is in the slot, and
    /// answers with it.
    ///
    /// The slot is read before the first park, because the child may have ended
    /// while its parent was doing something else — which is the whole point of
    /// posting separately — and a wake delivered before that park is one the
    /// park would otherwise wait out.
    pub fn collect(self) -> Answer<T> {
        loop {
            if let Some(answer) = lock(&self.slot).take() {
                return answer;
            }
            let resumed = suspend_current(Waiting::Parked);
            if resumed.cancelled() {
                // ADR 0184 § 4: the cancellation goes across and this call keeps
                // waiting for the acknowledgement, which arrives as the child's
                // teardown filling the slot. Ending the wait here would return
                // with work still running on another core.
                self.awaited.cancel();
            } else if !resumed.suspended() {
                // Unreachable in practice: this handle exists, so a task asked
                // for it and there is a scheduler. Yielding rather than
                // asserting keeps a hypothetical wrong answer here a slow loop
                // instead of a killed worker.
                std::thread::yield_now();
            }
        }
    }

    /// Ends the child now, discards whatever it had produced, and does not
    /// return while it is still running.
    ///
    /// [`nvs_runtime::host::Running::abandon`]'s contract, and its one carve-out
    /// with it: a stack the scheduler is force-unwinding may not park, so the
    /// cancellation goes out and the wait does not happen. The child dies either
    /// way — what the wait buys is that it has *already* died when this returns.
    pub fn abandon(self) {
        self.awaited.cancel();
        if self.finished() || nvs_runtime::Teardown::in_progress() {
            return;
        }
        drop(self.collect());
    }
}

/// The cores this process has started for placements, and the bound on them.
///
/// One per process in practice — [`cores`] holds it — and a type rather than a
/// set of statics so that a test can have a small one of its own, exactly as
/// [`crate::blocking::BlockingPool::new`] exists beside the thread-local pool.
#[derive(Debug)]
struct WorkerCores {
    cores: Vec<Core>,
    bound: usize,
    /// Where the next round-robin pick starts. Only reached once every core is
    /// busy and the bound is spent, which is the state the placement has to wait
    /// somewhere rather than run somewhere free.
    next: usize,
}

/// One core in the set: its inbox, and the thread this set started for it.
///
/// `worker` is `None` for a core that was **registered** rather than started. A
/// serving core already has a thread, a scheduler and a reactor of its own
/// before any placement is written, and drains its inbox from a receptionist on
/// that scheduler ([`WorkerCores::register`]), so what the set holds of one is
/// the inbox alone — there is no thread here that is this set's to end.
#[derive(Debug)]
struct Core {
    inbox: Arc<Inbox>,
    worker: Option<Worker<()>>,
}

impl WorkerCores {
    /// An empty set that will start at most `bound` cores.
    ///
    /// No thread is started here: the first one arrives with the first
    /// placement, so a process that places nothing never pays for this at all.
    /// `bound` is clamped up to one, because a set that may start no core is an
    /// inbox nobody drains.
    fn new(bound: usize) -> Self {
        Self {
            cores: Vec::new(),
            bound: bound.max(1),
            next: 0,
        }
    }

    /// How many cores this set has started a thread for.
    ///
    /// Not the size of the destination set: a registered core is in that set and
    /// costs this process no thread, which is the whole of what registering one
    /// buys. [`Self::pick`] measures the bound against the set's size rather
    /// than against this, because a core that is already reachable is a core
    /// this set may not start a second of.
    fn started(&self) -> usize {
        self.cores
            .iter()
            .filter(|core| core.worker.is_some())
            .count()
    }

    /// Adds a core this set did not start, and answers the inbox it is to drain.
    ///
    /// The draining is the caller's: a core that is already turning a scheduler
    /// spawns a receptionist on it, so what crosses to it is the same [`Start`] a
    /// started core reads and the only difference is whose thread reads it. That
    /// is ADR 0184 § 5's destination — a serving core's thread exists and is
    /// pinned already, and a second thread per core for placement work would
    /// oversubscribe exactly the cores the server's latency depends on.
    ///
    /// It occupies a slot against the bound like any other core, so a process
    /// that registers one per CPU starts none of its own.
    fn register(&mut self) -> Arc<Inbox> {
        let inbox = Inbox::new();
        self.cores.push(Core {
            inbox: Arc::clone(&inbox),
            worker: None,
        });
        inbox
    }

    /// Takes a registered core back out and closes its inbox, so nothing is
    /// written to a core whose receptionist has gone.
    ///
    /// Closing is the whole of what this set owes one: it answers the placements
    /// already queued there, each through its own [`Answering`]'s `Drop`, and
    /// there is no thread to join because there is none this set started.
    fn deregister(&mut self, inbox: &Arc<Inbox>) {
        self.cores.retain(|core| !Arc::ptr_eq(&core.inbox, inbox));
        inbox.close();
    }

    /// The inbox of a core to place work on, starting one if every core it can
    /// reach is busy and the bound allows another.
    ///
    /// `mine` is the placing core's own inbox where it registered one, and is
    /// never the answer: a child is started on a core *other than its parent's*
    /// (`rule:concurrency/on-worker-runs-the-child-on-another-core`), and a
    /// serving core that offered itself is in the very set being asked.
    ///
    /// The idle test first, the bound second, the round robin last — and **a set
    /// that can reach a registered core never grows itself**, which is ADR 0184
    /// § 5: a registered core's thread is already running and already pinned, so
    /// starting one beside it would oversubscribe exactly the core the placement
    /// is trying to use. At the bound the placement joins a queue instead, which
    /// is the bound doing its job: refusing the work would put an error on a
    /// spawn that means nothing but "the machine is busy", and the alternative to
    /// *that* is one thread per placement, which is what the rule bounds at the
    /// core count to avoid.
    ///
    /// `None` when there is no core here but the one asking — a platform that
    /// lists no CPU to pin a core to and an OS that refuses a thread for it, and
    /// a process whose registered cores are the single CPU it was given, where
    /// `on: "worker"` is `on: "here"` by arithmetic. [`place`] then runs the work
    /// where it stands.
    fn pick(&mut self, mine: Option<&Arc<Inbox>>) -> Option<Arc<Inbox>> {
        let elsewhere = |core: &Core| match mine {
            Some(mine) => !Arc::ptr_eq(&core.inbox, mine),
            None => true,
        };
        if let Some(idle) = self
            .cores
            .iter()
            .find(|core| elsewhere(core) && core.inbox.live() == 0)
        {
            return Some(Arc::clone(&idle.inbox));
        }
        let reaches_a_registered_core = self
            .cores
            .iter()
            .any(|core| elsewhere(core) && core.worker.is_none());
        if !reaches_a_registered_core && self.cores.len() < self.bound && self.start_one() {
            return self.cores.last().map(|core| Arc::clone(&core.inbox));
        }
        // Round robin from where the last one left off, over the cores that are
        // not the one asking. `held` is zero only for a set that has neither
        // registered nor started anything, and then the loop does not run.
        let held = self.cores.len();
        for step in 1..=held {
            let at = (self.next + step) % held;
            if elsewhere(&self.cores[at]) {
                self.next = at;
                return Some(Arc::clone(&self.cores[at].inbox));
            }
        }
        None
    }

    /// Starts one core and adds it to the set, answering whether it is there.
    ///
    /// A CPU the platform did not list and a thread the OS refused are the same
    /// answer: this set is one core smaller than its bound allows, and the work
    /// goes to a core that exists — which is the same shape as being at the
    /// bound. `affinity`'s module doc owns why a refusal to *pin* is not an
    /// error at all.
    ///
    /// The core is started **under the process's published resolver**, which is
    /// the one thing it could not acquire for itself afterwards: a resolver is
    /// per thread and this thread is this crate's, so the handle goes in where
    /// the reactor and the receptionist do. What that shares is the unit cache
    /// and nothing else, which is the sharing
    /// `rule:security/isolate-shares-nothing` permits and the only one.
    fn start_one(&mut self) -> bool {
        let Some(cpu) = cpus().get(self.cores.len()).copied() else {
            return false;
        };
        let inbox = Inbox::new();
        let mine = Arc::clone(&inbox);
        // The process's resolver handle, read here rather than on the new
        // thread. It has to be `Send` to reach one at all, which is what
        // `nvs_runtime::script::SharedResolver` is; taking it at the moment the
        // core is decided on is what makes the resolver a placement was measured
        // against and the resolver its core runs under the same one. `None` is a
        // process that published none — a `nvs check`, a test — and that core
        // answers a path entry the way it always has.
        let resolver = nvs_runtime::script::published();
        let Ok(worker) = Worker::spawn(cpu, move |sched| match resolver {
            Some(handle) => handle.scoped(|| run_core(&mine, sched)),
            None => run_core(&mine, sched),
        }) else {
            return false;
        };
        self.cores.push(Core {
            inbox,
            worker: Some(worker),
        });
        true
    }
}

impl Drop for WorkerCores {
    fn drop(&mut self) {
        // And **a join**, unlike `BlockingPool`: this set is owned by a `static`
        // in the process that never drops it, or by a test's own stack, and
        // neither of those is the TLS destructor whose loader lock made joining
        // a deadlock there. What is joined is a core that has answered every
        // placement it held, because closing the inbox answers the queued ones
        // and ending the receptionist cancels the running ones.
        for core in std::mem::take(&mut self.cores) {
            core.inbox.close();
            // A thread this set started is one it may wait for; a registered
            // core's thread is somebody else's, and the close above is all it is
            // owed — the receptionist there reads it and returns on its own
            // scheduler.
            if let Some(worker) = core.worker {
                let _ = worker.join();
            }
        }
    }
}

/// The cores this process places work on, built on first use.
///
/// A `static` rather than a thread-local, which is the one place this module
/// departs from [`crate::blocking`]: a pool is per worker because its threads
/// are its own, while a *core* is a property of the machine — two cores each
/// starting their own set would oversubscribe the very cores the bound exists to
/// protect.
fn cores() -> &'static Mutex<WorkerCores> {
    static CORES: OnceLock<Mutex<WorkerCores>> = OnceLock::new();
    CORES.get_or_init(|| Mutex::new(WorkerCores::new(bound())))
}

thread_local! {
    /// The inbox this thread registered for its own core, if it registered one.
    ///
    /// The one thing a pick has to know about the *placing* core rather than
    /// about the set, and the reason it is here rather than in [`WorkerCores`]:
    /// which core is asking is a property of the thread, and the set is one
    /// process-wide thing every core reads. A thread that registered nothing —
    /// every `nvs run`, every test, and the lazily started cores themselves —
    /// leaves it `None` and is excluded from nothing.
    static MINE: RefCell<Option<Arc<Inbox>>> = const { RefCell::new(None) };
}

/// Runs `f` on another core and returns what it answered, giving this core back
/// while it runs.
///
/// This is `rule:concurrency/on-worker-runs-the-child-on-another-core`'s start,
/// and the placement is decided here and never revisited: `f` runs as a task on
/// the core this hands it to, and that task never migrates.
///
/// [`post`] and [`Posted::collect`] back to back, for a caller that has nothing
/// to do between them. A caller that does — one child per core, all of them
/// running before any is waited for — posts them itself.
///
/// Off a core the function is simply called on this thread — there is no core to
/// protect, no task to park, and nothing to hand back. So is the case where no
/// core could be started at all, which is a platform that lists no CPU or an OS
/// refusing a thread.
///
/// # Panics
///
/// Not here. A panic inside `f` is contained at the far core's own task root and
/// crosses as [`Answer::Panicked`], because a panic is a value at this boundary
/// and resuming one on the parent's stack would make a child able to end its
/// parent.
pub fn place<T, F>(f: F) -> Answer<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    place_on(cores(), f)
}

/// Starts `f` on another core and hands back the handle that collects it, or
/// gives `f` back when there is no core to start it on.
///
/// The eager half of a placement, and the one
/// [`nvs_runtime::host::Host::start_isolate`]'s contract asks for: `f` is a
/// runnable task on the far core before this returns, so a parent that posts
/// three children has three of them going while it is still posting. The park
/// is [`Posted::collect`]'s and belongs to whoever wants the answer.
///
/// `Err(f)` is the function handed back untouched, for a caller off a core, one
/// whose reactor cannot issue a wake, and a set that could start no core at all
/// — the three states [`place`] answers by calling `f` where it stands. It is
/// not a failure: nothing has been started, so the caller still owns every
/// choice it had.
///
/// # Panics
///
/// Not here, and not in [`Posted::collect`] either: [`place`]'s own note owns
/// why a panic on the far core crosses as [`Answer::Panicked`].
pub fn post<T, F>(f: F) -> Result<Posted<T>, F>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    post_on(cores(), f)
}

/// How many cores this process has started for placements, and the bound on
/// them.
///
/// `(0, _)` for a process that has never placed a child, which is where every
/// program that writes no `on: "worker"` stays — the same question, and the same
/// answer shape, as [`crate::blocking::pool_size`].
#[must_use]
pub fn cores_started() -> (usize, usize) {
    let cores = lock(cores());
    (cores.started(), cores.bound)
}

/// A core this process offered as a placement destination, withdrawn when this
/// is dropped.
///
/// A guard rather than a pair of calls because withdrawing is not optional: an
/// inbox left in the set after its receptionist has gone is a core placements
/// are written to and never drained from, and the parent parked on one would
/// wait for an answer nobody is going to write.
#[derive(Debug)]
pub struct Registered {
    inbox: Arc<Inbox>,
}

impl Drop for Registered {
    fn drop(&mut self) {
        // Matched rather than cleared, because this guard is `Send` and the
        // thread that drops one need not be the thread that made it. A thread
        // holding somebody else's inbox in `MINE` would exclude itself from a
        // core it may perfectly well place on.
        MINE.with(|mine| {
            let mut mine = mine.borrow_mut();
            if mine
                .as_ref()
                .is_some_and(|held| Arc::ptr_eq(held, &self.inbox))
            {
                *mine = None;
            }
        });
        lock(cores()).deregister(&self.inbox);
    }
}

/// Offers the core this thread is turning as a placement destination, and
/// answers the guard that withdraws it.
///
/// The receptionist is spawned on `sched` — the caller's own scheduler, which is
/// the difference this whole seam exists for. [`run_core`] spawns the identical
/// task on a scheduler this module started; a core that already has one, has a
/// reactor and is pinned needs neither of those built for it a second time, and
/// ADR 0184 § 5 is why a serving fleet may not afford them.
///
/// # What it spends
///
/// One inbox and one long-lived task on this core, and no thread at all — which
/// is what `rule:programs/memory-priority` asks to be said out loud, and the
/// reason a deployment that places nothing pays nothing for having offered.
pub fn register_this_core(sched: &mut Scheduler) -> Registered {
    let inbox = lock(cores()).register();
    // This thread's own core, so that a placement written here is answered a
    // sibling and never this inbox — [`WorkerCores::pick`]'s `mine`.
    MINE.with(|mine| *mine.borrow_mut() = Some(Arc::clone(&inbox)));
    let mine = Arc::clone(&inbox);
    // `TaskRoot::Worker` for [`run_core`]'s reason: the receptionist is
    // worker-owned work with no request beneath it, so a fault in it retires
    // this core's inbox rather than failing somebody's request.
    sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Worker, move |_| {
        receive(&mine);
    });
    Registered { inbox }
}

/// [`place`], against a named set of cores.
///
/// The seam a test reaches: a set of two proves the bound without starting one
/// core per CPU on the machine running the suite.
fn place_on<T, F>(cores: &Mutex<WorkerCores>, f: F) -> Answer<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match post_on(cores, f) {
        Ok(posted) => posted.collect(),
        Err(f) => here(f),
    }
}

/// A core that has agreed to take one placement, held before the work exists.
///
/// [`post`] is the whole of this for a caller whose work is a closure it already
/// owns: it answers a [`Posted`] or hands the closure back untouched. A caller
/// that has to **consume** something to build the closure needs the two steps
/// apart, and `spawn script … on: "worker"` is one —
/// [`nvs_runtime::graph::encode`] takes the argument's reference, so a spawn
/// that encoded first and was then refused a core would hold neither a placement
/// nor a value to start where it stands.
///
/// So the destination is secured first and [`Destination::post`] cannot fail.
/// What it holds is exactly the two things [`post`] refuses for: a core to write
/// to, and this task's own [`RemoteWake`] to be woken by. Dropping one without
/// posting is free — the core was never told anything.
pub struct Destination {
    inbox: Arc<Inbox>,
    wake: RemoteWake,
}

impl std::fmt::Debug for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hand-written because a `RemoteWake` has no `Debug`, and what a reader
        // asks of one of these is which core is waiting for the work.
        f.debug_struct("Destination")
            .field("inbox", &self.inbox)
            .finish_non_exhaustive()
    }
}

impl Destination {
    /// Starts `f` on the core this holds, and answers the handle that collects
    /// it.
    ///
    /// [`post`]'s body with the three refusals already behind it, so there is
    /// no failure left to report: the work is queued and the core is rung.
    ///
    /// # Panics
    ///
    /// Not here — [`place`]'s own note owns why a panic on the far core crosses
    /// as [`Answer::Panicked`].
    pub fn post<T, F>(self, f: F) -> Posted<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let Self { inbox, wake } = self;
        post_to(&inbox, wake, f)
    }
}

/// The core a placement written now would go to, or `None` where there is none —
/// off a core, with a reactor that cannot issue a wake, or with a set that could
/// start no core at all.
///
/// The three states [`post`] hands the function back for, asked before the work
/// is built. [`Destination`]'s own doc owns why a caller would want them asked
/// that early.
#[must_use]
pub fn destination() -> Option<Destination> {
    destination_on(cores())
}

/// [`destination`], against a named set of cores — [`post_on`]'s seam.
fn destination_on(cores: &Mutex<WorkerCores>) -> Option<Destination> {
    let me = current_task()?;
    let mine = MINE.with(|mine| mine.borrow().clone());
    let inbox = lock(cores).pick(mine.as_ref())?;
    // Rule 1's ordering, and the reason the core is picked first: the handle
    // exists before anything can be written to the inbox that would fire it.
    let wake = reactor::with_current(|reactor| reactor.remote_wake(me))?;
    Some(Destination { inbox, wake })
}

/// [`post`], against a named set of cores — [`place_on`]'s seam, for the half
/// of it that does not park.
fn post_on<T, F>(cores: &Mutex<WorkerCores>, f: F) -> Result<Posted<T>, F>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match destination_on(cores) {
        Some(destination) => Ok(destination.post(f)),
        None => Err(f),
    }
}

/// Queues the work on a core already picked and rings its bell.
///
/// The half of a placement that cannot refuse, shared by [`post_on`] and
/// [`Destination::post`] so that a caller which secured its core early and one
/// which did not build the same [`Start`].
fn post_to<T, F>(inbox: &Arc<Inbox>, wake: RemoteWake, f: F) -> Posted<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let inbox = Arc::clone(inbox);
    let slot: Arc<Mutex<Option<Answer<T>>>> = Arc::new(Mutex::new(None));
    let placed = Arc::new(Placed::default());
    inbox.live.fetch_add(1, Ordering::Relaxed);
    let mut answering = Answering {
        slot: Arc::clone(&slot),
        wake: Some(wake),
        placed: Arc::clone(&placed),
        live: Arc::clone(&inbox.live),
    };
    let start = Start {
        placed: Arc::clone(&placed),
        job: Box::new(move || {
            // `run_task` rather than a bare `catch_unwind`: it is the boundary
            // that knows a forced unwind from a panic, so a child cancelled
            // mid-flight is torn down rather than reported as having failed.
            answering.answered(
                nvs_runtime::run_task(TaskRoot::Request, f)
                    .map_or_else(Answer::Panicked, Answer::Value),
            );
        }),
    };
    // A refused post is answered by dropping it: the job's `Answering` fires
    // from its own `Drop`, so the park below reads `Stopped` on its first turn
    // rather than waiting for a core that has stopped receiving.
    drop(inbox.post(start));

    Posted {
        slot,
        awaited: Awaited { placed, inbox },
    }
}

/// The answer for a caller with no core to hand back.
fn here<T, F>(f: F) -> Answer<T>
where
    F: FnOnce() -> T,
{
    Answer::Value(f())
}

/// One worker core's whole life: a reactor, a receptionist, and the same loop
/// every other core in this crate runs.
///
/// A reactor the OS refused is the one state this cannot proceed from — a
/// receptionist could not park and a child could not wait on anything — so the
/// inbox is closed, which answers every placement already queued and refuses
/// every later one, and the core returns rather than pretending to receive.
fn run_core(inbox: &Arc<Inbox>, sched: &mut Scheduler) {
    let Ok(reactor) = Reactor::new() else {
        inbox.close();
        return;
    };
    let _installed = reactor::install(reactor);
    let mine = Arc::clone(inbox);
    // `TaskRoot::Worker`: the receptionist is worker-owned work with no request
    // beneath it, so a fault in it retires this core rather than failing
    // somebody's placement — `nvs_runtime::TaskRoot`'s own split.
    sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Worker, move |_| {
        receive(&mine);
    });
    let _ = run_until_idle(sched);
}

/// The receptionist: drain the inbox, start what is in it, sweep for
/// cancellations, park.
///
/// The order is the module doc's, and each step of it is load-bearing. The bell
/// is armed *before* the drain so nothing written between them is missed. The
/// sweep runs after the drain so a cancellation that arrived with the start it
/// cancels is seen on the same turn. The close test is last, so a core closed
/// while it was holding work still answers that work before it goes.
fn receive(inbox: &Arc<Inbox>) {
    let mut live: Vec<Arc<Placed>> = Vec::new();
    loop {
        arm(inbox);
        for Start { placed, job } in inbox.take() {
            // A child of the receptionist rather than a bare root, which is what
            // makes this core's teardown its children's: the receptionist
            // returning cancels everything still placed here, and nothing else
            // on this core can end it. Its resources are counted against this
            // core, which is the module doc's known gap and the reason nothing
            // above this crate places an isolate yet.
            let started = spawn_child(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_| {
                job()
            });
            *lock(&placed.child) = started;
            live.push(placed);
        }
        live.retain(|placed| {
            if placed.is_finished() {
                return false;
            }
            if placed.is_cancelled() {
                // Marks and returns; the child dies on this core's own teardown
                // path, and its `Answering` is what tells the parent so.
                if let Some(id) = *lock(&placed.child) {
                    cancel_task(id);
                }
            }
            true
        });
        if inbox.is_closed() {
            return;
        }
        suspend_current(Waiting::Parked);
    }
}

/// Arms the bell for the next core that writes here, if nobody is holding one.
///
/// Never a second one while the first is in flight: dropping a [`RemoteWake`]
/// delivers it, so replacing an armed bell would wake this task with nothing to
/// find and park it again on the next turn, forever.
fn arm(inbox: &Inbox) {
    let Some(me) = current_task() else {
        return;
    };
    let mut bell = lock(&inbox.bell);
    if bell.is_none() {
        *bell = reactor::with_current(|reactor| reactor.remote_wake(me));
    }
}

/// The lock, with a poisoned one taken anyway.
///
/// Nothing in this module panics while holding one, so poisoning can only be
/// inherited from a job's unwind — and what is behind these locks is a queue of
/// boxes, an `Option` and an answer slot, none of which is less consistent for
/// it. Refusing instead would wedge every later placement on one panicking
/// child, which is precisely the tier B failure
/// `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers` exists to
/// prevent.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::HelperFrame;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::mpsc;
    use std::thread::ThreadId;
    use std::time::Duration;

    /// Where a task on this thread's core leaves what its placement answered,
    /// for the test that spawned it to read once the loop is over.
    type Collected<T> = Rc<Cell<Option<Answer<T>>>>;

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    /// A core of this thread's own, so a test can spawn tasks and drive them the
    /// way a worker does.
    fn core() -> (Scheduler, reactor::Installed) {
        (
            Scheduler::new(),
            reactor::install(Reactor::new().expect("the OS refused a poll")),
        )
    }

    #[test]
    fn the_bound_is_the_core_count() {
        assert_eq!(
            bound(),
            cpus().len().max(1),
            "`rule:concurrency/on-worker-runs-the-child-on-another-core`'s number"
        );
        assert!(bound() >= 1, "a set that can start no core drains nothing");
    }

    #[test]
    fn a_destination_is_secured_before_the_work_is_built() {
        // The question a spawn has to ask in this order: encoding its argument
        // consumes it, so "is there a core" must be answerable while the value
        // is still a value.
        assert!(
            destination().is_none(),
            "a thread with no task on it secured a core"
        );

        let (mut sched, _installed) = core();
        let parent = std::thread::current().id();

        let answer: Collected<ThreadId> = Rc::new(Cell::new(None));
        let collected = Rc::clone(&answer);
        sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            let destination = destination().expect("a task on a core secured none");
            collected.set(Some(
                destination.post(|| std::thread::current().id()).collect(),
            ));
        });
        run_until_idle(&mut sched).expect("the loop failed");

        let Some(Answer::Value(ran_on)) = answer.take() else {
            panic!("the placement never answered");
        };
        assert_ne!(
            ran_on, parent,
            "a secured destination ran the work on the core that posted it"
        );
    }

    #[test]
    fn a_worker_child_runs_on_another_core_and_its_answer_is_copied_back() {
        let (mut sched, _installed) = core();
        let parent = std::thread::current().id();

        let answer: Collected<(ThreadId, u32)> = Rc::new(Cell::new(None));
        let collected = Rc::clone(&answer);
        sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            collected.set(Some(place(move || (std::thread::current().id(), 6 * 7))));
        });
        let report = run_until_idle(&mut sched).expect("the loop failed");

        let Some(Answer::Value((ran_on, value))) = answer.take() else {
            panic!("the child never answered");
        };
        assert_ne!(
            ran_on, parent,
            "the child ran on the core that placed it, which is `on: \"here\"`"
        );
        assert_eq!(value, 42, "the answer did not cross");
        assert_eq!(report.finished, 1);
        assert_eq!(
            report.parked, 0,
            "the core gave up on a task it was told to expect"
        );
    }

    #[test]
    fn a_cancelled_parent_cancels_its_worker_child_across_cores() {
        let (mut sched, _installed) = core();
        let (stopped, child_stopped) = mpsc::channel();
        let (running, child_running) = mpsc::channel();

        let answer: Collected<()> = Rc::new(Cell::new(None));
        let collected = Rc::clone(&answer);
        let parent = sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            // A helper frame makes this stack one a cancellation may not unwind,
            // which is what every `spawn script` parent is standing on: the task
            // is resumed with its cancellation instead, so `place` is reached
            // again and gets to wait for the acknowledgement. `nvs_runtime`'s
            // `HelperFrame` owns why an `extern "C"` frame decides that.
            let _frame = HelperFrame::enter();
            collected.set(Some(place(move || {
                // The far side reports that it is up, then parks until this core
                // tears it down. `Answering`'s `Drop` is what answers.
                let _ending = Stopping(stopped);
                running.send(()).expect("the parent went away");
                loop {
                    suspend_current(Waiting::Parked);
                }
            })));
        });

        // Cancelled from a peer on the parent's own core, which is where a
        // sibling's throw or a deadline cancels a task from.
        let killer = sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            child_running
                .recv_timeout(Duration::from_secs(5))
                .expect("the placed child never started");
            cancel_task(parent);
        });
        assert_ne!(killer, parent);
        run_until_idle(&mut sched).expect("the loop failed");

        child_stopped
            .recv_timeout(Duration::from_secs(5))
            .expect("the child was still running after its parent's call was over");
        assert!(
            matches!(answer.take(), Some(Answer::Stopped)),
            "a cancelled child answered as if it had run to its end"
        );
    }

    /// Reports that the far side's task has ended, from the teardown that ends
    /// it — the one place a force-unwound stack can still say anything.
    struct Stopping(mpsc::Sender<()>);

    impl Drop for Stopping {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }

    #[test]
    fn worker_cores_are_started_lazily_and_bounded_by_the_core_count() {
        let cores = Arc::new(Mutex::new(WorkerCores::new(2)));
        assert_eq!(
            lock(&cores).started(),
            0,
            "a set started a core before anything was placed on it"
        );

        let (up, arrived) = mpsc::channel();
        let (release, go) = mpsc::channel::<()>();
        let go = Arc::new(Mutex::new(go));

        // The placing side is a core of its own, so the assertions below can be
        // made from this thread while the placements are still in flight.
        let placing = Arc::clone(&cores);
        let placer = std::thread::spawn(move || {
            let (mut sched, _installed) = core();
            let answers = Rc::new(Cell::new(0_usize));
            for _ in 0..3 {
                let cores = Arc::clone(&placing);
                let up = up.clone();
                let go = Arc::clone(&go);
                let counted = Rc::clone(&answers);
                sched.spawn(ctx(), TaskRoot::Worker, move |_| {
                    let answer = place_on(&cores, move || {
                        up.send(()).expect("the test went away");
                        lock(&go).recv().expect("the test went away");
                    });
                    assert!(matches!(answer, Answer::Value(())));
                    counted.set(counted.get() + 1);
                });
            }
            run_until_idle(&mut sched).expect("the loop failed");
            answers.get()
        });

        // Two placements are running; the third is queued behind one of them,
        // because two is the bound this set was built with.
        for _ in 0..2 {
            arrived
                .recv_timeout(Duration::from_secs(5))
                .expect("a placement never reached a core");
        }
        assert_eq!(
            lock(&cores).started(),
            2,
            "the bound was exceeded, or a core was not started for work that was waiting"
        );
        assert!(
            arrived.recv_timeout(Duration::from_millis(50)).is_err(),
            "a third core took the work the bound says has to wait"
        );

        for _ in 0..3 {
            release.send(()).expect("a placed child went away");
        }
        assert_eq!(placer.join().expect("the placing core panicked"), 3);
        assert_eq!(
            lock(&cores).started(),
            2,
            "the set grew while it was draining"
        );
    }

    /// A core the set only registered takes placements, and this set starts no
    /// thread for it.
    ///
    /// The shape a serving core has: its own scheduler, its own reactor, and the
    /// receptionist draining the inbox as one task beside everything else it
    /// runs. `started()` answering zero while the placement is running on it is
    /// the point — the destination set grew and this process's thread count did
    /// not.
    #[test]
    fn a_registered_core_runs_placements_on_a_scheduler_this_set_never_started() {
        let cores = Arc::new(Mutex::new(WorkerCores::new(2)));
        let inbox = lock(&cores).register();
        assert_eq!(
            lock(&cores).started(),
            0,
            "registering a core started a thread for it"
        );

        let mine = Arc::clone(&inbox);
        let (up, arrived) = mpsc::channel();
        let registered = std::thread::spawn(move || {
            let (mut sched, _installed) = core();
            let drained = Arc::clone(&mine);
            up.send(std::thread::current().id())
                .expect("the test went away");
            sched.spawn(ctx(), TaskRoot::Worker, move |_| receive(&drained));
            run_until_idle(&mut sched).expect("the loop failed");
        });
        let receptionist = arrived
            .recv_timeout(Duration::from_secs(5))
            .expect("the registered core never reached its loop");

        // The placing side is a core of its own, as it is for any placement:
        // `place_on` parks the task that asked, and a parked task needs a
        // scheduler under it.
        let placing = Arc::clone(&cores);
        let placer = std::thread::spawn(move || {
            let (mut sched, _installed) = core();
            let answer: Collected<ThreadId> = Rc::new(Cell::new(None));
            let collected = Rc::clone(&answer);
            sched.spawn(ctx(), TaskRoot::Worker, move |_| {
                collected.set(Some(place_on(&placing, || std::thread::current().id())));
            });
            run_until_idle(&mut sched).expect("the loop failed");
            answer.take()
        });

        let Some(Answer::Value(ran_on)) = placer.join().expect("the placing core panicked") else {
            panic!("the placement never answered");
        };
        assert_eq!(
            ran_on, receptionist,
            "the placement ran somewhere other than the core it was handed"
        );
        assert_eq!(
            lock(&cores).started(),
            0,
            "a set that could reach a core started one of its own anyway"
        );

        lock(&cores).deregister(&inbox);
        registered.join().expect("the registered core panicked");
        assert!(
            inbox.is_closed(),
            "a withdrawn core is still being written to"
        );
    }

    /// A set that can reach a core somebody else is running starts none of its
    /// own, and never answers the placing core its own inbox.
    ///
    /// Both halves are ADR 0184 § 5: the destination under `nvs serve` is a
    /// *sibling* serving core, and the threads it round-robins over are the ones
    /// already running. A set whose only core is the one asking has nowhere to
    /// put the work, which is the single-CPU arithmetic [`WorkerCores::pick`]'s
    /// own doc names and the one place this answers `None`.
    #[test]
    fn a_registered_core_is_preferred_to_starting_one_and_is_never_its_own_destination() {
        let mut cores = WorkerCores::new(4);
        let mine = cores.register();
        let sibling = cores.register();

        let picked = cores.pick(Some(&mine)).expect("a set of two answered none");
        assert!(
            Arc::ptr_eq(&picked, &sibling),
            "a core was handed its own inbox, which is `on: \"here\"` in costume"
        );
        assert_eq!(
            cores.started(),
            0,
            "a set with a running core to reach started a thread beside it"
        );

        let mut alone = WorkerCores::new(1);
        let only = alone.register();
        assert!(
            alone.pick(Some(&only)).is_none(),
            "a process whose one core is the one asking placed a child on itself"
        );
        assert_eq!(
            alone.started(),
            0,
            "the bound was spent by the core it holds"
        );

        // Nothing registered is the unchanged case: `nvs run` grows the set
        // lazily, one core at a time, exactly as it did before any of this.
        let mut lazily = WorkerCores::new(2);
        assert!(
            lazily.pick(None).is_some(),
            "a set with room started nothing"
        );
        assert_eq!(lazily.started(), 1, "the first placement started no core");
    }

    #[test]
    fn two_posted_children_run_at_once_before_either_is_collected() {
        let cores = Mutex::new(WorkerCores::new(2));
        let (mut sched, _installed) = core();

        let (first_up, first_arrived) = mpsc::channel();
        let (second_up, second_arrived) = mpsc::channel();

        let answers: Rc<Cell<Option<(ThreadId, ThreadId)>>> = Rc::new(Cell::new(None));
        let collected = Rc::clone(&answers);
        sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            // Each child waits for the other, so neither can answer until both
            // are running. A placement that only started at the collect would
            // leave the first one waiting out its own timeout here.
            let Ok(first) = post_on(&cores, move || {
                first_up.send(()).expect("the test went away");
                second_arrived
                    .recv_timeout(Duration::from_secs(5))
                    .expect("the sibling was not running while this one was");
                std::thread::current().id()
            }) else {
                panic!("the first child was not posted");
            };
            let Ok(second) = post_on(&cores, move || {
                second_up.send(()).expect("the test went away");
                first_arrived
                    .recv_timeout(Duration::from_secs(5))
                    .expect("the sibling was not running while this one was");
                std::thread::current().id()
            }) else {
                panic!("the second child was not posted");
            };
            let (Answer::Value(one), Answer::Value(two)) = (first.collect(), second.collect())
            else {
                panic!("a posted child never answered");
            };
            collected.set(Some((one, two)));
        });
        run_until_idle(&mut sched).expect("the loop failed");

        let Some((one, two)) = answers.take() else {
            panic!("the parent never collected what it posted");
        };
        let placing = std::thread::current().id();
        assert_ne!(
            one, placing,
            "a posted child ran on the core that posted it"
        );
        assert_ne!(
            two, placing,
            "a posted child ran on the core that posted it"
        );
        assert_ne!(
            one, two,
            "both children took one core, which two overlapping placements may not"
        );
    }

    #[test]
    fn a_placement_posted_off_a_core_hands_the_function_back() {
        let cores = Mutex::new(WorkerCores::new(1));
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);

        let Err(f) = post_on(&cores, move || flag.store(true, Ordering::Relaxed)) else {
            panic!("a thread with no task of its own was handed a core to place on");
        };
        assert!(
            !ran.load(Ordering::Relaxed),
            "the function ran before its caller had decided where to run it"
        );
        f();
        assert!(
            ran.load(Ordering::Relaxed),
            "what came back was not the function that was handed in"
        );
        assert_eq!(
            lock(&cores).started(),
            0,
            "a core was started for a placement that was never made"
        );
    }
}
