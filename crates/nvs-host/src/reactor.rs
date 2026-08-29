//! Readiness for one core: `mio` underneath, [`TaskId`] on top.
//!
//! [ADR 0115](../../../docs/adr/0115-the-reactor-reports-readiness-and-a-stream-that-would-block-parks.md)
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
//! [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md) calls a leak
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
use std::time::{Duration, Instant};

use mio::{Events, Poll, Token};

use crate::scheduler::{RunReport, Scheduler, TaskId};
use crate::timer::Timers;

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
            .finish()
    }
}

impl Reactor {
    /// Opens this core's readiness source.
    ///
    /// # Errors
    ///
    /// Whatever the platform reported for `epoll_create`, `kqueue` or the
    /// completion port — the process is out of descriptors or handles.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            poll: Poll::new()?,
            events: Events::with_capacity(EVENT_CAPACITY),
            registrations: HashMap::new(),
            ready: Vec::new(),
            timers: Timers::default(),
            _pinned_to_one_thread: PhantomData,
        })
    }

    /// Registers `source` so that readiness on it wakes `id`.
    ///
    /// Call this **before** parking the task, never after: see this module's
    /// docs, and ADR 0115 § 2 rule 1.
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
    /// ADR 0115 § 2 rule 3, and the caller is whoever takes a
    /// [`Finished`](crate::Finished) — [`run_until_idle`] does it on every
    /// turn. A later wake for a retired id is already harmless; the table entry
    /// is what would otherwise outlive its request.
    pub fn retire(&mut self, id: TaskId) -> bool {
        // A deadline the task never reached goes with it, and for the same
        // reason: an entry outliving its request is the growth ADR 0004 calls a
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
        for event in self.events.iter() {
            // A `usize` wider than a `u64` exists on no target this builds
            // for; `u64::MAX` is never an issued id, so a hypothetical one
            // would wake nothing rather than wake a stranger.
            let raw = u64::try_from(event.token().0).unwrap_or(u64::MAX);
            self.ready.push(TaskId::from_raw(raw));
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
    /// This is ADR 0115 § 2 rule 4's three states in one place: it polls with a
    /// zero timeout while `sched` has work ready, blocks when nothing is ready
    /// and something parked can still be woken, and returns `0` without a
    /// syscall when there is nothing to wait for — including the case where
    /// tasks are parked but this reactor holds neither a registration nor a
    /// deadline that could ever wake one, which is a bug in the caller rather
    /// than a reason to block forever.
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
                && !(self.registrations.is_empty() && self.timers.is_empty())
            {
                // A filed deadline bounds the wait; with none, only readiness
                // can end it. Either way this is the blocking state rule 4
                // names, and the deadline only says how long it lasts.
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
            if woken > 0 || timeout == Some(Duration::ZERO) || self.timers.is_empty() {
                return Ok(woken);
            }
            // A bounded wait that came back with nothing to show for it, and a
            // deadline still filed. That is ordinary rather than exceptional —
            // a platform rounds a wait to its own timer granularity and can
            // return a fraction of a millisecond early — and returning `0` here
            // would tell the caller that nothing can wake these tasks any more,
            // which is exactly the task-abandoned bug. So the wait is retried
            // until it has genuinely reached the earliest deadline. The retry
            // is not free if readiness is also arriving for a task that no
            // longer parks; it is bounded by that deadline, and abandoning a
            // parked task is the worse of the two.
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

/// Runs `sched` under this thread's reactor until neither has anything left to
/// do.
///
/// The loop is the whole of this crate's I/O story in five lines:
/// [`Scheduler::run`] returns when the run queue empties, every task it
/// finished is retired from the reactor's table, and a non-zero parked count is
/// the "block in the reactor now" test. This is what a worker's body is, and
/// `Worker::spawn` hands the scheduler to a closure precisely so that closure
/// can be this.
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
        // The borrow opens here and closes here. It deliberately does not span
        // the `sched.run()` above, because a task suspending inside that call
        // takes the same borrow to register what it is waiting for.
        let woken = with_current(|reactor| {
            // Rule 3, on every turn rather than at the end: a task that
            // finished is one whose registrations must not outlive it, and the
            // finished list is only drained when its owner asks for it.
            for finished in sched.finished() {
                reactor.retire(finished.id);
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
            assert!(suspend(ctx, Waiting::Parked), "there was no scheduler");
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

    /// The decision this module's docs record, exercised the way `NvsTcp` will:
    /// the task is handed no reactor and no `Ctx`, and reaches both from free
    /// functions the way a `std::io::Read` will have to.
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
            assert!(suspend_current(Waiting::Parked), "there was no scheduler");
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
}
