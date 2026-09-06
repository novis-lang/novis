//! The watchdog: one thread for the process, noticing a core that has stopped
//! turning at all.
//!
//! [ADR 0106](/docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//! § 7 is the whole specification, and its first clause decides this module's
//! shape: the watchdog **reads the in-flight deadline each worker already
//! maintains**. So there is no heartbeat here. A worker writes nothing for the
//! watchdog's benefit, on any path; what it reads is [`crate::timer`]'s
//! [`DeadlineView`], which is the first entry of a table the deadline mechanism
//! was keeping anyway. A per-request beat would be the wrong implementation of
//! this item even if it were cheap, because it measures the request rather than
//! the core.
//!
//! # Why the earliest deadline is the right thing to read
//!
//! `Timers::take_due` drops a deadline the moment the core polls after it, so a
//! core that is turning never leaves one behind the clock for longer than one
//! poll. A published deadline that is still a *margin* in the past therefore
//! says the core has not turned in that long — not that a request is slow, and
//! not that a handler is taking its time. That is the fault § 7 exists for: an
//! engine deadlock, or a loop with no poll site in it, which no other mechanism
//! in that ADR answers because the worker is not faulting in any way it could
//! be asked about.
//!
//! # Report and shed, never kill
//!
//! Firing writes one record to `rule:errors/escalation-ladder`
//! § 4's floor — [`Watchdog::new`]'s sink is `stderr`, which is that ADR's
//! default target — and does nothing else to the core. A thread cannot be
//! safely killed in-process and ADR 0106 § 14 declines the process boundary
//! that would make it possible, so detection is the whole of what happens here.
//! The other half of § 7's sentence — the core stops accepting new work and
//! `max_in_flight` accounts for its share as unavailable — belongs to admission
//! control, which does not exist in this crate yet; when it does, it subscribes
//! to the same report rather than growing a second detector.
//!
//! A stall is reported **once per deadline**, not once per sweep: a wedged core
//! republishes nothing, so its earliest deadline is a stable identity for the
//! episode. That is [ADR 0106](/docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//! § 10's coalescing rule arriving for free rather than as a second bound on
//! the sink. A core that recovers and stalls again on a later deadline is a new
//! episode and reports again.
//!
//! # What it costs, and what it is configured by
//!
//! One OS thread for the process — not one per core — started when the first
//! core registers, so a process running no workers has none. It holds one
//! entry per *registered core*, which is O(cores) and never grows with requests
//! served, and it wakes once per [`DEFAULT_INTERVAL`] to read one atomic per
//! entry.
//!
//! [`DEFAULT_MARGIN`] and [`DEFAULT_INTERVAL`] are **compiled-in defaults**.
//! `docs/agent/loop-goal.md` § *Standing decisions* puts the `[limits]` block
//! that will make them configurable in the goal after this one, and says to run
//! under compiled-in defaults and to say so at the site — this paragraph is
//! that.
//!
//! # Registering a core
//!
//! Whoever installs a core's [`Reactor`](crate::Reactor) registers it, because
//! that is where a [`DeadlineView`] comes from:
//!
//! ```ignore
//! let installed = reactor::install(Reactor::new()?);
//! let _watched = watchdog.register(cpu, reactor::with_current(|r| r.deadline_view()).unwrap());
//! ```
//!
//! [`Registration`] deregisters on drop, so a worker that ends stops being
//! watched. Without that, a finished core's frozen view would read as a wedged
//! one forever — its deadline cannot move again — and the watched set would
//! grow with workers *started* rather than with workers running.

use std::io::Write;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::affinity::CpuId;
use crate::timer::DeadlineView;

/// How far past its earliest deadline a core must be before it is reported.
///
/// Generous on purpose. The margin is not a latency target — a request that
/// overruns its deadline is answered by the deadline, not by this — it is the
/// point past which "the core did not turn" stops being explicable by a long
/// poll, a descheduled thread or a stepped clock.
pub const DEFAULT_MARGIN: Duration = Duration::from_secs(5);

/// How often the watchdog reads every registered core.
///
/// The detection latency is this plus [`DEFAULT_MARGIN`] at worst; the cost is
/// one wakeup and one relaxed load per core, which is why it can be short
/// without being an expense worth tuning.
pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(500);

/// One report: a core whose earliest deadline passed and stayed passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stall {
    /// The CPU the wedged worker was pinned to, which is how an operator names
    /// it in a log and how admission control will name it later.
    pub cpu: CpuId,
    /// The deadline that went unanswered. Stable for as long as the core stays
    /// wedged, which is what makes it this episode's identity.
    pub deadline: Instant,
    /// How far past that deadline the core was when the report fired — at
    /// least [`Watchdog`]'s margin, and more when the wedge started between two
    /// sweeps.
    pub overdue_by: Duration,
}

/// Where a report goes: `rule:errors/engine-floor`'s floor, or a caller's own sink.
type Sink = Box<dyn Fn(&Stall) + Send + Sync + 'static>;

/// One registered core, and what has already been said about it.
struct Watched {
    /// Identity for deregistration. A counter rather than the [`CpuId`],
    /// because a replacement worker for a wedged core is a second registration
    /// on the same CPU and must not deregister the first.
    id: u64,
    cpu: CpuId,
    view: DeadlineView,
    /// The deadline this core was last reported for, so an episode produces one
    /// record rather than one per sweep. Cleared whenever the core is not
    /// overdue, which is what makes a later stall a new episode.
    reported: Option<Instant>,
}

/// What the watchdog thread and the cores registering with it share.
struct Shared {
    margin: Duration,
    interval: Duration,
    sink: Sink,
    state: Mutex<State>,
    /// Signalled to end the wait early — at shutdown, so a drop does not have
    /// to wait out an interval to join.
    change: Condvar,
}

/// The mutable half, which the watchdog thread holds only between sweeps.
struct State {
    watching: Vec<Watched>,
    /// Cleared by [`Watchdog::drop`]; the thread returns the next time it looks.
    running: bool,
    next_id: u64,
    /// Started with the first registration and joined by [`Watchdog::drop`].
    /// Here rather than beside the `Arc` so that registering takes `&self`, as
    /// it must when every core registers itself.
    thread: Option<JoinHandle<()>>,
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    // A poisoned lock means a sink panicked. The state behind it is a list of
    // views and a flag — nothing a panic can leave half-written — and refusing
    // to watch any core again because one report panicked is the wrong trade
    // for a mechanism whose whole job is to notice silence.
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The process's watchdog.
///
/// Nothing is watched until a core [`Watchdog::register`]s, and the thread
/// starts with the first one. Dropping it stops and **joins** that thread —
/// unlike [`crate::blocking::BlockingPool`], which deliberately does not join,
/// because this one is owned by the process rather than by a thread-local and
/// so its drop never runs from a TLS destructor.
pub struct Watchdog {
    shared: Arc<Shared>,
}

impl std::fmt::Debug for Watchdog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = lock(&self.shared.state);
        f.debug_struct("Watchdog")
            .field("watching", &state.watching.len())
            .field("margin", &self.shared.margin)
            .field("interval", &self.shared.interval)
            .field("started", &state.thread.is_some())
            .finish()
    }
}

impl Default for Watchdog {
    fn default() -> Self {
        Self::new()
    }
}

impl Watchdog {
    /// A watchdog reporting to `rule:errors/engine-floor`'s floor, on this module's defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::with(DEFAULT_MARGIN, DEFAULT_INTERVAL, |stall| {
            // Written through the handle rather than with `eprintln!`, which
            // this workspace's clippy denies and which would panic on a broken
            // pipe — the one thing `rule:errors/engine-floor`'s floor may not do. A failed
            // write here is swallowed, per that section's last paragraph:
            // there is nothing further to escalate to.
            let _ = writeln!(
                std::io::stderr(),
                "nvs: the worker on cpu {} has not answered a deadline for {:?} — the core is \
                 not turning",
                stall.cpu.raw(),
                stall.overdue_by
            );
        })
    }

    /// A watchdog with its own margin, interval and sink.
    ///
    /// The sink is called on the watchdog's thread with no lock held, so it may
    /// do anything a log write may do — including taking its own lock, and
    /// including registering or deregistering a core.
    #[must_use]
    pub fn with(
        margin: Duration,
        interval: Duration,
        report: impl Fn(&Stall) + Send + Sync + 'static,
    ) -> Self {
        Self {
            shared: Arc::new(Shared {
                margin,
                interval,
                sink: Box::new(report),
                state: Mutex::new(State {
                    watching: Vec::new(),
                    running: true,
                    next_id: 0,
                    thread: None,
                }),
                change: Condvar::new(),
            }),
        }
    }

    /// Watches `cpu`'s core through `view` until the returned handle is dropped.
    ///
    /// Called from the core's own thread, right after it installs its reactor —
    /// this module's docs carry the two lines. The first call starts the
    /// watchdog thread; a thread the OS refuses leaves the core registered and
    /// unwatched rather than failing a worker's startup for it, which is the
    /// same trade [`crate::blocking`] makes for a pool thread.
    pub fn register(&self, cpu: CpuId, view: DeadlineView) -> Registration {
        let mut state = lock(&self.shared.state);
        let id = state.next_id;
        state.next_id += 1;
        state.watching.push(Watched {
            id,
            cpu,
            view,
            reported: None,
        });
        if state.thread.is_none() && state.running {
            let shared = Arc::clone(&self.shared);
            state.thread = std::thread::Builder::new()
                .name("nvs-watchdog".to_owned())
                .spawn(move || watch(&shared))
                .ok();
        }
        drop(state);
        Registration {
            shared: Arc::clone(&self.shared),
            id,
        }
    }

    /// How many cores are registered.
    #[must_use]
    pub fn watching(&self) -> usize {
        lock(&self.shared.state).watching.len()
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        let thread = {
            let mut state = lock(&self.shared.state);
            state.running = false;
            state.thread.take()
        };
        self.shared.change.notify_all();
        if let Some(thread) = thread {
            // Joining is safe here for the reason this type's docs give, and it
            // is worth doing: a watchdog outliving its `Watchdog` would keep
            // reporting cores that are already gone.
            let _ = thread.join();
        }
    }
}

/// One core's place in the watched set, which it holds for as long as it runs.
pub struct Registration {
    shared: Arc<Shared>,
    id: u64,
}

impl std::fmt::Debug for Registration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registration")
            .field("id", &self.id)
            .finish()
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        let mut state = lock(&self.shared.state);
        state.watching.retain(|watched| watched.id != self.id);
    }
}

impl Shared {
    /// Every core that is past its deadline by the margin and has not already
    /// been reported for that deadline.
    ///
    /// Takes `now` rather than reading the clock so that the whole decision is
    /// testable without waiting for one.
    fn sweep(&self, now: Instant) -> Vec<Stall> {
        let mut state = lock(&self.state);
        let margin = self.margin;
        let mut stalls = Vec::new();
        for watched in &mut state.watching {
            let overdue = watched
                .view
                .oldest()
                .and_then(|deadline| Some((deadline, now.checked_duration_since(deadline)?)));
            let Some((deadline, overdue_by)) = overdue.filter(|&(_, by)| by >= margin) else {
                // Holding nothing, or holding something still in the future: the
                // core is answering its deadlines, so the next stall is a new
                // episode and gets its own record.
                watched.reported = None;
                continue;
            };
            if watched.reported == Some(deadline) {
                continue;
            }
            watched.reported = Some(deadline);
            stalls.push(Stall {
                cpu: watched.cpu,
                deadline,
                overdue_by,
            });
        }
        stalls
    }
}

/// The watchdog thread: wait an interval, read every core, report what is new.
fn watch(shared: &Arc<Shared>) {
    loop {
        {
            let state = lock(&shared.state);
            if !state.running {
                return;
            }
            let (state, _) = shared
                .change
                .wait_timeout(state, shared.interval)
                .unwrap_or_else(PoisonError::into_inner);
            if !state.running {
                return;
            }
        }
        // Outside the lock, so a sink that takes its time cannot keep a core
        // from registering or deregistering.
        for stall in shared.sweep(Instant::now()) {
            (shared.sink)(&stall);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Worker;
    use crate::affinity::cpus;
    use crate::reactor::{Reactor, install, run_until_idle, with_current};
    use crate::scheduler::TaskId;
    use crate::timer::{Timers, sleep};
    use nvs_runtime::{Ctx, OutputSink, TaskRoot};
    use std::sync::mpsc;

    fn a_cpu() -> CpuId {
        *cpus().first().expect("the platform reported no CPUs")
    }

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    fn watchdog_of(margin: Duration, sink: mpsc::Sender<Stall>) -> Watchdog {
        Watchdog::with(margin, Duration::from_millis(1), move |stall| {
            let _ = sink.send(*stall);
        })
    }

    /// The fault this whole module exists for: a deadline that passed and
    /// stayed passed, because the core that filed it stopped turning.
    #[test]
    fn a_core_past_its_deadline_by_the_margin_is_reported() {
        let (tx, _rx) = mpsc::channel();
        let margin = Duration::from_secs(5);
        let dog = watchdog_of(margin, tx);
        let mut timers = Timers::default();
        let filed = Instant::now() + Duration::from_secs(1);
        timers.arm(TaskId::from_raw(1), filed);
        let _watched = dog.register(a_cpu(), timers.view());

        // Inside the margin: late is not wedged, and a poll that came back slow
        // must not be reported as one.
        assert!(
            dog.shared
                .sweep(filed + margin - Duration::from_millis(1))
                .is_empty()
        );

        let stalls = dog.shared.sweep(filed + margin);
        assert_eq!(stalls.len(), 1, "the wedged core was not reported");
        assert_eq!(stalls[0].cpu, a_cpu());
        assert_eq!(stalls[0].deadline, filed);
        assert_eq!(stalls[0].overdue_by, margin);

        // The same wedge is one episode, however many times it is looked at.
        assert!(
            dog.shared.sweep(filed + margin * 2).is_empty(),
            "a still-wedged core was reported twice for one deadline"
        );
    }

    /// The other half, and the one a naive heartbeat gets wrong: a core doing
    /// its job is not reported however long its requests are allowed to take.
    #[test]
    fn a_busy_core_is_not_reported() {
        let (tx, _rx) = mpsc::channel();
        let margin = Duration::from_secs(5);
        let dog = watchdog_of(margin, tx);
        let mut timers = Timers::default();
        let start = Instant::now();
        let _watched = dog.register(a_cpu(), timers.view());

        // Nothing filed at all — an idle core, which is silent for exactly the
        // same reason a wedged one is loud: nobody is writing a beat.
        assert!(
            dog.shared
                .sweep(start + Duration::from_secs(3600))
                .is_empty()
        );

        // Answering deadlines as they fall due, which is `take_due` running:
        // a core that turns takes each one within a poll of its instant, so
        // what is published is never a margin behind — even though every
        // request here was allowed a full second and used all of it.
        for step in 1..5_u64 {
            let at = start + Duration::from_secs(step);
            timers.arm(TaskId::from_raw(step), at);
            assert!(
                dog.shared.sweep(at + Duration::from_millis(1)).is_empty(),
                "a deadline one poll old was called a wedge"
            );
            assert_eq!(timers.take_due(at), Some(TaskId::from_raw(step)));
            assert!(dog.shared.sweep(at + margin).is_empty());
        }
    }

    /// A recovered core that wedges again is a second episode, because the
    /// deadline identifying it is a different one.
    #[test]
    fn a_second_wedge_on_a_later_deadline_reports_again() {
        let (tx, _rx) = mpsc::channel();
        let margin = Duration::from_secs(5);
        let dog = watchdog_of(margin, tx);
        let mut timers = Timers::default();
        // Strictly after the instant `timers` fixed as its publishing base, and
        // that is load-bearing rather than tidy: `Timers::publish` keeps a
        // deadline out of `NOTHING`'s way by publishing it at least one
        // nanosecond after that base, so a deadline armed at the base itself
        // reads back a nanosecond late and a sweep at exactly `start + margin`
        // finds it one nanosecond short of overdue. Two adjacent
        // `Instant::now()` calls can return the same value, so taking the
        // second one for `start` made this test fail under a loaded machine.
        let start = Instant::now() + Duration::from_millis(1);
        let _watched = dog.register(a_cpu(), timers.view());

        timers.arm(TaskId::from_raw(1), start);
        assert_eq!(dog.shared.sweep(start + margin).len(), 1);
        assert!(timers.disarm(TaskId::from_raw(1)));
        assert!(dog.shared.sweep(start + margin).is_empty());

        let later = start + Duration::from_secs(60);
        timers.arm(TaskId::from_raw(2), later);
        assert_eq!(dog.shared.sweep(later + margin).len(), 1);
    }

    /// § 7 end to end, on real threads and a real reactor: a worker that stops
    /// turning is reported, and one that turns throughout a longer interval is
    /// not — which is the pair a per-request heartbeat cannot tell apart,
    /// because both of them are running a request the whole time.
    ///
    /// The name's other half is structural, and this test is where it is
    /// visible: neither worker writes anything for the watchdog. The wedged one
    /// arms one deadline and then stops executing altogether, and the report
    /// still arrives — so what fired it was the timer state the core keeps for
    /// its own poll timeout and nothing else.
    #[test]
    fn the_watchdog_reports_a_wedged_worker_without_a_heartbeat() {
        let margin = Duration::from_millis(250);
        let (tx, rx) = mpsc::channel();
        let dog = Arc::new(Watchdog::with(
            margin,
            Duration::from_millis(5),
            move |stall| {
                let _ = tx.send(*stall);
            },
        ));
        let cpu = a_cpu();

        // A core that keeps turning, for twice the margin, while never leaving
        // a deadline unanswered for more than the sleep that armed it.
        let busy = Arc::clone(&dog);
        let working = Worker::spawn(cpu, move |sched| {
            let _installed = install(Reactor::new().expect("the OS refused a poll"));
            let view =
                with_current(|reactor| reactor.deadline_view()).expect("no reactor installed");
            let _watched = busy.register(cpu, view);
            sched.spawn(ctx(), TaskRoot::Worker, |_ctx| {
                for _ in 0..5 {
                    sleep(Duration::from_millis(100));
                }
            });
            run_until_idle(sched).expect("the loop failed")
        })
        .expect("the OS refused a thread");
        assert_eq!(working.join().expect("the worker panicked").finished, 1);
        assert!(
            rx.try_recv().is_err(),
            "a core that answered every deadline was reported as wedged"
        );

        // And a core that files a deadline and then never polls again — § 7's
        // own example, a loop with no poll site in it.
        let stuck = Arc::clone(&dog);
        let wedged = Worker::spawn(cpu, move |_sched| {
            let _installed = install(Reactor::new().expect("the OS refused a poll"));
            let view =
                with_current(|reactor| reactor.deadline_view()).expect("no reactor installed");
            let _watched = stuck.register(cpu, view);
            with_current(|reactor| {
                reactor.timers().arm(
                    TaskId::from_raw(1),
                    Instant::now() + Duration::from_millis(10),
                );
            });
            std::thread::sleep(margin * 4);
        })
        .expect("the OS refused a thread");

        let stall = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the wedged worker was never reported");
        assert_eq!(stall.cpu, cpu);
        assert!(stall.overdue_by >= margin);
        wedged.join().expect("the worker panicked");
    }

    /// The thread does the sweeping without anyone asking it to, and a
    /// deregistered core stops being watched.
    #[test]
    fn the_thread_reports_on_its_own_and_stops_when_told() {
        let (tx, rx) = mpsc::channel();
        let dog = watchdog_of(Duration::from_millis(5), tx);
        let mut timers = Timers::default();
        // Armed at the instant the timers were created, so it is overdue by the
        // margin a handful of milliseconds later.
        timers.arm(TaskId::from_raw(1), Instant::now());
        let watched = dog.register(a_cpu(), timers.view());
        assert_eq!(dog.watching(), 1);

        let stall = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the watchdog thread never reported the wedged core");
        assert_eq!(stall.cpu, a_cpu());

        drop(watched);
        assert_eq!(dog.watching(), 0, "a dropped registration stayed watched");
        // And the drop below joins the thread, so a hang here is a real bug and
        // not a flake.
    }
}
