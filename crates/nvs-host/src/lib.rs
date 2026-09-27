//! The thread-per-core host: one pinned OS thread, one scheduler, one run
//! queue of stackful coroutines.
//!
//! This crate is where Novis stops being a compiler and starts being a
//! runtime. Everything above it — a server's accept loop, `Core\Task`, a socket
//! that looks blocking and is not — is a consumer of the two things declared
//! here: a [`Worker`], which is one OS thread pinned to one CPU, and the
//! [`Scheduler`] it runs, which owns every task on that core.
//!
//! # The one rule this crate exists to hold
//!
//! **The runtime is ours and it is not `async`.** `docs/plan/design.md`
//! § *Thread-per-core, shared-nothing runtime* is that decision's only home,
//! and `rule:http-server/a-core-is-never-blocked-on-a-syscall`
//! is what it buys: a core is never blocked on a syscall, because a task
//! that would wait suspends its own stack and hands the core back. A helper
//! reaches its yielder through the [`nvs_runtime::Ctx`] it was already given
//! ([`scheduler::suspend`]), so no function in the chain is marked, no `Core`
//! member has two spellings, and the language surface never grows a colour.
//!
//! The corollary matters as much: **a task never migrates**, so a refcount in
//! `nvs-runtime` is a non-atomic increment and an arena is reached without a
//! lock. [`Scheduler`] is `!Send` and `!Sync` so that this is enforced by the
//! compiler rather than remembered by contributors — `scheduler`'s module doc
//! owns the reasoning.
//!
//! # What is here, and what is not
//!
//! [`Worker::spawn`] starts a thread, pins it (best-effort — `affinity`'s
//! module doc says why a refusal is never fatal), builds a [`Scheduler`] *on*
//! that thread, and hands it to a closure. Nothing `!Send` crosses the thread
//! boundary, because the run queue is created on the far side of it.
//!
//! [`Reactor`] is what calls [`Scheduler::wake`]: readiness on epoll, kqueue or
//! a poll of `\Device\Afd`, keyed by [`TaskId`], with `rule:concurrency/the-parking-contract`'s parking
//! contract in its module doc. [`run_until_idle`] joins the two, and is
//! what a worker's body is. The scheduler itself still knows nothing about I/O,
//! so a run queue remains testable with none in it at all.
//!
//! A wake does not have to come from the kernel. [`RemoteWake`] is a one-shot,
//! `Send` permission to wake one task from another thread, which is how a call
//! with no readiness to wait on gets off the core at all; `reactor`'s docs
//! § *A wake that comes from another thread* own the mechanism and the counter
//! that stops the core giving up on a task while one is outstanding.
//!
//! A task reaches that reactor through a thread-local, not through an argument:
//! [`reactor::install`] holds one on the worker's thread and
//! [`reactor::with_current`] borrows it, because a socket's `Read` is handed a
//! buffer and nothing else. [`current_task`] and [`suspend_current`] are the
//! other half, the task's own id and its yielder without a `Ctx` to carry them.
//! `reactor`'s module doc records that decision and what it was chosen over.
//!
//! [`NvsTcp`] is that route's first consumer: a socket whose `Read` and `Write`
//! are `std::io`'s own and which parks instead of blocking, `rule:concurrency/try-the-syscall-then-park`. Its
//! module doc owns the try-then-park order and what a repeat park costs. There
//! is one such type and not one per socket family — [`NvsStream`] is generic
//! over what it parks on, `NvsUnix` is the same type over a local socket, and
//! [`NvsUdp`] is it again over a datagram one, waiting the same way under
//! `send_to` and `recv_from` instead of the two traits; `net`'s docs § *One
//! type over the source* own that call.
//!
//! What a task *costs* is [`stack`]: [`TASK_STACK_SIZE`] of reserved address
//! space per task, resident only in the pages its handler touched, pooled per
//! worker and recycled by the [`Scheduler`] that ended the task. That module's
//! doc is `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`'s only home in this tree, and it is also where the
//! recursion limit's bounds come from — a task's limit is armed from the stack
//! this crate handed it, not asserted from a ceiling.
//!
//! What a stack switch costs the *sanitizer* is `src/tsan.rs`: ThreadSanitizer
//! keeps one history of accesses per thread and expects a stack to belong to
//! one of them, so every resume and every forced unwind is bracketed by a fiber
//! switch saying the stack moved. Without it a run under `tools/tsan.sh` reports
//! at every task boundary. None of it is compiled into any build but that leg's,
//! and that module's doc owns why the switch draws a happens-before edge rather
//! than suppressing one.
//!
//! [`blocking`] is the other half of that: the pool `rule:http-server/a-core-is-never-blocked-on-a-syscall` sends a
//! filesystem call, a name resolution or a wait on a child process to, bounded
//! at twice the core count and started only when work arrives.
//! [`blocking::run`] is the whole handoff — off the core, back through a
//! [`RemoteWake`] — and it is the only way a call with no readiness to wait on
//! reaches a thread here.
//!
//! [`mod@worker`] is that handoff pointing the other way: the cores a child
//! placed `on: "worker"` is started on, bounded at the core count and started
//! only when work is first placed. [`worker::place`] runs a function as a *task*
//! on another core — so it may park and spawn children there, which is the whole
//! difference from a pool thread — and brings its answer back through the same
//! slot and [`RemoteWake`] the pool uses. That module's doc owns the inbox, the
//! bell that ends its receptionist's park, and what a placement spends.
//!
//! [`mod@channel`] is the first thing built *on* the scheduler rather than beside
//! it: a bounded queue between two tasks on one core, whose `send` suspends
//! when it is full instead of growing and whose `recv` suspends when it is
//! empty. Its module doc owns why the bound is the point and what route a wake
//! takes — [`Wake`], through the task tree, because a running task cannot reach
//! the scheduler that is resuming it.
//!
//! [`mod@block_on`] is the one place in this tree that speaks `Future`, and
//! `rule:concurrency/one-future-per-connection`
//! is why it is a loop rather than a runtime: an HTTP/1 connection is one
//! future, driven to completion on the coroutine that accepted it, whose waker
//! is a permission to poll again and nothing else. Nothing is spawned, nothing
//! is queued, and a wake fired from another thread queues an id on the parked
//! task's own core rather than moving the task. That module's doc owns the
//! ordering that makes a lost wakeup impossible and the one permission a whole
//! connection costs.
//!
//! [`watchdog`] is `rule:http-server/a-wedged-core-is-detected-by-its-deadline`: one thread for the process, reading the
//! earliest deadline each core publishes through [`timer::DeadlineView`] and
//! reporting a core that has been behind its own clock by a margin. It reads
//! state the deadline mechanism keeps anyway, so nothing on a request path is
//! written for it — that module's doc is why, and why detection is the whole of
//! what it does.
//!
//! [`cpuclock`] is the half of a CPU ceiling that belongs to the operating
//! system: a handle a core takes on its own thread's clock, which a thread that
//! is not that core may read. CPU time and never wall clock, for the reason
//! that module's doc gives, and `None` on a platform that offers no such clock
//! rather than a wall-clock one wearing its name.

pub mod affinity;
pub mod block_on;
pub mod blocking;
pub mod channel;
pub mod cpuclock;
pub mod group;
pub mod isolate;
pub mod ladder;
pub mod net;
mod placed;
pub mod reactor;
pub mod scheduler;
pub mod stack;
pub mod timer;
pub mod tls;
mod tsan;
pub mod watchdog;
pub mod worker;

pub use affinity::{CpuId, cpus, pin_current_thread};
pub use block_on::block_on;
pub use blocking::BlockingPool;
pub use channel::{Receiver, RecvError, SendError, Sender, TryRecvError, TrySendError, channel};
pub use cpuclock::ThreadClock;
pub use group::SchedulerHost;
pub use isolate::{Completion, Failure, Isolate, Output, Program, Running};
pub use net::{
    Accepted, Accepting, NvsAcceptor, NvsConnection, NvsListener, NvsStream, NvsTcp, NvsUdp,
};
#[cfg(unix)]
pub use net::{NvsUnix, NvsUnixListener};
pub use reactor::{Installed, Interest, Reactor, RemoteWake, run_until_idle, wake_at_drain};
pub use scheduler::{
    Finished, RunReport, Scheduler, TaskId, Waiting, Wake, cancel_task, children_still_running,
    current_task, detach_current, spawn_child, suspend, suspend_current,
};
pub use stack::{MAX_POOLED_STACKS, SPARE_STACK_SIZE, TASK_STACK_SIZE, on_spare_stack};
pub use timer::{DeadlineView, Timers, park_until, sleep};
pub use watchdog::{Registration, RunningRequest, Stall, Watchdog};

/// One OS thread, pinned to one CPU, running one [`Scheduler`].
///
/// The scheduler is built on the worker's own thread and never leaves it, so
/// the `!Send` run queue and the `!Send` contexts in it are structurally
/// unable to be shared — see this crate's module docs. What crosses the thread
/// boundary is the closure and its return value, and those carry `Send` in the
/// ordinary way.
pub struct Worker<T> {
    cpu: CpuId,
    pinned: bool,
    handle: std::thread::JoinHandle<T>,
}

impl<T> std::fmt::Debug for Worker<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hand-written rather than derived: deriving would demand `T: Debug`
        // from every caller for a field that is not printed anyway.
        f.debug_struct("Worker")
            .field("cpu", &self.cpu.raw())
            .field("pinned", &self.pinned)
            .finish()
    }
}

/// A name resolved on this worker's blocking pool: `nvs-runtime`'s own lookup,
/// handed off the core.
///
/// This is the whole of what that crate cannot write for itself — it is the
/// bottom of the crate tree and the pool is here — and nothing more: the
/// grammar of resolution, the cap on the answer and the refusal a caller reads
/// all stay in the door, per
/// `rule:http-server/an-outbound-call-tries-every-approved-address`. The name is
/// copied because a job outlives the borrow that submitted it, which is one
/// allocation per lookup against a call that costs a network round trip.
fn resolve_off_core(host: &str) -> std::io::Result<Vec<std::net::IpAddr>> {
    let named = host.to_owned();
    blocking::run(move || nvs_runtime::capability::lookup_on_this_thread(&named))
}

impl<T: Send + 'static> Worker<T> {
    /// Starts a worker on `cpu` and runs `body` with that core's scheduler.
    ///
    /// Returns once the thread has been pinned and is about to enter `body`,
    /// so [`Worker::pinned`] is answerable immediately rather than racing the
    /// thread it describes. The handshake costs one channel round trip per
    /// worker, which is per *process start* and not per request.
    ///
    /// # Errors
    ///
    /// Whatever `std::thread::Builder::spawn` reports — the OS refused to
    /// create a thread. Nothing about pinning is an error here; see
    /// [`affinity`].
    pub fn spawn<F>(cpu: CpuId, body: F) -> std::io::Result<Self>
    where
        F: FnOnce(&mut Scheduler) -> T + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::sync_channel::<bool>(1);
        let handle = std::thread::Builder::new()
            .name(format!("nvs-worker-{}", cpu.raw()))
            .spawn(move || {
                let pinned = pin_current_thread(cpu);
                // A send failure means the spawning thread is already gone,
                // which cannot happen while it is blocked on `recv` below; if
                // it somehow did, running the body anyway is still correct.
                let _ = tx.send(pinned);
                // `rule:http-server/a-core-is-never-blocked-on-a-syscall`: the
                // capability door's name lookup is the one call `nvs-runtime`
                // cannot wait on, and this crate is the half that owns a pool to
                // hand it to. Installed on the worker's own thread because the
                // seam is per thread, for the reason
                // `nvs_runtime::capability::install_resolver` gives.
                nvs_runtime::capability::install_resolver(resolve_off_core);
                // Built here, on this thread, which is the whole reason
                // `Scheduler` never has to be `Send`.
                let mut sched = Scheduler::new();
                body(&mut sched)
            })?;

        // A closed channel means the thread died before it reported, which
        // `join` will surface properly; treat it as unpinned rather than
        // failing here for a reason that is not the caller's.
        let pinned = rx.recv().unwrap_or(false);
        Ok(Self {
            cpu,
            pinned,
            handle,
        })
    }
}

impl<T> Worker<T> {
    /// The CPU this worker asked for.
    #[must_use]
    pub const fn cpu(&self) -> CpuId {
        self.cpu
    }

    /// Whether the OS agreed to pin this worker.
    ///
    /// `false` costs cache locality and nothing else — [`affinity`]'s module
    /// doc owns why that is not a reason to refuse to start.
    #[must_use]
    pub const fn pinned(&self) -> bool {
        self.pinned
    }

    /// Waits for the worker to finish and returns what its body returned.
    ///
    /// # Errors
    ///
    /// The panic payload, if one escaped the body. A panic *inside a task* does
    /// not reach here — `nvs_runtime::run_task` contains it at the task root
    /// and reports it through [`Finished::outcome`], which is `rule:http-server/containment-does-not-end-at-the-helper`'s
    /// split between a failed request and a retired worker.
    pub fn join(self) -> std::thread::Result<T> {
        self.handle.join()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::{Ctx, OutputSink, TaskRoot};

    #[test]
    fn a_worker_runs_its_scheduler_on_its_own_thread() {
        let Some(cpu) = cpus().into_iter().next() else {
            // A platform that lists no CPU is supported; see `affinity`.
            return;
        };

        let spawner = std::thread::current().id();
        let worker = Worker::spawn(cpu, move |sched| {
            let ran_on = std::thread::current().id();
            sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Worker, |_| {});
            let report = sched.run();
            (ran_on, report.finished)
        })
        .expect("the OS refused a thread");

        // A platform may list a CPU and refuse to pin it — arm64 macOS refuses
        // every one it lists — so the assertion is that the worker reports what
        // this platform actually does, which is what `pinned()` is for. Probed
        // on a thread of its own, so the probe does not leave the test thread
        // bound to a CPU.
        let platform_pins = std::thread::spawn(move || pin_current_thread(cpu))
            .join()
            .expect("the probe thread panicked");
        assert_eq!(
            worker.pinned(),
            platform_pins,
            "the worker and a bare pin disagreed about CPU {}",
            cpu.raw()
        );
        assert_eq!(worker.cpu(), cpu);

        let (ran_on, finished) = worker.join().expect("the worker panicked");
        assert_ne!(ran_on, spawner, "the scheduler must run on the worker");
        assert_eq!(finished, 1);
    }

    #[test]
    fn one_worker_per_listed_cpu_can_run_at_once() {
        // The shape a server start-up has: one worker per CPU, each with its
        // own run queue, nothing shared between them.
        let listed = cpus();
        if listed.is_empty() {
            return;
        }

        let workers: Vec<_> = listed
            .iter()
            .map(|&cpu| {
                Worker::spawn(cpu, move |sched| {
                    sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Worker, |_| {});
                    sched.run().finished
                })
                .expect("the OS refused a thread")
            })
            .collect();

        assert_eq!(workers.len(), listed.len());
        for worker in workers {
            assert_eq!(worker.join().expect("a worker panicked"), 1);
        }
    }
}
