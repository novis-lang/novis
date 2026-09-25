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
        SafepointFlags::from_bits_retain(self.tree.word.load(std::sync::atomic::Ordering::Relaxed))
    }

    /// Joins `parent`'s request tree: from here on this context polls the word
    /// `parent` polls, carrying whatever was already raised in it, and reads the
    /// tree's off-core counters exactly as `parent` reads them.
    ///
    /// The one place the handle and the address are written together, so the
    /// hot slot can never name a word this context holds no share of.
    /// `rule:security/isolate-shares-nothing` is why a child may not be born
    /// clean, and [`Ctx::tree`]'s field doc owns the rest.
    ///
    /// A child of a context that is itself off the root's core is off it too,
    /// which is what `on_root_core` carries down: its bytes are already on that
    /// core's balance and inside the share its ancestor there publishes, so it
    /// reads its own thread and adds nothing.
    pub(super) fn share_safepoint_with(&mut self, parent: &Self) {
        self.tree = std::sync::Arc::clone(&parent.tree);
        self.safepoint = std::ptr::from_ref(&self.tree.word);
        self.on_root_core = parent.on_root_core;
    }

    /// Starts a request tree of its own on this context: a safepoint word and a
    /// deadline nothing else polls, in place of the two it was sharing.
    ///
    /// The inverse of [`Self::share_safepoint_with`], and what a connection
    /// serving one request after another calls in front of each of them. A stop
    /// is the **tree's** — one store reaches every isolate under it — so the
    /// tree a request is stopped in has to be that request's, and on the served
    /// path it is the connection's context that every request's isolate is made
    /// from ([`Self::isolate`]). A request stopped at `rule:errors/on-limit`'s
    /// CPU ceiling leaves [`SafepointFlags::CPU_LIMIT`] standing in that word
    /// with the deadline beside it expired, and the next request down the same
    /// keep-alive connection would be a `FATAL` at its first poll.
    ///
    /// **Replaced rather than cleared, and the deadline is why.** Lowering the
    /// flag where a stopped request ends would have to know that nothing else in
    /// the tree is still running on it, an isolate ending being no more than
    /// that isolate ending; and the deadline half has no such move at all.
    /// [`SafepointView::expire_deadline`] is a one-way store any stranger thread
    /// may make, deliberately with no un-expire for the watchdog that made it to
    /// race. A fresh pair is the one answer that is right for both words.
    ///
    /// Called **before** whatever arms this context's ceilings, for the order
    /// [`Self::isolate`] keeps: [`Self::set_memory_limit`] arms the allocator
    /// with this context's safepoint address beside the number, so an arming
    /// that ran first would leave the allocator raising its flag in the word
    /// this replaces.
    ///
    /// **What it spends:** nothing on the ordinary request, which reuses the
    /// allocation below, and one [`TreeState`] and one word on a request that
    /// follows one whose deferred work is still running. O(in-flight), per
    /// `rule:programs/memory-priority`.
    pub fn reroot(&mut self) {
        debug_assert!(
            self.on_root_core && self.tree_share.is_none(),
            "a context publishing a share into a tree rooted on another core has \
             a share to give back, which re-rooting it would drop on the floor"
        );
        // Reset in place where this context is the tree's only holder, which is
        // the request path's ordinary case, and replaced where it is not —
        // which is exactly the case a reset would be wrong in.
        // `rule:concurrency/after-response-outlives-the-connection`'s deferred
        // work belongs to the tree it was spawned in and is still polling that
        // word, so a reset would hand it back the stop and the budget its own
        // request had already spent. The uniqueness that makes the reuse free is
        // the same fact that makes it safe, which is why one branch answers both
        // questions.
        if let Some(tree) = std::sync::Arc::get_mut(&mut self.tree) {
            *tree = TreeState::default();
        } else {
            self.tree = std::sync::Arc::new(TreeState::default());
            // Beside the handle and never without it, for
            // [`Self::share_safepoint_with`]'s reason: the hot slot may not name
            // a word this context holds no share of.
            self.safepoint = std::ptr::from_ref(&self.tree.word);
        }
        if let Some(deadline) = std::sync::Arc::get_mut(&mut self.deadline) {
            *deadline.get_mut() = 0;
        } else {
            self.deadline = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        }
    }

    /// Joins `tree` as a member running on a core **other** than the one that
    /// tree's root runs on — what a child placed `on: "worker"` is made with,
    /// where a same-core child is made by [`Ctx::isolate`].
    ///
    /// It is [`Self::share_safepoint_with`] across a thread boundary, and it has
    /// to be a second entry point rather than that one because the parent's
    /// `Ctx` is not reachable from here: what crosses is the handle, which is
    /// the whole of what a stranger core may hold
    /// (`rule:concurrency/a-wake-never-moves-a-task`). The word crosses so a
    /// stop reaches the child, and the counters cross so the tree's budget stays
    /// the tree's (`rule:security/isolate-budget-is-the-trees`).
    ///
    /// From here on this context **publishes its own share** into those
    /// counters — [`Ctx::publish_off_core`] is that step and owns its cadence —
    /// and gives it back when it ends. Its zero points are this thread's, taken
    /// by [`Ctx::new`] where this context was made, so what it publishes is what
    /// it and every context beneath it on this core hold.
    pub fn join_tree(&mut self, tree: std::sync::Arc<TreeState>) {
        self.tree = tree;
        self.safepoint = std::ptr::from_ref(&self.tree.word);
        self.on_root_core = false;
        self.tree_share = Some(TreeShare::default());
    }

    /// A handle on the state this request tree shares — what a core other than
    /// this one is given to start a member of the tree with, through
    /// [`Self::join_tree`].
    #[must_use]
    pub fn tree_handle(&self) -> std::sync::Arc<TreeState> {
        std::sync::Arc::clone(&self.tree)
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
        self.tree
            .word
            .fetch_or(flags.bits(), std::sync::atomic::Ordering::Relaxed);
    }

    /// Clears `flags`, leaving every other bit in the word alone.
    ///
    /// Read-modify-write rather than a store, because the word is the tree's:
    /// a store would drop a bit another thread raised between this caller's
    /// load and its write.
    pub(super) fn lower_safepoint(&self, flags: SafepointFlags) {
        self.tree
            .word
            .fetch_and(!flags.bits(), std::sync::atomic::Ordering::Relaxed);
    }

    /// Runs [`crate::object::collect`] over this context's own live list if
    /// [`SafepointFlags::COLLECT`] is standing, and answers whether it
    /// reclaimed anything.
    ///
    /// **The one door into the in-flight collector**, and the whole of what
    /// makes it a collector that runs only near the memory ceiling: the flag
    /// is raised by [`crate::budget`]'s threshold, at the allocation that
    /// crossed the ceiling and nowhere else, so a request inside its ceiling
    /// reaches this and finds nothing to do — and the two callers that ask at
    /// all ([`crate::nvs_safepoint`] and [`crate::run_helper`]) ask only where
    /// they were about to report the breach.
    ///
    /// Lowered before the walk rather than after it, so that the allocations
    /// the walk itself makes can raise the flag again for the crossing they
    /// are, rather than being answered by the walk that provoked them.
    ///
    /// The list is this context's alone. A request tree shares the word this
    /// reads, so a crossing anywhere under it brings *whichever* isolate is
    /// running to a poll — and each then collects what it owns, which is the
    /// same division of the budget `rule:security/isolate-shares-nothing`
    /// already gives it.
    pub(crate) fn collect_if_asked(&mut self) -> bool {
        if !self.safepoint_flags().contains(SafepointFlags::COLLECT) {
            return false;
        }
        self.lower_safepoint(SafepointFlags::COLLECT);
        // `rule:observability/gc-pause-is-its-own-event`'s event, opened here
        // and not in the poll above it: this side of the flag test is the
        // collector's run and the other side is the hot path. The clock is read
        // only where something records it — [`Ctx::open_gc_pause`].
        let started = self.open_gc_pause();
        let freed = crate::object::collect(&self.live);
        if let Some(started) = started {
            self.close_gc_pause(started, freed);
        }
        freed > 0
    }

    /// A handle on the two words this request tree can be stopped through, for
    /// a thread that does not own the request.
    ///
    /// The word this hands out is the **tree root's**, because
    /// `rule:security/isolate-shares-nothing` gives a tree one ceiling to
    /// divide and so one word to be stopped by: a store through the handle
    /// reaches every isolate and task under this context, whether it was
    /// spawned before that store or after it. [`Ctx::tree`]'s field doc owns the
    /// rest of that argument.
    ///
    /// This is the shape `nvs-host`'s watchdog already reads a core's earliest
    /// deadline through, for the same reason: the reader is a stranger to the
    /// request, holds no reference into it, and may outlive it.
    ///
    /// It carries [`Ctx::deadline`] as well as the safepoint word, because the
    /// two words are polled in different places and a stop has to reach both:
    /// compiled code reads the flags between calls, and
    /// `rule:http-server/time-is-bounded-inside-a-helper`'s
    /// [`crate::bounded_loop`] reads the deadline from inside a single call
    /// that is still running. Both are the tree's, so one handle is one clone
    /// of each and never a second handle to be kept in step with this one.
    #[must_use]
    pub fn safepoint_view(&self) -> SafepointView {
        SafepointView {
            tree: std::sync::Arc::clone(&self.tree),
            deadline: std::sync::Arc::clone(&self.deadline),
        }
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

/// The two words one request tree can be stopped through, in a form a thread
/// that is not running the request may write.
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
pub struct SafepointView {
    tree: std::sync::Arc<TreeState>,
    deadline: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl SafepointView {
    /// Asks the next poll in the request tree to act — the same store
    /// [`Ctx::request_safepoint`] makes for the thread that owns the request.
    pub fn request(&self, flags: SafepointFlags) {
        self.tree
            .word
            .fetch_or(flags.bits(), std::sync::atomic::Ordering::Relaxed);
    }

    /// The flags standing in the word, which is what the next poll reads.
    #[must_use]
    pub fn flags(&self) -> SafepointFlags {
        SafepointFlags::from_bits_retain(self.tree.word.load(std::sync::atomic::Ordering::Relaxed))
    }

    /// Expires the request tree's deadline — the same store
    /// [`Ctx::expire_deadline`] makes for the thread that owns the request, and
    /// the poll [`crate::bounded_loop`] makes from inside a member whose
    /// runtime scales with its input.
    ///
    /// A holder that raises [`SafepointFlags::CPU_LIMIT`] raises this beside
    /// it, because a request part-way through one long call reaches no other
    /// poll: the flag stops it at its next back edge, and this stops the call
    /// it is already inside. Which of the two ceilings it was is the flags'
    /// answer, and [`crate::bounded_loop`] reads it there.
    pub fn expire_deadline(&self) {
        self.deadline.store(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Whether that deadline stands as passed — [`Ctx::deadline_expired`]'s
    /// read, from a thread that owns none of the request.
    #[must_use]
    pub fn deadline_expired(&self) -> bool {
        self.deadline.load(std::sync::atomic::Ordering::Relaxed) != 0
    }
}

/// The state a request **tree** shares: the word every context in it is stopped
/// through, and the counters a context running off its root's core charges into.
///
/// One allocation per tree, held by every context in it through [`Ctx::tree`]
/// and by a stranger thread through [`SafepointView`]. The two live together
/// because they are the same decision made twice — a tree is stopped as one and
/// budgeted as one (`rule:security/isolate-shares-nothing`), and neither answer
/// can be a copy a child carries, since the root has no registry of live
/// descendants to correct afterwards. Sharing one allocation is also what lets a
/// child placed on another core be started from one handle
/// (`rule:concurrency/on-worker-runs-the-child-on-another-core`).
///
/// **What it spends:** one allocation per request tree — which is what the
/// safepoint word alone already cost — and two words more inside it, which stay
/// zero and unwritten for a tree that places nothing off its core.
#[derive(Debug, Default)]
pub struct TreeState {
    /// The safepoint word itself, named by [`Ctx::safepoint`] and polled by
    /// compiled code through that address.
    pub(super) word: std::sync::atomic::AtomicU64,
    /// What this tree holds, and has written, on cores other than its root's —
    /// [`crate::budget::OffCore`] owns what the pair means and what it costs.
    pub(super) off_core: crate::budget::OffCore,
}

/// What a context off its tree's root core has already published into
/// [`TreeState::off_core`], so that each poll charges the difference rather than
/// the whole of its share again.
///
/// `Cell`s because publishing is a read of counters this context does not own
/// and a store into a number it shares — neither needs a `&mut Ctx`, and the
/// poll that publishes is reached through `&self`
/// ([`Ctx::memory_breach`](Ctx::memory_breach)).
#[derive(Debug, Default)]
pub(super) struct TreeShare {
    /// The live balance last charged, given back whole when this context ends.
    pub(super) memory: std::cell::Cell<isize>,
    /// The written count last charged, which nothing ever gives back.
    pub(super) output: std::cell::Cell<usize>,
}

/// The safepoint poll's slow path — reached only when the word compiled code
/// loaded was non-zero.
///
/// Returns [`crate::FATAL`] for a request that must stop, and [`crate::OK`]
/// otherwise. A resource-limit stop is deliberately not a `THROWN`:
/// `rule:errors/escalation-ladder` makes it not
/// a `Throwable` at the type level, so no Novis `catch` can see it.
///
/// Every flag but one acts: `DEBUG_BREAK` is cleared and ignored, which is the
/// crate docs' known gap `nvs-runtime/nvs-safepoint-clears-debug-break-and` and waits on `nvs dap`.
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
                  unwind out of the JIT frame above — the two things reached \
                  from here that are not a load and a compare are \
                  `rule:errors/on-limit`'s handler and the collection a \
                  crossing of its memory ceiling asks for, and every helper \
                  call either of them makes is contained by `run_helper`"
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
        // The clock is `nvs_host::watchdog`'s: it samples every request a
        // thread has published against [`Ctx::cpu_limit`] once per interval and
        // raises this flag through a [`SafepointView`] ([`Ctx::cpu_limit`]'s
        // field doc owns why the sampling is the host's). A `nvs run` publishes
        // the one request it is, so a program that reaches no member at all is
        // stopped here; a served core publishes nothing yet, and on that path
        // the flag still arrives only from a caller that had already decided
        // the request was over.
        //
        // One thing that sampler is not told, and it is not an edit here: the
        // ceiling a publication carries is the one that stood before the
        // handler below widened it, so a handler is bounded by the next sweep
        // rather than by its reserve.
        ctx.run_limit_handler(Limit::CpuTime);
        ctx.set_pending("the request exceeded its CPU-time limit");
        return crate::FATAL;
    }
    // Ahead of the memory branch below, which is the whole point of it: the
    // flag was raised by the allocation that crossed
    // `rule:errors/on-limit`'s ceiling, and reclaiming the cyclic garbage
    // before the counter is read is what turns hitting the ceiling into
    // collecting and carrying on. [`Ctx::collect_if_asked`] owns why this
    // costs a request inside its ceiling one load.
    ctx.collect_if_asked();
    // `rule:errors/on-limit`'s memory limit, asked here as well as at every helper
    // boundary ([`crate::run_helper`]): this poll sits between two Novis
    // statements, which is the one place a limit can stop a program that is
    // allocating without calling anything. What *brings* a program here is
    // [`SafepointFlags::MEMORY_LIMIT`], raised by the allocator at the crossing
    // itself — but the question asked is the counter and not the flag, so a
    // crossing already given back is lowered at the foot of this function
    // rather than reported. `crate::budget`'s module doc owns which allocations
    // the threshold reaches and which it does not.
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
    // Reached only by a request that is under every ceiling it has, so the
    // memory bit is lowered with the two housekeeping ones: a request whose
    // allocation crossed the ceiling and whose release brought it back again
    // would otherwise take this slow path at every back edge it has left, for a
    // breach the branch above has already declined to report. The collect bit
    // is here for the same reading of the same fact — any ask still standing
    // was raised by the walk's own allocations, above a ceiling this request
    // is no longer over.
    ctx.lower_safepoint(
        SafepointFlags::COLLECT | SafepointFlags::DEBUG_BREAK | SafepointFlags::MEMORY_LIMIT,
    );
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
                std::sync::Arc::ptr_eq(&root.tree, &ctx.tree),
                "{kind} holds a word of its own"
            );
            assert!(
                std::ptr::eq(root.safepoint, ctx.safepoint),
                "{kind} polls a word of its own"
            );
        }
    }

    /// The second word a stranger stops a request through. A flag is polled
    /// between calls; the deadline is polled from inside one call that is still
    /// running, so a handle that carried only the first could not stop a
    /// request already inside a long member.
    #[test]
    fn a_view_expires_the_deadline_of_the_whole_tree() {
        let root = Ctx::buffered();
        let born_clean = root.isolate(OutputSink::Buffer(Vec::new()));
        let view = root.safepoint_view();
        assert!(!view.deadline_expired());
        assert!(!root.deadline_expired());

        view.expire_deadline();
        assert!(view.deadline_expired());
        assert!(
            root.deadline_expired(),
            "the request the handle names kept running"
        );
        assert!(
            born_clean.deadline_expired(),
            "an isolate already under the tree kept running"
        );
        let born_stopped = root.isolate(OutputSink::Buffer(Vec::new()));
        assert!(
            born_stopped.deadline_expired(),
            "an isolate built after the stop was born clean"
        );
    }

    /// The connection's half of that: both words belong to the request that
    /// spent them, so the request after it polls neither.
    #[test]
    fn a_rerooted_context_keeps_neither_word_the_request_before_it_was_stopped_in() {
        let mut connection = Ctx::buffered();
        let stopped = connection.isolate(OutputSink::Buffer(Vec::new()));
        // What `nvs_host::watchdog` does to a request past its CPU ceiling,
        // through the handle the door published for it: both words, because
        // both are polled.
        let view = stopped.safepoint_view();
        view.request(SafepointFlags::CPU_LIMIT);
        view.expire_deadline();
        assert!(
            connection
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT)
        );
        assert!(connection.deadline_expired());

        drop(view);
        drop(stopped);
        connection.reroot();
        assert!(
            connection.safepoint_flags().is_empty(),
            "the connection went on polling the word its last request was stopped in"
        );
        assert!(
            !connection.deadline_expired(),
            "the connection went on polling the deadline its last request expired"
        );
        let next = connection.isolate(OutputSink::Buffer(Vec::new()));
        assert!(
            next.safepoint_flags().is_empty() && !next.deadline_expired(),
            "the request after a stopped one was born stopped"
        );
    }

    /// The other direction, and the one the reuse inside
    /// [`Ctx::reroot`] has to answer: work still running in the tree the
    /// connection left keeps the stop that was raised on it.
    #[test]
    fn a_reroot_leaves_the_tree_behind_it_stopped() {
        let mut connection = Ctx::buffered();
        let deferred = connection.isolate(OutputSink::Buffer(Vec::new()));
        let view = deferred.safepoint_view();
        view.request(SafepointFlags::CPU_LIMIT);
        view.expire_deadline();

        connection.reroot();
        assert!(
            deferred
                .safepoint_flags()
                .contains(SafepointFlags::CPU_LIMIT),
            "deferred work outliving its response was un-stopped by the next request's re-root"
        );
        assert!(
            deferred.deadline_expired(),
            "deferred work outliving its response had its deadline given back"
        );
        assert!(
            view.flags().contains(SafepointFlags::CPU_LIMIT) && view.deadline_expired(),
            "a handle on the tree that was left behind now names the tree that replaced it"
        );
        assert!(connection.safepoint_flags().is_empty() && !connection.deadline_expired());
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
                std::ptr::eq(ctx.safepoint, std::ptr::from_ref(&ctx.tree.word)),
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
    fn a_debug_break_request_is_cleared_rather_than_acted_on() {
        // The one flag the poll still drops — the crate docs' known gap `nvs-runtime/nvs-safepoint-clears-debug-break-and`,
        // waiting on `nvs dap`. Its neighbour `COLLECT` is answered now, and
        // `crate::object`'s tests are where that is shown.
        let mut ctx = Ctx::buffered();
        ctx.request_safepoint(SafepointFlags::DEBUG_BREAK);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let status = unsafe { nvs_safepoint(&raw mut ctx) };
        assert_eq!(status, crate::OK);
        assert!(ctx.safepoint_flags().is_empty());
        assert!(ctx.pending().is_none());
    }

    #[test]
    fn a_collect_request_is_answered_and_lowered_by_the_poll() {
        // The flag's own half of the collector, with nothing on the list to
        // find: the poll runs the walk, lowers the ask and carries on, so a
        // request brought here by a crossing that a release had already given
        // back is not left polling at every back edge it has left.
        let mut ctx = Ctx::buffered();
        ctx.request_safepoint(SafepointFlags::COLLECT);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let status = unsafe { nvs_safepoint(&raw mut ctx) };
        assert_eq!(status, crate::OK);
        assert!(ctx.safepoint_flags().is_empty());
        assert!(ctx.pending().is_none());
    }
}
