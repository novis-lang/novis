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
//! fourth state.
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
//! # One reactor per worker
//!
//! Rule 5, and it is the same rule [`Scheduler`] holds: a [`Reactor`] is
//! `!Send` and `!Sync`, so a descriptor a request owns is registered with, and
//! woken by, that request's own core. Nothing crosses, which is what keeps the
//! refcounts in `nvs-runtime` non-atomic.

use std::collections::HashMap;
use std::io;
use std::marker::PhantomData;
use std::time::Duration;

use mio::{Events, Poll, Token};

use crate::scheduler::{RunReport, Scheduler, TaskId};

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
    _pinned_to_one_thread: PhantomData<*const ()>,
}

impl std::fmt::Debug for Reactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Neither the poll handle nor the event buffer says anything a debug
        // line can use, and both would print differently between runs.
        f.debug_struct("Reactor")
            .field("registrations", &self.registrations.len())
            .field("ready", &self.ready.len())
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
        self.registrations.remove(&id).is_some()
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
    /// tasks are parked but this reactor holds no registration that could ever
    /// wake one, which is a bug in the caller rather than a reason to block
    /// forever.
    ///
    /// # Errors
    ///
    /// As [`Reactor::poll`].
    pub fn turn(&mut self, sched: &mut Scheduler) -> io::Result<usize> {
        let timeout = if sched.ready_count() > 0 {
            Some(Duration::ZERO)
        } else if sched.parked_count() > 0 && !self.registrations.is_empty() {
            None
        } else {
            return Ok(0);
        };

        let mut woken = 0;
        for &id in self.poll(timeout)? {
            // `false` is ordinary — rule 2's stale or racing wake — so it is
            // counted out rather than reported.
            if sched.wake(id) {
                woken += 1;
            }
        }
        Ok(woken)
    }
}

/// Runs `sched` under `reactor` until neither has anything left to do.
///
/// The loop is the whole of this crate's I/O story in five lines:
/// [`Scheduler::run`] returns when the run queue empties, every task it
/// finished is retired from the reactor's table, and a non-zero parked count is
/// the "block in the reactor now" test. This is what a worker's body is, and
/// `Worker::spawn` hands the scheduler to a closure precisely so that closure
/// can be this.
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
/// right trade for a call that cannot continue.
pub fn run_until_idle(sched: &mut Scheduler, reactor: &mut Reactor) -> io::Result<RunReport> {
    let mut total = RunReport::default();
    loop {
        let turn = sched.run();
        total.resumes += turn.resumes;
        total.finished += turn.finished;
        // Rule 3, on every turn rather than at the end: a task that finished
        // is one whose registrations must not outlive it, and the finished
        // list is only drained when its owner asks for it.
        for finished in sched.finished() {
            reactor.retire(finished.id);
        }
        if sched.parked_count() == 0 {
            break;
        }
        if reactor.turn(sched)? == 0 && sched.ready_count() == 0 {
            break;
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
    use crate::scheduler::{Waiting, suspend};
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
        let mut reactor = Reactor::new().expect("the OS refused a poll");

        let resumes = Rc::new(Cell::new(0_usize));
        let counter = Rc::clone(&resumes);
        let id = sched.spawn(ctx(), TaskRoot::Worker, move |ctx| {
            counter.set(counter.get() + 1);
            assert!(suspend(ctx, Waiting::Parked), "there was no scheduler");
            counter.set(counter.get() + 1);
        });
        reactor
            .register(&mut server, id, Interest::READABLE)
            .expect("the OS refused a registration");
        // Readiness is arranged before the loop is entered, so the blocking
        // poll inside it has something waiting for it already.
        client.write_all(b"hi").expect("the write failed");

        let report = run_until_idle(&mut sched, &mut reactor).expect("the loop failed");
        assert_eq!(resumes.get(), 2, "the task did not resume after its park");
        assert_eq!(report.finished, 1);
        assert_eq!(report.parked, 0);
        // Rule 3: the loop retired the registration when the task ended.
        assert_eq!(reactor.registrations(), 0);
        assert!(!reactor.is_registered(id));
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
        let mut reactor = Reactor::new().expect("the OS refused a poll");

        sched.spawn(ctx(), TaskRoot::Worker, |ctx| {
            suspend(ctx, Waiting::Parked);
        });

        let report = run_until_idle(&mut sched, &mut reactor).expect("the loop failed");
        assert_eq!(report.parked, 1, "the stuck task should be reported");
        assert_eq!(report.finished, 0);
    }
}
