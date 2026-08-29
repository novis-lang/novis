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
//! and [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//! § 6 is what it buys: a core is never blocked on a syscall, because a task
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
//! a poll of `\Device\Afd`, keyed by [`TaskId`], with ADR 0115 § 2's five-rule
//! parking contract in its module doc. [`run_until_idle`] joins the two, and is
//! what a worker's body is. The scheduler itself still knows nothing about I/O,
//! so a run queue remains testable with none in it at all.
//!
//! **Still outstanding:** `NvsTcp` — the stream whose `Read`/`Write` look
//! blocking and park instead (ADR 0115 § 3) — and the blocking pool ADR 0106
//! § 6 sends filesystem calls, name resolution and child processes to.

pub mod affinity;
pub mod reactor;
pub mod scheduler;

pub use affinity::{CpuId, cpus, pin_current_thread};
pub use reactor::{Interest, Reactor, run_until_idle};
pub use scheduler::{Finished, RunReport, Scheduler, TaskId, Waiting, suspend};

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
    /// and reports it through [`Finished::outcome`], which is ADR 0106 § 2's
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

        assert!(
            worker.pinned(),
            "the OS listed CPU {} and then refused it",
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
