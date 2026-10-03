//! The watchdog: one thread for the process, noticing a core that has stopped
//! turning at all.
//!
//! `rule:http-server/a-wedged-core-is-detected-by-its-deadline`
//! is the whole specification, and its first clause decides this module's
//! shape: the watchdog **reads the in-flight deadline each worker already
//! maintains**. So there is no heartbeat here, and nothing on a request path is
//! written for this module's benefit. What it reads is [`crate::timer`]'s
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
//! Firing writes one record to `rule:errors/engine-floor`'s floor — [`Watchdog::new`]'s sink is `stderr`, which is that ADR's
//! default target — and does nothing else to the core. A thread cannot be
//! safely killed in-process and `rule:http-server/the-residue-is-one-named-fault-class` declines the process boundary
//! that would make it possible, so detection is the whole of what happens here.
//! The other half of § 7's sentence — the core stops accepting new work and
//! `max_in_flight` accounts for its share as unavailable — belongs to admission
//! control, which is `nvs_server::admit` and does not subscribe to this report
//! yet (`rule:http-server/a-wedged-core-is-shed-never-killed` is `designed`);
//! when it does, it subscribes to the same report rather than growing a second
//! detector.
//!
//! A stall is reported **once per deadline**, not once per sweep: a wedged core
//! republishes nothing, so its earliest deadline is a stable identity for the
//! episode. That is `rule:http-server/the-floor-cannot-fill-the-disk`
//! 's coalescing rule arriving for free rather than as a second bound on
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
//! `[server] watchdog_margin` sets the margin, and [`DEFAULT_MARGIN`] is the
//! margin with the key left out. `nvs serve` resolves the key at boot through
//! `nvs_config::server::watchdog_margin_for` and builds the process's one
//! watchdog with [`Watchdog::reporting_after`], which is why the key is `Boot`:
//! a reload has no second watchdog to hand a new margin to. [`DEFAULT_INTERVAL`]
//! has no key, because a sweep costs one wakeup and one relaxed load per core
//! and a shorter or longer one buys an operator nothing the margin does not.
//!
//! # The request a core is running
//!
//! Reporting a wedged core answers the *core*. Stopping the **request** that
//! wedged it needs a handle on that request, and
//! [`Registration::publish_safepoint`] is where a core hands one over:
//! `nvs-runtime`'s [`SafepointView`] on its request tree's safepoint word,
//! which is the word compiled code polls at every function entry and loop back
//! edge. One store through it stops every isolate and task under that tree,
//! because `rule:security/isolate-shares-nothing` gives a tree one ceiling to
//! divide and so one word to be stopped by.
//!
//! The handle goes through the lock this module already holds rather than
//! through a second atomic beside the deadline. A core takes that lock to
//! register and to deregister, and the watchdog holds it only across a clone,
//! so a published handle costs a core no synchronisation the watched set was
//! not already costing it. A *deadline* could not be published this way —
//! [`crate::timer`]'s docs own that argument — because it is rewritten every
//! time any task arms or disarms one.
//!
//! What a core publishes is a [`RunningRequest`]: that handle, and the reading
//! the core's own [`ThreadClock`] gave at the instant of publication. The
//! handle alone cannot be charged against a ceiling — the watchdog has to know
//! where this request's spending starts — and a core is the only thread that
//! may take a clock on itself, so the baseline is taken where the publication
//! is. The ceiling is not part of the record: it is read through the handle on
//! every sweep, so one a program narrowed with `Core\Config::set` is the one
//! charged, from the baseline its request started with. A request under no cap
//! is published and compared with nothing until it has one. A core on a
//! platform with no per-thread clock publishes nothing and is sampled not at
//! all.
//!
//! [`Shared::sweep`] reads that charge on the walk it already makes, and a
//! request whose charge has reached its ceiling is asked to stop through the
//! handle beside it: `SafepointFlags::CPU_LIMIT` into the word compiled code
//! polls, which is `rule:errors/on-limit`'s ceiling arriving at the request
//! that spent it. Nothing there touches the *core* — the section above still
//! holds, and a core is reported and shed rather than killed; what changes is
//! that the request wedging it now stops itself at its next poll.
//!
//! The window a charge is read over runs from that baseline, and it is the
//! *core's* time rather than the request's: a core runs many tasks, so a
//! neighbour that ran since the publication is charged to the request that was
//! published. The fault the ceiling exists for is the one where that cannot
//! happen — a request that never yields is a core that never republishes — so
//! the over-charge needs the very thing the runaway case denies it. A request
//! that does yield is charged from its most recent publication and no total is
//! kept across them, because accounting a yielding request's whole spend needs
//! a store at the yield, which is the request-path write this module's first
//! paragraph refuses.
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
//!
//! # Registering a thread that is no core
//!
//! A `nvs run` is one request on one thread: it has a ceiling to be stopped by
//! and no CPU it is pinned to, no fleet to shed its share onto and no operator
//! but the one reading the stderr the program itself writes.
//! [`Watchdog::register_requests`] is that caller's door. It buys the half of
//! this module that answers a *request* — publication, sampling, and the flags
//! raised at the ceiling — and none of the half that answers a core, so such an
//! entry takes no [`DeadlineView`] and is reported as wedged never.

use std::io::Write;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use nvs_runtime::{SafepointFlags, SafepointView};

use crate::affinity::CpuId;
use crate::cpuclock::ThreadClock;
use crate::timer::DeadlineView;

/// How far past its earliest deadline a core must be before it is reported,
/// when `[server] watchdog_margin` is not written.
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

/// What a core publishes about the request it is running: the handle that stops
/// it and carries the CPU ceiling it runs under, and where its charge starts.
///
/// Minted on the core's own thread by [`RunningRequest::new`] and handed to
/// [`Registration::publish_safepoint`]. It holds no borrow of the request — the
/// handle is `Send` where a `Ctx` is not, and the rest is a duration and an
/// identifier — so the watchdog thread reads all of it without touching
/// anything the request owns.
#[derive(Clone, Debug)]
pub struct RunningRequest {
    view: SafepointView,
    clock: ThreadClock,
    baseline: Duration,
}

impl RunningRequest {
    /// The request a core is about to run, or `None` where no CPU ceiling can
    /// be enforced on it.
    ///
    /// Called on the core's own thread, and `clock` must be that core's: what
    /// is read here is the baseline every later charge is measured from, and a
    /// clock on any other thread measures somebody else's work.
    ///
    /// `None` in two cases a caller does not have to tell apart, because the
    /// answer to both is the same — publish nothing, be sampled not at all: the
    /// platform has no per-thread clock a stranger may read, or the clock
    /// refuses a reading.
    ///
    /// **A request under no cap is published like any other.** Its ceiling is
    /// read through the handle on every sweep ([`Self::limit`]) and is not
    /// copied here, because `Core\Config::set` may narrow `limits.cpu_time`
    /// while the request runs, and the charge that ceiling is compared with has
    /// to start where the request did. A baseline taken at the narrowing would
    /// let a program clear its own charge by setting the directive again.
    ///
    /// **What it spends:** one reading of the thread's clock per publication,
    /// capped or not.
    #[must_use]
    pub fn new(view: SafepointView, clock: Option<ThreadClock>) -> Option<Self> {
        let clock = clock?;
        Some(Self {
            view,
            clock,
            baseline: clock.burned()?,
        })
    }

    /// The handle that stops this request, and with it every isolate and task
    /// under its tree.
    #[must_use]
    pub fn view(&self) -> &SafepointView {
        &self.view
    }

    /// The CPU time this request may burn as its tree's root states it now —
    /// `[limits] cpu_time`, as `Ctx::cpu_limit` reports it — or `None` while
    /// the request is under no cap.
    #[must_use]
    pub fn limit(&self) -> Option<Duration> {
        let nanos = self.view.cpu_limit();
        (nanos > 0).then(|| Duration::from_nanos(nanos))
    }

    /// What the core had burned at the instant this request was published.
    #[must_use]
    pub fn baseline(&self) -> Duration {
        self.baseline
    }

    /// What the core has burned since, or `None` once its thread has ended.
    ///
    /// Read from the watchdog's thread rather than the core's, which is what
    /// [`ThreadClock`] exists for. Saturating, because a clock the operating
    /// system steps backwards must read as no charge rather than as an enormous
    /// one.
    #[must_use]
    pub fn burned(&self) -> Option<Duration> {
        Some(self.clock.burned()?.saturating_sub(self.baseline))
    }
}

/// One registered thread, and what has already been said about it.
struct Watched {
    /// Identity for deregistration. A counter rather than the [`CpuId`],
    /// because a replacement worker for a wedged core is a second registration
    /// on the same CPU and must not deregister the first.
    id: u64,
    /// The server core this entry is, as the pair that makes a stall report
    /// possible: the CPU the worker is pinned to and the deadline table it
    /// keeps. `None` for a thread that runs requests and is no core —
    /// [`Watchdog::register_requests`]'s caller — which is sampled against its
    /// ceilings exactly as a core is and is reported as wedged never, because
    /// there is no deadline table to read and nothing to shed it onto.
    core: Option<(CpuId, DeadlineView)>,
    /// The request tree this core is running and the baseline it is charged
    /// from, or `None` while it is running none —
    /// [`Registration::publish_safepoint`] is the only writer, and this
    /// module's docs say why it lives behind the same lock as the rest of the
    /// entry.
    running: Option<RunningRequest>,
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
        Self::reporting_after(DEFAULT_MARGIN)
    }

    /// A watchdog reporting to `rule:errors/engine-floor`'s floor a core that is
    /// `margin` past its earliest deadline, sweeping every [`DEFAULT_INTERVAL`].
    ///
    /// This is the constructor `[server] watchdog_margin` reaches, so a
    /// configured margin and the default one report through the same sink and
    /// in the same words.
    #[must_use]
    pub fn reporting_after(margin: Duration) -> Self {
        Self::with(margin, DEFAULT_INTERVAL, |stall| {
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

    /// How far past its earliest deadline a core must be before it is reported.
    #[must_use]
    pub fn margin(&self) -> Duration {
        self.shared.margin
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
    /// this module's docs carry the snippet. The first call starts the
    /// watchdog thread; a thread the OS refuses leaves the core registered and
    /// unwatched rather than failing a worker's startup for it, which is the
    /// same trade [`crate::blocking`] makes for a pool thread.
    pub fn register(&self, cpu: CpuId, view: DeadlineView) -> Registration {
        self.watch(Some((cpu, view)))
    }

    /// Watches a thread that runs requests and is no core, sampling whatever it
    /// publishes against its ceiling until the returned handle is dropped.
    ///
    /// `nvs run` is the caller this exists for: one request on one thread, with
    /// no CPU it is pinned to, no fleet to shed its share onto and an operator
    /// reading the same stderr the program writes. It takes no [`DeadlineView`]
    /// because it makes no stall report —
    /// `rule:http-server/a-wedged-core-is-detected-by-its-deadline` answers a
    /// worker that stopped turning, and the process that would be named by such
    /// a report here is the one already printing the `FATAL` the ceiling
    /// raised. What it does get is the half that answers a *request*:
    /// [`Registration::publish_safepoint`], sampled on the same walk and
    /// stopped at the same ceiling as any core's.
    pub fn register_requests(&self) -> Registration {
        self.watch(None)
    }

    fn watch(&self, core: Option<(CpuId, DeadlineView)>) -> Registration {
        let mut state = lock(&self.shared.state);
        let id = state.next_id;
        state.next_id += 1;
        state.watching.push(Watched {
            id,
            core,
            // A core registers when it installs its reactor, which is before it
            // has a request to be running — so the request arrives later, by
            // publication, and never as a second argument here.
            running: None,
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
            // On the registering thread's own stack, which is the one thing
            // this call can be sure of: `register` is called from the core it
            // watches, so the clock this handle charges every publication
            // against is that core's. The field doc owns why it is not taken
            // per request.
            clock: ThreadClock::current(),
        }
    }

    /// How many threads are registered.
    #[must_use]
    pub fn watching(&self) -> usize {
        lock(&self.shared.state).watching.len()
    }

    /// Whether any watched core was answering its deadlines at the last sweep.
    ///
    /// The read side of the episode [`Shared::sweep`] already keeps: a core
    /// counts as wedged for exactly as long as its earliest deadline stays a
    /// margin in the past, and the entry recording that is cleared on the first
    /// sweep where it is not. So this costs a walk of the watched set, lags by
    /// at most one interval, and needs nothing published on a request path for
    /// its benefit — which is what makes it the only honest evidence this
    /// process is doing its job rather than merely existing.
    ///
    /// **`true` while one core of many is wedged**, because that is
    /// `rule:http-server/a-wedged-core-is-shed-never-killed`: the fleet is
    /// still serving, and what answers the wedged core is admission control
    /// rather than anything that ends the process. It goes `false` only when
    /// every watched core is wedged at once, which is a process with nothing
    /// left to shed onto.
    ///
    /// `true` while no core is registered at all — a process whose fleet has
    /// not started yet, and the host that enumerates no CPU to pin to, are both
    /// answering every deadline they hold, which is none. A thread that is no
    /// core ([`Self::register_requests`]) is not counted on either side: it is
    /// reported as wedged never, so counting it would keep this `true` for a
    /// process whose every core had stopped.
    #[must_use]
    pub fn turning(&self) -> bool {
        let state = lock(&self.shared.state);
        let mut cores = 0_usize;
        let mut wedged = 0_usize;
        for watched in &state.watching {
            if watched.core.is_some() {
                cores += 1;
                wedged += usize::from(watched.reported.is_some());
            }
        }
        cores == 0 || wedged < cores
    }

    /// Every registered thread that has published a request, paired with what
    /// it published — the handle that stops that request, and the baseline a
    /// charge against its ceiling is measured from. The CPU is the one the
    /// publisher is pinned to, and `None` where it is no core.
    ///
    /// The read side of [`Registration::publish_safepoint`]. Cloned out under
    /// the lock and handed back owned, so whatever decides to stop a request —
    /// a sampler, a test — does that with the watched set released and cannot
    /// wedge a core registering beside it.
    #[must_use]
    pub fn running(&self) -> Vec<(Option<CpuId>, RunningRequest)> {
        lock(&self.shared.state)
            .watching
            .iter()
            .filter_map(|watched| {
                Some((
                    watched.core.as_ref().map(|&(cpu, _)| cpu),
                    watched.running.clone()?,
                ))
            })
            .collect()
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
    /// The clock of the thread that registered, which is the thread every
    /// request published through this handle runs on. Taken once here rather
    /// than once per publication because [`ThreadClock::current`] measures its
    /// caller and nothing else: a clock taken where a request is *built* would
    /// measure whichever thread built it, and one taken per request would pay a
    /// platform call for an answer that cannot have changed. `None` on a
    /// platform with no per-thread clock a stranger may read, which
    /// [`RunningRequest::new`] turns into a request that is sampled not at all.
    clock: Option<ThreadClock>,
}

impl Registration {
    /// Publishes the request this core is now running, or clears it with `None`
    /// when the core is running none.
    ///
    /// The handle inside must be the **tree root's** — `Ctx::safepoint_view`
    /// hands out no other — so one store through it reaches every isolate and
    /// task under it, including one spawned after the store. This module's docs
    /// say what the watchdog does with it, why it rides the watched set's own
    /// lock, and what the baseline beside it buys.
    ///
    /// Called from the core's own thread, so a request is published before it
    /// can wedge the core; a core that never publishes is watched exactly as it
    /// is today and stops nothing. [`RunningRequest::new`]'s `None` — a platform
    /// with no clock — belongs here unchanged: it clears whatever this core was
    /// running before, which is what a core taking up an unsampled request has
    /// to do.
    pub fn publish_safepoint(&self, running: Option<RunningRequest>) {
        let mut state = lock(&self.shared.state);
        if let Some(watched) = state.watching.iter_mut().find(|w| w.id == self.id) {
            watched.running = running;
        }
    }

    /// Publishes the request this thread is now running, charged from this
    /// registration's own clock.
    ///
    /// The same store [`Self::publish_safepoint`] makes, for the caller that
    /// holds the request rather than a [`RunningRequest`]: `view` is the tree
    /// root's handle, which carries the ceiling, and the baseline comes from
    /// the clock taken where this handle was made. A request under no cap is
    /// published too, because its program may narrow the ceiling later and the
    /// charge has to start here — [`RunningRequest::new`] owns that.
    ///
    /// Called from the thread that registered, like every other writer here:
    /// the ceiling is charged against *that* thread's clock, so publishing a
    /// request running anywhere else would charge it to a stranger.
    pub fn publish(&self, view: SafepointView) {
        self.publish_safepoint(RunningRequest::new(view, self.clock));
    }

    /// Clears whatever this thread was running, leaving it registered and
    /// sampled against no ceiling until it publishes again.
    ///
    /// What a thread owes when a request it published ends: the entry outlives
    /// the request, so a slot left filled would go on charging a finished tree
    /// for whatever the thread does next. Clearing it can only ever *stop*
    /// sampling, never start it, so a caller that clears one request's
    /// publication where another's still stands loses coverage and raises no
    /// flag.
    pub fn clear(&self) {
        self.publish_safepoint(None);
    }
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
    /// One pass over every registered thread: it stops each published request
    /// that has burned past its CPU ceiling, and answers with every *core* that
    /// is past its deadline by the margin and has not already been reported for
    /// that deadline.
    ///
    /// Takes `now` rather than reading the clock so that the whole deadline
    /// decision is testable without waiting for one. A CPU ceiling is not
    /// measured against `now` — it is `rule:errors/on-limit`'s cpu time and
    /// never wall clock — so that half reads the core's own clock here, once
    /// per core whose published request is under a cap and not at all for any
    /// other. That is the one read per core per interval
    /// `rule:http-server/a-wedged-core-is-detected-by-its-deadline` prices this
    /// thread at, and it stays on the walk the deadlines already cost.
    ///
    /// The stop is made **here, under the lock**, while a stall report is
    /// deliberately made by the caller once the lock is released: a report runs
    /// a sink, which is somebody else's code and may take as long as it likes,
    /// while a stop is one relaxed `fetch_or` into a word that cannot block, so
    /// handing it out to be made later would buy a core registering beside it
    /// nothing at all.
    fn sweep(&self, now: Instant) -> Vec<Stall> {
        let mut state = lock(&self.state);
        let margin = self.margin;
        let mut stalls = Vec::new();
        for watched in &mut state.watching {
            let past_its_ceiling = watched.running.as_ref().filter(|running| {
                // The ceiling first, so a request under no cap costs this walk
                // one load and no reading of its core's clock.
                running
                    .limit()
                    .is_some_and(|limit| running.burned().is_some_and(|burned| burned >= limit))
            });
            if let Some(running) = past_its_ceiling {
                // Both halves, because they are two different polls: the flag
                // is what compiled code reads between calls, and the deadline
                // is what `rule:http-server/time-is-bounded-inside-a-helper`'s
                // combinator reads from inside one call that is still running —
                // a request already inside a long member reaches no other.
                // `SafepointView::expire_deadline` owns the rest of that.
                //
                // Asked again on every sweep for as long as it stands, which
                // costs two stores and answers the case the flags exist for: a
                // request that polls stops at its next poll and its core
                // publishes something else, and one that does not poll is the
                // wedge the rest of this walk reports.
                running.view().request(SafepointFlags::CPU_LIMIT);
                running.view().expire_deadline();
            }
            // The other half answers a *core*, so an entry that is none — a
            // `nvs run`, which [`Watchdog::register_requests`] takes — is
            // sampled above and reported here never.
            let Some((cpu, view)) = &watched.core else {
                continue;
            };
            let cpu = *cpu;
            let overdue = view
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
                cpu,
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
    use nvs_runtime::{Ctx, OutputSink, SafepointFlags, TaskRoot};
    use std::sync::mpsc;

    fn a_cpu() -> CpuId {
        *cpus().first().expect("the platform reported no CPUs")
    }

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    /// A ceiling no case here comes near, in the nanoseconds `Ctx::cpu_limit`
    /// reports and `Ctx::set_cpu_limit` takes.
    fn a_minute() -> u64 {
        u64::try_from(Duration::from_secs(60).as_nanos()).expect("a minute does not fit a u64")
    }

    /// A request whose tree is charged against `nanos` of CPU time.
    fn capped(nanos: u64) -> Ctx {
        let mut request = ctx();
        request.set_cpu_limit(nanos);
        request
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

    /// `[server] watchdog_margin`'s half of the module: a margin the file wrote
    /// is the one a stall is measured against, on the constructor `nvs serve`
    /// builds the process's watchdog with, and [`DEFAULT_MARGIN`] plays no part.
    #[test]
    fn a_configured_margin_is_the_one_a_stall_is_reported_against() {
        let margin = Duration::from_secs(2);
        assert!(
            margin < DEFAULT_MARGIN,
            "the case needs a margin the default would not report at"
        );
        let dog = Watchdog::reporting_after(margin);
        assert_eq!(dog.margin(), margin);
        let mut timers = Timers::default();
        let filed = Instant::now() + Duration::from_secs(1);
        timers.arm(TaskId::from_raw(1), filed);
        let _watched = dog.register(a_cpu(), timers.view());

        assert!(
            dog.shared
                .sweep(filed + margin - Duration::from_millis(1))
                .is_empty()
        );
        let stalls = dog.shared.sweep(filed + margin);
        assert_eq!(
            stalls.len(),
            1,
            "a core past the configured margin was not reported"
        );
        assert_eq!(stalls[0].overdue_by, margin);
        assert_eq!(Watchdog::new().margin(), DEFAULT_MARGIN);
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

    /// The gate a service manager's watchdog ping hangs off: one wedged core of
    /// several is a fleet that is still serving, and only a fleet where no core
    /// turns at all stops answering for the process.
    #[test]
    fn a_process_turns_until_every_core_it_watches_is_wedged_at_once() {
        let (tx, _rx) = mpsc::channel();
        let margin = Duration::from_secs(5);
        let dog = watchdog_of(margin, tx);
        assert!(
            dog.turning(),
            "a process watching no core is answering every deadline it holds, which is none"
        );

        // Strictly after each table's publishing base, for the reason the case
        // below this one spells out.
        let filed = Instant::now() + Duration::from_millis(1);
        let mut one = Timers::default();
        let mut two = Timers::default();
        one.arm(TaskId::from_raw(1), filed);
        let _first = dog.register(a_cpu(), one.view());
        let _second = dog.register(a_cpu(), two.view());

        dog.shared.sweep(filed + margin);
        assert!(
            dog.turning(),
            "one wedged core of two is shed rather than killed, and the other is still accepting"
        );

        two.arm(TaskId::from_raw(2), filed);
        dog.shared.sweep(filed + margin);
        assert!(
            !dog.turning(),
            "every watched core is a margin past its deadline and the process still reads as \
             turning"
        );

        // A core that answers again ends its episode, and the process reads as
        // turning on the next sweep rather than staying condemned by one.
        assert!(two.disarm(TaskId::from_raw(2)));
        dog.shared.sweep(filed + margin);
        assert!(
            dog.turning(),
            "a core that came back was not enough to make the process read as turning again"
        );
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
        // `Instant::now()` calls can return the same value, so `start` has to
        // be strictly later rather than a second reading of the clock.
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
        // a deadline unanswered for more than the sleep that armed it. Each
        // wake measures how late it came: a loaded machine can keep this thread
        // off every CPU for longer than the margin, and then the core really
        // did stall and a report is the right answer.
        let (late_tx, late_rx) = mpsc::channel();
        let busy = Arc::clone(&dog);
        let working = Worker::spawn(cpu, move |sched| {
            let _installed = install(Reactor::new().expect("the OS refused a poll"));
            let view =
                with_current(|reactor| reactor.deadline_view()).expect("no reactor installed");
            let _watched = busy.register(cpu, view);
            sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
                for _ in 0..5 {
                    // At or before the instant `sleep` files, so a lateness
                    // measured from it is never an underestimate.
                    let due = Instant::now() + Duration::from_millis(100);
                    sleep(Duration::from_millis(100));
                    let _ = late_tx.send(Instant::now().saturating_duration_since(due));
                }
            });
            run_until_idle(sched).expect("the loop failed")
        })
        .expect("the OS refused a thread");
        assert_eq!(working.join().expect("the worker panicked").finished, 1);
        let latest = late_rx.try_iter().max().expect("the busy task never woke");
        let reported = rx.try_iter().count();
        // A sweep reads its clock before it reads a core's deadline, so a
        // report means a deadline was still filed a whole margin after it fell
        // due. The reactor takes a deadline off before the task it wakes runs,
        // so a task that woke sooner than that from every sleep filed none.
        if latest < margin {
            assert_eq!(
                reported, 0,
                "a core that answered every deadline was reported as wedged"
            );
        }

        // And a core that files a deadline and then never polls again — § 7's
        // own example, a loop with no poll site in it. It stays wedged until
        // the test has its report, however long the watchdog thread waits for
        // a CPU; dropping `release` is what lets it go.
        let started = Instant::now();
        let (release, held) = mpsc::channel::<()>();
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
            let _ = held.recv();
        })
        .expect("the OS refused a thread");

        // The first half's core may have been reported late, after the drain
        // above, and every deadline it filed fell due before `started`. The
        // bound only turns a watchdog that never reports into a failure rather
        // than a hang.
        let stall = loop {
            let stall = rx
                .recv_timeout(Duration::from_secs(60))
                .expect("the wedged worker was never reported");
            if stall.deadline > started {
                break stall;
            }
        };
        drop(release);
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

    /// What a core publishes is the request *tree's* root word, so one store
    /// from a thread that owns none of it stops an isolate built after that
    /// store — `rule:security/isolate-shares-nothing`.
    #[test]
    fn a_published_handle_stops_the_whole_request_tree() {
        let (tx, _rx) = mpsc::channel();
        // A margin nothing reaches: this case is about the handle, and a stall
        // report would only add a second reason for the sink to fire.
        let dog = watchdog_of(Duration::from_secs(3600), tx);
        let timers = Timers::default();
        let registered = dog.register(a_cpu(), timers.view());
        assert!(
            dog.running().is_empty(),
            "a core running no request still offered a handle to stop one"
        );

        let request = capped(a_minute());
        let Some(running) = RunningRequest::new(request.safepoint_view(), ThreadClock::current())
        else {
            // This platform has no per-thread clock, so it enforces no CPU
            // ceiling and a core on it publishes nothing — `crate::cpuclock`'s
            // docs own that answer, and nothing about a handle can be asserted
            // where no handle is offered.
            return;
        };
        registered.publish_safepoint(Some(running));
        let published = dog.running();
        assert_eq!(published.len(), 1, "the published handle was not readable");
        assert_eq!(published[0].0, Some(a_cpu()));

        // Raised from this thread, which owns neither the request nor the `&mut
        // Ctx` — the whole reason the word is reachable through a handle.
        published[0].1.view().request(SafepointFlags::CPU_LIMIT);
        assert!(
            request
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT)
        );
        assert!(
            published[0]
                .1
                .view()
                .flags()
                .contains(SafepointFlags::CPU_LIMIT)
        );
        let isolate = request.isolate(OutputSink::Sink);
        assert!(
            isolate
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT),
            "an isolate built after the store was born clean"
        );

        registered.publish_safepoint(None);
        assert!(
            dog.running().is_empty(),
            "a core that finished its request kept offering the handle to it"
        );
    }

    /// A publication is charged against the clock of the thread that
    /// *registered*, which is the one thing a handle can be sure of about the
    /// requests published through it: [`Registration::publish`] asks its caller
    /// for no clock, so no caller can hand it one belonging to another thread.
    #[test]
    fn a_registration_publishes_against_the_clock_it_was_made_with() {
        let (tx, _rx) = mpsc::channel();
        let dog = watchdog_of(Duration::from_secs(3600), tx);
        let timers = Timers::default();
        let registered = dog.register(a_cpu(), timers.view());

        let request = capped(a_minute());
        registered.publish(request.safepoint_view());
        let published = dog.running();
        if ThreadClock::current().is_none() {
            // No per-thread clock here, so this core enforces no ceiling and
            // publishes nothing — `crate::cpuclock`'s docs own that answer.
            assert!(published.is_empty(), "a core with no clock published one");
            return;
        }
        assert_eq!(published.len(), 1, "the published request was not readable");
        assert_eq!(
            published[0].1.limit(),
            Some(Duration::from_nanos(a_minute()))
        );
        // The baseline is this thread's own reading, taken at the registration
        // and not at some zero: a clock read from the watchdog's thread would
        // charge this request whatever *that* thread had burned since it
        // started.
        assert!(
            published[0]
                .1
                .burned()
                .is_some_and(|burned| burned < Duration::from_secs(1)),
            "the baseline was not taken on the thread that registered"
        );

        registered.clear();
        assert!(
            dog.running().is_empty(),
            "a cleared registration still offered a request to stop"
        );
    }

    /// A request under no cap is published like any other and compared with
    /// nothing, and a ceiling its program narrows afterwards is charged from
    /// the baseline it was published with — what `Core\Config::set` does to
    /// `limits.cpu_time` in a request that started uncapped,
    /// `rule:errors/on-limit`.
    #[test]
    fn a_request_published_under_no_cap_is_stopped_at_a_ceiling_narrowed_later() {
        let (tx, _rx) = mpsc::channel();
        let dog = watchdog_of(Duration::from_secs(3600), tx);
        let registered = dog.register_requests();
        if ThreadClock::current().is_none() {
            // No per-thread clock here, so nothing is sampled at all —
            // `crate::cpuclock`'s docs own that answer.
            return;
        }

        let mut request = ctx();
        registered.publish(request.safepoint_view());
        let published = dog.running();
        assert_eq!(published.len(), 1, "an uncapped request was not published");
        assert_eq!(published[0].1.limit(), None);
        let baseline = published[0].1.baseline();

        burn_a_charge();
        assert!(dog.shared.sweep(Instant::now()).is_empty());
        assert!(
            !request
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT),
            "a request under no cap was stopped at one"
        );

        // Narrowed where it stands and published by nobody again. A millisecond
        // is far under what was burned above and far over what the lines since
        // have cost, so the request is past it only if its charge still starts
        // where the request did.
        request.set_cpu_limit(1_000_000);
        assert!(dog.shared.sweep(Instant::now()).is_empty());
        assert!(
            request
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT),
            "a ceiling narrowed after the publication was charged from a later start, or not at all"
        );
        assert_eq!(
            dog.running()[0].1.baseline(),
            baseline,
            "narrowing the ceiling moved where the charge starts"
        );
    }

    /// `nvs run`'s side of the watched set: it is stopped at its ceiling like
    /// any core's request, and the report a core would get is one a run never
    /// makes — the stderr such a record would land on is the program's own.
    #[test]
    fn a_run_that_is_no_core_is_sampled_and_reported_never() {
        let (tx, rx) = mpsc::channel();
        let dog = watchdog_of(Duration::from_secs(3600), tx);
        let registered = dog.register_requests();

        // One nanosecond: the ceiling is passed by the time the publication
        // returns, so this case asserts the walk rather than the clock.
        let request = capped(1);
        let Some(running) = RunningRequest::new(request.safepoint_view(), ThreadClock::current())
        else {
            // No per-thread clock here, so nothing is sampled at all —
            // `a_published_handle_stops_the_whole_request_tree` owns why that
            // is a return rather than a failure.
            return;
        };
        registered.publish_safepoint(Some(running));
        let published = dog.running();
        assert_eq!(published.len(), 1, "the published request was not readable");
        assert_eq!(
            published[0].0, None,
            "a run named a CPU it is not pinned to"
        );

        burn_a_charge();
        assert!(
            dog.shared.sweep(Instant::now()).is_empty(),
            "a thread that is no core was reported as a wedged one"
        );
        assert!(
            request
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT),
            "a run past its ceiling was not stopped"
        );
        assert!(
            rx.try_recv().is_err(),
            "a run with no core still reached the stall sink"
        );
    }

    /// What a core publishes is what a ceiling can be charged against: the
    /// baseline its charge is measured from, beside a handle that reads the
    /// ceiling its tree states — `rule:errors/on-limit`.
    #[test]
    fn a_published_request_carries_a_baseline_and_reads_its_trees_ceiling() {
        let (tx, _rx) = mpsc::channel();
        let dog = watchdog_of(Duration::from_secs(3600), tx);
        let timers = Timers::default();
        let registered = dog.register(a_cpu(), timers.view());
        let request = capped(a_minute());

        let Some(running) = RunningRequest::new(request.safepoint_view(), ThreadClock::current())
        else {
            // The platform's own answer, as the case above this one gives it.
            return;
        };
        let limit = running.limit().expect("a capped request read as uncapped");
        assert_eq!(limit, Duration::from_secs(60));
        let baseline = running.baseline();
        registered.publish_safepoint(Some(running));

        let published = dog.running();
        let (_, running) = published
            .first()
            .expect("the published request was not readable");
        assert_eq!(
            running.baseline(),
            baseline,
            "what the watchdog read back was charged from a different instant"
        );
        let before = running.burned().expect("a live core read as no thread");

        burn_a_charge();

        let after = running.burned().expect("a live core read as no thread");
        assert!(
            after > before,
            "a core that spun was charged nothing against the request it published"
        );
        assert!(
            after < limit,
            "a hundred milliseconds of spinning reached a ceiling of a minute"
        );
    }

    /// The sampler itself, on both sides of the ceiling: the walk that reads
    /// deadlines also reads the CPU clock, and the request that has burned past
    /// `[limits] cpu_time` is the only one asked to stop — `rule:errors/on-limit`.
    #[test]
    fn the_walk_stops_a_request_past_its_cpu_ceiling_and_leaves_one_under_it() {
        let (tx, _rx) = mpsc::channel();
        // A margin nothing reaches: this case is about the clock, and a stall
        // report would only add a second reason for the sink to fire.
        let dog = watchdog_of(Duration::from_secs(3600), tx);
        let timers = Timers::default();
        let registered = dog.register(a_cpu(), timers.view());
        let mut request = capped(a_minute());

        let Some(under) = RunningRequest::new(request.safepoint_view(), ThreadClock::current())
        else {
            // The platform's own answer: no per-thread clock, so no CPU
            // ceiling — `crate::cpuclock`'s docs own it.
            return;
        };
        registered.publish_safepoint(Some(under));
        burn_a_charge();
        assert!(dog.shared.sweep(Instant::now()).is_empty());
        assert!(
            !request
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT),
            "a request a long way under its ceiling was asked to stop"
        );
        assert!(
            !request.deadline_expired(),
            "a request a long way under its ceiling had its deadline expired"
        );

        // Narrowed where it stands, and the same publication is charged against
        // the new ceiling — a nanosecond's rather than a real one, because what
        // is under test is the comparison and spinning to a ceiling worth
        // configuring would price this case in seconds.
        request.set_cpu_limit(1);
        assert!(dog.shared.sweep(Instant::now()).is_empty());
        assert!(
            request
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT),
            "a request past its ceiling ran on with a clean safepoint word"
        );
        assert!(
            request.deadline_expired(),
            "a request past its ceiling was left the poll a long member makes"
        );
    }

    /// The distinction the ceiling exists for, from the other side: a request
    /// waiting on the world spends wall time and none of what is sampled, so a
    /// wait several times its own ceiling is left alone where a spin that
    /// length is stopped — `rule:errors/on-limit`, and `crate::cpuclock`'s
    /// *CPU time, never wall clock*.
    #[test]
    fn a_core_parked_on_io_past_the_same_wall_time_is_not_flagged() {
        let (tx, _rx) = mpsc::channel();
        // A margin nothing reaches: a core parked on a socket answers its
        // deadlines, and a stall report would only add a second reason for the
        // sink to fire.
        let dog = watchdog_of(Duration::from_secs(3600), tx);
        let timers = Timers::default();
        let registered = dog.register(a_cpu(), timers.view());
        // Fifty milliseconds, passed several times over by the wait below in
        // wall time and not at all in CPU time. A sleep rather than a socket:
        // what the clock sees of either is the same nothing, and a socket would
        // put a reactor between this case and the reading under test.
        let ceiling = Duration::from_millis(50);
        let request = capped(
            u64::try_from(ceiling.as_nanos()).expect("fifty milliseconds does not fit a u64"),
        );
        let Some(parked) = RunningRequest::new(request.safepoint_view(), ThreadClock::current())
        else {
            // The platform's own answer: no per-thread clock, so no CPU
            // ceiling — `crate::cpuclock`'s docs own it.
            return;
        };
        registered.publish_safepoint(Some(parked));
        std::thread::sleep(ceiling * 4);

        assert!(dog.shared.sweep(Instant::now()).is_empty());
        assert!(
            !request
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT),
            "a request that only waited was stopped as a runaway"
        );
        assert!(
            !request.deadline_expired(),
            "a request that only waited was left the poll a long member makes"
        );
    }

    /// CPU time rather than wall time: a sleep accumulates none of what the
    /// ceiling measures, so a case that needs a charge spins until its own
    /// clock shows one. `crate::cpuclock::burn_at_least` owns why the clock,
    /// and never a wall interval, is what the spin waits on.
    fn burn_a_charge() {
        crate::cpuclock::burn_at_least(Duration::from_millis(20));
    }
}
