//! The user code a limit, an uncaught throw, a shutdown or the request's end
//! runs.
//!
//! The callables a context holds for the length of one request:
//! `rule:errors/handler-script`'s limit handler,
//! its uncaught handler, `Core\Signal::onShutdown`'s handler, the exit hooks a
//! program registers, and
//! `rule:concurrency/after-response-outlives-the-connection`'s
//! deferred work.
//!
//! They share a shape, which is why they share a file: each runs *after*
//! something outside the program's control has happened, so each is entered
//! from a place the program did not write, and each has to answer what happens
//! when the handler itself breaches. [`Ctx::run_limit_handler`] and
//! [`Ctx::abandon_exit_hook`] are the two places that answer it.
//!
//! [`Ctx::run_shutdown_handler`] is the one of them that is not a failure: the
//! request keeps running afterwards, so it widens no ceiling and reserves
//! nothing, and what it costs comes out of the budget the request still has.

use super::*;

impl Ctx {
    /// Takes ownership of the closure `Core\Fatal::onLimit` registered —
    /// `rule:errors/on-limit`'s
    /// tier 1.
    ///
    /// **Last registration wins, and there is no unregister but the request
    /// ending.** § 1 gives the tier one handler, not a chain: a ladder whose
    /// first rung ran an unbounded list of handlers out of one reserved slice
    /// would have to decide what a second handler sees after the first
    /// exhausted it, and "zero retries" is that section's answer to every such
    /// question. So a second call releases the first closure here, which is
    /// also what makes this the one place with both the reference and the
    /// request's lifetime in hand.
    ///
    /// The caller passes an **owned** reference; every `Core` helper's
    /// arguments are borrowed from the call frame, so the one in
    /// `nvs_stdlib::fatal` retains before it calls this.
    #[expect(
        unsafe_code,
        reason = "this context owned the reference it is replacing, having \
                  been handed it by exactly one earlier call"
    )]
    pub fn set_limit_handler(&mut self, handler: Value) {
        let previous = std::mem::replace(&mut self.limit_handler, handler);
        // SAFETY: `limit_handler` holds one owned reference or null, and
        // nothing else points at it — the field is private and handed out only
        // by the borrowing accessor below.
        unsafe { previous.release() };
    }

    /// The registered handler, **borrowed** — `null` when nothing registered
    /// one, which is every request that never called `Core\Fatal::onLimit`.
    ///
    /// No reference is handed over, exactly as [`Self::isolate_argument`] hands
    /// none over. The ladder calls through this rather than taking the value,
    /// because a breach does not end the registration: it is the request ending
    /// that does.
    #[must_use]
    pub fn limit_handler(&self) -> Value {
        self.limit_handler
    }

    /// Whether this request registered a tier-1 handler at all.
    ///
    /// The question the ladder asks first, and the reason it is a method rather
    /// than a comparison at each call site: a `null` slot is the encoding of
    /// "none", and nothing outside this file should know that.
    #[must_use]
    pub fn has_limit_handler(&self) -> bool {
        self.limit_handler.tag() != Some(crate::Tag::Null)
    }

    /// Takes ownership of the closure `Core\Fatal::onUncaughtThrow` registered
    /// — `rule:errors/on-uncaught-throw`'s
    /// tier 2.
    ///
    /// [`Self::set_limit_handler`]'s contract exactly, and deliberately: last
    /// registration wins, there is no unregister but the request ending, and
    /// the caller passes an **owned** reference because a `Core` helper's
    /// arguments are borrowed from a call frame this one outlives. The two
    /// tiers differ in what fires them and in what the handler is handed, never
    /// in how a registration is held.
    #[expect(
        unsafe_code,
        reason = "this context owned the reference it is replacing, having \
                  been handed it by exactly one earlier call"
    )]
    pub fn set_uncaught_handler(&mut self, handler: Value) {
        let previous = std::mem::replace(&mut self.uncaught_handler, handler);
        // SAFETY: `uncaught_handler` holds one owned reference or null, and
        // nothing else points at it — the field is private and never handed
        // out, since the only reader is `Self::run_uncaught_handler`.
        unsafe { previous.release() };
    }

    /// Whether this request registered a tier-2 handler at all.
    ///
    /// [`Self::has_limit_handler`]'s reason for being a method rather than a
    /// comparison: a `null` slot is the encoding of "none", and nothing outside
    /// this file should know that.
    #[must_use]
    pub fn has_uncaught_handler(&self) -> bool {
        self.uncaught_handler.tag() != Some(crate::Tag::Null)
    }

    /// Runs `rule:errors/on-uncaught-throw`'s
    /// tier 2 over `thrown`, if this request registered one.
    ///
    /// **The handler is handed the real exception object**, not a report built
    /// from it, which is the one way this differs from
    /// [`Self::run_limit_handler`]'s array. § 2 says why: this is the request's
    /// own root rather than an isolate boundary
    /// `rule:security/isolate-shares-nothing` has to
    /// copy across, so the object the program threw is still the object it
    /// threw, with its own class, message and backtrace reachable by the
    /// ordinary members. A handler declaring no parameter still runs, for
    /// [`Self::run_limit_handler`]'s reason.
    ///
    /// **No ceiling moves.** § 2 has no reserved slice — see
    /// [`Self::uncaught_handler`] — so a request that reached the root with its
    /// budget nearly spent runs this handler out of what is left, and a handler
    /// that exhausts it breaches like any other code.
    ///
    /// **The registration is taken out of the slot on the way in**, exactly as
    /// [`Self::run_limit_handler`] takes tier 1's: that is the whole of "zero
    /// retries" here too, since a handler that throws reaches an isolate root
    /// of its own inside [`crate::script`] and would otherwise find itself.
    ///
    /// Whatever the handler leaves behind is dropped, and a throw or a fault of
    /// its own is abandoned where it stands — § 3's "handler faulted" drops to
    /// tier 3, and what tier 3 is handed is still the throw that got here. The
    /// pending status is cleared for that reason: the request reports the
    /// failure that reached the root, never the one its reporter had.
    ///
    /// **Running does not suppress the tiers below.** § 3 is the shared
    /// catch-all and the floor beneath it is § 6's record of a request that
    /// died, so an operator's pipeline does not lose one because the
    /// application registered a handler — which is also how tier 1 already
    /// behaves, since every caller of [`Self::run_limit_handler`] records its
    /// breach afterwards regardless.
    #[expect(
        unsafe_code,
        reason = "this context owned the reference it just took out of the \
                  slot, and owns the answer the call produced"
    )]
    pub fn run_uncaught_handler(&mut self, thrown: &Thrown) {
        if !self.has_uncaught_handler() {
            return;
        }
        // `Value::default()` is the null this leaves behind, which is the
        // encoding of "nothing registered" `Self::has_uncaught_handler` reads.
        let handler = std::mem::take(&mut self.uncaught_handler);
        // Borrowed: the reference keeping the object alive across the call is
        // the caller's `Thrown`, and `crate::call_closure` takes one of its own
        // for the callee to release.
        let answer = crate::call_closure(self, handler, &[thrown.as_value()]);
        // Zero retries, and the failure the request reports is the one that
        // reached the root — so a handler's own throw ends here rather than
        // travelling on as this request's status.
        drop(self.take_pending());
        // SAFETY: the slot held one owned reference, which this frame now
        // holds. An `Ok` answer is a fresh value this frame owns, and releasing
        // a `null` — which is what a `void` closure returns — is a no-op. The
        // exception itself is not released here: this frame never owned a
        // reference to it.
        unsafe {
            if let Ok(answer) = answer {
                answer.release();
            }
            handler.release();
        }
    }

    /// Appends `hook` to
    /// `rule:observability/script-on-exit`
    /// 's end-of-script queue — what `Core\Script::onExit` does, which is
    /// register and run nothing.
    ///
    /// The caller passes an **owned** reference, exactly as
    /// [`Self::set_limit_handler`] takes one and for the same reason: a `Core`
    /// helper's arguments are borrowed from a call frame this registration
    /// outlives. Nothing is replaced and nothing is refused — see
    /// [`Self::exit_hooks`] for why a queue rather than a slot, and
    /// [`Self::run_exit_hooks`] for what a registration made *during* the drain
    /// joins.
    pub fn push_exit_hook(&mut self, hook: Value) {
        self.exit_hooks.push(hook);
    }

    /// How many hooks the end-of-script queue holds — what a test asserts a
    /// registration against, and the reason [`Self::exit_hooks`] is private.
    #[must_use]
    pub fn exit_hook_count(&self) -> usize {
        self.exit_hooks.len()
    }

    /// Whether the queue has already had its one drain — `rule:observability/three-endings-fire-the-exit-queue`.
    #[must_use]
    pub fn exit_hooks_drained(&self) -> bool {
        self.exit_hooks_drained
    }

    /// Runs the end-of-script queue FIFO, handing each hook `report` — `rule:observability/three-endings-fire-the-exit-queue` and `rule:observability/a-hook-observes-and-never-steers`
    /// .
    ///
    /// **Which endings reach here is not this method's question.** § 3's `FATAL`
    /// and cancellation never fire the queue, and the one place that decides is
    /// `nvs_stdlib::script::run_exit_hooks`, which is also where the report is
    /// built — a `Core` instance is that crate's to lay out. This end owns the
    /// queue and its ordering rules, and nothing else.
    ///
    /// **Once.** A second call runs nothing, however it is reached: § 2 says the
    /// queue runs at most once per script, and a drain that reached an ending
    /// twice would be a second ending the report was never fixed for.
    ///
    /// **A hook registered by a hook joins the tail of the same drain**, which
    /// is why this walks by index instead of taking the vec: § 5 names that
    /// case, and a queue drained into a local would silently drop it.
    ///
    /// `report` is **borrowed** — the caller keeps the only reference and every
    /// hook is handed the same object, so all of them observe one report rather
    /// than one each.
    ///
    /// Whatever a hook answers is dropped, and a hook that fails is abandoned
    /// where it stands with the queue continuing — [`Self::abandon_exit_hook`]
    /// is the home of § 5's failure readings.
    #[expect(
        unsafe_code,
        reason = "the queue owns one reference per registration and this is the \
                  frame that gives every one of them back, plus whatever each \
                  hook answered"
    )]
    pub fn run_exit_hooks(&mut self, report: Value) {
        if self.exit_hooks_drained {
            return;
        }
        self.exit_hooks_drained = true;
        let mut index = 0;
        while index < self.exit_hooks.len() {
            let hook = self.exit_hooks[index];
            index += 1;
            match crate::call_closure(self, hook, &[report]) {
                // SAFETY: an `Ok` answer is a fresh value this frame owns, and
                // releasing the `null` a `void` closure answers is a no-op.
                Ok(answer) => unsafe { answer.release() },
                Err(fault) => {
                    if !self.abandon_exit_hook(&fault) {
                        break;
                    }
                }
            }
        }
        // SAFETY: the queue holds exactly one reference per registration and
        // nothing else points at it — the hooks are the caller's only through
        // `Self::push_exit_hook`, which hands its reference over.
        for hook in std::mem::take(&mut self.exit_hooks) {
            unsafe { hook.release() };
        }
    }

    /// Reports one failed exit hook and answers whether the drain continues —
    /// `rule:observability/a-hook-observes-and-never-steers`.
    ///
    /// The readings § 5 gives, and only the last stops the queue:
    ///
    /// - **A throw** is written to the same record `Core\Log` writes, through
    ///   [`crate::floor`], and abandoned — § 5's "logged with the request's
    ///   trace id rather than swallowed", which is
    ///   `rule:concurrency/nothing-is-still-running-when-a-call-returns`
    ///   's rule for a second throw and [`crate::deferred`]'s reading of it
    ///   for after-response work.
    /// - **An `exit`** is § 5's refusal: a hook that could end the script would
    ///   suppress every hook behind it, so the status it named is dropped and
    ///   the `RuntimeError` that section names is reported in its place.
    /// - **A `Core\Script::finish()`** is that refusal for the same reason and
    ///   with more to suppress: the ending is already fixed, and the queue *is*
    ///   what a finish delays the end of, so a hook allowed to raise the marker
    ///   would cut short the very drain it is running inside. It arrives as a
    ///   throw rather than as a status of its own — [`crate::is_finish`] is what
    ///   separates the two — and leaves the same `RuntimeError` behind.
    /// - **A `FATAL`** is the one that stops the drain. § 5's last sentence: a
    ///   limit breach inside a hook is a `FATAL` like any other, the ladder
    ///   takes over and the rest of the queue never runs — so the pending state
    ///   is left exactly as the breach recorded it.
    fn abandon_exit_hook(&mut self, fault: &crate::Fault) -> bool {
        match fault {
            crate::Fault::Pending(status) if *status == crate::FATAL => return false,
            crate::Fault::Pending(status) if *status == crate::EXITED => self.set_pending(
                "`exit` inside a `Core\\Script::onExit` hook: a hook observes the ending it was \
                 given and cannot choose another",
            ),
            crate::Fault::Pending(status)
                if *status == crate::THROWN
                    && self
                        .pending_class()
                        .as_deref()
                        .is_some_and(crate::is_finish) =>
            {
                // The marker is released here rather than left for the take
                // below: `set_pending` overwrites the slot and releases nothing
                // it displaces, so the object the hook raised would otherwise
                // outlive every reference to it.
                drop(self.take_thrown());
                self.set_pending(
                    "`Core\\Script::finish()` inside a `Core\\Script::onExit` hook: a hook \
                     observes the ending it was given and cannot choose another",
                );
            }
            // The callee already recorded what failed; that is the whole of what
            // this variant means.
            crate::Fault::Pending(_) => {}
            crate::Fault::Thrown(class, message) => self.set_pending_as(*class, message.clone()),
            // `Fault` is `#[non_exhaustive]`, and the remaining variants reach
            // a closure call only as `crate::call_closure`'s own two engine
            // faults — a value that is not a closure, or one declaring more
            // parameters than the one report there is to offer.
            other => self.set_pending(format!("a `Core\\Script::onExit` hook failed: {other:?}")),
        }
        let thrown = self.take_thrown();
        let mut record = crate::floor::uncaught(&thrown);
        record
            .envelope
            .fields
            .push(("origin".to_owned(), crate::floor::text("exit-hook")));
        crate::floor::report(self, &record);
        true
    }

    /// Registers `closure` to run once this request's own frame has returned —
    /// `rule:concurrency/after-response-outlives-the-connection`
    /// , and [`mod@crate::deferred`] owns when that is on a host with no
    /// response.
    ///
    /// The caller passes an **owned** reference, exactly as
    /// [`Self::set_limit_handler`] takes one and for the same reason: a
    /// helper's arguments are borrowed from the call frame and this one
    /// outlives it. A refused registration hands the reference back by leaving
    /// it with the caller, which is then what releases it.
    ///
    /// `deadline_nanos` is § 7's `deadline` already resolved — the option the
    /// call named, or [`Self::deferred_deadline`] — and `0` is no deadline.
    ///
    /// **Two refusals, and the caller keeps the reference on both.**
    /// [`crate::deferred::DeferError::Sealed`] is § 6's last bullet — this
    /// context is a child, and deferred work may not defer more.
    /// [`crate::deferred::DeferError::AtCapacity`] is § 7's cap, asked here
    /// because this is where a *tree* first becomes one of the ones a core is
    /// holding open. Both become a `RuntimeError` at the call site, while there
    /// is still a request to decide what to do about it.
    ///
    /// # Errors
    ///
    /// The two above, and nothing else: a registration that gets past them is
    /// queued.
    pub fn defer(
        &mut self,
        closure: Value,
        deadline_nanos: u64,
    ) -> Result<(), crate::deferred::DeferError> {
        if self.deferred.is_none() {
            return Err(crate::deferred::DeferError::Sealed);
        }
        // Only the first registration takes a slot: what the cap counts is
        // trees, so a request that defers twenty closures is one tree held open
        // exactly as a request that defers one is.
        if !self.holds_deferred_slot {
            let cap = self.deferred_max_concurrent();
            if !crate::deferred::take_tree_slot(cap) {
                return Err(crate::deferred::DeferError::AtCapacity { cap });
            }
            self.holds_deferred_slot = true;
        }
        self.deferred
            .as_mut()
            .expect("the queue was there a line ago")
            .push(crate::deferred::Deferred {
                closure,
                deadline_nanos,
            });
        Ok(())
    }

    /// Gives back the slot this tree holds against § 7's cap, if it has one.
    ///
    /// Called at the end of the drain ([`crate::deferred::run_deferred`]) and
    /// once more as the context goes down, for the tree that never drained at
    /// all. Idempotent, because those two are not exclusive: a request that ran
    /// its work still reaches its own teardown afterwards.
    pub(crate) fn release_deferred_slot(&mut self) {
        if self.holds_deferred_slot {
            self.holds_deferred_slot = false;
            crate::deferred::give_back_tree_slot();
        }
    }

    /// Whether this request registered any after-response work.
    #[must_use]
    pub fn has_deferred(&self) -> bool {
        self.deferred
            .as_ref()
            .is_some_and(|queue| !queue.is_empty())
    }

    /// Takes the whole queue and **seals** it, so nothing registered afterwards
    /// extends the drain — [`mod@crate::deferred`]'s *the queue is a leaf*.
    ///
    /// Each entry carries one reference the caller then owes; both callers are
    /// in this crate, which is why this hands the values out at all rather than
    /// running them here.
    pub(crate) fn take_deferred(&mut self) -> Vec<crate::deferred::Deferred> {
        self.deferred.take().unwrap_or_default()
    }

    /// § 7's printed default for [`Self::deferred_max_concurrent`], for a tree
    /// that writes no `[deferred] max_concurrent` of its own.
    const DEFAULT_DEFERRED_MAX_CONCURRENT: u64 = 256;

    /// `[deferred] max_concurrent` — how many request trees this core may hold
    /// open for after-response work at once (§ 7).
    ///
    /// 256 where nothing is written, which is the number `rule:concurrency/deferred-is-bounded-by-two-directives` prints
    /// beside the directive. A written `0` is honoured rather than corrected:
    /// it says this deployment does not want after-response work at all, and
    /// the refusal a call then gets names the directive it would have to
    /// change.
    #[must_use]
    pub fn deferred_max_concurrent(&self) -> u64 {
        let Some(written) = self
            .config
            .as_ref()
            .and_then(|config| config.get("deferred.max_concurrent"))
        else {
            return Self::DEFAULT_DEFERRED_MAX_CONCURRENT;
        };
        let setting = nvs_config::Setting::Text(written);
        match nvs_config::Quantity::parse(
            "deferred.max_concurrent",
            nvs_config::Unit::Count,
            &setting,
        ) {
            Ok(nvs_config::Quantity::Count(count)) => count,
            _ => Self::DEFAULT_DEFERRED_MAX_CONCURRENT,
        }
    }

    /// `[deferred] deadline` in nanoseconds, or `0` when the tree names none —
    /// the default a call that names no `deadline` of its own inherits (§ 7).
    #[must_use]
    pub fn deferred_deadline(&self) -> u64 {
        let Some(written) = self
            .config
            .as_ref()
            .and_then(|config| config.get("deferred.deadline"))
        else {
            return 0;
        };
        let setting = nvs_config::Setting::Text(written);
        match nvs_config::Quantity::parse("deferred.deadline", nvs_config::Unit::Duration, &setting)
        {
            Ok(nvs_config::Quantity::Nanos(nanos)) => nanos,
            _ => 0,
        }
    }

    /// Runs `rule:errors/on-limit`'s tier-1 handler, if this request registered one — the last thing a
    /// program gets to do about a resource limit, and it happens *before* the
    /// breach is recorded as the `FATAL` the ladder goes on to print.
    ///
    /// **The slot is cleared before the call and not after, and that is the
    /// whole of "zero retries".** A handler runs with the limit still breached,
    /// so it reaches this ladder again from inside itself at its first helper
    /// call ([`crate::run_helper`]) or its first safepoint poll
    /// ([`nvs_safepoint`]); both ask [`Self::has_limit_handler`] first, so
    /// taking the registration out of the slot on the way in is what makes that
    /// second breach find nothing and fall straight through to the next tier.
    /// Expressing the rule as an ownership move rather than as a flag is what
    /// stops it from depending on any path remembering to unset one.
    ///
    /// Whatever the handler leaves behind is dropped here. It answers nothing
    /// by its signature, and a throw or a fault of its own is abandoned where
    /// it stands: the breach that got here is what the request reports, so the
    /// caller records *its* fault after this returns, over any pending status
    /// the handler set.
    ///
    /// It is handed § 1's `LimitReport`: one array, whose `limit` key names the
    /// limit that stopped the request in the spelling [`Limit::name`] owns. A
    /// handler declaring no parameter still runs — [`crate::call_closure`] trims
    /// the call to the arity the closure recorded — so the report costs nothing
    /// to a program that does not read it beyond the allocations building it.
    #[expect(
        unsafe_code,
        reason = "this context owned the reference it just took out of the \
                  slot, owns the report it built, and owns the answer the call \
                  produced"
    )]
    pub fn run_limit_handler(&mut self, limit: Limit) {
        if !self.has_limit_handler() {
            return;
        }
        // `Value::default()` is the null this leaves behind, which is the
        // encoding of "nothing registered" [`Self::has_limit_handler`] reads.
        let handler = std::mem::take(&mut self.limit_handler);
        // § 1's reserved slice, added back for exactly the length of the call,
        // and both halves of it are added back whichever limit got here: a
        // handler stopped by the CPU-time flag still allocates to say so, and
        // one stopped by the memory cap still takes time to write it.
        // The handler is entered with the ceiling already breached, so without
        // this it could not allocate a byte or reach a single `Core` member —
        // every one of them asks [`crate::run_helper`]'s question first — and a
        // tier that can say nothing is not a tier. Restored afterwards because
        // the reserve is the handler's and not the request's: what the ladder
        // records next is the breach ordinary execution reached.
        let ordinary = self.memory_limit;
        let reserve = self.fatal_reserve;
        if ordinary != 0 {
            self.memory_limit = ordinary.saturating_add(reserve);
            // Spent, not merely lent: a handler that breaches *again* is one
            // that exhausted the whole budget, and the message it fails with
            // should say so rather than name a slice still being held for it.
            self.fatal_reserve = 0;
        }
        // The ceiling above is a number two readers share, and the allocator is
        // the one that would otherwise be left holding the old one: it refuses
        // against the armed threshold rather than against this field, so a
        // handler lent the reserve here and refused every allocation there is a
        // tier that says nothing. [`Self::arm_memory_ceiling`] is the mirror,
        // and it is put back with the ceiling at the foot of this call.
        self.arm_memory_ceiling();
        // The verdict half of the same slice, and the reason a wider ceiling is
        // not enough on its own. A request stopped by a *refusal* holds fewer
        // bytes than its ceiling — the ones it asked for were never handed over
        // — so what stops it is a flag rather than a reading, and a handler
        // entered still carrying it would be stopped by it at the first `Core`
        // member it reached, which is [`crate::run_helper`]'s question. Taken
        // for the length of the call exactly as the CPU flag below is, and put
        // back for the same reason the reserve is: what the ladder records next
        // is the breach ordinary execution reached.
        let refused = crate::budget::take_refusal();
        // The third half of the same slice, and the one a widened ceiling and a
        // taken verdict together still do not buy. The pre-check in front of an
        // allocation refuses against the *balance*, which a request that
        // reached here is already past — so without this the report below is an
        // array of empty strings and the handler cannot allocate the one byte
        // it needs to read it, which is the tier saying nothing by another
        // route. What bounds the slice instead is the counting ceiling: the
        // widened threshold is compared behind every allocation, and § 1's
        // zero-retry rule is what a handler overrunning it meets.
        let _reserve = crate::budget::Reporting::begin();
        // The CPU half, and the reason it is a *flag* edit as well as a ceiling
        // edit. A handler entered under [`SafepointFlags::CPU_LIMIT`] would be
        // stopped again by the very flag it was entered under, at its own first
        // back edge, before anything raised it a second time — so the slice a
        // ceiling on its own buys is zero wide however many nanoseconds it
        // names. Lowering the flag for the length of the call is what makes the
        // slice `fatal_reserve_time` wide instead: the timer watching this
        // request re-raises it when the thread's clock passes the widened
        // ceiling, which is exactly the handler overrunning its slice, and § 1's
        // zero-retry rule already says what happens to one that does.
        //
        // Only what was lowered is raised again. A handler reached by the memory
        // branch never had the flag set, and setting it on the way out would
        // stop the *next* poll of a request that never went near its CPU
        // ceiling.
        let cpu_ordinary = self.cpu_limit;
        let cpu_reserve = self.fatal_reserve_time;
        let stopped_for_cpu = self.safepoint_flags().contains(SafepointFlags::CPU_LIMIT);
        if cpu_ordinary != 0 {
            self.cpu_limit = cpu_ordinary.saturating_add(cpu_reserve);
            // Spent, not merely lent, for [`Self::fatal_reserve`]'s reason.
            self.fatal_reserve_time = 0;
        }
        if stopped_for_cpu {
            self.lower_safepoint(SafepointFlags::CPU_LIMIT);
        }
        // Built here rather than by either caller, and *after* the reserve is
        // in force: it allocates, and a report the ladder could not afford to
        // build would be a tier that says nothing for the same reason a handler
        // that cannot allocate is. A widened ceiling is not enough on its own
        // for the same reason the verdict above is not — the pre-check in front
        // of every string this array holds refuses against the balance, which a
        // request that reached here is already past. [`crate::budget::Reporting`]
        // is what the *runtime's* allocations are lent; the handler's own body
        // runs after it is closed and is measured like any other program.
        let mut report = crate::NvsArray::new();
        report.set(
            crate::NvsStr::new(b"limit"),
            Value::str(crate::NvsStr::new(limit.name().as_bytes())),
        );
        let report = Value::array(report);
        let answer = crate::call_closure(self, handler, &[report]);
        self.memory_limit = ordinary;
        self.fatal_reserve = reserve;
        self.arm_memory_ceiling();
        crate::budget::restore_refusal(refused);
        self.cpu_limit = cpu_ordinary;
        self.fatal_reserve_time = cpu_reserve;
        if stopped_for_cpu {
            self.request_safepoint(SafepointFlags::CPU_LIMIT);
        }
        // SAFETY: the slot held one owned reference, which this frame now
        // holds; `call_closure` took its own of every slot for the callee to
        // release, so the report's reference here is still this frame's however
        // the call went. An `Ok` answer is a fresh value this frame owns, and
        // releasing a `null` — which is what a `void` closure returns — is a
        // no-op.
        unsafe {
            if let Ok(answer) = answer {
                answer.release();
            }
            report.release();
            handler.release();
        }
    }

    /// Takes ownership of the closure `Core\Signal::onShutdown` registered —
    /// what this request runs when the process is asked to stop.
    ///
    /// [`Self::set_limit_handler`]'s contract exactly, and for its reasons:
    /// last registration wins, there is no unregister but the request ending,
    /// and the caller passes an **owned** reference because a `Core` helper's
    /// arguments are borrowed from a call frame this one outlives.
    #[expect(
        unsafe_code,
        reason = "this context owned the reference it is replacing, having \
                  been handed it by exactly one earlier call"
    )]
    pub fn set_shutdown_handler(&mut self, handler: Value) {
        let previous = std::mem::replace(&mut self.shutdown_handler, handler);
        // SAFETY: `shutdown_handler` holds one owned reference or null, and
        // nothing else points at it — the field is private and this file is the
        // only place that writes it.
        unsafe { previous.release() };
    }

    /// Whether this request registered a shutdown handler at all.
    ///
    /// [`Self::has_limit_handler`]'s question over the third slot, and a method
    /// for its reason: a `null` slot is the encoding of "none", and nothing
    /// outside this file should know that.
    #[must_use]
    pub fn has_shutdown_handler(&self) -> bool {
        self.shutdown_handler.tag() != Some(crate::Tag::Null)
    }

    /// Runs the registered shutdown handler, once, and lets the request carry
    /// on.
    ///
    /// **This is the whole of what a signal delivery does to Novis code.** A
    /// delivery raises [`SafepointFlags::SHUTDOWN`] and returns; the handler
    /// itself is entered from [`nvs_safepoint`] between two Novis statements,
    /// as ordinary code on the request's own stack, with every `Core` member
    /// reachable and nothing about the frame it runs in a signal context.
    ///
    /// **Once, because the slot is taken rather than borrowed.** A drain begins
    /// once per process, so a second delivery has nothing left to say, and a
    /// handler re-entered at its own first back edge would be the CPU-limit
    /// mistake [`Self::run_limit_handler`] lowers a flag to avoid.
    ///
    /// No reserve, no widened ceiling and no report: the request has not
    /// breached anything and is not ending, so the handler runs under what the
    /// request still has, exactly as [`Self::run_uncaught_handler`] does and for
    /// a stronger version of the same reason. A handler that throws or that
    /// exhausts the budget is stopped by the ladder that already owns those two
    /// answers.
    #[expect(
        unsafe_code,
        reason = "this context owned the reference it just took out of the \
                  slot, and owns the answer the call produced"
    )]
    pub fn run_shutdown_handler(&mut self) {
        if !self.has_shutdown_handler() {
            return;
        }
        // `Value::default()` is the null this leaves behind, which is the
        // encoding of "nothing registered" [`Self::has_shutdown_handler`] reads.
        let handler = std::mem::take(&mut self.shutdown_handler);
        let answer = crate::call_closure(self, handler, &[]);
        // SAFETY: the slot held one owned reference, which this frame now
        // holds; `call_closure` took its own for the callee to release, so this
        // frame's is still this frame's however the call went. An `Ok` answer is
        // a fresh value this frame owns, and releasing a `null` — which is what
        // a `void` closure returns — is a no-op.
        unsafe {
            if let Ok(answer) = answer {
                answer.release();
            }
            handler.release();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{ClassTable, MethodRow, NvsObj};

    thread_local! {
        /// The temporary directory the hook below is asked about — the one the
        /// script was handed and the one its context will sweep.
        static WATCHED: std::cell::RefCell<Option<std::path::PathBuf>> =
            const { std::cell::RefCell::new(None) };
        /// What the hook found there, or `None` if it never ran at all — which
        /// is a distinct failure from finding the directory already gone.
        static STILL_STANDING: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
    }

    /// The `Core\Script::onExit` hook the case below registers: it looks for
    /// the temporary directory the script was handed and records whether it was
    /// still standing when the queue ran.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver and each parameter live, \
                  every one of them retained for this callee to release, and \
                  the address of a live `Value` for the result — neither is \
                  expressible in the signature compiled code calls through"
    )]
    unsafe extern "C" fn looks_for_the_directory(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        // The receiver and the one report parameter, given back exactly as a
        // compiled callee's exit sweep gives them back.
        for slot in 0..2 {
            // SAFETY: `call_closure` passed two live values and retained each.
            unsafe { (*args.add(slot)).release() };
        }
        let standing = WATCHED.with_borrow(|watched| {
            watched
                .as_ref()
                .expect("the case named a directory before running the queue")
                .is_dir()
        });
        STILL_STANDING.with(|seen| seen.set(Some(standing)));
        // SAFETY: the caller passed the address of a live `Value` to answer
        // into, and `void` is a `null` there.
        unsafe { *out = Value::null() };
        crate::abi::OK
    }

    /// A closure value declaring one parameter whose `invoke` is a plain Rust
    /// function.
    ///
    /// [`crate::call_closure`] reads only the arity slot, the parameter tags
    /// slot, and the `invoke` method's address in its class off a closure — so
    /// a test needs no compiler in front of it to register a hook. Everything else in `nvs_ir::lower::lower_closure`'s representation
    /// is captured state, and a native callback captures nothing.
    ///
    /// The table is leaked because a descriptor's *address* is its identity and
    /// it must outlive every instance made from it; the test process exiting is
    /// what reclaims it.
    fn hook_of(invoke: crate::abi::NvsFn) -> Value {
        let mut table = ClassTable::new();
        let id = table.define("{closure}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![MethodRow {
                name: crate::closure::CLOSURE_INVOKE.to_owned(),
                code: invoke as *const u8,
                // Read off the object's own two slots below rather than off
                // this row — `crate::call_closure` says so.
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                public: true,
                native: false,
            }],
        );
        table.set_closure(id);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives \
                      every instance made from it — `NvsObj::new`'s whole \
                      obligation"
        )]
        let object = unsafe { NvsObj::new(table.desc(id)) };
        object.set_field(crate::closure::CLOSURE_ARITY_SLOT, Value::int(1));
        object.set_field(
            crate::closure::CLOSURE_PARAM_TAGS_SLOT,
            Value::int(i64::from(crate::closure::CLOSURE_PARAM_TAG_ANY)),
        );
        Value::object(object)
    }

    /// `rule:core-classes/temporary-dir-sweep`'s ordering against `rule:observability/script-on-exit`'s
    /// queue, asserted from the one side that can observe it: the last user
    /// code still finds the directory it was handed, and the teardown behind it
    /// is what takes it away.
    ///
    /// Both halves are the test. A sweep that ran *before* the queue would
    /// leave the hook looking at nothing, and a sweep that never ran would
    /// leave the directory standing after the context is gone — either failure
    /// alone passes an assertion that names only the other.
    #[test]
    fn the_sweep_runs_after_the_on_exit_queue() {
        let standing = std::env::temp_dir().join(format!(
            "nvs-exit-hook-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&standing).expect("the platform root is writable");
        WATCHED.with_borrow_mut(|watched| *watched = Some(standing.clone()));

        let mut ctx = Ctx::buffered();
        ctx.track_temporary_dir(standing.clone());
        ctx.push_exit_hook(hook_of(looks_for_the_directory));
        ctx.run_exit_hooks(Value::null());

        assert_eq!(
            STILL_STANDING.with(std::cell::Cell::get),
            Some(true),
            "a hook is user code, and § 3 puts the sweep after all of it"
        );

        drop(ctx);
        assert!(
            !standing.exists(),
            "and the context's teardown, which is every ending at once, is what sweeps it"
        );
    }

    thread_local! {
        /// How many times [`overruns_its_slice`] has been entered. A second
        /// entry is the retry [`Ctx::run_limit_handler`] takes the slot out to
        /// make impossible, and a count is the only thing that can see one.
        static ENTRIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        /// Whether [`SafepointFlags::CPU_LIMIT`] was still raised when the
        /// handler was entered. `true` is a slice zero nanoseconds wide however
        /// wide the ceiling beside it was made, and it is also the reading of a
        /// handler that never ran.
        static FLAG_ON_ENTRY: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
        /// The CPU ceiling in force inside the handler — the widened one, which
        /// is the half of the slice a flag edit on its own does not buy.
        static SEEN_CEILING: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
        /// What the handler's own poll answered once the flag was raised on it
        /// again. `None` is a poll that was never made.
        static OVERRUN_STATUS: std::cell::Cell<Option<i32>> = const { std::cell::Cell::new(None) };
    }

    /// A limit handler that spends its whole widened slice: it reads what it
    /// was entered with, raises the CPU flag on itself the way the watchdog
    /// does when the thread's clock passes the widened ceiling, and then reaches
    /// a back edge.
    ///
    /// The status it answers is the poll's, because a handler stopped by a
    /// `FATAL` is one that returns it — there is nothing else it may do.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver and each parameter live, \
                  every one of them retained for this callee to release, and \
                  the address of a live `Value` for the result — plus the \
                  context pointer compiled code is called with, live for this \
                  call by the ABI it arrives under"
    )]
    unsafe extern "C" fn overruns_its_slice(
        ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        // The receiver and the one report parameter, given back exactly as
        // [`looks_for_the_directory`] gives them back.
        for slot in 0..2 {
            // SAFETY: `call_closure` passed two live values and retained each.
            unsafe { (*args.add(slot)).release() };
        }
        ENTRIES.with(|entries| entries.set(entries.get() + 1));
        {
            // SAFETY: the caller passed a context valid for this call.
            let ctx = unsafe { &mut *ctx };
            FLAG_ON_ENTRY
                .with(|flag| flag.set(ctx.safepoint_flags().contains(SafepointFlags::CPU_LIMIT)));
            SEEN_CEILING.with(|ceiling| ceiling.set(ctx.cpu_limit()));
            ctx.request_safepoint(SafepointFlags::CPU_LIMIT);
        }
        // SAFETY: the same context, and the borrow above ends here.
        let status = unsafe { crate::nvs_safepoint(ctx) };
        OVERRUN_STATUS.with(|seen| seen.set(Some(status)));
        // SAFETY: the caller passed the address of a live `Value` to answer
        // into, and a handler carrying a `FATAL` out answers nothing.
        unsafe { *out = Value::null() };
        status
    }

    /// `rule:errors/on-limit`'s zero-retry rule, asked of the handler that
    /// nothing else can stop: a request stopped by the CPU-time flag runs its
    /// handler with that flag lowered and its ceiling widened, and a handler
    /// that spends the widened ceiling too is stopped rather than started
    /// again.
    ///
    /// Four readings are the test, because the failure each one names passes an
    /// assertion over the others. A flag still raised on entry is a slice zero
    /// nanoseconds wide, whatever the ceiling beside it says; a ceiling that is
    /// still the ordinary one is a slice nothing consults. An entry count of
    /// two is the handler restarted by its own breach. And a poll inside the
    /// handler answering `OK` is a handler that overran and was let carry on.
    #[test]
    fn the_limit_handler_runs_once_and_is_not_re_entered_when_it_overruns() {
        let mut ctx = Ctx::buffered();
        ctx.set_cpu_limit(1_000);
        ctx.set_fatal_reserve_time(9_000);
        ctx.set_limit_handler(hook_of(overruns_its_slice));
        ctx.request_safepoint(SafepointFlags::CPU_LIMIT);

        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let status = unsafe { crate::nvs_safepoint(&raw mut ctx) };

        assert_eq!(
            status,
            crate::FATAL,
            "the request is stopped, and as a `FATAL`"
        );
        assert_eq!(
            ENTRIES.with(std::cell::Cell::get),
            1,
            "the slot is taken on the way in, so a handler breaching again finds nothing to enter"
        );
        assert!(
            !FLAG_ON_ENTRY.with(std::cell::Cell::get),
            "a handler entered under the flag that stopped the request is stopped at its own first \
             back edge, which is a slice zero nanoseconds wide"
        );
        assert_eq!(
            SEEN_CEILING.with(std::cell::Cell::get),
            10_000,
            "the reserve is added to the ceiling for the length of the call"
        );
        assert_eq!(
            OVERRUN_STATUS.with(std::cell::Cell::get),
            Some(crate::FATAL),
            "and a handler that spends the widened ceiling meets the flag a second time"
        );
        assert!(
            !ctx.has_limit_handler(),
            "the registration is spent, not borrowed"
        );
        assert!(
            ctx.safepoint_flags().contains(SafepointFlags::CPU_LIMIT),
            "what was lowered for the call is raised again after it"
        );
    }
}
