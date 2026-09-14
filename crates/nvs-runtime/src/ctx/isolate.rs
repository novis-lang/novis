//! Static-property storage, and the ways a context makes another one.
//!
//! `docs/adr/README.md` § *Decisions taken at project start* makes a static
//! property's slot request-scoped: [`Ctx::install_statics`] materializes one
//! per declared default when the request is armed and [`Ctx`]'s `Drop` releases
//! them, so nothing outlives the request that wrote it.
//!
//! The constructors are here because they are the same question asked of
//! a *tree* of contexts.
//! `rule:security/isolate-shares-nothing` gives a tree one
//! ceiling to divide, so [`Ctx::child`], [`Ctx::isolate`],
//! [`Ctx::method_isolate`] and [`Ctx::handler_isolate`] each decide what the
//! new context shares with its root and what it starts fresh — and the statics
//! are the largest thing it does *not* share. They are also the one thing
//! [`Ctx::method_isolate`] has to arm for itself, `rule:security/isolate-shares-nothing`'s method entry being the only child
//! that runs its parent's unit.

use super::*;

impl Ctx {
    /// Moves `rule:concurrency/a-connection-is-a-root-isolate`
    /// 's socket onto the connection isolate's own context.
    ///
    /// Written once, by `nvs_host::Isolate::over_socket`, before the isolate's
    /// program runs and after the server framed the upgrade — which is the same
    /// point [`Ctx::set_inbound`] is written at, and here for the same reason:
    /// a context that had to reach back for one of these would be reaching into
    /// a request that has ended.
    ///
    /// Nothing clears it. The socket lives as long as the isolate does and
    /// closes when this context is dropped, so § 1's "a connection that exceeds
    /// a limit is closed with a defined code" arrives through the ordinary
    /// teardown rather than through a second path that has to agree with it.
    /// [`crate::peer`] is the home of the seam itself.
    pub fn set_peer(&mut self, peer: Box<dyn crate::peer::PeerSocket>) {
        self.peer = Some(peer);
    }

    /// The socket, for the isolate that has one.
    ///
    /// `None` everywhere else, and that is what makes `Core\Socket::current()`
    /// a refusal outside a connection rather than a rule to remember: an
    /// ordinary request, a `spawn script` child and a CLI program each answer
    /// it, because none of them was handed a peer.
    pub fn peer(&mut self) -> Option<&mut (dyn crate::peer::PeerSocket + 'static)> {
        self.peer.as_deref_mut()
    }

    /// Whether this context is a connection's, without borrowing the socket.
    #[must_use]
    pub fn has_peer(&self) -> bool {
        self.peer.is_some()
    }

    /// Queues one topic delivery for this connection — `rule:concurrency/a-connection-is-a-loop`'s second
    /// source. § 4's bus reaches the same queue through the
    /// [`crate::peer::Inbox`] handle [`Self::inbox`] answers with, so this is
    /// the seam for a caller holding the *context* rather than the
    /// publisher's own route.
    ///
    /// The queue is on the context rather than on the socket because the two
    /// sources are not the same kind of thing: the peer is a descriptor this
    /// isolate owns, and a delivery is a value another task published. Putting
    /// it here is what lets `Core\Socket::receive()` be the **one** wait § 3
    /// specifies without the framing layer learning that topics exist.
    ///
    /// Takes over the value's reference; [`crate::peer::Delivery`] owns that
    /// convention and [`Self::take_delivery`] is where it is handed on.
    ///
    /// # Known gap
    ///
    /// A delivery queued while a **socket** connection is already parked inside
    /// [`crate::peer::PeerSocket::receive`] is answered by the *next*
    /// `receive()` rather than waking the parked one, because that park is on
    /// the descriptor alone. § 4's `publish` makes that **observable**: a
    /// connection parked with nothing coming from its peer sits on a value
    /// another connection on the same core already published to a topic it
    /// joined. Closing it needs the park to be over both
    /// sources, which is a wake seam the framing layer has to take part in —
    /// `nvs_stdlib::socket`'s `receive()` and `nvs_server::socket`, not this
    /// method.
    ///
    /// The event-stream door is not in the gap: it has no descriptor to park
    /// on, so its wait registers [`crate::peer::Inbox::wake_on`] and the
    /// fan-out that fills this queue fires it.
    ///
    /// **Answers the delivery back when the queue is full** — § 4's bound, and
    /// [`crate::peer::Inbox::push`] owns why a refusal comes back to the caller
    /// rather than being dropped here.
    #[must_use = "a refused delivery still owns a reference the caller has to release"]
    pub fn deliver(&mut self, delivery: crate::peer::Delivery) -> Option<crate::peer::Delivery> {
        self.deliveries
            .get_or_insert_with(|| std::rc::Rc::new(crate::peer::Inbox::default()))
            .push(delivery)
    }

    /// Whether this connection missed a delivery and is to be closed — `rule:core-classes/topic`'s bound, as the one question `Core\Socket::receive()` asks before it
    /// waits on anything.
    ///
    /// `false` for every context that never subscribed to a topic, with no
    /// queue made: the answer is the same either way, and an ordinary request
    /// pays one null check for it.
    #[must_use]
    pub fn inbox_overflowed(&self) -> bool {
        self.deliveries
            .as_ref()
            .is_some_and(|inbox| inbox.overflowed())
    }

    /// The oldest queued delivery, handing its reference to the caller.
    ///
    /// `None` when the bus has nothing waiting, which is every context that is
    /// not a connection's and most that are. It does **not** make the queue:
    /// a wait on a context nothing ever published to should cost no
    /// allocation, and the answer is the same either way.
    #[must_use]
    pub fn take_delivery(&mut self) -> Option<crate::peer::Delivery> {
        self.deliveries.as_ref().and_then(|inbox| inbox.pop())
    }

    /// This connection's delivery queue, made if it has none yet — `rule:core-classes/topic`'s subscriber table is what asks, and holds the [`std::rc::Weak`]
    /// half of what this answers.
    ///
    /// A handle rather than the values themselves, because a subscription
    /// outlives every call that touches it: `subscribe` runs on the connection
    /// isolate's own task, and the publish that fills the queue runs on
    /// another. [`crate::peer::Inbox`] owns the ownership rule, including why
    /// the table's reference is weak and why nothing has to unsubscribe when a
    /// connection ends.
    ///
    /// Cost is one small allocation, charged to the first connection that
    /// subscribes or is published to and released with its context — never to
    /// an ordinary request, which never reaches this.
    #[must_use]
    pub fn inbox(&mut self) -> std::rc::Rc<crate::peer::Inbox> {
        std::rc::Rc::clone(
            self.deliveries
                .get_or_insert_with(|| std::rc::Rc::new(crate::peer::Inbox::default())),
        )
    }

    /// Arms this request's static-property storage: one slot per entry in
    /// `defaults`, in that order, each materialized from its declared
    /// initializer.
    ///
    /// **Every embedder calls this before running any of a unit's code**, and
    /// the call is `nvs_codegen::Unit::install_in`'s job rather than an
    /// embedder's own — a unit's slot *numbering* is what the compiled code
    /// baked in, so the vector handed here has to be the one that unit
    /// produced. Calling it twice re-runs the initializers and releases the
    /// previous slots, which is what makes a `Ctx` reusable across requests.
    ///
    /// The initializers are constants (`nvs_types::defaults::ConstArg`), so
    /// arming a request runs no user code and cannot fail or throw — the whole
    /// reason a static's initializer is restricted to one. A `None` entry is a
    /// static `rule:classes/definite-property-initialization` required no default of (a nullable or `lateinit`
    /// one) and starts the request at `null`.
    ///
    /// **The list is shared, not borrowed, and that is the whole of how a
    /// method entry works.** Arming a context and being able to arm a *child*
    /// of it for the same unit are one operation rather than two things to keep
    /// in step: [`Self::method_isolate`] re-materializes these same recipes for
    /// `rule:security/isolate-shares-nothing`'s
    /// `Class::method` isolate, whose code is the parent's unit's and so has no
    /// path a resolver could compile. Cost is one atomic increment per arming,
    /// against a list `nvs_codegen::Unit` already owns — atomic because that
    /// unit is one per program and read by every core.
    pub fn install_statics(&mut self, defaults: std::sync::Arc<[Option<FieldDefault>]>) {
        self.release_statics();
        let mut store: Box<[Value]> = defaults
            .iter()
            .map(|default| {
                default
                    .as_ref()
                    .map_or_else(Value::null, FieldDefault::materialize)
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        // The pointer is taken before the move, and stays valid across it:
        // moving a `Box` moves the three words, never the heap buffer.
        self.statics = store.as_mut_ptr();
        self.statics_store = store;
        self.unit_statics = Some(defaults);
    }

    /// The recipes this context was armed with, as a **handle that outlives
    /// this borrow** — what [`Self::method_isolate`] hands a child, for a
    /// caller that will arm a context it is not holding yet.
    ///
    /// [`Self::method_isolate`] answers the case where the parent *is* the
    /// spawning context. A connection is the case where it is not:
    /// `rule:concurrency/a-connection-is-a-root-isolate`
    /// makes it a **root** isolate started from the connection's own context,
    /// which never ran the unit and so has nothing to re-materialize from — so
    /// the recipes have to be taken here, inside the request that prepared the
    /// upgrade, and carried in the program that arms the child. `nvs_stdlib`'s
    /// `socket` module owns why that preparation happens on this side at all.
    ///
    /// `None` for a context that was never armed, which is a unit declaring no
    /// static property and a bare embedder alike — [`Self::install_statics`]
    /// owns why those two are one answer.
    ///
    /// Cost is one atomic increment, against a list `nvs_codegen::Unit`
    /// already owns.
    #[must_use]
    pub fn unit_statics(&self) -> Option<std::sync::Arc<[Option<FieldDefault>]>> {
        self.unit_statics.clone()
    }

    /// How many static-property slots this request holds — the length
    /// [`Ctx::install_statics`] was last armed with.
    #[must_use]
    pub fn statics_len(&self) -> usize {
        self.statics_store.len()
    }

    /// A context for a **child task of this request** — what `nvs-host` hands
    /// [`crate::host::Job`] when it runs a group.
    ///
    /// `rule:concurrency/a-child-belongs-to-the-calling-task`
    /// 's children "share the request", and this is the one place that
    /// sharing is decided: `nvs-host`'s `group` module doc is the home of *why*
    /// each field is on the side of the line it is on, because it is the only
    /// code that builds one.
    ///
    /// **The static-property base is shared by aliasing it**, which is the
    /// whole point. Compiled code loads a static through [`Self::statics`]
    /// inline, so a child with its own store would give the request two copies
    /// of every static and `Gauge::$live += 1` inside a child would be
    /// invisible outside it. The child's own `statics_store` stays empty, so
    /// its [`Drop`] releases nothing the parent owns — there is exactly one
    /// owner of those slots and it is still the parent.
    ///
    /// Everything a *task* owns rather than a request starts fresh: the output
    /// buffer, the capture stack, the assertion ledger, the pending failure,
    /// the yielder and the stack bounds — the yielder and the bounds because
    /// the child will run on a stack of its own that this context has never
    /// seen. So does the diagnostic sink, which starts at
    /// [`OutputSink::Stderr`] like any fresh context's: an [`OutputSink`] is
    /// not `Clone`, and a redirected one is a test reading its own dumps back
    /// rather than a property of the request.
    ///
    /// **The configuration crosses including the parent's overlay**, exactly as
    /// it does for [`Self::isolate`] and for that constructor's reason: `rule:security/isolate-shares-nothing`'s
    /// table calls the overlay "derived, never shared: a copy of the parent's
    /// *effective* config, which the spawn may narrow", so a child starts from
    /// the values in force where it was spawned rather than from the file. The
    /// direction is what matters and it is a security one — `[capabilities]` is
    /// a `RuntimeTighten` directive, [`crate::capability::granted`] reads this
    /// field, and a child re-reading the snapshot alone would hand back a
    /// capability its parent had dropped. A `Core\Config::set` the *child* makes
    /// is the child's own, because the copy is a copy: nothing in
    /// [`nvs_config::Request`] is shared but the snapshot underneath it.
    ///
    /// **What it spends:** one `Ctx` per in-flight child, freed when that child
    /// ends, plus the origin's own bytes copied once and one `Arc` clone of the
    /// snapshot with one `String` pair per key the parent had set. O(in-flight)
    /// and not O(children ever spawned), per
    /// `rule:programs/memory-priority`.
    ///
    /// # Safety
    ///
    /// `self` must outlive the returned context, and no code may run on the
    /// child after `self` is gone: the child holds a bare pointer into this
    /// context's static-property storage and nothing in the type expresses
    /// that. `nvs-host`'s group runner discharges it structurally — the call
    /// does not return until no child is still running (§ 4), and a parent torn
    /// down first cancels every child, which the scheduler tears down without
    /// resuming it.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the parent-outlives-child obligation is a fact about the                   caller's control flow and cannot be expressed in the signature"
    )]
    pub unsafe fn child(&self) -> Self {
        // A fresh buffer, but not necessarily a fresh *sink*: `rule:concurrency/one-scheduler`'s task
        // is part of this request rather than a context of its own, so it is
        // still answering whatever this one is answering and `rule:tooling/echo-always-has-a-sink`'s
        // first row still applies to it. An isolate reaches the same conclusion
        // by the third row, and `nvs_host::Isolate` is where that is read.
        let mut child = Self::new(if matches!(self.output, OutputSink::Body(_)) {
            OutputSink::Body(Vec::new())
        } else {
            OutputSink::Buffer(Vec::new())
        });
        // Request-wide, and therefore shared or copied.
        child.statics = self.statics;
        child.debug = self.debug;
        child.origin = self.origin.clone();
        // The effective configuration, overlay included — see the doc above for
        // why the copy goes this way round and not through the snapshot alone.
        child.config = self.config.clone();
        child.runtime_error_class = self.runtime_error_class.clone();
        // The word, not its value: a task of this request is bounded by this
        // request's wall time and by no clock of its own. See the field doc.
        child.deadline = std::sync::Arc::clone(&self.deadline);
        // The safepoint word the same way, and the same decision made twice: a
        // task is part of this request, so a stop that reaches the request
        // reaches the task whether it was spawned before the flag was raised or
        // after it.
        child.share_safepoint_with(self);
        // Sealed rather than empty: `rule:concurrency/after-response-outlives-the-connection`'s queue is the *request's*, and
        // one on a child would be drained by nobody and released when the child
        // ended. `crate::deferred` is the one home for that rule and for why a
        // refusal is the only honest answer to a registration nothing would run.
        child.deferred = None;
        child
    }

    /// A context for an **isolate** — the other half of the pair
    /// [`Ctx::child`] opens, and the one place the two part.
    ///
    /// [ADR 0116](/docs/decisions/0116.md)
    /// § 4: an isolate's arena is an ownership root of its own, so its
    /// static-property base is **its own** rather than an alias of this
    /// request's. That single difference is the whole of `rule:security/isolate-shares-nothing`'s "globals,
    /// class statics and runtime-defined constants are fresh", and it is why
    /// this constructor is safe where [`Ctx::child`] is `unsafe`: nothing in
    /// the returned context points into this one, so there is no
    /// parent-outlives-child obligation for a caller to discharge.
    ///
    /// The store starts **empty**, not merely fresh. Slot numbering is the
    /// child unit's, baked into the code that will run here, so the caller arms
    /// it with that unit's own defaults through
    /// [`install_statics`](Ctx::install_statics) — which is
    /// `nvs_codegen::Unit::install_in`'s job, exactly as it is for a request.
    /// A child whose entry is a *method* has no second unit and therefore no
    /// second `install_in`, so it is built by [`Self::method_isolate`] instead.
    ///
    /// What crosses is what `rule:security/isolate-shares-nothing`'s table calls request-wide and immutable:
    /// the debug flags, the origin, the runtime error class table (compiled
    /// code, shared by design), the deadline word and the safepoint word, since
    /// a budget is accounted at the root of the request tree and never per
    /// isolate. Both cross as the *word* and not as its value — one store stops
    /// the whole tree, whenever in the child's life it happens — and
    /// [`Self::deadline`]'s and [`Self::safepoint_word`]'s field docs own why a
    /// copy was the wrong half of that. The output sink is the caller's,
    /// because `output: 'capture'` and `output: 'inherit'` differ in nothing
    /// else.
    ///
    /// **No ceiling crosses, and that is what makes the budget the tree's.**
    /// `rule:security/isolate-shares-nothing`'s table charges a child's memory and a child's output to the
    /// root, and both counters are the thread's ([`crate::budget`]) with the
    /// child's own zero point taken here by [`Self::new`]: a child reads back
    /// its own share, the root's base predates every child so its reading holds
    /// all of them at once, and the ceiling that stops the tree is the root's.
    /// A child handed a ceiling of its own — [`Self::set_memory_limit`],
    /// [`Self::set_output_limit`] — narrows itself further and can never widen
    /// the tree.
    ///
    /// A thread's balance says all of that only while the tree is one core's.
    /// The child that is *not* — one placed on another core, made through
    /// [`Self::join_tree`] rather than here — publishes its share into the
    /// counters its tree shares ([`TreeState`]), and a reading taken on the
    /// root's core adds them. That is the same division in a second place
    /// rather than a second budget: the arithmetic is still one tree's, and the
    /// pair exists because a thread-local cannot be read from the thread that
    /// did not open it.
    ///
    /// **What it spends:** one `Ctx` per in-flight isolate plus its own statics
    /// store once armed, both freed when that isolate ends. O(in-flight), per
    /// `rule:programs/memory-priority`.
    #[must_use]
    pub fn isolate(&self, output: OutputSink) -> Self {
        let mut isolate = Self::new(output);
        // Request-wide, and therefore copied. `statics` is deliberately absent:
        // it stays null until this context is armed with the child unit's own
        // defaults, which is the difference this constructor exists for.
        isolate.debug = self.debug;
        isolate.origin = self.origin.clone();
        // The configuration **including the parent's overlay**, so a child
        // starts from the values in force where it was spawned rather than from
        // the file. That is the direction `rule:security/isolate-shares-nothing`'s table wants: a parent that
        // narrowed a limit for itself has narrowed it for the tree beneath it,
        // and a child re-reading the snapshot would silently widen it back.
        isolate.config = self.config.clone();
        // And the grant narrowing laid over it, for the same reason one line
        // up: a parent held to a shorter list of capabilities has narrowed the
        // tree beneath it, and a child that started from the overlay alone
        // would read back the names its parent gave up.
        isolate.grant_filter = self.grant_filter.clone();
        // One deeper than whatever spawned it, and carrying the same ceiling.
        // The ceiling is *copied* rather than re-read out of the configuration
        // this constructor just cloned, for the reason the overlay crosses at
        // all: a parent that narrowed its own recursion ceiling has narrowed it
        // for the tree beneath it, and a child re-reading the file would widen
        // it back. Saturating because a depth that reached `u32::MAX` is past
        // every ceiling anyone could write, so the arithmetic has no answer the
        // refusal above it would treat differently.
        isolate.script_depth = self.script_depth.saturating_add(1);
        isolate.max_script_depth = self.max_script_depth;
        isolate.runtime_error_class = self.runtime_error_class.clone();
        isolate.deadline = std::sync::Arc::clone(&self.deadline);
        isolate.share_safepoint_with(self);
        // **Not** sealed, unlike [`Self::child`], and the difference is the one
        // `rule:concurrency/after-response-outlives-the-connection` draws: an isolate runs a whole program, so the frame
        // that produced its answer returning is a trigger it has, where a
        // `Core\Task` child's returning is not the end of anything a response
        // could be. `Self::new` above already gave this context its queue and
        // `nvs_host::isolate`'s completion path is what drains it.
        isolate
    }

    /// [`Self::isolate`] for `rule:security/isolate-shares-nothing`'s
    /// **method entry** — a child running a `static` method of the unit *this*
    /// context is already running, rather than another file.
    ///
    /// The one difference from the sibling above is that this constructor arms
    /// the child's statics itself, and it can only do that here: slot numbering
    /// is the *parent's* unit's, so the child unit's own `install_in` — which is
    /// what arms a path entry, from inside the resolver's program — has nothing
    /// to run and no unit to run it from. Everything else about the child is
    /// unchanged, including that the store is **fresh**: `rule:security/isolate-shares-nothing` keeps class
    /// statics unshared whichever form the entry took, and re-materializing the
    /// recipes rather than aliasing the parent's slots is that rule.
    ///
    /// A context that was never armed through [`Self::install_statics`] has no
    /// recipes to hand over and yields a child with an empty store, which is
    /// [`Self::isolate`] exactly. That is the honest answer rather than a
    /// failure: a unit declaring no static property arms an empty list, and the
    /// two are indistinguishable to the compiled code either would run.
    ///
    /// **What it spends:** [`Self::isolate`]'s, plus one materialized slot per
    /// static property the parent's unit declares — the same store a path entry
    /// pays for its own unit, and released with the child.
    #[must_use]
    pub fn method_isolate(&self, output: OutputSink) -> Self {
        let mut isolate = self.isolate(output);
        if let Some(defaults) = self.unit_statics.clone() {
            isolate.install_statics(defaults);
        }
        isolate
    }

    /// The memory ceiling [`Self::handler_isolate`] runs under where
    /// `[log] handler_reserve_memory` states none.
    ///
    /// 16 MiB, and a flat number rather than [`Self::reserve_within`]'s
    /// proportion, because there is no ceiling to take a proportion *of*: ADR
    /// 0020 § 1's slice is carved out of the request's own `[limits] memory`,
    /// while § 3's is the engine's and is the same whatever the request was
    /// allowed. The size is what a whole `.nvs` costs rather than what a
    /// message costs — this reserve compiles and runs a program, where § 1's
    /// runs a closure the request already loaded — and 16 MiB is spent here
    /// under `rule:programs/memory-priority`'s
    /// ordering: a handler that cannot report is a failure nobody hears about.
    pub const DEFAULT_HANDLER_RESERVE_MEMORY: usize = 16 << 20;

    /// The CPU ceiling [`Self::handler_isolate`] runs under where
    /// `[log] handler_reserve_time` states none.
    ///
    /// Five seconds, on [`Self::reserve_time_within`]'s reasoning and not its
    /// number: a handler's time is bounded by what writing its report blocks
    /// on, which is a log target or a socket. The number is larger than § 1's
    /// 50 ms for the reason the memory half is larger — this one compiles a
    /// script first — and it is a ceiling on a report, not a budget for work.
    pub const DEFAULT_HANDLER_RESERVE_TIME: u64 = 5_000_000_000;

    /// A context for `rule:errors/handler-script`'s **tier-3 handler** — [`Self::isolate`] with the failing request's
    /// budget left behind.
    ///
    /// § 3's one deliberate exception to `rule:security/isolate-shares-nothing`:
    /// an ordinary isolate spends the tree's budget, which is exactly wrong for
    /// the one isolate whose job is to report that the tree ran out of it.
    /// Everything `rule:security/isolate-shares-nothing` calls request-wide still crosses — the sibling above
    /// is the one home of that list — and these part from it:
    ///
    /// - **Its own deadline word**, not the tree's. The parent's word is set
    ///   the moment its wall clock runs out, so a handler sharing it would be
    ///   cancelled before its first statement, for precisely the failure it was
    ///   configured to report.
    /// - **Its own ceilings** — [`Self::DEFAULT_HANDLER_RESERVE_MEMORY`] and
    ///   [`Self::DEFAULT_HANDLER_RESERVE_TIME`], or what the two directives
    ///   state — where an ordinary isolate carries none and is bounded by the
    ///   root's reading once control returns there. Exceeding one is § 3's
    ///   "zero retries" and nothing else: the handler fails, `nvs-host`'s
    ///   `ladder::escalate` answers `false`, and tier 4 writes the record.
    /// - **A fresh script depth**, because a chain that reached
    ///   `[limits] max_script_depth` is itself one of the failures this handler
    ///   reports, and inheriting the depth would refuse the report on the
    ///   grounds of the thing being reported. What that ceiling guards against
    ///   is guarded here by `nvs_host::ladder`'s thread-local instead, since
    ///   this is the only spawn on the path.
    ///
    /// **What it spends:** nothing between failures. The reserve is a ceiling,
    /// not an allocation, exactly as `[limits] fatal_reserve_memory` is —
    /// § 3's "sized once per worker/core" is the value's *shape*, a number that
    /// does not vary with the request, and not a pre-allocation. In flight it
    /// is one more `Ctx`, which is [`Self::isolate`]'s accounting.
    #[must_use]
    pub fn handler_isolate(&self, output: OutputSink) -> Self {
        let mut handler = self.isolate(output);
        handler.deadline = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        handler.script_depth = 0;
        handler.memory_limit = self
            .configured_handler_reserve()
            .unwrap_or(Self::DEFAULT_HANDLER_RESERVE_MEMORY);
        handler.cpu_limit = self
            .configured_handler_reserve_time()
            .unwrap_or(Self::DEFAULT_HANDLER_RESERVE_TIME);
        handler
    }

    /// [`Self::isolate`]'s copy list, as a value that **crosses a thread** —
    /// what a child placed `on: "worker"` is built from on the core that runs
    /// it.
    ///
    /// The two halves of one constructor, split where the thread boundary is:
    /// this one reads the parent, [`PlacedIsolate::build`] writes the child, and
    /// the parent's `Ctx` is never reachable from the far side
    /// (`rule:concurrency/a-wake-never-moves-a-task`). The sub-cap the child
    /// runs under is computed **here**, against what remains of the tree's
    /// budget at the spawn, which is what
    /// `rule:security/isolate-budget-is-the-trees` means by tighter than what
    /// remains and never wider.
    ///
    /// That reading is also what lets a spawn site's own `limits:` and `grants:`
    /// cross with the child. [`PlacedIsolate::narrowed_by`] attaches them and
    /// [`Self::narrow_under`] applies them over there, against the three
    /// ceilings resolved here — which are the whole of what [`Self::narrow`]
    /// reads a parent for, so the far core needs no parent to hold a child to
    /// what was written beside it.
    #[must_use]
    pub fn placed_isolate(&self) -> PlacedIsolate {
        let remains = self.remains();
        PlacedIsolate {
            debug: self.debug,
            origin: self.origin.clone(),
            config: self.config.clone(),
            grant_filter: self.grant_filter.clone(),
            script_depth: self.script_depth.saturating_add(1),
            max_script_depth: self.max_script_depth,
            runtime_error_class: self.runtime_error_class.clone(),
            unit_statics: self.unit_statics.clone(),
            deadline: std::sync::Arc::clone(&self.deadline),
            tree: self.tree_handle(),
            memory_limit: remains.memory,
            output_limit: remains.output,
            cpu_limit: remains.cpu,
            fatal_reserve: self.fatal_reserve,
            narrowing: crate::host::Narrowing::default(),
        }
    }

    /// Holds `child` — an isolate this context just made — to what the spawn
    /// site wrote beside it, and to nothing wider.
    ///
    /// The same-core half of one narrowing. All it adds to
    /// [`Self::narrow_under`] is the reading of what this tree has already
    /// spent, which is the one thing a child built on another core cannot take
    /// for itself: there the same numbers cross in [`PlacedIsolate`], resolved
    /// by [`Self::placed_isolate`] where the parent could still be read.
    pub fn narrow(&self, child: &mut Self, narrowing: &crate::host::Narrowing) {
        child.narrow_under(narrowing, self.remains());
    }

    /// What is left of this context's three ceilings, in the units they are
    /// written in — what a sub-cap is clamped against, and what a child placed
    /// on another core is handed in place of the parent that holds them.
    fn remains(&self) -> Remains {
        Remains {
            memory: remaining(self.memory_limit, self.memory_used()),
            output: remaining(self.output_limit, self.output_used()),
            cpu: self.cpu_limit,
        }
    }

    /// Holds **this** context to `narrowing`, clamped against ceilings a caller
    /// already resolved rather than against a parent it can reach.
    ///
    /// **Two narrowings, two mechanisms, one word.** A sub-cap goes through this
    /// context's own configuration overlay, because `nvs_config::Request::set`
    /// already refuses a value wider than the one in force whoever writes it —
    /// one reader for the number a spawn site writes and the number the file
    /// writes, rather than a second one grown beside it. What that overlay
    /// cannot know is what the tree has already *spent*, so the ceilings it
    /// resolves are clamped against `remains`, and that clamp is the whole of
    /// what `rule:security/isolate-budget-is-the-trees` means by
    /// tighter than what remains. A grant list is an intersection instead: a
    /// grant is a list and not a quantity, and the overlay this context
    /// inherited is still asked underneath it, so a name the parent lacks stays
    /// lacking.
    ///
    /// [`Self::max_script_depth`] is put back after the refresh, and that line
    /// is the point of the two around it: the depth ceiling is *copied* into a
    /// child by [`Self::isolate`] precisely so that a child cannot re-read the
    /// file and widen it, and a refresh reading the configuration would do
    /// exactly that. A child holding no configuration takes no sub-cap at all —
    /// there is no ceiling in force for one to be tighter than.
    fn narrow_under(&mut self, narrowing: &crate::host::Narrowing, remains: Remains) {
        if !narrowing.limits.is_empty() && self.config.is_some() {
            if let Some(config) = self.config.as_mut() {
                for (key, written) in &narrowing.limits {
                    // The refusal is the answer: a sub-cap wider than what is in
                    // force leaves the inherited ceiling standing, which is the
                    // direction that fails closed. `Core\Config::set` reports
                    // the same refusal to a program that can read one, and a
                    // spawn site has nothing to read it with.
                    let _narrowed = config.set(key, written);
                }
            }
            let depth = self.max_script_depth;
            self.refresh_limits();
            self.max_script_depth = depth;
            self.cpu_limit = tighter(self.cpu_limit, remains.cpu);
            self.output_limit = tighter(self.output_limit, remains.output);
            // Last, and through the setter, because it arms the allocator with
            // the number beside this context's safepoint address — the same
            // order and the same reason [`PlacedIsolate::build`] has.
            self.set_memory_limit(tighter(self.memory_limit, remains.memory));
        }
        if let Some(names) = &narrowing.grants {
            self.restrict_grants(names);
        }
    }

    /// Narrows what `child` may ask a capability for to the names it was given,
    /// intersected with whatever narrowing it already inherited.
    ///
    /// A name no capability has is dropped rather than refused: it grants
    /// nothing, and dropping it from a list that only ever narrows fails closed.
    /// The names are the spelling `nvs.toml` grants them under, which is the one
    /// spelling either side of this has to learn.
    fn restrict_grants(&mut self, names: &[String]) {
        let asked = names
            .iter()
            .filter_map(|name| nvs_config::capability::Cap::parse(name));
        let kept: Vec<nvs_config::capability::Cap> = match &self.grant_filter {
            Some(held) => asked.filter(|cap| held.contains(cap)).collect(),
            None => asked.collect(),
        };
        self.grant_filter = Some(kept.into());
    }

    /// Whether `cap` survives the `grants:` narrowing this context carries —
    /// `true` for one that carries none.
    ///
    /// Asked *beside* the configuration's own answer and never instead of it
    /// ([`crate::capability::granted`]): this list can only ever subtract, so a
    /// capability the overlay does not hold is not granted by appearing in it.
    #[must_use]
    pub fn grants_allow(&self, cap: nvs_config::capability::Cap) -> bool {
        self.grant_filter
            .as_ref()
            .is_none_or(|kept| kept.contains(&cap))
    }

    /// The base of the static-property storage compiled code loads inline —
    /// the word at [`STATICS_OFFSET`], handed out rather than re-derived.
    ///
    /// Null before [`Ctx::install_statics`] has run, which is safe because a
    /// unit declaring no static emits no instruction that reads it. It is
    /// handed out because the callers that want it cannot reach the field:
    /// `nvs-host`'s group runner, which gives a child the *same* base so a
    /// request has one copy of every static rather than one per task
    /// ([`Ctx::child`]), and a test asserting that it did.
    #[must_use]
    pub fn statics_base(&self) -> *mut Value {
        self.statics
    }

    /// Releases every armed slot and disarms the pointer beside them.
    ///
    /// Each slot owns exactly one reference — [`FieldDefault::materialize`]
    /// hands one over and a static write releases what it overwrote — so this
    /// is one release per slot, never a scan of what compiled code did with
    /// them.
    #[expect(
        unsafe_code,
        reason = "a slot's owned reference is released exactly once here; the \
                  slots were materialized by `install_statics` and no other \
                  owner of them exists"
    )]
    pub(super) fn release_statics(&mut self) {
        let store = std::mem::replace(&mut self.statics_store, Vec::new().into_boxed_slice());
        self.statics = std::ptr::null_mut();
        for value in store.into_vec() {
            unsafe { value.release() };
        }
    }

    /// Takes ownership of the value that crossed into this isolate, so that
    /// releasing the isolate releases it too.
    ///
    /// This is where [`crate::script::Program`]'s "the argument is
    /// transferred" lands. A program is handed one reference and has to put it
    /// somewhere `rule:security/isolate-teardown-is-a-drain-then-a-sweep`
    /// 's wholesale release will reach; this context *is* that ownership
    /// root, so this is the one place with both the reference and the lifetime
    /// in hand. Calling it twice releases what it replaces, and a context that
    /// is never handed one holds `null` and releases nothing.
    ///
    /// The child's own surface for *reading* it is `Core\Script::args()`, and
    /// `nvs_stdlib::script`'s module doc is the one home of what that answers.
    /// It reads through [`Self::isolate_argument`] and retains, so this slot
    /// stays the only owner however often the child asks.
    #[expect(
        unsafe_code,
        reason = "this context owned the reference it is replacing, having \
                  been handed it by exactly one earlier call"
    )]
    pub fn set_isolate_argument(&mut self, value: Value) {
        let previous = std::mem::replace(&mut self.isolate_argument, value);
        // SAFETY: `isolate_argument` holds one owned reference or null, and
        // nothing else points at it — the field is private and handed out only
        // by the borrowing accessor below.
        unsafe { previous.release() };
    }

    /// The value that crossed into this isolate, **borrowed**.
    ///
    /// No reference is handed over, exactly as reading any other slot hands
    /// none over: a caller that keeps the value retains it first.
    #[must_use]
    pub fn isolate_argument(&self) -> Value {
        self.isolate_argument
    }
}

/// What a core other than this one is handed to build a placed child's root
/// context — [`Ctx::placed_isolate`]'s answer and [`Self::build`]'s input.
///
/// Every field is a plain value or an [`Arc`](std::sync::Arc) whose contents
/// every core already reads: the request tree's shared state, the deadline word,
/// the configuration snapshot and the compiled unit's class table and
/// static-property recipes. That is the whole of why an isolate's context can be
/// made on a core the spawn never ran on, and it is also the boundary — nothing
/// reachable from a [`Value`] is in here, because a refcount is non-atomic and
/// the argument crosses as bytes instead
/// (`rule:concurrency/on-worker-runs-the-child-on-another-core`).
///
/// **What it spends** (`rule:programs/memory-priority`): one of these per
/// placement in flight, holding one atomic increment on each of those handles
/// and a copy of the parent's origin string, released when the child's context
/// is built.
#[derive(Debug)]
pub struct PlacedIsolate {
    debug: DebugFlags,
    origin: Option<Box<str>>,
    config: Option<nvs_config::Request>,
    grant_filter: Option<std::sync::Arc<[nvs_config::capability::Cap]>>,
    script_depth: u32,
    max_script_depth: u32,
    runtime_error_class: Option<ErrorClass>,
    unit_statics: Option<std::sync::Arc<[Option<FieldDefault>]>>,
    deadline: std::sync::Arc<std::sync::atomic::AtomicU64>,
    tree: std::sync::Arc<TreeState>,
    memory_limit: usize,
    output_limit: usize,
    cpu_limit: u64,
    fatal_reserve: usize,
    narrowing: crate::host::Narrowing,
}

impl PlacedIsolate {
    /// Holds the child this seed builds to what the spawn site wrote beside it,
    /// the way `crate::host::Narrowing` reaches a same-core child through
    /// `nvs-host`'s `Isolate::narrowed_by`.
    ///
    /// It is carried rather than applied because the context it narrows does not
    /// exist yet: [`Self::build`] makes it on the far core and narrows it there,
    /// against the ceilings this seed already resolved.
    #[must_use]
    pub fn narrowed_by(mut self, narrowing: crate::host::Narrowing) -> Self {
        self.narrowing = narrowing;
        self
    }

    /// Builds the child's root context, **on the core that will run it**.
    ///
    /// [`Ctx::isolate`]'s body from the other side of the thread boundary, and
    /// the differences from it are the three things being on another core
    /// changes. The tree is joined through [`Ctx::join_tree`] rather than shared
    /// from a parent that is not here, so the child polls the word the root
    /// polls and publishes its share into the counters the root reads. The
    /// sub-cap is *armed*, where a same-core child leaves the ceiling to the
    /// root's own polls: the root cannot see this thread's balance, so what
    /// bounds the child is the number it was handed where it was placed. And the
    /// zero points are this thread's, taken by [`Ctx::new`] below, which is what
    /// makes the share this context publishes its own rather than the far
    /// thread's whole history.
    ///
    /// [`Self::narrowed_by`]'s narrowing lands here for that same reason.
    /// [`Ctx::narrow`] reads a parent only for what its tree has already spent,
    /// and this seed carries that reading, so the spawn site's `limits:` and
    /// `grants:` hold a child on a core the spawn never ran on.
    #[must_use]
    pub fn build(self, output: OutputSink) -> Ctx {
        let remains = Remains {
            memory: self.memory_limit,
            output: self.output_limit,
            cpu: self.cpu_limit,
        };
        let mut child = Ctx::new(output);
        child.debug = self.debug;
        child.origin = self.origin;
        child.config = self.config;
        child.grant_filter = self.grant_filter;
        child.script_depth = self.script_depth;
        child.max_script_depth = self.max_script_depth;
        child.runtime_error_class = self.runtime_error_class;
        child.deadline = self.deadline;
        child.cpu_limit = self.cpu_limit;
        child.fatal_reserve = self.fatal_reserve;
        child.output_limit = self.output_limit;
        // Before the ceiling below, and that order is load-bearing:
        // [`Ctx::arm_memory_ceiling`] arms the allocator with this context's
        // safepoint address beside the number, and until the tree is joined that
        // address is this context's own fresh word — which nothing in the tree
        // would ever poll.
        child.join_tree(self.tree);
        child.set_memory_limit(self.memory_limit);
        // After the ceiling above rather than before it, because a sub-cap is
        // only ever tighter: what it clamps is the number just armed, and
        // `remains` is what the parent had left when it placed this child. A
        // seed carrying no narrowing leaves every line of this untouched.
        child.narrow_under(&self.narrowing, remains);
        child
    }

    /// [`Self::build`] for `rule:security/isolate-shares-nothing`'s **method
    /// entry**, which arms its own statics from the parent's unit —
    /// [`Ctx::method_isolate`] across the same boundary.
    ///
    /// The recipes cross because the compiled unit owns them and every core
    /// reads that unit; what the child gets is a store of its own materialized
    /// from them here, which is the same freshness a same-core method entry
    /// gets. A seed carrying none yields [`Self::build`] exactly, for the reason
    /// [`Ctx::method_isolate`] gives: a unit declaring no static property arms an
    /// empty list, and the two are indistinguishable to the code either would
    /// run.
    #[must_use]
    pub fn build_method(self, output: OutputSink) -> Ctx {
        let defaults = self.unit_statics.clone();
        let mut child = self.build(output);
        if let Some(defaults) = defaults {
            child.install_statics(defaults);
        }
        child
    }
}

/// What is left of a context's three ceilings at a spawn, in the units they are
/// written in — [`Ctx::remains`]'s answer.
///
/// A value rather than a second reading of the parent, because the context a
/// placed child is narrowed in is built on a core the parent is not reachable
/// from (`rule:concurrency/a-wake-never-moves-a-task`). The numbers are resolved
/// where they can be read and cross in [`PlacedIsolate`] beside everything else
/// the far core builds from, which is what lets one narrowing have one
/// implementation on both sides of the boundary.
#[derive(Clone, Copy, Debug)]
struct Remains {
    memory: usize,
    output: usize,
    cpu: u64,
}

/// What is left of `limit` once `used` is taken off it, in the unit the ceiling
/// is written in — the sub-cap a child placed on another core runs under.
///
/// `0` in and `0` out, because that is the sentinel for a tree under no ceiling
/// at all. Everything else floors at **one** rather than at zero: a tree that
/// has already spent its whole budget must hand its child a cap it cannot
/// allocate under, and zero is the one number that would instead read as
/// permission to allocate without bound.
fn remaining(limit: usize, used: usize) -> usize {
    if limit == 0 {
        return 0;
    }
    limit.saturating_sub(used).max(1)
}

/// The tighter of two ceilings written in one unit, where `0` is the sentinel
/// for no ceiling at all and so loses to every number beside it.
///
/// [`Ctx::narrow`]'s clamp, and the reason a plain `min` is wrong here: an
/// uncapped side spells itself the smallest number there is, so `min` would read
/// "no ceiling" as the tightest ceiling of all and stop a child that nothing was
/// bounding.
fn tighter<T: Ord + Default>(one: T, other: T) -> T {
    if one == T::default() {
        return other;
    }
    if other == T::default() {
        return one;
    }
    one.min(other)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `rule:security/isolate-shares-nothing`'s "child output against the root's `max_output`", as the two
    /// readings that sentence implies: a child re-bases at `Ctx::new` and so
    /// reads back only its own bytes, while the root's base predates the child
    /// and its reading holds both. The ceiling that stops the tree is the
    /// root's, and no child was given one.
    #[test]
    fn an_isolates_output_is_charged_to_it_and_to_the_root_at_once() {
        let mut root = Ctx::buffered();
        root.set_output_limit(16);
        root.write_output(b"1234").expect("a buffer");
        assert_eq!(root.output_used(), 4);
        assert!(!root.over_output_limit());

        let mut child = root.isolate(OutputSink::Buffer(Vec::new()));
        child.write_output(b"567890").expect("a buffer");
        assert_eq!(child.output_used(), 6, "its own share, and only that");
        assert_eq!(root.output_used(), 10, "the child's bytes are the root's");
        assert!(!root.over_output_limit());

        child.write_output(b"1234567").expect("a buffer");
        assert_eq!(child.output_limit(), 0, "no ceiling crossed to the child");
        assert!(!child.over_output_limit());
        assert!(
            root.over_output_limit(),
            "17 bytes written beneath a ceiling of 16"
        );
        assert!(root.output_breach().is_some());
    }

    #[test]
    fn installing_statics_materializes_one_slot_per_declared_default() {
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        assert_eq!(ctx.statics_len(), 0);
        ctx.install_statics(std::sync::Arc::from(vec![
            Some(FieldDefault::Int(3)),
            Some(FieldDefault::Str("hi".to_owned())),
            None,
        ]));
        assert_eq!(ctx.statics_len(), 3);
        // Re-arming is what a second request on a reused context does: the
        // previous slots are released, never leaked, and the initializers run
        // again rather than the writes of the request before carrying over.
        ctx.install_statics(std::sync::Arc::from(vec![
            Some(FieldDefault::Int(3)),
            Some(FieldDefault::Str("hi".to_owned())),
            None,
        ]));
        assert_eq!(ctx.statics_len(), 3);
    }

    /// `rule:security/isolate-shares-nothing`'s method entry: the child runs the *parent's* unit, so the
    /// recipes the parent was armed with are what arm it — fresh slots at the
    /// parent's own numbering, and no resolver in the path.
    #[test]
    fn a_method_isolate_arms_its_own_slots_from_the_parents_unit() {
        let mut parent = Ctx::new(OutputSink::Buffer(Vec::new()));
        parent.install_statics(std::sync::Arc::from(vec![
            Some(FieldDefault::Int(3)),
            Some(FieldDefault::Str("hi".to_owned())),
        ]));

        let child = parent.method_isolate(OutputSink::Buffer(Vec::new()));
        assert_eq!(
            child.statics_len(),
            2,
            "the parent's unit declares two, and the child runs that unit"
        );
        assert_ne!(
            child.statics_base(),
            parent.statics_base(),
            "fresh, not aliased: `rule:security/isolate-shares-nothing` keeps class statics unshared"
        );
        // A plain isolate is the other file's, and stays empty until that
        // file's own unit arms it.
        let other = parent.isolate(OutputSink::Buffer(Vec::new()));
        assert_eq!(other.statics_len(), 0);

        // A grandchild works for the same reason the child does: arming a
        // context is what hands it the recipes, so the chain does not run out.
        let grandchild = child.method_isolate(OutputSink::Buffer(Vec::new()));
        assert_eq!(grandchild.statics_len(), 2);

        // And a context nobody armed hands over nothing rather than failing —
        // indistinguishable from a unit that declares no static property.
        let bare = Ctx::new(OutputSink::Buffer(Vec::new()));
        assert_eq!(bare.method_isolate(OutputSink::Sink).statics_len(), 0);
    }

    /// A stand-in for the compiled `static` method an `rule:security/isolate-shares-nothing` method entry
    /// names: it answers with how many static slots the context it was called
    /// on holds, which is the one thing such a child has to have been given
    /// before its first statement runs.
    ///
    /// The exit sweep is the callee's ([`crate::dispatch`]): slot 0 is the
    /// called class and the row declares no parameter, so releasing that one is
    /// the whole of it.
    #[expect(
        unsafe_code,
        reason = "a compiled callee's signature is `NvsFn`, which is a raw \
                  pointer contract with no safe spelling -- this is the same \
                  hand-written stand-in `nvs_stdlib::command`'s dispatch tests \
                  use, and `call_at` guarantees both pointers for the length of \
                  the call"
    )]
    unsafe extern "C" fn monthly(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        // SAFETY: `call_at` hands a callee its caller's context and a slot
        // buffer holding the receiver, both live for this call and neither
        // aliased while it runs.
        unsafe {
            let slots = (*ctx).statics_len();
            (*args).release();
            *out = Value::int(i64::try_from(slots).unwrap_or(-1));
        }
        crate::OK
    }

    /// The item this constructor exists for: a child whose entry is
    /// `Reports::monthly(...)` reaches the class that method is declared in
    /// with no resolver in the path, because the *class table* crossed with the
    /// context and the *statics* were armed from the parent's own recipes.
    /// `crate::script`'s module doc is the home of why those two are the whole
    /// handle.
    #[test]
    fn a_method_isolate_reaches_the_class_its_entry_is_declared_in() {
        let mut classes = crate::ClassTable::new();
        let id = classes.define("Reports", &[] as &[&str], &[]);
        classes.set_methods(
            id,
            vec![crate::MethodRow {
                name: "monthly".to_owned(),
                code: (monthly as crate::NvsFn) as *const u8,
                arity: 0,
                param_tags: 0,
                public: true,
                native: false,
            }],
        );

        let mut parent = Ctx::new(OutputSink::Buffer(Vec::new()));
        parent.set_runtime_error_class(crate::ErrorClass::new(std::sync::Arc::new(classes), id));
        parent.install_statics(std::sync::Arc::from(vec![Some(FieldDefault::Int(3)), None]));

        let mut child = parent.method_isolate(OutputSink::Buffer(Vec::new()));
        let answer = crate::call_static(&mut child, "Reports::monthly", &[])
            .expect("the stand-in does not throw")
            .expect("the parent's class table crossed, so the label resolves");
        assert_eq!(
            answer.as_int(),
            Some(2),
            "the method ran on a context armed with the parent unit's two slots"
        );

        // The contrast is the point: a *path* entry's isolate carries the same
        // class table — it is a clone of the parent's until its own unit's
        // `install_in` replaces it — and deliberately no statics, because the
        // slot numbering it will run under is the other unit's.
        let mut other = parent.isolate(OutputSink::Buffer(Vec::new()));
        let answer = crate::call_static(&mut other, "Reports::monthly", &[])
            .expect("the stand-in does not throw")
            .expect("the label resolves there too");
        assert_eq!(
            answer.as_int(),
            Some(0),
            "nothing armed it, and nothing may"
        );

        // A label naming a class this unit does not declare is `None` rather
        // than a throw, which is what lets a spawn report `rule:security/isolate-shares-nothing`'s failure as
        // a value.
        assert!(
            crate::call_static(&mut child, "Ledger::monthly", &[])
                .expect("no throw")
                .is_none()
        );
    }

    /// `rule:security/isolate-shares-nothing`'s "one ceiling to divide": the flag is the tree's own word, so
    /// the store reaches a child built before the timer fired. The child here
    /// is spawned first because that is the order a copied flag would answer
    /// wrongly, where spawning afterwards would pass either way.
    #[test]
    fn expiring_a_deadline_reaches_a_child_spawned_before_the_timer_fired() {
        let root = Ctx::buffered();
        let early = root.isolate(OutputSink::Sink);
        assert!(!early.deadline_expired());

        root.expire_deadline();
        assert!(early.deadline_expired(), "the child stops with the tree");
        assert!(
            root.isolate(OutputSink::Sink).deadline_expired(),
            "and so does one spawned afterwards",
        );
    }
}
