//! `rule:testing/task-tree-and-virtual-clock`'s virtual clock: the fixed wall
//! clock a `#[Test(at: …)]` arms, shared by the test's task tree, with the
//! deadlines its sleeps and timeouts park on.
//!
//! # One clock per tree, shared by its tasks
//!
//! A [`TestClock`] is one `Rc` the test's isolate holds and every task
//! [`Ctx::child`](crate::Ctx::child) opens under it shares, so a task that
//! sleeps moves the clock its siblings and its parent read. An isolate a test
//! starts — a `Core\Test::request` — gets a clock of its own, set from the
//! test's reading when the call starts, so its sleeps move its copy and never
//! the test's. An `Rc` and not an `Arc`: a tree never leaves the core it runs on
//! (`nvs_host::scheduler`'s module doc), and neither does the clock.
//!
//! # A sleep parks on a deadline, and an idle core moves the clock
//!
//! [`sleep`] files the calling task's waker under its deadline and parks with
//! no real bound. [`advance_idle`] is the one thing that moves the clock on its
//! own: the scheduler calls it when its run queue is empty, which is when no
//! task of any tree on the core can run, and it moves every clock with a
//! deadline filed to its earliest one and fires the wakers due there. Deadlines
//! are kept by `(instant, order filed)`, so two tasks with equal deadlines wake
//! in the order they parked. `Core\Test::advance` moves the clock through
//! [`TestClock::set`], which fires every deadline it passes the same way.
//!
//! A wake is a hint here as everywhere: [`sleep`] reads the clock after each
//! resume and parks again if its deadline is still ahead.
//!
//! # What it spends
//!
//! One `Rc` per test isolate and per request a test makes, with one entry per
//! task currently parked on a deadline, freed with the isolate. A thread-local
//! list holds a `Weak` to each clock that has a deadline filed, so the
//! scheduler's idle check is one length read where no test runs. Nothing on a
//! context with no fixed clock.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use crate::host::{Waker, Woken, with_current};

/// A fixed wall clock in nanoseconds since the Unix epoch, and the deadlines
/// parked on it. See the module docs.
pub struct TestClock {
    /// The reading `Core\Time::now` gives.
    now: Cell<i128>,
    /// The parked deadlines, by `(instant, order filed)`.
    deadlines: RefCell<BTreeMap<(i128, u64), Waker>>,
    /// The order the next deadline is filed in.
    filed: Cell<u64>,
    /// Whether this clock is on [`WAITING`].
    listed: Cell<bool>,
}

impl std::fmt::Debug for TestClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestClock")
            .field("now", &self.now.get())
            .field("deadlines", &self.deadlines.borrow().len())
            .finish()
    }
}

thread_local! {
    /// Every clock on this thread with a deadline filed, so the scheduler's
    /// idle check reads one list and not every task.
    static WAITING: RefCell<Vec<Weak<TestClock>>> = const { RefCell::new(Vec::new()) };
}

impl TestClock {
    /// A clock reading `nanos`, with no deadline filed.
    #[must_use]
    pub fn new(nanos: i128) -> Rc<Self> {
        Rc::new(Self {
            now: Cell::new(nanos),
            deadlines: RefCell::new(BTreeMap::new()),
            filed: Cell::new(0),
            listed: Cell::new(false),
        })
    }

    /// The clock's reading, in nanoseconds since the Unix epoch.
    #[must_use]
    pub fn now(&self) -> i128 {
        self.now.get()
    }

    /// Moves the clock to `nanos` and wakes every task whose deadline is at or
    /// before it, earliest first.
    pub fn set(&self, nanos: i128) {
        self.now.set(nanos);
        for waker in self.take_due(nanos) {
            waker();
        }
    }

    /// Files `waker` under `deadline`, and returns the key [`Self::forget`]
    /// takes it back out by.
    pub fn file(self: &Rc<Self>, deadline: i128, waker: Waker) -> (i128, u64) {
        let order = self.filed.get();
        self.filed.set(order.wrapping_add(1));
        self.deadlines.borrow_mut().insert((deadline, order), waker);
        if !self.listed.replace(true) {
            WAITING.with_borrow_mut(|waiting| waiting.push(Rc::downgrade(self)));
        }
        (deadline, order)
    }

    /// Takes the deadline filed under `key` back out, if it has not fired.
    pub fn forget(&self, key: (i128, u64)) {
        self.deadlines.borrow_mut().remove(&key);
    }

    /// The wakers due at `nanos`, in the order they wake, taken out of the
    /// map before any of them runs.
    fn take_due(&self, nanos: i128) -> Vec<Waker> {
        let mut deadlines = self.deadlines.borrow_mut();
        let later = deadlines.split_off(&(nanos.saturating_add(1), 0));
        let due = std::mem::replace(&mut *deadlines, later);
        due.into_values().collect()
    }

    /// The earliest deadline filed, if any.
    fn earliest(&self) -> Option<i128> {
        self.deadlines
            .borrow()
            .first_key_value()
            .map(|(&(at, _), _)| at)
    }
}

/// Whether any clock on this thread has a deadline filed. One length read.
#[must_use]
pub fn any_waiting() -> bool {
    WAITING.with_borrow(|waiting| !waiting.is_empty())
}

/// Moves every clock on this thread that has a deadline filed to its earliest
/// one, and fires the wakers due there. Returns how many it fired.
///
/// The scheduler calls this when its run queue is empty: no task on the core
/// can run, so the time until the next deadline would pass with nothing
/// happening, and the clock skips it. A clock left with no deadline is taken
/// off the list.
pub fn advance_idle() -> usize {
    if !any_waiting() {
        return 0;
    }
    let clocks: Vec<Rc<TestClock>> = WAITING.with_borrow_mut(|waiting| {
        waiting.retain(|clock| {
            clock.upgrade().is_some_and(|clock| {
                let keep = clock.earliest().is_some();
                clock.listed.set(keep);
                keep
            })
        });
        waiting.iter().filter_map(Weak::upgrade).collect()
    });
    let mut fired = 0;
    for clock in clocks {
        if let Some(at) = clock.earliest() {
            let at = at.max(clock.now());
            clock.now.set(at);
            let due = clock.take_due(at);
            fired += due.len();
            for waker in due {
                waker();
            }
        }
    }
    fired
}

/// Takes a filed deadline back out when it is dropped, so a task torn down
/// while it sleeps leaves no waker behind.
struct Filed<'a> {
    clock: &'a TestClock,
    key: (i128, u64),
}

impl Drop for Filed<'_> {
    fn drop(&mut self) {
        self.clock.forget(self.key);
    }
}

/// Waits until `clock` reads `deadline`: parks the calling task on the
/// deadline until the clock reaches it. With no task beneath the call nothing
/// else can run, so the clock moves to the deadline at once.
pub fn wait_until(clock: &Rc<TestClock>, deadline: i128) -> Woken {
    while clock.now() < deadline {
        let Some(waker) = with_current(|host| host.waker()).flatten() else {
            clock.set(deadline);
            break;
        };
        let filed = Filed {
            clock,
            key: clock.file(deadline, waker),
        };
        let woken = with_current(|host| host.park(None)).unwrap_or(Woken::Elapsed);
        drop(filed);
        if matches!(woken, Woken::Cancelled) {
            return Woken::Cancelled;
        }
    }
    Woken::Elapsed
}

/// A sleep of `duration` on `clock`: [`wait_until`] its reading plus the
/// duration.
pub fn sleep(clock: &Rc<TestClock>, duration: std::time::Duration) -> Woken {
    let nanos = i128::try_from(duration.as_nanos()).unwrap_or(i128::MAX);
    wait_until(clock, clock.now().saturating_add(nanos))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Moving the clock fires the deadlines it passes, earliest first and in
    /// filing order on a tie, and leaves the later ones filed.
    #[test]
    fn set_fires_the_deadlines_it_passes_in_order() {
        let clock = TestClock::new(0);
        let woken = Rc::new(RefCell::new(Vec::new()));
        for (at, name) in [(20, "b"), (10, "a"), (20, "c"), (30, "d")] {
            let woken = Rc::clone(&woken);
            clock.file(at, Box::new(move || woken.borrow_mut().push(name)));
        }
        clock.set(20);
        assert_eq!(*woken.borrow(), ["a", "b", "c"]);
        assert_eq!(clock.earliest(), Some(30));
        assert_eq!(advance_idle(), 1);
        assert_eq!(clock.now(), 30);
        assert_eq!(*woken.borrow(), ["a", "b", "c", "d"]);
        assert_eq!(advance_idle(), 0);
        assert!(!any_waiting());
    }
}
