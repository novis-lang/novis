//! The words compiled code polls, and the two helpers that poll them.
//!
//! The safepoint flags, the call-stack bounds and the deadline are the hot
//! words the module docs describe as part of the ABI; this file is the accessor
//! side of them, plus [`nvs_safepoint`] and [`nvs_stack_check`] — the slow paths
//! `nvs-codegen` branches to at every function entry and loop back edge.
//!
//! The two are one file because they are one decision made twice.
//! `rule:errors/on-limit`'s stack pair and
//! [ADR 0106](/docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//! § 5's deadline both put a *load and a compare* on the request path and push
//! everything else behind it, and reading the accessor beside the helper that
//! acts on it is what keeps the two ends of that compare in agreement.

use super::*;

impl Ctx {
    /// Arms `rule:errors/escalation-ladder`
    /// § 1's two stack addresses from a base address and a ceiling: the hard
    /// floor `ceiling` bytes below `base`, and the soft limit
    /// [`STACK_RESERVE`] above the floor.
    ///
    /// The one place the pair is computed, so no caller can write a floor
    /// above its own limit. An embedder that knows the request's real stack
    /// bounds — which [`Ctx::new`] cannot discover, see the module docs — calls
    /// this with them.
    pub fn arm_stack_limit(&mut self, base: usize, ceiling: usize) {
        self.stack_floor = base.saturating_sub(ceiling);
        self.stack_limit = self.stack_floor.saturating_add(STACK_RESERVE);
    }

    /// The armed `(soft, hard)` stack addresses — see
    /// [`Ctx::arm_stack_limit`].
    #[must_use]
    pub fn stack_bounds(&self) -> (usize, usize) {
        (self.stack_limit, self.stack_floor)
    }

    /// The pending safepoint requests.
    #[must_use]
    pub fn safepoint_flags(&self) -> SafepointFlags {
        self.safepoint
    }

    /// Whether this request's deadline has passed —
    /// [ADR 0106](/docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
    /// § 5's poll, and the module docs' *The request's deadline* section owns
    /// what it costs and who may call it.
    ///
    /// One relaxed load, through a handle in the line the stack check has
    /// already brought in. The word itself is the request tree's and lives
    /// elsewhere, which is the one cache line this poll costs and the reason
    /// [`crate::bounded_loop`]'s batch is what makes it affordable.
    #[must_use]
    pub fn deadline_expired(&self) -> bool {
        self.deadline.load(std::sync::atomic::Ordering::Relaxed) != 0
    }

    /// Marks this request's deadline as passed, so the next poll inside a
    /// long-running helper observes it.
    ///
    /// Takes `&self` rather than `&mut self` deliberately: the caller is the
    /// timer, and the thread running the request holds the `&mut` already —
    /// which is exactly the case a `&mut` writer could not serve.
    ///
    /// **It expires the whole tree**, this context's isolates and tasks
    /// included, whether they were spawned before this call or after it — see
    /// [`Self::deadline`]'s field doc.
    pub fn expire_deadline(&self) {
        self.deadline.store(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// This request's coroutine yielder, or null if it is running on the main
    /// stack — see the [`Ctx::yielder`] field for why it is opaque.
    ///
    /// A helper that means to suspend tests this for null first: a `Core`
    /// member reached from `nvs run` has no scheduler beneath it, and a member
    /// that would park has to fail or block rather than pretend.
    #[must_use]
    pub fn yielder(&self) -> *const () {
        self.yielder
    }

    /// Publishes the coroutine yielder for the task this request runs inside,
    /// or clears it with a null pointer.
    ///
    /// Safe to call, because storing a pointer cannot go wrong; what the
    /// caller owes is the contract [`Ctx::yielder`]'s *reader* relies on, and
    /// `nvs-host`'s scheduler is the one place that discharges it — **the
    /// pointer must outlive every read of it**, which holds exactly while the
    /// coroutine whose stack the yielder lives on is the one running this
    /// context, and is why the scheduler nulls it again before the context
    /// leaves the coroutine.
    pub fn set_yielder(&mut self, yielder: *const ()) {
        self.yielder = yielder;
    }

    /// Asks the next safepoint poll to act.
    pub fn request_safepoint(&mut self, flags: SafepointFlags) {
        self.safepoint |= flags;
    }

    /// Whether this request has been cancelled — the flag [`Ctx::cancel`] sets.
    ///
    /// What it distinguishes is a context that *failed* from one that was
    /// stopped: both carry a pending message afterwards, and only one of them
    /// is a `Throwable` anybody may see. `nvs_host::group` is the caller, for
    /// exactly that question about a child.
    #[must_use]
    pub fn cancelled(&self) -> bool {
        self.safepoint.contains(SafepointFlags::CANCEL)
    }

    /// Stops this request for a cancellation, and answers the [`crate::Fault`]
    /// the member that was told about it returns.
    ///
    /// A `Core` member that parks can be resumed by its own task's
    /// cancellation rather than by what it was waiting for
    /// ([`crate::host::Woken::Cancelled`], [`crate::host::Outcome::Cancelled`]),
    /// because a stack standing on an `extern "C"` helper frame is one no
    /// forced unwind may cross — [`crate::HelperFrame`] owns that. What the
    /// member owes then is
    /// [ADR 0072](/docs/adr/0072-core-task-structured-concurrency.md)
    /// § 5's teardown: no `catch`, no cleanup, no user code at all.
    ///
    /// That is already exactly what [`SafepointFlags::CANCEL`] means, so this
    /// sets the flag and asks [`nvs_safepoint`] for the answer rather than
    /// inventing a second one — the status and its message keep one home, and
    /// what comes back is what the poll compiled code was going to make anyway
    /// would have said, only without the statements in between.
    pub fn cancel(&mut self) -> crate::Fault {
        self.request_safepoint(SafepointFlags::CANCEL);
        #[expect(
            unsafe_code,
            reason = "the pointer is a reborrow of this `&mut self`, which is \
                      live for the whole call"
        )]
        let status = unsafe { nvs_safepoint(&raw mut *self) };
        crate::Fault::Pending(status)
    }

    /// The active [ADR 0018](/docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// probes.
    #[must_use]
    pub fn debug_flags(&self) -> DebugFlags {
        self.debug
    }

    /// Turns probes on or off for a request that may already be running.
    pub fn set_debug_flags(&mut self, flags: DebugFlags) {
        self.debug = flags;
    }
}

/// The safepoint poll's slow path — reached only when the word compiled code
/// loaded was non-zero.
///
/// Returns [`crate::FATAL`] for a request that must stop, and [`crate::OK`]
/// otherwise. A resource-limit stop is deliberately not a `THROWN`:
/// `rule:errors/escalation-ladder` makes it not
/// a `Throwable` at the type level, so no Novis `catch` can see it.
///
/// Two of the four flags act; see the crate docs' known gap 5.
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
pub unsafe extern "C" fn nvs_safepoint(ctx: *mut Ctx) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ctx` is valid for this call; nothing \
                  here can panic, so no `catch_unwind` is needed to keep the \
                  unwind out of the JIT frame above — the one thing reached \
                  from here that is not a load and a compare is `rule:errors/on-limit`'s \
                  handler, whose own helper calls are each contained by \
                  `run_helper`"
    )]
    let ctx = unsafe { &mut *ctx };

    if ctx.safepoint.contains(SafepointFlags::CPU_LIMIT) {
        // `rule:errors/on-limit` lists CPU time beside memory, so the ladder is the same
        // two lines the memory branch below carries, in the same order: the
        // handler runs before the breach becomes the pending message, and
        // `Ctx::run_limit_handler` owns the zero-retry rule.
        //
        // The slice is the same two halves as well, and both are
        // [`Ctx::run_limit_handler`]'s to open: it widens
        // [`Ctx::cpu_limit`] by [`Ctx::fatal_reserve_time`] and lowers this flag
        // for the length of the call, which is what a handler entered *by* the
        // flag needs — a ceiling nothing consults would have bought it nothing,
        // because the flag alone stops it again at its own first back edge.
        //
        // What is missing is not the slice but the clock. Nothing samples the
        // request thread's CPU time against that widened ceiling yet
        // ([`Ctx::cpu_limit`]'s field doc owns why the sampling is the host's),
        // so this flag is raised only by a caller that already decided the
        // request is over, and nothing re-raises it when a handler overruns.
        // Until a timer exists, a handler here runs to completion and the two
        // lines below are what abandon the request; once one does, the overrun
        // is stopped by § 1's zero-retry rule with no further edit here.
        ctx.run_limit_handler(Limit::CpuTime);
        ctx.set_pending("the request exceeded its CPU-time limit");
        return crate::FATAL;
    }
    // `rule:errors/on-limit`'s memory limit, asked here as well as at every helper
    // boundary ([`crate::run_helper`]): this poll sits between two Novis
    // statements, which is the one place a limit can stop a program that is
    // allocating without calling anything. `crate::budget`'s module doc owns
    // which allocations that reaches today and which it does not.
    if let Some(crate::Fault::Fatal(message)) = ctx.memory_breach() {
        // `rule:errors/on-limit`'s tier 1, ahead of the status this returns: the handler
        // is the last thing the program gets to run, and it runs before the
        // breach becomes the message the ladder prints — so a throw of its own
        // is overwritten by `set_pending` below rather than reported in place
        // of the limit that stopped the request. `Ctx::run_limit_handler` owns
        // the zero-retry rule.
        ctx.run_limit_handler(Limit::Memory);
        ctx.set_pending(message);
        return crate::FATAL;
    }
    // `rule:errors/on-limit`'s response ceiling, asked here and nowhere else. A program
    // writes through `Ctx::write_output` and reaches this poll between two
    // statements, so a loop that echoes is stopped at its next back edge.
    // Deliberately *not* asked at `crate::run_helper` the way memory is: that
    // seam exists to refuse in front of an allocation the frame would otherwise
    // have to release, and a write leaves no such value behind to protect.
    if let Some(crate::Fault::Fatal(message)) = ctx.output_breach() {
        // The same two lines and the same order as the branch above, including
        // why the handler runs before the breach becomes the message.
        ctx.run_limit_handler(Limit::Output);
        ctx.set_pending(message);
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

/// The call-stack limit's slow path — reached only when the stack pointer
/// compiled code compared was already below [`Ctx::stack_limit`].
///
/// Compiled code tests the **soft** address alone, so which of
/// `rule:errors/on-limit`'s two
/// tiers this is gets decided here: a catchable [`ThrownClass::Recursion`]
/// between the soft address and the floor, and a [`crate::FATAL`] no `catch`
/// sees below it. That is what makes two tiers cost the same as one at the
/// site.
///
/// `sp` is the callee's own stack pointer, passed rather than re-read: this
/// frame's is a different and lower address, and the compare that got here was
/// against the caller's.
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
pub unsafe extern "C" fn nvs_stack_check(ctx: *mut Ctx, sp: u64) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ctx` is valid for this call; nothing \
                  here can panic, so no `catch_unwind` is needed to keep the \
                  unwind out of the JIT frame above"
    )]
    let ctx = unsafe { &mut *ctx };

    // `I64` is the width `nvs-codegen` gives every pointer-shaped value, and
    // this JIT targets 64-bit hosts only; an address that does not fit a
    // `usize` is therefore not this machine's stack pointer, and the safe
    // reading of a value that cannot be one is the one that stops the request.
    let sp = usize::try_from(sp).unwrap_or(0);
    if sp < ctx.stack_floor {
        ctx.set_pending("the request exceeded its call-stack limit");
        return crate::FATAL;
    }
    if sp < ctx.stack_limit {
        ctx.set_pending_as(ThrownClass::Recursion, "the call stack is too deep");
        return crate::THROWN;
    }
    // The compare at the site was against the caller's stack pointer, and an
    // embedder may re-arm the bounds mid-request; neither is a reason to stop
    // a frame that is in fact within them.
    crate::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_deadline_flag_starts_clear_and_is_set_through_a_shared_borrow() {
        // The `&self` writer is the point: the thread running the request
        // holds the `&mut`, so a timer that needed one could never fire.
        let ctx = Ctx::buffered();
        assert!(!ctx.deadline_expired());

        let timer: &Ctx = &ctx;
        timer.expire_deadline();
        assert!(ctx.deadline_expired());
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
            let status = unsafe { nvs_safepoint(&raw mut ctx) };
            assert_eq!(status, crate::FATAL);
            assert_eq!(ctx.pending().as_deref(), Some(message));
        }
    }

    #[test]
    fn an_unimplemented_safepoint_request_is_cleared_rather_than_acted_on() {
        let mut ctx = Ctx::buffered();
        ctx.request_safepoint(SafepointFlags::COLLECT | SafepointFlags::DEBUG_BREAK);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let status = unsafe { nvs_safepoint(&raw mut ctx) };
        assert_eq!(status, crate::OK);
        assert!(ctx.safepoint_flags().is_empty());
        assert!(ctx.pending().is_none());
    }
}
