//! Deadlines on the same reactor: one mechanism, and a sleep and a timeout are
//! two views of it.
//!
//! `docs/agent/loop-goal.md` § *Stage 2* item 5 is why this is one thing and
//! not two: [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
//! § 3's `{limit, deadline}` and
//! [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 5's "no spelling
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
//! ones in flight — what [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md)
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
//! `Reactor::turn` already had ADR 0115 § 2 rule 4's three states; a timer
//! changes only *how long* the blocking one blocks. With something parked and a
//! deadline filed, the poll waits until that deadline instead of indefinitely,
//! and the tasks it comes back due are woken exactly as readiness wakes one. A
//! task parked with a timer and nothing else is therefore no longer the "parked
//! but unwakeable" case that rule returns `0` for.
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

use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};

use crate::reactor;
use crate::scheduler::{TaskId, Waiting, current_task, suspend_current};

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
    }

    /// Drops `id`'s deadline, reporting whether it had one.
    pub fn disarm(&mut self, id: TaskId) -> bool {
        let Some(at) = self.armed.remove(&id) else {
            return false;
        };
        self.due.remove(&(at, id));
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
        Some(id)
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
/// ADR 0115 § 2 rule 2 read on a clock rather than on a socket.
///
/// Off a core the thread sleeps instead; this module's docs say why that is not
/// the parking rule being bent.
pub fn park_until(at: Instant) {
    loop {
        let now = Instant::now();
        if now >= at {
            return;
        }
        let mut parked = false;
        if let Some(me) = current_task()
            && reactor::with_current(|reactor| reactor.timers().arm(me, at)).is_some()
        {
            parked = suspend_current(Waiting::Parked);
            if !parked {
                // Nothing suspended, so nothing is coming back for that
                // deadline; leaving it filed would have the reactor holding a
                // wake for a task that never stopped running.
                reactor::with_current(|reactor| reactor.timers().disarm(me));
            }
        }
        if !parked {
            std::thread::sleep(at - now);
            return;
        }
    }
}

/// Gives the core back for `duration`.
///
/// The other view of [`park_until`], and the one a script's `sleep` reaches:
/// same mechanism, same wake, one clock.
pub fn sleep(duration: Duration) {
    park_until(Instant::now() + duration);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactor::{Reactor, install, run_until_idle, with_current};
    use crate::scheduler::Scheduler;
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
