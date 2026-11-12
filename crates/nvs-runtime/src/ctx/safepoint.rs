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

    /// Sets the armed pair to `bounds`, a pair [`Ctx::stack_bounds`] returned.
    ///
    /// A native helper that runs compiled code on a second stack arms the
    /// context for that stack with [`Ctx::arm_stack_limit`], and gives the
    /// task's pair back with this on every way out.
    pub fn restore_stack_bounds(&mut self, bounds: (usize, usize)) {
        (self.stack_limit, self.stack_floor) = bounds;
    }

    /// Whether native code may take one more recursive step: an `Err` with
    /// the [`crate::FATAL`] [`nvs_stack_check`] gives below the floor, once the
    /// stack pointer is within [`NATIVE_STEP_ROOM`] of the hard floor.
    ///
    /// A native recursion — a walk over a value whose depth the request picks —
    /// runs no compiled prologue on its way down, so nothing else compares its
    /// depth with `rule:errors/on-limit`'s pair. Each such walk calls this
    /// before it goes one level deeper. It is a fatal and not a catchable
    /// `Recursion`, because a native walk has no catch point of its own to
    /// unwind to, and the room under it is the walk's own frame and the fault.
    ///
    /// What it costs: one register read, one load and one compare per step.
    ///
    /// # Errors
    ///
    /// The fatal above, when the next step could reach the floor.
    pub fn native_stack_step(&self) -> Result<(), crate::Fault> {
        if stack_pointer() < self.stack_floor.saturating_add(NATIVE_STEP_ROOM) {
            return Err(crate::Fault::fatal(
                "the request exceeded its call-stack limit",
            ));
        }
        Ok(())
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
        self.tree_root = false;
    }

    /// Writes this context's CPU ceiling into the state its tree shares, where
    /// this context is the tree's root, and nothing where it is not.
    ///
    /// Owed by everything that moves [`Self::cpu_limit`]:
    /// [`Self::refresh_limits`], [`Self::set_cpu_limit`], and
    /// [`Self::run_limit_handler`] on entry and on exit. [`Self::reroot`] owes
    /// it too, because it starts a tree that carries none.
    /// [`Self::tree_root`]'s field doc owns why a child writes nothing.
    ///
    /// A ceiling that goes down, or a tree's first one, is handed to the
    /// installed [`CpuLowered`] with whether an earlier one stood. The sampler
    /// decides from those two whether to sweep now: its next sweep might come
    /// a whole interval later, which is how far past a ceiling shorter than
    /// that the request would run. A first ceiling here is measured from a
    /// tree that has spent next to nothing, except where a program sets one
    /// while it runs, and [`Self::refresh_limits_at_run_time`] owns that case.
    ///
    /// What it spends: one load of the installed hook per ceiling that goes
    /// down. A request that keeps the default ceiling makes no wake, because
    /// that ceiling is longer than one sweep interval.
    pub(super) fn publish_cpu_limit(&self) {
        if self.tree_root {
            let before = self
                .tree
                .cpu_limit
                .swap(self.cpu_limit, std::sync::atomic::Ordering::Relaxed);
            let lowered = self.cpu_limit != 0 && (before == 0 || self.cpu_limit < before);
            if lowered {
                cpu_lowered(self.cpu_limit, before != 0);
            }
        }
    }

    /// Writes the request's `[limits] wall_time` into the state its tree
    /// shares, and wakes whoever watches it when the number moved.
    ///
    /// Written by the tree's root, which states the configuration the request
    /// starts with, and by the context answering the request, whose
    /// `Core\Config::set` moves it from there. A `spawn script` child is
    /// neither: its own `wall_time` is a narrowing of the child, never of the
    /// request.
    pub(super) fn publish_wall_time(&self) {
        if !self.tree_root && self.inbound.is_none() {
            return;
        }
        let nanos = self.configured_wall_time();
        let before = self
            .tree
            .wall_time
            .swap(nanos, std::sync::atomic::Ordering::Relaxed);
        if before != nanos
            && let Some(wake) = self.tree.wall_time_watch.take()
        {
            wake();
        }
    }

    /// Makes this context — a child of `parent`, already in its tree — the one
    /// that states the tree's CPU ceiling from here on, starting from the
    /// ceiling `parent` stated.
    ///
    /// For the one child that *is* its request: a served request's program
    /// runs in an isolate made from the connection's context
    /// ([`Self::isolate`]), and the tree is the request's, freshly started by
    /// [`Self::reroot`]. So the program's `Core\Config::set("cpu_time", …)` is
    /// the request moving its own ceiling, within `[limits.hard]` exactly as a
    /// root's is, and has to reach what the watchdog charges.
    /// `nvs_host::Isolate::watched_by` is the only caller; a `spawn script`
    /// child never is one, so `rule:security/isolate-budget-is-the-trees`'s
    /// child still moves its own number and never the tree's.
    ///
    /// Both halves of the slice cross, because the reserve is carved out of the
    /// ceiling ([`Self::refresh_limits`]) and a child stopped with none would
    /// have nothing to report the breach with.
    ///
    /// A connection's context is not server work itself, so where no
    /// `[limits] cpu_time` is written the parent states no ceiling. The
    /// request's own [`Self::set_inbound`] then gives this root
    /// [`nvs_config::defaults::LIMITS_CPU_TIME`] ([`Self::now_serving`]).
    pub fn take_tree_cpu_limit(&mut self, parent: &Self) {
        self.cpu_limit = parent.cpu_limit;
        self.fatal_reserve_time = parent.fatal_reserve_time;
        self.tree_root = true;
        self.publish_cpu_limit();
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
    /// **A new tree every time, never the old one reset in place.** A fresh
    /// [`TreeState`] starts with every field at its default by construction,
    /// and at an address no live tree has. A reset in place would be isolated
    /// only while it reset every field a fresh tree starts with, and it would
    /// keep the address, so anything keyed by [`Self::tree_key`] would carry
    /// over to the next request. Work still running in the tree left behind,
    /// such as `rule:concurrency/after-response-outlives-the-connection`'s,
    /// keeps that tree with its stop and the budget its request spent.
    ///
    /// A connection that goes idle while such work runs is rerooted then
    /// ([`Self::between_requests`]), and again as its next request starts.
    ///
    /// **What it spends:** one [`TreeState`] allocated and the old one freed
    /// per request, or kept until the last work still running in it ends.
    /// The deadline is one word, cleared in place where this context is its
    /// only holder and allocated anew where it is not. O(in-flight), per
    /// `rule:programs/memory-priority`.
    pub fn reroot(&mut self) {
        debug_assert!(
            self.on_root_core && self.tree_share.is_none(),
            "a context publishing a share into a tree rooted on another core has \
             a share to give back, which re-rooting it would drop on the floor"
        );
        // The connection is serving a request again, so what its tree set
        // aside while it waited counts from here (`Self::between_requests`).
        if std::mem::take(&mut self.idles_tree) {
            crate::budget::end_idle(&self.tree);
        }
        // A fresh tree, for the reason the doc above gives. The handle on the
        // old one is dropped at the end of this block, so what that frees is
        // gone before the fresh tree's base is read below.
        {
            let old = std::mem::replace(&mut self.tree, std::sync::Arc::new(TreeState::default()));
            // The running turn is the fresh tree's from here, in the units it
            // was read in, so bytes the old request allocated and the new one
            // frees are given back where they were charged.
            crate::budget::reroot_turn(&old, &self.tree);
            // And the task running it is charged CPU time under the fresh tree,
            // read while `old` is still alive so no later tree can share its
            // address: [`TreeReplaced`] owns why.
            tree_replaced(std::sync::Arc::as_ptr(&old).addr(), self.tree_key());
            // Beside the handle and never without it, for
            // [`Self::share_safepoint_with`]'s reason: the hot slot may not name
            // a word this context holds no share of.
            self.safepoint = std::ptr::from_ref(&self.tree.word);
        }
        crate::budget::count_from_here(&self.tree);
        if let Some(deadline) = std::sync::Arc::get_mut(&mut self.deadline) {
            *deadline.get_mut() = 0;
        } else {
            self.deadline = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        }
        // The tree this context was the root of carried its CPU ceiling beside
        // the flags, and the fresh one carries none until it is written here.
        // The `wall_time` beside it for the same reason.
        self.publish_cpu_limit();
        self.publish_wall_time();
    }

    /// [`Self::reroot`] for the upgraded connection this context starts next,
    /// whose tree states no `[limits] wall_time`.
    ///
    /// `rule:concurrency/a-connection-is-a-root-isolate`: `wall_time` bounds
    /// the request that upgraded, and that request has ended by now. A
    /// connection that kept its tree would end at that request's deadline, so
    /// every event stream under a `wall_time` would close and reconnect over
    /// and over. What ends a connection instead is
    /// `rule:concurrency/connection-bounds-are-finite`'s lifetime, and its
    /// `memory` and `cpu_time` still apply, the CPU ceiling from a fresh count.
    ///
    /// A request this connection frames later reroots again and publishes its
    /// own `wall_time` ([`Self::reroot`], then the configuration it runs under).
    ///
    /// **What it spends:** what [`Self::reroot`] does, once per upgrade.
    pub fn reroot_for_a_connection(&mut self) {
        self.reroot();
        // A fresh tree, so nothing watches its `wall_time` yet and no wake is owed.
        self.tree
            .wall_time
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// Marks this connection's context as between requests: the place its last
    /// request held has gone back, and the next request has not started.
    ///
    /// What the tree holds above the balance its last request started at, which
    /// is `hyper`'s buffers and the head of the next request, is left out of
    /// what this core publishes for admission from the tree's next park on, and
    /// [`Self::reroot`] counts it again. `crate::budget`'s gauge module owns
    /// why. A context whose stack is not its tree's turn is left counted.
    ///
    /// Where nothing but this connection holds the tree, its extension
    /// instances are dropped here (`rule:packaging/a-fresh-instance-per-request`).
    /// The connection holds the tree twice: through this context, and through
    /// the running turn where that turn is this tree's
    /// ([`crate::budget::turn_holds`]). A connection may wait a long time for its
    /// next request, and an instance it kept would hold one of the process's
    /// instance places all that time. Those places are sized for the requests
    /// in flight, so idle connections that kept theirs would make a request
    /// that calls an extension fail.
    ///
    /// Where other work still holds the tree, such as after-response work or a
    /// child placed on another core, the connection leaves the tree to that
    /// work and waits on a fresh one ([`Self::reroot`]). The tree then ends
    /// when its last work ends, and its instances with it, however long the
    /// connection waits. The connection's own turns are charged to the fresh
    /// tree, so they spend none of the CPU time that work has left.
    ///
    /// **What it spends:** nothing per request beyond the tree's three words, a
    /// thread-local read and a reference count read per finished request, and a
    /// thread-local add at each park of an idle tree. A request that leaves
    /// work behind pays one more [`TreeState`], allocated here and freed when
    /// the connection's next request starts.
    pub fn between_requests(&mut self) {
        let turn = crate::budget::turn_holds(&self.tree);
        let connection_holders = 1 + usize::from(turn);
        if std::sync::Arc::strong_count(&self.tree) == connection_holders {
            drop(self.tree.end_extensions());
        } else {
            self.reroot();
        }
        self.idles_tree = crate::budget::begin_idle(&self.tree) || self.idles_tree;
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
        // [`Ctx::new`] made this context's own state before it read
        // `memory_base`, so freeing it here would take its bytes off this
        // context's share, and the member would read less than it holds. The
        // base moves down by what the swap freed, so the share stays the same.
        let before = crate::budget::live_bytes();
        self.tree = tree;
        let freed = before.saturating_sub(crate::budget::live_bytes());
        self.memory_base = self.memory_base.saturating_sub(freed);
        self.safepoint = std::ptr::from_ref(&self.tree.word);
        self.on_root_core = false;
        self.tree_root = false;
        self.tree_share = Some(TreeShare::default());
    }

    /// A handle on the state this request tree shares — what a core other than
    /// this one is given to start a member of the tree with, through
    /// [`Self::join_tree`].
    #[must_use]
    pub fn tree_handle(&self) -> std::sync::Arc<TreeState> {
        std::sync::Arc::clone(&self.tree)
    }

    /// An identity for this context's request tree: the same for every context
    /// and [`SafepointView`] in it, and different from that of every other tree
    /// alive at the same time.
    ///
    /// The address of the state the tree shares, so reading it holds nothing
    /// and costs one load. A tree that has ended may lend its address to a
    /// later one, so a caller keys by it only trees it still holds a handle on,
    /// or that a context it is running still holds.
    #[must_use]
    pub fn tree_key(&self) -> usize {
        std::sync::Arc::as_ptr(&self.tree).addr()
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

    /// A handle on the word [`Self::deadline_expired`] reads, for a poll that
    /// is called with no context: a third-party engine's callback, such as the
    /// backtracking regex tier's (`nvs_stdlib::regex`). The word is the tree's,
    /// so it is nonzero once the deadline has passed. Cost: one reference count.
    #[must_use]
    pub fn deadline_word(&self) -> std::sync::Arc<std::sync::atomic::AtomicU64> {
        std::sync::Arc::clone(&self.deadline)
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
        #[cfg(test)]
        assert!(
            !tests::PANIC_IN_COLLECT.replace(false),
            "internal error: injected collection panic"
        );
        // `rule:observability/gc-pause-is-its-own-event`'s event, opened here
        // and not in the poll above it: this side of the flag test is the
        // collector's run and the other side is the hot path. The clock is read
        // only where something records it — [`Ctx::open_gc_pause`].
        let started = self.open_gc_pause();
        self.count_gc_cycle();
        let room = self
            .overshoot_bound()
            .map(|bound| bound.saturating_sub(self.memory_held(crate::budget::live_bytes())));
        let freed = crate::object::collect(&self.live, room);
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
    ///
    /// A wait the park funnel ended at the wall deadline raised
    /// [`SafepointFlags::WALL_TIME`] first. That is a stop and not a
    /// cancellation, so this records none: the poll runs the
    /// `Core\Fatal::onLimit` handler and ends the request with the wall-time
    /// `FATAL` (`rule:errors/on-limit`), and a group still tells a stopped
    /// child from a failed one. A poll that answered `OK` all the same, a raise
    /// lowered inside a handler's slice, is a cancellation after all.
    pub fn cancel(&mut self) -> crate::Fault {
        if !self.safepoint_flags().contains(SafepointFlags::WALL_TIME) {
            self.cancelled = true;
        }
        let mut status = self.poll_now();
        if status == crate::OK {
            self.cancelled = true;
            status = self.poll_now();
        }
        crate::Fault::Pending(status)
    }

    /// One run of the safepoint poll on this context.
    fn poll_now(&mut self) -> i32 {
        #[expect(
            unsafe_code,
            reason = "the pointer is a reborrow of this `&mut self`, which is \
                      live for the whole call"
        )]
        unsafe {
            nvs_safepoint(&raw mut *self)
        }
    }

    /// The active `rule:testing/debug-probes`
    /// probes.
    #[must_use]
    pub fn debug_flags(&self) -> DebugFlags {
        self.debug
    }

    /// Turns probes on or off for a request that may already be running.
    ///
    /// The host's call, never a program's: every bit it turns on also joins the ceiling
    /// [`Self::start_probes`] reads, so `Core\Debug` may stop and start it again.
    /// [`DebugFlags::SITES`] stays as it was, because only [`crate::probe_every_site`] sets it.
    pub fn set_debug_flags(&mut self, flags: DebugFlags) {
        self.debug = flags | (self.debug & DebugFlags::SITES);
        self.debug_ceiling |= flags;
    }

    /// The probes `Core\Debug` may turn on in this context —
    /// `rule:testing/debug-mode-directive`'s ceiling.
    #[must_use]
    pub fn debug_ceiling(&self) -> DebugFlags {
        self.debug_ceiling
    }

    /// Turns `bits` on when the ceiling has every one of them, which is the only way a program
    /// starts a probe.
    ///
    /// # Errors
    ///
    /// The bits the ceiling does not have, and nothing is turned on.
    pub fn start_probes(&mut self, bits: DebugFlags) -> Result<(), DebugFlags> {
        let missing = bits - self.debug_ceiling;
        if !missing.is_empty() {
            return Err(missing);
        }
        self.debug |= bits;
        Ok(())
    }

    /// Turns `bits` off. Stopping is always allowed, because it only narrows.
    pub fn stop_probes(&mut self, bits: DebugFlags) {
        self.debug -= bits;
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

    /// Whether `other` stops the same request tree as this handle.
    #[must_use]
    pub fn same_tree(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.tree, &other.tree)
    }

    /// The identity [`Ctx::tree_key`] gives every context in this tree.
    #[must_use]
    pub fn tree_key(&self) -> usize {
        std::sync::Arc::as_ptr(&self.tree).addr()
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

    /// The CPU time the request tree may burn, in nanoseconds, or `0` for a
    /// tree under no cap — [`Ctx::cpu_limit`] of the tree's root as it stands
    /// now, which is what a sampler compares the tree's charge with.
    #[must_use]
    pub fn cpu_limit(&self) -> u64 {
        self.tree
            .cpu_limit
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// The request's `[limits] wall_time` in nanoseconds as it stands now, or
    /// `0` for a request with none — [`TreeState::wall_time`].
    #[must_use]
    pub fn wall_time(&self) -> u64 {
        self.tree
            .wall_time
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// The instant the request's `wall_time` runs out, counted from the
    /// request's start, or `None` for a request with none. Read again after
    /// each wake [`Self::watch_wall_time`] files, since `Core\Config::set` moves
    /// it.
    #[must_use]
    pub fn wall_deadline(&self) -> Option<std::time::Instant> {
        self.tree.wall_deadline()
    }

    /// Where the wall-time slice of the limit handler now running ends, or
    /// `None` while no handler runs for a wall-time stop. A clock that stopped
    /// the request raises [`SafepointFlags::WALL_TIME`] again at this instant,
    /// and a raise before it does not stop the handler
    /// ([`TreeState::wall_slice_end`]). The wake [`Self::watch_wall_time`]
    /// files is also called when a slice opens.
    #[must_use]
    pub fn wall_slice_end(&self) -> Option<std::time::Instant> {
        self.tree.wall_slice_ends_at()
    }

    /// Files `wake` to be called once, the next time the request's
    /// `wall_time` moves or a limit handler opens its wall-time slice, in place
    /// of any wake filed before it.
    pub fn watch_wall_time(&self, wake: Box<dyn FnOnce() + Send>) {
        self.tree.wall_time_watch.put(wake);
    }

    /// Takes back the wake [`Self::watch_wall_time`] filed, or `None` once it
    /// has been called.
    pub fn unwatch_wall_time(&self) -> Option<Box<dyn FnOnce() + Send>> {
        self.tree.wall_time_watch.take()
    }

    /// Raises [`SafepointFlags::WALL_TIME`] for the request's `wall_time`,
    /// unless a `Core\Time::sleep` that ended at the deadline has already
    /// stopped the request itself, and returns whether it raised it.
    ///
    /// That sleep runs the limit's handler with the flag lowered
    /// ([`Ctx::wall_time_breach`]), so a raise that landed after the handler
    /// started would stop it at its first poll. The claim and the raise are one
    /// step under [`TreeState::wall_stop`]'s lock, so the handler starts either
    /// after this raise or with none to come, whichever thread reached the
    /// deadline first.
    pub fn stop_at_wall_time(&self) -> bool {
        let mut claimed = self
            .tree
            .wall_stop
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *claimed {
            return false;
        }
        *claimed = true;
        drop(claimed);
        self.request(SafepointFlags::WALL_TIME);
        write_wall_sweep_line(self.tree.wall_deadline());
        true
    }

    /// Whether the request was stopped at its `wall_time`, by the clock's raise
    /// ([`Self::stop_at_wall_time`]) or by a `Core\Time::sleep` that ended at the
    /// deadline ([`Ctx::wall_time_breach`]). The second leaves
    /// [`Self::deadline_expired`] false, so a caller that reports the stop reads this.
    #[must_use]
    pub fn wall_time_stopped(&self) -> bool {
        *self
            .tree
            .wall_stop
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// A handle on the same word that does not keep the tree alive.
    ///
    /// For a holder the tree itself keeps, such as the extension host's budget
    /// in [`TreeState::extensions`]: a strong view there would be a cycle, and
    /// the tree, its instances and its memory would outlive every request.
    #[must_use]
    pub fn downgrade(&self) -> WeakSafepointView {
        WeakSafepointView(std::sync::Arc::downgrade(&self.tree))
    }
}

/// A [`SafepointView`] that does not keep its tree alive, made by
/// [`SafepointView::downgrade`].
#[derive(Clone, Debug)]
pub struct WeakSafepointView(std::sync::Weak<TreeState>);

impl WeakSafepointView {
    /// The flags standing in the word, or `None` once the tree has ended.
    #[must_use]
    pub fn flags(&self) -> Option<SafepointFlags> {
        self.0.upgrade().map(|tree| {
            SafepointFlags::from_bits_retain(tree.word.load(std::sync::atomic::Ordering::Relaxed))
        })
    }

    /// [`SafepointView::request`] on the tree, and whether it is still alive.
    pub fn request(&self, flags: SafepointFlags) -> bool {
        self.0.upgrade().is_some_and(|tree| {
            tree.word
                .fetch_or(flags.bits(), std::sync::atomic::Ordering::Relaxed);
            true
        })
    }

    /// Ends a debug stop of the tree: lowers [`SafepointFlags::DEBUG_BREAK`]
    /// and ends the hold [`Ctx::hold_deadlines`] started. Returns whether the
    /// tree is still alive.
    ///
    /// A debug controller holds this weak handle, because the tree holds the
    /// controller's stop handler, and a strong one would keep both alive.
    pub fn release_debug_stop(&self) -> bool {
        self.0.upgrade().is_some_and(|tree| {
            tree.word.fetch_and(
                !SafepointFlags::DEBUG_BREAK.bits(),
                std::sync::atomic::Ordering::Relaxed,
            );
            tree.release_deadlines();
            true
        })
    }
}

/// The state a request **tree** shares: the word every context in it is stopped
/// through, the CPU ceiling it is charged against, and the counters a context
/// running off its root's core charges into.
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
/// safepoint word alone already cost — and a few words more inside it: the CPU
/// ceiling and the `wall_time`, written when the request's move; the two
/// off-core counters, which stay zero and unwritten for a tree that places
/// nothing off its core; the `wall_time`'s watch, an empty lock for a
/// request whose client stays; an empty cell for the extension host; the
/// count of running child isolates; and the debug state, an empty lock for a
/// tree nobody debugs.
#[derive(Debug, Default)]
pub struct TreeState {
    /// The safepoint word itself, named by [`Ctx::safepoint`] and polled by
    /// compiled code through that address.
    pub(super) word: std::sync::atomic::AtomicU64,
    /// The CPU ceiling this tree is charged against in nanoseconds, or `0` for
    /// a tree under no cap — the root's [`Ctx::cpu_limit`], written by
    /// [`Ctx::publish_cpu_limit`] each time that number moves.
    ///
    /// Here because the thread that charges a tree owns none of it. What
    /// `nvs_host::watchdog` holds is a [`SafepointView`], and a ceiling copied
    /// into its record would be the one in force where the request was
    /// published for as long as the request ran, whatever `Core\Config::set`
    /// had narrowed `limits.cpu_time` to since.
    pub(super) cpu_limit: std::sync::atomic::AtomicU64,
    /// The CPU time the tree had spent when its program last moved
    /// [`Self::cpu_limit`] while it ran, in nanoseconds, or `0` when it has
    /// not. A limit set below that is crossed at the moment it is set, so the
    /// CPU `FATAL` names this too and the overshoot scan measures from it.
    /// Written by [`Ctx::refresh_limits_at_run_time`].
    pub(super) cpu_limit_set_after: std::sync::atomic::AtomicU64,
    /// What this tree holds, and has written, on cores other than its root's —
    /// [`crate::budget::OffCore`] owns what the pair means and what it costs.
    pub(super) off_core: crate::budget::OffCore,
    /// This tree's memory balance on its root core, as its last turn there
    /// left it: [`crate::budget`]'s *per request tree across switches*. A
    /// task stores it as it parks and reads it back as it resumes, so the
    /// tasks of one tree share one count and other trees' turns stay out of
    /// it. Only the root core's thread reads or writes it, so `Relaxed`.
    ///
    /// **What it spends:** one word per tree, and a relaxed store and load per
    /// switch.
    pub(crate) balance: std::sync::atomic::AtomicIsize,
    /// [`Self::balance`] as the request this tree answers started, or as the
    /// connection first waited for one, which is what a connection's bytes
    /// between requests are measured from ([`crate::budget::count_from_here`]).
    pub(crate) base: std::sync::atomic::AtomicIsize,
    /// Whether [`Self::base`] is set, and whether this tree is a connection's
    /// between requests ([`Ctx::between_requests`]), so its core's published
    /// balance leaves out [`Self::idle_held`]: one of `crate::budget`'s
    /// `UNBASED`, `COUNTED` and `IDLE`.
    pub(crate) idling: std::sync::atomic::AtomicU8,
    /// What this tree held above [`Self::base`] at its last park while it was
    /// idle, which its core's published balance leaves out.
    ///
    /// **What the three spend:** three words per tree, and on an idle tree's
    /// park two relaxed loads and a swap.
    pub(crate) idle_held: std::sync::atomic::AtomicIsize,
    /// `[limits] wall_time` of the request this tree answers in nanoseconds,
    /// or `0` for one with none — written by [`Ctx::publish_wall_time`] each
    /// time that number moves.
    ///
    /// Here for `rule:http-server/a-request-outlives-a-client-that-goes-away`:
    /// a request whose client left runs under its `wall_time`, or under
    /// `disconnect_grace` while it has none, and the connection that decides
    /// which owns none of the request. `wall_time` is a `Runtime` key, so the
    /// answer is read again each time the connection wakes.
    pub(super) wall_time: std::sync::atomic::AtomicU64,
    /// Who is told when [`Self::wall_time`] moves: the connection waiting on a
    /// request whose client left, which files a wake before each wait and
    /// takes it back after. Empty for every request whose client stays, so
    /// nothing locks it on the request path.
    pub(super) wall_time_watch: WallTimeWatch,
    /// When the request this tree answers started, which is where its
    /// [`Self::wall_time`] is counted from: a served request's tree is made
    /// fresh by [`Ctx::reroot`] in front of it, and a `nvs run`'s with its
    /// first context.
    pub(super) started: Started,
    /// Whether the stop at [`Self::wall_time`] is claimed, by `nvs run`'s clock
    /// or `nvs_host::watchdog`'s sweep ([`SafepointView::stop_at_wall_time`]),
    /// or by a sleep that ended at the deadline ([`Ctx::wall_time_breach`]).
    /// Locked only past the deadline, by the clocks and that sleep, so nothing
    /// on the request path takes it.
    pub(super) wall_stop: std::sync::Mutex<bool>,
    /// Where the wall-time slice of a running limit handler ends, in
    /// nanoseconds after [`Self::started`], or `0` while no handler runs for a
    /// wall-time stop. [`Ctx::run_limit_handler`] opens it at the handler's
    /// entry and closes it at its exit.
    ///
    /// The slice is counted from the handler's entry, never from the deadline.
    /// A request thread that reaches its poll late, or is preempted inside the
    /// handler, would otherwise lose part of the slice to the host's speed, and
    /// a clock's raise could stop a handler that has run for no time at all. The
    /// poll ignores a raise of [`SafepointFlags::WALL_TIME`] while the slice is
    /// open ([`Ctx::in_wall_slice`]), and `nvs run`'s clock raises it again at
    /// the slice's end ([`SafepointView::wall_slice_end`]).
    ///
    /// **What it spends:** one word per tree, a store at each end of a
    /// wall-time handler, and a clock read at each raise that lands inside it.
    pub(super) wall_slice_end: std::sync::atomic::AtomicU64,
    /// What [`crate::extension`]'s host keeps for this request — the
    /// extension instances its calls made, and the resources they returned —
    /// set at its first call into an extension. It is dropped when the
    /// request's connection waits for its next request and nothing else holds
    /// the tree ([`Ctx::between_requests`]), or else with the tree. The
    /// connection leaves a tree other work still holds, so that tree ends when
    /// the request's after-response work does. Empty for every request
    /// that calls no extension. A call holds its own handle on it, so the lock
    /// is never held while a guest runs.
    pub(super) extensions: std::sync::Mutex<Option<ExtensionHost>>,
    /// How many child isolates of this tree are running now — one per
    /// [`TaskSlot`] alive, which [`Ctx::claim_task`] hands out against
    /// `[limits] max_tasks` and the child's context gives back as it drops
    /// (`rule:security/isolate-budget-is-the-trees`). Shared rather than
    /// counted on the root because a child placed on another core spawns from
    /// there.
    pub(crate) tasks: std::sync::atomic::AtomicU64,
    /// The stop handler every context of this tree runs, and how long the
    /// tree has been stopped (`rule:testing/a-stop-pauses-the-whole-task-tree`).
    /// Locked only at a stop, a pause or a resume, and by a reader of
    /// [`Self::wall_deadline`] of a request with a `wall_time`.
    pub(super) debug: super::frames::TreeDebug,
}

/// One running child isolate, counted in its tree's [`TreeState::tasks`] for as
/// long as this lives.
///
/// [`Ctx::claim_task`] is the only maker, and the child's context holds it
/// ([`Ctx::hold_task`]), so the count falls when that context drops — the
/// isolate's end, however it ended. It is `Send` so that a child placed on
/// another core carries it there inside its [`crate::PlacedIsolate`].
#[derive(Debug)]
pub struct TaskSlot(pub(super) std::sync::Arc<TreeState>);

impl Drop for TaskSlot {
    fn drop(&mut self) {
        self.0
            .tasks
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// What [`TreeState::extensions`] keeps: the extension host's per-request
/// state, which only the host that set it can read back.
pub type ExtensionHost = std::sync::Arc<dyn std::any::Any + Send + Sync>;

impl TreeState {
    /// [`Self::extensions`], made by `make` at the request's first call into
    /// an extension and returned as it stands at every later one.
    ///
    /// # Errors
    ///
    /// What `make` returns, in which case nothing is kept.
    pub fn extensions<E>(
        &self,
        make: impl FnOnce() -> Result<ExtensionHost, E>,
    ) -> Result<ExtensionHost, E> {
        let mut slot = self
            .extensions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(host) = slot.as_ref() {
            return Ok(std::sync::Arc::clone(host));
        }
        let host = make()?;
        *slot = Some(std::sync::Arc::clone(&host));
        Ok(host)
    }

    /// Takes [`Self::extensions`] out, so the caller drops it outside the lock.
    pub(super) fn end_extensions(&self) -> Option<ExtensionHost> {
        self.extensions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

/// [`TreeState::started`]: the instant the tree was made, which is what its
/// `Default` reads.
#[derive(Debug)]
pub(super) struct Started(std::time::Instant);

impl Default for Started {
    fn default() -> Self {
        Self(std::time::Instant::now())
    }
}

impl TreeState {
    /// The instant the request's `[limits] wall_time` runs out, or `None` for a
    /// request with none, and for one a debugger has stopped.
    ///
    /// The time the request spent stopped is added, so a stop never uses up
    /// the request's time (`rule:testing/a-stop-pauses-the-whole-task-tree`).
    pub(super) fn wall_deadline(&self) -> Option<std::time::Instant> {
        match self.wall_time.load(std::sync::atomic::Ordering::Relaxed) {
            0 => None,
            nanos => self
                .started
                .0
                .checked_add(std::time::Duration::from_nanos(nanos))?
                .checked_add(self.debug.held_for()?),
        }
    }

    /// Where the open wall-time slice of a limit handler ends, or `None` while
    /// none is open ([`Self::wall_slice_end`]'s field doc).
    pub(super) fn wall_slice_ends_at(&self) -> Option<std::time::Instant> {
        match self
            .wall_slice_end
            .load(std::sync::atomic::Ordering::Acquire)
        {
            0 => None,
            nanos => self
                .started
                .0
                .checked_add(std::time::Duration::from_nanos(nanos)),
        }
    }

    /// The instant a park of this tree ends at, by `rule:errors/on-limit`: the
    /// open wall-time slice's end while a limit handler runs in one, the wall
    /// deadline otherwise, and `None` for a tree with no `wall_time` or one a
    /// debugger holds. One load for a tree with no `wall_time`.
    pub(crate) fn park_bound(&self) -> Option<std::time::Instant> {
        if self.wall_time.load(std::sync::atomic::Ordering::Relaxed) == 0 {
            return None;
        }
        self.wall_slice_ends_at().or_else(|| self.wall_deadline())
    }

    /// Stops the tree for a park that reached [`Self::park_bound`]: the claim
    /// the watchdog takes at the wall deadline, and the raise of
    /// [`SafepointFlags::WALL_TIME`]. Past a slice's end the claim is already
    /// taken, and the raise alone stops the handler at the poll.
    pub(crate) fn stop_past_park_bound(&self) {
        if self.wall_slice_ends_at().is_none() {
            let first = {
                let mut claimed = self
                    .wall_stop
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                !std::mem::replace(&mut *claimed, true)
            };
            if first {
                write_wall_sweep_line(self.wall_deadline());
            }
        }
        self.word.fetch_or(
            SafepointFlags::WALL_TIME.bits(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// Opens a wall-time slice `reserve` nanoseconds long, starting now, and
    /// wakes whoever waits on the wall deadline, so `nvs run`'s clock reads where
    /// the slice ends. A `reserve` of `0` opens nothing.
    pub(crate) fn open_wall_slice(&self, reserve: u64) {
        if reserve == 0 {
            return;
        }
        let since_start = u64::try_from(self.started.0.elapsed().as_nanos()).unwrap_or(u64::MAX);
        // At least 1, because `0` is the encoding of no open slice.
        let end = since_start.saturating_add(reserve).max(1);
        self.wall_slice_end
            .store(end, std::sync::atomic::Ordering::Release);
        if let Some(wake) = self.wall_time_watch.take() {
            wake();
        }
    }

    /// The wall time the request has used: the time since the tree was made,
    /// less every debug stop that has ended. A stop still in progress is
    /// counted, which only a breach read during a stop would see.
    pub(super) fn wall_used(&self) -> std::time::Duration {
        self.started
            .0
            .elapsed()
            .saturating_sub(self.debug.held_for().unwrap_or_default())
    }

    /// Starts holding the tree's deadlines for a debug stop, and wakes
    /// whoever waits on the wall deadline, so it reads that there is none.
    pub(super) fn hold_deadlines(&self) {
        if self.debug.hold()
            && let Some(wake) = self.wall_time_watch.take()
        {
            wake();
        }
    }

    /// Ends the hold, and wakes whoever waits on the wall deadline, so it
    /// reads the deadline the stop moved.
    fn release_deadlines(&self) {
        if self.debug.release()
            && let Some(wake) = self.wall_time_watch.take()
        {
            wake();
        }
    }
}

/// The one wake [`TreeState::wall_time_watch`] holds, fired at most once.
///
/// A lock rather than an atomic because the wake is a Rust closure, and both sides
/// take it rarely: the connection once per wait of a detached request, and the
/// request once per `Core\Config::set` that moves its `wall_time`.
#[derive(Default)]
pub(super) struct WallTimeWatch(std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>>);

impl WallTimeWatch {
    fn put(&self, wake: Box<dyn FnOnce() + Send>) {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(wake);
    }

    fn take(&self) -> Option<Box<dyn FnOnce() + Send>> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

impl std::fmt::Debug for WallTimeWatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WallTimeWatch")
    }
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
/// `DEBUG_BREAK` runs the tree's debug stop handler, the one an armed stop
/// site runs ([`Ctx::set_debug_stop`]), and runs on when none is set.
///
/// # Safety
///
/// `ctx` must be non-null, aligned, and valid for the duration of the call.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer; the contract cannot be \
              expressed in the signature"
)]
#[deny(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::unreachable,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_safepoint(ctx: *mut Ctx) -> i32 {
    // The whole slow path is contained, because it runs Rust that can panic:
    // the collection, the limit handler's runtime half and the shutdown
    // handler's. An unwind that reached the JIT frame above would abort the
    // process, so a panic here ends this one request with a `FATAL`, the way
    // a panic in a helper body does. The poll's fast path is in compiled code
    // and pays nothing for it.
    #[expect(unsafe_code, reason = "the caller's contract, passed on unchanged")]
    unsafe {
        crate::abi::contain_panic(ctx, safepoint_slow_path)
    }
}

/// [`nvs_safepoint`]'s body, run inside its `catch_unwind`.
fn safepoint_slow_path(ctx: &mut Ctx) -> i32 {
    if ctx.safepoint_flags().contains(SafepointFlags::CPU_LIMIT) && ctx.in_cpu_slice() {
        // A sweep compared the tree with the ceiling from before the handler's
        // entry and raised the flag after the handler lowered it. The tree is
        // under the widened ceiling, so the raise is lowered and the handler
        // goes on. The next sweep compares with the widened ceiling, and raises
        // again only once the handler has spent its slice.
        ctx.lower_safepoint(SafepointFlags::CPU_LIMIT);
    }
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
        // the one request it is, and a served core publishes each request it
        // runs, so a program that reaches no member at all is stopped here. A
        // core the boot could not name a CPU for publishes nothing, and there
        // the flag arrives only from a caller that had already decided the
        // request was over.
        //
        // The handler below publishes its widened ceiling to the tree, so the
        // sampler bounds the handler by its reserve, counted from what the tree
        // had spent at the handler's entry.
        ctx.run_limit_handler(Limit::CpuTime);
        ctx.set_pending(ctx.cpu_time_message());
        return crate::FATAL;
    }
    if ctx.safepoint_flags().contains(SafepointFlags::WALL_TIME) {
        if ctx.in_wall_slice() {
            // A clock raised the flag inside the handler's slice. The slice is
            // counted from the handler's entry, so the raise is lowered and the
            // handler goes on. The next raise past the slice's end stops it:
            // `nvs run`'s clock raises at that instant, and
            // `nvs_host::watchdog`'s sweep raises on every pass.
            ctx.lower_safepoint(SafepointFlags::WALL_TIME);
        } else {
            // The CPU branch's two lines in the same order.
            // `Ctx::run_limit_handler` lowers this flag for the handler's call
            // and opens the handler's wall-time slice.
            ctx.run_limit_handler(Limit::WallTime);
            ctx.set_pending(ctx.wall_time_message());
            return crate::FATAL;
        }
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
    if ctx.safepoint_flags().contains(SafepointFlags::YIELD) {
        // The slice clock raised this: the task has held its core for a slice
        // without waiting, so it gives the core back here and its neighbours
        // run. Below the limit branches, because a request that is stopping
        // has nothing to hand back for, and above the cancel branch, which is
        // where a cancellation delivered during the yield is read.
        //
        // Lowered before the call, so the task's next back edge does not find
        // the slice it has just given up. The yield stands inside a
        // `HelperFrame`, the way a member's park does: the frames above this
        // one are compiled code, and a host that saw no frame would treat the
        // suspended stack as one it may force-unwind
        // (`rule:http-server/time-is-bounded-inside-a-helper`).
        ctx.lower_safepoint(SafepointFlags::YIELD);
        let woken = {
            let _frame = crate::HelperFrame::enter();
            crate::host::with_current(|host| host.yield_slice())
        };
        if woken == Some(crate::host::Woken::Cancelled) {
            // The park funnel ends a yield past the wall bound with the flag
            // raised: that is the wall time's stop, and its branch above runs
            // the handler and gives the `FATAL` (`rule:errors/on-limit`).
            if ctx.safepoint_flags().contains(SafepointFlags::WALL_TIME) {
                return safepoint_slow_path(ctx);
            }
            ctx.cancelled = true;
        }
    }
    if ctx.safepoint_flags().contains(SafepointFlags::DEBUG_BREAK) {
        // A debug controller asked this request to pause, or another task of
        // its tree stopped. The handler is the host's, and it parks the task
        // until the controller resumes it
        // (`rule:testing/a-stop-pauses-the-whole-task-tree`). Below the limit
        // branches for the yield's reason, and above the cancel branch, which
        // reads a cancellation delivered while it was stopped.
        //
        // The flag stays raised while a handler is set, so every task of the
        // tree stops at its next poll, and the controller lowers it at the
        // resume. A tree with no handler lowers it here and runs on.
        if !ctx.has_debug_stop() {
            ctx.lower_safepoint(SafepointFlags::DEBUG_BREAK);
        }
        let status = ctx.run_debug_stop(super::StopCause::Break);
        if status != crate::OK {
            return status;
        }
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
    // memory and output bits are lowered with the two housekeeping ones: a
    // request whose allocation crossed the ceiling and whose release brought it
    // back again would otherwise take this slow path at every back edge it has
    // left, for a breach the branch above has already declined to report. The
    // output bit raised by a child under a ceiling its parent does not share
    // is lowered by the same reading. The collect bit
    // is here for the same reading of the same fact — any ask still standing
    // was raised by the walk's own allocations, above a ceiling this request
    // is no longer over.
    ctx.lower_safepoint(
        SafepointFlags::COLLECT | SafepointFlags::MEMORY_LIMIT | SafepointFlags::OUTPUT_LIMIT,
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
#[deny(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::unreachable,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_stack_check(ctx: *mut Ctx, sp: u64) -> i32 {
    // Contained like `nvs_safepoint`: recording the message allocates, and an
    // unwind out of this frame into the JIT frame above would abort the process.
    #[expect(unsafe_code, reason = "the caller's contract, passed on unchanged")]
    unsafe {
        crate::abi::contain_panic(ctx, |ctx| stack_check_slow_path(ctx, sp))
    }
}

/// [`nvs_stack_check`]'s body, run inside its `catch_unwind`.
fn stack_check_slow_path(ctx: &mut Ctx, sp: u64) -> i32 {
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

/// What a request tree has spent of the CPU, given its [`Ctx::tree_key`], or
/// `None` where nothing charges it. The host's sampler is the one clock that
/// charges a tree (`nvs_host::watchdog`), and `nvs-runtime` cannot read it, so
/// the host installs this with [`install_cpu_spent`].
pub type CpuSpent = fn(usize) -> Option<std::time::Duration>;

/// The process's [`CpuSpent`], or none before a host installs one.
static CPU_SPENT: std::sync::OnceLock<CpuSpent> = std::sync::OnceLock::new();

/// Installs the process's [`CpuSpent`]. The first call wins: there is one
/// sampler per process, and every thread asks it.
pub fn install_cpu_spent(spent: CpuSpent) {
    let _ = CPU_SPENT.set(spent);
}

/// Tells the sampler that charges the calling thread that the thread's tree
/// has a lower CPU ceiling: the CPU time left under it in nanoseconds, and
/// whether an earlier ceiling stood. The sampler sweeps now where its next
/// regular sweep could come too late. The host installs it with
/// [`install_cpu_lowered`], for the same reason as [`CpuSpent`].
pub type CpuLowered = fn(u64, bool);

/// The process's [`CpuLowered`], or none before a host installs one.
static CPU_LOWERED: std::sync::OnceLock<CpuLowered> = std::sync::OnceLock::new();

/// Installs the process's [`CpuLowered`]. The first call wins, as with
/// [`install_cpu_spent`].
pub fn install_cpu_lowered(wake: CpuLowered) {
    let _ = CPU_LOWERED.set(wake);
}

/// Calls the installed [`CpuLowered`] with `left` and `earlier`, or does
/// nothing before a host installs one.
pub(super) fn cpu_lowered(left: u64, earlier: bool) {
    if let Some(wake) = CPU_LOWERED.get() {
        wake(left, earlier);
    }
}

/// Tells the scheduler running the calling thread that [`Ctx::reroot`] replaced
/// the tree whose [`Ctx::tree_key`] is the first argument with the one whose key
/// is the second. The host installs it with [`install_tree_replaced`], for the
/// same reason as [`CpuSpent`].
///
/// A connection's task parks with no context in hand, so the scheduler keeps
/// charging the key the task last stated. Without this, every turn after a
/// reroot is charged to the old tree's address, which is freed, and which the
/// next tree made on the core may be given: the connection's CPU time is then
/// charged to an unrelated request.
///
/// **What it spends:** one load and one call per reroot, which is one per
/// served request.
pub type TreeReplaced = fn(usize, usize);

/// The process's [`TreeReplaced`], or none before a host installs one.
static TREE_REPLACED: std::sync::OnceLock<TreeReplaced> = std::sync::OnceLock::new();

/// Installs the process's [`TreeReplaced`]. The first call wins, as with
/// [`install_cpu_spent`].
pub fn install_tree_replaced(replaced: TreeReplaced) {
    let _ = TREE_REPLACED.set(replaced);
}

/// Calls the installed [`TreeReplaced`] with `old` and `new`, or does nothing
/// before a host installs one.
fn tree_replaced(old: usize, new: usize) {
    if let Some(replaced) = TREE_REPLACED.get() {
        replaced(old, new);
    }
}

/// What `tree` has spent of the CPU, by the installed [`CpuSpent`].
pub(super) fn cpu_spent_by(tree: usize) -> Option<std::time::Duration> {
    CPU_SPENT.get()?(tree)
}

/// The text a time limit's `FATAL` adds after its prefix: `: <used> ms used of
/// a limit of <limit> ms`, the one form `bun nv proofs --overshoot` parses, or
/// ` of <limit> ms` with no reading, or nothing with no limit.
pub(super) fn time_used_text(
    used: Option<std::time::Duration>,
    limit: std::time::Duration,
) -> String {
    match (used, limit.is_zero()) {
        (_, true) => String::new(),
        (Some(used), false) => format!(
            ": {} ms used of a limit of {} ms",
            used.as_millis(),
            limit.as_millis()
        ),
        (None, false) => format!(" of {} ms", limit.as_millis()),
    }
}

/// The sweep line of a wall-time stop that was claimed now: how far past
/// `deadline` the claim came, which counts both the clock's wait and how late it
/// woke. A request with no deadline writes nothing.
pub(super) fn write_wall_sweep_line(deadline: Option<std::time::Instant>) {
    if let Some(at) = deadline {
        let late = std::time::Instant::now().saturating_duration_since(at);
        write_sweep_line("wall", late, std::time::Duration::ZERO, 1);
    }
}

/// The variable `bun nv proofs` sets on every program it runs, so a time stop
/// writes a [`write_sweep_line`] the runner judges the stop by. A process
/// without it writes no such line.
pub const SWEEP_LINES_ENV: &str = "NVS_PROOF_SWEEPS";

/// The CPU time one poll site may be from the next, as a stop is judged: one
/// watchdog sweep interval. `rule:http-server/time-is-bounded-inside-a-helper`
/// sizes a helper's batch so its polls come far more often. A helper whose batch
/// runs longer than this on the machine running it holds its core this long past
/// a stop, and the proofs runner reds that run.
pub const POLL_GAP: std::time::Duration = std::time::Duration::from_millis(50);

/// Writes the line the proofs runner explains a time stop's overshoot by, when
/// [`SWEEP_LINES_ENV`] is set:
/// `nvs-proof-sweep: <kind> late_us=<late> interval_us=<interval> poll_us=<poll> threads=<threads> tick_us=<tick>`.
///
/// `late` is how much later than it should have the stop's clock woke, measured
/// on the clock's own thread. `interval` is the most CPU time a tree's thread can
/// spend between two sweeps that are on time, which is zero for a wall-time stop
/// whose `late` already counts from the deadline. `threads` is how many threads
/// the tree was charged on. `tick` is [`cpu_clock_tick`] for a CPU stop and zero
/// for a wall-time stop. `tools/nv/proofs/run.ts`'s `timeOvershoot` reads it.
///
/// What it spends: one environment read per process, and a line per stop only
/// when the variable is set. A request's `FATAL` does not change.
pub fn write_sweep_line(
    kind: &str,
    late: std::time::Duration,
    interval: std::time::Duration,
    threads: u32,
) {
    use std::io::Write as _;
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*ON.get_or_init(|| std::env::var_os(SWEEP_LINES_ENV).is_some()) {
        return;
    }
    let tick = if kind == "cpu" {
        cpu_clock_tick()
    } else {
        std::time::Duration::ZERO
    };
    // Through the handle, as every other floor write is: `eprintln!` would
    // panic on a closed pipe.
    let _ = writeln!(
        std::io::stderr(),
        "nvs-proof-sweep: {kind} late_us={} interval_us={} poll_us={} threads={threads} tick_us={}",
        late.as_micros(),
        interval.as_micros(),
        POLL_GAP.as_micros(),
        tick.as_micros()
    );
}

/// How far one reading of a thread's CPU clock may trail the CPU time the
/// thread has spent.
///
/// On Windows, `GetThreadTimes` moves only at the system clock interrupt, so a
/// reading is up to one clock interval old: 15.625 ms on a default system.
/// A sweep that reads a thread just short of the limit sees the crossing one
/// interval late, so the proofs runner counts one tick per thread. The Unix
/// thread clocks read to the microsecond or better, and the tick is zero.
///
/// The Windows tick stays because its finer clock, `QueryThreadCycleTime`, is
/// declined in `rule:errors/on-limit`: a read of a running core costs about ten
/// times a `GetThreadTimes` read.
///
/// Outside tests only [`write_sweep_line`] calls it, so only a proofs run reads
/// it. `nvs-host`'s `cpuclock` tests read it to bound the steps of each
/// platform's clock.
#[must_use]
pub fn cpu_clock_tick() -> std::time::Duration {
    #[cfg(windows)]
    {
        let mut adjustment: u32 = 0;
        let mut increment: u32 = 0;
        let mut disabled: windows_sys::core::BOOL = 0;
        #[expect(
            unsafe_code,
            reason = "`GetSystemTimeAdjustment` writes three plain integers through \
                      out-pointers to locals this frame owns, and reads nothing else"
        )]
        let read = unsafe {
            windows_sys::Win32::System::SystemInformation::GetSystemTimeAdjustment(
                &raw mut adjustment,
                &raw mut increment,
                &raw mut disabled,
            )
        };
        if read == 0 {
            // The default clock interval, when the system did not say.
            return std::time::Duration::from_micros(15_625);
        }
        // `increment` counts 100-nanosecond units.
        std::time::Duration::from_nanos(u64::from(increment) * 100)
    }
    #[cfg(not(windows))]
    {
        std::time::Duration::ZERO
    }
}

/// The calling thread's stack pointer, read from the register.
///
/// What native code compares against [`Ctx::stack_bounds`] and what
/// [`Ctx::new`] arms from, so that both ends of the compare measure the stack
/// compiled code walks down. The address of a local is not that under ASAN's
/// `detect_stack_use_after_return`: it moves every address-taken local into a
/// fake frame on the heap, megabytes from the real stack, and a compare
/// between two such addresses says nothing about depth. A register read has no
/// local to move.
///
/// Inlined, so the address is the caller's frame. On a target without a
/// register read here, and under Miri, which interprets no inline assembly, it
/// is the address of a local — exact on an uninstrumented build.
#[inline(always)]
#[must_use]
pub fn stack_pointer() -> usize {
    #[cfg(all(target_arch = "x86_64", not(miri)))]
    {
        let sp: usize;
        #[expect(
            unsafe_code,
            reason = "one register move into an output operand: it reads no \
                      memory, writes none and leaves the stack and the flags alone"
        )]
        unsafe {
            std::arch::asm!("mov {}, rsp", out(reg) sp, options(nomem, nostack, preserves_flags));
        }
        sp
    }
    #[cfg(all(target_arch = "aarch64", not(miri)))]
    {
        let sp: usize;
        #[expect(
            unsafe_code,
            reason = "one register move into an output operand: it reads no \
                      memory, writes none and leaves the stack and the flags alone"
        )]
        unsafe {
            std::arch::asm!("mov {}, sp", out(reg) sp, options(nomem, nostack, preserves_flags));
        }
        sp
    }
    #[cfg(not(all(any(target_arch = "x86_64", target_arch = "aarch64"), not(miri))))]
    {
        let here = 0_u8;
        std::hint::black_box(&raw const here).addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    std::thread_local! {
        /// Set by a test to make the next [`Ctx::collect_if_asked`] on this
        /// thread panic once it has lowered the ask.
        pub(super) static PANIC_IN_COLLECT: std::cell::Cell<bool> =
            const { std::cell::Cell::new(false) };
    }

    #[test]
    fn a_panic_in_the_collection_branch_is_a_fatal_and_the_context_drops() {
        // The poll's slow path is reached from compiled code, which has no
        // unwind tables, so a panic here must end the request and never the
        // process.
        let mut ctx = Ctx::buffered();
        ctx.request_safepoint(SafepointFlags::COLLECT);
        PANIC_IN_COLLECT.set(true);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let status = unsafe { nvs_safepoint(&raw mut ctx) };
        assert_eq!(status, crate::FATAL);
        assert_eq!(
            ctx.pending().as_deref(),
            Some("internal error: injected collection panic")
        );
        assert!(!PANIC_IN_COLLECT.get(), "the injection was never reached");
        drop(ctx);

        // The next request on this thread is answered.
        let mut next = Ctx::buffered();
        next.request_safepoint(SafepointFlags::COLLECT);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let status = unsafe { nvs_safepoint(&raw mut next) };
        assert_eq!(status, crate::OK);
        assert!(next.pending().is_none());
    }

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

    /// A reroot starts a new tree even where the context is the old tree's
    /// only holder, so the next request never shares its address with the
    /// last one, and nothing keyed by that address carries over.
    #[test]
    fn a_reroot_starts_a_new_tree_where_nothing_else_holds_the_old_one() {
        let mut connection = Ctx::buffered();
        let request = connection.isolate(OutputSink::Buffer(Vec::new()));
        drop(request);
        let left = connection.tree_key();
        connection.reroot();
        assert_ne!(
            connection.tree_key(),
            left,
            "the next request was given the tree of the request before it"
        );
    }

    /// The other direction: work still running in the tree the connection
    /// left keeps the stop that was raised on it.
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

    /// What a request's calls into extensions keep in its tree, standing in for
    /// the instances: it sets its flag when it is dropped.
    struct Instances(std::sync::Arc<std::sync::atomic::AtomicBool>);

    impl Drop for Instances {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// A connection's context, handed to a task and armed on its stack as a
    /// spawn does, whose request called an extension. Returns the context and
    /// the flag its instances set when they are dropped.
    fn connection_that_called_an_extension() -> (Ctx, std::sync::Arc<std::sync::atomic::AtomicBool>)
    {
        let mut connection = Ctx::buffered();
        connection.leave_this_stack();
        connection.arm_on_this_stack();
        assert!(
            crate::budget::turn_holds(&connection.tree),
            "the connection's task does not run its tree's turn"
        );
        let dropped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = std::sync::Arc::clone(&dropped);
        let made = connection
            .tree
            .extensions(|| Ok::<ExtensionHost, ()>(std::sync::Arc::new(Instances(flag))));
        drop(made);
        (connection, dropped)
    }

    /// `rule:packaging/a-fresh-instance-per-request`: a connection that waits
    /// for its next request holds no extension instance. Its context and the
    /// turn its task runs both hold its tree, and the instances are dropped
    /// all the same.
    #[test]
    fn a_connection_between_requests_drops_its_extension_instances() {
        std::thread::spawn(|| {
            let (mut connection, dropped) = connection_that_called_an_extension();
            connection.between_requests();
            assert!(
                dropped.load(std::sync::atomic::Ordering::Relaxed),
                "a connection waiting for its next request kept its last request's instances"
            );
        })
        .join()
        .expect("the connection's thread panicked");
    }

    /// The other direction: after-response work still running in the tree
    /// keeps the instances while the connection waits, and they are dropped
    /// when that work ends. The connection stays open and starts no request,
    /// so only its leaving the tree can let the tree end.
    #[test]
    fn after_response_work_keeps_the_extension_instances_until_it_ends() {
        std::thread::spawn(|| {
            let (mut connection, dropped) = connection_that_called_an_extension();
            let mut deferred = connection.isolate(OutputSink::Buffer(Vec::new()));
            deferred.leave_this_stack();
            let left = connection.tree_key();
            connection.between_requests();
            assert!(
                !dropped.load(std::sync::atomic::Ordering::Relaxed),
                "the instances were dropped while after-response work could still call them"
            );
            assert_eq!(
                deferred.tree_key(),
                left,
                "after-response work was moved off the tree its request ran in"
            );
            assert_ne!(
                connection.tree_key(),
                left,
                "a waiting connection kept the tree its after-response work runs in"
            );

            drop(deferred);
            assert!(
                dropped.load(std::sync::atomic::Ordering::Relaxed),
                "the instances outlived the after-response work, on a connection that still waits"
            );
        })
        .join()
        .expect("the connection's thread panicked");
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
    fn a_debug_break_request_runs_the_stop_handler_once_and_its_status_ends_the_poll() {
        // With no handler the flag is lowered and the poll runs on.
        let mut ctx = Ctx::buffered();
        ctx.request_safepoint(SafepointFlags::DEBUG_BREAK);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let status = unsafe { nvs_safepoint(&raw mut ctx) };
        assert_eq!(status, crate::OK);
        assert!(ctx.safepoint_flags().is_empty());

        // A handler runs once per poll, inside a helper frame, with the flag
        // still raised for the tree, and the status it returns is the poll's.
        // A poll inside the handler does not run it again.
        let stops = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let seen = std::sync::Arc::clone(&stops);
        let weak = ctx.safepoint_view().downgrade();
        ctx.set_debug_stop(Some(std::sync::Arc::new(
            move |ctx: &mut Ctx, cause: super::super::StopCause| {
                assert!(
                    crate::HelperFrame::on_this_stack(),
                    "a stop runs inside a helper frame"
                );
                assert_eq!(cause, super::super::StopCause::Break);
                assert!(ctx.safepoint_flags().contains(SafepointFlags::DEBUG_BREAK));
                #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
                let nested = unsafe { nvs_safepoint(&raw mut *ctx) };
                assert_eq!(nested, crate::OK);
                assert!(weak.release_debug_stop());
                let seen = seen.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if seen == 1 {
                    crate::OK
                } else {
                    ctx.set_pending("stopped by the controller");
                    crate::FATAL
                }
            },
        )));
        for want in [crate::OK, crate::FATAL] {
            ctx.request_safepoint(SafepointFlags::DEBUG_BREAK);
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            let status = unsafe { nvs_safepoint(&raw mut ctx) };
            assert_eq!(status, want);
        }
        assert_eq!(stops.load(std::sync::atomic::Ordering::Relaxed), 2);
        assert_eq!(ctx.pending().as_deref(), Some("stopped by the controller"));
    }

    #[test]
    fn a_held_wall_deadline_is_none_and_moves_by_the_time_held() {
        let tree = TreeState::default();
        tree.wall_time
            .store(10_000_000_000, std::sync::atomic::Ordering::Relaxed);
        let before = tree.wall_deadline().expect("a wall time");
        tree.hold_deadlines();
        assert_eq!(tree.wall_deadline(), None, "a stopped tree has no deadline");
        std::thread::sleep(std::time::Duration::from_millis(5));
        tree.release_deadlines();
        let after = tree.wall_deadline().expect("a wall time again");
        assert!(after >= before + std::time::Duration::from_millis(5));
    }

    /// The `<used>` and `<limit>` of a time `FATAL` that starts with `prefix`,
    /// in the one form the overshoot scan parses.
    fn time_used(message: &str, prefix: &str) -> (u128, u128) {
        let rest = message
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix(": "))
            .and_then(|rest| rest.strip_suffix(" ms"))
            .unwrap_or_else(|| panic!("not a time reading: {message}"));
        let (used, limit) = rest
            .split_once(" ms used of a limit of ")
            .unwrap_or_else(|| panic!("not a time reading: {message}"));
        (
            used.parse().expect("the used time is a number"),
            limit.parse().expect("the limit is a number"),
        )
    }

    #[test]
    fn a_wall_time_fatal_names_the_time_used_and_its_limit() {
        let ctx = Ctx::buffered();
        ctx.tree
            .wall_time
            .store(1_000_000, std::sync::atomic::Ordering::Relaxed);
        // Waits for the deadline by the clock the message reads.
        while ctx.wall_time_left() != Some(std::time::Duration::ZERO) {
            std::thread::yield_now();
        }
        let message = ctx.wall_time_message();
        let (used, limit) = time_used(&message, Ctx::WALL_TIME_EXCEEDED);
        assert_eq!(limit, 1, "{message}");
        assert!(used >= limit, "{message}");
    }

    #[test]
    fn a_cpu_time_fatal_with_no_reading_names_its_limit_alone() {
        // No host installed a reading in this test binary.
        let mut ctx = Ctx::buffered();
        ctx.set_cpu_limit(150_000_000);
        assert_eq!(ctx.cpu_time_spent(), None);
        assert_eq!(ctx.cpu_time_left(), None);
        assert_eq!(
            ctx.cpu_time_message(),
            "the request exceeded its CPU-time limit of 150 ms"
        );
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
