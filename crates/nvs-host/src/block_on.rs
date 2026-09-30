//! Driving one `Future` on a coroutine: the seam between a library that is
//! `async` and a runtime that is not.
//!
//! `rule:concurrency/one-future-per-connection`
//! is this module's specification and its § 1 is the whole loop: **clear the
//! flag, poll, park.** [`block_on`] runs that on the stack of the task that
//! called it, which for the server is the coroutine that accepted the
//! connection, so what this adds to the runtime is a loop and not a second
//! scheduler. There is no ready queue here, no spawn and no work stealing: a
//! future that wants concurrency asks [`crate::Scheduler`] for a task, exactly
//! as `Core\Task` does.
//!
//! `Pending` suspends the **task** and not the thread. That is the only reason
//! the word `block` is honest in the name — the core goes back to its run queue
//! and serves its other connections, which is
//! `rule:http-server/a-core-is-never-blocked-on-a-syscall`
//! applied to a poll instead of to a syscall.
//!
//! # A waker is a permission to poll again, and it decides nothing
//!
//! `rule:concurrency/the-parking-contract`'s rule 2, in the shape `std::task` defines. Waking sets an
//! atomic flag and delivers the one permission the drive installed; it never
//! concludes the future is ready, never re-polls, and never touches the run
//! queue. The re-poll is the loop's, and the loop clears the flag *before* it
//! polls, so a wake fired from inside a `poll` is seen by the check after it
//! rather than by the iteration after that.
//!
//! Both lost-wakeup shapes fall out of that ordering. A wake landing between
//! the poll and the park leaves the flag set, and the drive re-reads it after
//! arming and before it suspends. A waker cloned and held past the call —
//! `hyper` keeps one for an upgrade — fires into an empty slot and sets a flag
//! nobody reads, which is inert in exactly the way the reactor's late wake is.
//!
//! # One permission per call, not per park
//!
//! The reason is [`crate::Reactor`]'s outstanding count: it goes up when a
//! [`RemoteWake`] is issued and comes down on the core — when the id the handle
//! queued is drained, or on the spot when the task it names gives it up — so
//! **every handle issued is collected there**. A handle taken freshly for every
//! park would therefore be issued and collected once per readiness edge for a
//! wake nobody asked for. Instead the drive installs one and leaves it
//! installed across parks; the waker *takes* it when it fires; the next park
//! finds the slot empty and issues a fresh one. A connection woken only by
//! socket readiness — the ordinary case, since the reactor wakes a task by id
//! and never through a waker — issues exactly one for its whole life, and gives
//! it back unused when the drive ends.
//!
//! What it spends, per `rule:programs/memory-priority`:
//! one `Arc` holding a flag and a slot, one `Waker`, at most one `RemoteWake`,
//! and the future itself on the coroutine's own stack. Per connection, so
//! O(in-flight), and nothing at all per poll or per park.
//!
//! # A wake from another thread wakes the task; it never moves it
//!
//! [`Waker`] promises `Send + Sync` to whoever holds it, while nothing else in
//! this crate is either. What crosses the boundary is a [`RemoteWake`]: a
//! [`TaskId`](crate::TaskId) and a poke of one core's poller, carrying no
//! reference to a scheduler, a stack or an arena. Firing it queues that id on
//! *that* core and ends the poll it is asleep in; the re-poll then happens on
//! the core that parked, on the coroutine's own stack. **A task never
//! migrates**, so "the future woke on another core" is not a state this design
//! has — and it is why the waker's payload is an `Arc` rather than
//! [`crate::Wake`], which is `Rc`-based and would be unsound at the first clone.
//!
//! # Off a core it blocks the thread, which is not the rule being bent
//!
//! The route is chosen once, when the drive starts, because a task never
//! migrates and the answer cannot change underneath it: a core with a reactor
//! parks and is woken by its permission; a core with no reactor — a
//! scheduler-only test — yields, because there is nothing there to arrange a
//! wake with and a park nothing can end is a wedge; off a core entirely there
//! is no coroutine to suspend and no neighbour to starve, so the thread parks
//! and the waker unparks it. `net`'s module doc makes the same call for the
//! same reason.

use std::future::Future;
use std::pin::pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use crate::reactor::{self, RemoteWake};
use crate::scheduler::{Waiting, current_task, suspend_current};

/// Drives `future` to completion on this task, giving the core back whenever it
/// answers `Pending`.
///
/// `None` means the task was cancelled while the drive was parked, and it is
/// the *rarer* of the two ways that ends: a stack standing on a
/// [`nvs_runtime::HelperFrame`] cannot be unwound, so the scheduler resumes it
/// with `Resumed::Cancelled` and this answers rather than parking again on a
/// wake that is not coming. The ordinary cancellation never returns from here
/// at all — the coroutine is torn down where it parked and the unwind drops the
/// future, closing whatever it held through Rust's own drops. Both run no Novis
/// frame, which is
/// `rule:concurrency/cancellation-runs-no-user-code`'s
/// rule; `Scheduler`'s teardown owns which of the two applies.
///
/// One future, on the calling task's own stack. Nothing is spawned and nothing
/// is queued; this module's docs and `rule:concurrency/one-future-per-connection` own why that is the whole
/// definition of the seam.
pub fn block_on<F: Future>(future: F) -> Option<F::Output> {
    let signal = Arc::new(Signal::here());
    let waker = Waker::from(Arc::clone(&signal));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        // Cleared before the poll and re-read after it, never the other way
        // round: the commonest wake there is comes from inside `poll` itself,
        // and clearing afterwards would drop it. `rule:concurrency/a-waker-is-one-permission-to-poll`.
        signal.woken.store(false, Ordering::SeqCst);
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            signal.disarm();
            return Some(value);
        }
        if !signal.park() {
            signal.disarm();
            return None;
        }
    }
}

/// What one drive shares with every clone of its waker: whether a wake has
/// happened, and how to reach the thing that is waiting.
///
/// `Send + Sync` because [`Waker`] is, which is why the route is held here
/// rather than the scheduler's own `!Send` [`crate::Wake`].
struct Signal {
    /// Set by a wake, cleared by the drive before each poll. This is what a
    /// wake leaves behind when there is no park to end, and it is the whole of
    /// the lost-wakeup argument.
    woken: AtomicBool,
    /// Where a park goes and what ends it. Decided once, when the drive starts.
    route: Route,
}

/// Where a drive can be standing, and what a park means in each place.
///
/// `rule:concurrency/the-park-route-is-chosen-once`'s table. Not re-derived per park: a task never migrates, and a
/// thread that has no core when the drive starts does not acquire one.
enum Route {
    /// On a core with a reactor — the server. The slot holds the one permission
    /// this drive has outstanding, installed before a park and taken by
    /// whichever wake fires first.
    Core { permit: Mutex<Option<RemoteWake>> },
    /// On a core with no reactor. There is nothing to arrange a wake with, so
    /// the drive goes to the back of the run queue and polls again.
    Yield,
    /// No core at all: `nvs run`, or a unit test. The thread itself waits.
    Thread(std::thread::Thread),
}

impl Signal {
    /// Reads which route this thread is on, right now.
    fn here() -> Self {
        let route = match current_task() {
            // Borrowed and dropped on the spot, only to ask whether this thread
            // has one at all — the permission itself is taken per park, in
            // `arm`, where the ordering rule applies to it.
            Some(_) if reactor::with_current(|_| ()).is_some() => Route::Core {
                permit: Mutex::new(None),
            },
            Some(_) => Route::Yield,
            None => Route::Thread(std::thread::current()),
        };
        Self {
            woken: AtomicBool::new(false),
            route,
        }
    }

    /// Waits for something to wake this drive, answering whether it may go on:
    /// `false` is a cancellation and nothing else.
    fn park(&self) -> bool {
        match &self.route {
            Route::Core { permit } => {
                // Rule 1's ordering: the permission exists *before* the
                // suspend, because a wake arriving in the gap would have
                // nothing to be recorded against.
                self.arm(permit);
                if self.woken.load(Ordering::SeqCst) {
                    // A wake landed between the poll and the arming and left
                    // only the flag. Going round again is what makes it not a
                    // lost wakeup; the permission stays installed for the next
                    // park, which is § 3's whole point.
                    return true;
                }
                let resumed = suspend_current(Waiting::Parked);
                // `NotSuspended` is unreachable here — `current_task` answered
                // when the route was chosen, and a task does not lose its
                // scheduler — so this reads only the one answer that matters.
                !resumed.cancelled()
            }
            Route::Yield => !suspend_current(Waiting::Yielded).cancelled(),
            Route::Thread(_) => {
                // The flag check is an optimization and not the correctness
                // argument: `unpark` leaves a token, so a wake that raced this
                // check makes the park below return at once.
                if !self.woken.load(Ordering::SeqCst) {
                    std::thread::park();
                }
                true
            }
        }
    }

    /// Makes sure this drive has a permission outstanding, issuing one only if
    /// the last one has been fired.
    fn arm(&self, permit: &Mutex<Option<RemoteWake>>) {
        let mut held = lock(permit);
        if held.is_some() {
            // Still armed, so the last park ended in readiness on this task's
            // own reactor registration rather than in a waker. Re-issuing here
            // is what would cost a handle per readiness edge —
            // `rule:concurrency/one-permission-per-drive`.
            return;
        }
        let Some(me) = current_task() else {
            return;
        };
        *held = reactor::with_current(|reactor| reactor.remote_wake(me));
    }

    /// Gives up any permission still outstanding, at the end of the drive.
    ///
    /// Giving it up is not optional and dropping it is how that is done: the
    /// reactor's outstanding count comes back down when a handle is collected,
    /// so one merely forgotten would keep the core from ever concluding that
    /// nothing can wake it. The drive is the task the handle names and it is
    /// running, so the drop queues no wake — [`RemoteWake`]'s own doc.
    fn disarm(&self) {
        if let Route::Core { permit } = &self.route {
            drop(lock(permit).take());
        }
    }
}

impl std::task::Wake for Signal {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // The flag first and the permission second, always. A wake that finds
        // no permission installed — one fired during a poll, or by a waker
        // outliving the drive — has to leave something behind, and the flag is
        // it.
        self.woken.store(true, Ordering::SeqCst);
        match &self.route {
            Route::Core { permit } => {
                if let Some(wake) = lock(permit).take() {
                    // A failed poke loses the wake's promptness and not the
                    // wake — the id is queued before the poke is attempted —
                    // and a waker has nobody to report an error to.
                    let _ = wake.wake();
                }
            }
            Route::Yield => {}
            Route::Thread(thread) => thread.unpark(),
        }
    }
}

impl std::fmt::Debug for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let route = match &self.route {
            Route::Core { .. } => "core",
            Route::Yield => "yield",
            Route::Thread(_) => "thread",
        };
        f.debug_struct("Signal")
            .field("woken", &self.woken.load(Ordering::SeqCst))
            .field("route", &route)
            .finish()
    }
}

/// The same poison rule the rest of this crate uses: a slot whose writer
/// panicked still holds a permission that has to be delivered.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactor::{Reactor, install, run_until_idle, with_current};
    use crate::scheduler::Scheduler;
    use nvs_runtime::{Ctx, OutputSink, TaskRoot};
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    #[test]
    fn a_ready_future_is_polled_once_and_never_parks() {
        let polls = Cell::new(0_usize);
        let answer = block_on(std::future::poll_fn(|_| {
            polls.set(polls.get() + 1);
            Poll::Ready(7_u32)
        }));
        assert_eq!(answer, Some(7));
        assert_eq!(polls.get(), 1, "a ready future was polled more than once");
    }

    /// `rule:concurrency/a-waker-is-one-permission-to-poll`'s ordering, from the only side that can observe it: a wake
    /// fired *inside* a poll is seen by the check after that poll. A drive that
    /// cleared its flag afterwards would park on a wake that has already
    /// happened, and this test would hang rather than fail.
    #[test]
    fn a_future_that_wakes_itself_inside_its_own_poll_is_polled_again() {
        let polls = Cell::new(0_usize);
        let answer = block_on(std::future::poll_fn(|cx| {
            polls.set(polls.get() + 1);
            if polls.get() < 3 {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            Poll::Ready("done")
        }));
        assert_eq!(answer, Some("done"));
        assert_eq!(polls.get(), 3);
    }

    /// The shape the server is built on: the drive parks its coroutine, the
    /// core goes to sleep in its poll, and a thread that is not this core ends
    /// both with one wake. `rule:concurrency/a-wake-never-moves-a-task`.
    #[test]
    fn a_wake_from_another_thread_drives_a_parked_future_to_completion() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let slot: Arc<Mutex<Option<Waker>>> = Arc::new(Mutex::new(None));
        let far = Arc::clone(&slot);
        let finished = Rc::new(Cell::new(0_usize));
        let counter = Rc::clone(&finished);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut polls = 0_usize;
            let answer = block_on(std::future::poll_fn(move |cx| {
                polls += 1;
                if polls == 1 {
                    *lock(&slot) = Some(cx.waker().clone());
                    return Poll::Pending;
                }
                Poll::Ready(polls)
            }));
            counter.set(answer.expect("the drive was cancelled"));
        });

        let far_side = std::thread::spawn(move || {
            // Take the waker as soon as the first poll has published it, then
            // wait long enough that the core is inside its poll rather than
            // racing it. A wake that arrives early is still not lost — it is
            // the flag — so this timing decides which path runs, not whether
            // the test passes.
            let waker = loop {
                if let Some(waker) = lock(&far).take() {
                    break waker;
                }
                std::thread::sleep(Duration::from_millis(1));
            };
            std::thread::sleep(Duration::from_millis(20));
            waker.wake();
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        far_side.join().expect("the far side panicked");
        assert_eq!(finished.get(), 2, "the drive did not reach its second poll");
        assert_eq!(report.finished, 1);
        assert_eq!(
            report.parked, 0,
            "the core gave up on a task it was told to expect"
        );
        assert_eq!(
            with_current(|reactor| reactor.remote_waits()),
            Some(0),
            "the drive left a permission outstanding"
        );
    }

    /// `rule:concurrency/one-permission-per-drive`: one permission for the whole drive, not one per park.
    #[test]
    fn a_parked_drive_holds_exactly_one_permission() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let id = sched.spawn(ctx(), TaskRoot::Worker, |_ctx| {
            block_on(std::future::pending::<()>());
        });
        let report = sched.run();
        assert_eq!(report.parked, 1, "the drive did not park");
        assert_eq!(
            with_current(|reactor| reactor.remote_waits()),
            Some(1),
            "a drive issued a permission per park"
        );
        // Left tidy: the cancellation ends the drive, and the one permission it
        // was holding goes with it.
        assert_eq!(sched.cancel(id), 1);
        sched.run();
    }

    /// A future that never finishes and reports its own drop.
    struct NeverReady(Rc<Cell<bool>>);

    impl Future for NeverReady {
        type Output = ();

        fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
            Poll::Pending
        }
    }

    impl Drop for NeverReady {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    /// `rule:concurrency/a-cancelled-drive-never-parks-again`, the ordinary half: an unwindable stack is torn down where
    /// it parked, so the drive never returns at all and the future is dropped
    /// by the unwind. No `Resumed` is involved and no Novis frame runs — the
    /// scheduler's `Drop for RunQueue` doc owns the rule this asserts.
    #[test]
    fn a_cancelled_drive_lets_the_unwind_drop_its_future() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let dropped = Rc::new(Cell::new(false));
        let flag = Rc::clone(&dropped);
        let id = sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            block_on(NeverReady(flag));
            unreachable!("an unwindable drive is torn down where it parked");
        });
        assert_eq!(sched.run().parked, 1, "the drive did not park");
        assert_eq!(sched.cancel(id), 1);
        let report = sched.run();
        assert!(dropped.get(), "the cancellation did not drop the future");
        assert_eq!(report.parked, 0, "the cancelled task is still parked");
    }

    /// `rule:concurrency/a-cancelled-drive-never-parks-again`, the other half: a stack standing on a helper frame cannot
    /// be unwound, so the scheduler hands it `Resumed::Cancelled` instead — and
    /// the drive has to answer rather than park again on a wake that is not
    /// coming.
    #[test]
    fn a_cancelled_drive_on_a_helper_frame_answers_none() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let answered = Rc::new(Cell::new(false));
        let seen = Rc::clone(&answered);
        let id = sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let _frame = nvs_runtime::HelperFrame::enter();
            seen.set(block_on(std::future::pending::<()>()).is_none());
        });
        assert_eq!(sched.run().parked, 1);
        assert_eq!(sched.cancel(id), 1);
        let report = sched.run();
        assert!(answered.get(), "the drive did not end on the cancellation");
        assert_eq!(report.finished, 1);
    }
}
