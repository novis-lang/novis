//! One thread's CPU time, sampled by a thread that is not it.
//!
//! The sampler runs on [`crate::watchdog`]'s thread — one thread for the process, already waking on
//! an interval and already holding an entry per core. That thread is not the
//! one burning the CPU it has to measure, so it needs a handle on somebody
//! else's clock, and this module is the platform half of that: a core takes a
//! [`ThreadClock`] on **itself**, and any thread holding it can read what that
//! core has spent. The core reads the same clock itself at each task switch
//! that moves its time from one request to another, so the two readers share
//! one measure.
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
//! be owned, closed and taught to be `Send`. A sample from another thread is
//! one `OpenThread` and one `CloseHandle` per core per watchdog interval, on a
//! thread that is already awake — the price of holding no operating-system
//! resource between sweeps. A thread reading its own clock, as a core does at a
//! task switch, uses its pseudo-handle and makes the one call.
//!
//! `GetThreadTimes` moves in 15.625 ms steps: the kernel charges each clock
//! interrupt's whole interval to the thread running at that moment. Its finer
//! neighbour, `QueryThreadCycleTime`, is not used, because of what a read of a
//! running thread costs. A probe on two of CI's Windows runners (AMD EPYC 7763
//! and Intel Xeon 8573C, four vCPUs) measured, as AMD / Intel:
//!
//! * own-thread steps of 0.4 / 0.3 µs at the 99th percentile, and a read from
//!   another thread that never trailed the thread's own;
//! * cycles over a 2 s spin within 1.1% of `GetThreadTimes`, converted at a
//!   rate from `_rdtsc` and `QueryPerformanceCounter` pairs 1 ms apart;
//! * an own-thread read at 1.18 / 1.13 times `GetThreadTimes`'s cost;
//! * a read of another thread at 1.0 times while it is parked, and 7.5 / 10.3
//!   times, about 10 µs, while it runs. A suspended read costs 25 to 37 µs.
//!
//! The watchdog reads cores while they run a request, so that last figure is
//! the one paid. The likely cause is that Windows interrupts the running core
//! to read its count, which would cost the request path as well.
//! `rule:errors/on-limit` records it as declined.
//!
//! macOS has no `pthread_getcpuclockid`, and its `CLOCK_THREAD_CPUTIME_ID` only
//! reads the caller's own thread, so it answers with the thread's mach port
//! name from `pthread_mach_thread_np`. Any thread in the process may pass that
//! name to `thread_info(THREAD_BASIC_INFO)`, whose user and system times are
//! the thread's CPU time. The name is plain data: the send right behind it
//! belongs to `libpthread` for as long as the thread lives, so this type holds
//! nothing, and a thread that has ended is refused by the kernel rather than
//! read.
//!
//! The kernel brings those times up to date when the thread stops running,
//! so a thread that has run without a break reads as of when it started: a
//! sampler would stop a request that much past its limit. So a read of
//! another thread suspends it with `thread_suspend`, which returns once it is
//! off its processor, reads it and resumes it, one reader at a time. A read of
//! the caller's own thread is `clock_gettime(CLOCK_THREAD_CPUTIME_ID)`, which
//! counts the run in progress.
//!
//! Request path, on macOS alone: each sampler read of a core under a CPU
//! ceiling stops that core for the two context switches a suspend and a resume
//! take, tens of microseconds, once per watchdog sweep. Memory: nothing.
//!
//! Every other platform is the `None` arm, which is the case the section above
//! describes.
//!
//! # A switch reads the clock with a system call
//!
//! A task switch that leaves or enters a published request tree reads this
//! clock once (`crate::watchdog`'s *Charging a request for the time it runs*),
//! which is five reads for a JSON request. Each read is a system call on every
//! platform here, and that is a stated bound, not a gap: no cheaper clock
//! charges every switch to the request it ran for.
//!
//! * Linux's vDSO answers the wall clocks without entering the kernel, and
//!   never a thread's CPU clock. `perf_event_open`'s task clock can be read from
//!   a mapped page with no system call, but Docker's default seccomp profile and
//!   the `perf_event_paranoid` default of Debian and Ubuntu kernels refuse it to
//!   an unprivileged process. That is where a server runs, so the fast path
//!   would mostly be the fallback, and the clock `[limits] cpu_time` is enforced
//!   on would have a second implementation CI does not run.
//! * A wall clock between two switches is cheap and charges a request for the
//!   time the operating system ran other processes, so a busy host would stop
//!   requests that spent nothing.
//! * Leaving the tree running across the core's wait for events saves the two
//!   reads around it, and charges the reactor's work for other requests to the
//!   one that ran last.
//!
//! Request path: one system call per switch between trees, which is what
//! request isolation costs here, and nothing between two tasks of one tree.

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
/// clock afterwards. Beside the other test binaries `nv verify` runs at
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
        GetCurrentThread, GetCurrentThreadId, GetThreadTimes, OpenThread,
        THREAD_QUERY_LIMITED_INFORMATION,
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
                  has none. The caller's own thread is read through its pseudo-handle, which \
                  needs no closing; any other is opened and closed inside this call for the \
                  narrowest right that answers the question, and a thread that has ended is \
                  refused rather than read. The four out-parameters are stack `FILETIME`s this \
                  call owns exclusively."
    )]
    let spent = unsafe {
        let own = GetCurrentThreadId() == clock;
        let thread = if own {
            GetCurrentThread()
        } else {
            OpenThread(THREAD_QUERY_LIMITED_INFORMATION, 0, clock)
        };
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
        if !own {
            CloseHandle(thread);
        }
        read
    };
    if !spent {
        return None;
    }
    Some(Duration::from_nanos(
        (hundred_nanos(kernel) + hundred_nanos(user)).saturating_mul(100),
    ))
}

#[cfg(target_os = "macos")]
type Clock = libc::mach_port_t;

#[cfg(target_os = "macos")]
fn current() -> Option<Clock> {
    #[expect(
        unsafe_code,
        reason = "`pthread_mach_thread_np` is the only way to name a thread's own CPU clock in a \
                  form another thread may read on macOS, and `std` has no equivalent. The thread \
                  argument is the caller's own, the call cannot fail on a live thread, and the \
                  port name it returns is plain data whose send right `libpthread` keeps."
    )]
    let port = unsafe { libc::pthread_mach_thread_np(libc::pthread_self()) };
    // `MACH_PORT_NULL` names no thread, and a clock on no thread is no clock.
    (port != 0).then_some(port)
}

#[cfg(target_os = "macos")]
#[expect(
    unsafe_code,
    reason = "`thread_suspend` and `thread_resume` are the mach calls that stop and restart a \
              thread of this task, and neither `std` nor `libc` declares them. Both take a port \
              name and return a `kern_return_t`, as `<mach/thread_act.h>` declares them."
)]
unsafe extern "C" {
    fn thread_suspend(target: libc::thread_act_t) -> libc::kern_return_t;
    fn thread_resume(target: libc::thread_act_t) -> libc::kern_return_t;
}

/// Held across one suspended read, so no two threads suspend each other at
/// once: each would wait for the other to resume it.
#[cfg(target_os = "macos")]
static SUSPENDING: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(target_os = "macos")]
fn burned(clock: Clock) -> Option<Duration> {
    if current() == Some(clock) {
        return own_burned();
    }
    let _one_reader = SUSPENDING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    #[expect(
        unsafe_code,
        reason = "`thread_suspend` on a port name of this task stops that thread and returns once \
                  it is off its processor, and the matching `thread_resume` below restarts it. \
                  The thread named is never the caller, checked above, and nothing between the \
                  two calls takes a lock or allocates, so it cannot wait on the stopped thread. \
                  A thread that has ended is refused by the kernel rather than stopped."
    )]
    let held = unsafe { thread_suspend(clock) } == libc::KERN_SUCCESS;
    let read = basic_info_burned(clock);
    if held {
        #[expect(
            unsafe_code,
            reason = "Restarts the thread the `thread_suspend` above stopped, once, on the same \
                      port name. A thread that ended in between is refused, and has nothing to \
                      restart."
        )]
        unsafe {
            thread_resume(clock);
        }
    }
    read
}

/// The calling thread's own CPU time, from `clock_gettime`. For the caller's
/// own thread it counts the time since the thread last started running, which
/// `thread_info` leaves out.
#[cfg(target_os = "macos")]
fn own_burned() -> Option<Duration> {
    #[expect(
        unsafe_code,
        reason = "`clock_gettime` on the caller's own CPU clock has no `std` spelling. The \
                  out-parameter is a stack `timespec` this call owns exclusively, plain data \
                  with no invalid bit pattern."
    )]
    let spent = unsafe {
        let mut spent: libc::timespec = std::mem::zeroed();
        (libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &raw mut spent) == 0).then_some(spent)
    }?;
    Some(Duration::new(
        u64::try_from(spent.tv_sec).ok()?,
        u32::try_from(spent.tv_nsec).ok()?,
    ))
}

/// What `thread_info(THREAD_BASIC_INFO)` reports for the thread `clock` names.
#[cfg(target_os = "macos")]
fn basic_info_burned(clock: Clock) -> Option<Duration> {
    /// A mach `time_value_t`: whole seconds and the microseconds past them.
    fn micros(time: libc::time_value_t) -> Option<Duration> {
        Some(
            Duration::from_secs(u64::try_from(time.seconds).ok()?)
                + Duration::from_micros(u64::try_from(time.microseconds).ok()?),
        )
    }

    let flavor = libc::thread_flavor_t::try_from(libc::THREAD_BASIC_INFO).ok()?;
    let mut count = libc::THREAD_BASIC_INFO_COUNT;
    #[expect(
        unsafe_code,
        reason = "`thread_info` is the macOS spelling of another thread's CPU clock and `std` has \
                  none. The out-parameter is a stack `thread_basic_info` this call owns \
                  exclusively, plain integers with no invalid bit pattern, and `count` tells the \
                  kernel its exact size in `integer_t`s. A port name whose thread has ended is \
                  refused by the kernel rather than read."
    )]
    let info = unsafe {
        let mut info: libc::thread_basic_info = std::mem::zeroed();
        let read = libc::thread_info(
            clock,
            flavor,
            (&raw mut info).cast::<libc::integer_t>(),
            &raw mut count,
        ) == libc::KERN_SUCCESS;
        read.then_some(info)
    }?;
    // A reply shorter than the struct left part of it unwritten, and a zero
    // read from there would charge the thread less than it spent.
    if count != libc::THREAD_BASIC_INFO_COUNT {
        return None;
    }
    Some(micros(info.user_time)? + micros(info.system_time)?)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
type Clock = ();

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn current() -> Option<Clock> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn burned(_clock: Clock) -> Option<Duration> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The platforms this module has a clock arm for. On each of them a missing
    /// clock is a request that loops forever and is never stopped, so the cases
    /// below fail there rather than skip.
    const CLOCKED: bool = cfg!(any(target_os = "linux", target_os = "macos", windows));

    /// Every Unix this module has an arm for names the calling thread's clock,
    /// and that clock moves while the thread spins. `burn_at_least` panics if a
    /// minute of spinning is charged less than it waits for, so returning is
    /// the proof.
    #[cfg(unix)]
    #[test]
    fn a_busy_thread_on_a_clocked_unix_is_charged_for_what_it_spins() {
        if !CLOCKED {
            assert!(
                no_ceiling_note().is_some(),
                "a Unix with no clock arm announced no missing ceiling"
            );
            return;
        }
        let clock = ThreadClock::current().expect("a clocked Unix answered `None`");
        let before = clock
            .burned()
            .expect("this thread's own clock did not read");
        burn_at_least(Duration::from_millis(30));
        let after = clock
            .burned()
            .expect("this thread's own clock did not read");
        assert!(
            after >= before + Duration::from_millis(30),
            "a thread charged 30ms by its own clock read {:?} across the spin",
            after.saturating_sub(before)
        );
    }

    /// The property the sampler rests on: burning the CPU moves this clock, and
    /// it moves for the thread that burned it rather than for the reader.
    #[test]
    fn a_clock_read_from_another_thread_follows_the_thread_that_burned_it() {
        let Some(idle) = ThreadClock::current() else {
            // The platform's own answer, and this module's docs say what a
            // caller does with it. Nothing here can be asserted about a clock
            // that does not exist, and a platform this module gives a clock
            // to may not skip the case by losing it.
            if CLOCKED {
                panic!(
                    "a platform with a thread clock answered `None` and would enforce no ceiling"
                );
            }
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

    /// The property the sampler stops a request on: a thread read from another
    /// **while it is still running** is charged at least what it had already
    /// read on its own clock. A clock brought up to date only when its thread
    /// stops running reads short by however long the thread has run since, and
    /// the sampler would stop a request that much past its limit.
    ///
    /// Waits on the busy thread's own clock, never on a wall interval, so a
    /// loaded runner slows it and cannot fail it. The worst lag is written to
    /// stderr past the test harness's capture, so a CI log shows each
    /// platform's figure on a green run too.
    #[test]
    fn a_running_thread_read_from_another_is_charged_what_it_has_seen_itself() {
        read_a_running_thread("spins", || {
            for _ in 0..4096 {
                std::hint::spin_loop();
            }
        });
    }

    /// The same property for a thread that spends much of its run in the
    /// kernel, as `Core\Compress::compress` does: it grows a 16 MiB buffer,
    /// copies it and frees both, so most of its time is page faults and
    /// unmapping. A clock that counted kernel time on one side only, or a
    /// suspend that waited for the thread to leave a long system call, would
    /// stop such a request past its limit while a spinning one stops on time.
    #[test]
    fn a_running_thread_in_the_kernel_read_from_another_is_charged_what_it_has_seen_itself() {
        read_a_running_thread("grows and frees 16 MiB buffers", || {
            // One growth step, as a reader's chunk is.
            let chunk = vec![7_u8; 64 << 10];
            let mut grown = Vec::new();
            while grown.len() < 16 << 20 {
                grown.extend_from_slice(&chunk);
            }
            let copy = grown.clone();
            std::hint::black_box((grown, copy));
        });
    }

    /// Reads a thread that runs `work` in a loop from another thread, once per
    /// millisecond of the busy thread's own clock, and checks each read is at
    /// least what the busy thread had already seen, within `LAG`. Each read
    /// is also timed, since the sampler waits for it under its lock, and the
    /// slowest is printed with the lag to stderr past the capture, for every
    /// platform's CI log. The time is wall time on a loaded runner, so it is a
    /// figure and no bound: CI's macOS runner took 4 to 72 ms for one read.
    fn read_a_running_thread(doing: &str, work: fn()) {
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

        /// How long the busy thread runs without a break, on its own clock:
        /// longer than any interval a scheduler tick or a sweep would hide.
        const SPIN: Duration = Duration::from_millis(300);
        /// How far the reader may read below what the busy thread had already
        /// seen: a clock rounded to microseconds, and nothing like a run.
        const LAG: Duration = Duration::from_millis(5);
        const PATIENCE: Duration = Duration::from_secs(60);

        if ThreadClock::current().is_none() {
            if CLOCKED {
                panic!("a platform with a thread clock answered `None`");
            }
            return;
        }
        let seen = std::sync::Arc::new(AtomicU64::new(0));
        let done = std::sync::Arc::new(AtomicBool::new(false));
        let (clocks, clock) = std::sync::mpsc::channel();
        let busy = std::thread::spawn({
            let seen = std::sync::Arc::clone(&seen);
            let done = std::sync::Arc::clone(&done);
            move || {
                let mine = ThreadClock::current().expect("a second thread had no clock");
                clocks.send(mine).expect("the reader hung up");
                // Runs until the reader has finished, so every read below is
                // of a thread that is running, never of one that has stopped.
                while !done.load(Ordering::Acquire) {
                    let now = mine.burned().expect("this thread's own clock did not read");
                    let now = u64::try_from(now.as_nanos()).unwrap_or(u64::MAX);
                    seen.store(now, Ordering::Release);
                    work();
                }
            }
        });

        let clock = clock
            .recv()
            .expect("the busy thread never published its clock");
        let give_up_at = std::time::Instant::now() + PATIENCE;
        let (mut worst, mut reads, mut last) = (Duration::ZERO, 0_u32, 0_u64);
        let mut slowest = Duration::ZERO;
        loop {
            let own = seen.load(Ordering::Acquire);
            // One read per millisecond the busy thread has been charged, so the
            // reads spread over the whole run.
            if own < last + 1_000_000 {
                assert!(
                    std::time::Instant::now() < give_up_at,
                    "the busy thread was charged {own} ns in {PATIENCE:?} of wall time"
                );
                std::hint::spin_loop();
                continue;
            }
            last = own;
            let started = std::time::Instant::now();
            let read = clock.burned().expect("a running thread read as no thread");
            slowest = slowest.max(started.elapsed());
            worst = worst.max(Duration::from_nanos(own).saturating_sub(read));
            reads += 1;
            if Duration::from_nanos(own) >= SPIN {
                break;
            }
        }
        done.store(true, Ordering::Release);
        busy.join().expect("the busy thread panicked");

        let line = format!(
            "cpuclock: {reads} reads of a running thread that {doing}, from another: worst lag \
             {worst:?} behind its own clock, slowest read {slowest:?}\n"
        );
        std::io::Write::write_all(&mut std::io::stderr(), line.as_bytes()).ok();
        assert!(
            worst <= LAG,
            "a running thread that {doing}, read from another, lagged {worst:?} behind its own \
             clock over {reads} reads, past the {LAG:?} a rounded clock explains"
        );
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
