//! `rule:testing/debug-probes`'s
//! probes, and what a request records for them.
//!
//! The probe sites are emitted unconditionally and cost a load and a
//! predicted-not-taken branch while [`DebugFlags`] is empty, which is the whole
//! of why coverage and tracing can be turned on *mid-request*. This file holds
//! both ends: [`nvs_probe_stmt`], [`nvs_probe_call_enter`] and
//! [`nvs_probe_call_exit`], and the [`Ctx`] methods they call into.
//!
//! [`FaultSite`] rides along because it is the same shape — a site compiled in,
//! armed by a test, and fired at most once.

use super::*;

use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// The moment this process first timed a spawn, so that the events two requests
/// record sit on one timeline.
///
/// An [`Instant`] has no absolute rendering and a per-request epoch would give
/// every request a timeline of its own, which is exactly the join
/// `rule:observability/spawn-is-its-own-event`'s export-time nesting has to
/// make. **What it spends:** one `Instant` per process, initialised by the
/// first spawn that is timed at all, so a run with both bits off never reads a
/// clock for it.
static SPAWN_EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

/// A failure a run can be *asked* to produce, for a mode that by definition
/// has no user-facing trigger.
///
/// The set is closed on purpose, and reachable only through `nvs run
/// --fault-inject=<site>`: it must never be reachable from a served request
/// (`nvs serve`, M7), and nothing in Novis source can arm one.
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
/// The tag is the whole of the distinction here, and deliberately so: § 1 keeps
/// a `call` event's shape exactly as `rule:testing/debug-probes` defines it, and the other
/// kinds carry facts of their own that this stand-in vector has nowhere to put.
/// A `query`'s field set is fixed by `rule:observability/a-query-is-a-trace-event`
/// and lives in `nvs_db::QuerySpan`, and an `http`'s by the same rule as this
/// one in `Core\Http\Client`'s own span — each in the crate that is already
/// holding the facts; a per-kind payload is what § 4's export needs and what lands with
/// `rule:testing/debug-probes`'s sink, alongside the `PROFILE` timing the `trace` field's own doc
/// comment defers for the same reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceKind {
    /// A call site's entry or exit — `rule:testing/debug-probes`'s probe pair, and with
    /// [`TraceKind::Query`] one of the kinds anything in the tree records.
    Call,
    /// A cycle-collector pause — `rule:observability/gc-pause-is-its-own-event`. The collector's run routine
    /// does not record one yet.
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
/// The remaining two fields are the `call` kind's shape, and a `query` and an
/// `http` reuse the first of them rather than adding a field per kind —
/// [`Ctx::record_query`] owns that reasoning, and [`TraceKind`]'s doc comment
/// owns what a per-kind payload waits on.
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
    /// When the child was started, on [`SPAWN_EPOCH`]'s timeline.
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
/// recorded run files before it stops filing them.
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
/// A debugging run is not held to it ([`Ctx::records_spans`]): `DebugFlags::TRACE`
/// files a `call` event per compiled call site, so a ceiling would truncate the
/// trace a debugger asked for at the first few statements.
pub const SPAN_EVENT_CEILING: usize = 512;

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
    /// attached**: [`DebugFlags::TRACE`] is on, which additionally files a
    /// `call` event per compiled call site — the cost
    /// `rule:observability/a-call-never-becomes-a-span` refuses on a served
    /// request outright, and the reason the two cannot be one bit. A sampled
    /// request never pays it, because the call probes read that bit and this
    /// answer is not it.
    ///
    /// The ceiling is asked here rather than beside each site, so that a filer
    /// cannot skip it: past [`SPAN_EVENT_CEILING`] filed events a sampled run
    /// answers `false` and a debugging one still answers `true`.
    #[must_use]
    pub fn records_spans(&self) -> bool {
        if self.debug.contains(DebugFlags::TRACE) {
            return true;
        }
        // The vector holds nothing else when that bit is off — the call probes
        // read it themselves — so its length is the filed-event count this
        // ceiling is about, and no second counter has to be kept true.
        self.trace_context.sampled() && self.trace.len() < SPAN_EVENT_CEILING
    }

    /// Counts one hit for the statement `stmt` names — [`nvs_probe_stmt`]'s
    /// whole effect under [`DebugFlags::COVERAGE`].
    pub fn record_stmt_hit(&mut self, stmt: u32) {
        let index = stmt as usize;
        if self.stmt_hits.len() <= index {
            self.stmt_hits.resize(index + 1, 0);
        }
        self.stmt_hits[index] += 1;
    }

    /// The per-statement hit counters gathered so far, indexed by
    /// `nvs_ir::StmtId` — empty for a request that ran with
    /// [`DebugFlags::COVERAGE`] off throughout. See the field's own doc
    /// comment for why this is a stand-in for `rule:testing/debug-probes`'s path → line → count
    /// shape rather than that shape itself.
    #[must_use]
    pub fn stmt_hits(&self) -> &[u64] {
        &self.stmt_hits
    }

    /// Records one call-site trace event — [`nvs_probe_call_enter`]/
    /// [`nvs_probe_call_exit`]'s whole effect under [`DebugFlags::TRACE`].
    ///
    /// The kind is [`TraceKind::Call`] and is not a parameter: a probe is the
    /// only thing that reaches this method, and the other kinds are
    /// emitted from routines that carry facts this record has no field for
    /// (`rule:observability/trace-events-carry-a-kind`).
    pub fn record_trace(&mut self, callee: &str, status: Option<i32>) {
        self.trace.push(TraceEvent {
            kind: TraceKind::Call,
            callee: callee.to_owned(),
            status,
        });
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
    /// **The span arrives already rendered, and that is the crate boundary
    /// rather than laziness.** `rule:observability/a-query-is-a-trace-event`
    /// 's field set lives in `nvs_db::QuerySpan`, in a crate that depends on
    /// this one; a struct here holding the same facts would be that field
    /// set's second home, and the one nobody edits when a driver adds to it.
    /// What it costs is that a consumer reads text where it will later read
    /// fields — which is what the `trace` field's own doc comment already says
    /// this vector is, a stand-in until `rule:testing/debug-probes`'s sink gives every kind its
    /// payload.
    ///
    /// The rendering is `QuerySpan`'s `Display`, so § 11's "never parameters"
    /// is held where the span is built and
    /// `a_query_span_contains_no_parameter_value_anywhere` asserts it over that
    /// same rendering; nothing here can put a bound value back.
    pub fn record_query(&mut self, span: &str) {
        self.trace.push(TraceEvent {
            kind: TraceKind::Query,
            callee: span.to_owned(),
            status: None,
        });
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
    pub fn record_http(&mut self, span: &str) {
        self.trace.push(TraceEvent {
            kind: TraceKind::Http,
            callee: span.to_owned(),
            status: None,
        });
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
        let started = SPAWN_EPOCH.elapsed();
        self.trace.push(TraceEvent {
            kind: TraceKind::Spawn,
            callee: format!("{} started={started:?} unjoined", form.spelling()),
            status: None,
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
        let payload = render_spawn(open.form, open.started, SPAWN_EPOCH.elapsed(), child);
        // Indexed rather than assumed: the trace is only ever appended to, so
        // the position holds, and a miss loses one event rather than panicking
        // inside a request that was only being observed.
        if let Some(event) = self.trace.get_mut(open.index) {
            event.callee = payload;
        }
    }

    /// The call-site trace gathered so far, in the order the probes fired —
    /// empty for a request that ran with [`DebugFlags::TRACE`] off
    /// throughout. See the field's own doc comment for why this accumulates
    /// in memory and will not once `rule:testing/debug-probes`'s sink exists.
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
/// differs in returning nothing — coverage bookkeeping cannot fail, and ADR
/// 0018 puts a debugger break at a safepoint, not at a probe — so a compiled
/// probe site has no status to check and no error edge to emit.
///
/// Only [`DebugFlags::COVERAGE`] acts here. `BRANCH` needs the per-edge probe
/// site that lands with `nvs_ir::Terminator::Branch`'s lowering; `TRACE` and
/// `PROFILE` are the call-site pair below.
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_probe_with_coverage_off_records_nothing() {
        let mut ctx = Ctx::buffered();
        for stmt in 0..4 {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            unsafe {
                nvs_probe_stmt(&raw mut ctx, stmt);
            }
        }
        assert!(ctx.stmt_hits().is_empty());
    }

    #[test]
    fn a_probe_counts_a_hit_per_statement_once_coverage_is_on() {
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        for stmt in [2_u32, 0, 2] {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            unsafe {
                nvs_probe_stmt(&raw mut ctx, stmt);
            }
        }
        // Statement 1 never ran; 2 ran twice. The table is dense, so an
        // unexecuted statement between two executed ones reads back as zero
        // rather than as absent.
        assert_eq!(ctx.stmt_hits(), [1, 0, 2]);
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

    #[test]
    fn a_call_probe_with_tracing_off_records_nothing() {
        let mut ctx = Ctx::buffered();
        let name = b"Math::double";
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
            nvs_probe_call_exit(&raw mut ctx, name.as_ptr(), name.len(), crate::OK);
        }
        // Coverage on, tracing still off: the two flags are independent, and
        // the call probe reads its own bit rather than "any bit set".
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
        }
        assert!(ctx.trace().is_empty());
    }

    #[test]
    fn a_call_probe_records_the_callee_and_the_status_it_is_handed() {
        // The exit probe carries the checked-return status the call site is
        // about to branch on, so a thrown or `FATAL` exit is recorded as it
        // happened rather than reconstructed — `rule:testing/debug-probes`.
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::TRACE);
        let name = b"Boom::inner";
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
            nvs_probe_call_exit(&raw mut ctx, name.as_ptr(), name.len(), crate::THROWN);
        }
        assert_eq!(
            ctx.trace(),
            [
                TraceEvent {
                    kind: TraceKind::Call,
                    callee: "Boom::inner".to_owned(),
                    status: None,
                },
                TraceEvent {
                    kind: TraceKind::Call,
                    callee: "Boom::inner".to_owned(),
                    status: Some(crate::THROWN),
                },
            ]
        );
    }

    /// `rule:observability/trace-events-carry-a-kind`'s kind, over the kinds
    /// anything in the tree records: a probe files a `call`, `Core\Db`'s
    /// statement routine files a `query` and `Core\Http\Client`'s transport
    /// files an `http`, and one vector keeps them apart. Asserted as the whole
    /// trace rather than on the events after the first, because the tag only
    /// earns its place if the `call` still reads as one beside them.
    #[test]
    fn each_recorded_kind_is_filed_under_its_own_tag_beside_a_call() {
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::TRACE);
        ctx.record_trace("People::all", Some(crate::OK));
        // What `nvs_db::QuerySpan`'s `Display` hands over, which is the whole
        // of what a `query` event carries — no bound value among it, per
        // `rule:observability/a-query-is-a-trace-event`.
        ctx.record_query("driver=postgres connection=main rows=2 sql=select 1");
        // And the transport's own span, which carries no query string, header
        // value or body for the same rule's reason.
        ctx.record_http("http method=GET scheme=https host=api.example.com port=443 path=/things");
        assert_eq!(
            ctx.trace(),
            [
                TraceEvent {
                    kind: TraceKind::Call,
                    callee: "People::all".to_owned(),
                    status: Some(crate::OK),
                },
                TraceEvent {
                    kind: TraceKind::Query,
                    callee: "driver=postgres connection=main rows=2 sql=select 1".to_owned(),
                    status: None,
                },
                TraceEvent {
                    kind: TraceKind::Http,
                    callee: "http method=GET scheme=https host=api.example.com port=443 \
                             path=/things"
                        .to_owned(),
                    status: None,
                },
            ]
        );
    }

    #[test]
    fn a_call_probe_accepts_an_empty_label_without_reading_the_pointer() {
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::TRACE);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, std::ptr::null(), 0);
        }
        assert_eq!(ctx.trace().len(), 1);
        assert_eq!(ctx.trace()[0].callee, "");
    }

    #[test]
    fn coverage_can_be_turned_on_and_off_mid_request() {
        // `rule:testing/debug-probes`'s whole argument for a runtime-checked flag over a second
        // compiled tier: a harness brackets one test inside a running request.
        let mut ctx = Ctx::buffered();
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

        assert_eq!(ctx.stmt_hits(), [0, 1]);
    }

    /// `rule:observability/four-kinds-become-a-span`'s three derived kinds are
    /// filed because the trace is recorded, with no debug flag anywhere — and
    /// the fourth, a `call`, still is not, because the probes read their own bit
    /// and a span per compiled call site is the cost
    /// `rule:observability/a-call-never-becomes-a-span` refuses.
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
        ctx.record_query("driver=postgres connection=main rows=1 sql=select 1");
        ctx.record_http("http method=GET scheme=https host=api.example.com port=443 path=/things");

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

    /// The bound a recorded run is held to, and the debugging run that is not:
    /// a program querying in a loop stops filing, while a trace somebody asked
    /// for is never truncated under them.
    #[test]
    fn a_recorded_runs_filing_stops_at_the_ceiling_and_a_debugged_ones_does_not() {
        let mut ctx = Ctx::buffered();
        ctx.set_trace_context(crate::TraceContext::rooted(1.0));
        for _ in 0..SPAN_EVENT_CEILING {
            assert!(ctx.records_spans(), "the ceiling closed early");
            ctx.record_query("driver=postgres connection=main rows=1 sql=select 1");
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
            ctx.records_spans(),
            "a debugger's trace was truncated by a ceiling that is not its"
        );
    }
}
