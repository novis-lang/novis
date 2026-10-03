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
    pub fn memory_used(&self) -> usize {
        self.memory_share(crate::budget::live_bytes())
    }

    /// This request's share of the thread's absolute balance `live`, read the
    /// way [`Self::memory_used`] reads the balance now.
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
    pub fn publish_off_core(&self) {
        let Some(share) = &self.tree_share else {
            return;
        };
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
    /// A request under no ceiling arms nothing at all: [`crate::budget`]'s
    /// sentinel is `0`, and short-circuiting on it is what keeps the uncapped
    /// case to one compare per allocation.
    pub(super) fn arm_memory_ceiling(&mut self) {
        crate::budget::arm(if self.memory_limit == 0 {
            crate::budget::Armed::NONE
        } else {
            // Absolute, and saturating for the reason [`Self::memory_used`]
            // floors at zero: the threshold is a balance rather than a size, so
            // a ceiling that cannot be added to this request's baseline is one
            // no allocation on this thread will reach.
            let ceiling = isize::try_from(self.memory_limit).unwrap_or(isize::MAX);
            crate::budget::Armed::new(self.memory_base.saturating_add(ceiling), self.safepoint)
        });
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
    /// Both sit behind the ceiling's own compare, which is what keeps an
    /// uncapped request to one: it can be refused nothing, because
    /// [`crate::budget`] arms it no threshold to be refused against.
    #[must_use]
    pub fn over_memory_limit(&self) -> bool {
        self.memory_limit != 0
            && (crate::budget::refused() || self.memory_used() > self.memory_limit)
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
    /// the program. After a refusal the reading is the one taken when the
    /// allocation was refused, followed by what it asked for
    /// ([`crate::budget::Refusal`] owns why): by the poll, a refused
    /// concatenation has already given back the text it was growing.
    #[must_use]
    pub fn memory_breach(&self) -> Option<crate::Fault> {
        // A member off its tree's root core publishes here and nowhere else —
        // [`Self::publish_off_core`] owns why the poll is the cadence, and this
        // is the poll every `Core` member passes through.
        self.publish_off_core();
        if !self.over_memory_limit() {
            return None;
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
        Some(crate::Fault::fatal(match crate::budget::refusal() {
            Some(refusal) => format!(
                "the request exceeded its memory limit — {} bytes held against a ceiling of {ceiling}{reserved}, when an allocation of {} bytes was refused",
                self.memory_share(refusal.live),
                refusal.asked,
            ),
            None => format!(
                "the request exceeded its memory limit — {} bytes held against a ceiling of {ceiling}{reserved}",
                self.memory_used(),
            ),
        }))
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
        // Read in the same pass and for the same reason, though there is nothing
        // to carve out of it: a request's ceilings are one reading of one
        // configuration.
        self.max_script_depth = self.configured_max_script_depth();
        // The response ceiling, in the same pass and for the same reason.
        // Nothing is carved out of it — [`Self::output_limit`]'s field doc owns
        // why a ceiling on writing needs no slice reserved from it.
        self.output_limit = self.configured_output_limit();
        // Last, because it mirrors what the lines above just decided: the
        // allocator is held to the ceiling ordinary execution is held to, and
        // arming it from the same pass is what stops the two from being left
        // disagreeing by a caller that moved one of them.
        self.arm_memory_ceiling();
    }

    /// `[limits] cpu_time` in nanoseconds, or `0` for a request under no cap.
    ///
    /// See [`Self::cpu_limit`]'s field doc for what the number measures. A
    /// malformed value answers "no cap" for the reason
    /// [`Self::configured_memory_limit`] does, and `false` — `rule:config/three-changeability-classes`'s
    /// spelling of no ceiling at all — answers the same `0`, because a request
    /// that may burn any amount of CPU and one whose ceiling nothing states are
    /// the same request to everything downstream.
    ///
    fn configured_cpu_time(&self) -> u64 {
        let Some(written) = self
            .config
            .as_ref()
            .and_then(|config| config.get("cpu_time"))
        else {
            return 0;
        };
        let setting = nvs_config::Setting::Text(written);
        match nvs_config::Quantity::parse("cpu_time", nvs_config::Unit::Duration, &setting) {
            Ok(nvs_config::Quantity::Nanos(nanos)) => nanos,
            _ => 0,
        }
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
    pub const DEFAULT_MAX_SCRIPT_DEPTH: u32 = 64;

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
        let Some(written) = self
            .config
            .as_ref()
            .and_then(|config| config.get("max_script_depth"))
        else {
            return Self::DEFAULT_MAX_SCRIPT_DEPTH;
        };
        let setting = nvs_config::Setting::Text(written);
        match nvs_config::Quantity::parse("max_script_depth", nvs_config::Unit::Count, &setting) {
            Ok(nvs_config::Quantity::Count(0)) => Self::DEFAULT_MAX_SCRIPT_DEPTH,
            Ok(nvs_config::Quantity::Count(depth)) => u32::try_from(depth).unwrap_or(u32::MAX),
            Ok(nvs_config::Quantity::Unbounded) => 0,
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
        let written = self
            .config
            .as_ref()
            .and_then(|config| config.get("fatal_reserve_memory"))?;
        let setting = nvs_config::Setting::Text(written);
        match nvs_config::Quantity::parse("fatal_reserve_memory", nvs_config::Unit::Bytes, &setting)
        {
            Ok(nvs_config::Quantity::Bytes(bytes)) => Some(usize::try_from(bytes).unwrap_or(0)),
            _ => None,
        }
    }

    /// `[limits] fatal_reserve_time` as nanoseconds, or `None` where the
    /// configuration does not state it.
    ///
    /// Malformed is `None` and takes the default below, for
    /// [`Self::configured_fatal_reserve`]'s reason.
    fn configured_fatal_reserve_time(&self) -> Option<u64> {
        let written = self
            .config
            .as_ref()
            .and_then(|config| config.get("fatal_reserve_time"))?;
        let setting = nvs_config::Setting::Text(written);
        match nvs_config::Quantity::parse(
            "fatal_reserve_time",
            nvs_config::Unit::Duration,
            &setting,
        ) {
            Ok(nvs_config::Quantity::Nanos(nanos)) => Some(nanos),
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
        asked.unwrap_or(50_000_000).min(ceiling / 4)
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
        asked.unwrap_or(1 << 20).min(ceiling / 4)
    }

    /// `[limits] memory` as bytes, or `0` when there is no configuration, no
    /// such directive, or a value that is not a size.
    ///
    /// A malformed value answers "no cap" rather than refusing here: the
    /// configuration was already parsed and refused once, at the boundary that
    /// can name the file and the line (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`
    /// ), and a second refusal from inside a running request could only be
    /// a worse-worded copy of it.
    fn configured_memory_limit(&self) -> usize {
        self.configured_bytes("memory")
    }

    /// `[limits] max_output` as bytes, or `0` when there is no configuration,
    /// no such directive, or a value that is not a size.
    ///
    /// Read the same way and answering "no cap" on a malformed value for the
    /// same reason its sibling above does.
    fn configured_output_limit(&self) -> usize {
        self.configured_bytes("max_output")
    }

    /// One `[limits]` directive read as a byte count — the arithmetic the
    /// readers above share.
    ///
    /// Written once rather than per ceiling because a size directive growing
    /// its own parse is how they would come to disagree about what
    /// `"32M"` means, and `nvs_config::Quantity` is the one place that question
    /// is answered (`rule:config/ini-set-is-core-config-set`
    /// ).
    fn configured_bytes(&self, key: &str) -> usize {
        let Some(written) = self.config.as_ref().and_then(|config| config.get(key)) else {
            return 0;
        };
        let setting = nvs_config::Setting::Text(written);
        match nvs_config::Quantity::parse(key, nvs_config::Unit::Bytes, &setting) {
            Ok(nvs_config::Quantity::Bytes(bytes)) => usize::try_from(bytes).unwrap_or(usize::MAX),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
