//! `rule:testing/debug-probes`'s
//! probes, and what a request records for them.
//!
//! The probe sites are emitted unconditionally and cost a load and a
//! predicted-not-taken branch while [`DebugFlags`] is empty, which is the whole
//! of why coverage and tracing can be turned on *mid-request*. This file holds
//! both ends: [`nvs_probe_stmt`], [`nvs_probe_edge`], [`nvs_probe_call_enter`] and
//! [`nvs_probe_call_exit`], and the [`Ctx`] methods they call into.
//!
//! [`FaultSite`] rides along because it is the same shape — a site compiled in,
//! armed by a test, and fired at most once.

use super::*;

use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The one clock every [`TraceEvent`] is read on: the moment this process
/// first timed an event, as a monotonic reading and a wall reading taken
/// together.
///
/// An [`Instant`] has no absolute rendering and a per-request epoch would give
/// every request a timeline of its own, which is exactly the join
/// `rule:observability/spawn-is-its-own-event`'s export-time nesting has to
/// make. The wall reading is what puts a reading on a collector's clock
/// ([`unix_nanos`]); events are timed on the monotonic one, so a wall clock
/// stepped mid-run moves no event against another. **What it spends:** one
/// `Instant` and one `SystemTime` per process, initialised by the first event
/// that is timed at all, so a run that records nothing never reads a clock for
/// it.
static EPOCH: LazyLock<Epoch> = LazyLock::new(|| Epoch {
    monotonic: Instant::now(),
    wall: SystemTime::now(),
});

/// [`EPOCH`]'s two readings.
struct Epoch {
    /// What every event's start and end are measured from.
    monotonic: Instant,
    /// The wall clock at that same moment.
    wall: SystemTime,
}

/// A reading of the trace clock: the time since [`EPOCH`], which is what a
/// [`TraceEvent`]'s start and end are.
///
/// Public because the filing sites outside this crate — the root span a door
/// derives — read the same clock the events are on, and a second clock would
/// put a root's window beside its children's rather than around them.
#[must_use]
pub fn trace_clock() -> Duration {
    EPOCH.monotonic.elapsed()
}

/// `reading`, a [`trace_clock`] reading, as nanoseconds since the Unix epoch —
/// how OTLP stamps a span's start and end.
///
/// A reading past what a `u64` of nanoseconds holds saturates, and a wall clock
/// set before 1970 reads as `0`; both are a clock this side cannot correct.
#[must_use]
pub fn unix_nanos(reading: Duration) -> u64 {
    EPOCH
        .wall
        .checked_add(reading)
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| {
            u64::try_from(since.as_nanos()).unwrap_or(u64::MAX)
        })
}

/// A failure a run can be *asked* to produce, for a mode that by definition
/// has no user-facing trigger.
///
/// The set is closed on purpose, and reachable only through `nvs run
/// --fault-inject=<site>`: it must never be reachable from a served request
/// (`nvs serve`), and nothing in Novis source can arm one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FaultSite {
    /// The request's **second** runtime helper call panics, which
    /// [`crate::run_helper`]'s `catch_unwind` contains into a `FATAL`.
    ///
    /// Deliberately not the *first*: `echo` is itself a helper, so faulting
    /// the very first call would give a run that produced no output at all —
    /// and half of what containing an engine panic means is that what the
    /// request already produced survives it. Two is the smallest count that
    /// leaves room for a byte to have been written.
    ///
    /// It is a fixed count rather than a condition on the output for one
    /// reason: an injected fault that can silently never fire is worse than
    /// one whose site reads slightly arbitrarily. A request that enters fewer
    /// than two helpers does nothing observable at all, so there is no
    /// program this leaves un-faulted that anyone would want to fault.
    HelperPanic,
}

/// Which of
/// `rule:observability/trace-events-carry-a-kind`
/// 's kinds a [`TraceEvent`] is.
///
/// The tag is the whole of the distinction here, and deliberately so: that rule keeps
/// a `call` event's shape exactly as `rule:testing/debug-probes` defines it, and the other
/// kinds carry facts of their own that this stand-in vector has nowhere to put.
/// A `query`'s field set is fixed by `rule:observability/a-query-is-a-trace-event`
/// and lives in `nvs_db::QuerySpan`, and an `http`'s by the same rule as this
/// one in `Core\Http\Client`'s own span — each in the crate that is already
/// holding the facts; a per-kind payload is what
/// `rule:observability/speedscope-timeline-export` needs and what lands with
/// `rule:testing/debug-probes`'s sink, alongside the `PROFILE` timing the `trace` field's own doc
/// comment defers for the same reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceKind {
    /// A call site's entry or exit — `rule:testing/debug-probes`'s probe pair, and with
    /// [`TraceKind::Query`] one of the kinds anything in the tree records.
    Call,
    /// A cycle-collector pause — `rule:observability/gc-pause-is-its-own-event`,
    /// filed by [`Ctx::close_gc_pause`] from inside the run
    /// [`Ctx::collect_if_asked`] brackets, with the start, the duration and the
    /// number of objects freed.
    Gc,
    /// A spawned child's start, filled in at its join —
    /// `rule:observability/spawn-is-its-own-event`, opened by [`Ctx::open_spawn`]
    /// and completed by [`Ctx::close_spawn`], for each of [`SpawnForm`]'s forms.
    Spawn,
    /// One statement, filed from inside a driver's own statement routine —
    /// `rule:observability/trace-events-carry-a-kind` and `rule:observability/a-query-is-a-trace-event`.
    Query,
    /// One outbound call, filed from `Core\Http\Client`'s transport — the fifth
    /// of `rule:observability/trace-events-carry-a-kind`'s kinds, and **one
    /// event per call whatever its attempt count**, which is the same rule's
    /// sentence and what makes this the event an outbound span is derived from
    /// (`rule:observability/four-kinds-become-a-span`).
    Http,
}

/// One `rule:testing/debug-probes` call-site trace record, tagged with
/// `rule:observability/trace-events-carry-a-kind`
/// 's kind.
///
/// `callee` and `status` are the `call` kind's shape, and a `query` and an
/// `http` reuse the first of them rather than adding a field per kind —
/// [`Ctx::record_query`] owns that reasoning, and [`TraceKind`]'s doc comment
/// owns what a per-kind payload waits on. `start` and `end` are every kind's,
/// on the one clock [`trace_clock`] reads.
///
/// **What it spends:** two `Duration`s per event, and one clock read per event
/// at the site that files it — only while something records events, so a
/// request with every bit off pays nothing for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceEvent {
    /// Which of `rule:observability/trace-events-carry-a-kind`'s kinds this is.
    pub kind: TraceKind,
    /// What the event is *of*: a [`TraceKind::Call`]'s callee as a
    /// `Class::method` label, a [`TraceKind::Query`]'s span as the driver
    /// rendered it, and a [`TraceKind::Http`]'s as the transport did — see
    /// [`Ctx::record_query`] for why one field carries all of them.
    pub callee: String,
    /// `None` on entry; on exit, the status the call site is about to branch
    /// on — so a trace records a thrown or `FATAL` exit exactly as it
    /// happened rather than as a reconstruction.
    pub status: Option<i32>,
    /// When the event began, as a [`trace_clock`] reading. A `call` entry and
    /// a `call` exit are each an instant, so their start is their end.
    pub start: Duration,
    /// When it ended, on the same clock: a `query` and an `http` when the
    /// routine filed it, a `gc` when the walk finished, and a `spawn` at its
    /// join. A spawn that was never joined ends where it started.
    pub end: Duration,
}

/// Which of `rule:observability/spawn-is-its-own-event`'s constructs a
/// [`TraceKind::Spawn`] event is of.
///
/// The tag is the rule's own spelling and carries nothing else, because the
/// forms differ in where the child runs and in nothing this event records:
/// `spawn` is a `Core\Task` child over the parent's heap, `spawn script` an
/// isolate on this core, `spawn worker` an isolate started on another, and
/// `Core\Process::spawn` a child of the operating system's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnForm {
    /// `spawn` — a `Core\Task` child, a task on this core.
    Task,
    /// `spawn worker` — an isolate started on another core.
    Worker,
    /// `spawn script` — an isolate on this core.
    Script,
    /// `Core\Process::spawn` — a child process, which the operating system
    /// starts rather than the runtime.
    ///
    /// It reports no wall time of its own, so its join carries the
    /// parent-observed wall alone where an isolate's carries the split, and
    /// its join is the handle's `wait` (`rule:core-classes/process-spawn`).
    Process,
}

impl SpawnForm {
    /// The construct as the language spells it, which is what the event
    /// carries.
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Task => "spawn",
            Self::Worker => "spawn worker",
            Self::Script => "spawn script",
            Self::Process => r"Core\Process::spawn",
        }
    }
}

/// A spawn that has been recorded and not yet joined — [`Ctx::open_spawn`]'s
/// answer, and what [`Ctx::close_spawn`] closes.
///
/// It carries the event's own position, so a spawn is one record written twice
/// rather than two records a consumer has to pair up: the start is filed where
/// the child is started, in among the request's other events, and the join
/// fills in the rest of that same record.
#[derive(Debug)]
#[must_use = "a spawn that is opened and never closed leaves an event with no join"]
pub struct OpenSpawn {
    /// Which construct started the child.
    form: SpawnForm,
    /// Where in this request's trace the event sits.
    index: usize,
    /// When the child was started, as a [`trace_clock`] reading.
    started: Duration,
}

/// [`Ctx::close_spawn`]'s payload, as a function of the numbers alone.
///
/// The fields are rendered for [`Ctx::record_query`]'s reason — [`TraceEvent`]
/// is a stand-in until `rule:testing/debug-probes`'s sink gives every kind a
/// payload of its own — and both subtractions saturate, because a child that
/// reports more wall time than its parent observed is a clock this side cannot
/// correct and a negative overhead would read as one.
fn render_spawn(
    form: SpawnForm,
    started: Duration,
    joined: Duration,
    child: Option<Duration>,
) -> String {
    let wall = joined.saturating_sub(started);
    let split = match child {
        Some(child) => format!(" child={child:?} overhead={:?}", wall.saturating_sub(child)),
        None => String::new(),
    };
    format!(
        "{} started={started:?} joined={joined:?} wall={wall:?}{split}",
        form.spelling()
    )
}

/// How many of `rule:observability/four-kinds-become-a-span`'s events a
/// recorded or traced run files before it stops filing them.
///
/// **This is the one home of that number**, and `nvs_server::trace::SPAN_CEILING`
/// — the span count a derived graph carries — is the root plus this. The bound is
/// here because this is where an event is filed and where the memory is held: a
/// request keeps its events until it ends, so an unbounded count would make a
/// sampled request's footprint a function of how many statements it ran, and a
/// program that queries in a loop would be the one paying for it.
///
/// A fixed count rather than a share of anything, so that the worst case is
/// arithmetic an operator can do: at most this many rendered spans per *sampled
/// in-flight* request, each of them a line the driver or the transport already
/// bounds. Past it a sampled run renders nothing further and files nothing
/// further, and the run itself is untouched — a trace is an observation of a
/// request and never a limit on one.
///
/// A run under `DebugFlags::TRACE` is held to it too. Its `call` events, the
/// ones that grow with the call count, stream to a [`TraceSink`] and are never
/// filed here, so the ceiling truncates no call.
pub const SPAN_EVENT_CEILING: usize = 512;

/// Whether every context this process makes starts with [`DebugFlags::SITES`] on, which
/// [`probe_every_site`] sets once, before anything runs.
static EVERY_SITE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Turns [`DebugFlags::SITES`] on in every context this process makes from now on —
/// `nvs test --probes`'s configuration (`rule:testing/probes-on-is-a-tested-configuration`).
/// Process-wide, as `nvs_codegen::set_opt_level` is, so a context the host makes anywhere in
/// the run has it, and no host call can leave it out. Request path: one relaxed load per
/// context made.
pub fn probe_every_site() {
    EVERY_SITE.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Whether [`probe_every_site`] ran in this process.
#[must_use]
pub fn every_site_probed() -> bool {
    EVERY_SITE.load(std::sync::atomic::Ordering::Relaxed)
}

/// The flags a new context starts with: [`DebugFlags::SITES`] after [`probe_every_site`], and
/// none otherwise.
pub(crate) fn starting_debug_flags() -> DebugFlags {
    if every_site_probed() {
        DebugFlags::SITES
    } else {
        DebugFlags::empty()
    }
}

impl Ctx {
    /// Whether this run files the events
    /// `rule:observability/four-kinds-become-a-span`'s spans are derived from —
    /// the one question every filing site asks, and the one the two sites that
    /// *render* a span ask before paying for the rendering.
    ///
    /// Two states answer `true`, and they are different questions rather than
    /// one flag with two names. **The trace is recorded**: the head draw
    /// (`rule:observability/sampling-is-head-based`) went that way, so a
    /// collector is going to be handed this request's graph. **A debugger is
    /// attached**: [`DebugFlags::TRACE`] is on, which additionally streams a
    /// `call` per compiled call site to the trace's sink — the cost
    /// `rule:observability/four-kinds-become-a-span` refuses on a served
    /// request outright, and the reason the two cannot be one bit. A sampled
    /// request never pays it, because the call probes read that bit and this
    /// answer is not it.
    ///
    /// A third state answers `true` too. **The request is stored**: `[debug]
    /// store` is in force, so its bounded span events go to the spool whatever
    /// the head draw said
    /// (`rule:testing/writing-a-debug-record-to-disk-is-its-own-gate`). It is
    /// asked last, so a sampled or traced run never reads the configuration
    /// for it.
    ///
    /// The ceiling is asked here rather than beside each site, so that a filer
    /// cannot skip it: past [`SPAN_EVENT_CEILING`] filed events every state
    /// answers `false`.
    #[must_use]
    pub fn records_spans(&self) -> bool {
        // The vector holds nothing but these events — a call goes to the sink —
        // so its length is the filed-event count this ceiling is about, and no
        // second counter has to be kept true.
        (self.trace_context.sampled()
            || self.debug.contains(DebugFlags::TRACE)
            || self.config().is_some_and(nvs_config::Request::debug_store))
            && self.trace.len() < SPAN_EVENT_CEILING
    }

    /// Counts one hit for the statement `stmt` names in this context's
    /// [`StmtHits`] table — [`nvs_probe_stmt`]'s whole effect under
    /// [`DebugFlags::COVERAGE`]. A context with no table counts nothing.
    pub fn record_stmt_hit(&self, stmt: u32) {
        if let Some(hits) = &self.stmt_hits {
            hits.record(stmt);
        }
    }

    /// Counts one hit for the conditional edge `edge` names in this context's
    /// [`StmtHits`] table — [`nvs_probe_edge`]'s whole effect under
    /// [`DebugFlags::BRANCH`]. A context with no table counts nothing.
    pub fn record_edge_hit(&self, edge: u32) {
        if let Some(hits) = &self.stmt_hits {
            hits.record_edge(edge);
        }
    }

    /// Gives this context the table [`DebugFlags::COVERAGE`] counts into, or
    /// takes it away with `None`. Every child context made from this one
    /// afterwards shares the same table.
    pub fn set_stmt_hits(&mut self, hits: Option<Arc<StmtHits>>) {
        self.stmt_hits = hits;
    }

    /// The table [`DebugFlags::COVERAGE`] counts into, when one was given.
    #[must_use]
    pub fn stmt_hits(&self) -> Option<&Arc<StmtHits>> {
        self.stmt_hits.as_ref()
    }

    /// Gives this context the query site table of the program it runs. Every task made from this
    /// one afterwards shares it, and an isolate is given its own program's.
    pub fn set_query_sites(&mut self, sites: Arc<QuerySites>) {
        self.query_sites = Some(sites);
    }

    /// The query site table the host gave this context, when it gave one.
    #[must_use]
    pub fn query_sites(&self) -> Option<&Arc<QuerySites>> {
        self.query_sites.as_ref()
    }

    /// The query call site that `id` names, or `None` where the host gave this context no
    /// table, and for the zero id.
    #[must_use]
    pub fn query_site(&self, id: u32) -> Option<crate::spool::QuerySite> {
        self.query_sites.as_ref()?.at(id)
    }

    /// The statements executed, the compiled call sites entered and the
    /// collections run while [`DebugFlags::COUNT`] was on — [`nvs_probe_stmt`]'s
    /// and [`nvs_probe_call_enter`]'s whole effect under that bit, and one
    /// increment in `Ctx::collect_if_asked`. All three are zero for a request
    /// that ran with it off throughout.
    ///
    /// A call is a call the program made: a compiled call site, a `Core`
    /// member's call, and every call through a `callable`, a callback a `Core`
    /// member makes included. A helper the compiler calls on its own is not
    /// one. What a `Core` member does inside is visible only to the
    /// allocator's own counters (`budget::allocations`) and to the clock.
    #[must_use]
    pub fn counted(&self) -> Counted {
        Counted {
            statements: self.counted_stmts,
            calls: self.counted_calls,
            gc_cycles: self.counted_gc_cycles,
        }
    }

    /// Counts one in-flight collection under [`DebugFlags::COUNT`]. A
    /// collection runs only where an allocation crossed the memory ceiling, and
    /// both are decided by what the program allocated, so the count is the same
    /// on every run of the same program.
    pub(crate) fn count_gc_cycle(&mut self) {
        if self.debug.contains(DebugFlags::COUNT) {
            self.counted_gc_cycles += 1;
        }
    }

    /// [`nvs_probe_call_enter`]'s effect for a call the runtime makes rather
    /// than compiled code: a call through a `callable`
    /// (`crate::callable::call_callable`), whose callee has no name the call
    /// site could hold. `callee` builds the label, and runs only under
    /// [`DebugFlags::TRACE`], so a call with every bit off pays one flags test.
    pub(crate) fn enter_runtime_call(&mut self, callee: impl FnOnce() -> String) {
        if self
            .debug
            .intersects(DebugFlags::TRACE | DebugFlags::PROFILE)
        {
            let label = callee();
            if self.debug.contains(DebugFlags::TRACE) {
                self.record_trace(&label, None);
            }
            if self.debug.contains(DebugFlags::PROFILE) {
                self.profile_enter(&label);
            }
        }
        if self.debug.contains(DebugFlags::COUNT) {
            self.counted_calls += 1;
        }
    }

    /// [`nvs_probe_call_exit`]'s effect for the call
    /// [`Self::enter_runtime_call`] entered, with the status the callee
    /// returned.
    pub(crate) fn exit_runtime_call(&mut self, callee: impl FnOnce() -> String, status: i32) {
        if self.debug.contains(DebugFlags::TRACE) {
            self.record_trace(&callee(), Some(status));
        }
        if self.debug.contains(DebugFlags::PROFILE) {
            self.profile_exit();
        }
    }

    /// A call to `callee` begins, in the running profile — [`nvs_probe_call_enter`]'s effect
    /// under [`DebugFlags::PROFILE`]. With no profile running it reads no clock.
    pub fn profile_enter(&mut self, callee: &str) {
        if let Some(track) = &mut self.debug_sinks.profile {
            track.enter(callee);
        }
    }

    /// The innermost call in the running profile returns — [`nvs_probe_call_exit`]'s effect
    /// under [`DebugFlags::PROFILE`].
    pub fn profile_exit(&mut self) {
        if let Some(track) = &mut self.debug_sinks.profile {
            track.exit();
        }
    }

    /// Records one call's entry (`status` is `None`) or its exit —
    /// [`nvs_probe_call_enter`]/[`nvs_probe_call_exit`]'s whole effect under
    /// [`DebugFlags::TRACE`].
    ///
    /// The call goes to the [`TraceSink`] `Core\Debug::startTrace` opened, as
    /// one line when it returns, and never into [`Ctx::trace`]. With no sink it
    /// is written nowhere and reads no clock.
    pub fn record_trace(&mut self, callee: &str, status: Option<i32>) {
        if let Some(sink) = &mut self.debug_sinks.trace {
            match status {
                None => sink.enter(),
                Some(status) => sink.exit(callee, status),
            }
        }
    }

    /// The window a routine that timed itself files: it ends now, and began
    /// `took` before. The routine's own duration is the start rather than a
    /// second clock read where it began, which a `query` and an `http` would
    /// otherwise have to carry here through a crate that has no `Ctx`.
    fn ended_after(took: Duration) -> (Duration, Duration) {
        let end = trace_clock();
        (end.saturating_sub(took), end)
    }

    /// Records one statement as
    /// `rule:observability/trace-events-carry-a-kind`
    /// 's `query` event — `Core\Db`'s statement routines' whole effect under
    /// [`Ctx::records_spans`], called once the rows have ended so the span is
    /// complete.
    ///
    /// The caller asks that question and this does not, because the answer also
    /// decides whether the span is rendered at all: a site that filed here
    /// unasked would have paid for the rendering of an event nothing reads.
    ///
    /// **The span arrives rendered and as the spool's fields.**
    /// `rule:observability/a-query-is-a-trace-event`'s field set lives in
    /// `nvs_db::QuerySpan`, in a crate that depends on this one, so the trace
    /// vector reads text — a stand-in until `rule:testing/debug-probes`'s sink
    /// gives every kind its payload. The debug spool's `event` line needs the
    /// fields, and [`crate::spool::Query`] is the one type the writer and the
    /// ingester share, so `nvs_db::QuerySpan::spooled` fills it there.
    ///
    /// The rendering is `QuerySpan`'s `Display`, so
    /// `rule:observability/a-query-is-a-trace-event`'s "never parameters"
    /// is held where the span is built and
    /// `a_query_span_contains_no_parameter_value_anywhere` asserts it over that
    /// same rendering; nothing here can put a bound value back.
    ///
    /// `took` is the statement's own duration, which the span already measured,
    /// and the event's window is the `took` that ends here.
    ///
    /// `query` is the same span's fields as the debug spool records them
    /// (`nvs_db::QuerySpan::spooled`), and they go to the spool's `event` line
    /// beside the rendering. The trace vector keeps the rendering alone.
    pub fn record_query(&mut self, span: &str, query: crate::spool::Query, took: Duration) {
        let (start, end) = Self::ended_after(took);
        self.profile_window("query", start, end);
        self.file_event(
            TraceEvent {
                kind: TraceKind::Query,
                callee: span.to_owned(),
                status: None,
                start,
                end,
            },
            Some(query),
        );
    }

    /// Files `event` in the trace and writes it to the debug spool as an
    /// `event` line when `[debug] store` is in force for this request
    /// (`rule:testing/the-debug-stream-reaches-the-service-through-the-disk`).
    ///
    /// With `store` off the spool costs one read of the resolved configuration,
    /// and only on a run [`Ctx::records_spans`] already answered `true` for.
    fn file_event(&mut self, event: TraceEvent, query: Option<crate::spool::Query>) {
        self.spool_event(&event, query);
        self.trace.push(event);
    }

    /// Writes `event` to the debug spool, with a `query` event's own fields.
    fn spool_event(&self, event: &TraceEvent, query: Option<crate::spool::Query>) {
        let mut line = crate::spool::Event::of(event);
        line.query = query;
        crate::spool::record(self, crate::spool::Record::Event(line));
    }

    /// Records one outbound call as
    /// `rule:observability/trace-events-carry-a-kind`
    /// 's `http` event — `Core\Http\Client`'s whole effect under
    /// [`Ctx::records_spans`], called once the reply's head is in hand and
    /// **once**, however many attempts and hops it took to get there. The
    /// caller asks that question, for [`Ctx::record_query`]'s reason.
    ///
    /// The span arrives already rendered for [`Ctx::record_query`]'s reason,
    /// one crate further out: the field set the rule fixes — the method, the
    /// scheme, host and port, the path without its query, the status, the
    /// attempt and hop counts, the address connected to and where the time went
    /// — lives beside the transport that is already holding every one of those,
    /// in a crate that depends on this one.
    ///
    /// **[`TraceEvent::status`] stays `None` here**, and the reply's status is
    /// inside the span instead: that field is the checked-return status a
    /// *call site* is about to branch on, and an HTTP status written into it
    /// would be a second meaning for one field that every reader would then
    /// have to disambiguate by kind.
    ///
    /// `took` is the call's duration as the transport's span measured it, for
    /// [`Ctx::record_query`]'s reason.
    pub fn record_http(&mut self, span: &str, took: Duration) {
        let (start, end) = Self::ended_after(took);
        self.profile_window("http", start, end);
        self.file_event(
            TraceEvent {
                kind: TraceKind::Http,
                callee: span.to_owned(),
                status: None,
                start,
                end,
            },
            None,
        );
    }

    /// Opens `rule:observability/spawn-is-its-own-event`'s event where a child
    /// is started, and answers what closes it at the join — `None`, having
    /// recorded nothing and read no clock, for a run nothing records
    /// ([`Ctx::records_spans`]) and [`DebugFlags::PROFILE`] does not time.
    ///
    /// A spawn is not a call and does not flow through
    /// [`nvs_probe_call_enter`]'s pair, which is the rule's own reason for an
    /// instrumentation point of its own. The event is filed here rather than at
    /// the join so that it sits where the spawn happened among the request's
    /// other events, and so that a child that is never joined — a cancelled one
    /// — reads as a spawn with no join rather than as nothing at all.
    pub fn open_spawn(&mut self, form: SpawnForm) -> Option<OpenSpawn> {
        if !self.records_spans() && !self.debug.contains(DebugFlags::PROFILE) {
            return None;
        }
        let started = trace_clock();
        self.trace.push(TraceEvent {
            kind: TraceKind::Spawn,
            callee: format!("{} started={started:?} unjoined", form.spelling()),
            status: None,
            start: started,
            end: started,
        });
        Some(OpenSpawn {
            form,
            index: self.trace.len() - 1,
            started,
        })
    }

    /// Closes the event [`Ctx::open_spawn`] filed, at the point the child is
    /// joined and with the child's own wall time where it reported one.
    ///
    /// The split is what the rule asks for and why both numbers are carried:
    /// the parent-observed wall less the child's own is what the spawn itself
    /// cost — scheduling the child and copying its answer back — and one opaque
    /// total cannot be read as either.
    pub fn close_spawn(&mut self, open: OpenSpawn, child: Option<Duration>) {
        let joined = trace_clock();
        self.profile_window(open.form.spelling(), open.started, joined);
        let payload = render_spawn(open.form, open.started, joined, child);
        // Indexed rather than assumed: the trace is only ever appended to, so
        // the position holds, and a miss loses one event rather than panicking
        // inside a request that was only being observed.
        if let Some(event) = self.trace.get_mut(open.index) {
            event.callee = payload;
            event.end = joined;
        }
        // Written at the join, once the event is whole. A child that is never
        // joined writes no spool line.
        if let Some(event) = self.trace.get(open.index) {
            self.spool_event(event, None);
        }
    }

    /// Reads the clock a `rule:observability/gc-pause-is-its-own-event` event
    /// opens with, or answers `None` — having read no clock — for a run that
    /// records nothing ([`Ctx::records_spans`]) and does not time
    /// ([`DebugFlags::PROFILE`]).
    ///
    /// The pair with [`Ctx::close_gc_pause`] is [`Ctx::open_spawn`]'s, one
    /// question shorter: a collection has no join to be filled in at, so the
    /// event is filed whole at the end and there is nothing to reserve a slot
    /// for. Asked from inside [`Ctx::collect_if_asked`] **after** the flag that
    /// brings a request there, which is what the rule means by recording from
    /// the collector's routine and never from the poll: a back edge that finds
    /// no collection asked for reads no clock and tests no debug bit.
    pub(crate) fn open_gc_pause(&self) -> Option<Duration> {
        (self.records_spans() || self.debug.contains(DebugFlags::PROFILE)).then(trace_clock)
    }

    /// Files the `gc` event [`Ctx::open_gc_pause`] read the clock for, with the
    /// duration of the run and the number of objects it freed —
    /// `rule:observability/gc-pause-is-its-own-event`'s whole field set.
    ///
    /// A pause of zero objects is filed like any other. What the event buys is
    /// attribution, and a walk that found nothing still took the time it took.
    pub(crate) fn close_gc_pause(&mut self, started: Duration, freed: usize) {
        let ended = trace_clock();
        let took = ended.saturating_sub(started);
        self.profile_window("gc", started, ended);
        self.file_event(
            TraceEvent {
                kind: TraceKind::Gc,
                callee: format!("gc started={started:?} took={took:?} freed={freed}"),
                status: None,
                start: started,
                end: ended,
            },
            None,
        );
    }

    /// The span events filed so far, in the order they were filed — empty for
    /// a request that was neither recorded nor traced. No `call` is among
    /// them: the field's own doc comment says where those go.
    #[must_use]
    pub fn trace(&self) -> &[TraceEvent] {
        &self.trace
    }

    /// The events an exporter derives this request's spans from, **moved** out
    /// of this context — and nothing at all where nobody is recording.
    ///
    /// The question is `rule:observability/sampling-is-head-based`'s flag and
    /// not [`DebugFlags::TRACE`], because those two answer different things: the
    /// flag says whether this trace is being recorded, and the bit says whether
    /// the events were filed in the first place. A request that is not sampled
    /// leaves them here untouched, which is what makes deriving spans cost it a
    /// load and a branch; a sampled one gives them up, because the context is
    /// about to be torn down and moving what is already owned costs less than
    /// copying it. [`Ctx::trace`] answers empty afterwards, so this is called
    /// once, where the request ends.
    ///
    /// [`nvs_host::Completion::trace`](crate::host::Completion::trace) is where
    /// they go and owns the rest of the reasoning.
    #[must_use]
    pub fn take_sampled_trace(&mut self) -> Vec<TraceEvent> {
        if !self.trace_context().sampled() {
            return Vec::new();
        }
        std::mem::take(&mut self.trace)
    }

    /// Arms a fault-injection site for this request — see [`FaultSite`] for
    /// what each one does and why the set is closed.
    pub fn inject_fault(&mut self, site: FaultSite) {
        self.fault = Some(site);
    }

    /// Whether [`FaultSite::HelperPanic`] is armed *and* this is the helper
    /// call it names, disarming it if so. Called once per helper entry from
    /// [`crate::run_helper`]; a request with nothing armed pays one
    /// already-loaded `Option` test and nothing else.
    pub(crate) fn take_armed_helper_panic(&mut self) -> bool {
        if self.fault != Some(FaultSite::HelperPanic) {
            return false;
        }
        self.helper_calls += 1;
        if self.helper_calls < 2 {
            return false;
        }
        self.fault = None;
        true
    }
}

/// `rule:testing/debug-probes`'s statement-boundary probe — the slow path behind the debug-flags
/// check, reached only when the word compiled code loaded was non-zero.
///
/// Deliberately the same *shape* as [`nvs_safepoint`]: one cached load and one
/// predicted-not-taken branch at the site, everything else out of line. It
/// differs in returning nothing — coverage bookkeeping cannot fail, and a
/// debugger stops at its own stop sites, not at a probe
/// (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`) — so a compiled
/// probe site has no status to check and no error edge to emit.
///
/// Only [`DebugFlags::COVERAGE`] and [`DebugFlags::COUNT`] act here.
/// `BRANCH` is [`nvs_probe_edge`]'s, and `TRACE` and `PROFILE` are the
/// call-site pair below.
///
/// # Safety
///
/// `ctx` must be non-null, aligned, and valid for the duration of the call.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer; the contract cannot be \
              expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_probe_stmt(ctx: *mut Ctx, stmt: u32) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ctx` is valid for this call; the only \
                  thing that can panic here is the allocator, which aborts \
                  rather than unwinding into the JIT frame above"
    )]
    let ctx = unsafe { &mut *ctx };

    if ctx.debug.contains(DebugFlags::COVERAGE) {
        ctx.record_stmt_hit(stmt);
    }
    if ctx.debug.contains(DebugFlags::COUNT) {
        ctx.counted_stmts += 1;
    }
}

/// `rule:testing/debug-probes`'s conditional-edge probe — the slow path a
/// compiled two-way branch takes when the debug-flags word it loaded was
/// non-zero, called on the edge the branch took.
///
/// [`nvs_probe_stmt`]'s shape: nothing to return, so the site has no status
/// to check. Only [`DebugFlags::BRANCH`] acts here, counting into the edge
/// half of the run's [`StmtHits`]. `edge` is the program-wide number
/// `nvs_ir::Program::edge_spans` indexes.
///
/// # Safety
///
/// `ctx` must be non-null, aligned, and valid for the duration of the call.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer; the contract cannot be \
              expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_probe_edge(ctx: *mut Ctx, edge: u32) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ctx` is valid for this call, and \
                  nothing here allocates or panics"
    )]
    let ctx = unsafe { &*ctx };

    if ctx.debug.contains(DebugFlags::BRANCH) {
        ctx.record_edge_hit(edge);
    }
}

/// What [`Ctx::counted`] answers: `rule:testing/bench-counters`'s two
/// probe-site counts and its GC cycle count, read once at the end of a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counted {
    /// Statement boundaries crossed.
    pub statements: u64,
    /// Compiled call sites entered.
    pub calls: u64,
    /// In-flight collections run, whether or not each reclaimed anything.
    pub gc_cycles: u64,
}

/// `rule:testing/debug-probes`'s hit counters for one run: one per statement,
/// indexed by the program-wide number codegen passes to [`nvs_probe_stmt`],
/// and one per conditional edge, indexed by the number it passes to
/// [`nvs_probe_edge`] (`nvs_ir::Program::stmt_spans` and
/// `nvs_ir::Program::edge_spans` say which source span each number is).
///
/// **One table per isolate, and one more for the run.** A task ([`Ctx::child`])
/// is part of its request and counts into the request's table. An isolate
/// ([`Ctx::isolate`], [`Ctx::placed_isolate`]) is given a table of its own,
/// [`Self::fresh_like`] its parent's, so `Core\Debug::coverage` in a parent
/// never lists a line only its child ran (`rule:security/isolate-shares-nothing`).
/// The child's table leaves on its `Completion`, and the join adds it into the
/// context's run table ([`Ctx::run_hits`]), which the host installs and Novis
/// code cannot read. So `nvs test`'s report still counts what a test's isolates
/// ran: each test gets a table of its own as its run table, added to the
/// suite's with [`StmtHits::add`] when the test finishes. The counters are
/// atomic because a placed isolate runs on another core.
///
/// The edge counters ride in the same handle, so every place that hands a
/// child the statement table hands it the edge table too.
///
/// **What it spends:** one `u64` per statement in the program, and one per
/// edge when the run asks for branches, once per run that asks for coverage,
/// allocated by whoever installs it, and once more per isolate in flight under
/// coverage, freed at its join. A run that does not ask allocates nothing.
#[derive(Debug)]
pub struct StmtHits {
    stmts: Box<[std::sync::atomic::AtomicU64]>,
    edges: Box<[std::sync::atomic::AtomicU64]>,
    lines: Option<Arc<StmtLines>>,
}

/// Where each statement a [`StmtHits`] table counts is written: a file and a line per statement
/// number, which is what `Core\Debug::coverage` reads the table back through.
///
/// **What it spends:** one pair of `u32`s per statement and one string per file, once per run
/// whose host located its tables ([`StmtHits::located`]). The tables of one run share it, so
/// `nvs test`'s table per test adds a pointer each.
#[derive(Debug, Default)]
pub struct StmtLines {
    files: Vec<Box<str>>,
    sites: Vec<(u32, u32)>,
}

impl StmtLines {
    /// No statements yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Says where the next statement number is written. The numbers are given in order, from 0.
    pub fn push(&mut self, file: &str, line: u32) {
        let index = match self.files.iter().rposition(|known| &**known == file) {
            Some(index) => index,
            None => {
                self.files.push(file.into());
                self.files.len() - 1
            }
        };
        self.sites
            .push((u32::try_from(index).unwrap_or(u32::MAX), line));
    }

    /// Says the next statement number is in no file a coverage answer lists, such as a file
    /// that is not on disk.
    pub fn skip(&mut self) {
        self.sites.push((u32::MAX, 0));
    }
}

/// The program's query site table: the file, line and file text hash of each `Core\Db` call that
/// sends a statement, by the id the compiler passes the call
/// (`rule:observability/a-query-span-names-its-call-site`). A traced `query` event records the
/// row its id names ([`Ctx::query_site`]), so a reader of the spool needs no table of its own.
///
/// **What it spends:** one pair of `u32`s per call site and one name and hash per file, once per
/// compiled program whose host gave its contexts the table. A context holds a pointer to it.
#[derive(Debug, Default)]
pub struct QuerySites {
    files: Vec<(Box<str>, Box<str>)>,
    sites: Vec<(u32, u32)>,
}

impl QuerySites {
    /// No sites yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Says where the next site id is written, and the hash of that file's text. The ids are
    /// given in order, from 1.
    pub fn push(&mut self, file: &str, line: u32, hash: &str) {
        let index = match self.files.iter().rposition(|(known, _)| &**known == file) {
            Some(index) => index,
            None => {
                self.files.push((file.into(), hash.into()));
                self.files.len() - 1
            }
        };
        self.sites
            .push((u32::try_from(index).unwrap_or(u32::MAX), line));
    }

    /// The site `id` names, or `None` for the zero id and an id past the table.
    #[must_use]
    pub fn at(&self, id: u32) -> Option<crate::spool::QuerySite> {
        let &(file, line) = self.sites.get(usize::try_from(id.checked_sub(1)?).ok()?)?;
        let (name, hash) = self.files.get(usize::try_from(file).ok()?)?;
        Some(crate::spool::QuerySite {
            file: name.to_string(),
            line,
            hash: hash.to_string(),
        })
    }
}

/// The files `Core\Debug::startTrace` and `Core\Debug::startProfile` named
/// (`rule:testing/debug-surface`), each from its start until its stop. A context that started
/// neither has `None` for both. A child context does not share them.
///
/// The profile is a [`ProfileTrack`] of the file's [`Profiler`]. A child of a context whose host
/// installed a profiler ([`Ctx::set_run_profile`]) starts with a track of its own in it.
///
/// Both are boxed so a context with neither running carries one word for each.
#[derive(Debug, Default)]
pub(crate) struct DebugSinks {
    trace: Option<Box<TraceSink>>,
    profile: Option<Box<ProfileTrack>>,
}

impl DebugSinks {
    /// A child context's sinks: no trace, and a track of the host's profiler when there is one.
    pub(crate) fn for_child(run_profile: Option<&Arc<Profiler>>, parent: &Self) -> Self {
        Self {
            trace: None,
            profile: run_profile
                .map(|profiler| Box::new(profiler.child_track(parent.profile.as_deref()))),
        }
    }
}

/// A trace an isolate left running, closed at its end: what its result carries in place of the
/// events, which stay in the file.
#[derive(Debug)]
pub struct ClosedSink {
    /// The file the trace wrote.
    pub path: std::path::PathBuf,
    /// The first write that failed, or the final flush's failure.
    pub failed: Option<std::io::Error>,
}

/// An open trace file: `rule:testing/debug-surface`'s newline-delimited JSON, one line per call,
/// written when the call returns.
///
/// A line is `{"kind":"call","callee":"Math::double","status":"ok","start":1200,"end":1450}`:
/// the callee's label, how it returned (`ok`, `thrown`, `fatal` or `exited`), and its entry and
/// exit as nanoseconds on [`trace_clock`]. A call still running when the trace stops has not
/// returned, so it has no line. An exit whose entry came before the trace started is an instant
/// at its exit.
///
/// A write that fails stops the writing and is kept until [`Self::finish`] returns it, because a
/// trace observes a run and never stops one.
///
/// **What it spends:** one buffered file per running trace, and one reading per call in flight,
/// so it grows with the call depth and never with the call count.
#[derive(Debug)]
pub struct TraceSink {
    path: std::path::PathBuf,
    out: std::io::BufWriter<std::fs::File>,
    /// When each call in flight was entered, the innermost last.
    open: Vec<Duration>,
    failed: Option<std::io::Error>,
}

impl TraceSink {
    /// Creates `path`, or empties it when it exists.
    ///
    /// # Errors
    ///
    /// What the operating system returned for the create.
    pub fn create(path: &std::path::Path) -> std::io::Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            out: std::io::BufWriter::new(std::fs::File::create(path)?),
            open: Vec::new(),
            failed: None,
        })
    }

    /// The file this sink writes.
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Writes what is still buffered and closes the file.
    ///
    /// # Errors
    ///
    /// The first write that failed while the trace ran, or the final flush's own failure.
    pub fn finish(mut self) -> std::io::Result<()> {
        use std::io::Write as _;
        match self.failed.take() {
            Some(failed) => Err(failed),
            None => self.out.flush(),
        }
    }

    fn enter(&mut self) {
        self.open.push(trace_clock());
    }

    fn exit(&mut self, callee: &str, status: i32) {
        let end = trace_clock();
        let start = self.open.pop().unwrap_or(end);
        if self.failed.is_none()
            && let Err(failed) = self.write_call(callee, status, start, end)
        {
            self.failed = Some(failed);
        }
    }

    fn write_call(
        &mut self,
        callee: &str,
        status: i32,
        start: Duration,
        end: Duration,
    ) -> std::io::Result<()> {
        use std::io::Write as _;
        self.out.write_all(br#"{"kind":"call","callee":""#)?;
        write_json_text(&mut self.out, callee)?;
        let returned = match status {
            crate::OK => "ok",
            crate::THROWN => "thrown",
            crate::FATAL => "fatal",
            crate::EXITED => "exited",
            _ => "unknown",
        };
        writeln!(
            self.out,
            r#"","status":"{returned}","start":{},"end":{}}}"#,
            start.as_nanos(),
            end.as_nanos()
        )
    }
}

/// `text` as the inside of a JSON string: `"`, `\` and every control character escaped.
pub(crate) fn write_json_text(out: &mut impl std::io::Write, text: &str) -> std::io::Result<()> {
    let mut rest = text;
    while let Some(at) = rest.find(|c: char| c == '"' || c == '\\' || c.is_control()) {
        out.write_all(&rest.as_bytes()[..at])?;
        let c = rest[at..].chars().next().unwrap_or_default();
        match c {
            '"' => out.write_all(br#"\""#)?,
            '\\' => out.write_all(br"\\")?,
            '\n' => out.write_all(br"\n")?,
            _ => write!(out, "\\u{:04x}", u32::from(c))?,
        }
        rest = &rest[at + c.len_utf8()..];
    }
    out.write_all(rest.as_bytes())
}

impl Ctx {
    /// The file the running trace writes to, while one runs.
    #[must_use]
    pub fn trace_sink(&self) -> Option<&std::path::Path> {
        self.debug_sinks.trace.as_deref().map(TraceSink::path)
    }

    /// Gives this context the trace its `call` events stream to, or takes it away with `None`,
    /// and returns the one it had, which its taker closes with [`TraceSink::finish`].
    pub fn set_trace_sink(&mut self, sink: Option<TraceSink>) -> Option<TraceSink> {
        std::mem::replace(&mut self.debug_sinks.trace, sink.map(Box::new)).map(|old| *old)
    }

    /// The file the running profile writes to, while one runs.
    #[must_use]
    pub fn profile_sink(&self) -> Option<&std::path::Path> {
        self.debug_sinks
            .profile
            .as_deref()
            .map(|track| track.profiler().path())
    }

    /// Gives this context the track its calls are timed into, or takes it away with `None`, and
    /// returns the one it had. A track closes when it is dropped.
    pub fn set_profile_sink(&mut self, track: Option<ProfileTrack>) -> Option<ProfileTrack> {
        std::mem::replace(&mut self.debug_sinks.profile, track.map(Box::new)).map(|old| *old)
    }

    /// The host's profiler, which every child context gets a track of its own in.
    #[must_use]
    pub fn run_profile(&self) -> Option<&Arc<Profiler>> {
        self.run_profile.as_ref()
    }

    /// Installs (or with `None`, removes) the host's profiler. This context's own track is
    /// [`Self::set_profile_sink`]'s.
    pub fn set_run_profile(&mut self, profiler: Option<Arc<Profiler>>) {
        self.run_profile = profiler;
    }

    /// A child context's sinks, for the constructors in `isolate.rs`.
    pub(crate) fn child_sinks(&self) -> DebugSinks {
        DebugSinks::for_child(self.run_profile.as_ref(), &self.debug_sinks)
    }

    /// Times something that is not a call, from `start` to `end`, into the running profile.
    fn profile_window(&mut self, label: &str, start: Duration, end: Duration) {
        if self.debug.contains(DebugFlags::PROFILE)
            && let Some(track) = &mut self.debug_sinks.profile
        {
            track.window(label, start, end);
        }
    }

    /// Closes the trace this context left running, for an isolate's result. A sink is never
    /// shared, so a child's trace ends with the child and its parent's goes on.
    pub fn close_trace_sink(&mut self) -> Option<ClosedSink> {
        let sink = self.set_trace_sink(None)?;
        let path = sink.path().to_path_buf();
        Some(ClosedSink {
            path,
            failed: sink.finish().err(),
        })
    }

    /// The host's run table, which a joined isolate's table is added into. `None` unless the host
    /// installed one; every child context gets a clone of the handle.
    #[must_use]
    pub fn run_hits(&self) -> Option<&Arc<StmtHits>> {
        self.run_hits.as_ref()
    }

    /// Installs (or with `None`, removes) the run table.
    pub fn set_run_hits(&mut self, hits: Option<Arc<StmtHits>>) {
        self.run_hits = hits;
    }

    /// Adds a joined isolate's table into the run table. A table that is the run table itself is
    /// not added, so a host that gave an isolate the run's own table counts it once.
    pub fn join_coverage(&self, child: &StmtHits) {
        if let Some(run) = &self.run_hits
            && !std::ptr::eq(Arc::as_ptr(run), child)
        {
            run.add(&child.counts(), &child.edge_counts());
        }
    }

    /// A table of this context's shape for an isolate it starts, or `None` when it counts nothing.
    pub(crate) fn fresh_hits(&self) -> Option<Arc<StmtHits>> {
        self.stmt_hits
            .as_deref()
            .map(|hits| Arc::new(hits.fresh_like()))
    }
}

/// `len` counters, all zero.
fn zeroed(len: usize) -> Box<[std::sync::atomic::AtomicU64]> {
    (0..len)
        .map(|_| std::sync::atomic::AtomicU64::new(0))
        .collect()
}

/// Counts one hit at `index`. A number past the end is ignored: it can only
/// come from a unit the table was not sized for.
fn bump(counters: &[std::sync::atomic::AtomicU64], index: u32) {
    if let Some(counter) = counters.get(index as usize) {
        counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Every counter's value, in order.
fn read(counters: &[std::sync::atomic::AtomicU64]) -> Vec<u64> {
    counters
        .iter()
        .map(|counter| counter.load(std::sync::atomic::Ordering::Relaxed))
        .collect()
}

impl StmtHits {
    /// A table of `statements` counters, all zero, and no edge counters.
    #[must_use]
    pub fn new(statements: usize) -> Self {
        Self::with_edges(statements, 0)
    }

    /// A table of `statements` statement counters and `edges` edge counters,
    /// all zero.
    #[must_use]
    pub fn with_edges(statements: usize, edges: usize) -> Self {
        Self {
            stmts: zeroed(statements),
            edges: zeroed(edges),
            lines: None,
        }
    }

    /// A table of the same shape, all zero, located as this one is: what an isolate counts into.
    #[must_use]
    pub fn fresh_like(&self) -> Self {
        Self {
            stmts: zeroed(self.stmts.len()),
            edges: zeroed(self.edges.len()),
            lines: self.lines.clone(),
        }
    }

    /// The same table, with where each statement is written, so [`Self::line_counts`] can read it.
    #[must_use]
    pub fn located(mut self, lines: Arc<StmtLines>) -> Self {
        self.lines = Some(lines);
        self
    }

    /// Where each statement is written, when the host located this table.
    #[must_use]
    pub fn lines(&self) -> Option<&Arc<StmtLines>> {
        self.lines.as_ref()
    }

    /// Each line a statement ran on, by file, with its count — `Core\Debug::coverage`'s answer.
    ///
    /// A line's count is the largest count of a statement on it, the rule `nvs test`'s coverage
    /// reports use, so a loop header is not counted twice. A line no statement ran on is not
    /// listed. A table its host did not locate lists nothing.
    #[must_use]
    pub fn line_counts(
        &self,
    ) -> std::collections::BTreeMap<&str, std::collections::BTreeMap<u32, u64>> {
        let mut by_file = std::collections::BTreeMap::new();
        let Some(lines) = &self.lines else {
            return by_file;
        };
        for (counter, &(file, line)) in self.stmts.iter().zip(&lines.sites) {
            let count = counter.load(std::sync::atomic::Ordering::Relaxed);
            let Some(path) = lines.files.get(file as usize) else {
                continue;
            };
            if count == 0 {
                continue;
            }
            let at: &mut std::collections::BTreeMap<u32, u64> = by_file.entry(&**path).or_default();
            let kept = at.entry(line).or_insert(0);
            *kept = (*kept).max(count);
        }
        by_file
    }

    /// Counts one hit for statement `stmt`.
    pub fn record(&self, stmt: u32) {
        bump(&self.stmts, stmt);
    }

    /// Counts one hit for conditional edge `edge`.
    pub fn record_edge(&self, edge: u32) {
        bump(&self.edges, edge);
    }

    /// Every statement counter's value, in statement order.
    #[must_use]
    pub fn counts(&self) -> Vec<u64> {
        read(&self.stmts)
    }

    /// Every edge counter's value, in edge order.
    #[must_use]
    pub fn edge_counts(&self) -> Vec<u64> {
        read(&self.edges)
    }

    /// Adds another table's [`Self::counts`] and [`Self::edge_counts`] into
    /// this one, counter by counter. A count past the end of this table is
    /// ignored, as [`Self::record`] ignores a number past the end.
    ///
    /// `nvs test` gives each test a table of its own and adds it to the run's
    /// when the test finishes, so the lines one test reached stay that test's
    /// while other tests run at the same time.
    pub fn add(&self, counts: &[u64], edges: &[u64]) {
        for (counter, count) in self.stmts.iter().zip(counts) {
            counter.fetch_add(*count, std::sync::atomic::Ordering::Relaxed);
        }
        for (counter, count) in self.edges.iter().zip(edges) {
            counter.fetch_add(*count, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// Reads a callee label a compiled call site passed as a pointer/length pair
/// into the unit's own data section.
///
/// Lossy rather than fallible: the bytes come from an Novis identifier the
/// compiler wrote there, so they are already valid UTF-8, and a trace record
/// is not a place to fail a request from if that assumption were ever wrong.
///
/// # Safety
///
/// `name`/`len` must describe a live, readable byte range, or `len` must be
/// zero.
#[expect(
    unsafe_code,
    reason = "compiled code passes a pointer and a length; the contract cannot \
              be expressed in the signature"
)]
unsafe fn callee_label<'a>(name: *const u8, len: usize) -> Cow<'a, str> {
    if len == 0 {
        return Cow::Borrowed("");
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the range is readable; the zero-length \
                  case is split out because `from_raw_parts` rejects a null \
                  pointer even for an empty slice"
    )]
    let bytes = unsafe { std::slice::from_raw_parts(name, len) };
    String::from_utf8_lossy(bytes)
}

/// `rule:testing/debug-probes`'s call-site **entry** probe — the slow path behind the debug-flags
/// check compiled code emits before every call.
///
/// The callee is passed as a pointer and length into the compiled unit's own
/// data section rather than as an index into a side table: the name is
/// already a static constant of the unit, so there is nothing for a table to
/// add and nothing to keep in sync.
///
/// Like [`nvs_probe_stmt`], it returns nothing — a trace record cannot fail —
/// so the site has no status to check.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the call, and `name`/`len`
/// must describe a readable byte range.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer and a static byte \
              range; neither contract can be expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_probe_call_enter(ctx: *mut Ctx, name: *const u8, len: usize) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both are valid for this call; the only \
                  thing that can panic here is the allocator, which aborts \
                  rather than unwinding into the JIT frame above"
    )]
    let (ctx, label) = unsafe { (&mut *ctx, callee_label(name, len)) };
    if ctx.debug.contains(DebugFlags::TRACE) {
        ctx.record_trace(&label, None);
    }
    if ctx.debug.contains(DebugFlags::PROFILE) {
        ctx.profile_enter(&label);
    }
    if ctx.debug.contains(DebugFlags::COUNT) {
        ctx.counted_calls += 1;
    }
}

/// `rule:testing/debug-probes`'s call-site **exit** probe, carrying the checked-return `status` the
/// call site is about to branch on — which is why a trace shows a thrown or
/// `FATAL` exit as it happened rather than as a reconstruction.
///
/// See [`nvs_probe_call_enter`] for the rest, including why the flags word is
/// re-read here rather than the entry probe's answer being reused: a request
/// may turn tracing on or off *during* the call, and an exit whose flag state
/// differs from its entry's is the honest record of that.
///
/// # Safety
///
/// The same contract as [`nvs_probe_call_enter`].
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer and a static byte \
              range; neither contract can be expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_probe_call_exit(
    ctx: *mut Ctx,
    name: *const u8,
    len: usize,
    status: i32,
) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both are valid for this call; the only \
                  thing that can panic here is the allocator, which aborts \
                  rather than unwinding into the JIT frame above"
    )]
    let (ctx, label) = unsafe { (&mut *ctx, callee_label(name, len)) };
    if ctx.debug.contains(DebugFlags::TRACE) {
        ctx.record_trace(&label, Some(status));
    }
    if ctx.debug.contains(DebugFlags::PROFILE) {
        ctx.profile_exit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `query` event's spool fields, for a test that files one.
    fn a_query() -> crate::spool::Query {
        crate::spool::Query {
            driver: "postgres".to_owned(),
            sql: "select 1".to_owned(),
            rows: Some(1),
            site: None,
        }
    }

    /// The event carries the rule's own spelling of the form, both timestamps
    /// and the split, and a request with both bits off does not pay for any of
    /// it.
    #[test]
    fn a_spawn_event_carries_its_form_and_both_timestamps_and_splits_the_overhead() {
        // The rendering against numbers no clock can vary: the parent observed
        // eight milliseconds, the child reported five of them, and the three
        // that are left are what the spawn itself cost.
        assert_eq!(
            render_spawn(
                SpawnForm::Worker,
                Duration::from_millis(1),
                Duration::from_millis(9),
                Some(Duration::from_millis(5)),
            ),
            "spawn worker started=1ms joined=9ms wall=8ms child=5ms overhead=3ms"
        );
        // A child reporting more than its parent observed leaves no overhead
        // rather than a negative one.
        assert_eq!(
            render_spawn(
                SpawnForm::Task,
                Duration::ZERO,
                Duration::from_millis(2),
                Some(Duration::from_millis(3)),
            ),
            "spawn started=0ns joined=2ms wall=2ms child=3ms overhead=0ns"
        );
        // A child that reported nothing carries the wall alone, rather than a
        // split computed against a zero it never claimed.
        assert_eq!(
            render_spawn(
                SpawnForm::Script,
                Duration::ZERO,
                Duration::from_millis(2),
                None,
            ),
            "spawn script started=0ns joined=2ms wall=2ms"
        );

        let mut ctx = Ctx::buffered();
        assert!(
            ctx.open_spawn(SpawnForm::Script).is_none(),
            "both bits are off, so there is nothing to record and no clock to read"
        );
        assert!(ctx.trace().is_empty());

        // `PROFILE` alone arms the site: the two bits are independent and this
        // event is the profile's as much as the trace's.
        ctx.set_debug_flags(DebugFlags::PROFILE);
        let open = ctx
            .open_spawn(SpawnForm::Script)
            .expect("a bit this site reads is on");
        assert_eq!(
            ctx.trace().len(),
            1,
            "the event is filed where the child starts"
        );
        assert_eq!(ctx.trace()[0].kind, TraceKind::Spawn);
        assert!(
            ctx.trace()[0].callee.ends_with("unjoined"),
            "a spawn that has not been joined must not read as one that has"
        );

        ctx.close_spawn(open, Some(Duration::ZERO));
        assert_eq!(
            ctx.trace().len(),
            1,
            "the join closes the event rather than filing a second"
        );
        let closed = &ctx.trace()[0].callee;
        assert!(closed.starts_with("spawn script started="), "{closed}");
        assert!(closed.contains(" joined="), "{closed}");
        assert!(closed.contains(" child=0ns overhead="), "{closed}");
        assert_eq!(
            ctx.trace()[0].status,
            None,
            "the status field is a call site's, and a spawn is not a call"
        );
    }

    #[test]
    fn a_collection_run_files_a_gc_event_and_an_unrecorded_run_reads_no_clock() {
        // `rule:observability/gc-pause-is-its-own-event`, at the one site that
        // can hold it: the event is the collector's run, and the poll that
        // found nothing to collect never gets this far.
        let mut ctx = Ctx::buffered();
        ctx.request_safepoint(crate::SafepointFlags::COLLECT);
        assert!(
            !ctx.collect_if_asked(),
            "there is nothing on this context's list to reclaim"
        );
        assert!(
            ctx.trace().is_empty(),
            "both bits are off, so the run reads no clock and files nothing"
        );

        // `PROFILE` alone arms it, for `open_spawn`'s reason: a pause folded
        // into a function's self time is exactly what the profile must not do.
        ctx.set_debug_flags(DebugFlags::PROFILE);
        ctx.request_safepoint(crate::SafepointFlags::COLLECT);
        assert!(!ctx.collect_if_asked());
        assert_eq!(ctx.trace().len(), 1, "the run files its own event");
        assert_eq!(ctx.trace()[0].kind, TraceKind::Gc);
        let filed = &ctx.trace()[0].callee;
        assert!(filed.starts_with("gc started="), "{filed}");
        assert!(filed.contains(" took="), "{filed}");
        assert!(
            filed.ends_with(" freed=0"),
            "a walk that freed nothing says so rather than reading as a call: {filed}"
        );
    }

    #[test]
    fn a_probe_with_coverage_off_records_nothing() {
        let mut ctx = Ctx::buffered();
        let hits = Arc::new(StmtHits::new(4));
        ctx.set_stmt_hits(Some(Arc::clone(&hits)));
        for stmt in 0..4 {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            unsafe {
                nvs_probe_stmt(&raw mut ctx, stmt);
            }
        }
        assert_eq!(hits.counts(), [0, 0, 0, 0]);
    }

    #[test]
    fn a_probe_counts_a_hit_per_statement_once_coverage_is_on() {
        let mut ctx = Ctx::buffered();
        let hits = Arc::new(StmtHits::new(3));
        ctx.set_stmt_hits(Some(Arc::clone(&hits)));
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        for stmt in [2_u32, 0, 2, 7] {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            unsafe {
                nvs_probe_stmt(&raw mut ctx, stmt);
            }
        }
        // Statement 1 never ran and reads back as zero; 2 ran twice. 7 is past
        // the table's end and is ignored.
        assert_eq!(hits.counts(), [1, 0, 2]);
    }

    #[test]
    fn an_edge_probe_counts_into_the_edge_table_under_branch_alone() {
        let mut ctx = Ctx::buffered();
        let hits = Arc::new(StmtHits::with_edges(2, 3));
        ctx.set_stmt_hits(Some(Arc::clone(&hits)));
        let probe = |ctx: &mut Ctx, edges: &[u32]| {
            for edge in edges {
                #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
                unsafe {
                    nvs_probe_edge(&raw mut *ctx, *edge);
                }
            }
        };
        // Line coverage alone does not count edges.
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        probe(&mut ctx, &[0, 1]);
        assert_eq!(hits.edge_counts(), [0, 0, 0]);

        // Under `BRANCH`, edge 9 is past the table's end and is ignored, and no
        // statement is counted.
        ctx.set_debug_flags(DebugFlags::BRANCH);
        probe(&mut ctx, &[2, 0, 2, 9]);
        assert_eq!(hits.edge_counts(), [1, 0, 2]);
        assert_eq!(hits.counts(), [0, 0]);
    }

    /// `SITES` sends every site to its helper, which records nothing for it, and a host call
    /// that sets the flags keeps it on.
    #[test]
    fn every_site_probed_records_nothing_and_stays_on() {
        let mut ctx = Ctx::buffered();
        let hits = Arc::new(StmtHits::with_edges(2, 2));
        ctx.set_stmt_hits(Some(Arc::clone(&hits)));
        ctx.set_debug_flags(DebugFlags::SITES);
        let label = b"Shop::total";
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_stmt(&raw mut ctx, 0);
            nvs_probe_edge(&raw mut ctx, 1);
            nvs_probe_call_enter(&raw mut ctx, label.as_ptr(), label.len());
            nvs_probe_call_exit(&raw mut ctx, label.as_ptr(), label.len(), 0);
        }
        assert_eq!(hits.counts(), [0, 0]);
        assert_eq!(hits.edge_counts(), [0, 0]);
        assert_eq!(ctx.counted(), Counted::default());
        assert!(!ctx.records_spans());

        ctx.set_debug_flags(DebugFlags::COUNT);
        assert_eq!(ctx.debug_flags(), DebugFlags::COUNT | DebugFlags::SITES);
        assert!(ctx.start_probes(DebugFlags::COVERAGE).is_err());
    }

    #[test]
    fn coverage_with_no_table_counts_nothing_and_does_not_fail() {
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            nvs_probe_stmt(&raw mut ctx, 0);
        }
        assert!(ctx.stmt_hits().is_none());
    }

    /// An isolate counts into a table of its own, and a task into its request's. Joining the
    /// isolate adds its table into the run table and leaves the parent's own unchanged.
    #[test]
    fn an_isolate_counts_into_its_own_table_and_a_task_into_its_requests() {
        let mut parent = Ctx::buffered();
        let own = Arc::new(StmtHits::new(2));
        let run = Arc::new(StmtHits::new(2));
        parent.set_stmt_hits(Some(Arc::clone(&own)));
        parent.set_run_hits(Some(Arc::clone(&run)));
        parent.set_debug_flags(DebugFlags::COVERAGE);
        let mut child = parent.isolate(OutputSink::Sink);
        let mut placed = parent.placed_isolate().build(OutputSink::Sink);
        #[expect(unsafe_code, reason = "the task is dropped before its parent")]
        let mut task = unsafe { parent.child() };
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            nvs_probe_stmt(&raw mut child, 1);
            nvs_probe_stmt(&raw mut placed, 1);
            nvs_probe_stmt(&raw mut task, 0);
        }
        assert_eq!(own.counts(), [1, 0]);
        for isolate in [&child, &placed] {
            let table = isolate
                .stmt_hits()
                .expect("an isolate of a counted run has a table");
            assert!(!Arc::ptr_eq(table, &own));
            assert_eq!(table.counts(), [0, 1]);
            assert!(Arc::ptr_eq(
                isolate.run_hits().expect("the run table"),
                &run
            ));
            parent.join_coverage(table);
        }
        drop((task, child, placed));
        assert_eq!(own.counts(), [1, 0]);
        assert_eq!(run.counts(), [0, 2]);
    }

    /// Every child form starts from its parent's flag word at the spawn, and that word is its
    /// ceiling: it may stop a probe and start it again, and never start one its parent had off —
    /// `rule:testing/debug-surface`'s *narrow and never widen*, even where the parent's mode was
    /// wider than what it had on.
    #[test]
    fn every_child_form_inherits_the_flag_word_and_may_only_narrow_it() {
        let mut parent = Ctx::buffered();
        parent.set_debug_flags(DebugFlags::COVERAGE | DebugFlags::TRACE);
        parent.stop_probes(DebugFlags::TRACE);
        #[expect(unsafe_code, reason = "the child is dropped before its parent")]
        let task = unsafe { parent.child() };
        let children = [
            task,
            parent.isolate(OutputSink::Sink),
            parent.placed_isolate().build(OutputSink::Sink),
        ];
        for mut child in children {
            assert_eq!(child.debug_flags(), DebugFlags::COVERAGE);
            assert_eq!(child.debug_ceiling(), DebugFlags::COVERAGE);
            assert_eq!(
                child.start_probes(DebugFlags::TRACE),
                Err(DebugFlags::TRACE)
            );
            child.stop_probes(DebugFlags::COVERAGE);
            assert!(child.debug_flags().is_empty());
            assert_eq!(child.start_probes(DebugFlags::COVERAGE), Ok(()));
            assert_eq!(child.debug_flags(), DebugFlags::COVERAGE);
        }
        assert_eq!(parent.debug_flags(), DebugFlags::COVERAGE);
    }

    #[test]
    fn nothing_is_armed_by_default_and_the_site_fires_exactly_once() {
        let mut ctx = Ctx::buffered();
        for _ in 0..4 {
            assert!(!ctx.take_armed_helper_panic());
        }

        ctx.inject_fault(FaultSite::HelperPanic);
        // The first call is let through; the second is the site. Nothing
        // after it fires again — one injected fault, not a poisoned request.
        assert!(!ctx.take_armed_helper_panic());
        assert!(ctx.take_armed_helper_panic());
        assert!(!ctx.take_armed_helper_panic());
        assert!(!ctx.take_armed_helper_panic());
    }

    /// A context under [`DebugFlags::TRACE`] streaming its calls to a new file, and the scratch
    /// directory holding it, which the test keeps until it ends.
    fn tracing(name: &str) -> (Ctx, nvs_repo::Scratch) {
        let dir = nvs_repo::scratch(name);
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::TRACE);
        let sink =
            TraceSink::create(&dir.join("calls.jsonl")).expect("the scratch directory exists");
        assert!(ctx.set_trace_sink(Some(sink)).is_none());
        (ctx, dir)
    }

    /// Every line the running trace wrote, read back once it is closed.
    fn written(ctx: &mut Ctx) -> Vec<String> {
        let sink = ctx.set_trace_sink(None).expect("a trace is running");
        let path = sink.path().to_path_buf();
        sink.finish().expect("every line was written");
        std::fs::read_to_string(path)
            .expect("the trace file exists")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// A line's callee and status, and its entry and exit readings as nanoseconds.
    fn call_of(line: &str) -> (&str, &str, u128, u128) {
        let (head, times) = line
            .split_once(r#"","status":""#)
            .expect("a call line has a status");
        let callee = head
            .strip_prefix(r#"{"kind":"call","callee":""#)
            .expect("a call line starts with its kind and callee");
        let (status, times) = times
            .split_once(r#"","start":"#)
            .expect("a status is followed by the start");
        let (start, end) = times
            .strip_suffix('}')
            .and_then(|times| times.split_once(r#","end":"#))
            .expect("the line ends with its exit");
        (
            callee,
            status,
            start.parse().expect("the start is a number"),
            end.parse().expect("the end is a number"),
        )
    }

    /// One call through both compiled-code probes, returning `status`.
    fn probe_call(ctx: &mut Ctx, name: &[u8], status: i32) {
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut *ctx, name.as_ptr(), name.len());
            nvs_probe_call_exit(&raw mut *ctx, name.as_ptr(), name.len(), status);
        }
    }

    #[test]
    fn a_call_probe_with_tracing_off_records_nothing() {
        let (mut ctx, _dir) = tracing("trace-off");
        ctx.set_debug_flags(DebugFlags::empty());
        probe_call(&mut ctx, b"Math::double", crate::OK);
        // Coverage on, tracing still off: the two flags are independent, and
        // the call probe reads its own bit rather than "any bit set".
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        probe_call(&mut ctx, b"Math::double", crate::OK);
        assert!(written(&mut ctx).is_empty());
        assert!(ctx.trace().is_empty());
    }

    #[test]
    fn a_call_probe_records_the_callee_and_the_status_it_is_handed() {
        // The exit probe carries the checked-return status the call site is
        // about to branch on, so a thrown or `FATAL` exit is recorded as it
        // happened rather than reconstructed — `rule:testing/debug-probes`.
        let (mut ctx, _dir) = tracing("trace-status");
        probe_call(&mut ctx, b"Boom::inner", crate::THROWN);
        probe_call(&mut ctx, br"Core\Arr::map", crate::OK);
        let lines = written(&mut ctx);
        let calls: Vec<_> = lines.iter().map(|line| call_of(line)).collect();
        let [(inner, thrown, entered, left), (map, ok, _, _)] = calls[..] else {
            panic!("one line per call: {lines:?}");
        };
        assert_eq!((inner, thrown), ("Boom::inner", "thrown"));
        assert_eq!(
            (map, ok),
            (r"Core\\Arr::map", "ok"),
            "a backslash is escaped"
        );
        assert!(entered <= left, "the exit is not before the entry");
        assert!(ctx.trace().is_empty(), "a call is never filed in memory");
    }

    /// `rule:testing/debug-surface`: the call half of a trace streams, so a loop of many calls
    /// holds nothing in the context and writes one line per call. A nested call's line comes
    /// first, because a line is written when its call returns, and its window is inside its
    /// caller's.
    #[test]
    fn a_traced_loop_of_many_calls_holds_no_call_and_writes_one_line_per_call() {
        // Under Miri, 100 calls still show that nothing is filed in memory.
        let calls = if cfg!(miri) { 100 } else { 10_000 };
        let (mut ctx, _dir) = tracing("trace-loop");
        let outer = b"Orders::total";
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, outer.as_ptr(), outer.len());
        }
        for _ in 0..calls {
            probe_call(&mut ctx, b"Orders::line", crate::OK);
        }
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_exit(&raw mut ctx, outer.as_ptr(), outer.len(), crate::OK);
        }
        assert!(ctx.trace().is_empty(), "a call was filed in memory");

        let lines = written(&mut ctx);
        assert_eq!(lines.len(), calls + 1);
        let (first, _, first_in, first_out) = call_of(&lines[0]);
        let (last, _, last_in, last_out) = call_of(&lines[calls]);
        assert_eq!((first, last), ("Orders::line", "Orders::total"));
        assert!(last_in <= first_in && first_out <= last_out);
    }

    /// A call still running when the trace stops has no line, and a stopped trace writes
    /// nothing more.
    #[test]
    fn a_trace_writes_only_the_calls_that_returned_while_it_ran() {
        let (mut ctx, _dir) = tracing("trace-open");
        let name = b"Main::run";
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
        }
        probe_call(&mut ctx, b"Main::step", crate::OK);
        assert_eq!(written(&mut ctx).len(), 1);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_exit(&raw mut ctx, name.as_ptr(), name.len(), crate::OK);
        }
        assert!(ctx.trace().is_empty());
    }

    /// The kind, the callee and the status of each event: what these cases
    /// assert, with the readings of a clock left to the cases about time.
    fn shape(events: &[TraceEvent]) -> Vec<(TraceKind, &str, Option<i32>)> {
        events
            .iter()
            .map(|event| (event.kind, event.callee.as_str(), event.status))
            .collect()
    }

    /// `rule:observability/trace-events-carry-a-kind`'s window, on the one
    /// clock: a routine that timed itself files the `took` that ends where it
    /// files, so its window is that long and sits after an event filed before.
    #[test]
    fn a_timed_event_ends_where_it_is_filed_and_began_its_duration_before() {
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::TRACE);
        ctx.record_http("http method=GET", Duration::ZERO);
        let before = trace_clock();
        ctx.record_query("sql=select 1", a_query(), Duration::from_nanos(1));
        let after = trace_clock();
        let [http, query] = ctx.trace() else {
            panic!("two events");
        };
        assert!(http.end <= before && before <= query.end && query.end <= after);
        assert_eq!(query.end - query.start, Duration::from_nanos(1));
    }

    /// `rule:observability/trace-events-carry-a-kind`'s kind, over the kinds
    /// anything in the tree records: `Core\Db`'s statement routine files a
    /// `query` and `Core\Http\Client`'s transport files an `http`, and one
    /// vector keeps them apart. A `call` beside them goes to the trace's file
    /// and never into that vector, so the whole vector is asserted.
    #[test]
    fn each_recorded_kind_is_filed_under_its_own_tag_and_a_call_streams() {
        let (mut ctx, _dir) = tracing("trace-kinds");
        ctx.record_trace("People::all", None);
        ctx.record_trace("People::all", Some(crate::OK));
        // What `nvs_db::QuerySpan`'s `Display` hands over, which is the whole
        // of what a `query` event carries — no bound value among it, per
        // `rule:observability/a-query-is-a-trace-event`.
        ctx.record_query(
            "driver=postgres connection=main rows=2 sql=select 1",
            a_query(),
            Duration::ZERO,
        );
        // And the transport's own span, which carries no query string, header
        // value or body for the same rule's reason.
        ctx.record_http(
            "http method=GET scheme=https host=api.example.com port=443 path=/things",
            Duration::ZERO,
        );
        assert_eq!(
            shape(ctx.trace()),
            [
                (
                    TraceKind::Query,
                    "driver=postgres connection=main rows=2 sql=select 1",
                    None
                ),
                (
                    TraceKind::Http,
                    "http method=GET scheme=https host=api.example.com port=443 path=/things",
                    None
                ),
            ]
        );
        let lines = written(&mut ctx);
        assert_eq!(
            lines.iter().map(|line| call_of(line).0).collect::<Vec<_>>(),
            ["People::all"]
        );
    }

    #[test]
    fn a_call_probe_accepts_an_empty_label_without_reading_the_pointer() {
        let (mut ctx, _dir) = tracing("trace-empty-label");
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, std::ptr::null(), 0);
            nvs_probe_call_exit(&raw mut ctx, std::ptr::null(), 0, crate::OK);
        }
        let lines = written(&mut ctx);
        assert_eq!(
            lines.iter().map(|line| call_of(line).0).collect::<Vec<_>>(),
            [""]
        );
    }

    #[test]
    fn coverage_can_be_turned_on_and_off_mid_request() {
        // `rule:testing/debug-probes`'s whole argument for a runtime-checked flag over a second
        // compiled tier: a harness brackets one test inside a running request.
        let mut ctx = Ctx::buffered();
        let hits = Arc::new(StmtHits::new(3));
        ctx.set_stmt_hits(Some(Arc::clone(&hits)));
        let probe = |ctx: &mut Ctx, stmt: u32| {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            unsafe {
                nvs_probe_stmt(&raw mut *ctx, stmt);
            }
        };

        probe(&mut ctx, 0);
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        probe(&mut ctx, 1);
        ctx.set_debug_flags(DebugFlags::empty());
        probe(&mut ctx, 2);

        assert_eq!(hits.counts(), [0, 1, 0]);
    }

    /// `rule:testing/bench-counters`'s counting mode is its own bit: it counts
    /// while set, records nothing per site or per call, and leaves the
    /// coverage table and the trace exactly as a request with it off would.
    #[test]
    fn count_totals_statements_and_calls_and_files_nothing() {
        let mut ctx = Ctx::buffered();
        let hits = Arc::new(StmtHits::new(2));
        ctx.set_stmt_hits(Some(Arc::clone(&hits)));
        let name = b"Math::double";
        let probes = |ctx: &mut Ctx| {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
            unsafe {
                nvs_probe_stmt(&raw mut *ctx, 0);
                nvs_probe_stmt(&raw mut *ctx, 1);
                nvs_probe_call_enter(&raw mut *ctx, name.as_ptr(), name.len());
                nvs_probe_call_exit(&raw mut *ctx, name.as_ptr(), name.len(), crate::OK);
            }
        };

        probes(&mut ctx);
        assert_eq!(
            ctx.counted(),
            Counted::default(),
            "nothing is counted with the bit off"
        );

        ctx.set_debug_flags(DebugFlags::COUNT);
        probes(&mut ctx);
        probes(&mut ctx);
        assert_eq!(
            ctx.counted(),
            Counted {
                statements: 4,
                calls: 2,
                gc_cycles: 0,
            }
        );
        assert_eq!(hits.counts(), [0, 0], "counting is not coverage");
        assert!(ctx.trace().is_empty(), "counting is not tracing");
    }

    /// `rule:observability/four-kinds-become-a-span`'s three derived kinds are
    /// filed because the trace is recorded, with no debug flag anywhere — and
    /// the fourth, a `call`, still is not, because the probes read their own bit
    /// and a span per compiled call site is the cost
    /// `rule:observability/four-kinds-become-a-span` refuses.
    #[test]
    fn a_recorded_run_files_a_spans_events_and_still_no_call_event() {
        let mut ctx = Ctx::buffered();
        // The eager root every context draws is recorded by nothing, which is
        // what a request the head draw sampled out runs as.
        assert!(!ctx.records_spans());
        assert!(
            ctx.open_spawn(SpawnForm::Task).is_none(),
            "a run nothing records read a clock"
        );

        ctx.set_trace_context(crate::TraceContext::rooted(1.0));
        assert!(ctx.records_spans());
        let open = ctx
            .open_spawn(SpawnForm::Task)
            .expect("a recorded run files its spawn");
        ctx.close_spawn(open, None);
        ctx.record_query(
            "driver=postgres connection=main rows=1 sql=select 1",
            a_query(),
            Duration::ZERO,
        );
        ctx.record_http(
            "http method=GET scheme=https host=api.example.com port=443 path=/things",
            Duration::ZERO,
        );

        let name = b"Math::double";
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
            nvs_probe_call_exit(&raw mut ctx, name.as_ptr(), name.len(), crate::OK);
        }

        let kinds: Vec<_> = ctx.trace().iter().map(|event| event.kind).collect();
        assert_eq!(
            kinds,
            [TraceKind::Spawn, TraceKind::Query, TraceKind::Http],
            "a recorded run files the three kinds a span is derived from, and nothing else"
        );
    }

    /// The bound a recorded run and a traced one are both held to: a program
    /// querying in a loop stops filing, while the calls a trace streams are
    /// never truncated by it.
    #[test]
    fn filing_stops_at_the_ceiling_and_a_traced_runs_calls_still_stream() {
        let (mut ctx, _dir) = tracing("trace-ceiling");
        ctx.set_debug_flags(DebugFlags::empty());
        ctx.set_trace_context(crate::TraceContext::rooted(1.0));
        for _ in 0..SPAN_EVENT_CEILING {
            assert!(ctx.records_spans(), "the ceiling closed early");
            ctx.record_query(
                "driver=postgres connection=main rows=1 sql=select 1",
                a_query(),
                Duration::ZERO,
            );
        }

        assert!(
            !ctx.records_spans(),
            "a recorded run kept filing past `SPAN_EVENT_CEILING`"
        );
        assert!(
            ctx.open_spawn(SpawnForm::Task).is_none(),
            "a run past the ceiling still read a clock"
        );

        ctx.set_debug_flags(DebugFlags::TRACE);
        assert!(
            !ctx.records_spans(),
            "a traced run kept filing past `SPAN_EVENT_CEILING`"
        );
        probe_call(&mut ctx, b"Math::double", crate::OK);
        assert_eq!(written(&mut ctx).len(), 1, "the ceiling truncated a call");
        assert_eq!(ctx.trace().len(), SPAN_EVENT_CEILING);
    }
}
