//! The bell `Core\Queue::push` rings and an idle queue worker waits on
//! (`rule:concurrency/a-push-wakes-an-idle-worker`).
//!
//! A worker that finds no due job waits before it asks the database again, and that wait is what
//! an idle `[queue]` costs: the longer it is, the less an empty queue is asked. A job this process
//! pushed is the one piece of new work the process knows about without asking, so `push` rings
//! once the job is committed and every idle worker claims at once.
//!
//! **Only this process hears it.** A job another process pushed, a delayed job coming due, a retry
//! and a job whose worker died ring nothing here, and the bound on a worker's idle wait is what
//! finds each of them.
//!
//! **Nothing in it names a driver.** A ring is a call made after a statement, or after the commit
//! that made the statement durable, and neither is a message a database sends. So the one path is
//! every backend's, and a backend with a notification channel of its own is not asked for it.
//!
//! **The count closes the gap between a claim and the wait after it.** A worker reads
//! [`Bell::rings`] before it asks for work and hands that reading to [`Bell::wake_at_ring`] when
//! the answer was nothing. A push that committed after the worker's claim looked, and rang before
//! the worker registered, has moved the count, so the registration is refused and the worker
//! claims again instead of waiting out a job it was told about.
//!
//! A registered wake is `Send` for [`nvs_runtime::Drain`]'s reason: the request that pushes runs
//! on whichever core accepted it, and the workers are parked on the one core that arms them. A
//! host supplies the Rust closure, so the hop across threads is that host's own.
//!
//! **What it holds:** one boxed Rust closure per idle worker, dropped when the wait ends or when a
//! ring fires it. That is O(idle workers), which `[queue] workers` bounds. A ring with nobody
//! waiting takes one uncontended lock and allocates nothing.

use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

/// What a bell has counted, and the wakes owed at its next ring.
struct Waiting {
    /// How many times this bell has been rung.
    rings: u64,
    /// The ticket the next registration takes. Never reused, so a stale ticket matches nothing.
    next: u64,
    /// One wake per idle worker, each under the ticket its holder gives back.
    held: Vec<(u64, Box<dyn FnOnce() + Send>)>,
}

impl std::fmt::Debug for Waiting {
    /// Hand-written: the wakes are boxed Rust closures, which have nothing to print.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Waiting")
            .field("rings", &self.rings)
            .field("held", &self.held.len())
            .finish_non_exhaustive()
    }
}

/// A handle on one bell. Cloning it shares the bell.
///
/// [`Bell::process`] is the one `Core\Queue::push` rings and the one both binaries hand their
/// workers. [`Bell::detached`] is a bell nothing else in the process can ring, for workers whose
/// wakes a caller wants to account for one by one.
#[derive(Clone, Debug)]
pub struct Bell(Arc<Mutex<Waiting>>);

impl Bell {
    /// This process's bell, allocated on the first ask and never again.
    #[must_use]
    pub fn process() -> Self {
        static PROCESS: OnceLock<Bell> = OnceLock::new();
        PROCESS.get_or_init(Self::detached).clone()
    }

    /// A bell nothing else in this process holds.
    #[must_use]
    pub fn detached() -> Self {
        Self(Arc::new(Mutex::new(Waiting {
            rings: 0,
            next: 0,
            held: Vec::new(),
        })))
    }

    /// How many times this bell has been rung — what a worker reads before it asks for work.
    #[must_use]
    pub fn rings(&self) -> u64 {
        self.waiting().rings
    }

    /// Counts one ring and wakes every worker waiting on this bell.
    ///
    /// A wake is a hint, as [`nvs_runtime::Drain::begin`]'s is: the worker it resumes asks the
    /// database, and waits again when the answer is nothing.
    pub fn ring(&self) {
        // Taken out under the lock and fired outside it: a wake runs a scheduler's own code, and
        // a worker that resumes far enough to drop its `BellWake` takes this same lock.
        let owed = {
            let mut waiting = self.waiting();
            waiting.rings += 1;
            std::mem::take(&mut waiting.held)
        };
        for (_, wake) in owed {
            wake();
        }
    }

    /// Fires `wake` at the next ring, for a worker that read `heard` before its claim and is
    /// about to wait.
    ///
    /// `None` is a bell rung since `heard` was read: there is work the claim did not look at, so
    /// the caller claims again instead of waiting, and `wake` is dropped unfired. Otherwise the
    /// registration lives as long as the returned [`BellWake`], so a wait that ends for its own
    /// reason takes its wake with it.
    pub fn wake_at_ring(
        &self,
        heard: u64,
        wake: impl FnOnce() + Send + 'static,
    ) -> Option<BellWake> {
        let mut waiting = self.waiting();
        // Compared inside the lock a ring counts under, so a ring either moved the count before
        // this read or finds the entry pushed below.
        if waiting.rings != heard {
            drop(waiting);
            return None;
        }
        let ticket = waiting.next;
        waiting.next += 1;
        waiting.held.push((ticket, Box::new(wake)));
        drop(waiting);
        Some(BellWake {
            bell: Arc::clone(&self.0),
            ticket,
        })
    }

    /// The bell's state, taking a poisoned lock as the state it holds.
    ///
    /// A panic on the far side of it happened while a `Vec` was being pushed to or taken, neither
    /// of which leaves a half-written entry, and a bell that stopped ringing because another
    /// thread panicked would leave every job to the idle wait.
    fn waiting(&self) -> MutexGuard<'_, Waiting> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A wake registered on [`Bell::wake_at_ring`], held for as long as the worker that registered it
/// waits.
///
/// Dropping it deregisters, so the list grows with the workers waiting and never with the waits
/// that have ended.
#[must_use = "the wake is deregistered when this is dropped"]
#[derive(Debug)]
pub struct BellWake {
    bell: Arc<Mutex<Waiting>>,
    ticket: u64,
}

impl Drop for BellWake {
    fn drop(&mut self) {
        let mut waiting = self.bell.lock().unwrap_or_else(PoisonError::into_inner);
        // Nothing to remove once a ring has fired: it took the whole list, and this ticket is not
        // in it.
        waiting.held.retain(|(ticket, _)| *ticket != self.ticket);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

    use super::Bell;

    /// The lock a case in this crate holds while it counts what [`Bell::process`] was rung.
    ///
    /// Every case in a test binary shares the process and so shares that bell. A case that asserts
    /// a count of its rings holds this for as long as it counts, and so does any other case that
    /// rings it.
    pub(crate) fn process_bell_alone() -> MutexGuard<'static, ()> {
        static ALONE: Mutex<()> = Mutex::new(());
        ALONE.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A counter a wake can be built over, so a case asserts how many times it fired.
    fn counter() -> (Arc<AtomicUsize>, impl FnOnce() + Send + 'static) {
        let fired = Arc::new(AtomicUsize::new(0));
        let held = Arc::clone(&fired);
        (fired, move || {
            held.fetch_add(1, Ordering::Relaxed);
        })
    }

    /// A ring wakes what is waiting, once, and a second ring finds nobody.
    #[test]
    fn a_ring_fires_a_registered_wake_once() {
        let bell = Bell::detached();
        let (fired, wake) = counter();
        let registration = bell.wake_at_ring(bell.rings(), wake);
        assert!(
            registration.is_some(),
            "a bell nobody had rung gave nothing to hold"
        );
        assert_eq!(fired.load(Ordering::Relaxed), 0, "a wake fired early");
        bell.ring();
        bell.ring();
        assert_eq!(
            fired.load(Ordering::Relaxed),
            1,
            "two rings did not wake the one waiting worker exactly once"
        );
    }

    /// A ring between a worker's reading of the count and its registration refuses the
    /// registration, which is what stops a worker waiting out a job it was told about.
    #[test]
    fn a_ring_since_the_count_was_read_refuses_the_registration() {
        let bell = Bell::detached();
        let heard = bell.rings();
        bell.ring();
        let (fired, wake) = counter();
        assert!(
            bell.wake_at_ring(heard, wake).is_none(),
            "a worker was let wait after a ring its claim never looked for"
        );
        assert_eq!(
            fired.load(Ordering::Relaxed),
            0,
            "a refused registration fired the wake it was handed"
        );
    }

    /// A wait that ends for its own reason takes its wake with it.
    #[test]
    fn a_dropped_registration_is_not_woken() {
        let bell = Bell::detached();
        let (ended, ending) = counter();
        let (waiting, parked) = counter();
        drop(bell.wake_at_ring(bell.rings(), ending));
        let _still_waiting = bell.wake_at_ring(bell.rings(), parked);
        bell.ring();
        assert_eq!(
            (
                ended.load(Ordering::Relaxed),
                waiting.load(Ordering::Relaxed)
            ),
            (0, 1),
            "a ring fired the wake of a wait that had ended, or missed the one still waiting"
        );
    }

    /// A detached bell is its holder's alone, and a clone is the same bell.
    #[test]
    fn a_detached_bell_is_rung_by_its_own_handles_only() {
        let first = Bell::detached();
        let second = Bell::detached();
        first.clone().ring();
        assert_eq!(
            (first.rings(), second.rings()),
            (1, 0),
            "a clone rang a different bell, or one bell's ring was counted by another"
        );
    }
}
