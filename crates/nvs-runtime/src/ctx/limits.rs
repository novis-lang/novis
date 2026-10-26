//! The ceilings a request runs inside, and how it reports crossing one.
//!
//! `rule:errors/escalation-ladder`'s resource limits:
//! memory, output, CPU time and script depth, each with the accessor pair that
//! arms it and the `*_breach` that turns a crossing into a [`crate::Fault`].
//! [`Ctx::refresh_limits`] is where a configuration snapshot becomes armed
//! numbers, and the `configured_*` readers below it are the one place a key's
//! default is written down. The `intake_*` triple is the one ceiling read per
//! call rather than per request — the same directive as the output one, in a
//! different unit, and [`Ctx::intake_limit`] owns why.
//!
//! A breach is *reported*, not raised. What runs on it is
//! [`super::hooks`]'s handler, which is a separate question and a separate
//! file: the ceiling is arithmetic over fields, the response is user code.

use super::*;

/// [`Ctx::new`]'s note of where it ran and what it allocated, by address:
/// what [`Ctx::hand_over_its_making`] checks before it credits anything.
#[derive(Debug, Clone, Copy)]
pub(super) struct Made {
    /// The tree whose turn made the context.
    pub(super) turn: *const TreeState,
    pub(super) tree: *const TreeState,
    pub(super) deadline: *const std::sync::atomic::AtomicU64,
    pub(super) live: *const crate::object::LiveList,
}

impl Made {
    /// The bytes an `Arc<T>` or an `Rc<T>` allocates: two counts, then the
    /// value, as the standard library lays both out.
    fn inner<T>() -> usize {
        std::alloc::Layout::new::<[usize; 2]>()
            .extend(std::alloc::Layout::new::<T>())
            .map_or(0, |(layout, _)| layout.pad_to_align().size())
    }
}

impl Ctx {
    /// How many bytes this request has allocated and not yet freed.
    ///
    /// The difference between the thread's balance now and what it was when
    /// this context was made, floored at zero: a request that frees more than
    /// it allocated — because it released what it inherited — has used none of
    /// its own budget rather than a negative amount of it. [`crate::budget`]
    /// says why the underlying counter is per thread.
    ///
    /// **Plus whatever the tree holds on another core**, for a context on the
    /// tree's root core — one relaxed load of
    /// [`crate::budget::OffCore`], which is what makes the ceiling that stops
    /// the tree the root's whichever core a child was placed on
    /// (`rule:security/isolate-budget-is-the-trees`). A context off that core
    /// adds nothing and reads its own thread, which already holds every context
    /// beneath it there — [`Self::on_root_core`]'s field doc owns the division.
    #[must_use]
    #[inline]
    pub fn memory_used(&self) -> usize {
        self.memory_share(crate::budget::live_bytes())
    }

    /// This request's share of the thread's absolute balance `live`, read the
    /// way [`Self::memory_used`] reads the balance now.
    #[inline]
    fn memory_share(&self, live: isize) -> usize {
        let own = live.saturating_sub(self.memory_base);
        let tree = if self.on_root_core {
            self.tree.off_core.memory()
        } else {
            0
        };
        usize::try_from(own.saturating_add(tree)).unwrap_or(0)
    }

    /// Charges this context's share of the tree's budget into the counters its
    /// tree shares, where it runs on a core other than its root's.
    ///
    /// Nothing at all for any other context — the `None` this returns on is what
    /// keeps an ordinary request to one predictable branch — and the difference
    /// since the last publication for one that does, so a member that has not
    /// moved since its last poll writes nothing either.
    ///
    /// **The cadence is the poll, not the allocation.** Charging an atomic per
    /// allocation would put a contended read-modify-write on the far core's
    /// allocation path to buy a freshness nothing enforced depends on: what
    /// bounds a child placed off the root's core is the sub-cap it was handed
    /// where it was placed, which is what remained of the tree's budget there
    /// (`rule:security/isolate-budget-is-the-trees`). So the tree's reading of
    /// such a member is as fresh as that member's last poll, and
    /// [`Self::memory_breach`] — the question `crate::run_helper` asks ahead of
    /// every `Core` member — is where the poll happens.
    #[inline]
    pub fn publish_off_core(&self) {
        if let Some(share) = &self.tree_share {
            self.publish_share(share);
        }
    }

    /// [`Self::publish_off_core`]'s work, out of line so the test in front of it
    /// is all an ordinary request's poll inlines.
    #[inline(never)]
    fn publish_share(&self, share: &TreeShare) {
        let memory = crate::budget::live_bytes().saturating_sub(self.memory_base);
        let moved = memory.saturating_sub(share.memory.get());
        if moved != 0 {
            self.tree.off_core.charge_memory(moved);
            share.memory.set(memory);
        }
        let written = crate::budget::written_bytes().saturating_sub(self.output_base);
        let wrote = written.saturating_sub(share.output.get());
        if wrote != 0 {
            self.tree.off_core.charge_output(wrote);
            share.output.set(written);
        }
    }

    /// Gives this context's share of the tree's memory back, publishing what it
    /// wrote one last time — what a member off the root's core owes as it ends.
    ///
    /// The two counters part here, and they part for the reason
    /// [`crate::budget`] keeps one signed and one monotonic: the bytes a context
    /// held are released with it, so leaving its share on the tree's balance
    /// would charge the tree for a member that has gone, while the bytes it
    /// wrote are in a response and are the tree's for good.
    ///
    /// The memory share is given back **whole** rather than re-measured. A
    /// context that ends holding bytes has handed them to whatever outlives it
    /// on its own core, and that thread's balance is where they are counted;
    /// carrying a remainder on the tree's pair would be a figure nothing is ever
    /// going to take off again.
    pub(super) fn end_off_core_share(&mut self) {
        let Some(share) = self.tree_share.take() else {
            return;
        };
        let written = crate::budget::written_bytes().saturating_sub(self.output_base);
        self.tree
            .off_core
            .charge_output(written.saturating_sub(share.output.get()));
        self.tree.off_core.charge_memory(-share.memory.get());
    }

    /// The highest [`Self::memory_used`] has been during this request.
    ///
    /// The mark [`crate::budget`] recorded, read against the same zero point and
    /// floored the same way, so it is comparable with
    /// [`Self::memory_limit`] exactly as the current figure is.
    /// `rule:observability/a-memory-peak-is-recorded-not-asked-for` is why the
    /// allocator keeps it rather than this reader sampling: Novis releases on
    /// the last reference dying, so the current figure has already fallen back
    /// by the time anything asks, and the spike is the thing worth knowing.
    #[must_use]
    pub fn memory_peak(&self) -> usize {
        usize::try_from(crate::budget::peak_bytes().saturating_sub(self.memory_base)).unwrap_or(0)
    }

    /// Reports a served request's memory peak at its end: one observation of
    /// `nvs_request_memory_peak_bytes`, and one `Warn` when the peak crossed
    /// `[limits] memory_high_water`.
    ///
    /// The two readings `rule:observability/default-series` and
    /// `rule:observability/memory-high-water-writes-a-warn` take of the mark
    /// [`Self::memory_peak`] already holds, so neither adds a probe site. Called
    /// by the isolate root while this context still exists, because the door
    /// that counts the request never holds it. Both carry the declared route
    /// name, `rule:observability/route-label-is-the-declared-name`'s label.
    ///
    /// The ceiling a fraction is taken of is the one the operator wrote, which
    /// is [`Self::memory_limit`] with [`Self::fatal_reserve`] added back. A
    /// request with no ceiling has no threshold. The record goes to the log
    /// target, or to the diagnostic stream where none is configured, and never
    /// into the response; a record that cannot be written is dropped, as
    /// `Core\Log::write`'s is.
    ///
    /// What it spends: one configuration lookup per request with a ceiling,
    /// and a histogram observation on a core that has a registry.
    pub fn report_memory_peak(&mut self) {
        let peak = self.memory_peak();
        let route = self
            .inbound()
            .and_then(crate::Inbound::route)
            .and_then(crate::routes::Match::name)
            .map(str::to_owned);
        crate::metrics::count_request_peak(route.as_deref(), peak);
        let ceiling = self.memory_limit.saturating_add(self.fatal_reserve);
        if self.memory_limit == 0 {
            return;
        }
        let Some(fraction) = self.configured_memory_high_water() else {
            return;
        };
        #[allow(clippy::cast_precision_loss)]
        let share = peak as f64 / ceiling as f64;
        if share < fraction {
            return;
        }
        let route = route.map_or_else(
            || "no declared route".to_owned(),
            |name| format!("route `{name}`"),
        );
        let line = format!(
            "the request's memory peak reached {peak} bytes of its {ceiling}-byte ceiling, \
             past `[limits] memory_high_water` = {fraction}, on {route}",
        );
        let mut record = nvs_render::Record::at(nvs_render::Level::Warn);
        self.stamp_envelope(&mut record.envelope);
        record.envelope.message = Some(nvs_render::Rendered::new(&line));
        let _dropped = self.write_log_record(&record, crate::LogChannel::Diagnostic);
    }

    /// `[limits] memory_high_water` as a fraction, or `None` where the
    /// configuration does not state one. The boot path refuses a value that is
    /// not between `0` and `1`, so anything else read here is treated as unset.
    fn configured_memory_high_water(&self) -> Option<f64> {
        match self.limit("memory_high_water", nvs_config::Unit::Ratio) {
            nvs_config::Reading::Read(nvs_config::Quantity::Ratio(fraction)) => Some(fraction),
            _ => None,
        }
    }

    /// `[limits] leaf` as this request reads it, in `unit`: the table its
    /// snapshot resolved once unless its overlay holds the key
    /// (`nvs_config::Request::limit`). A context with no configuration reads
    /// every limit as unset.
    fn limit(&self, leaf: &str, unit: nvs_config::Unit) -> nvs_config::Reading {
        self.config
            .as_ref()
            .map_or(nvs_config::Reading::Unset, |config| {
                config.limit(leaf, unit)
            })
    }

    /// The ceiling **ordinary execution** is held to, in bytes, `0` for no cap.
    ///
    /// `[limits] memory` less [`Self::fatal_reserve`], because `rule:errors/on-limit`'s
    /// slice is carved out of the request's own budget rather than added to it
    /// — so this is the number that moves, once, while the tier-1 handler runs
    /// ([`Self::run_limit_handler`]).
    #[must_use]
    pub fn memory_limit(&self) -> usize {
        self.memory_limit
    }

    /// Sets the ceiling directly, for a caller holding no configuration —
    /// `nvs-host`'s isolates and this crate's own tests.
    ///
    /// A request with a configuration gets its ceiling from
    /// [`Self::set_config`] instead, so this is never the way `[limits] memory`
    /// arrives.
    pub fn set_memory_limit(&mut self, bytes: usize) {
        self.memory_limit = bytes;
        self.arm_memory_ceiling();
    }

    /// Reads this request's baseline again at the thread's balance now, starts
    /// its memory peak there, and arms its ceiling against it.
    ///
    /// `nvs run` calls it once the context is wired, just before the program's
    /// task is spawned. What the run's setup made after the context — the
    /// command line, the program's name and id, the route table — is the
    /// host's, and its size follows the paths the run was started with. Charged
    /// to the program, it would move the allocation at which the ceiling is
    /// crossed, and with it every collection, by the length of a folder's name
    /// (`rule:testing/bench-counters`).
    ///
    /// **What it spends:** those setup bytes, a few kilobytes per run, are not
    /// charged against `[limits] memory`. They are bounded by the run's own
    /// arguments and configuration, not by anything the program does.
    pub fn rebase_memory(&mut self) {
        self.memory_base = crate::budget::live_bytes();
        crate::budget::restart_peak();
        self.arm_memory_ceiling();
    }

    /// Arms the allocator with this request's ceiling as an absolute balance,
    /// so a growing allocation is measured against it where it happens rather
    /// than at the next poll.
    ///
    /// **The writers that resolve a ceiling call this** — [`Self::set_config`]
    /// through [`Self::refresh_limits`], [`Self::set_memory_limit`], and
    /// [`Self::run_limit_handler`] at both ends of the reserve it lends the
    /// handler — because what it arms is a mirror of [`Self::memory_limit`] and
    /// a stale mirror would hold a request to a ceiling it no longer has. The
    /// handler's end of it is the one that cannot be skipped: the threshold is
    /// what [`crate::budget::affords`] refuses against, so a handler given the
    /// reserve in the ceiling alone would be refused the very allocations
    /// `rule:errors/on-limit` reserved it bytes to make. The address armed
    /// beside the number is this context's safepoint word, which is the request
    /// *tree*'s — so a crossing stops whichever context in the tree is running,
    /// the same division `rule:security/isolate-shares-nothing` gives the budget
    /// itself.
    ///
    /// [`Self::handler_isolate`] writes its reserve as a ceiling and arms
    /// nothing, which is the direction that fails open: its handler is bounded
    /// by the counter its own polls read, exactly as every request was before
    /// this existed.
    ///
    /// A request under no ceiling arms the threshold `0`: [`crate::budget`]'s
    /// sentinel, and short-circuiting on it is what keeps the uncapped case to
    /// one compare per allocation. It still arms its word, because the server's
    /// memory running out stops an uncapped request as well
    /// (`crate::budget::held_by_cores`).
    pub(super) fn arm_memory_ceiling(&mut self) {
        crate::budget::arm(if self.memory_limit == 0 {
            crate::budget::Armed::new(0, self.safepoint)
        } else {
            // Absolute, and saturating for the reason [`Self::memory_used`]
            // floors at zero: the threshold is a balance rather than a size, so
            // a ceiling that cannot be added to this request's baseline is one
            // no allocation on this thread will reach.
            let ceiling = isize::try_from(self.memory_limit).unwrap_or(isize::MAX);
            crate::budget::Armed::new(self.memory_base.saturating_add(ceiling), self.safepoint)
        });
    }

    /// Gives the stack that made this context back the arming and the refusal
    /// the context displaced, and keeps the context's own until
    /// [`Self::arm_on_this_stack`].
    ///
    /// `nvs_host`'s `spawn` and `spawn_child` call it on the stack that made the
    /// context, as they hand it to a task. Without it, that stack runs on under
    /// the child's ceiling. It assumes the context is the last one this stack
    /// armed, which holds because a spawn hands a context over as soon as it is
    /// made.
    ///
    /// It also gives this stack back the memory mark the context displaced,
    /// and stores the tree's balance as it stands, in the units this
    /// context's baseline was read in. The task's first turn starts from that
    /// balance ([`crate::budget`]'s *per request tree across switches*).
    pub fn leave_this_stack(&mut self) {
        if !self.memory_off_stack {
            self.swap_arming();
            if self.on_root_core {
                self.tree.balance.store(
                    crate::budget::live_bytes(),
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
            crate::budget::publish_peak(self.memory_peak_saved);
            self.memory_peak_saved = isize::MIN;
            self.memory_off_stack = true;
        }
    }

    /// Arms this context's own ceiling and refusal on the stack that now runs
    /// it, and saves what that stack had: the first thing a task does with the
    /// context it was handed.
    ///
    /// The scheduler starts every task under nothing
    /// ([`crate::budget::ArmedStack`]), so what is saved is nothing, and the
    /// context's drop puts nothing back. On the tree's root core the task's
    /// turns are the tree's from here, so the baseline and the ceiling are
    /// read in the tree's units. The memory mark starts here.
    pub fn arm_on_this_stack(&mut self) {
        if self.memory_off_stack {
            if self.on_root_core {
                crate::budget::begin_turn(&self.tree);
            }
            self.swap_arming();
            self.memory_peak_saved = crate::budget::rebase_peak();
            self.memory_off_stack = false;
        }
    }

    /// Drops this context as one more turn of its own tree: what `nvs_host`'s
    /// scheduler does with a child's context after the child's last turn.
    ///
    /// The context holds the request's arena and carrier, and the scheduler's
    /// own stack runs no tree's turn. Dropped there, those bytes would leave
    /// the thread's balance and no tree's, so the tree's next turn would read
    /// them as still held ([`crate::budget`]'s *per request tree across
    /// switches*). Off the tree's root core the thread's balance is the
    /// reading already, so the context is simply dropped.
    ///
    /// **Cost:** one [`crate::budget::ArmedStack`] take and restore per child
    /// that ends, nothing per turn.
    pub fn end_in_its_turn(self) {
        if !self.on_root_core {
            drop(self);
            return;
        }
        let outside = crate::budget::ArmedStack::take();
        crate::budget::begin_turn(&self.tree);
        drop(self);
        drop(crate::budget::ArmedStack::take());
        outside.restore();
    }

    /// Moves what making this context cost, and `body` bytes more, off the
    /// running tree's balance: `nvs_host`'s `spawn_child` and `spawn_root`, as
    /// they hand the context to a task of its own tree.
    ///
    /// [`Self::new`] allocates the tree's state, the deadline and the object
    /// list on the turn that spawns, and `body` is the task's boxed body. The
    /// child frees all of them, in its last turn or after it. Left on the
    /// spawning tree, each spawn is a charge nobody frees, so an accept loop's
    /// balance grows with every connection it serves
    /// (`rule:security/isolate-budget-is-the-trees`).
    ///
    /// It credits nothing unless the context is still its own tree, unshared,
    /// and was made on this turn. A context that joined another tree frees its
    /// state on that tree's turns, and crediting it as well would let a loop of
    /// spawns lower a request's balance with no bound. Each allocation counts
    /// only while it is still the one [`Self::new`] made and nothing else
    /// holds it. One that was replaced was freed on the turn that replaced it.
    ///
    /// **Cost:** a thread-local read and three compares per spawn.
    pub fn hand_over_its_making(&mut self, body: usize) {
        let Some(made) = self.made.take() else {
            return;
        };
        let own_tree = std::ptr::eq(std::sync::Arc::as_ptr(&self.tree), made.tree)
            && std::sync::Arc::strong_count(&self.tree) == 1;
        if !own_tree || !std::ptr::eq(made.turn, crate::budget::running_turn()) {
            return;
        }
        let mut bytes = Made::inner::<TreeState>().saturating_add(body);
        if std::ptr::eq(std::sync::Arc::as_ptr(&self.deadline), made.deadline)
            && std::sync::Arc::strong_count(&self.deadline) == 1
        {
            bytes = bytes.saturating_add(Made::inner::<std::sync::atomic::AtomicU64>());
        }
        if std::ptr::eq(std::rc::Rc::as_ptr(&self.live), made.live)
            && std::rc::Rc::strong_count(&self.live) == 1
        {
            bytes = bytes.saturating_add(Made::inner::<crate::object::LiveList>());
        }
        crate::budget::credit_turn(isize::try_from(bytes).unwrap_or(isize::MAX));
    }

    /// Swaps the thread's armed pair and refusal with the ones this context
    /// saved. The two calls above are its only callers, and each is the other's
    /// inverse.
    fn swap_arming(&mut self) {
        self.memory_ceiling_saved = crate::budget::displace(self.memory_ceiling_saved);
        let refused = crate::budget::take_refusal();
        crate::budget::restore_refusal(self.memory_refused_saved);
        self.memory_refused_saved = refused;
    }

    /// This request's reserved slice in bytes — the bytes ordinary execution's
    /// ceiling was reduced by, and the room
    /// [`Self::run_limit_handler`] adds back for the length of the handler.
    #[must_use]
    pub fn fatal_reserve(&self) -> usize {
        self.fatal_reserve
    }

    /// Sets the reserved slice directly, for the same callers
    /// [`Self::set_memory_limit`] exists for and with the same division of
    /// labour: this is the slice *on top of* the ceiling stated there, where a
    /// request with a configuration has it carved out of `[limits] memory`
    /// instead.
    pub fn set_fatal_reserve(&mut self, bytes: usize) {
        self.fatal_reserve = bytes;
    }

    /// Whether this request has allocated past its ceiling, or been refused an
    /// allocation that would have taken it there.
    ///
    /// **Two questions, because a refusal is a breach no counter can see.** The
    /// bytes were never handed over, so a request stopped in front of one holds
    /// *less* than its ceiling and reads as comfortably inside it;
    /// [`crate::budget::affords`] records the verdict where it happens and this
    /// is where it is read, so both polls that share this reader —
    /// [`crate::nvs_safepoint`] between two statements and
    /// [`crate::run_helper`] ahead of every `Core` member — report the refusal
    /// as the breach it is.
    ///
    /// An uncapped request can be refused nothing, because [`crate::budget`]
    /// arms it no threshold to be refused against. It asks one thing: whether
    /// the server's memory ran out while it grew, which stops a capped request
    /// too and is recorded as a refusal.
    #[must_use]
    #[inline]
    pub fn over_memory_limit(&self) -> bool {
        if self.memory_limit == 0 {
            return crate::budget::server_full();
        }
        crate::budget::refused() || self.memory_used() > self.memory_limit
    }

    /// How many bytes this request has written to its response, in the sense
    /// `[limits] max_output` means.
    ///
    /// This request's share of the thread's count, taken against
    /// [`Self::output_base`] — so a root's reading holds every isolate spawned
    /// beneath it and each isolate's holds only its own, which is
    /// `rule:security/isolate-shares-nothing`'s
    /// "child output against the root's `max_output`" and the same arrangement
    /// [`Self::memory_used`] already has, plus the tree's off-core count for the
    /// same reason and on the same branch.
    #[must_use]
    pub fn output_used(&self) -> usize {
        let own = crate::budget::written_bytes().saturating_sub(self.output_base);
        if self.on_root_core {
            own.saturating_add(self.tree.off_core.output())
        } else {
            own
        }
    }

    /// The response-size ceiling this request is held to, in bytes, `0` for no
    /// cap.
    ///
    /// `[limits] max_output` as written, with nothing carved out of it — see
    /// the field doc for why this ceiling has no reserved slice where the
    /// memory one does.
    #[must_use]
    pub fn output_limit(&self) -> usize {
        self.output_limit
    }

    /// Sets the response ceiling directly, for a caller holding no
    /// configuration — [`Self::set_memory_limit`] exists for the same callers
    /// and with the same division of labour.
    pub fn set_output_limit(&mut self, bytes: usize) {
        self.output_limit = bytes;
    }

    /// Whether this request has written past its response ceiling.
    ///
    /// [`Self::over_memory_limit`]'s shape exactly: an uncapped request answers
    /// `false` on the first compare without reading the counter at all.
    #[must_use]
    pub fn over_output_limit(&self) -> bool {
        self.output_limit != 0 && self.output_used() > self.output_limit
    }

    /// The largest single buffer a `Core` member may fill from **outside** this
    /// request, in bytes, `0` for no cap.
    ///
    /// `[limits] max_output`'s number a second time, and deliberately not a
    /// second directive: `rule:core-classes/process-run` reuses the one an
    /// operator already writes rather than adding a cap they would have to keep
    /// in step with it. `Core\Process::run`'s two captures and `Core\IO::read`'s
    /// buffer are what ask, through [`Self::intake_bound`] and
    /// [`Self::intake_breach`] — the three together are the whole of what a call
    /// site needs, so no member does this arithmetic itself.
    ///
    /// **Per call, where [`Self::output_limit`] is per request.** A response is
    /// a running total because every byte written to one stays in it; a buffer
    /// read in from a child or a file is freed when the value holding it dies,
    /// so a running total would bound a request's *lifetime* reading rather than
    /// what it holds at once, and a loop reading a small file a thousand times
    /// would fail on a ceiling meant to catch one enormous read. Each call is
    /// measured on its own and nothing accumulates.
    #[must_use]
    pub fn intake_limit(&self) -> usize {
        self.output_limit
    }

    /// [`Self::intake_limit`] as the count a bounded read stops at: one byte
    /// past the ceiling, and `u64::MAX` where there is no ceiling at all.
    ///
    /// One past rather than the ceiling itself, so that a read which fills this
    /// is over by exactly the byte that proves it — a reader stopping *at* the
    /// ceiling cannot tell a file that exactly fits from one that does not.
    /// Nothing unbounded is ever held: the extra byte is the whole of what a
    /// refusal costs over an acceptance.
    #[must_use]
    pub fn intake_bound(&self) -> u64 {
        match self.intake_limit() {
            0 => u64::MAX,
            limit => u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1),
        }
    }

    /// The [`crate::Fault`] `member` owes for taking `bytes` in past
    /// [`Self::intake_limit`], or `None` where the read is inside it.
    ///
    /// **A throw, where [`Self::output_breach`] is a `FATAL`**, and the two are
    /// not the same event. A breach is the request having already gone too far,
    /// noticed at a poll with nothing left to decide. This is a member
    /// *refusing* before anything crosses that ceiling — nothing has reached the
    /// response, so no limit has been exceeded and the request is intact. It is
    /// `rule:security/denial-is-a-runtime-error`'s shape rather than
    /// `rule:errors/on-limit`'s: the caller asked for something this request may
    /// not hold, can catch that, and can ask for less.
    #[must_use]
    pub fn intake_breach(&self, member: &str, bytes: usize) -> Option<crate::Fault> {
        let limit = self.intake_limit();
        if limit == 0 || bytes <= limit {
            return None;
        }
        Some(crate::Fault::thrown(format!(
            "{member} read at least {bytes} bytes, past the `[limits] max_output` ceiling of {limit} \
             this request holds a single read to",
        )))
    }

    /// The [`crate::Fault`] a request past its memory ceiling owes, or `None`
    /// while it is inside it.
    ///
    /// [`crate::Fault::fatal`] and never a throw:
    /// `rule:errors/on-limit` makes
    /// every resource-limit breach a `FATAL`, so no `catch` sees this and a
    /// fixture that wraps the loop in one has found the rule rather than a bug.
    /// The message names the ceiling as well as the reading, because the two
    /// together are what tells an operator whether to raise the limit or to fix
    /// the program, and a help line names both ways to raise it. The subject is
    /// "the program" on a CLI run and "the request" in server work. After a refusal the reading is the one taken when the
    /// allocation was refused, followed by what it asked for
    /// ([`crate::budget::Refusal`] owns why): by the poll, a refused
    /// concatenation has already given back the text it was growing.
    ///
    /// **Under the ceiling this is a few loads and branches, and it is inlined
    /// into each member's frame.** `crate::run_helper` is generic, so it is
    /// built in the crate that defines the member, and a call here was a call
    /// across crates into a frame sized for the message below. The message is
    /// [`Self::memory_fault`], out of line and cold. What is left is
    /// [`Self::publish_off_core`]'s one test, [`Self::over_memory_limit`]'s
    /// reads, the root-core test and one comparison.
    #[must_use]
    #[inline]
    pub fn memory_breach(&self) -> Option<crate::Fault> {
        // A member off its tree's root core publishes here and nowhere else —
        // [`Self::publish_off_core`] owns why the poll is the cadence, and this
        // is the poll every `Core` member passes through.
        self.publish_off_core();
        if !self.over_memory_limit() {
            return None;
        }
        Some(self.memory_fault())
    }

    /// The fault [`Self::memory_breach`] reports, once the poll has found the
    /// request past its ceiling.
    #[cold]
    #[inline(never)]
    fn memory_fault(&self) -> crate::Fault {
        // The server's memory ran out while this request grew: its own ceiling
        // was not reached, so the message names neither the ceiling nor a
        // refused size.
        if let Some(refusal) = crate::budget::refusal().filter(|refusal| refusal.full) {
            return crate::Fault::fatal(format!(
                "the server's memory is full, so the request was stopped while it held {} bytes",
                self.memory_share(refusal.live),
            ));
        }
        // The ceiling named is the request's **whole** budget, not the reduced
        // one it was measured against: `[limits] memory` is the number the
        // operator wrote and the only one they can recognise. Where a slice of
        // it is `rule:errors/on-limit`'s reserve, saying so is what keeps the sentence
        // from reading as a reading below its own ceiling.
        let reserved = match self.fatal_reserve {
            0 => String::new(),
            bytes => format!(", of which {bytes} is reserved for the limit handler"),
        };
        let ceiling = self.memory_limit.saturating_add(self.fatal_reserve);
        let run = self.memory_run();
        let reading = match crate::budget::refusal() {
            Some(refusal) => format!(
                "the {run} exceeded its memory limit — {} bytes held against a ceiling of {ceiling}{reserved}, when an allocation of {} bytes was refused",
                self.memory_share(refusal.live),
                refusal.asked,
            ),
            None => format!(
                "the {run} exceeded its memory limit — {} bytes held against a ceiling of {ceiling}{reserved}",
                self.memory_used(),
            ),
        };
        crate::Fault::fatal(format!("{reading}\n{}", self.memory_help(ceiling)))
    }

    /// What [`Self::memory_breach`] calls the run: "program" on a CLI run, "request" in server
    /// work.
    fn memory_run(&self) -> &'static str {
        if self.run_kind().is_server_work() {
            "request"
        } else {
            "program"
        }
    }

    /// The help line under a memory breach: the `[limits] memory` line to raise, and the
    /// `Core\Config::set` call that raises it from the code, with twice `ceiling` as the value.
    /// Where a `[limits.hard] memory` bounds the call, the line says so. A CLI run with none
    /// written has no bound (`rule:config/ceilings-are-their-own-directives`).
    fn memory_help(&self, ceiling: usize) -> String {
        const MIB: usize = 1 << 20;
        let raised = ceiling.saturating_mul(2).div_ceil(MIB);
        let bounded = self
            .config()
            .and_then(|config| config.get("limits.hard.memory"))
            .is_some_and(|hard| hard != "false");
        let bound = if bounded {
            ", up to `[limits.hard] memory`"
        } else {
            ""
        };
        format!(
            "help: raise `memory` under `[limits]` in nvs.toml, or call \
             `Core\\Config::set('memory', '{raised}M')` in the {}{bound}",
            self.memory_run(),
        )
    }

    /// The [`crate::Fault`] a request past its response ceiling owes, or `None`
    /// while it is inside it.
    ///
    /// A `FATAL` for [`Self::memory_breach`]'s reason, and it names both
    /// numbers for that method's reason too — except that there is no reserve
    /// to subtract here, so the ceiling named is the one the operator wrote
    /// with nothing to explain about it.
    ///
    /// **What it reports is the tree's reading, not this context's writing.**
    /// A root stopped here may have written nothing itself and be over because
    /// its isolates were: that is what the directive bounds, so the message
    /// says "the request and everything it spawned" rather than implying a
    /// single `echo` went too far.
    #[must_use]
    pub fn output_breach(&self) -> Option<crate::Fault> {
        if !self.over_output_limit() {
            return None;
        }
        Some(crate::Fault::fatal(format!(
            "the request exceeded its output limit — {} bytes written by the request and everything it spawned, against a ceiling of {}",
            self.output_used(),
            self.output_limit,
        )))
    }

    /// The `FATAL` a `spawn script` from this context owes, or `None` where the
    /// child it is about to build is still under the ceiling.
    ///
    /// Asked of the *child's* depth rather than this one's, and asked before
    /// the child exists: a context is never itself over `max_script_depth`,
    /// because whatever built it asked this question first. That is what makes
    /// this a refusal rather than a stop — there is no task to interrupt and no
    /// safepoint to interrupt it at, so unlike [`Self::memory_breach`] the
    /// answer is not polled but taken once, at the one call that could widen
    /// the tree. [`Limit::ScriptDepth`] is the report it becomes.
    ///
    /// A ceiling of `0` is `rule:config/three-changeability-classes`'s no-ceiling-at-all and answers `None`
    /// however deep the chain already is — see [`Self::max_script_depth`]'s
    /// field doc, which owns why only an explicit `false` reads that way here.
    ///
    #[must_use]
    pub fn script_depth_breach(&self) -> Option<crate::Fault> {
        let ceiling = self.max_script_depth;
        if ceiling == 0 {
            return None;
        }
        let child = self.script_depth.saturating_add(1);
        if child <= ceiling {
            return None;
        }
        // Both numbers, for the reason `memory_breach` names both of its own:
        // the ceiling is the number the operator wrote and the only one they
        // can recognise, and the depth beside it is what says whether the
        // program recursed or the ceiling is simply low.
        Some(crate::Fault::fatal(format!(
            "the request exceeded its `spawn script` nesting limit — a script spawned at depth {child} against a ceiling of {ceiling}",
        )))
    }

    /// Counts one more running child isolate in this context's tree, or answers
    /// the `FATAL` a tree already at `[limits] max_tasks` owes.
    ///
    /// Asked where [`Self::script_depth_breach`] is asked and for its reason: a
    /// spawn is the one call that adds a task, so the answer is taken once
    /// there and never polled. The count is the tree's ([`TreeState::tasks`]),
    /// and the claim is one compare-and-swap on it, because a child placed on
    /// another core spawns from there at the same time.
    ///
    /// The ceiling is the one this context reads. That is the root's, unless a
    /// spawn site's `limits:` narrowed it on the way down, and a narrowed one
    /// is only ever lower, so a child cannot let the tree grow past the root's
    /// number (`rule:security/isolate-budget-is-the-trees`). The request
    /// itself is not a task here: only its children are counted.
    ///
    /// # Errors
    ///
    /// The message of the `FATAL` naming both numbers, which the spawn raises
    /// beside [`Limit::Tasks`]'s report.
    ///
    /// [`TreeState::tasks`]: super::TreeState
    /// [`Limit::Tasks`]: crate::Limit::Tasks
    pub fn claim_task(&self) -> Result<super::TaskSlot, String> {
        let ceiling = self.configured_max_tasks();
        let claimed = self.tree.tasks.try_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |running| {
                ceiling
                    .is_none_or(|ceiling| running < ceiling)
                    .then_some(running + 1)
            },
        );
        match claimed {
            Ok(_) => Ok(super::TaskSlot(std::sync::Arc::clone(&self.tree))),
            Err(running) => Err(format!(
                "the request exceeded its task limit — a spawn was asked for with {running} running against a ceiling of {}",
                ceiling.unwrap_or(0),
            )),
        }
    }

    /// `[limits] max_tasks` as a count, or `None` for a tree under no ceiling.
    ///
    /// Unwritten, and `false`, are no ceiling, which is what
    /// `nvs_config::defaults` ships. A written `0` is a ceiling of no children
    /// at all. A malformed value is that same `0`: the boot refuses one where it
    /// can name the line, and a reading that got this far stops every spawn
    /// rather than allowing any number of them.
    fn configured_max_tasks(&self) -> Option<u64> {
        match self.limit("max_tasks", nvs_config::Unit::Count) {
            nvs_config::Reading::Read(nvs_config::Quantity::Count(count)) => Some(count),
            nvs_config::Reading::Unset
            | nvs_config::Reading::Read(nvs_config::Quantity::Unbounded) => None,
            _ => Some(0),
        }
    }

    /// Makes `slot` this context's, so the task it counts ends when this
    /// context drops — the isolate's end, however it ended.
    pub fn hold_task(&mut self, slot: super::TaskSlot) {
        self.task_slot = Some(slot);
    }

    /// Re-reads the resource ceilings this request's configuration states.
    ///
    /// Called by [`Self::set_config`], and owed by anything that moves the
    /// request's own overlay afterwards — `Core\Config::set` and `::restore`,
    /// which is why [`Self::memory_limit`]'s field doc calls the value cached
    /// rather than derived.
    pub fn refresh_limits(&mut self) {
        let ceiling = self.configured_memory_limit();
        self.fatal_reserve = Self::reserve_within(ceiling, self.configured_fatal_reserve());
        // `rule:errors/on-limit`: the slice is *carved out of* the request's own budget
        // and unavailable to ordinary execution, so the ceiling everything but
        // the handler is measured against is what is left after it. An
        // uncapped request has nothing to carve and reserves nothing: there is
        // no ceiling for a handler to be given room past.
        self.memory_limit = ceiling.saturating_sub(self.fatal_reserve);
        // The CPU half of the same slice, by the same arithmetic and in the same
        // pass. One pass rather than two because a ceiling and the reserve
        // carved out of it are one reading of one configuration: set apart, they
        // could be left disagreeing about which snapshot they came from by any
        // caller that remembered one of them.
        let cpu_ceiling = self.configured_cpu_time();
        self.fatal_reserve_time =
            Self::reserve_time_within(cpu_ceiling, self.configured_fatal_reserve_time());
        self.cpu_limit = cpu_ceiling.saturating_sub(self.fatal_reserve_time);
        // The thread that charges this tree reads the ceiling out of the tree's
        // own state, so the one `Core\Config::set` just narrowed is the one its
        // next sweep compares with.
        self.publish_cpu_limit();
        // The `wall_time` the connection reads once the request's client has
        // gone, published from the same pass for the same reason.
        self.publish_wall_time();
        // Read in the same pass and for the same reason, though there is nothing
        // to carve out of it: a request's ceilings are one reading of one
        // configuration.
        self.max_script_depth = self.configured_max_script_depth();
        // The response ceiling, in the same pass and for the same reason.
        // Nothing is carved out of it — [`Self::output_limit`]'s field doc owns
        // why a ceiling on writing needs no slice reserved from it.
        self.output_limit = self.configured_output_limit();
        // Not a ceiling, but read off the same configuration in the same pass,
        // so a `Core\Config::set` of `[errors] deprecated` reaches the word
        // compiled code loads.
        self.deprecated = self.configured_on_deprecated() as u64;
        // Last, because it mirrors what the lines above just decided: the
        // allocator is held to the ceiling ordinary execution is held to, and
        // arming it from the same pass is what stops the two from being left
        // disagreeing by a caller that moved one of them.
        self.arm_memory_ceiling();
    }

    /// `[limits] cpu_time` in nanoseconds, or `0` for a request under no cap.
    ///
    /// See [`Self::cpu_limit`]'s field doc for what the number measures. Where
    /// the configuration writes no such directive, `get` gives its default:
    /// [`nvs_config::defaults::LIMITS_CPU_TIME`] for server work
    /// ([`nvs_config::Request::serves`]) and nothing, so `0`, for a CLI program
    /// or a test. A context with no configuration is `0` too. A
    /// malformed value answers "no cap" for the reason
    /// [`Self::configured_memory_limit`] does, and `false` — `rule:config/three-changeability-classes`'s
    /// spelling of no ceiling at all — answers the same `0`, because a request
    /// that may burn any amount of CPU and one whose ceiling nothing states are
    /// the same request to everything downstream.
    ///
    fn configured_cpu_time(&self) -> u64 {
        match self.limit("cpu_time", nvs_config::Unit::Duration) {
            nvs_config::Reading::Read(nvs_config::Quantity::Nanos(nanos)) => nanos,
            _ => 0,
        }
    }

    /// Marks this context's configuration as server work, for a context whose
    /// kind of run became server work after its configuration arrived —
    /// [`Self::set_run_kind`], [`Self::set_inbound`], [`Self::set_peer`] and
    /// the connection door of [`Self::mark_event_stream`] call it.
    ///
    /// Where nothing has given this context a CPU ceiling yet, it takes the
    /// `[limits] cpu_time` in force for server work now — the written one, or
    /// [`nvs_config::defaults::LIMITS_CPU_TIME`] — with the reserve carved out
    /// of it as [`Self::refresh_limits`] carves it, and publishes it to the
    /// tree where this context is the root. A context that already serves, or
    /// that has no configuration, is left alone.
    pub(super) fn now_serving(&mut self) {
        let Some(config) = self.config.as_mut() else {
            return;
        };
        if config.serves() {
            return;
        }
        config.serve();
        if self.cpu_limit != 0 {
            return;
        }
        let ceiling = self.configured_cpu_time();
        if ceiling == 0 {
            return;
        }
        self.fatal_reserve_time =
            Self::reserve_time_within(ceiling, self.configured_fatal_reserve_time());
        self.cpu_limit = ceiling.saturating_sub(self.fatal_reserve_time);
        self.publish_cpu_limit();
    }

    /// `[limits] wall_time` in nanoseconds, or `0` for a request with none,
    /// read as [`Self::configured_cpu_time`] reads its key: a malformed value
    /// and `false` both answer `0`.
    pub(super) fn configured_wall_time(&self) -> u64 {
        match self.limit("wall_time", nvs_config::Unit::Duration) {
            nvs_config::Reading::Read(nvs_config::Quantity::Nanos(nanos)) => nanos,
            _ => 0,
        }
    }

    /// How long this request has left before its `[limits] wall_time` runs
    /// out, or `None` for a request with none. `Some(ZERO)` once it has.
    ///
    /// Counted from the start of the request this context's tree answers, so a
    /// `spawn script` child reads its request's time and not its own.
    /// `Core\Time::sleep` is the reader: a parked task reaches no poll, so a
    /// sleep that would end past the deadline ends at it instead.
    #[must_use]
    pub fn wall_time_left(&self) -> Option<std::time::Duration> {
        self.tree
            .wall_deadline()
            .map(|at| at.saturating_duration_since(std::time::Instant::now()))
    }

    /// How [`Self::wall_time_message`] starts, so `nvs test` can tell a test stopped at its
    /// limit and write its own line in place of this one, which names a request.
    pub const WALL_TIME_EXCEEDED: &'static str = "the request exceeded its wall-time limit";

    /// The `FATAL`'s message for a request past its `[limits] wall_time`,
    /// naming the ceiling as the request has it now.
    pub(super) fn wall_time_message(&self) -> String {
        let ceiling = std::time::Duration::from_nanos(
            self.tree
                .wall_time
                .load(std::sync::atomic::Ordering::Relaxed),
        );
        format!("{} of {ceiling:?}", Self::WALL_TIME_EXCEEDED)
    }

    /// Runs `rule:errors/on-limit`'s handler for a request past its
    /// `[limits] wall_time`, and gives the [`crate::Fault`] the helper that
    /// found it returns. The safepoint poll's branch, for a helper:
    /// `Core\Time::sleep` is the one caller.
    pub fn wall_time_breach(&mut self) -> crate::Fault {
        // Claimed before the handler lowers the flag, so the clock's own raise
        // has either landed already or will not come
        // ([`crate::SafepointView::stop_at_wall_time`]).
        *self
            .tree
            .wall_stop
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        self.run_limit_handler(crate::Limit::WallTime);
        crate::Fault::fatal(self.wall_time_message())
    }

    /// The CPU time this request may burn, in nanoseconds, or `0` for one under
    /// no cap — `rule:errors/on-limit`.
    ///
    /// This is the ceiling a timer compares the request thread's CPU clock
    /// against before it raises [`SafepointFlags::CPU_LIMIT`]; the field doc
    /// says why the reading is not taken here.
    #[must_use]
    pub fn cpu_limit(&self) -> u64 {
        self.cpu_limit
    }

    /// This request's reserved slice of CPU time in nanoseconds — the time
    /// [`Self::cpu_limit`] was reduced by, and the room the tier-1 handler is
    /// meant to run in.
    ///
    /// [`Self::fatal_reserve`] is the sibling this follows: both halves are
    /// added back for the length of the call in [`Self::run_limit_handler`],
    /// because a handler that cannot allocate or cannot run is a tier that says
    /// nothing. This one is a flag edit as well as a ceiling edit — that method
    /// owns why a handler entered under [`SafepointFlags::CPU_LIMIT`] would
    /// otherwise stop at its own first back edge. What samples the clock is
    /// `nvs_host::watchdog`, and it is told of neither edit: it charges against
    /// the ceiling the tree carries ([`Self::publish_cpu_limit`]), which the
    /// handler's widening leaves alone, so a handler is bounded by that
    /// sampler's next sweep rather than by the slice this names.
    #[must_use]
    pub fn fatal_reserve_time(&self) -> u64 {
        self.fatal_reserve_time
    }

    /// Sets the CPU ceiling directly, for the callers
    /// [`Self::set_memory_limit`] exists for and with the same division of
    /// labour: a request holding a configuration gets it from
    /// [`Self::set_config`] instead.
    pub fn set_cpu_limit(&mut self, nanos: u64) {
        self.cpu_limit = nanos;
        self.publish_cpu_limit();
    }

    /// Sets the reserved slice of CPU time directly, the way
    /// [`Self::set_fatal_reserve`] sets the memory half — *on top of* the
    /// ceiling stated beside it, where a request with a configuration has it
    /// carved out of `[limits] cpu_time`.
    pub fn set_fatal_reserve_time(&mut self, nanos: u64) {
        self.fatal_reserve_time = nanos;
    }

    /// The nesting a `spawn script` chain is allowed where `[limits]` states no
    /// `max_script_depth` — **the only home of this number.**
    ///
    /// Sixty-four because every level is a whole isolate with its own heap
    /// rather than a stack frame, so the depth at which a legitimate program
    /// still works is far below the depth at which recursion is the diagnosis:
    /// a generator spawning a worker that spawns a helper is three, and nothing
    /// written on purpose is sixty-four. Chosen well under where the heap would
    /// notice, so that the refusal that arrives says what is actually wrong —
    /// [`Self::max_script_depth`]'s field doc owns why that ordering is the
    /// whole reason the default exists.
    pub const DEFAULT_MAX_SCRIPT_DEPTH: u32 = nvs_config::defaults::LIMITS_MAX_SCRIPT_DEPTH;

    /// How deep a chain of `spawn script` may nest, or `0` for a tree under no
    /// ceiling — see [`Self::max_script_depth`]'s field doc, which owns why an
    /// unstated value is a default here and a `0` everywhere else.
    #[must_use]
    pub fn max_script_depth(&self) -> u32 {
        self.max_script_depth
    }

    /// Sets the nesting ceiling directly, for the callers
    /// [`Self::set_cpu_limit`] exists for and with the same division of labour.
    pub fn set_max_script_depth(&mut self, depth: u32) {
        self.max_script_depth = depth;
    }

    /// How deep in a `spawn script` chain this context already is — `0` for the
    /// request that started the tree, and see [`Self::script_depth`]'s field doc
    /// for why the number lives on a context rather than on an isolate.
    #[must_use]
    pub fn script_depth(&self) -> u32 {
        self.script_depth
    }

    /// `[limits] max_script_depth` as a count, or
    /// [`Self::DEFAULT_MAX_SCRIPT_DEPTH`] where the configuration does not state
    /// one.
    ///
    /// `false` — `rule:config/three-changeability-classes`'s spelling of no ceiling at all — is the one value
    /// that answers `0` and turns the net off. A malformed one takes the default
    /// instead, which is where this reader parts company with
    /// [`Self::configured_cpu_time`]; the field doc owns why. Either way the
    /// file was already parsed and refused at the boundary that could name the
    /// line, so this is not a second place to refuse it.
    ///
    /// A written `0` is a count and not that spelling, so it takes the default
    /// as well: this field holds `0` as its own sentinel for an absent ceiling,
    /// and reading a number an operator wrote as the sentinel would take the
    /// net off for the value they were most likely reaching for the opposite
    /// with. Nothing here can mean "no nesting at all" — the smallest ceiling
    /// is one level — so the safe direction is the only one left.
    ///
    fn configured_max_script_depth(&self) -> u32 {
        match self.limit("max_script_depth", nvs_config::Unit::Count) {
            nvs_config::Reading::Read(nvs_config::Quantity::Count(depth)) if depth != 0 => {
                u32::try_from(depth).unwrap_or(u32::MAX)
            }
            nvs_config::Reading::Read(nvs_config::Quantity::Unbounded) => 0,
            _ => Self::DEFAULT_MAX_SCRIPT_DEPTH,
        }
    }

    /// `[limits] fatal_reserve_memory` as bytes, or `None` where the
    /// configuration does not state it.
    ///
    /// A malformed value is `None` and takes the default below, for the reason
    /// [`Self::configured_memory_limit`] answers "no cap": the file was already
    /// parsed and refused at the boundary that can name the line.
    fn configured_fatal_reserve(&self) -> Option<usize> {
        match self.limit("fatal_reserve_memory", nvs_config::Unit::Bytes) {
            nvs_config::Reading::Read(nvs_config::Quantity::Bytes(bytes)) => {
                Some(usize::try_from(bytes).unwrap_or(0))
            }
            _ => None,
        }
    }

    /// `[limits] fatal_reserve_time` as nanoseconds, or `None` where the
    /// configuration does not state it.
    ///
    /// Malformed is `None` and takes the default below, for
    /// [`Self::configured_fatal_reserve`]'s reason.
    fn configured_fatal_reserve_time(&self) -> Option<u64> {
        match self.limit("fatal_reserve_time", nvs_config::Unit::Duration) {
            nvs_config::Reading::Read(nvs_config::Quantity::Nanos(nanos)) => Some(nanos),
            _ => None,
        }
    }

    /// `[log] handler_reserve_memory` as bytes, or `None` where the
    /// configuration does not state it.
    ///
    /// Malformed is `None` and takes [`Self::DEFAULT_HANDLER_RESERVE_MEMORY`],
    /// for [`Self::configured_fatal_reserve`]'s reason.
    pub(super) fn configured_handler_reserve(&self) -> Option<usize> {
        let written = self
            .config
            .as_ref()
            .and_then(|config| config.get("log.handler_reserve_memory"))?;
        let setting = nvs_config::Setting::Text(written);
        match nvs_config::Quantity::parse(
            "log.handler_reserve_memory",
            nvs_config::Unit::Bytes,
            &setting,
        ) {
            Ok(nvs_config::Quantity::Bytes(bytes)) => Some(usize::try_from(bytes).unwrap_or(0)),
            _ => None,
        }
    }

    /// `[log] handler_reserve_time` as nanoseconds, or `None` where the
    /// configuration does not state it.
    ///
    /// Malformed is `None` and takes [`Self::DEFAULT_HANDLER_RESERVE_TIME`], for
    /// the reader above's reason.
    pub(super) fn configured_handler_reserve_time(&self) -> Option<u64> {
        let written = self
            .config
            .as_ref()
            .and_then(|config| config.get("log.handler_reserve_time"))?;
        let setting = nvs_config::Setting::Text(written);
        match nvs_config::Quantity::parse(
            "log.handler_reserve_time",
            nvs_config::Unit::Duration,
            &setting,
        ) {
            Ok(nvs_config::Quantity::Nanos(nanos)) => Some(nanos),
            _ => None,
        }
    }

    /// The reserved slice of CPU time a request with this `ceiling` gets, given
    /// what its configuration asked for.
    ///
    /// **The default is 50 ms, and a quarter of the ceiling where a quarter is
    /// less** — the same shape as [`Self::reserve_within`] and for the same two
    /// reasons: enough for a handler to format a message and write it, and the
    /// clamp is what keeps a short ceiling from being mostly reserve rather than
    /// mostly program. `rule:errors/on-limit` names `fatal_reserve_time` and states no
    /// number; this is the number.
    ///
    /// 50 ms rather than the memory half's proportion of a typical ceiling,
    /// because the two slices are not sized by the same question. A handler's
    /// memory is bounded by what the message it builds costs, which is small and
    /// known; its *time* is bounded by what writing that message blocks on,
    /// which is a log target or a socket and is neither. So this is a wall-clock
    /// intuition about a slow write, floored well under the shortest ceiling
    /// anyone would set and clamped for the ones shorter still.
    ///
    /// An **asked-for** reserve is clamped the same way rather than refused, for
    /// [`Self::reserve_within`]'s reason: a reserve larger than the ceiling
    /// leaves ordinary execution nothing at all.
    fn reserve_time_within(ceiling: u64, asked: Option<u64>) -> u64 {
        if ceiling == 0 {
            return 0;
        }
        asked
            .unwrap_or(nvs_config::defaults::nanos(
                nvs_config::defaults::LIMITS_FATAL_RESERVE_TIME,
            ))
            .min(ceiling / 4)
    }

    /// The reserved slice a request with this `ceiling` gets, given what its
    /// configuration asked for.
    ///
    /// **The default is 1 MiB, and a quarter of the ceiling where a quarter is
    /// less** — enough for a handler to format a message and write it, and the
    /// clamp is what keeps a small ceiling from being mostly reserve rather
    /// than mostly program. `rule:errors/on-limit` states that the slice exists and that
    /// it is `System`-class, and states no number; this is the number, and an
    /// operator who wants another writes it.
    ///
    /// An **asked-for** reserve is clamped the same way for the same reason,
    /// and not refused: a reserve larger than the ceiling would leave ordinary
    /// execution nothing at all, which is a configuration that cannot run a
    /// program rather than one that runs it carefully.
    fn reserve_within(ceiling: usize, asked: Option<usize>) -> usize {
        if ceiling == 0 {
            return 0;
        }
        asked
            .unwrap_or(nvs_config::defaults::LIMITS_FATAL_RESERVE_MEMORY)
            .min(ceiling / 4)
    }

    /// `[limits] memory` as bytes. Where the configuration writes no such
    /// directive, `get` gives [`nvs_config::defaults::LIMITS_MEMORY`], for
    /// every kind of run. It is `0` (no cap) for `false` and for a context with
    /// no configuration at all.
    ///
    /// A value that is not a size reads as that default. The resolve refuses
    /// one at boot and `Core\Config::set` refuses one at run time, both through
    /// `nvs_config::Quantity`, so only a host that builds its snapshot by hand
    /// can hand one in (`rule:config/one-parser-for-the-boot-path-and-config-set`).
    fn configured_memory_limit(&self) -> usize {
        self.configured_bytes("memory")
    }

    /// `[limits] max_output` as bytes, or `0` when there is no configuration
    /// or no such directive, which is its shipped default.
    ///
    /// A value that is not a size reads as that default, for its sibling's
    /// reason above.
    fn configured_output_limit(&self) -> usize {
        self.configured_bytes("max_output")
    }

    /// `[limits] request_body` in force for this request: the most bytes of a
    /// body a reader may parse into memory
    /// (`rule:http-server/request-body-and-upload-total-are-two-caps`).
    ///
    /// The shipped default where there is no configuration or no usable value,
    /// never "no cap": this bound is what keeps a body out of memory.
    #[must_use]
    pub fn request_body_limit(&self) -> usize {
        match self.configured_bytes("request_body") {
            0 => usize::try_from(nvs_config::defaults::LIMITS_REQUEST_BODY).unwrap_or(usize::MAX),
            bytes => bytes,
        }
    }

    /// `[limits] upload_total` in force for this request: the most bytes of a
    /// body the wire may carry, streamed or parsed. The shipped default where
    /// there is no configuration or no usable value, for
    /// [`Self::request_body_limit`]'s reason.
    #[must_use]
    pub fn upload_total_limit(&self) -> u64 {
        match self.configured_bytes("upload_total") {
            0 => nvs_config::defaults::LIMITS_UPLOAD_TOTAL,
            bytes => u64::try_from(bytes).unwrap_or(u64::MAX),
        }
    }

    /// `[limits] max_multipart_parts` in force for this request: the most
    /// parts, field parts included, one multipart body may hold
    /// (`rule:errors/multipart-part-count`).
    ///
    /// The shipped default where there is no configuration or no usable value,
    /// for [`Self::request_body_limit`]'s reason. `false` reads as that default
    /// too: a body of unbounded parts is the shape the rule exists to stop.
    #[must_use]
    pub fn multipart_parts_limit(&self) -> usize {
        match self.limit("max_multipart_parts", nvs_config::Unit::Count) {
            nvs_config::Reading::Read(nvs_config::Quantity::Count(count)) => {
                usize::try_from(count).unwrap_or(usize::MAX)
            }
            _ => nvs_config::defaults::LIMITS_MAX_MULTIPART_PARTS,
        }
    }

    /// One `[limits]` directive read as a byte count — the arithmetic the
    /// readers above share.
    ///
    /// Written once rather than per ceiling because a size directive growing
    /// its own parse is how they would come to disagree about what
    /// `"32M"` means, and `nvs_config::Quantity` is the one place that question
    /// is answered (`rule:config/ini-set-is-core-config-set`
    /// ).
    ///
    /// A value that is not a size reads as the key's shipped default, which is
    /// what an unwritten one reads as, and never as no cap.
    fn configured_bytes(&self, key: &str) -> usize {
        match self.limit(key, nvs_config::Unit::Bytes) {
            nvs_config::Reading::Read(nvs_config::Quantity::Bytes(bytes)) => {
                usize::try_from(bytes).unwrap_or(usize::MAX)
            }
            nvs_config::Reading::Unset | nvs_config::Reading::Read(_) => 0,
            nvs_config::Reading::Malformed => {
                match nvs_config::defaults::row(&format!("limits.{key}")) {
                    Some(nvs_config::defaults::Builtin {
                        value: nvs_config::defaults::Value::Size(bytes),
                        ..
                    }) => usize::try_from(*bytes).unwrap_or(usize::MAX),
                    _ => 0,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context under the configuration `written` would give it, with its
    /// diagnostic stream buffered so a test can read what was written there.
    fn high_water_ctx(written: &str) -> Ctx {
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        let mut ctx = Ctx::buffered();
        ctx.set_diagnostic_sink(crate::OutputSink::Buffer(Vec::new()));
        ctx.set_config(std::sync::Arc::new(nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        }));
        ctx
    }

    fn diagnosed(ctx: &Ctx) -> String {
        match &ctx.diagnostic {
            crate::OutputSink::Buffer(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            _ => panic!("the case buffered the diagnostic stream"),
        }
    }

    /// A request that sets nothing reads every ceiling off the table its
    /// snapshot resolved once, so arming them makes no allocation and parses
    /// no text. The first read resolves the table, and the counted one is the
    /// next request's.
    #[test]
    fn a_request_that_sets_nothing_reads_its_limits_without_allocating() {
        let mut ctx = high_water_ctx("[limits]\nmemory = \"64M\"\nwall_time = \"30s\"\n");
        ctx.refresh_limits();
        let before = crate::counting_alloc::allocated_bytes();
        ctx.refresh_limits();
        let wall = ctx.configured_wall_time();
        let high_water = ctx.configured_memory_high_water();
        let made = crate::counting_alloc::allocated_bytes() - before;
        assert_eq!(made, 0, "reading the limits allocated {made} bytes");
        assert_eq!(ctx.memory_limit + ctx.fatal_reserve, 64 << 20);
        assert_eq!(wall, 30_000_000_000);
        assert_eq!(high_water, None);
    }

    /// `rule:observability/memory-high-water-writes-a-warn`: a written `0` is a
    /// threshold every request passes, so the end of the request writes one
    /// `Warn` naming the peak and the ceiling. The response gets none of it.
    #[test]
    fn a_peak_past_the_high_water_fraction_writes_one_warn() {
        let mut ctx = high_water_ctx("[limits]\nmemory = \"1G\"\nmemory_high_water = 0\n");
        ctx.report_memory_peak();
        let written = diagnosed(&ctx);
        assert_eq!(
            written.lines().count(),
            1,
            "one request wrote {written:?} rather than one record"
        );
        assert!(written.contains("\"warn\""), "{written}");
        assert!(written.contains("1073741824-byte ceiling"), "{written}");
        assert!(written.contains("no declared route"), "{written}");
        assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
    }

    /// The three ways the same rule writes nothing: no fraction written, a
    /// peak under the fraction, and a request with no ceiling for a fraction
    /// to be a share of.
    #[test]
    fn an_unwritten_fraction_a_low_peak_and_no_ceiling_write_nothing() {
        for written in [
            "[limits]\nmemory = \"1G\"\n",
            "[limits]\nmemory = \"1G\"\nmemory_high_water = 1.0\n",
            "[limits]\nmemory = false\nmemory_high_water = 0\n",
        ] {
            let mut ctx = high_water_ctx(written);
            ctx.report_memory_peak();
            assert_eq!(diagnosed(&ctx), "", "{written}");
        }
    }

    /// `rule:errors/on-limit`'s first resource limit, as far as this slice goes: the
    /// counter follows what the request holds *now*, so a breach that is
    /// released stops being one. What a breach then becomes is
    /// [`nvs_safepoint`]'s, not this test's.
    #[test]
    fn a_request_is_over_its_ceiling_only_while_it_still_holds_the_bytes() {
        let mut ctx = Ctx::buffered();
        ctx.set_memory_limit(1 << 20);
        assert!(!ctx.over_memory_limit());

        let held = vec![0_u8; 4 << 20];
        assert!(ctx.memory_used() >= 4 << 20);
        assert!(ctx.over_memory_limit());

        drop(held);
        assert!(!ctx.over_memory_limit());
    }

    /// A context nobody configured is uncapped, which is why every other test
    /// in this file allocates freely without arranging anything.
    #[test]
    fn a_context_with_no_configuration_has_no_ceiling() {
        let ctx = Ctx::buffered();
        assert_eq!(ctx.memory_limit(), 0);
        assert!(!ctx.over_memory_limit());
        assert_eq!(ctx.output_limit(), 0);
        assert!(!ctx.over_output_limit());
    }
}
