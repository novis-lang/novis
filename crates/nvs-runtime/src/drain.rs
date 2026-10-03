//! `rule:http-server/the-server-block-is-boot-class`
//! 's drain: the one bit a health probe and an application both read.
//!
//! The bit lives here rather than beside the accept loop because its readers
//! sit on opposite sides of the crate graph. `nvs_server::Draining` is the
//! loop's handle over it and owns the rule that the loop is its only writer;
//! `Core\Server::isDraining()` is the other reader, and `nvs-stdlib` does not
//! depend on `nvs-server` — nor should it, since a `Core` member that needed
//! the HTTP server linked in to answer a question about this process would put
//! `hyper` in the graph of every CLI program. This crate is what both rest on,
//! so this is where a fact both of them state has to sit.
//!
//! # One relaxed atomic
//!
//! Read once per probe, written once per process. There is no ordering to
//! establish because the bit is the whole of the message: a reader that sees
//! `false` an instant before the drain begins is indistinguishable from one
//! that asked an instant earlier, and no second fact has to be visible with it.
//! This is not on the value path [`crate::string`]'s non-atomic refcounts
//! protect — it is one relaxed load per health request, and a health request is
//! not the request path. The relaxed load stays sound beside the registry
//! below because every reader that has to agree with the list reads the bit
//! *while holding the list's lock*, and [`Drain::begin`] sets it before taking
//! that lock: one of the two critical sections runs first, and either the
//! register saw the bit or the drain found the entry.
//!
//! # Waking what is parked
//!
//! A drain that only set a bit would be discovered by a connection when its own
//! idle timer expired, so a stop would take the longest wait a deployment
//! configured before the drain period even started
//! ([ADR 0186](/docs/decisions/0186.md) § 3). So a
//! wait that is about to park registers a wake through [`Drain::wake_at_drain`]
//! and holds the [`DrainWake`] for as long as it is parked;
//! [`Drain::begin`] fires every one of them.
//!
//! **A wake is not a close.** It makes a parked task runnable and nothing else:
//! the task re-reads the bit at the wait it parked on and closes its own
//! connection there, which is
//! `rule:concurrency/a-drain-closes-a-connection-cleanly`'s requirement that
//! the close be the connection's own. How long it has from that moment is
//! `[server] drain_timeout`, resolved into `nvs_config::Waits::drain` and
//! carried to a connection with the rest of its clock rather than held here —
//! this crate has the bit because two crates that cannot see each other read
//! it, which is not true of a number a connection is already handed.
//!
//! A registered wake is `Send`, which [`crate::host::Waker`] deliberately is
//! not: the drain is begun by whichever thread took the signal, and the tasks
//! it wakes are parked on every core. A host supplies the boxed closure, so the
//! cross-thread hop is that host's own — `nvs-host`'s `RemoteWake` — and this
//! module names neither a core nor a task.
//!
//! What it holds is one boxed closure per parked wait, dropped when the wait
//! ends or when the drain fires it: O(parked waits), which is O(in-flight).
//!
//! # Process-wide, and one handle that is not
//!
//! [`Drain::process`] is the bit every real server and every reader of the
//! probe takes, for [`Drain`]'s own stated reason: a shutdown drains the
//! process, so a bit held per core would answer differently depending on which
//! core happened to take the connection.
//!
//! [`Drain::detached`] is the other case, and it is a server rather than a
//! test-only escape hatch: an accept loop run inside a process that is not
//! *only* that server — a test binary, or a second listener a harness ends on
//! its own — stops a **server**, and reporting the process as draining because
//! one of them stopped would be the fail-*closed* direction of the same error
//! `Drain::process` exists to prevent.
//!
//! # A drain that follows others
//!
//! [`Drain::any_of`] is a drain that begins when any of the drains it was made
//! from begins, and that can also be begun on its own. A connection reads one:
//! its server's drain, which a stop begins, joined with the drain of the
//! configuration it was accepted under, which a reload begins
//! (`rule:concurrency/a-drain-closes-a-connection-cleanly`). Everything a
//! connection does at its waits is then the same for both endings. The link
//! is an ordinary registration on each parent, held by the joined drain and
//! dropped with it, and the parent's wake reaches the joined drain through a
//! weak pointer, so a parent never keeps a joined drain alive.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};

/// One drain: the bit, the wakes owed the moment it is set, and the
/// registrations that begin it when a drain it follows begins.
struct State {
    begun: AtomicBool,
    parked: Mutex<Parked>,
    follows: Vec<DrainWake>,
}

impl State {
    /// A drain nothing has begun and nothing is waiting on.
    fn fresh() -> Self {
        Self {
            begun: AtomicBool::new(false),
            follows: Vec::new(),
            parked: Mutex::new(Parked {
                next: 0,
                held: Vec::new(),
            }),
        }
    }
}

/// The wakes registered against a drain that has not begun, each under the
/// ticket its holder gives back.
///
/// A ticket rather than a position, because a [`DrainWake`] is dropped in
/// whatever order the waits it belongs to end in and a position would name a
/// different entry the moment one before it went away. The counter is never
/// reused, which is what makes a stale ticket match nothing rather than the
/// wrong registration.
struct Parked {
    next: u64,
    held: Vec<(u64, Box<dyn FnOnce() + Send>)>,
}

/// This process's own drain, allocated on the first ask and never again.
///
/// A `OnceLock` rather than a `static` of its own so that a handle is one type
/// whichever drain it points at: [`Drain::detached`] owns its state, so the
/// shared one has to be ownable too, and an `Arc` is what makes both spellings
/// the same one-word struct.
fn process_state() -> &'static Arc<State> {
    static PROCESS: OnceLock<Arc<State>> = OnceLock::new();
    PROCESS.get_or_init(|| Arc::new(State::fresh()))
}

/// A handle on a server's drain — whether it has stopped accepting, and what
/// to wake when it does.
///
/// Cloning a handle shares the drain; that is the whole of what a handle is
/// for. Which drain it is comes from the constructor and from nowhere else, so
/// a caller that wants the process's answer has said so.
#[derive(Clone)]
pub struct Drain(Arc<State>);

impl Drain {
    /// The process's drain — what a health probe reports and what
    /// `Core\Server::isDraining()` answers.
    #[must_use]
    pub fn process() -> Self {
        Self(Arc::clone(process_state()))
    }

    /// A drain nothing else in this process can see.
    ///
    /// For a server whose stopping is not this process stopping; the module
    /// docs above own why that is a real case and not a test affordance.
    #[must_use]
    pub fn detached() -> Self {
        Self(Arc::new(State::fresh()))
    }

    /// A drain that begins when any of `parents` begins, and that
    /// [`Drain::begin`] can also begin on its own.
    ///
    /// It has already begun when one of `parents` has. Memory: one registration
    /// on each parent, for as long as the returned drain lives.
    #[must_use]
    pub fn any_of(parents: &[&Self]) -> Self {
        let joined = Self(Arc::new_cyclic(|me: &Weak<State>| {
            let mut state = State::fresh();
            for parent in parents {
                let me = Weak::clone(me);
                let link = parent.wake_at_drain(move || {
                    if let Some(me) = me.upgrade() {
                        Self(me).begin();
                    }
                });
                state.follows.extend(link);
            }
            state
        }));
        // A parent that began while the state above was being built fired a
        // wake that could not reach it yet. That parent's bit is set before it
        // fires anything, so this read finds it.
        if parents.iter().any(|parent| parent.is_draining()) {
            joined.begin();
        }
        joined
    }

    /// Stop accepting, and wake everything parked: from here the probe answers
    /// `503`.
    ///
    /// Idempotent, because a drain that has begun cannot begin again and a
    /// second core reaching this is the same shutdown, not a new one. The
    /// second call finds an empty list, since a wake is owed once.
    pub fn begin(&self) {
        self.0.begun.store(true, Ordering::Relaxed);
        // Taken out under the lock and fired outside it: a wake runs a
        // scheduler's own code, and a task that resumes far enough to drop its
        // `DrainWake` would take this same lock.
        let owed = std::mem::take(&mut self.parked().held);
        for (_, wake) in owed {
            wake();
        }
    }

    /// Whether the drain this handle names has begun.
    #[must_use]
    pub fn is_draining(&self) -> bool {
        self.0.begun.load(Ordering::Relaxed)
    }

    /// Fire `wake` when this drain begins, for a wait that is about to park.
    ///
    /// The registration lives as long as the returned [`DrainWake`], so a wait
    /// that ends for its own reason takes its wake with it. `None` is a drain
    /// that has **already** begun: `wake` has been fired before this returns,
    /// so a caller that is about to park is told by the same call that
    /// registers it, and there is nothing left to hold.
    ///
    /// A wake is a hint, exactly as [`crate::host::Waker`] is: whoever parks
    /// re-reads [`Drain::is_draining`] when it resumes rather than treating the
    /// resume as the answer.
    pub fn wake_at_drain(&self, wake: impl FnOnce() + Send + 'static) -> Option<DrainWake> {
        let mut parked = self.parked();
        // Read inside the lock, against a `begin` that stores before it takes
        // it: whichever critical section runs first, this either fires here or
        // is in the list `begin` takes.
        if self.0.begun.load(Ordering::Relaxed) {
            drop(parked);
            wake();
            return None;
        }
        let ticket = parked.next;
        parked.next += 1;
        parked.held.push((ticket, Box::new(wake)));
        drop(parked);
        Some(DrainWake {
            drain: Arc::clone(&self.0),
            ticket,
        })
    }

    /// The wake list, taking a poisoned lock as the list it holds.
    ///
    /// A panic on the far side of it happened while a `Vec` was being pushed to
    /// or drained, neither of which can leave a half-written entry, and
    /// refusing to wake a connection because some other thread panicked is the
    /// direction that hangs a shutdown.
    fn parked(&self) -> std::sync::MutexGuard<'_, Parked> {
        self.0.parked.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl std::fmt::Debug for Drain {
    /// Hand-written: the wakes are boxed closures, which have nothing to print.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drain")
            .field("draining", &self.is_draining())
            .finish_non_exhaustive()
    }
}

/// A wake registered on [`Drain::wake_at_drain`], held for as long as the wait
/// that registered it is parked.
///
/// Dropping it deregisters, which is why it is the return value rather than a
/// registration a caller could forget: a wake left behind by every wait that
/// ever ended would grow the list with connections *served* instead of with
/// connections in flight.
#[must_use = "the wake is deregistered when this is dropped"]
#[derive(Debug)]
pub struct DrainWake {
    drain: Arc<State>,
    ticket: u64,
}

impl Drop for DrainWake {
    fn drop(&mut self) {
        let mut parked = self
            .drain
            .parked
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        // Nothing to remove once the drain has fired: `begin` took the whole
        // list, and this ticket is simply not in it.
        parked.held.retain(|(ticket, _)| *ticket != self.ticket);
    }
}

impl std::fmt::Debug for State {
    /// For [`DrainWake`]'s derive, which cannot print the closures either.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("begun", &self.begun)
            .finish_non_exhaustive()
    }
}

/// Whether *this process* has begun draining, with no handle to ask through.
///
/// The same answer as [`Drain::process`]'s, for a caller that has no server to
/// have taken a handle from — a `Core` member reached from a request, which is
/// the only reader that is neither the accept loop nor its own probe.
#[must_use]
pub fn is_draining() -> bool {
    process_state().begun.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{Drain, is_draining};

    /// A counter a wake can be built over, so a test asserts how many times it
    /// fired rather than merely that it did.
    fn counter() -> (Arc<AtomicUsize>, impl FnOnce() + Send + 'static) {
        let fired = Arc::new(AtomicUsize::new(0));
        let held = Arc::clone(&fired);
        (fired, move || {
            held.fetch_add(1, Ordering::Relaxed);
        })
    }

    /// A detached drain is one server's, and beginning it says nothing about
    /// the process or about any other server in it.
    ///
    /// This is the property an in-process accept loop rests on, so it is
    /// asserted rather than assumed from the constructor's name.
    #[test]
    fn a_detached_drain_is_one_servers_alone() {
        let first = Drain::detached();
        let second = Drain::detached();
        first.begin();
        assert!(first.is_draining(), "a drain that began reported accepting");
        assert!(
            !second.is_draining(),
            "one server's drain was reported by another's handle"
        );
        assert!(
            !Drain::process().is_draining(),
            "a detached drain reported this process as draining"
        );
    }

    /// A clone shares the bit — the whole of what makes this a handle rather
    /// than a value a caller could copy and then disagree with.
    #[test]
    fn a_cloned_handle_shares_the_bit() {
        let held = Drain::detached();
        let copy = held.clone();
        held.begin();
        assert!(
            copy.is_draining(),
            "a cloned handle answered from a bit of its own"
        );
    }

    /// The free function and the process handle are one bit, whatever that bit
    /// currently says.
    ///
    /// Asserted as *agreement* rather than against `false`: this test binary's
    /// other tests share the process, and a test that pinned the value would be
    /// pinning the order they run in.
    #[test]
    fn the_free_reader_and_the_process_handle_agree() {
        assert_eq!(
            is_draining(),
            Drain::process().is_draining(),
            "the reader with no handle answered from a different bit"
        );
    }

    /// The drain wakes what is parked on it, once — the property a shutdown
    /// bounded by `drain_timeout` rather than by the longest configured wait
    /// rests on.
    ///
    /// The second `begin` is the assertion that a wake is owed once: a
    /// shutdown that reached this twice would otherwise fire a closure the
    /// connection it belongs to has already answered.
    #[test]
    fn a_registered_wake_fires_once_when_the_drain_begins() {
        let drain = Drain::detached();
        let (fired, wake) = counter();
        let registration = drain.wake_at_drain(wake);
        assert!(
            registration.is_some(),
            "a drain that had not begun gave nothing to hold"
        );
        assert_eq!(fired.load(Ordering::Relaxed), 0, "a wake fired early");
        drain.begin();
        drain.begin();
        assert_eq!(
            fired.load(Ordering::Relaxed),
            1,
            "the drain did not wake what was parked on it exactly once"
        );
    }

    /// A wait that registers after the drain has begun is told by the same
    /// call, because it is about to park and nothing would wake it again.
    #[test]
    fn a_wake_registered_on_a_drain_that_has_begun_fires_at_once() {
        let drain = Drain::detached();
        drain.begin();
        let (fired, wake) = counter();
        let registration = drain.wake_at_drain(wake);
        assert!(
            registration.is_none(),
            "a drain that had already begun handed back something to hold"
        );
        assert_eq!(
            fired.load(Ordering::Relaxed),
            1,
            "a wake registered on a drain that had begun was never fired"
        );
    }

    /// A wait that ends for its own reason takes its wake with it, which is
    /// what keeps the list O(parked waits) rather than O(waits served).
    #[test]
    fn a_dropped_registration_is_not_woken() {
        let drain = Drain::detached();
        let (ended, ending) = counter();
        let (parked, parking) = counter();
        drop(drain.wake_at_drain(ending));
        let _still_parked = drain.wake_at_drain(parking);
        drain.begin();
        assert_eq!(
            ended.load(Ordering::Relaxed),
            0,
            "a wake whose wait had ended was fired"
        );
        assert_eq!(
            parked.load(Ordering::Relaxed),
            1,
            "dropping one registration took another one's wake with it"
        );
    }

    /// A joined drain begins with either parent, wakes what is parked on it,
    /// and beginning it begins neither parent.
    #[test]
    fn a_joined_drain_begins_with_either_parent_and_never_the_other_way() {
        let server = Drain::detached();
        let generation = Drain::detached();
        let joined = Drain::any_of(&[&server, &generation]);
        let (woken, wake) = counter();
        let _parked = joined.wake_at_drain(wake);
        generation.begin();
        assert!(
            joined.is_draining(),
            "the second parent's drain was not followed"
        );
        assert_eq!(
            woken.load(Ordering::Relaxed),
            1,
            "the joined drain's wait was not woken"
        );
        assert!(
            !server.is_draining(),
            "a joined drain began its other parent"
        );

        let alone = Drain::any_of(&[&server]);
        alone.begin();
        assert!(
            !server.is_draining(),
            "beginning a joined drain began its parent"
        );
        server.begin();
        assert!(
            Drain::any_of(&[&server]).is_draining(),
            "a drain joined to one that had begun reported accepting"
        );
    }

    /// A parent holds no joined drain alive: dropping the joined drain takes
    /// its registration off the parent, so a parent's list grows with the
    /// drains alive now and not with every one made from it.
    #[test]
    fn a_dropped_joined_drain_leaves_nothing_on_its_parent() {
        let parent = Drain::detached();
        for _ in 0..3 {
            drop(Drain::any_of(&[&parent]));
        }
        assert!(
            parent.parked().held.is_empty(),
            "a dropped joined drain stayed registered"
        );
    }
}
