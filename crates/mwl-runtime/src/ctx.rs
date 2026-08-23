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
//! process's stdout directly. Two reasons, in `CLAUDE.md`'s priority order:
//! under `mwl serve` a request's output is its HTTP response body, not a
//! process-wide stream (priority 1, request isolation); and a test can assert
//! on [`OutputSink::Buffer`] without capturing the process's real stdout
//! (priority 4).

use std::borrow::Cow;
use std::io::{self, Write};

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
    /// The message behind a pending `THROWN` or `FATAL` status.
    ///
    /// `Cow` rather than `String` deliberately: `benches/abi-probe` measured a
    /// throw at 2.8x a normal return when the message allocated, versus
    /// *cheaper* than a return when it does not — and PHP code throws on
    /// ordinary control-flow paths. [ADR 0002](../../../docs/adr/0002-error-propagation.md)
    /// § *Measured cost* holds the numbers; this field is the property they
    /// depend on.
    pending: Option<Cow<'static, str>>,
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
            output,
            stmt_hits: Vec::new(),
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

    /// Records the message behind a `THROWN` or `FATAL` status.
    pub fn set_pending(&mut self, message: impl Into<Cow<'static, str>>) {
        self.pending = Some(message.into());
    }

    /// The pending message, if any, without clearing it.
    #[must_use]
    pub fn pending(&self) -> Option<&str> {
        self.pending.as_deref()
    }

    /// Takes the pending message, clearing it — what a `catch` does once it
    /// has handled the throw.
    #[must_use]
    pub fn take_pending(&mut self) -> Option<Cow<'static, str>> {
        self.pending.take()
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
/// Only [`DebugFlags::COVERAGE`] acts. `BRANCH` needs the per-edge probe site
/// that lands with `mwl_ir::Terminator::Branch`'s lowering, and `TRACE`/
/// `PROFILE` are call-site probes with no MWL-level call compiled yet — see
/// `mwl-codegen`'s own module docs for that split.
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
        assert_eq!(ctx.pending(), None);
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
        assert_eq!(ctx.pending(), Some("boom"));
        assert_eq!(ctx.take_pending().as_deref(), Some("boom"));
        assert_eq!(ctx.pending(), None);
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
            assert_eq!(ctx.pending(), Some(message));
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
        assert_eq!(ctx.pending(), None);
    }
}
