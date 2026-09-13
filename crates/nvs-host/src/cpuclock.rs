//! One thread's CPU time, sampled by a thread that is not it.
//!
//! `docs/agent/goals/40-resource-ceilings.md` § *Stage 3* puts the sampler on
//! [`crate::watchdog`]'s thread — one thread for the process, already waking on
//! an interval and already holding an entry per core. That thread is not the
//! one burning the CPU it has to measure, so it needs a handle on somebody
//! else's clock, and this module is the platform half of that: a core takes a
//! [`ThreadClock`] on **itself**, and any thread holding it can read what that
//! core has spent.
//!
//! # CPU time, never wall clock
//!
//! A wall clock cannot tell a runaway loop from a slow database query: both sit
//! in one request for a long time, and only one of them is spending the
//! machine. `[limits] cpu_time` exists to stop the first without touching the
//! second, so what is sampled here is the thread's own user plus kernel time
//! and never an instant. A request parked on a socket accumulates none of it.
//!
//! # Where there is no such clock
//!
//! [`ThreadClock::current`] answers `None` on a platform with no per-thread
//! clock a stranger may read, and a caller that gets `None` enforces no CPU
//! ceiling and says so — never a wall-clock one in its place, which would stop
//! exactly the requests the ceiling exists to leave alone.
//!
//! Linux answers with `pthread_getcpuclockid`, whose clock id is plain data any
//! thread may pass to `clock_gettime`. Windows answers with the thread id,
//! reopened per sample: `GetThreadTimes` wants a handle, and an id is a `u32`
//! this type can be `Copy` over while a handle is something that would have to
//! be owned, closed and taught to be `Send`. A sample is one `OpenThread` and
//! one `CloseHandle` per core per watchdog interval, on a thread that is
//! already awake — the price of holding no operating-system resource between
//! sweeps.
//!
//! Every other platform is the `None` arm. macOS has no `pthread_getcpuclockid`
//! and its answer is `thread_info` on a mach port, which nothing in this tree
//! reaches yet; that is the gap the paragraph above describes rather than a
//! case anything here pretends to cover.

use std::time::Duration;

/// A handle on one thread's CPU clock, taken by that thread and read by any.
///
/// Plain data — an identifier, never a borrow and never an operating-system
/// handle — so it is `Copy`, `Send` and `Sync`, and holding one costs the
/// thread it names nothing at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreadClock(Clock);

impl ThreadClock {
    /// A clock on the **calling** thread, or `None` where this platform has no
    /// per-thread clock another thread may read.
    ///
    /// Must be taken on the thread it is to measure: every platform here asks
    /// the operating system about the caller, and there is no spelling that
    /// takes a thread as an argument.
    #[must_use]
    pub fn current() -> Option<Self> {
        current().map(Self)
    }

    /// What that thread has burned, user and kernel time together.
    ///
    /// `None` once the thread has ended, and on any platform whose
    /// [`ThreadClock::current`] answered `None` — a caller cannot tell the two
    /// apart and does not need to, because the answer to both is the same: this
    /// core is not one a CPU ceiling can be enforced on.
    #[must_use]
    pub fn burned(&self) -> Option<Duration> {
        burned(self.0)
    }
}

/// Spin on the calling thread until its own clock has charged it `at_least`
/// more than it read on entry.
///
/// A test that needs a charge cannot spin for a *wall* interval and read the
/// clock afterwards. Beside the other test binaries `tools/verify.py` runs at
/// the same time, a thread is preempted for most of any window it is given,
/// and what the scheduler charges for the rest is quantised at its tick, so a
/// fixed window can be charged nothing at all. The clock the ceiling is
/// enforced on is the one to wait on; it is read every few thousand turns
/// rather than every turn, which keeps the system call out of the loop's cost.
///
/// Panics once `PATIENCE` of wall time has passed with the charge still short:
/// a thread that spun that long and was charged nothing is the property under
/// test failing, not a slow machine.
#[cfg(test)]
pub(crate) fn burn_at_least(at_least: Duration) {
    const PATIENCE: Duration = Duration::from_secs(60);
    let clock =
        ThreadClock::current().expect("a test that burns has checked this thread has a clock");
    let from = clock
        .burned()
        .expect("this thread's own clock did not read");
    let give_up_at = std::time::Instant::now() + PATIENCE;
    let mut turns = 0_u64;
    loop {
        turns = turns.wrapping_add(1);
        if !turns.is_multiple_of(4096) {
            continue;
        }
        let charged = clock
            .burned()
            .expect("a live thread read as no thread")
            .saturating_sub(from);
        if charged >= at_least {
            return;
        }
        assert!(
            std::time::Instant::now() < give_up_at,
            "a thread that spun for {PATIENCE:?} of wall time was charged {charged:?} of the \
             {at_least:?} it waited for"
        );
    }
}

/// What a host says as it starts about the ceiling it can enforce: the note to
/// print where this platform offers no per-thread clock, and `None` where it
/// offers one and there is nothing to report.
///
/// The *Where there is no such clock* section above is what a caller owes its
/// operator, and saying it at boot is how it is paid: `[limits] cpu_time` that
/// nothing enforces is worse unsaid than unenforced, because the operator who
/// wrote the key has no other way to learn it does nothing here.
#[must_use]
pub fn no_ceiling_note() -> Option<&'static str> {
    ThreadClock::current().is_none().then_some(
        "note: this platform offers no per-thread CPU clock, so `[limits] cpu_time` is not \
         enforced on this host",
    )
}

#[cfg(target_os = "linux")]
type Clock = libc::clockid_t;

#[cfg(target_os = "linux")]
fn current() -> Option<Clock> {
    let mut clock: Clock = 0;
    #[expect(
        unsafe_code,
        reason = "`pthread_getcpuclockid` is the only way to name a thread's own CPU clock in a \
                  form another thread may read, and `std` has no equivalent. The out-parameter is \
                  a stack `clockid_t` this call owns exclusively, the thread argument is the \
                  caller's own, and nothing here outlives the call."
    )]
    let named = unsafe { libc::pthread_getcpuclockid(libc::pthread_self(), &raw mut clock) } == 0;
    named.then_some(clock)
}

#[cfg(target_os = "linux")]
fn burned(clock: Clock) -> Option<Duration> {
    #[expect(
        unsafe_code,
        reason = "`clock_gettime` on a thread's CPU clock has no `std` spelling. The \
                  out-parameter is a stack `timespec` this call owns exclusively, the struct is \
                  plain data with no invalid bit pattern, and a clock id whose thread has ended \
                  is refused by the kernel rather than read."
    )]
    let spent = unsafe {
        let mut spent: libc::timespec = std::mem::zeroed();
        (libc::clock_gettime(clock, &raw mut spent) == 0).then_some(spent)
    }?;
    Some(Duration::new(
        u64::try_from(spent.tv_sec).ok()?,
        u32::try_from(spent.tv_nsec).ok()?,
    ))
}

#[cfg(windows)]
type Clock = u32;

#[cfg(windows)]
fn current() -> Option<Clock> {
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;

    #[expect(
        unsafe_code,
        reason = "`GetCurrentThreadId` reads the caller's own thread id and cannot fail. It takes \
                  no argument, touches no memory and returns a `u32`."
    )]
    let id = unsafe { GetCurrentThreadId() };
    Some(id)
}

#[cfg(windows)]
fn burned(clock: Clock) -> Option<Duration> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::Threading::{
        GetThreadTimes, OpenThread, THREAD_QUERY_LIMITED_INFORMATION,
    };

    /// A `FILETIME` is a count of 100-nanosecond intervals, in two halves.
    fn hundred_nanos(time: FILETIME) -> u64 {
        u64::from(time.dwHighDateTime) << 32 | u64::from(time.dwLowDateTime)
    }

    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    #[expect(
        unsafe_code,
        reason = "`GetThreadTimes` is the Windows spelling of a thread's own CPU clock and `std` \
                  has none. The handle is opened and closed inside this call for the narrowest \
                  right that answers the question, a thread that has ended is refused rather than \
                  read, and the four out-parameters are stack `FILETIME`s this call owns \
                  exclusively."
    )]
    let spent = unsafe {
        let thread = OpenThread(THREAD_QUERY_LIMITED_INFORMATION, 0, clock);
        if thread.is_null() {
            return None;
        }
        let mut created = FILETIME::default();
        let mut exited = FILETIME::default();
        let read = GetThreadTimes(
            thread,
            &raw mut created,
            &raw mut exited,
            &raw mut kernel,
            &raw mut user,
        ) != 0;
        CloseHandle(thread);
        read
    };
    if !spent {
        return None;
    }
    Some(Duration::from_nanos(
        (hundred_nanos(kernel) + hundred_nanos(user)).saturating_mul(100),
    ))
}

#[cfg(not(any(target_os = "linux", windows)))]
type Clock = ();

#[cfg(not(any(target_os = "linux", windows)))]
fn current() -> Option<Clock> {
    None
}

#[cfg(not(any(target_os = "linux", windows)))]
fn burned(_clock: Clock) -> Option<Duration> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property the sampler rests on: burning the CPU moves this clock, and
    /// it moves for the thread that burned it rather than for the reader.
    #[test]
    fn a_clock_read_from_another_thread_follows_the_thread_that_burned_it() {
        let Some(idle) = ThreadClock::current() else {
            // The platform's own answer, and this module's docs say what a
            // caller does with it. Nothing here can be asserted about a clock
            // that does not exist.
            return;
        };
        let before = idle.burned().expect("this thread's own clock did not read");
        /// What the busy thread waits to see charged on its own clock before
        /// it stops: enough that a tick the reader is charged for while it
        /// blocks cannot reach it.
        const BURNED: Duration = Duration::from_millis(50);

        let (clocks, clock) = std::sync::mpsc::channel();
        let (spins, spun) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel::<()>();
        let busy = std::thread::spawn(move || {
            let mine = ThreadClock::current().expect("a second thread had no clock");
            clocks.send(mine).expect("the reader hung up");
            // Spun rather than slept: a sleeping thread accumulates wall time
            // and no CPU time, which is the whole distinction under test.
            burn_at_least(BURNED);
            spins.send(()).expect("the reader hung up");
            // Held alive until the reader has sampled: Windows reopens a thread
            // by id, and a thread that has ended has no clock to reopen.
            released.recv().ok();
        });

        let clock = clock
            .recv()
            .expect("the busy thread never published its clock");
        spun.recv()
            .expect("the busy thread never finished spinning");

        // At least what the busy thread saw itself charged before it stopped:
        // a clock only moves forward, so the reader's figure is a floor and
        // not a number to pin.
        let burned = clock.burned().expect("a live thread read as no thread");
        assert!(
            burned >= BURNED,
            "a thread charged {BURNED:?} on its own clock read as {burned:?} from another"
        );

        // And the reader, which blocked on a channel throughout, is not charged
        // for what the other thread spent.
        let after = idle.burned().expect("this thread's own clock did not read");
        assert!(
            after.saturating_sub(before) < burned,
            "the reading thread was charged the busy thread's CPU time"
        );

        release.send(()).ok();
        busy.join().expect("the busy thread panicked");
    }

    /// The other half of the platform's answer, and the one an operator reads:
    /// a host with no clock announces the ceiling it is not enforcing, and a
    /// host with one says nothing rather than warning about a key that works.
    #[test]
    fn a_platform_without_a_thread_clock_reports_no_cpu_ceiling_at_boot() {
        match ThreadClock::current() {
            Some(_) => assert!(
                no_ceiling_note().is_none(),
                "a platform that enforces the ceiling announced that it does not"
            ),
            None => {
                let note =
                    no_ceiling_note().expect("a platform with no clock announced no such thing");
                assert!(
                    note.contains("cpu_time"),
                    "the note does not name the key it is about"
                );
            }
        }
    }
}
