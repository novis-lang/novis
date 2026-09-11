//! The words compiled code polls, and the two helpers that poll them.
//!
//! The safepoint flags, the call-stack bounds and the deadline are the hot
//! words the module docs describe as part of the ABI; this file is the accessor
//! side of them, plus [`nvs_safepoint`] and [`nvs_stack_check`] — the slow paths
//! `nvs-codegen` branches to at every function entry and loop back edge.
//!
//! The two are one file because they are one decision made twice.
//! `rule:errors/on-limit`'s stack pair and
//! `rule:http-server/time-is-bounded-inside-a-helper`
//! 's deadline both put a *load and a compare* on the request path and push
//! everything else behind it, and reading the accessor beside the helper that
//! acts on it is what keeps the two ends of that compare in agreement.

use super::*;

impl Ctx {
    /// Arms `rule:errors/on-limit`'s two stack addresses from a base address and a ceiling: the hard
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

    /// The pending safepoint requests — one relaxed load of the word this
    /// request tree shares, which is where every reader on this side goes.
    /// Only compiled code follows [`Ctx`]'s raw address of it.
    #[must_use]
    pub fn safepoint_flags(&self) -> SafepointFlags {
        SafepointFlags::from_bits_retain(
            self.safepoint_word
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Joins `parent`'s request tree: from here on this context polls the word
    /// `parent` polls, carrying whatever was already raised in it.
    ///
    /// The one place the handle and the address are written together, so the
    /// hot slot can never name a word this context holds no share of.
    /// `rule:security/isolate-shares-nothing` is why a child may not be born
    /// clean, and [`Ctx::safepoint_word`]'s field doc owns the rest.
    pub(super) fn share_safepoint_with(&mut self, parent: &Self) {
        self.safepoint_word = std::sync::Arc::clone(&parent.safepoint_word);
        self.safepoint = std::sync::Arc::as_ptr(&self.safepoint_word);
    }

    /// Whether this request's deadline has passed —
    /// `rule:http-server/time-is-bounded-inside-a-helper`
    /// 's poll, and the module docs' *The request's deadline* section owns
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

    /// Asks the next safepoint poll to act — this request's and every other
    /// poll in its tree's, which read the one word.
    ///
    /// Takes `&self` rather than `&mut self` for [`Self::expire_deadline`]'s
    /// reason, and it is the reason the word sits outside [`Ctx`] at all: the
    /// callers this exists to serve are threads that do not own the request,
    /// and the thread that does holds the `&mut` for as long as it is inside a
    /// helper body.
    pub fn request_safepoint(&self, flags: SafepointFlags) {
        self.safepoint_word
            .fetch_or(flags.bits(), std::sync::atomic::Ordering::Relaxed);
    }

    /// Clears `flags`, leaving every other bit in the word alone.
    ///
    /// Read-modify-write rather than a store, because the word is the tree's:
    /// a store would drop a bit another thread raised between this caller's
    /// load and its write.
    pub(super) fn lower_safepoint(&self, flags: SafepointFlags) {
        self.safepoint_word
            .fetch_and(!flags.bits(), std::sync::atomic::Ordering::Relaxed);
    }

    /// A handle on this request tree's safepoint word, for a thread that does
    /// not own the request.
    ///
    /// The word this hands out is the **tree root's**, because
    /// `rule:security/isolate-shares-nothing` gives a tree one ceiling to
    /// divide and so one word to be stopped by: a store through the handle
    /// reaches every isolate and task under this context, whether it was
    /// spawned before that store or after it. [`Ctx::safepoint_word`]'s field
    /// doc owns the rest of that argument.
    ///
    /// This is the shape `nvs-host`'s watchdog already reads a core's earliest
    /// deadline through, for the same reason: the reader is a stranger to the
    /// request, holds no reference into it, and may outlive it.
    #[must_use]
    pub fn safepoint_view(&self) -> SafepointView {
        SafepointView(std::sync::Arc::clone(&self.safepoint_word))
    }

    /// Whether this request has been cancelled — the flag [`Ctx::cancel`] sets.
    ///
    /// What it distinguishes is a context that *failed* from one that was
    /// stopped: both carry a pending message afterwards, and only one of them
    /// is a `Throwable` anybody may see. `nvs_host::group` is the caller, for
    /// exactly that question about a child.
    #[must_use]
    pub fn cancelled(&self) -> bool {
        self.cancelled
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
    /// `rule:concurrency/cancellation-runs-no-user-code`
    /// 's teardown: no `catch`, no cleanup, no user code at all.
    ///
    /// That is already exactly what this context's cancellation flag means, so
    /// this raises it and asks [`nvs_safepoint`] for the answer rather than
    /// inventing a second one — the status and its message keep one home, and
    /// what comes back is what the poll compiled code was going to make anyway
    /// would have said, only without the statements in between.
    pub fn cancel(&mut self) -> crate::Fault {
        self.cancelled = true;
        #[expect(
            unsafe_code,
            reason = "the pointer is a reborrow of this `&mut self`, which is \
                      live for the whole call"
        )]
        let status = unsafe { nvs_safepoint(&raw mut *self) };
        crate::Fault::Pending(status)
    }

    /// The active `rule:testing/debug-probes`
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

/// One request tree's safepoint word, in a form a thread that is not running
/// the request may write.
///
/// Minted by [`Ctx::safepoint_view`], and `Send` where [`Ctx`] is not: it
/// carries the word and nothing else — no reference to a context, a value, a
/// stack or a scheduler — so raising a flag through it neither blocks the
/// request nor can be blocked by it. That is the whole point. The thread that
/// owns the request holds the `&mut Ctx` for as long as it is inside a helper
/// body, which is exactly the interval a stop has to be able to reach it in.
///
/// A view outliving its request is inert rather than wrong: the word stays
/// alive behind the handle, and a store into it reaches a poll that will never
/// run again. A holder that means to stop a *live* request therefore drops the
/// view when that request ends, rather than relying on the store to be refused.
#[derive(Clone, Debug)]
pub struct SafepointView(std::sync::Arc<std::sync::atomic::AtomicU64>);

impl SafepointView {
    /// Asks the next poll in the request tree to act — the same store
    /// [`Ctx::request_safepoint`] makes for the thread that owns the request.
    pub fn request(&self, flags: SafepointFlags) {
        self.0
            .fetch_or(flags.bits(), std::sync::atomic::Ordering::Relaxed);
    }

    /// The flags standing in the word, which is what the next poll reads.
    #[must_use]
    pub fn flags(&self) -> SafepointFlags {
        SafepointFlags::from_bits_retain(self.0.load(std::sync::atomic::Ordering::Relaxed))
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
/// Not every flag acts; see the crate docs' known gap 5.
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

    if ctx.safepoint_flags().contains(SafepointFlags::CPU_LIMIT) {
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
    // which allocations that reaches and which it does not.
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
    if ctx.cancelled {
        ctx.set_pending("the request was cancelled");
        return crate::FATAL;
    }
    if ctx.safepoint_flags().contains(SafepointFlags::SHUTDOWN) {
        // The one branch here that does not stop the request, and the reason
        // `Core\Signal`'s handler is Novis code rather than a signal handler:
        // the delivery raised this bit and returned, and *this* frame — between
        // two Novis statements, on the request's own stack, with the whole
        // budget and every `Core` member reachable — is where the program's
        // code runs. `Ctx::run_shutdown_handler` owns the once-only rule.
        //
        // Below the cancel branch above, deliberately:
        // `rule:concurrency/cancellation-runs-no-user-code` runs none of it for
        // a request already being torn down, and a drain is the case where
        // there is still a request to hand back to.
        //
        // Lowered before the call, so a safepoint the handler's own back edges
        // reach does not find the request still asked to shut down.
        ctx.lower_safepoint(SafepointFlags::SHUTDOWN);
        ctx.run_shutdown_handler();
    }
    ctx.lower_safepoint(SafepointFlags::COLLECT | SafepointFlags::DEBUG_BREAK);
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
    fn a_child_polls_the_same_word_its_root_does() {
        // `rule:security/isolate-shares-nothing` gives the tree one ceiling to
        // divide, so it gives it one word to be stopped by, whichever of the
        // two constructors made the context.
        let root = Ctx::buffered();
        let isolate = root.isolate(OutputSink::Buffer(Vec::new()));
        #[expect(
            unsafe_code,
            reason = "`root` is declared first and therefore outlives the child, \
                      which is the obligation `Ctx::child` carries"
        )]
        let child = unsafe { root.child() };

        for (kind, ctx) in [("an isolate", &isolate), ("a task child", &child)] {
            assert!(
                std::sync::Arc::ptr_eq(&root.safepoint_word, &ctx.safepoint_word),
                "{kind} holds a word of its own"
            );
            assert!(
                std::ptr::eq(root.safepoint, ctx.safepoint),
                "{kind} polls a word of its own"
            );
        }
    }

    #[test]
    fn a_child_built_after_the_flag_was_set_is_not_born_clean() {
        // The direction a copied flag cannot satisfy: whatever raises a stop
        // holds the root and has no registry of live children to walk, so a
        // child built a microsecond after it has to be born stopped.
        let root = Ctx::buffered();
        root.request_safepoint(SafepointFlags::CPU_LIMIT);
        let born_stopped = root.isolate(OutputSink::Buffer(Vec::new()));
        assert!(
            born_stopped
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT)
        );

        // And the other direction, which is the one a copy satisfies only by
        // accident: a child that started clean is stopped by a flag raised
        // after it existed.
        let born_clean = root.isolate(OutputSink::Buffer(Vec::new()));
        root.lower_safepoint(SafepointFlags::CPU_LIMIT);
        assert!(born_clean.safepoint_flags().is_empty());
        root.request_safepoint(SafepointFlags::CPU_LIMIT);
        assert!(
            born_clean
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT)
        );
    }

    #[test]
    fn every_rust_side_read_goes_through_the_handle() {
        // The address in the hot slot and the handle beside it name one word in
        // every context this crate builds, so the two sides cannot read
        // different answers — which is what lets every reader here take the
        // handle and leaves the raw pointer to compiled code alone.
        let root = Ctx::buffered();
        let isolate = root.isolate(OutputSink::Buffer(Vec::new()));
        #[expect(
            unsafe_code,
            reason = "`root` is declared first and therefore outlives the child, \
                      which is the obligation `Ctx::child` carries"
        )]
        let child = unsafe { root.child() };

        for (kind, ctx) in [
            ("a request", &root),
            ("an isolate", &isolate),
            ("a task child", &child),
        ] {
            assert!(
                std::ptr::eq(ctx.safepoint, std::sync::Arc::as_ptr(&ctx.safepoint_word)),
                "{kind}'s hot slot does not name the word beside it"
            );
        }

        root.request_safepoint(SafepointFlags::COLLECT);
        #[expect(
            unsafe_code,
            reason = "the read compiled code makes, made here: the address names \
                      a word this frame holds a share of for the whole read"
        )]
        let raw = unsafe { &*root.safepoint }.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            SafepointFlags::from_bits_retain(raw),
            root.safepoint_flags()
        );
    }

    #[test]
    fn a_limit_or_cancel_safepoint_is_fatal_and_uncatchable() {
        let mut ctx = Ctx::buffered();
        ctx.request_safepoint(SafepointFlags::CPU_LIMIT);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let status = unsafe { nvs_safepoint(&raw mut ctx) };
        assert_eq!(status, crate::FATAL);
        assert_eq!(
            ctx.pending().as_deref(),
            Some("the request exceeded its CPU-time limit")
        );

        // The cancellation half, which is this context's own and not the tree's
        // — and which asks the same slow path for its answer.
        let mut ctx = Ctx::buffered();
        let crate::Fault::Pending(status) = ctx.cancel() else {
            panic!("a cancellation is a pending stop");
        };
        assert_eq!(status, crate::FATAL);
        assert_eq!(ctx.pending().as_deref(), Some("the request was cancelled"));
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
