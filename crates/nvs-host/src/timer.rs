//! Deadlines on the same reactor: one mechanism, and a sleep and a timeout are
//! two views of it.
//!
//! `docs/agent/loop-goal.md` § *Stage 2* item 5 is why this is one thing and
//! not two: `rule:concurrency/limit-and-deadline-are-the-only-bounds`
//! 's `{limit, deadline}` and
//! `rule:http-server/no-spelling-for-an-unbounded-wait`'s "no spelling
//! for an unbounded wait" both resolve to *this task must be runnable again at
//! this instant*. A sleep is that with nothing else to wait for; a deadline is
//! that raced against readiness. Writing them twice would be two clocks to keep
//! agreeing.
//!
//! # What the reactor holds, and why it is exact
//!
//! [`Timers`] is one entry per task — a task is one stack, and a stack waiting
//! on a deadline is not simultaneously waiting on another, so arming a second
//! one **replaces** the first rather than stacking. That is what makes
//! [`Timers::disarm`] exact, and being exact is the point: a heap of
//! fire-and-forget entries would keep an entry for a task that has already
//! finished until its deadline came round, which is a table that grows with
//! requests *served* under a load of abandoned deadlines rather than with the
//! ones in flight — what `rule:programs/memory-priority`
//! calls a leak rather than a trade-off. `Reactor::retire` drops a finished
//! task's timer for the same reason it drops its registrations.
//!
//! What it spends: two small entries per *task currently waiting on a deadline*
//! — the ordered index and the by-task one — and nothing per task that is not.
//! O(in-flight), and the ordering is a `BTreeSet` so the reactor reads the
//! earliest deadline without scanning.
//!
//! # How a deadline becomes a poll timeout
//!
//! `Reactor::turn` has `rule:concurrency/the-parking-contract` rule 4's states; a timer
//! changes only *how long* the blocking one blocks. With something parked and a
//! deadline filed, the poll waits until that deadline instead of indefinitely,
//! and the tasks it comes back due are woken exactly as readiness wakes one. A
//! task parked with a timer and nothing else is therefore not the "parked but
//! unwakeable" case that rule returns `0` for.
//!
//! # A wake is still a hint
//!
//! Rule 2 does not change here: [`park_until`] re-reads the clock after every
//! resume and re-arms if it was woken early, because the wake may have been
//! another descriptor this task holds. A caller therefore gets "the instant has
//! passed" and never "something happened".
//!
//! Off a core — `nvs run`, or a unit test — there is no coroutine to suspend
//! and no neighbour to starve, so the thread sleeps, exactly as
//! [`crate::net`]'s reads block there. That path is not the one with a
//! throughput target.
//!
//! # Reading a core's earliest deadline from another thread
//!
//! `rule:http-server/a-wedged-core-is-detected-by-its-deadline`
//! 's watchdog notices a core that has stopped turning at all, and it does
//! that by *reading the deadline this module already keeps* rather than by a
//! heartbeat written per request. [`Timers`] therefore publishes its first
//! entry into a small `Arc` — [`DeadlineView`] — every time that entry can have
//! changed, which is every method that touches the ordered index. This module's
//! docs are that mechanism's one home.
//!
//! **What a reader sees means "the core has not turned since", not "a request
//! is slow".** [`Timers::take_due`] removes a deadline the moment the core
//! polls after it, so a core that is turning never leaves one behind the clock
//! for longer than a poll. A view whose instant is a margin in the past is
//! therefore a statement about the *core*, and it is the same statement however
//! long the request that armed it asked for.
//!
//! The published value is nanoseconds after a `base` instant fixed when the
//! core's timers were created, held in one `AtomicU64` and written `Relaxed`.
//! An integer rather than a lock because the reader is a stranger: a watchdog
//! that could be descheduled while holding a mutex a core wants is a way to
//! wedge the very thing it was added to detect. There is nothing for the
//! ordering to publish alongside — the value is the whole message — and a
//! reader one store behind is a watchdog one interval late, not a wrong one.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use nvs_runtime::host::Woken;

use crate::reactor;
use crate::scheduler::{TaskId, Waiting, current_task, suspend_current};

/// The sentinel [`Published::earliest`] carries when no deadline is filed.
///
/// Zero, which would otherwise name `base` itself — so a deadline armed at or
/// before the instant its core started publishes as `1` instead (see
/// [`Timers::publish`]) and the sentinel is unambiguous rather than merely
/// unlikely.
const NOTHING: u64 = 0;

/// The cross-thread half of one core's [`Timers`]: its earliest deadline, in a
/// form a thread that is not that core may read.
///
/// Behind an `Arc` for the same reason `reactor::Remote` is — the reader
/// outlives the call that handed it out, and may outlive the core itself. A
/// view of a core that is gone keeps reporting whatever that core published
/// last, which is inert rather than wrong: the only thing above it is looking
/// for a deadline that stopped moving.
#[derive(Debug)]
struct Published {
    /// What the nanosecond count below is measured from, fixed when the core's
    /// timers were created. Carried here so the two ends share no clock and no
    /// process-wide epoch.
    base: Instant,
    /// Nanoseconds after `base` of the earliest deadline filed, or [`NOTHING`].
    earliest: AtomicU64,
}

impl Default for Published {
    fn default() -> Self {
        Self {
            base: Instant::now(),
            earliest: AtomicU64::new(NOTHING),
        }
    }
}

/// A read-only handle on one core's earliest deadline.
///
/// `rule:http-server/a-wedged-core-is-detected-by-its-deadline`'s watchdog reads a core through this and touches nothing else:
/// it carries no reference to the scheduler, the reactor or any task's stack,
/// and reading it neither blocks the core nor can be blocked by it. That makes
/// it — with `reactor::RemoteWake` — one of the few things in this crate that
/// is `Send`. This module's docs say what a reading means.
#[derive(Clone, Debug)]
pub struct DeadlineView(Arc<Published>);

impl DeadlineView {
    /// The earliest deadline the core is holding, or `None` if it holds none.
    #[must_use]
    pub fn oldest(&self) -> Option<Instant> {
        match self.0.earliest.load(Ordering::Relaxed) {
            NOTHING => None,
            nanos => Some(self.0.base + Duration::from_nanos(nanos)),
        }
    }
}

/// The deadlines one core is holding, earliest first.
///
/// Lives on the core's `Reactor` because the poll timeout is where a deadline
/// is actually enforced, and reached through `Reactor::timers`.
#[derive(Debug, Default)]
pub struct Timers {
    /// The ordered index. [`TaskId`] is in the key only to make it total —
    /// two tasks can be due at the same instant — and never to look one up.
    due: BTreeSet<(Instant, TaskId)>,
    /// Each task's own deadline, which is what makes [`Timers::disarm`] a
    /// removal rather than a scan.
    armed: HashMap<TaskId, Instant>,
    /// What the first entry of `due` is, for readers off this core. Shared
    /// with every [`DeadlineView`] handed out, which is the only reason any of
    /// it is behind an `Arc`.
    published: Arc<Published>,
}

impl Timers {
    /// Files `at` as `id`'s deadline, replacing any it already had.
    ///
    /// Replacing rather than adding is this module's one-timer-per-task rule;
    /// its docs say why that is sound.
    pub fn arm(&mut self, id: TaskId, at: Instant) {
        if let Some(previous) = self.armed.insert(id, at) {
            self.due.remove(&(previous, id));
        }
        self.due.insert((at, id));
        self.publish();
    }

    /// Drops `id`'s deadline, reporting whether it had one.
    pub fn disarm(&mut self, id: TaskId) -> bool {
        let Some(at) = self.armed.remove(&id) else {
            return false;
        };
        self.due.remove(&(at, id));
        self.publish();
        true
    }

    /// The earliest deadline filed, which is what the next poll waits for.
    #[must_use]
    pub fn next_due(&self) -> Option<Instant> {
        self.due.first().map(|&(at, _)| at)
    }

    /// Takes one task whose deadline has passed at `now`, or `None`.
    ///
    /// Called in a loop, so that a poll that came back late wakes everything
    /// that fell due while it waited rather than one per turn.
    pub fn take_due(&mut self, now: Instant) -> Option<TaskId> {
        let &(at, id) = self.due.first()?;
        if at > now {
            return None;
        }
        self.due.remove(&(at, id));
        self.armed.remove(&id);
        self.publish();
        Some(id)
    }

    /// A handle another thread may read this core's earliest deadline through.
    ///
    /// Any number of them; they all read the one value this core publishes.
    #[must_use]
    pub fn view(&self) -> DeadlineView {
        DeadlineView(Arc::clone(&self.published))
    }

    /// Republishes the first entry for [`DeadlineView`] readers.
    ///
    /// Called from every method that can change which entry that is, which is
    /// every method that touches `due`. It is one relaxed store on a path
    /// already doing a `BTreeSet` insert or removal, which is how `rule:http-server/a-wedged-core-is-detected-by-its-deadline`
    /// gets its watchdog for nothing: what is written is a change to state the
    /// deadline mechanism was keeping anyway, never a beat per request.
    fn publish(&self) {
        let value = self.due.first().map_or(NOTHING, |&(at, _)| {
            let nanos = at.saturating_duration_since(self.published.base).as_nanos();
            // A deadline further out than 584 years is not a deadline, and
            // pinning it at the maximum keeps it out of the sentinel's way.
            u64::try_from(nanos).unwrap_or(u64::MAX).max(NOTHING + 1)
        });
        self.published.earliest.store(value, Ordering::Relaxed);
    }

    /// How many tasks are waiting on a deadline.
    #[must_use]
    pub fn len(&self) -> usize {
        self.due.len()
    }

    /// Whether nothing is waiting on a deadline.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.due.is_empty()
    }
}

/// Parks the running task until `at`, giving the core back until then.
///
/// Returns when the instant has passed and not before: an early wake — another
/// descriptor this task holds becoming ready — sends it round again, which is
/// `rule:concurrency/the-parking-contract` rule 2 read on a clock rather than on a socket.
///
/// Off a core the thread sleeps instead; this module's docs say why that is not
/// the parking rule being bent.
pub fn park_until(at: Instant) -> Woken {
    loop {
        let now = Instant::now();
        if now >= at {
            return Woken::Elapsed;
        }
        let mut parked = false;
        if let Some(me) = current_task()
            && reactor::with_current(|reactor| reactor.timers().arm(me, at)).is_some()
        {
            let resumed = suspend_current(Waiting::Parked);
            parked = resumed.suspended();
            if !parked || resumed.cancelled() {
                // Nothing suspended, so nothing is coming back for that
                // deadline; leaving it filed would have the reactor holding a
                // wake for a task that never stopped running. A cancellation
                // owes the same tidy-up for the same reason — the wait is over
                // and the instant is still in the future.
                reactor::with_current(|reactor| reactor.timers().disarm(me));
            }
            if resumed.cancelled() {
                return Woken::Cancelled;
            }
        }
        if !parked {
            std::thread::sleep(at - now);
            return Woken::Elapsed;
        }
    }
}

/// Gives the core back for `duration`.
///
/// The other view of [`park_until`], and the one a script's `sleep` reaches:
/// same mechanism, same wake, one clock.
pub fn sleep(duration: Duration) -> Woken {
    park_until(Instant::now() + duration)
}

/// Parks the running task until `at` **or** until something wakes it, whichever
/// comes first.
///
/// The same mechanism as [`park_until`] with the loop taken off, and the
/// difference is the whole point: there, an early wake is noise to be re-armed
/// past, because the caller asked for an instant. Here the caller is waiting on
/// state a peer changes and the instant is only the bound on how long it may —
/// so a wake is the answer and the deadline is the failure, and returning on
/// either is what makes the caller's `loop { look; check the clock; wait }` a
/// wait rather than a poll.
///
/// [`Woken::Elapsed`] covers both endings for that reason: the caller re-reads
/// the state and its own clock, and neither answer would tell it anything it is
/// not about to look up. Off a core the thread sleeps out the remainder, since
/// nothing that could wake it is running either.
pub fn wait_until(at: Instant) -> Woken {
    let now = Instant::now();
    if now >= at {
        return Woken::Elapsed;
    }
    let armed = current_task()
        .filter(|&me| reactor::with_current(|reactor| reactor.timers().arm(me, at)).is_some());
    let Some(me) = armed else {
        std::thread::sleep(at - now);
        return Woken::Elapsed;
    };
    let resumed = suspend_current(Waiting::Parked);
    // Always, and not only on the paths that failed: a deadline that fired is
    // already off the reactor, and one that did not would otherwise stay filed
    // against a task that has gone back to work — this module's one-timer-per-
    // task rule is exact only if every arm has its disarm.
    reactor::with_current(|reactor| reactor.timers().disarm(me));
    if resumed.cancelled() {
        return Woken::Cancelled;
    }
    if !resumed.suspended() {
        // Nothing suspended, so nothing here could have been woken; sleeping the
        // remainder is the same answer `park_until` gives off a core, and the
        // alternative is the caller spinning its loop against the clock.
        std::thread::sleep(at - Instant::now().min(at));
    }
    Woken::Elapsed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactor::{Reactor, install, run_until_idle, with_current};
    use crate::scheduler::{Scheduler, Wake};
    use nvs_runtime::{Ctx, OutputSink, TaskRoot};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    /// The whole reason a sleep is not `thread::sleep`: the neighbour runs
    /// while it waits, and the sleeper still comes back.
    #[test]
    fn a_sleeping_task_hands_the_core_to_its_neighbour() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let order = Rc::new(RefCell::new(Vec::new()));
        let sleeper = Rc::clone(&order);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let start = Instant::now();
            sleep(Duration::from_millis(30));
            assert!(
                start.elapsed() >= Duration::from_millis(30),
                "the sleep came back early"
            );
            sleeper.borrow_mut().push("slept");
        });
        let neighbour = Rc::clone(&order);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            neighbour.borrow_mut().push("neighbour");
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.finished, 2);
        assert_eq!(
            *order.borrow(),
            ["neighbour", "slept"],
            "the sleep blocked the core instead of parking"
        );
        // The reactor is left holding nothing: a fired timer is taken, and a
        // finished task's is retired.
        assert_eq!(with_current(|reactor| reactor.timers().len()), Some(0));
    }

    /// Deadlines are answered in their own order, not the order they were
    /// filed — which is what makes the ordered index worth having.
    #[test]
    fn the_earliest_deadline_is_the_first_one_answered() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let order = Rc::new(RefCell::new(Vec::new()));
        for (label, millis) in [("late", 40_u64), ("early", 5)] {
            let woken = Rc::clone(&order);
            sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
                sleep(Duration::from_millis(millis));
                woken.borrow_mut().push(label);
            });
        }

        run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(*order.borrow(), ["early", "late"]);
    }

    /// A task parked on a deadline and nothing else is not the "parked but
    /// unwakeable" state rule 4 returns from: the poll has to bound its wait by
    /// the timer, or this hangs.
    #[test]
    fn a_timer_is_the_only_thing_that_needs_to_wake_a_task() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let done = Rc::new(RefCell::new(false));
        let reported = Rc::clone(&done);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            sleep(Duration::from_millis(10));
            *reported.borrow_mut() = true;
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert!(*done.borrow(), "the task never came back");
        assert_eq!(report.finished, 1);
        assert!(
            report.resumes >= 1,
            "the sleep never parked, so no timer was exercised"
        );
    }

    /// [`wait_until`]'s deadline half: with nothing arranged to wake it, a
    /// bounded wait is a sleep and comes back at its instant rather than hanging
    /// on the peer that never arrived.
    #[test]
    fn a_bounded_wait_with_nothing_to_wake_it_ends_on_its_deadline() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let start = Instant::now();
            assert!(matches!(
                wait_until(start + Duration::from_millis(20)),
                Woken::Elapsed
            ));
            assert!(
                start.elapsed() >= Duration::from_millis(20),
                "the wait came back early with nothing to have woken it"
            );
        });

        assert_eq!(
            run_until_idle(&mut sched)
                .expect("the loop failed")
                .finished,
            1
        );
        assert_eq!(with_current(|reactor| reactor.timers().len()), Some(0));
    }

    /// [`wait_until`]'s other half, and the whole difference from [`park_until`]:
    /// a wake ends it, because the caller is waiting on a peer and the instant
    /// is only how long it may. A deadline of five seconds against a test that
    /// finishes at once is the assertion.
    #[test]
    fn a_wake_ends_a_bounded_wait_before_its_deadline() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let handed = Rc::new(RefCell::new(None));
        let waiter = Rc::clone(&handed);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            // Taken before the wait, which is the ordering every caller of this
            // owes: a handle taken afterwards could be registered by a peer that
            // has already fired.
            *waiter.borrow_mut() = Wake::current();
            let start = Instant::now();
            assert!(matches!(
                wait_until(start + Duration::from_secs(5)),
                Woken::Elapsed
            ));
            assert!(
                start.elapsed() < Duration::from_secs(1),
                "the wake did not end the wait, so the deadline did"
            );
        });
        let peer = Rc::clone(&handed);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            peer.borrow()
                .as_ref()
                .expect("the waiter parked without handing its wake over")
                .wake();
        });

        assert_eq!(
            run_until_idle(&mut sched)
                .expect("the loop failed")
                .finished,
            2
        );
        // The deadline it did not use is off the reactor, which is what keeps
        // this module's one-timer-per-task rule exact.
        assert_eq!(with_current(|reactor| reactor.timers().len()), Some(0));
    }

    /// Item 5's whole claim, asserted with both spellings live on one core: a
    /// script's `sleep` and a socket's timeout are not two clocks. The table
    /// holds *both* waits, the reactor holds one registration — the socket's,
    /// and nothing of its own for the sleeper — and the two come back in
    /// deadline order rather than in the order they were filed.
    #[test]
    fn a_timer_and_a_deadline_are_the_same_wheel() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        // Held open for the whole test, and never written to: the read below
        // has to end on its deadline and on nothing else.
        let _client =
            std::net::TcpStream::connect(addr).expect("the loopback refused a connection");
        let (server, _) = listener.accept().expect("the accept failed");
        let mut stream =
            crate::net::NvsTcp::from_std(server).expect("the socket refused non-blocking mode");

        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let order = Rc::new(RefCell::new(Vec::new()));
        let by_reader = Rc::clone(&order);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            use std::io::Read as _;
            // Armed inside the task, so the window this read parks for is its
            // own and carries none of the fixture's setup. A deadline already
            // past when the read begins is answered without parking, and under
            // a sanitizer binding a listener and installing a reactor can take
            // longer than the window itself.
            stream.set_deadline(Some(Instant::now() + Duration::from_millis(25)));
            let mut buf = [0_u8; 4];
            let err = stream
                .read(&mut buf)
                .expect_err("a socket nothing was written to returned bytes");
            assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
            by_reader.borrow_mut().push("the socket's deadline");
        });
        let by_sleeper = Rc::clone(&order);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            // Filed second and due last, so deadline order and filing order
            // disagree — which is the only way the ordering assertion says
            // anything. The gap between the two is wide enough that a core
            // switch between them cannot reorder the pair.
            sleep(Duration::from_millis(150));
            by_sleeper.borrow_mut().push("the sleep");
        });

        assert_eq!(sched.run().parked, 2, "one of the two waits did not park");
        assert_eq!(
            with_current(|reactor| reactor.timers().len()),
            Some(2),
            "a timeout and a sleep were filed in two different tables"
        );
        assert_eq!(
            with_current(|reactor| reactor.registrations()),
            Some(1),
            "the sleeper registered a descriptor, or the reader registered none"
        );

        run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(*order.borrow(), ["the socket's deadline", "the sleep"]);
        assert_eq!(
            with_current(|reactor| reactor.timers().len()),
            Some(0),
            "a served deadline stayed in the table"
        );
    }

    /// Arming twice replaces, so the table holds one entry per waiting task and
    /// disarming is a removal rather than a sweep.
    #[test]
    fn arming_a_second_deadline_replaces_the_first() {
        let mut timers = Timers::default();
        let now = Instant::now();
        let id = TaskId::from_raw(7);

        timers.arm(id, now + Duration::from_secs(10));
        timers.arm(id, now + Duration::from_secs(1));
        assert_eq!(timers.len(), 1, "the replaced deadline was left filed");
        assert_eq!(timers.next_due(), Some(now + Duration::from_secs(1)));

        assert_eq!(timers.take_due(now), None, "a future deadline came due");
        assert_eq!(timers.take_due(now + Duration::from_secs(2)), Some(id));
        assert!(timers.is_empty());
        assert!(!timers.disarm(id), "a fired timer was still armed");
    }

    /// What the watchdog reads is the *first* entry and nothing else, and it
    /// tracks every edit to the index rather than only the arm that made it.
    #[test]
    fn the_published_deadline_is_the_earliest_one_filed() {
        let mut timers = Timers::default();
        let view = timers.view();
        let now = Instant::now();
        let (early, late) = (TaskId::from_raw(1), TaskId::from_raw(2));

        assert_eq!(view.oldest(), None, "an empty core published a deadline");
        timers.arm(late, now + Duration::from_secs(30));
        timers.arm(early, now + Duration::from_secs(5));
        assert_eq!(view.oldest(), Some(now + Duration::from_secs(5)));

        // Losing the first entry republishes the one behind it, whichever way
        // it is lost — otherwise the watchdog would read a deadline the core
        // has already answered and call a working core wedged.
        assert_eq!(timers.take_due(now + Duration::from_secs(6)), Some(early));
        assert_eq!(view.oldest(), Some(now + Duration::from_secs(30)));
        assert!(timers.disarm(late));
        assert_eq!(view.oldest(), None, "a disarmed deadline stayed published");
    }

    /// The whole point of the handle: a thread that is not the core reads it,
    /// and it stays readable after the core it describes is gone.
    #[test]
    fn a_view_crosses_a_thread_and_outlives_its_core() {
        let mut timers = Timers::default();
        let view = timers.view();
        let at = Instant::now() + Duration::from_secs(10);
        timers.arm(TaskId::from_raw(4), at);

        let reader = view.clone();
        let read = std::thread::spawn(move || reader.oldest())
            .join()
            .expect("the reader panicked");
        assert_eq!(read, Some(at), "the deadline did not survive the crossing");

        drop(timers);
        assert_eq!(
            view.oldest(),
            Some(at),
            "a view of a dead core answered nothing, so a wedged core would \
             read as an idle one"
        );
    }

    /// A core hands out its view through the reactor, without `&mut` and so
    /// without waiting for the core to be between turns.
    #[test]
    fn a_reactor_publishes_what_its_timers_hold() {
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        let view = with_current(|reactor| reactor.deadline_view()).expect("no reactor installed");
        let at = Instant::now() + Duration::from_secs(30);
        let id = TaskId::from_raw(9);

        assert_eq!(view.oldest(), None);
        with_current(|reactor| reactor.timers().arm(id, at));
        assert_eq!(view.oldest(), Some(at));
        with_current(|reactor| reactor.timers().disarm(id));
        assert_eq!(view.oldest(), None);
    }

    /// Off a core there is no coroutine to suspend, so the thread waits — and
    /// leaves nothing filed anywhere.
    #[test]
    fn a_sleep_off_a_core_waits_on_the_thread() {
        assert!(current_task().is_none(), "this test must run off a core");
        let start = Instant::now();
        sleep(Duration::from_millis(10));
        assert!(start.elapsed() >= Duration::from_millis(10));
    }
}
