//! The blocking pool: where a call that has no readiness to wait on runs, so
//! that it runs somewhere other than on a core.
//!
//! `rule:http-server/a-core-is-never-blocked-on-a-syscall`
//! is this module's specification and its one home: filesystem calls, name
//! resolution and waiting on a child process go here, the reactor thread issues
//! no call that can block on external state, and the pool is **bounded at twice
//! the core count**. A socket read is not one of these — [`crate::net`] parks on
//! readiness and hands the core back, which is the cheaper mechanism and the one
//! to reach for whenever the kernel can report the wait.
//!
//! # What a handoff is, end to end
//!
//! [`run`] is the whole of it. On the core: take a [`RemoteWake`](reactor::RemoteWake) for the
//! running task, box the work with that handle and a slot for its answer, hand
//! it to this thread's pool, and park. Off the core: run it, put the outcome in
//! the slot, and drop the handle, which wakes the task. Back on the core: take
//! the answer out of the slot and return it — or park again, because a wake is
//! always a hint ([`crate::reactor`]'s rule 2), and here that means *the slot is
//! the record and the wake only ends the wait*.
//!
//! **Off a core, [`run`] simply calls the function.** There is no core to hand
//! back, so there is nothing to protect and no reason to pay a thread handoff —
//! the same answer [`crate::timer::park_until`] gives a sleep, for the same
//! reason.
//!
//! # The bound, and what the pool spends
//!
//! [`bound`] threads, lazily: a thread is started only when work arrives and
//! every existing one is busy, so the ordinary steady state of a worker doing no
//! blocking work is **no threads at all**. What `rule:programs/memory-priority` asks to be said out
//! loud: at most [`bound`] OS thread stacks per worker, reserved by the platform
//! and resident only in what a job touches, plus one boxed closure per job in
//! flight. That is O(cores) and O(in-flight); nothing here grows with requests
//! served, which is the property that makes an unbounded pool the wrong default
//! rather than a generous one.
//!
//! A pool is **per worker** and its threads are its own. That is the ADR's word,
//! and it is also what keeps this crate shared-nothing: a queue shared between
//! cores would be one piece of contended state on the request path, and the
//! [`RemoteWake`](reactor::RemoteWake) a job carries is already per-core because the reactor it pokes
//! is.
//!
//! # A job that panics does not shrink the pool
//!
//! Two containments, and they are not the same one. [`run`] catches the panic of
//! the function it was given and carries it back to the *task's own stack*,
//! where it resumes and meets the containment boundary `rule:http-server/containment-does-not-end-at-the-helper` put at the
//! task root — a panicking `Core\IO` call fails one request, exactly as it
//! would have on the core. The pool thread catches anything that still escapes a
//! job, because a thread lost to an unwind is a thread the bound above no longer
//! accounts for. In both cases the [`RemoteWake`](reactor::RemoteWake) fires from its own `Drop`, so
//! a task is never left parked on an answer that has stopped coming.

use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use crate::affinity::cpus;
use crate::reactor;
use crate::scheduler::{Waiting, current_task, suspend_current};

/// A unit of work handed off the core.
type Job = Box<dyn FnOnce() + Send + 'static>;

/// How many threads one worker's pool may start.
///
/// **Twice the core count**, which is `rule:http-server/a-core-is-never-blocked-on-a-syscall`'s number and its reasoning:
/// blocking work is waiting rather than computing, so a core may have several
/// calls outstanding at once, and the factor is what lets it — while the *bound*
/// is what keeps a workload's blocking fan-out from turning into a thread count
/// nothing accounts for. One when the platform will not say how many CPUs there
/// are, matching [`cpus`]'s own answer for that case.
#[must_use]
pub fn bound() -> usize {
    cpus().len().max(1) * 2
}

/// What the pool's threads and their feeder share.
struct Shared {
    queue: Mutex<Queue>,
    /// Signalled for each job pushed and broadcast once at shutdown.
    work: Condvar,
}

/// The queue itself, and the state that decides whether a new thread is started
/// for a job or an existing one is woken for it.
struct Queue {
    jobs: VecDeque<Job>,
    /// How many threads are parked on [`Shared::work`] waiting for a job. A
    /// count rather than a set: nothing here needs to name an idle thread, only
    /// to know whether there is one, and a job pushed with none is the whole
    /// test for starting another.
    idle: usize,
    /// Set once, by [`BlockingPool::drop`]. A thread that sees it with an empty
    /// queue returns rather than waiting again.
    shutdown: bool,
}

/// One worker's supply of threads for calls that cannot be waited on.
///
/// Created on the worker's thread and dropped with it — see this module's docs
/// for why a pool is per worker rather than per process. Dropping it tells every
/// thread to stop and does **not** wait for them; [`BlockingPool::drop`] owns
/// the reason, which is a real deadlock and not a preference.
pub struct BlockingPool {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
    bound: usize,
}

impl std::fmt::Debug for BlockingPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let queued = self.queued();
        f.debug_struct("BlockingPool")
            .field("threads", &self.threads.len())
            .field("bound", &self.bound)
            .field("queued", &queued)
            .finish()
    }
}

impl Default for BlockingPool {
    fn default() -> Self {
        Self::new(bound())
    }
}

impl BlockingPool {
    /// An empty pool that will start at most `bound` threads.
    ///
    /// No thread is started here: the first one arrives with the first job, so
    /// a worker that never blocks never pays for this at all. `bound` is
    /// clamped up to one, because a pool that may start no threads is a queue
    /// nothing drains.
    #[must_use]
    pub fn new(bound: usize) -> Self {
        Self {
            shared: Arc::new(Shared {
                queue: Mutex::new(Queue {
                    jobs: VecDeque::new(),
                    idle: 0,
                    shutdown: false,
                }),
                work: Condvar::new(),
            }),
            threads: Vec::new(),
            bound: bound.max(1),
        }
    }

    /// Hands `job` to the pool, starting a thread for it if every existing one
    /// is busy and the bound allows another.
    ///
    /// At the bound the job waits in the queue instead, which is the bound
    /// doing its job rather than a failure: the alternative — refusing the work
    /// — would put an error on every filesystem call that means nothing but
    /// "the machine is busy", and the alternative to *that* is the unbounded
    /// pool `rule:http-server/a-core-is-never-blocked-on-a-syscall` rejects.
    pub fn submit<F>(&mut self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut queue = lock(&self.shared.queue);
        queue.jobs.push_back(Box::new(job));
        let start_one = queue.idle == 0 && self.threads.len() < self.bound;
        drop(queue);

        if start_one {
            self.start_thread();
        } else {
            // One job, one wake: `notify_all` here would wake every idle thread
            // to have all but one of them find the queue empty and park again.
            self.shared.work.notify_one();
        }
    }

    /// How many threads this pool has started.
    #[must_use]
    pub fn threads(&self) -> usize {
        self.threads.len()
    }

    /// The most threads this pool may start.
    #[must_use]
    pub fn bound(&self) -> usize {
        self.bound
    }

    /// How many jobs are waiting for a thread.
    #[must_use]
    pub fn queued(&self) -> usize {
        lock(&self.shared.queue).jobs.len()
    }

    /// Starts one thread and adds it to the pool.
    ///
    /// A thread that the OS refuses is not an error the caller can do anything
    /// with — the job is already queued and whatever threads exist will reach
    /// it — so a refusal leaves the pool one thread smaller and the queue
    /// draining more slowly, which is the same shape as being at the bound.
    fn start_thread(&mut self) {
        let shared = Arc::clone(&self.shared);
        let started = std::thread::Builder::new()
            .name("nvs-blocking".to_owned())
            .spawn(move || run_jobs(&shared));
        if let Ok(handle) = started {
            self.threads.push(handle);
        } else {
            // Nothing is idle by construction here, so this only matters if a
            // thread is about to become idle; the wake costs a syscall at most.
            self.shared.work.notify_one();
        }
    }
}

impl Drop for BlockingPool {
    fn drop(&mut self) {
        {
            let mut queue = lock(&self.shared.queue);
            queue.shutdown = true;
        }
        self.shared.work.notify_all();
        // And **no join**, which is not the obvious choice and is not a
        // shortcut. This pool lives in a thread-local, so its drop runs from a
        // TLS destructor — and on Windows those run under the loader lock,
        // while a thread being joined needs that same lock to run its own
        // destructors and exit. Joining here deadlocks the worker at the one
        // moment it is trying to finish, which is the wedge `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers` is about,
        // arriving from the teardown path rather than from a request.
        //
        // What the threads do instead: each sees `shutdown` the next time it
        // looks for work and returns, and the `Arc` above keeps the queue alive
        // until the last of them is gone. A job still running keeps running —
        // its `RemoteWake` fires into a reactor that may already be closed,
        // which `RemoteWake`'s own docs record as inert rather than unsound.
        self.threads.clear();
    }
}

/// One pool thread's whole life: take a job, run it, wait for the next.
fn run_jobs(shared: &Shared) {
    loop {
        let mut queue = lock(&shared.queue);
        let job = loop {
            if let Some(job) = queue.jobs.pop_front() {
                break job;
            }
            if queue.shutdown {
                return;
            }
            queue.idle += 1;
            queue = shared
                .work
                .wait(queue)
                .unwrap_or_else(PoisonError::into_inner);
            queue.idle -= 1;
        };
        drop(queue);
        // The lock is not held while the job runs — a slow call is the whole
        // reason the pool exists — and a panic escaping a job is
        // swallowed rather than allowed to end this thread, which would shrink
        // the pool below what its bound accounts for. `run` has already turned
        // its own function's panic into a value by this point; what this
        // catches is a job submitted directly.
        let _ = std::panic::catch_unwind(AssertUnwindSafe(job));
    }
}

thread_local! {
    /// This thread's pool, built on first use and dropped with the thread.
    ///
    /// Lazily rather than installed by [`crate::Worker`], because a worker that
    /// never makes a blocking call should not own a queue and a mutex to prove
    /// it, and because the thread-local is what lets [`run`] be reachable from
    /// a `Core` member that was handed nothing but its arguments — the same
    /// route, and the same reasoning, as [`crate::reactor::with_current`].
    static POOL: std::cell::RefCell<Option<BlockingPool>> =
        const { std::cell::RefCell::new(None) };
}

/// Runs `f` against this thread's pool, building one if this is its first job.
///
/// # Panics
///
/// If a borrow is already outstanding on this thread, which means one was held
/// across a suspend point. [`run`] takes it for the submission alone and drops
/// it before it parks, which is the reactor's borrow rule applied to the pool.
fn with_pool<T>(f: impl FnOnce(&mut BlockingPool) -> T) -> T {
    POOL.with_borrow_mut(|slot| f(slot.get_or_insert_with(BlockingPool::default)))
}

/// How many threads this thread's pool has started, and its bound.
///
/// `(0, _)` for a thread that has never made a blocking call, which is what a
/// worker serving only sockets stays at.
#[must_use]
pub fn pool_size() -> (usize, usize) {
    with_pool(|pool| (pool.threads(), pool.bound()))
}

/// Runs `f` off this core and returns what it returned, giving the core back
/// while it runs.
///
/// This is the *only* way a call with no readiness to wait on reaches a thread
/// in this crate — see this module's docs for the handoff end to end, and ADR
/// 0106 § 6 for why there is no third option beside this and parking on
/// readiness.
///
/// Off a core the function is simply called on this thread: there is no core to
/// protect and nothing to hand back.
///
/// # Panics
///
/// If `f` panics, the panic is carried back and resumed on the task's own
/// stack, where the task root's containment boundary meets it and one request
/// fails. It is not swallowed and it does not reach the pool thread's own
/// unwind.
pub fn run<T, F>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let Some(me) = current_task() else {
        return f();
    };
    let Some(wake) = reactor::with_current(|reactor| reactor.remote_wake(me)) else {
        return f();
    };

    let slot: Arc<Mutex<Option<std::thread::Result<T>>>> = Arc::new(Mutex::new(None));
    let answer = Arc::clone(&slot);
    with_pool(|pool| {
        pool.submit(move || {
            let outcome = std::panic::catch_unwind(AssertUnwindSafe(f));
            *lock(&answer) = Some(outcome);
            // Explicit rather than left to the end of the closure: the answer is
            // in the slot *before* the wake goes out, so the task cannot be
            // resumed to find it missing. The drop would deliver anyway, which
            // is what covers the panicking path above.
            drop(wake);
        });
    });

    loop {
        if let Some(outcome) = lock(&slot).take() {
            return match outcome {
                Ok(value) => value,
                Err(panic) => std::panic::resume_unwind(panic),
            };
        }
        // A cancellation is not a way out of this wait: the pool thread is
        // already running the closure and the slot it writes into is on this
        // stack, so the task has to be here when it lands. Ending early is what
        // would be unsound, and a cancelled task dies at its next safepoint
        // once the call it is inside returns.
        if !suspend_current(Waiting::Parked).suspended() {
            // Unreachable in practice: `current_task` answered, so there is a
            // scheduler. Yielding rather than asserting keeps a hypothetical
            // wrong answer here a slow loop instead of a killed worker.
            std::thread::yield_now();
        }
    }
}

/// The lock, with a poisoned one taken anyway.
///
/// Nothing in this module panics while holding one, so poisoning can only be
/// inherited from a job's unwind — and the queue behind the lock is a `VecDeque`
/// of boxes that is no less consistent for it. Refusing the work instead would
/// wedge every later blocking call on one panicking job, which is precisely the
/// tier B failure `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers` exists to prevent.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactor::{Reactor, install, run_until_idle};
    use crate::scheduler::Scheduler;
    use nvs_runtime::{Ctx, OutputSink, TaskRoot};
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    #[test]
    fn the_bound_is_twice_the_core_count() {
        assert_eq!(
            bound(),
            cpus().len().max(1) * 2,
            "`rule:http-server/a-core-is-never-blocked-on-a-syscall`'s number"
        );
        assert!(
            bound() >= 2,
            "a pool that can start no threads drains nothing"
        );
    }

    #[test]
    fn a_pool_starts_no_threads_until_it_is_given_work() {
        let mut pool = BlockingPool::new(4);
        assert_eq!(pool.threads(), 0);

        let (done, ran) = mpsc::channel();
        pool.submit(move || done.send(()).expect("the receiver went away"));
        ran.recv_timeout(Duration::from_secs(5))
            .expect("the job never ran");
        assert_eq!(pool.threads(), 1);
    }

    #[test]
    fn work_past_the_bound_waits_for_a_thread_rather_than_starting_one() {
        let mut pool = BlockingPool::new(1);
        let (started, running) = mpsc::channel();
        let (release, wait) = mpsc::channel::<()>();
        let (done, ran) = mpsc::channel();

        // The one thread this pool may start is held inside the first job, and
        // the test waits until it genuinely is: asserting on the queue while
        // the thread might not have taken its job yet is a race, not a bound.
        pool.submit(move || {
            started.send(()).expect("the receiver went away");
            wait.recv().expect("the sender went away");
        });
        running
            .recv_timeout(Duration::from_secs(5))
            .expect("the first job never started");
        for _ in 0..3 {
            let done = done.clone();
            pool.submit(move || done.send(()).expect("the receiver went away"));
        }
        assert_eq!(pool.threads(), 1, "the bound was exceeded");
        assert_eq!(pool.queued(), 3, "work past the bound did not queue");

        // The same thread now runs all three, which is the other half of the
        // bound: at it, work waits for a thread rather than for a new one.
        release.send(()).expect("the job went away");
        for _ in 0..3 {
            ran.recv_timeout(Duration::from_secs(5))
                .expect("a queued job never ran");
        }
        assert_eq!(pool.threads(), 1);
    }

    #[test]
    fn a_panicking_job_leaves_the_pool_its_thread() {
        // Bound of one, so the second job can only be answered by the thread
        // the first one panicked on. A larger bound would let the pool paper
        // over a lost thread by starting another, which is the thing under
        // test.
        let mut pool = BlockingPool::new(1);
        let (done, ran) = mpsc::channel();

        pool.submit(|| panic!("a job went wrong"));
        pool.submit(move || done.send(()).expect("the receiver went away"));
        ran.recv_timeout(Duration::from_secs(5))
            .expect("the pool lost its thread to a panic");
        assert_eq!(pool.threads(), 1);
    }

    /// The point of the whole module: the blocking call happens somewhere else,
    /// and the core runs its other task while it does.
    #[test]
    fn a_blocking_call_hands_the_core_to_the_neighbour_and_still_answers() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let order = Rc::new(std::cell::RefCell::new(Vec::new()));
        let blocker = Rc::clone(&order);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let answer = run(|| {
                std::thread::sleep(Duration::from_millis(30));
                7_u32
            });
            assert_eq!(answer, 7, "the value did not come back off the pool");
            blocker.borrow_mut().push("blocked");
        });
        let neighbour = Rc::clone(&order);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            neighbour.borrow_mut().push("neighbour");
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.finished, 2);
        assert_eq!(
            *order.borrow(),
            ["neighbour", "blocked"],
            "the call blocked the core instead of parking"
        );
    }

    /// Item 6's two halves in one place: the call reaches the *pool* rather
    /// than the core, and the pool it reaches is `rule:http-server/a-core-is-never-blocked-on-a-syscall`'s — bounded at
    /// twice the core count, per worker, however many calls are in flight. The
    /// jobs overlap by construction rather than by timing: each holds its
    /// thread until the pool is full, so a pool that grew with the fan-out would
    /// be caught here rather than merely being under its bound by luck, and a
    /// loaded machine — the sanitizer leg running beside a release build —
    /// cannot let one job finish before the rest are submitted and bring the
    /// thread count up short.
    #[test]
    fn a_blocking_call_goes_to_a_pool_bounded_at_twice_the_core_count() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        assert_eq!(
            pool_size(),
            (0, bound()),
            "this thread's pool had already started threads"
        );

        // Two more calls than the pool may have threads, so the last two have
        // to wait for a thread rather than make one.
        let full = bound();
        let calls = full + 2;
        let elsewhere = Rc::new(Cell::new(0_usize));
        let here = std::thread::current().id();
        // How many jobs have reached a pool thread. A job waits until the count
        // is the bound, which needs that many threads alive at once; the two
        // extra jobs find the count already there and pass straight through, on
        // a thread one of the first ones freed.
        let running = Arc::new(AtomicUsize::new(0));
        for _ in 0..calls {
            let counted = Rc::clone(&elsewhere);
            let running = Arc::clone(&running);
            sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
                let ran_on = run(move || {
                    running.fetch_add(1, Ordering::SeqCst);
                    while running.load(Ordering::SeqCst) < full {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    std::thread::current().id()
                });
                assert_ne!(ran_on, here, "the blocking call ran on the core");
                counted.set(counted.get() + 1);
            });
        }

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.finished, calls, "a blocking call never came back");
        assert_eq!(elsewhere.get(), calls);

        let (threads, limit) = pool_size();
        assert_eq!(
            limit,
            cpus().len().max(1) * 2,
            "`rule:http-server/a-core-is-never-blocked-on-a-syscall`'s number"
        );
        assert!(
            threads <= limit,
            "{calls} concurrent calls started {threads} threads against a bound of {limit}"
        );
        assert_eq!(
            threads, limit,
            "the calls did not overlap, so the bound was never the thing being tested"
        );
    }

    #[test]
    fn a_blocking_call_off_a_core_runs_on_this_thread() {
        let ran = Rc::new(Cell::new(false));
        let flag = Rc::clone(&ran);
        // Not `Send`, which is the point: off a core there is no handoff, so
        // there is nothing for the value to cross.
        let answer = if current_task().is_none() {
            flag.set(true);
            run(|| 3_u8)
        } else {
            unreachable!("a test thread is not a task")
        };
        assert_eq!(answer, 3);
        assert!(ran.get());
        assert_eq!(pool_size().0, 0, "an off-core call started a thread");
    }

    /// A panic in the work is the request's, not the pool's: it comes back to
    /// the task's own stack, where `rule:http-server/containment-does-not-end-at-the-helper`'s containment boundary is.
    #[test]
    fn a_panic_in_the_work_comes_back_to_the_task() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        sched.spawn(ctx(), TaskRoot::Worker, |_ctx| {
            let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
                run(|| panic!("the syscall wrapper went wrong"));
            }));
            assert!(outcome.is_err(), "the panic was swallowed off the core");
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.finished, 1, "the task never came back");
    }
}
