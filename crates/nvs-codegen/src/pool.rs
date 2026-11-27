//! The threads a compile generates machine code on, and the priority they run at.
//!
//! [`crate::UnitBuilder::compile_all`] emits each function's Cranelift IR on the
//! compiling thread, because the emit writes the unit's literals and
//! declarations. Cranelift's compile of that IR (optimisation, register
//! allocation, emission) reads nothing shared, so it runs here, on
//! [`threads`] scoped threads. The compiling thread defines each result into the
//! module in the program's function order, so the object is byte for byte the
//! one a single thread writes.
//!
//! **Below the request workers.** Each thread lowers its own scheduling
//! priority before its first job ([`lower_current_thread_priority`]): a
//! server's request workers keep their CPU while a reload compiles, and a
//! compile on an idle machine still gets every core. Lowering is best-effort, as
//! pinning a worker is (`nvs_host::affinity`): a refusal makes a compile compete
//! for CPU, never makes it wrong.
//!
//! **Cost:** the threads are started per compile and joined before it returns,
//! so an idle server holds none. At most two jobs per thread are in flight, each
//! one function's Cranelift context, and they are freed with the pool. There is
//! no setting: [`std::thread::available_parallelism`] already respects a
//! container's CPU quota. [`with_compile_threads`] fixes the count for a test.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::{Mutex, PoisonError};

thread_local! {
    /// The thread count [`with_compile_threads`] fixed for this thread's
    /// compiles, or `None` for every core.
    static FIXED_THREADS: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Runs `f` with every compile it starts on this thread using `threads`
/// threads for code generation, at least one. A test compares the bytes of
/// one thread and of many with it; nothing else calls it.
#[doc(hidden)]
pub fn with_compile_threads<R>(threads: usize, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<usize>);
    impl Drop for Restore {
        fn drop(&mut self) {
            FIXED_THREADS.with(|fixed| fixed.set(self.0));
        }
    }
    let _restore = Restore(FIXED_THREADS.with(|fixed| fixed.replace(Some(threads.max(1)))));
    f()
}

/// How many threads a compile of `jobs` functions generates code on: every
/// core the process may use, and never more threads than functions.
pub(crate) fn threads(jobs: usize) -> usize {
    let cores = FIXED_THREADS.with(Cell::get).unwrap_or_else(|| {
        std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
    });
    cores.min(jobs).max(1)
}

/// The jobs one [`run`] has started and not yet returned, and the results that
/// came back before the ones in front of them.
pub(crate) struct Lanes<J, R> {
    jobs: Option<mpsc::SyncSender<(usize, J)>>,
    results: mpsc::Receiver<(usize, std::thread::Result<R>)>,
    in_flight: usize,
    cap: usize,
    early: BTreeMap<usize, R>,
}

impl<J, R> Lanes<J, R> {
    /// Whether the pool has its bound of jobs that are running, or done and not
    /// yet taken. The caller takes the oldest one before it submits another,
    /// so the contexts in flight never grow with the program.
    pub(crate) fn full(&self) -> bool {
        self.in_flight + self.early.len() >= self.cap
    }

    /// Hands job `index` to a thread.
    pub(crate) fn submit(&mut self, index: usize, job: J) {
        if let Some(jobs) = &self.jobs {
            // A send fails only when every thread has ended, and a thread ends
            // only when this sender is dropped.
            if jobs.send((index, job)).is_ok() {
                self.in_flight += 1;
            }
        }
    }

    /// The result of job `index`, waiting for it if it is still running, or
    /// `None` when no such job was submitted.
    pub(crate) fn take(&mut self, index: usize) -> Option<R> {
        loop {
            if let Some(result) = self.early.remove(&index) {
                return Some(result);
            }
            if self.in_flight == 0 {
                return None;
            }
            self.wait_one();
        }
    }

    fn wait_one(&mut self) {
        if let Ok((done, result)) = self.results.recv() {
            self.keep(done, result);
        }
    }

    fn keep(&mut self, index: usize, result: std::thread::Result<R>) {
        self.in_flight -= 1;
        match result {
            Ok(result) => {
                self.early.insert(index, result);
            }
            // A panic in a job is a panic of the compile, on the thread that
            // started it, exactly as a single-threaded compile panics.
            Err(payload) => panic::resume_unwind(payload),
        }
    }
}

/// Starts `threads` scoped threads that each run `work` on the jobs `drive`
/// submits, and returns what `drive` returns once every thread has ended.
///
/// Each thread lowers its own priority first. A job that panics resumes the
/// panic on the calling thread. When `drive` returns early, the jobs still in
/// flight finish and their results are dropped.
pub(crate) fn run<J, R, T>(
    threads: usize,
    work: impl Fn(J) -> R + Sync,
    drive: impl FnOnce(&mut Lanes<J, R>) -> T,
) -> T
where
    J: Send,
    R: Send,
{
    let threads = threads.max(1);
    let cap = threads * 2;
    let (job_tx, job_rx) = mpsc::sync_channel::<(usize, J)>(cap);
    let job_rx = Mutex::new(job_rx);
    let (result_tx, result_rx) = mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let result_tx = result_tx.clone();
            let (job_rx, work) = (&job_rx, &work);
            scope.spawn(move || {
                lower_current_thread_priority();
                loop {
                    let next = job_rx.lock().unwrap_or_else(PoisonError::into_inner).recv();
                    let Ok((index, job)) = next else {
                        return;
                    };
                    let result = panic::catch_unwind(AssertUnwindSafe(|| work(job)));
                    if result_tx.send((index, result)).is_err() {
                        return;
                    }
                }
            });
        }
        drop(result_tx);
        let mut lanes = Lanes {
            jobs: Some(job_tx),
            results: result_rx,
            in_flight: 0,
            cap,
            early: BTreeMap::new(),
        };
        let out = drive(&mut lanes);
        // Ends every thread once it finishes the job it is on.
        lanes.jobs = None;
        out
    })
}

/// The scheduling priority of every thread a compile's pool starts, read on
/// each of `threads` threads after it lowered its own, in this crate's scale
/// ([`current_thread_priority`]). A test holds each below the caller's.
#[doc(hidden)]
#[must_use]
pub fn compile_thread_priorities(threads: usize) -> Vec<i32> {
    run(
        threads,
        |()| current_thread_priority(),
        |lanes| {
            for index in 0..threads {
                lanes.submit(index, ());
            }
            (0..threads).filter_map(|index| lanes.take(index)).collect()
        },
    )
}

/// The calling thread's scheduling priority, where a larger number runs
/// first: the Windows thread priority, the Linux nice value negated, and the
/// macOS quality-of-service class, with an unset class read as the default.
/// It is `0` on any other platform.
#[doc(hidden)]
#[must_use]
pub fn current_thread_priority() -> i32 {
    if cfg!(miri) {
        return 0;
    }
    platform::priority()
}

/// Lowers the calling thread's scheduling priority below the one a request
/// worker runs at, and returns whether the system accepted it.
fn lower_current_thread_priority() -> bool {
    if cfg!(miri) {
        return false;
    }
    platform::lower()
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::System::Threading::{
        GetCurrentThread, GetThreadPriority, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
    };

    pub(super) fn lower() -> bool {
        #[expect(
            unsafe_code,
            reason = "a Win32 call on the calling thread's own pseudo-handle"
        )]
        // SAFETY: `GetCurrentThread` returns a pseudo-handle that is always
        // valid for the calling thread and needs no close.
        unsafe {
            SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) != 0
        }
    }

    pub(super) fn priority() -> i32 {
        #[expect(
            unsafe_code,
            reason = "a Win32 call on the calling thread's own pseudo-handle"
        )]
        // SAFETY: as in `lower`.
        unsafe {
            GetThreadPriority(GetCurrentThread())
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    /// How far a compile thread's nice value is raised above the caller's.
    /// `nice` without an argument uses the same step.
    const STEP: i32 = 10;

    fn thread_id() -> libc::id_t {
        #[expect(
            unsafe_code,
            reason = "the `gettid` system call, which takes no argument"
        )]
        // SAFETY: `gettid` reads the calling thread's id and cannot fail.
        let tid = unsafe { libc::syscall(libc::SYS_gettid) };
        libc::id_t::try_from(tid).unwrap_or(0)
    }

    fn nice(tid: libc::id_t) -> i32 {
        #[expect(unsafe_code, reason = "`getpriority` on the calling thread's own id")]
        // SAFETY: the call reads one integer for the calling thread.
        unsafe {
            libc::getpriority(libc::PRIO_PROCESS, tid)
        }
    }

    pub(super) fn lower() -> bool {
        let tid = thread_id();
        let target = (nice(tid) + STEP).min(19);
        #[expect(unsafe_code, reason = "`setpriority` on the calling thread's own id")]
        // SAFETY: the call writes one integer for the calling thread. Raising
        // a nice value needs no privilege.
        unsafe {
            libc::setpriority(libc::PRIO_PROCESS, tid, target) == 0
        }
    }

    pub(super) fn priority() -> i32 {
        -nice(thread_id())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use libc::qos_class_t;

    pub(super) fn lower() -> bool {
        #[expect(unsafe_code, reason = "a QoS call on the calling thread")]
        // SAFETY: the call changes only the calling thread's own class.
        unsafe {
            libc::pthread_set_qos_class_self_np(qos_class_t::QOS_CLASS_UTILITY, 0) == 0
        }
    }

    pub(super) fn priority() -> i32 {
        let mut class = qos_class_t::QOS_CLASS_UNSPECIFIED;
        let mut relative = 0;
        #[expect(unsafe_code, reason = "a QoS read of the calling thread")]
        // SAFETY: both out-parameters are locals that outlive the call.
        let status = unsafe {
            libc::pthread_get_qos_class_np(libc::pthread_self(), &raw mut class, &raw mut relative)
        };
        if status != 0 {
            return 0;
        }
        let class = match class {
            qos_class_t::QOS_CLASS_UNSPECIFIED => qos_class_t::QOS_CLASS_DEFAULT,
            other => other,
        };
        i32::try_from(class as u32).unwrap_or(0)
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod platform {
    pub(super) fn lower() -> bool {
        false
    }

    pub(super) fn priority() -> i32 {
        0
    }
}
