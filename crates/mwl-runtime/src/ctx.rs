//! The per-request context: the first argument of every compiled function and
//! every helper.
//!
//! `Ctx` is where everything that is "ambient" to running MWL code lives,
//! because [ADR 0012](../../../docs/adr/0012-no-superglobals.md) means nothing
//! is ambient to the *language*: no variable is host-populated, so the host's
//! state has to travel somewhere, and it travels here.
//!
//! # Layout is part of the ABI
//!
//! The two hot words come first, in a `#[repr(C)]` struct, because compiled
//! code loads them inline rather than calling anything:
//!
//! * [`SAFEPOINT_OFFSET`] — the safepoint poll `mwl-codegen` emits at every
//!   function entry and loop back edge (`docs/adr/README.md`'s project-start
//!   decisions). Load, test, predicted-not-taken branch to the
//!   [`mwl_safepoint`] slow path.
//! * [`DEBUG_FLAGS_OFFSET`] — [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
//!   § 1's probe check, at every statement boundary and every call site. Same
//!   shape, same cost class, and present in every compiled unit whether or not
//!   any request ever sets a bit — that is what makes coverage and tracing
//!   start/stoppable *mid-request*, which the rejected instrumented-tier
//!   design could not do.
//!
//! Both are exposed as `offset_of!` constants rather than restated numbers, so
//! adding a field can never silently desynchronise codegen from this struct.
//!
//! # Output
//!
//! `Ctx` owns where `echo` writes, rather than the runtime writing to the
//! process's stdout directly. Two reasons, in `AGENTS.md`'s priority order:
//! under `mwl serve` a request's output is its HTTP response body, not a
//! process-wide stream (priority 1, request isolation); and a test can assert
//! on [`OutputSink::Buffer`] without capturing the process's real stdout
//! (priority 4).

use std::borrow::Cow;
use std::io::{self, Write};

use crate::object::{ClassDesc, ClassId, ClassTable};
use crate::throwable::Thrown;

bitflags::bitflags! {
    /// What a safepoint poll has been asked to do.
    ///
    /// The word is checked, not the individual bits: compiled code branches on
    /// "is this non-zero", and only the [`mwl_safepoint`] slow path looks at
    /// which bit is set.
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    #[repr(transparent)]
    pub struct SafepointFlags: u64 {
        /// The request has exceeded its CPU-time budget.
        const CPU_LIMIT = 1 << 0;
        /// The client disconnected, or the request was cancelled.
        const CANCEL = 1 << 1;
        /// The cycle collector wants to stop the world.
        const COLLECT = 1 << 2;
        /// A debugger wants to break here.
        const DEBUG_BREAK = 1 << 3;
    }
}

bitflags::bitflags! {
    /// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// § 1's per-request debug-flags word.
    ///
    /// Setting a bit on a request that is already running is the whole
    /// mechanism: no recompilation, no re-resolution, no second cache key.
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    #[repr(transparent)]
    pub struct DebugFlags: u64 {
        /// Count a hit per line at every statement boundary.
        const COVERAGE = 1 << 0;
        /// Count a hit per conditional CFG edge, keyed by `mwl_ir::EdgeId`.
        const BRANCH = 1 << 1;
        /// Emit an entry/exit probe around every call.
        const TRACE = 1 << 2;
        /// Accumulate self/inclusive time around every call.
        const PROFILE = 1 << 3;
    }
}

/// Where a request's `echo` output goes.
#[derive(Debug)]
#[non_exhaustive]
pub enum OutputSink {
    /// The process's standard output — `mwl run`'s destination.
    Stdout,
    /// An in-memory buffer, read back with [`Ctx::take_buffered_output`].
    ///
    /// This is what a test uses, and the shape an HTTP response body will
    /// reuse in M7.
    Buffer(Vec<u8>),
    /// Discarded.
    Sink,
}

/// Per-request state, passed to every compiled MWL function and every helper.
#[repr(C)]
#[derive(Debug)]
pub struct Ctx {
    /// Hot. Read inline by every safepoint poll; see the module docs.
    safepoint: SafepointFlags,
    /// Hot. Read inline by every ADR 0018 probe site; see the module docs.
    debug: DebugFlags,
    /// What is behind a pending `THROWN` or `FATAL` status — see [`Pending`]
    /// for why one field carries both shapes rather than two sitting beside
    /// each other.
    pending: Option<Pending>,
    /// Where `echo` writes.
    output: OutputSink,
    /// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// § 1's statement-boundary hit counters, indexed by `mwl_ir::StmtId`.
    ///
    /// Written only from [`mwl_probe_stmt`], which compiled code reaches only
    /// when the [`DebugFlags`] word above is non-zero — so a request with no
    /// probe enabled never touches this vector and never allocates it.
    ///
    /// **A stand-in, not the final shape.** `mwl_ir::StmtId` numbers from zero
    /// within *each* function, so two functions' statements collide in this
    /// one table. ADR 0018 wants path → line → count, which needs the unit and
    /// function a statement belongs to; that qualification arrives with
    /// `Core\Debug` and the Clover/lcov exporters in M10. What this table is
    /// for now is proving the mechanism: the probe fires at exactly the
    /// statements a request executed, and nowhere else.
    stmt_hits: Vec<u64>,
    /// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// § 1's call-site trace, in the order the probes fired.
    ///
    /// Written only from [`mwl_probe_call_enter`]/[`mwl_probe_call_exit`],
    /// under the same "the flags word was non-zero" gate `stmt_hits` is under.
    ///
    /// **A stand-in, not the final shape**, for the same reason `stmt_hits`
    /// is one, plus a second: ADR 0018 has trace and profile data *stream to
    /// a sink* rather than accumulate, precisely because a long-running
    /// request's trace is call-count-proportional. This vector is bounded by
    /// nothing, which is why it exists only until `Core\Debug` names a sink —
    /// it is proving the probe fires at the right places, not serving a
    /// request. `PROFILE`'s self/inclusive timing shares these two sites and
    /// lands with that sink.
    trace: Vec<TraceEvent>,
    /// The class a bare-message failure is promoted to, if one was
    /// installed — see [`Ctx::set_runtime_error_class`].
    runtime_error_class: Option<ErrorClass>,
    /// The armed fault-injection site, if any. See [`FaultSite`].
    ///
    /// A request with nothing armed — every request that is not a
    /// `mwl run --fault-inject=…` — pays one `Option` test per helper entry
    /// and touches `helper_calls` never.
    fault: Option<FaultSite>,
    /// How many runtime helpers this request has entered, counted only while
    /// `fault` is armed. See [`FaultSite::HelperPanic`].
    helper_calls: u32,
}

/// One class descriptor, plus the table that owns it.
///
/// The safe way to hand a [`Ctx`] a descriptor that must outlive it: a
/// [`ClassDesc`]'s *address* is its identity (`crate::object`), so a context
/// cannot make one up and still match a `catch` clause's — it has to be handed
/// the compiled unit's. Carrying the [`Rc`](std::rc::Rc)-shared
/// [`ClassTable`] along with the id is what turns "the table must outlive the
/// context" from a contract into a fact, which is why
/// [`Ctx::set_runtime_error_class`] needs no `unsafe` at all.
#[derive(Clone, Debug)]
pub struct ErrorClass {
    table: std::rc::Rc<ClassTable>,
    id: ClassId,
}

impl ErrorClass {
    /// A handle on `id` within `table`.
    #[must_use]
    pub fn new(table: std::rc::Rc<ClassTable>, id: ClassId) -> Self {
        Self { table, id }
    }

    /// The descriptor's address, live for as long as this handle is.
    #[must_use]
    pub fn desc(&self) -> *const ClassDesc {
        self.table.desc(self.id)
    }
}

/// What is behind a pending non-[`crate::OK`] status.
///
/// One field on [`Ctx`], not two: the exception object **subsumes** the
/// message a [`crate::Fault`] used to leave behind, rather than sitting beside
/// it. Two fields would mean two places to ask "what failed", and every read
/// would have to state which one wins.
///
/// The two variants are not two kinds of failure — they are the same failure
/// at two levels of detail:
///
/// * [`Pending::Message`] is what a runtime helper's [`crate::Fault`] and
///   every [`crate::FATAL`] produce. It allocates nothing when the message is
///   `'static`, which is the property
///   [ADR 0002](../../../docs/adr/0002-error-propagation.md) § *Measured cost*
///   depends on: `benches/abi-probe` measured a throw at 2.8x a normal return
///   with an allocating message and *cheaper* than a return without one, and
///   PHP code throws on ordinary control-flow paths.
/// * [`Pending::Thrown`] is what MWL's own `throw` produces, and the only one
///   carrying a backtrace. A `Message` is promoted to one on demand — by
///   [`Ctx::take_thrown`] when a `catch` dispatch takes it, or by
///   [`Ctx::push_frame`] when a `THROWN` unwinds a compiled frame — so a
///   helper-raised throw is catchable and traceable without every helper
///   paying for an allocation it usually does not need.
///
/// Promotion needs a class to build the object from, which only the compiled
/// unit has — see [`Ctx::set_runtime_error_class`]. With none installed, a
/// `Message` stays a message: it is still reported, and still ends the
/// request, but no `catch` clause matches it and it grows no backtrace.
///
/// A [`crate::FATAL`] never becomes a `Thrown`: compiled code only ever pushes
/// a frame for a `THROWN` status, and no `catch` is ever entered for a
/// `FATAL` ([ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)).
#[derive(Debug)]
enum Pending {
    /// A message alone, with no exception object behind it yet.
    Message(Cow<'static, str>),
    /// MWL's own exception object.
    Thrown(Thrown),
}

impl Pending {
    /// The message, whichever shape this is.
    fn message(&self) -> Cow<'_, str> {
        match self {
            Self::Message(message) => Cow::Borrowed(message),
            Self::Thrown(thrown) => Cow::Owned(thrown.message()),
        }
    }

    /// This failure as an exception object, allocating one of `class` around a
    /// bare message if that is all there is.
    ///
    /// # Safety
    ///
    /// `class` must be null or refer to a live class descriptor that outlives
    /// every instance made from it.
    #[expect(
        unsafe_code,
        reason = "the descriptor's liveness is the caller's obligation and \
                  cannot be expressed in the signature"
    )]
    unsafe fn into_thrown(self, class: *const ClassDesc) -> Thrown {
        match self {
            #[expect(unsafe_code, reason = "forwarding this function's own contract")]
            Self::Message(message) => unsafe { Thrown::new(class, &message) },
            Self::Thrown(thrown) => thrown,
        }
    }
}

/// A failure a run can be *asked* to produce, for a mode that by definition
/// has no user-facing trigger.
///
/// The set is closed on purpose, and reachable only through `mwl run
/// --fault-inject=<site>`: it must never be reachable from a served request
/// (`mwl serve`, M7), and nothing in MWL source can arm one.
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

/// One [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
/// § 1 call-site trace record.
///
/// [ADR 0041](../../../docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md)
/// adds a `call`/`gc`/`spawn` kind alongside this; every event here is a
/// `call`, since the GC and isolate-spawn routines it also instruments do not
/// exist yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceEvent {
    /// The callee's `Class::method` label.
    pub callee: String,
    /// `None` on entry; on exit, the status the call site is about to branch
    /// on — so a trace records a thrown or `FATAL` exit exactly as it
    /// happened rather than as a reconstruction.
    pub status: Option<i32>,
}

/// Byte offset of the safepoint word within [`Ctx`] — see the module docs.
pub const SAFEPOINT_OFFSET: usize = std::mem::offset_of!(Ctx, safepoint);

/// Byte offset of the debug-flags word within [`Ctx`] — see the module docs.
pub const DEBUG_FLAGS_OFFSET: usize = std::mem::offset_of!(Ctx, debug);

impl Ctx {
    /// A context writing to the given sink, with nothing pending and every
    /// flag clear.
    #[must_use]
    pub fn new(output: OutputSink) -> Self {
        Self {
            safepoint: SafepointFlags::empty(),
            debug: DebugFlags::empty(),
            pending: None,
            runtime_error_class: None,
            output,
            stmt_hits: Vec::new(),
            trace: Vec::new(),
            fault: None,
            helper_calls: 0,
        }
    }

    /// A context writing to the process's standard output.
    #[must_use]
    pub fn stdout() -> Self {
        Self::new(OutputSink::Stdout)
    }

    /// A context buffering its output in memory.
    #[must_use]
    pub fn buffered() -> Self {
        Self::new(OutputSink::Buffer(Vec::new()))
    }

    /// The pending safepoint requests.
    #[must_use]
    pub fn safepoint_flags(&self) -> SafepointFlags {
        self.safepoint
    }

    /// Asks the next safepoint poll to act.
    pub fn request_safepoint(&mut self, flags: SafepointFlags) {
        self.safepoint |= flags;
    }

    /// The active [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// probes.
    #[must_use]
    pub fn debug_flags(&self) -> DebugFlags {
        self.debug
    }

    /// Turns probes on or off for a request that may already be running.
    pub fn set_debug_flags(&mut self, flags: DebugFlags) {
        self.debug = flags;
    }

    /// Counts one hit for the statement `stmt` names — [`mwl_probe_stmt`]'s
    /// whole effect under [`DebugFlags::COVERAGE`].
    pub fn record_stmt_hit(&mut self, stmt: u32) {
        let index = stmt as usize;
        if self.stmt_hits.len() <= index {
            self.stmt_hits.resize(index + 1, 0);
        }
        self.stmt_hits[index] += 1;
    }

    /// The per-statement hit counters gathered so far, indexed by
    /// `mwl_ir::StmtId` — empty for a request that ran with
    /// [`DebugFlags::COVERAGE`] off throughout. See the field's own doc
    /// comment for why this is a stand-in for ADR 0018's path → line → count
    /// shape rather than that shape itself.
    #[must_use]
    pub fn stmt_hits(&self) -> &[u64] {
        &self.stmt_hits
    }

    /// Records one call-site trace event — [`mwl_probe_call_enter`]/
    /// [`mwl_probe_call_exit`]'s whole effect under [`DebugFlags::TRACE`].
    pub fn record_trace(&mut self, callee: &str, status: Option<i32>) {
        self.trace.push(TraceEvent {
            callee: callee.to_owned(),
            status,
        });
    }

    /// The call-site trace gathered so far, in the order the probes fired —
    /// empty for a request that ran with [`DebugFlags::TRACE`] off
    /// throughout. See the field's own doc comment for why this accumulates
    /// in memory today and will not once ADR 0018's sink exists.
    #[must_use]
    pub fn trace(&self) -> &[TraceEvent] {
        &self.trace
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

    /// Records the message behind a `THROWN` or `FATAL` status.
    pub fn set_pending(&mut self, message: impl Into<Cow<'static, str>>) {
        self.pending = Some(Pending::Message(message.into()));
    }

    /// Installs the class a bare-message failure is promoted to — spec
    /// § 10's `RuntimeError`, "the world said no", which is what a runtime
    /// helper's failure is.
    ///
    /// A caller that never installs one gets the degraded behaviour
    /// [`Pending`] describes, never a crash.
    pub fn set_runtime_error_class(&mut self, class: ErrorClass) {
        self.runtime_error_class = Some(class);
    }

    /// The installed descriptor's address, or null.
    fn runtime_error_desc(&self) -> *const ClassDesc {
        self.runtime_error_class
            .as_ref()
            .map_or(std::ptr::null(), ErrorClass::desc)
    }

    /// Records an already-built exception as the pending `THROWN` — what
    /// [`mwl_raise`] does for MWL's own `throw`, taking ownership of the
    /// reference it was handed.
    pub fn raise(&mut self, thrown: Thrown) {
        self.pending = Some(Pending::Thrown(thrown));
    }

    /// The pending message, if any, without clearing it.
    #[must_use]
    pub fn pending(&self) -> Option<Cow<'_, str>> {
        self.pending.as_ref().map(Pending::message)
    }

    /// Takes the pending message, clearing it — and dropping the exception
    /// object behind it, if there was one.
    #[must_use]
    pub fn take_pending(&mut self) -> Option<Cow<'static, str>> {
        Some(match self.pending.take()? {
            Pending::Message(message) => message,
            Pending::Thrown(thrown) => Cow::Owned(thrown.message()),
        })
    }

    /// Takes the pending failure as an exception object, clearing it — what a
    /// `catch` binds to its variable, and what `mwl run` reports a backtrace
    /// from.
    ///
    /// Promotes a bare [`Pending::Message`] rather than returning `None` for
    /// one: a helper-raised `THROWN` is as catchable as MWL's own, it just has
    /// no backtrace to show.
    #[must_use]
    pub fn take_thrown(&mut self) -> Thrown {
        let Some(pending) = self.pending.take() else {
            return Thrown::none();
        };
        #[expect(
            unsafe_code,
            reason = "`set_runtime_error_class`'s own contract makes the \
                      installed descriptor outlive every instance built here"
        )]
        unsafe {
            pending.into_thrown(self.runtime_error_desc())
        }
    }

    /// Records one more frame a pending `THROWN` has unwound out of.
    ///
    /// A bare message is promoted to a real exception here, so a helper-raised
    /// throw accumulates a backtrace from the first compiled frame it leaves.
    pub fn push_frame(&mut self, label: &str) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        // Captured before promotion, so the message survives the one case
        // promotion cannot produce an object for — see `Pending`'s note on an
        // uninstalled class.
        let message = pending.message().into_owned();
        #[expect(
            unsafe_code,
            reason = "`set_runtime_error_class`'s own contract makes the \
                      installed descriptor outlive every instance built here"
        )]
        let thrown = unsafe { pending.into_thrown(self.runtime_error_desc()) };
        if thrown.is_none() {
            self.pending = Some(Pending::Message(Cow::Owned(message)));
            return;
        }
        thrown.push_frame(label);
        self.pending = Some(Pending::Thrown(thrown));
    }

    /// Writes raw bytes to this request's output, unescaped.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns. [`OutputSink::Buffer`] and
    /// [`OutputSink::Sink`] never fail.
    pub fn write_output(&mut self, bytes: &[u8]) -> io::Result<()> {
        match &mut self.output {
            OutputSink::Stdout => io::stdout().write_all(bytes),
            OutputSink::Buffer(buffer) => {
                buffer.extend_from_slice(bytes);
                Ok(())
            }
            OutputSink::Sink => Ok(()),
        }
    }

    /// Flushes this request's output.
    ///
    /// `mwl run` calls this once the script's frame returns: Rust's standard
    /// output is line-buffered, and a script whose last `echo` has no trailing
    /// newline would otherwise depend on the process-exit flush.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns.
    pub fn flush_output(&mut self) -> io::Result<()> {
        match &mut self.output {
            OutputSink::Stdout => io::stdout().flush(),
            OutputSink::Buffer(_) | OutputSink::Sink => Ok(()),
        }
    }

    /// Takes everything written so far, if this context buffers its output.
    #[must_use]
    pub fn take_buffered_output(&mut self) -> Option<Vec<u8>> {
        match &mut self.output {
            OutputSink::Buffer(buffer) => Some(std::mem::take(buffer)),
            OutputSink::Stdout | OutputSink::Sink => None,
        }
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Self::stdout()
    }
}

/// The safepoint poll's slow path — reached only when the word compiled code
/// loaded was non-zero.
///
/// Returns [`crate::FATAL`] for a request that must stop, and [`crate::OK`]
/// otherwise. A resource-limit stop is deliberately not a `THROWN`:
/// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) makes it not
/// a `Throwable` at the type level, so no MWL `catch` can see it.
///
/// Two of the four flags act; see the crate docs' known gap 6.
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
pub unsafe extern "C" fn mwl_safepoint(ctx: *mut Ctx) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ctx` is valid for this call; nothing \
                  here can panic, so no `catch_unwind` is needed to keep the \
                  unwind out of the JIT frame above"
    )]
    let ctx = unsafe { &mut *ctx };

    if ctx.safepoint.contains(SafepointFlags::CPU_LIMIT) {
        ctx.set_pending("the request exceeded its CPU-time limit");
        return crate::FATAL;
    }
    if ctx.safepoint.contains(SafepointFlags::CANCEL) {
        ctx.set_pending("the request was cancelled");
        return crate::FATAL;
    }
    ctx.safepoint
        .remove(SafepointFlags::COLLECT | SafepointFlags::DEBUG_BREAK);
    crate::OK
}

/// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
/// § 1's statement-boundary probe — the slow path behind the debug-flags
/// check, reached only when the word compiled code loaded was non-zero.
///
/// Deliberately the same *shape* as [`mwl_safepoint`]: one cached load and one
/// predicted-not-taken branch at the site, everything else out of line. It
/// differs in returning nothing — coverage bookkeeping cannot fail, and ADR
/// 0018 puts a debugger break at a safepoint, not at a probe — so a compiled
/// probe site has no status to check and no error edge to emit.
///
/// Only [`DebugFlags::COVERAGE`] acts here. `BRANCH` needs the per-edge probe
/// site that lands with `mwl_ir::Terminator::Branch`'s lowering; `TRACE` and
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
pub unsafe extern "C" fn mwl_probe_stmt(ctx: *mut Ctx, stmt: u32) {
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
/// Lossy rather than fallible: the bytes come from an MWL identifier the
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

/// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
/// § 1's call-site **entry** probe — the slow path behind the debug-flags
/// check compiled code emits before every call.
///
/// The callee is passed as a pointer and length into the compiled unit's own
/// data section rather than as an index into a side table: the name is
/// already a static constant of the unit, so there is nothing for a table to
/// add and nothing to keep in sync.
///
/// Like [`mwl_probe_stmt`], it returns nothing — a trace record cannot fail —
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
pub unsafe extern "C" fn mwl_probe_call_enter(ctx: *mut Ctx, name: *const u8, len: usize) {
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

/// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
/// § 1's call-site **exit** probe, carrying the checked-return `status` the
/// call site is about to branch on — which is why a trace shows a thrown or
/// `FATAL` exit as it happened rather than as a reconstruction.
///
/// See [`mwl_probe_call_enter`] for the rest, including why the flags word is
/// re-read here rather than the entry probe's answer being reused: a request
/// may turn tracing on or off *during* the call, and an exit whose flag state
/// differs from its entry's is the honest record of that.
///
/// # Safety
///
/// The same contract as [`mwl_probe_call_enter`].
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer and a static byte \
              range; neither contract can be expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_probe_call_exit(
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

    #[test]
    fn the_hot_words_come_first_and_are_a_word_apart() {
        assert_eq!(SAFEPOINT_OFFSET, 0);
        assert_eq!(DEBUG_FLAGS_OFFSET, 8);
    }

    #[test]
    fn a_fresh_context_has_nothing_set() {
        let ctx = Ctx::buffered();
        assert!(ctx.safepoint_flags().is_empty());
        assert!(ctx.debug_flags().is_empty());
        assert!(ctx.pending().is_none());
    }

    #[test]
    fn buffered_output_accumulates_and_is_taken_once() {
        let mut ctx = Ctx::buffered();
        ctx.write_output(b"Hello, ").unwrap();
        ctx.write_output(b"World!").unwrap();
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"Hello, World!"[..])
        );
        assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
    }

    #[test]
    fn a_discarding_sink_reports_nothing_buffered() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.write_output(b"gone").unwrap();
        assert!(ctx.take_buffered_output().is_none());
    }

    #[test]
    fn a_pending_message_is_taken_once() {
        let mut ctx = Ctx::buffered();
        ctx.set_pending("boom");
        assert_eq!(ctx.pending().as_deref(), Some("boom"));
        assert_eq!(ctx.take_pending().as_deref(), Some("boom"));
        assert!(ctx.pending().is_none());
    }

    #[test]
    fn a_limit_or_cancel_safepoint_is_fatal_and_uncatchable() {
        for (flag, message) in [
            (
                SafepointFlags::CPU_LIMIT,
                "the request exceeded its CPU-time limit",
            ),
            (SafepointFlags::CANCEL, "the request was cancelled"),
        ] {
            let mut ctx = Ctx::buffered();
            ctx.request_safepoint(flag);
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            let status = unsafe { mwl_safepoint(&raw mut ctx) };
            assert_eq!(status, crate::FATAL);
            assert_eq!(ctx.pending().as_deref(), Some(message));
        }
    }

    #[test]
    fn a_probe_with_coverage_off_records_nothing() {
        let mut ctx = Ctx::buffered();
        for stmt in 0..4 {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            unsafe {
                mwl_probe_stmt(&raw mut ctx, stmt);
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
                mwl_probe_stmt(&raw mut ctx, stmt);
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
            mwl_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
            mwl_probe_call_exit(&raw mut ctx, name.as_ptr(), name.len(), crate::OK);
        }
        // Coverage on, tracing still off: the two flags are independent, and
        // the call probe reads its own bit rather than "any bit set".
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            mwl_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
        }
        assert!(ctx.trace().is_empty());
    }

    #[test]
    fn a_call_probe_records_the_callee_and_the_status_it_is_handed() {
        // The exit probe carries the checked-return status the call site is
        // about to branch on, so a thrown or `FATAL` exit is recorded as it
        // happened rather than reconstructed — ADR 0018 § 1.
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::TRACE);
        let name = b"Boom::inner";
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            mwl_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
            mwl_probe_call_exit(&raw mut ctx, name.as_ptr(), name.len(), crate::THROWN);
        }
        assert_eq!(
            ctx.trace(),
            [
                TraceEvent {
                    callee: "Boom::inner".to_owned(),
                    status: None,
                },
                TraceEvent {
                    callee: "Boom::inner".to_owned(),
                    status: Some(crate::THROWN),
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
            mwl_probe_call_enter(&raw mut ctx, std::ptr::null(), 0);
        }
        assert_eq!(ctx.trace().len(), 1);
        assert_eq!(ctx.trace()[0].callee, "");
    }

    #[test]
    fn coverage_can_be_turned_on_and_off_mid_request() {
        // ADR 0018's whole argument for a runtime-checked flag over a second
        // compiled tier: a harness brackets one test inside a running request.
        let mut ctx = Ctx::buffered();
        let probe = |ctx: &mut Ctx, stmt: u32| {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            unsafe {
                mwl_probe_stmt(&raw mut *ctx, stmt);
            }
        };

        probe(&mut ctx, 0);
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        probe(&mut ctx, 1);
        ctx.set_debug_flags(DebugFlags::empty());
        probe(&mut ctx, 2);

        assert_eq!(ctx.stmt_hits(), [0, 1]);
    }

    #[test]
    fn an_unimplemented_safepoint_request_is_cleared_rather_than_acted_on() {
        let mut ctx = Ctx::buffered();
        ctx.request_safepoint(SafepointFlags::COLLECT | SafepointFlags::DEBUG_BREAK);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let status = unsafe { mwl_safepoint(&raw mut ctx) };
        assert_eq!(status, crate::OK);
        assert!(ctx.safepoint_flags().is_empty());
        assert!(ctx.pending().is_none());
    }
}
