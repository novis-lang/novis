//! The slice clock: one thread for the process, asking a request that has held
//! its core for a time slice to give the core back.
//!
//! A core runs one task at a time, and a task runs until it waits. A request
//! that computes and never waits would keep every other request on its core
//! waiting with it. This module is the half of the answer that notices: it
//! raises [`nvs_runtime::SafepointFlags::YIELD`] on the request a core has been running for
//! about [`SLICE`], and the request's next safepoint poll is where it gives the
//! core back.
//!
//! # Epochs, not clocks
//!
//! The thread keeps an epoch counter and adds one to it every [`TICK`] while any
//! core is in a turn. A core stores the epoch it reads into its own [`Slot`]
//! when it resumes a task ([`begin_turn`]), and clears the slot when its
//! scheduler's turn ends ([`end_turn`]). A slot two epochs behind the counter
//! has run at least one whole tick and at most two, and a turn that has run
//! [`SLICE`] is always two behind: that is when the thread raises the bit. It
//! raises it again at every tick after that, until the turn ends, so a request
//! that lowered the bit and kept running is asked again.
//!
//! The thread reads no core's clock. The watchdog's sweep reads each core's
//! thread clock once per `crate::watchdog::DEFAULT_INTERVAL`, which is fifty
//! slices: far too coarse to share a core, and too costly to run every tick.
//!
//! # Which request is asked
//!
//! A slot has no request in it. A core registers with the watchdog, and the
//! watchdog's meter already says which published request tree that thread is
//! running ([`crate::watchdog::switch_to`]). So the watchdog gives each
//! registration a [`Seat`] here, with a raise that finds the running
//! publication under the watchdog's lock and raises the bit through its
//! `SafepointView`. The lock is taken only when a slot is old, which is at most
//! once per tick per busy core. A request that is not published, which is a
//! thread with no per-thread clock, is asked by nobody.
//!
//! # Idle, the thread sleeps
//!
//! With every slot clear the thread parks with no timeout, so an idle server
//! wakes no thread for this. The core that starts the next turn unparks it.
//! The slot's store and the core's read of [`Clock::parked`] are sequentially
//! consistent, and so are the thread's store of that flag and its second look
//! at the slots: with relaxed ordering, each side could miss the other and the
//! thread would sleep through a turn that never ends.
//!
//! # What it costs
//!
//! One OS thread per process, parked while no core is in a turn. One slot per
//! registered thread, which is O(cores). A resume pays one thread-local read,
//! one relaxed load of the epoch, one sequentially consistent store and one
//! sequentially consistent load. The end of a turn pays one store. Nothing is
//! allocated per turn and no reference count changes.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::thread::{JoinHandle, Thread};
use std::time::Duration;

/// How long a task may hold its core before it is asked to give it back.
///
/// A constant with no configuration key. A switch costs a reactor poll, which
/// is small next to this, and a second number to explain buys an operator
/// nothing.
pub const SLICE: Duration = Duration::from_millis(10);

/// How often the epoch moves while any core is in a turn: half a [`SLICE`], so
/// a slot two epochs behind has run at most one slice.
pub const TICK: Duration = Duration::from_millis(5);

/// What one seat asks of its request when its turn is old.
type Raise = Box<dyn Fn() + Send + Sync + 'static>;

/// One registered thread's turn.
struct Slot {
    /// The epoch this thread's turn began in, or `0` while it is in no turn.
    turn: AtomicU64,
    raise: Raise,
}

/// What the slice thread and the seated threads share.
struct Clock {
    tick: Duration,
    /// Starts at `1`, so a slot's `0` always means no turn.
    epoch: AtomicU64,
    /// Set by the thread just before it parks with no timeout, and cleared by
    /// whichever core unparks it.
    parked: AtomicBool,
    /// Cleared by [`SliceClock`]'s drop; the thread returns the next time it
    /// looks.
    running: AtomicBool,
    slots: Mutex<Vec<Arc<Slot>>>,
    /// The thread, for an unpark. Set once, before the thread can park.
    thread: OnceLock<Thread>,
}

impl Clock {
    /// Unparks the thread if it is parked.
    fn wake(&self) {
        if self.parked.swap(false, Ordering::SeqCst)
            && let Some(thread) = self.thread.get()
        {
            thread.unpark();
        }
    }

    /// Raises every slot whose turn began two or more epochs before `epoch`,
    /// and says whether any slot is in a turn at all.
    fn sweep(&self, epoch: u64) -> bool {
        let mut busy = false;
        let mut old = Vec::new();
        {
            let slots = self.slots.lock().unwrap_or_else(PoisonError::into_inner);
            for slot in slots.iter() {
                let turn = slot.turn.load(Ordering::SeqCst);
                if turn == 0 {
                    continue;
                }
                busy = true;
                if epoch.saturating_sub(turn) >= 2 {
                    old.push(Arc::clone(slot));
                }
            }
        }
        // Outside the lock: a raise takes the watchdog's lock, and a seat is
        // made and dropped while a registration is changing.
        for slot in old {
            (slot.raise)();
        }
        busy
    }
}

/// The process's slice clock. Dropping it stops and joins its thread.
pub struct SliceClock {
    clock: Arc<Clock>,
    handle: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for SliceClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SliceClock")
            .field("epoch", &self.clock.epoch.load(Ordering::Relaxed))
            .field("parked", &self.clock.parked.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl SliceClock {
    /// Starts the slice thread, ticking every [`TICK`].
    ///
    /// A thread the OS refuses leaves a clock that asks nobody to yield, which
    /// is the same trade [`crate::watchdog`] makes for its own thread.
    #[must_use]
    pub fn start() -> Self {
        Self::ticking_every(TICK)
    }

    /// Starts the slice thread with its own tick.
    fn ticking_every(tick: Duration) -> Self {
        let clock = Arc::new(Clock {
            tick,
            epoch: AtomicU64::new(1),
            parked: AtomicBool::new(false),
            running: AtomicBool::new(true),
            slots: Mutex::new(Vec::new()),
            thread: OnceLock::new(),
        });
        // Locked once here, before the slice thread can see it. Where std's mutex
        // is a pthread one (macOS), its first lock allocates it. A seat taken on
        // this thread and the thread's first sweep race for that lock, so a
        // counted run's allocations would depend on which got there first.
        drop(clock.slots.lock().unwrap_or_else(PoisonError::into_inner));
        let ticking = Arc::clone(&clock);
        let handle = std::thread::Builder::new()
            .name("nvs-slice".to_owned())
            .spawn(move || {
                let _ = ticking.thread.set(std::thread::current());
                tick_until_stopped(&ticking);
            })
            .ok();
        Self { clock, handle }
    }

    /// A handle a seat is made from, on whichever thread takes the seat.
    pub(crate) fn handle(&self) -> SliceHandle {
        SliceHandle(Arc::clone(&self.clock))
    }

    #[cfg(test)]
    fn is_parked(&self) -> bool {
        self.clock.parked.load(Ordering::SeqCst)
    }
}

impl Drop for SliceClock {
    fn drop(&mut self) {
        self.clock.running.store(false, Ordering::SeqCst);
        self.clock.parked.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            handle.thread().unpark();
            let _ = handle.join();
        }
    }
}

/// A slice clock as another thread holds it: a seat can be made from it, and it
/// keeps no thread alive.
#[derive(Clone)]
pub(crate) struct SliceHandle(Arc<Clock>);

impl SliceHandle {
    /// Gives the calling thread a slot, which its scheduler's turns are stored
    /// in until the returned seat is dropped. `raise` is called on the slice
    /// thread when the thread's turn is old.
    pub(crate) fn seat(&self, raise: Raise) -> Seat {
        let slot = Arc::new(Slot {
            turn: AtomicU64::new(0),
            raise,
        });
        self.0
            .slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Arc::clone(&slot));
        let seated = Rc::new(Seated {
            clock: Arc::clone(&self.0),
            slot,
        });
        let outer = SEAT.replace(Some(Rc::clone(&seated)));
        Seat { seated, outer }
    }
}

/// What [`SEAT`] holds: the clock and this thread's slot in it.
struct Seated {
    clock: Arc<Clock>,
    slot: Arc<Slot>,
}

/// One thread's slot in the slice clock, given up when this is dropped.
///
/// `!Send`, like the registration that holds it: the slot is the turns of the
/// thread that made it.
pub(crate) struct Seat {
    seated: Rc<Seated>,
    outer: Option<Rc<Seated>>,
}

impl std::fmt::Debug for Seat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Seat").finish_non_exhaustive()
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        self.seated
            .clock
            .slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|slot| !Arc::ptr_eq(slot, &self.seated.slot));
        // `try_with`, because a seat a thread-local owns may be dropped after
        // this one is gone, and there is nothing left to put back then.
        let _ = SEAT.try_with(|seat| {
            let mut seat = seat.borrow_mut();
            if seat
                .as_ref()
                .is_some_and(|installed| Rc::ptr_eq(installed, &self.seated))
            {
                *seat = self.outer.take();
            }
        });
    }
}

thread_local! {
    /// The seat this thread took last, which its scheduler's turns are stored
    /// in. Empty on a thread with no seat.
    static SEAT: RefCell<Option<Rc<Seated>>> = const { RefCell::new(None) };
}

/// The turn this thread's slot holds now, which [`end_turn`] puts back when a
/// scheduler's turn ends. `0` on a thread with no seat or in no turn.
pub(crate) fn current_turn() -> u64 {
    SEAT.with_borrow(|seat| {
        seat.as_ref()
            .map_or(0, |seat| seat.slot.turn.load(Ordering::Relaxed))
    })
}

/// A task on this thread starts running: its slot holds the epoch from now on.
///
/// [`crate::scheduler::Scheduler`] calls it before every task it resumes. The
/// module doc's *What it costs* section owns the price.
pub(crate) fn begin_turn() {
    SEAT.with_borrow(|seat| {
        if let Some(seat) = seat {
            let epoch = seat.clock.epoch.load(Ordering::Relaxed);
            seat.slot.turn.store(epoch, Ordering::SeqCst);
            if seat.clock.parked.load(Ordering::SeqCst) {
                seat.clock.wake();
            }
        }
    });
}

/// A scheduler's turn on this thread ends: its slot holds `outer` again, which
/// is what [`current_turn`] gave at its start. A turn run inside another task's
/// turn so hands the slot back to that task.
pub(crate) fn end_turn(outer: u64) {
    SEAT.with_borrow(|seat| {
        if let Some(seat) = seat {
            seat.slot.turn.store(outer, Ordering::Relaxed);
        }
    });
}

/// The slice thread's body.
fn tick_until_stopped(clock: &Clock) {
    while clock.running.load(Ordering::SeqCst) {
        let epoch = clock.epoch.load(Ordering::Relaxed);
        if clock.sweep(epoch) {
            std::thread::park_timeout(clock.tick);
            clock.epoch.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        clock.parked.store(true, Ordering::SeqCst);
        // A second look after the flag is set: a core that stored its turn
        // before it could see the flag is found here.
        if clock.sweep(epoch) {
            clock.parked.store(false, Ordering::SeqCst);
            continue;
        }
        while clock.parked.load(Ordering::SeqCst) && clock.running.load(Ordering::SeqCst) {
            std::thread::park();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;
    use std::time::Instant;

    use nvs_runtime::SafepointFlags;

    use super::*;

    fn counting() -> (Arc<AtomicUsize>, Raise) {
        let count = Arc::new(AtomicUsize::new(0));
        let raised = Arc::clone(&count);
        let raise = Box::new(move || {
            raised.fetch_add(1, Ordering::SeqCst);
        });
        (count, raise)
    }

    /// Waits up to `bound` for `done`, and says whether it came.
    fn within(bound: Duration, done: impl Fn() -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < bound {
            if done() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        done()
    }

    #[test]
    fn a_turn_two_epochs_old_is_raised() {
        let clock = SliceClock::ticking_every(Duration::from_millis(2));
        let (count, raise) = counting();
        let _seat = clock.handle().seat(raise);
        let outer = current_turn();
        begin_turn();
        assert!(within(Duration::from_secs(10), || count
            .load(Ordering::SeqCst)
            > 0));
        end_turn(outer);
    }

    #[test]
    fn a_short_turn_is_not_raised() {
        let clock = SliceClock::ticking_every(Duration::from_millis(50));
        let (count, raise) = counting();
        let _seat = clock.handle().seat(raise);
        for _ in 0..20 {
            let outer = current_turn();
            begin_turn();
            end_turn(outer);
        }
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn the_thread_parks_with_every_core_idle_and_wakes_on_the_next_turn() {
        let clock = SliceClock::ticking_every(Duration::from_millis(2));
        let (count, raise) = counting();
        let _seat = clock.handle().seat(raise);
        assert!(within(Duration::from_secs(10), || clock.is_parked()));
        let parked_at = clock.clock.epoch.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            clock.clock.epoch.load(Ordering::SeqCst),
            parked_at,
            "a parked clock does not tick"
        );
        let outer = current_turn();
        begin_turn();
        assert!(!clock.is_parked());
        assert!(within(Duration::from_secs(10), || count
            .load(Ordering::SeqCst)
            > 0));
        end_turn(outer);
        assert!(within(Duration::from_secs(10), || clock.is_parked()));
    }

    #[test]
    fn a_turn_inside_a_turn_hands_the_slot_back() {
        let clock = SliceClock::ticking_every(Duration::from_secs(60));
        let (_count, raise) = counting();
        let _seat = clock.handle().seat(raise);
        let outermost = current_turn();
        begin_turn();
        let outer = current_turn();
        assert_ne!(outer, 0);
        begin_turn();
        end_turn(outer);
        assert_eq!(current_turn(), outer);
        end_turn(outermost);
        assert_eq!(current_turn(), 0);
    }

    /// Through the watchdog: the request its thread is running is asked, and a
    /// request published beside it and not running is not.
    #[test]
    fn the_request_a_registered_thread_runs_for_a_slice_is_asked_to_yield() {
        let clock = SliceClock::ticking_every(Duration::from_millis(2));
        let dog =
            crate::Watchdog::with(Duration::from_secs(3600), Duration::from_secs(3600), |_| {});
        dog.slice_with(&clock);
        let registered = dog.register_requests();
        if crate::ThreadClock::current().is_none() {
            // No per-thread clock, so nothing is published and nobody is asked.
            return;
        }
        let idle = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        let busy = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        // Each is published outside any task, so the thread runs the one
        // published last.
        registered.publish(idle.safepoint_view());
        registered.publish(busy.safepoint_view());
        let outer = current_turn();
        begin_turn();
        assert!(within(Duration::from_secs(10), || {
            busy.safepoint_flags().contains(SafepointFlags::YIELD)
        }));
        end_turn(outer);
        assert!(!idle.safepoint_flags().contains(SafepointFlags::YIELD));
        registered.withdraw(&busy.safepoint_view());
        registered.withdraw(&idle.safepoint_view());
    }

    #[test]
    fn a_dropped_seat_leaves_the_clock_and_the_thread() {
        let clock = SliceClock::ticking_every(Duration::from_millis(2));
        let (_count, raise) = counting();
        let seat = clock.handle().seat(raise);
        drop(seat);
        assert!(clock.clock.slots.lock().unwrap().is_empty());
        begin_turn();
        assert_eq!(current_turn(), 0);
    }
}
