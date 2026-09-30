//! Readiness for one core: `mio` underneath, [`TaskId`] on top.
//!
//! `rule:concurrency/the-reactor-reports-readiness`
//! is this module's specification — § 1 for why the mechanism is readiness on
//! all three platforms and why a poller is not the async runtime the goal's
//! standing decisions forbid, § 2 for the five-rule parking contract this file
//! implements. What follows is only what a reader of *this* code needs.
//!
//! # The contract, as it appears here
//!
//! A task registers a descriptor under its own [`TaskId`] and *then* yields
//! [`Waiting::Parked`](crate::Waiting) — [`Reactor::register`] before
//! [`crate::suspend`], never the other way round, because readiness arriving in
//! the gap would have nothing to be recorded against. When readiness comes back
//! the reactor calls [`Scheduler::wake`], and that is a **hint**: the task
//! re-tries its syscall and parks again on another `WouldBlock`. A woken id
//! that names no parked task is ordinary, not a fault — a level-triggered
//! report of a socket another read already drained, or a wake racing a task
//! that ended — so [`Scheduler::wake`] answering `false` is a normal outcome
//! here and is counted, not raised.
//!
//! [`run_until_idle`] is rule 4: with work on the run queue the reactor is
//! polled with a zero timeout, so readiness is collected without giving the
//! core up; with nothing ready and something parked it blocks; with nothing
//! ready and nothing parked there is no work at all and it returns. There is no
//! fourth state. A deadline does not add one — it only bounds how long the
//! blocking state blocks, and a task parked on nothing but a timer is a task
//! this reactor can still wake ([`crate::timer`]).
//!
//! # Who owns a registration, and what `retire` frees
//!
//! Two halves, and confusing them is how rule 3 gets read as impossible. The
//! **kernel-side** registration is owned by the descriptor: closing it removes
//! it, and the descriptor lives on the task's own stack, so a task that ends
//! takes its registration with it (`net`'s stream deregisters explicitly in
//! `Drop` as well, for the case where it outlives the wait). The
//! **reactor-side** half is the table below, keyed by `TaskId`, and that is
//! what [`Reactor::retire`] frees when the scheduler hands back a
//! [`Finished`](crate::Finished). Leaving it would be a per-request entry that
//! outlives its request — the O(requests served) growth
//! `rule:programs/memory-priority` calls a leak
//! rather than a trade-off — which is why [`run_until_idle`] retires on every
//! turn rather than at the end.
//!
//! What this module spends, per ADR 0004: one [`mio::Poll`] and one
//! [`EVENT_CAPACITY`]-entry event buffer **per worker**, plus one small table
//! entry per task with a live registration. Both are O(cores) and O(in-flight);
//! neither grows with requests served.
//!
//! # How a task reaches this reactor
//!
//! **Through a thread-local, and that is the only route.** `NvsTcp::read` is a
//! plain [`std::io::Read`]: it is handed a buffer and nothing else, so the core
//! it belongs to has to be reachable from inside a free function.
//! [`install`] puts a reactor on the running thread for as long as its guard
//! lives — a worker does that once, around everything it runs — and
//! [`with_current`] is how a task borrows it. [`run_until_idle`] reaches it the
//! same way rather than taking one by reference, and that is not tidiness: a
//! `&mut Reactor` held across [`Scheduler::run`] is exactly the borrow no task
//! running *inside* that call could ever get through, so one route or the other
//! had to go.
//!
//! The alternative considered was a second opaque pointer in `Ctx`, beside the
//! yielder. Two reasons it lost, neither of them style. A `std::io::Read` has no
//! `Ctx` either, so the stream would have had to store one — a raw pointer into
//! its request kept valid by convention, which is a new invariant for every
//! future contributor rather than a borrow the compiler checks. And a reactor is
//! per *core* while a `Ctx` is per *request*: putting it there is one copy of
//! the core's identity per in-flight request, written on every spawn, to say
//! something that is true of the whole thread. The yielder is in `Ctx` because
//! it genuinely is per-task; this is not.
//!
//! *Which* task is asking is the other half of the route and it lives in
//! [`crate::scheduler`]: [`current_task`](crate::scheduler::current_task) is the
//! running task's [`TaskId`] and
//! [`suspend_current`](crate::scheduler::suspend_current) parks it with no `Ctx`
//! to travel in. Both are maintained by the task's own stack across a switch,
//! never by bookkeeping in [`Scheduler::run`]. A stream's park is therefore
//! three ordinary calls — [`with_current`] to register, `suspend_current` to
//! park, and rule 2's retry — with nothing threaded through `std::io`.
//!
//! **A borrow of the installed reactor never spans a suspend point.** That is
//! rule 1's ordering read as a borrow rule: register, drop the borrow, *then*
//! yield. A borrow left outstanding across a stack switch would still be held
//! while the scheduler polled, and [`with_current`] answers that with a panic
//! rather than with silent aliasing. Nothing in this crate holds one across
//! anything but a single call.
//!
//! # A wake that comes from another thread
//!
//! Everything above is readiness, and readiness is something the kernel already
//! knows about. A call with no readiness to wait on — a filesystem call, a name
//! resolution, a wait on a child process — goes to the blocking pool
//! `rule:http-server/a-core-is-never-blocked-on-a-syscall`
//! requires, and the thread that finishes it has to be able to say so to a
//! core that is asleep inside `Poll::poll`. [`Reactor::remote_wake`] is that:
//! it hands out a [`RemoteWake`], the one `Send` thing in this crate, and the
//! far side wakes it when the answer is ready — or drops it, which wakes the
//! task just the same, because a pool thread that panicked must not leave a
//! task parked on an answer that is not coming.
//!
//! **A handle its own task gives up wakes nothing.** The drop stands in for a
//! wake because the task may be parked on the far side's answer, and a task
//! that drops the handle itself is running, so there is no park to end. That
//! drop is settled on the spot — the count comes down and nothing is queued.
//! A queued wake is collected only once the task has parked again, so it would
//! end *that* wait instead, and a loop that takes a handle, waits and lets it
//! go would never wait at all.
//! `rule:concurrency/a-handle-given-up-by-its-task-wakes-nothing`.
//!
//! **The wake is a queue plus one poke, not a token per waiter.** `mio` allows
//! exactly one [`Waker`] per `Poll`, so [`WAKE_TOKEN`] is the single token here
//! that is not a [`TaskId`], and the ids themselves travel in a mutex-guarded
//! `Vec` the core drains when that token comes back ready. One poke can stand
//! for several ids and several pokes for one; both are fine, because the queue
//! is the record and the poke only ends the wait.
//!
//! The part worth reading twice is rule 4's third state. A task waiting on the
//! pool has **no registration and possibly no deadline**, so the "nothing here
//! can wake anything, return rather than block forever" test would have
//! abandoned it. [`Reactor::remote_waits`] is the third thing that test asks
//! about, and it is a count both of whose ends are on the core — up when a
//! handle is issued, down when its id is drained or its own task gives it up —
//! so it can never read zero with a wake in flight. That ordering is the whole
//! correctness argument; [`Remote::outstanding`] holds it in full.
//!
//! # One reactor per worker
//!
//! Rule 5, and it is the same rule [`Scheduler`] holds: a [`Reactor`] is
//! `!Send` and `!Sync`, so a descriptor a request owns is registered with, and
//! woken by, that request's own core. Nothing crosses, which is what keeps the
//! refcounts in `nvs-runtime` non-atomic.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use mio::{Events, Poll, Token, Waker};
use nvs_runtime::{Drain, DrainWake};

use crate::scheduler::{RunReport, Scheduler, TaskId};
use crate::timer::{DeadlineView, Timers};

pub use mio::Interest;
pub use mio::event::Source;

/// How many readiness events one poll collects before the rest wait for the
/// next one.
///
/// A fixed per-worker buffer rather than one sized by the number of
/// registrations: the events left behind are reported again by the very next
/// poll, so the only cost of a small number is an extra syscall under a burst,
/// and the alternative is a buffer that grows with concurrency.
pub const EVENT_CAPACITY: usize = 256;

/// The one token in this reactor that is not a [`TaskId`].
///
/// A cross-thread wake arrives through `mio`'s own waker, which needs a token
/// of its own; every other token here *is* the raw id of the task it belongs
/// to, which is what saves the reactor a second table. `usize::MAX` is that
/// token because ids are issued from zero upwards by one scheduler, so reaching
/// it needs `usize::MAX` tasks on one core — and [`Reactor::poll`] already
/// treats an id that large as naming nothing rather than naming a stranger.
const WAKE_TOKEN: Token = Token(usize::MAX);

/// The cross-thread half of one reactor: what a thread that is *not* this core
/// touches when it has finished work a task here is parked on.
///
/// Behind an `Arc` because a [`RemoteWake`] outlives the call that issued it by
/// definition — it is handed to another thread — and may outlive the reactor
/// itself. A wake delivered after the reactor is gone pushes an id onto a queue
/// nobody drains and pokes a waker nobody polls, which is inert rather than
/// unsound: the same lateness rule 2 already allows for readiness.
#[derive(Debug)]
struct Remote {
    /// `mio`'s cross-thread poke. Waking it makes the very poll
    /// [`Reactor::turn`] is blocked in return with [`WAKE_TOKEN`] ready.
    waker: Waker,
    /// The ids waiting to be woken: pushed off the core, drained on it.
    ///
    /// A `Mutex<Vec<_>>` rather than a channel. This is the only shared mutable
    /// state in the crate, the lock is held for a push or a `Vec::append` and
    /// never across a syscall, and a channel would be a second mechanism saying
    /// the same thing with a second allocation per wake. Contention is bounded
    /// by the size of the pool, which `rule:http-server/a-core-is-never-blocked-on-a-syscall` bounds at twice the core
    /// count.
    pending: Mutex<Vec<TaskId>>,
    /// How many [`RemoteWake`] handles this core has issued and not yet
    /// collected.
    ///
    /// **Both ends of this count are on the core**: it goes up when a handle is
    /// made, and down when the id it queued is drained or when the task it
    /// names gives it up unused, so it never reads zero while a wake is still in
    /// flight. That is what makes it safe for
    /// [`Reactor::turn`] to read as *something off this core can still wake a
    /// task*. A count the waking thread decremented would have a window between
    /// its decrement and its push in which the core concluded the opposite and
    /// abandoned the task — which is the bug this field exists to make
    /// impossible, not a race to be tuned away with an ordering.
    ///
    /// It is an atomic because `Remote` has to be `Sync` to cross a thread
    /// boundary at all, not because two threads write it; `Relaxed` is enough
    /// for the same reason.
    outstanding: AtomicUsize,
}

impl Remote {
    /// Takes `handles` off [`Remote::outstanding`]. Called on the core and
    /// nowhere else.
    ///
    /// A `saturating_sub` over a plain load and store rather than a `fetch_sub`
    /// because no other thread writes the count, and an underflow would be a
    /// wrap to `usize::MAX`, which reads as *a wake is coming forever*.
    fn collected(&self, handles: usize) {
        let outstanding = self.outstanding.load(Ordering::Relaxed);
        self.outstanding
            .store(outstanding.saturating_sub(handles), Ordering::Relaxed);
    }
}

/// A one-shot permission to wake one task from another thread.
///
/// Issued on the core by [`Reactor::remote_wake`] **before** the task parks —
/// rule 1's ordering, and for rule 1's reason: a wake arriving in the gap
/// between a park and its arrangement would have nothing to be recorded
/// against. Then it is moved to whatever thread is doing the work, which is the
/// blocking pool `rule:http-server/a-core-is-never-blocked-on-a-syscall`
/// sends a filesystem call, a name resolution or a wait on a child process
/// to. This is the only thing in this crate that is `Send`, and it carries no
/// reference to the scheduler, the reactor or the task's stack — just the id
/// and the poke.
///
/// **Dropping it wakes the task too.** A handle dropped without
/// [`RemoteWake::wake`] means the far side gave up, panicked, or was torn down,
/// and in every one of those cases the task it names is parked on an answer
/// that is never coming. Waking it turns that into a retry that observes the
/// failure, which is tier B containment rather than a wedged core. The delivery
/// is idempotent: `wake` and the drop that follows it deliver once between them.
///
/// **Unless the task it names is the one dropping it.** That task is running,
/// so it is parked on nothing, and a wake queued now would wait until the task
/// had parked again and end that wait instead. The handle is settled on the
/// spot — [`Remote::outstanding`] comes down and nothing is queued or poked —
/// which is what lets a task take one for a single wait and let it go when the
/// wait ends for its own reason. [`RemoteWake::wake`] does not ask who calls
/// it: a wake that is asked for is always queued.
#[derive(Debug)]
pub struct RemoteWake {
    remote: Arc<Remote>,
    id: TaskId,
    delivered: bool,
}

impl RemoteWake {
    /// Which task this wakes.
    #[must_use]
    pub fn task(&self) -> TaskId {
        self.id
    }

    /// Wakes the task, ending the poll its core is blocked in.
    ///
    /// Like every other wake here this is a **hint** — the task re-tries
    /// whatever it parked on and parks again if the answer is still not there.
    ///
    /// # Errors
    ///
    /// Whatever the platform reported for the poke itself: the write to an
    /// `eventfd`, the `EVFILT_USER` trigger, or the post to the completion
    /// port. The id is queued before the poke is attempted, so a failure here
    /// loses the wake's *promptness*, not the wake: the next poke from any
    /// handle on this core collects it.
    pub fn wake(mut self) -> io::Result<()> {
        self.deliver()
    }

    /// The delivery both [`RemoteWake::wake`] and the drop go through.
    fn deliver(&mut self) -> io::Result<()> {
        if std::mem::replace(&mut self.delivered, true) {
            return Ok(());
        }
        // The id goes into the queue before the poke, never after. A poke that
        // arrived first would end the core's poll, find an empty queue, and
        // leave the task parked until something else happened to wake it.
        self.remote
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(self.id);
        self.remote.waker.wake()
    }

    /// Whether the thread dropping this handle is the core that issued it,
    /// running the task it names.
    ///
    /// Both halves are asked, because an id is issued per scheduler and so
    /// names a different task on every core. A reactor that is borrowed, gone,
    /// or another core's answers `false`, which leaves the drop to queue its
    /// wake as any other does.
    fn is_held_by_its_running_task(&self) -> bool {
        crate::scheduler::current_task() == Some(self.id)
            && INSTALLED
                .try_with(|slot| {
                    slot.try_borrow().is_ok_and(|reactor| {
                        reactor
                            .as_ref()
                            .is_some_and(|reactor| Arc::ptr_eq(&reactor.remote, &self.remote))
                    })
                })
                .unwrap_or(false)
    }
}

impl Drop for RemoteWake {
    fn drop(&mut self) {
        if !self.delivered && self.is_held_by_its_running_task() {
            // Nothing is parked on this handle, so there is no wake for the
            // drop to stand in for. It is collected here, on the core, the way
            // a drained id is.
            self.delivered = true;
            self.remote.collected(1);
            return;
        }
        // A failed poke is not reportable from a drop and must not panic out of
        // one: the thread dropping this handle is quite possibly already
        // unwinding, and a panic there is the abort `rule:http-server/containment-does-not-end-at-the-helper` spent a
        // containment boundary to avoid. The task stays parked until its
        // deadline, which is the bound `crate::timer` puts under every wait.
        let _ = self.deliver();
    }
}

/// The readiness source of one core.
///
/// Created on the thread that polls it and never moved off it — `!Send` and
/// `!Sync` for the same structural reason [`Scheduler`] is, which this module's
/// docs state as rule 5.
pub struct Reactor {
    poll: Poll,
    events: Events,
    /// How many live registrations each task holds. A count rather than a set
    /// because one task may wait on more than one descriptor, and dropping the
    /// entry on the first `deregister` would lose the others.
    registrations: HashMap<TaskId, u32>,
    /// The ids the last poll reported, reused across calls so a poll allocates
    /// nothing on the steady path.
    ready: Vec<TaskId>,
    /// The deadlines this core is holding. Here rather than beside the
    /// scheduler because a deadline is enforced as a poll timeout, which is
    /// this type's call to make; [`crate::timer`] owns the rest of the reason.
    timers: Timers,
    /// The cross-thread wake queue and the waker that announces it. Shared with
    /// every [`RemoteWake`] this reactor has issued, which is the only reason
    /// any of it is behind an `Arc`.
    remote: Arc<Remote>,
    _pinned_to_one_thread: PhantomData<*const ()>,
}

impl std::fmt::Debug for Reactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Neither the poll handle nor the event buffer says anything a debug
        // line can use, and both would print differently between runs.
        f.debug_struct("Reactor")
            .field("registrations", &self.registrations.len())
            .field("ready", &self.ready.len())
            .field("timers", &self.timers.len())
            .field("remote_waits", &self.remote_waits())
            .finish()
    }
}

impl Reactor {
    /// Opens this core's readiness source.
    ///
    /// # Errors
    ///
    /// Whatever the platform reported for `epoll_create`, `kqueue` or the
    /// completion port — the process is out of descriptors or handles. Or the
    /// same for the waker beneath it, which is one more descriptor on Linux and
    /// none at all on the platforms that have a user event filter.
    pub fn new() -> io::Result<Self> {
        let poll = Poll::new()?;
        // The waker is created with the reactor and not on first use: it is one
        // descriptor per core, it cannot be added to a `Poll` that is already
        // being polled by a parked worker, and a fallible `remote_wake` would
        // put an error path on the handoff that nothing sensible could do with.
        let waker = Waker::new(poll.registry(), WAKE_TOKEN)?;
        Ok(Self {
            poll,
            events: Events::with_capacity(EVENT_CAPACITY),
            registrations: HashMap::new(),
            ready: Vec::new(),
            timers: Timers::default(),
            remote: Arc::new(Remote {
                waker,
                pending: Mutex::new(Vec::new()),
                outstanding: AtomicUsize::new(0),
            }),
            _pinned_to_one_thread: PhantomData,
        })
    }

    /// Registers `source` so that readiness on it wakes `id`.
    ///
    /// Call this **before** parking the task, never after: see this module's
    /// docs, and `rule:concurrency/the-parking-contract` rule 1.
    ///
    /// # Errors
    ///
    /// The platform refused the registration — most often because the same
    /// descriptor is already registered, which is [`Reactor::reregister`]'s
    /// job. `InvalidInput` for an id too large to be a token, which needs
    /// `usize::MAX` tasks on a 32-bit host to reach.
    pub fn register<S>(&mut self, source: &mut S, id: TaskId, interest: Interest) -> io::Result<()>
    where
        S: Source + ?Sized,
    {
        self.poll
            .registry()
            .register(source, token_of(id)?, interest)?;
        *self.registrations.entry(id).or_insert(0) += 1;
        Ok(())
    }

    /// Changes what an already-registered `source` wakes `id` for.
    ///
    /// The registration count does not move: this replaces an interest set, it
    /// does not add a descriptor.
    ///
    /// # Errors
    ///
    /// As [`Reactor::register`], plus the platform's answer for a source that
    /// was never registered.
    pub fn reregister<S>(
        &mut self,
        source: &mut S,
        id: TaskId,
        interest: Interest,
    ) -> io::Result<()>
    where
        S: Source + ?Sized,
    {
        self.poll
            .registry()
            .reregister(source, token_of(id)?, interest)
    }

    /// Removes `source` from this reactor and drops one of `id`'s
    /// registrations.
    ///
    /// Only needed for a descriptor that outlives the wait it was registered
    /// for; one that is simply dropped takes its kernel-side registration with
    /// it, and [`Reactor::retire`] frees the table entry.
    ///
    /// # Errors
    ///
    /// The platform's answer for a source it does not hold.
    pub fn deregister<S>(&mut self, source: &mut S, id: TaskId) -> io::Result<()>
    where
        S: Source + ?Sized,
    {
        self.poll.registry().deregister(source)?;
        if let Some(count) = self.registrations.get_mut(&id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.registrations.remove(&id);
            }
        }
        Ok(())
    }

    /// Drops every registration this reactor holds for a task that has ended,
    /// reporting whether there was one.
    ///
    /// `rule:concurrency/the-parking-contract` rule 3, and the caller is whoever takes a
    /// [`Finished`](crate::Finished) — [`run_until_idle`] does it on every
    /// turn. A later wake for a retired id is already harmless; the table entry
    /// is what would otherwise outlive its request.
    pub fn retire(&mut self, id: TaskId) -> bool {
        // A deadline the task never reached goes with it, and for the same
        // reason: an entry outliving its request is the growth `rule:programs/memory-priority` calls a
        // leak. `crate::timer` is the home of that argument.
        let had_timer = self.timers.disarm(id);
        self.registrations.remove(&id).is_some() || had_timer
    }

    /// The deadlines this core is holding.
    ///
    /// The reactor owns them because a deadline is enforced as the timeout of
    /// the very poll [`Reactor::turn`] is about to make; [`crate::timer`] is
    /// where they are armed from and where the rest of the reasoning lives.
    pub fn timers(&mut self) -> &mut Timers {
        &mut self.timers
    }

    /// A handle another thread may read this core's earliest deadline through.
    ///
    /// The watchdog's end of `rule:http-server/a-wedged-core-is-detected-by-its-deadline`, taken here rather than from the
    /// [`Timers`] directly because the reactor is what a core has a handle on:
    /// a worker takes its own view once, on its own thread, and hands the copy
    /// out. [`crate::timer`]'s docs own what a reading of it means and why it
    /// costs the core nothing.
    #[must_use]
    pub fn deadline_view(&self) -> DeadlineView {
        self.timers.view()
    }

    /// How many tasks hold at least one registration.
    #[must_use]
    pub fn registrations(&self) -> usize {
        self.registrations.len()
    }

    /// Whether `id` has anything registered that could wake it.
    #[must_use]
    pub fn is_registered(&self, id: TaskId) -> bool {
        self.registrations.contains_key(&id)
    }

    /// Issues a one-shot handle that wakes `id` from another thread.
    ///
    /// Take it **before** parking the task, exactly as a registration is taken
    /// before a park, and for the same reason — see [`RemoteWake`], which owns
    /// the rest of the contract including what a dropped handle does.
    ///
    /// While a handle is outstanding this core has something that can wake a
    /// task even with no descriptor registered and no deadline filed, which is
    /// precisely the state a task waiting on the blocking pool is in.
    /// [`Reactor::turn`] reads [`Reactor::remote_waits`] for that and blocks
    /// rather than reporting there is nothing left to wait for.
    pub fn remote_wake(&self, id: TaskId) -> RemoteWake {
        self.remote.outstanding.fetch_add(1, Ordering::Relaxed);
        RemoteWake {
            remote: Arc::clone(&self.remote),
            id,
            delivered: false,
        }
    }

    /// How many cross-thread wakes this core has issued and not yet collected.
    #[must_use]
    pub fn remote_waits(&self) -> usize {
        self.remote.outstanding.load(Ordering::Relaxed)
    }

    /// Moves whatever another thread has queued into this poll's ready list.
    ///
    /// Called for [`WAKE_TOKEN`] and nothing else. The count comes down by
    /// exactly what was taken, here on the core, which is the invariant
    /// [`Remote::outstanding`] documents.
    fn drain_remote(&mut self) {
        let mut pending = self
            .remote
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let taken = pending.len();
        self.ready.append(&mut pending);
        drop(pending);
        self.remote.collected(taken);
    }

    /// Waits for readiness and reports the tasks it names.
    ///
    /// `None` blocks until something is ready; `Some(Duration::ZERO)` collects
    /// whatever has already arrived and returns at once. The slice is
    /// deduplicated and lives until the next poll.
    ///
    /// A signal interrupting the wait reports no readiness rather than an
    /// error: it is the same "nothing yet" a timeout reports, and a caller that
    /// treated it as a failure would tear down a request for a `SIGWINCH`.
    ///
    /// # Errors
    ///
    /// Whatever the platform reported for `epoll_wait`, `kevent` or the
    /// completion port, `Interrupted` excepted.
    pub fn poll(&mut self, timeout: Option<Duration>) -> io::Result<&[TaskId]> {
        self.ready.clear();
        match self.poll.poll(&mut self.events, timeout) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::Interrupted => return Ok(&self.ready),
            Err(err) => return Err(err),
        }
        let mut remote = false;
        for event in self.events.iter() {
            if event.token() == WAKE_TOKEN {
                // A cross-thread wake. The token says only "the queue moved" —
                // one poke can stand for several ids and several pokes for one,
                // so what was woken is read out of the queue below rather than
                // off the event.
                remote = true;
                continue;
            }
            // A `usize` wider than a `u64` exists on no target this builds
            // for; `u64::MAX` is never an issued id, so a hypothetical one
            // would wake nothing rather than wake a stranger.
            let raw = u64::try_from(event.token().0).unwrap_or(u64::MAX);
            self.ready.push(TaskId::from_raw(raw));
        }
        if remote {
            // Outside the loop because it borrows `self` whole, and before the
            // dedup below because a remote wake for a task that readiness also
            // named is one wake and not two.
            self.drain_remote();
        }
        // One task waiting on two descriptors is reported twice, and a second
        // `wake` for the same id is a no-op that would still be counted as
        // work. Sorting rather than preserving arrival order is deliberate:
        // within one batch there is no fairness to preserve, and the run queue
        // is FIFO from there on.
        self.ready.sort_unstable();
        self.ready.dedup();
        Ok(&self.ready)
    }

    /// Collects readiness once and wakes what it names, reporting how many
    /// tasks moved back to the run queue.
    ///
    /// This is `rule:concurrency/the-parking-contract` rule 4's three states in one place: it polls with a
    /// zero timeout while `sched` has work ready, blocks when nothing is ready
    /// and something parked can still be woken, and returns `0` without a
    /// syscall when there is nothing to wait for — including the case where
    /// tasks are parked but this reactor holds no registration, no deadline and
    /// no outstanding cross-thread wake that could ever wake one, which is a
    /// bug in the caller rather than a reason to block forever.
    ///
    /// The blocking state is bounded by the earliest deadline filed, and every
    /// timer due when the poll comes back is woken alongside the readiness it
    /// reported.
    ///
    /// # Errors
    ///
    /// As [`Reactor::poll`].
    pub fn turn(&mut self, sched: &mut Scheduler) -> io::Result<usize> {
        loop {
            let timeout = if sched.ready_count() > 0 {
                Some(Duration::ZERO)
            } else if sched.parked_count() > 0
                && !(self.registrations.is_empty()
                    && self.timers.is_empty()
                    && self.remote_waits() == 0)
            {
                // A filed deadline bounds the wait; with none, only readiness
                // or a cross-thread wake can end it. Either way this is the
                // blocking state rule 4 names, and the deadline only says how
                // long it lasts. An outstanding `RemoteWake` counts as
                // something that can end the wait even though this core holds
                // no descriptor for it — that is the whole state a task waiting
                // on the blocking pool is in.
                self.timers
                    .next_due()
                    .map(|at| at.saturating_duration_since(Instant::now()))
            } else {
                return Ok(0);
            };

            let mut woken = 0;
            for &id in self.poll(timeout)? {
                // `false` is ordinary — rule 2's stale or racing wake — so it
                // is counted out rather than reported.
                if sched.wake(id) {
                    woken += 1;
                }
            }
            // The clock is read after the poll and not before it: a poll that
            // waited longer than it was asked to has more than one timer due,
            // and this wakes all of them rather than one per turn.
            let now = Instant::now();
            while let Some(id) = self.timers.take_due(now) {
                if sched.wake(id) {
                    woken += 1;
                }
            }
            let can_still_wake = !self.timers.is_empty() || self.remote_waits() > 0;
            if woken > 0 || timeout == Some(Duration::ZERO) || !can_still_wake {
                return Ok(woken);
            }
            // A wait that came back with nothing to show for it, and something
            // that can still end it. Returning `0` here would tell the caller
            // that nothing can wake these tasks any more, which is exactly the
            // task-abandoned bug, so the wait is retried instead. The retry
            // test is deliberately the *same* one the blocking state above is
            // entered on: a turn that would block for a reason may not then
            // report that reason gone.
            //
            // Both halves are ordinary rather than exceptional. A platform
            // rounds a wait to its own timer granularity and can return a
            // fraction of a millisecond before the earliest deadline. And a
            // poke outlives the drain it belongs to: one drain takes the ids of
            // several pokes at once (the paragraph above `WAKE_TOKEN`'s
            // handling in `poll`), so the surplus pokes come back ready over an
            // empty queue — with the pool saturated, that is the common case
            // and not a rare one.
            //
            // The retry is not free if readiness is also arriving for a task
            // that no longer parks; it is bounded by the earliest deadline, and
            // abandoning a parked task is much the worse of the two.
        }
    }
}

thread_local! {
    /// This thread's reactor, for as long as an [`Installed`] guard holds it.
    ///
    /// A `RefCell` and not a `Cell<*mut _>`: the borrow is what makes the "never
    /// across a suspend point" rule in this module's docs an enforced one rather
    /// than a remembered one, and it costs a flag check against a call that is
    /// about to make a syscall.
    static INSTALLED: RefCell<Option<Reactor>> = const { RefCell::new(None) };
}

/// Installs `reactor` as this thread's reactor until the returned guard is
/// dropped.
///
/// A worker calls this once, around everything it runs, so that every task on
/// the core reaches the same reactor from a free function — see this module's
/// docs for why that is the route and not a pointer in `Ctx`. A test that wants
/// a core of its own calls it too; the thread-local is per thread, so tests
/// running in parallel do not see each other's.
///
/// # Panics
///
/// If this thread already has one installed. Two reactors on one core is not a
/// configuration with a sensible meaning — half the tasks would register with a
/// poll nothing polls — so it is refused loudly at the point of the mistake
/// rather than diagnosed later as a task that never wakes.
#[must_use = "the reactor is uninstalled when the guard is dropped"]
pub fn install(reactor: Reactor) -> Installed {
    INSTALLED.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "this thread already has a reactor installed"
        );
        *slot = Some(reactor);
    });
    Installed(PhantomData)
}

/// Holds this thread's reactor installed; uninstalls it when dropped.
///
/// `!Send` like everything else on a core, which is rule 5 expressed in the
/// type: a guard cannot be moved to a thread whose reactor it does not name.
pub struct Installed(PhantomData<*const ()>);

impl std::fmt::Debug for Installed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Installed")
    }
}

impl Drop for Installed {
    fn drop(&mut self) {
        // Dropping the reactor closes the poll and with it every kernel-side
        // registration still under it, which is the same freeing a closed
        // descriptor does — the table half goes with the map.
        INSTALLED.with_borrow_mut(|slot| *slot = None);
    }
}

/// Runs `f` against this thread's reactor, or answers `None` if there is none.
///
/// `None` is the same refusal [`crate::suspend`] answers with `false`: there is
/// no core beneath this call — a `Core` member reached from `nvs run`, or a unit
/// test — and **the caller must then do something else**, block or fail, rather
/// than treat a missing reactor as a registration that happened.
///
/// # Panics
///
/// If a borrow is already outstanding on this thread, which means one was held
/// across a suspend point or `f` re-entered. Both are bugs in this crate rather
/// than anything a request can drive, and this module's docs state the rule they
/// break.
pub fn with_current<T>(f: impl FnOnce(&mut Reactor) -> T) -> Option<T> {
    INSTALLED.with_borrow_mut(|slot| slot.as_mut().map(f))
}

/// Whether this thread has a reactor installed.
#[must_use]
pub fn is_installed() -> bool {
    INSTALLED.with_borrow(Option::is_some)
}

/// Wakes the task this call is on when `drain` begins, for a wait that is about
/// to park on something the drain itself will never make ready.
///
/// [`nvs_runtime::Drain`] holds the bit and the list of what is owed a wake, and
/// says that the closure has to come from a host because the hop it makes is
/// cross-thread: the drain is begun by whichever thread took the signal, and
/// the waits it owes are parked on every core. This is that closure — the same
/// [`RemoteWake`] the blocking pool uses, issued against the current task and
/// fired once.
///
/// Held for as long as the wait is, and **a wake is a hint**: the caller reads
/// [`nvs_runtime::Drain::is_draining`] when it resumes rather than treating the
/// resume as the answer (this module's docs, rule 2). `None` is either a call
/// with no task or reactor beneath it, or a drain that has **already** begun —
/// in which case the wake has fired before this returns and the caller's next
/// read of the bit is what tells it so.
///
/// Dropping the handle deregisters and wakes nothing: the task that held it
/// across its wait is the one letting it go, and a [`RemoteWake`] its own task
/// gives up is collected without a wake. A wait that ends for its own reason
/// therefore costs the core no poke, and a task may register afresh for every
/// wait.
pub fn wake_at_drain(drain: &Drain) -> Option<DrainWake> {
    let id = crate::scheduler::current_task()?;
    let wake = with_current(|reactor| reactor.remote_wake(id))?;
    drain.wake_at_drain(move || drop(wake.wake()))
}

/// Runs `sched` under this thread's reactor until neither has anything left to
/// do.
///
/// The loop is the whole of this crate's I/O story: [`Scheduler::run`] returns
/// when the run queue empties, every task it finished is retired from the
/// reactor's table, and a non-zero parked count is the "block in the reactor
/// now" test. This is what a worker's body is, and `Worker::spawn` hands the
/// scheduler to a closure precisely so that closure can be this.
///
/// The reactor is not a parameter: it is borrowed out of the thread-local
/// [`install`] put it in, once per turn and never across [`Scheduler::run`],
/// because a task suspending inside that call takes the same borrow for its own
/// registration. This module's docs own that reasoning.
///
/// It returns with `RunReport::parked` non-zero in exactly one case: a blocking
/// poll came back having woken nothing, which means the readiness it collected
/// belongs to no parked task. Continuing would spin on a level-triggered report
/// nobody owns, so the report says the state instead and the caller decides.
///
/// # Errors
///
/// As [`Reactor::poll`]. A failed poll leaves `sched` untouched and its tasks
/// parked; the counts already accumulated are lost with the error, which is the
/// right trade for a call that cannot continue. Plus one of its own: no reactor
/// is installed on this thread, which is a worker that never called
/// [`install`] and would otherwise park its first task forever.
pub fn run_until_idle(sched: &mut Scheduler) -> io::Result<RunReport> {
    let mut total = RunReport::default();
    loop {
        let turn = sched.run();
        total.resumes += turn.resumes;
        total.finished += turn.finished;
        total.cancelled += turn.cancelled;
        // The borrow opens here and closes here. It deliberately does not span
        // the `sched.run()` above, because a task suspending inside that call
        // takes the same borrow to register what it is waiting for.
        let woken = with_current(|reactor| {
            // Rule 3, on every turn rather than at the end: a task that ended
            // is one whose registrations must not outlive it. One list and one
            // drain for both ways of ending — a task torn down for a
            // cancellation owes this more urgently than one that returned,
            // since it was parked on a registration when it died and this is
            // the only place that registration is ever dropped.
            for id in sched.take_ended() {
                reactor.retire(id);
            }
            if sched.parked_count() == 0 {
                return Ok(None);
            }
            reactor.turn(sched).map(Some)
        })
        .ok_or_else(|| io::Error::other("no reactor is installed on this thread"))??;
        match woken {
            None => break,
            Some(0) if sched.ready_count() == 0 => break,
            Some(_) => {}
        }
    }
    total.parked = sched.parked_count();
    Ok(total)
}

/// A task's id as a `mio` token.
///
/// The token *is* the id, so the reverse direction needs no table — which is
/// the whole reason the reactor's own bookkeeping is one small map and not two.
fn token_of(id: TaskId) -> io::Result<Token> {
    usize::try_from(id.raw()).map(Token).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "task id does not fit in a readiness token on this target",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{Waiting, current_task, suspend, suspend_current};
    use nvs_runtime::{Ctx, OutputSink, TaskRoot};
    use std::cell::Cell;
    use std::io::Write;
    use std::rc::Rc;
    use std::time::Instant;

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    /// A connected pair on the loopback: the `mio` half is what gets
    /// registered, the `std` half is what the test writes to in order to make
    /// it readable.
    fn connected_pair() -> (mio::net::TcpStream, std::net::TcpStream) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener has an address");
        let client = std::net::TcpStream::connect(addr).expect("loopback refused a connection");
        let (server, _) = listener.accept().expect("the connection did not arrive");
        server
            .set_nonblocking(true)
            .expect("a socket refused non-blocking mode");
        (mio::net::TcpStream::from_std(server), client)
    }

    #[test]
    fn a_poll_with_nothing_registered_reports_no_readiness() {
        let mut reactor = Reactor::new().expect("the OS refused a poll");
        assert_eq!(reactor.registrations(), 0);
        assert!(
            reactor
                .poll(Some(Duration::ZERO))
                .expect("a zero-timeout poll failed")
                .is_empty()
        );
    }

    #[test]
    fn a_registered_socket_reports_the_task_that_parked_on_it() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let mut reactor = Reactor::new().expect("the OS refused a poll");

        let id = sched.spawn(ctx(), TaskRoot::Worker, |ctx| {
            suspend(ctx, Waiting::Parked);
        });
        // Rule 1: register, then park. The task has not run yet, so the
        // registration is in place before anything can be ready.
        reactor
            .register(&mut server, id, Interest::READABLE)
            .expect("the OS refused a registration");
        assert!(reactor.is_registered(id));

        sched.run();
        assert_eq!(sched.parked_count(), 1);

        client.write_all(b"hi").expect("the write failed");
        // Bounded rather than blocking: a broken mechanism should fail this
        // test, not hang the suite.
        let ready = reactor
            .poll(Some(Duration::from_secs(5)))
            .expect("the poll failed");
        assert_eq!(ready, [id], "readiness named the wrong task");

        assert!(sched.wake(id));
        let report = sched.run();
        assert_eq!(report.finished, 1);
        assert_eq!(report.parked, 0);
    }

    #[test]
    fn a_task_that_parks_on_a_socket_runs_to_completion_under_the_loop() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let resumes = Rc::new(Cell::new(0_usize));
        let counter = Rc::clone(&resumes);
        let id = sched.spawn(ctx(), TaskRoot::Worker, move |ctx| {
            counter.set(counter.get() + 1);
            assert!(
                suspend(ctx, Waiting::Parked).suspended(),
                "there was no scheduler"
            );
            counter.set(counter.get() + 1);
        });
        with_current(|reactor| reactor.register(&mut server, id, Interest::READABLE))
            .expect("no reactor was installed")
            .expect("the OS refused a registration");
        // Readiness is arranged before the loop is entered, so the blocking
        // poll inside it has something waiting for it already.
        client.write_all(b"hi").expect("the write failed");

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(resumes.get(), 2, "the task did not resume after its park");
        assert_eq!(report.finished, 1);
        assert_eq!(report.parked, 0);
        // Rule 3: the loop retired the registration when the task ended.
        let live = with_current(|reactor| (reactor.registrations(), reactor.is_registered(id)))
            .expect("no reactor was installed");
        assert_eq!(live.0, 0);
        assert!(!live.1);
    }

    #[test]
    fn retiring_a_task_drops_its_registrations_and_says_whether_it_had_any() {
        let (mut server, _client) = connected_pair();
        let mut reactor = Reactor::new().expect("the OS refused a poll");
        let id = TaskId::from_raw(7);

        reactor
            .register(&mut server, id, Interest::READABLE)
            .expect("the OS refused a registration");
        assert_eq!(reactor.registrations(), 1);

        assert!(reactor.retire(id), "a live registration reported none");
        assert_eq!(reactor.registrations(), 0);
        assert!(!reactor.retire(id), "retiring twice claimed to free twice");
    }

    #[test]
    fn deregistering_one_of_two_descriptors_leaves_the_task_registered() {
        let (mut first, _first_peer) = connected_pair();
        let (mut second, _second_peer) = connected_pair();
        let mut reactor = Reactor::new().expect("the OS refused a poll");
        let id = TaskId::from_raw(3);

        reactor
            .register(&mut first, id, Interest::READABLE)
            .expect("the OS refused a registration");
        reactor
            .register(&mut second, id, Interest::READABLE)
            .expect("the OS refused a registration");

        reactor
            .deregister(&mut first, id)
            .expect("the OS refused a deregistration");
        assert!(
            reactor.is_registered(id),
            "the second descriptor's registration was dropped with the first"
        );

        reactor
            .deregister(&mut second, id)
            .expect("the OS refused a deregistration");
        assert!(!reactor.is_registered(id));
    }

    #[test]
    fn readiness_for_a_task_that_already_finished_wakes_nothing() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let mut reactor = Reactor::new().expect("the OS refused a poll");

        let id = sched.spawn(ctx(), TaskRoot::Worker, |_| {});
        reactor
            .register(&mut server, id, Interest::READABLE)
            .expect("the OS refused a registration");
        assert_eq!(sched.run().finished, 1);

        client.write_all(b"late").expect("the write failed");
        // Rule 2: the id is reported, nothing is parked under it, and that is
        // an ordinary outcome rather than a fault.
        assert!(!sched.wake(id));
        assert_eq!(
            reactor.turn(&mut sched).expect("a turn failed"),
            0,
            "a stale readiness claimed to wake a task"
        );
    }

    #[test]
    fn a_turn_does_not_block_while_the_run_queue_has_work() {
        let (mut server, _client) = connected_pair();
        let mut sched = Scheduler::new();
        let mut reactor = Reactor::new().expect("the OS refused a poll");

        let parked = sched.spawn(ctx(), TaskRoot::Worker, |ctx| {
            suspend(ctx, Waiting::Parked);
        });
        reactor
            .register(&mut server, parked, Interest::READABLE)
            .expect("the OS refused a registration");
        sched.run();
        assert_eq!(sched.parked_count(), 1);

        // Nothing will ever make that socket readable, so the only thing
        // keeping this call from blocking forever is rule 4's zero timeout.
        sched.spawn(ctx(), TaskRoot::Worker, |_| {});
        assert_eq!(sched.ready_count(), 1);

        let started = Instant::now();
        assert_eq!(reactor.turn(&mut sched).expect("a turn failed"), 0);
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "the reactor waited while the run queue had work"
        );
    }

    #[test]
    fn a_park_with_nothing_registered_returns_rather_than_waiting_forever() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        sched.spawn(ctx(), TaskRoot::Worker, |ctx| {
            suspend(ctx, Waiting::Parked);
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.parked, 1, "the stuck task should be reported");
        assert_eq!(report.finished, 0);
    }

    #[test]
    fn a_loop_with_no_reactor_installed_is_refused_rather_than_parking_forever() {
        let mut sched = Scheduler::new();
        sched.spawn(ctx(), TaskRoot::Worker, |ctx| {
            suspend(ctx, Waiting::Parked);
        });

        let err = run_until_idle(&mut sched).expect_err("a loop with no reactor answered");
        assert!(
            err.to_string().contains("no reactor is installed"),
            "the refusal did not name what was missing: {err}"
        );
    }

    #[test]
    fn a_thread_without_an_install_has_no_reactor_to_borrow() {
        assert!(!is_installed());
        assert!(with_current(|reactor| reactor.registrations()).is_none());

        let installed = install(Reactor::new().expect("the OS refused a poll"));
        assert!(is_installed());
        assert_eq!(with_current(|reactor| reactor.registrations()), Some(0));

        drop(installed);
        assert!(!is_installed(), "the guard did not uninstall the reactor");
    }

    /// The decision this module's docs record, exercised the way `NvsTcp` does:
    /// the task is handed no reactor and no `Ctx`, and reaches both from free
    /// functions the way a `std::io::Read` has to.
    #[test]
    fn a_task_registers_and_parks_with_neither_a_reactor_nor_a_ctx_in_hand() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let seen = Rc::new(Cell::new(None));
        let reported = Rc::clone(&seen);
        let id = sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            // Rule 1's ordering, and the borrow rule that is the same rule: the
            // borrow is taken for the registration and dropped before the yield.
            let me = current_task().expect("a running task had no id");
            reported.set(Some(me));
            with_current(|reactor| reactor.register(&mut server, me, Interest::READABLE))
                .expect("no reactor was installed")
                .expect("the OS refused a registration");
            assert!(
                suspend_current(Waiting::Parked).suspended(),
                "there was no scheduler"
            );
            assert_eq!(
                current_task(),
                Some(me),
                "the task lost its identity across the switch"
            );
        });
        client.write_all(b"hi").expect("the write failed");

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(seen.get(), Some(id), "the task read a stranger's id");
        assert_eq!(report.finished, 1);
        assert_eq!(report.parked, 0);
    }

    /// The whole cross-thread path, in the shape a blocking-pool handoff has
    /// (`rule:http-server/a-core-is-never-blocked-on-a-syscall`): a task with **no** registration and no deadline parks,
    /// another thread finishes its work and wakes it, and the core comes back
    /// out of a poll it could otherwise only have left by giving up on the
    /// task.
    #[test]
    fn a_wake_from_another_thread_resumes_a_task_with_nothing_registered() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let resumes = Rc::new(Cell::new(0_usize));
        let counter = Rc::clone(&resumes);
        let id = sched.spawn(ctx(), TaskRoot::Worker, move |ctx| {
            counter.set(counter.get() + 1);
            assert!(
                suspend(ctx, Waiting::Parked).suspended(),
                "there was no scheduler"
            );
            counter.set(counter.get() + 1);
        });
        // Rule 1's ordering, for a handoff rather than a registration: the
        // handle exists before the task has run, let alone parked.
        let wake =
            with_current(|reactor| reactor.remote_wake(id)).expect("no reactor was installed");
        assert_eq!(with_current(|reactor| reactor.remote_waits()), Some(1));

        let far_side = std::thread::spawn(move || {
            // Long enough that the core is inside the poll rather than racing
            // it, short enough that a broken mechanism fails the test rather
            // than slowing the suite.
            std::thread::sleep(Duration::from_millis(20));
            wake.wake().expect("the poke failed");
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        far_side.join().expect("the far side panicked");
        assert_eq!(resumes.get(), 2, "the task did not resume after its park");
        assert_eq!(report.finished, 1);
        assert_eq!(
            report.parked, 0,
            "the core gave up on a task it was told to expect"
        );
        assert_eq!(
            with_current(|reactor| reactor.remote_waits()),
            Some(0),
            "the wake was delivered but never collected"
        );
    }

    #[test]
    fn a_remote_wake_dropped_without_firing_still_wakes_the_task() {
        let mut sched = Scheduler::new();
        let mut reactor = Reactor::new().expect("the OS refused a poll");

        let id = sched.spawn(ctx(), TaskRoot::Worker, |ctx| {
            suspend(ctx, Waiting::Parked);
        });
        let wake = reactor.remote_wake(id);
        assert_eq!(wake.task(), id);
        sched.run();
        assert_eq!(sched.parked_count(), 1);
        assert_eq!(reactor.remote_waits(), 1);

        // The far side gave up, panicked, or was torn down with the work
        // undone. Either way the task is parked on an answer that is not
        // coming, so the drop delivers rather than the core waiting forever.
        drop(wake);
        assert_eq!(
            reactor.turn(&mut sched).expect("a turn failed"),
            1,
            "a dropped handle abandoned its task"
        );
        assert_eq!(reactor.remote_waits(), 0);
        assert_eq!(sched.run().finished, 1);
    }

    /// The other drop: the task a handle names lets it go while it is running.
    /// Nothing is parked on it, so nothing is queued, and the park that follows
    /// is one this core has no reason to end.
    #[test]
    fn a_remote_wake_its_own_task_gives_up_wakes_nothing() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let resumed = Rc::new(Cell::new(false));
        let flag = Rc::clone(&resumed);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let me = current_task().expect("a running task had no id");
            let wake =
                with_current(|reactor| reactor.remote_wake(me)).expect("no reactor was installed");
            assert_eq!(with_current(|reactor| reactor.remote_waits()), Some(1));
            drop(wake);
            assert_eq!(
                with_current(|reactor| reactor.remote_waits()),
                Some(0),
                "a handle its task gave up stayed outstanding"
            );
            suspend_current(Waiting::Parked);
            flag.set(true);
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.parked, 1, "the task did not stay parked");
        assert!(
            !resumed.get(),
            "a handle the task gave up itself ended its next park"
        );
    }

    /// The shape a task that looks at something on a period has: a drain wake
    /// taken for one wait and let go when that wait ends on its deadline. Each
    /// wait lasts until its own deadline, because letting the last wake go
    /// queued nothing for the next wait to be ended by.
    #[test]
    fn a_drain_wake_let_go_by_its_task_does_not_end_that_tasks_next_wait() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let drain = Drain::detached();
        let early = Rc::new(Cell::new(0_usize));
        let counted = Rc::clone(&early);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            for _ in 0..3 {
                let at = Instant::now() + Duration::from_millis(20);
                {
                    let _woken = wake_at_drain(&drain).expect("the drain had begun");
                    crate::timer::wait_until(at);
                }
                if Instant::now() < at {
                    counted.set(counted.get() + 1);
                }
            }
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.finished, 1);
        assert_eq!(
            early.get(),
            0,
            "a wait ended before its deadline with no drain begun"
        );
        assert_eq!(
            with_current(|reactor| reactor.remote_waits()),
            Some(0),
            "a drain wake its task let go stayed outstanding"
        );
    }

    /// What the wake is held for: a drain begun on another thread ends the wait
    /// long before its deadline, and the task reads the bit when it resumes.
    #[test]
    fn a_drain_begun_on_another_thread_ends_the_wait_its_wake_is_held_across() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let drain = Drain::detached();
        let far = drain.clone();
        let seen = Rc::new(Cell::new(false));
        let read = Rc::clone(&seen);
        let bound = Duration::from_secs(60);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let _woken = wake_at_drain(&drain).expect("the drain had begun");
            crate::timer::wait_until(Instant::now() + bound);
            read.set(drain.is_draining());
        });

        let far_side = std::thread::spawn(move || {
            // Long enough that the core is inside its poll rather than racing
            // it. A drain begun early is still not lost: the task reads the bit.
            std::thread::sleep(Duration::from_millis(20));
            far.begin();
        });

        let started = Instant::now();
        let report = run_until_idle(&mut sched).expect("the loop failed");
        far_side.join().expect("the far side panicked");
        assert_eq!(report.finished, 1);
        assert!(
            seen.get(),
            "the task resumed without the drain having begun"
        );
        assert!(
            started.elapsed() < bound,
            "the drain did not end the wait before its deadline"
        );
        assert_eq!(
            with_current(|reactor| reactor.remote_waits()),
            Some(0),
            "the fired wake was never collected"
        );
    }

    #[test]
    fn a_remote_wake_for_a_task_that_already_finished_wakes_nothing() {
        let mut sched = Scheduler::new();
        let mut reactor = Reactor::new().expect("the OS refused a poll");

        let id = sched.spawn(ctx(), TaskRoot::Worker, |_| {});
        let wake = reactor.remote_wake(id);
        assert_eq!(sched.run().finished, 1);

        wake.wake().expect("the poke failed");
        // Rule 2 is not a rule about sockets: a wake racing the task that ended
        // is ordinary here too. What must not happen is the count staying up,
        // which would keep every later turn blocking for a wake already spent.
        let ready = reactor
            .poll(Some(Duration::from_secs(5)))
            .expect("the poll failed");
        assert_eq!(ready, [id], "the queue named the wrong task");
        assert!(!sched.wake(id));
        assert_eq!(
            reactor.remote_waits(),
            0,
            "a collected wake stayed outstanding"
        );
    }
}
