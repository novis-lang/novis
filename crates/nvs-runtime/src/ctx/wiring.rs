//! What a host writes into a context before any Novis code runs.
//!
//! [`Ctx::new`] and the setters around it: the session, the exit code, the
//! origin, the configuration snapshot, the trace context, the command and route
//! tables, `argv`, the program name and the program's identity — each one a
//! value the host knows and the language cannot ask for, because
//! `rule:statements/no-host-populated-variables` leaves nothing ambient to the
//! language itself.
//!
//! The determinism knobs live here for the same reason.
//! `rule:testing/determinism-declared-on-the-test`'s fixed
//! clock, the seeded random state and the scripted answers are all *the host
//! deciding what a request observes*, which is what every other setter in this
//! file is too.
//!
//! **The compiled unit's exception class table is part of this wiring too, and
//! it is not optional.** `nvs_codegen::Classes::install_in` is the one call
//! that writes it — four obligations share that call, and its own docs carry
//! them — and it reaches [`Ctx::set_runtime_error_class`] here. A context that
//! never took a table answers *every* ancestry question about a failure with
//! `false`, because a helper-raised failure has no descriptor to read ancestry
//! off, so the omission shows up as a wrong answer and not merely as less
//! detail: `Core\Test::assertThrows` reports a non-match for a failure that is
//! an instance of the class it named. Embedding the runtime without a table is
//! therefore not a supported configuration, and
//! [`Ctx::pending_conforms_to`] asserts it in a debug build.

use super::*;

/// The request's session as its `Core\Html::later` slots reach it
/// (`rule:core-classes/html-later`).
///
/// A slot runs on a child context, which has no session and no request of its
/// own. While the slots run, the request moves its session into this cell and
/// every slot reads it there. Where the main script never opened the session,
/// the first slot to use it opens it from [`Self::head`], into the same cell,
/// so the slots load it once between them. The request takes the cell's
/// session back when the slots end, so the write-back at its end sees it.
///
/// A slot only reads: every member that writes throws where the head is
/// closed, so the cell is never written after it is filled.
///
/// It is also the one copy of the request a slot reads: [`Ctx::inbound`] falls
/// back to [`Self::head`], so `Core\Request` in a slot reads the same request
/// as the main script, and a body read throws.
///
/// What it spends: one cell and one copy of the request without its body
/// ([`Inbound::head_alone`]), per in-flight request with slots, for the length
/// of its slots.
///
/// Declined: opening the session before every slot run, because a request with
/// slots and a session cookie would pay a store round trip it may not need; and
/// a child holding a pointer to its parent's context, because it would alias
/// the parent's `&mut` while the parent is parked in the group.
#[derive(Debug)]
pub(crate) struct LentSession {
    /// The session, once it is open.
    open: std::cell::OnceCell<Session>,
    /// The request without its body, which a slot reads and opens the session
    /// from.
    head: Inbound,
}

/// The session one request has open — what a request holds between its first
/// `Core\Session` call and the write-back at its end, kept per key as
/// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key` says.
///
/// **Deliberately not a [`Value`].** Each value crosses the store boundary as
/// `rule:classes/two-copy-depths`'s byte carrier in both directions, for the
/// same reason a `Core\Cache` entry does, so holding the bytes means the
/// context has nothing to release at teardown and holds no object that could
/// name a `ClassDesc` an
/// `rule:config/an-edit-reaches-the-next-request-without-a-restart` unit swap
/// has retired. Decoding is [`crate::decode`]'s, of one key's value per `get`.
///
/// What it spends: one identifier, one encoded value per key, the names of
/// the keys the request set or removed and of the long-lived keys, and one
/// short token, per **in-flight** request that started a session, freed with
/// the request — never O(sessions served).
///
/// `Clone` for one case only: [`Ctx::reclaim_session`] copies a lent session
/// back where a child's context still holds the cell, which the group runner
/// makes rare.
#[derive(Clone, Debug)]
pub struct Session {
    /// The identifier the store issued, as it rides in the cookie.
    ///
    /// Empty for a fresh session nobody has written yet: it has no row in the
    /// store and no cookie, and `Core\Session`'s first write draws the
    /// identifier.
    pub id: String,
    /// Each key of the session and its value, encoded alone by
    /// [`crate::encode`].
    ///
    /// A `BTreeMap` rather than a hash map, so the write-back sends the keys in
    /// one order and a test can read them back in it. What it spends: the
    /// loaded session's bytes, once, per in-flight request.
    pub entries: std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
    /// The keys this request set or removed since the load: what the
    /// write-back sends.
    ///
    /// A touched key that is in [`Self::entries`] is a change, and one that is
    /// not is a removal. A key the request only read is never here, so a stale
    /// value it read cannot overwrite a newer one another request wrote. What it
    /// spends: one key name per key the request wrote.
    pub touched: std::collections::BTreeSet<Vec<u8>>,
    /// The keys of [`Self::entries`] whose lifetime is long
    /// (`rule:http-server/a-session-value-is-short-lived-or-long-lived`).
    ///
    /// Every other key is short-lived. A write of a key sets its lifetime, so
    /// a key moves in or out of this set with each write. What it spends: one
    /// key name per long-lived key, per in-flight request.
    pub long: std::collections::BTreeSet<Vec<u8>>,
    /// The short token this browser presented and the record holds, or the
    /// one this request drew: 22 characters of base64url.
    ///
    /// `None` while the record has no token this request may use: a new
    /// session, and a browser that presented no matching short cookie. The
    /// next short-lived write draws one and sends the short cookie. What it
    /// spends: one 22-byte string per in-flight session that has one.
    pub short_token: Option<String>,
    /// When the long clock ends, in milliseconds since the epoch, or `0`
    /// while the record has never held a long-lived value.
    ///
    /// It moves only where the identifier cookie is sent with a new
    /// `Max-Age`, so the cookie and the store end together.
    pub long_until: i64,
    /// Whether the program opened the session under an identifier it read
    /// itself. Its short-lived values are bound to the identifier alone, so
    /// no short token is checked, drawn or sent for it.
    pub by_hand: bool,
    /// Whether the request called `clearShortLived` since the load, so the
    /// write-back deletes every short-lived key the store holds before it sends
    /// [`Self::touched`], also one another request wrote.
    pub cleared_short: bool,
    /// The same for `clearLongLived` and the long-lived keys.
    pub cleared_long: bool,
    /// The `Core\Session` member whose call opened the session, as its message
    /// names it: `openWithIdentifier` throws naming it once the session is
    /// open. Empty for a session a host or a test opened.
    pub opened_by: &'static str,
    /// Whether the write-back owes the store a write.
    ///
    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`'s
    /// "writing only when the session changed" is what keeps a read-only
    /// request off the write path: a request that starts a session and reads it
    /// makes one round trip, not two. It is also set for an identifier the
    /// request drew while the session is still empty, whose row the write-back
    /// creates.
    pub dirty: bool,
    /// Whether the store holds a row under [`Self::id`].
    ///
    /// The first use sets it for a row it loaded, and the write-back sets it after the
    /// write that creates the row. The write-back reads it to choose its write:
    /// a stored session is *saved*, which changes only a session that still
    /// exists, and a session this request drew is *issued*. So a session that
    /// another request destroyed while this one ran stays destroyed, and this
    /// request's changes to it are dropped
    /// (`rule:http-server/a-session-is-loaded-once-and-written-back-per-key`).
    /// What it spends: one `bool` per in-flight session, and no round trip.
    pub stored: bool,
    /// How [`Self::touched`] is sent when the program that opened the session
    /// ends — that rule's write-back, travelling **with the session** rather
    /// than being reached for at the end.
    ///
    /// It has to travel, because none of the three crates involved can name
    /// the other two. The store is `nvs-stdlib`'s —
    /// `rule:http-server/a-session-store-answers-six-operations`'s six operations are
    /// over `Core\Cache`'s wire — while the two places a program *ends* are
    /// `nvs-host`'s isolate teardown, which an HTTP request is
    /// (`rule:security/isolate-shares-nothing`), and
    /// `nvs run`'s root task; neither of those crates depends on `nvs-stdlib`,
    /// and `nvs-stdlib` may not depend on either. This crate is the one all of
    /// them already rest on, so the seam is inverted through it exactly as
    /// [`crate::host`]'s module doc inverts the scheduler's, and the pointer is
    /// filled in by `Core\Session`'s first-use open, the one place that opens a
    /// record at all.
    ///
    /// A bare `fn` rather than a boxed Rust closure: there is one implementation
    /// and it captures nothing — everything it needs is on the [`Ctx`] it is
    /// handed — so a `dyn FnOnce` would be an allocation per request to say
    /// what one word already says.
    pub write_back: fn(&mut Ctx),
}

impl Session {
    /// An empty session with no identifier and no row in the store, written
    /// back by `write_back`: what a first use opens for a visitor with no
    /// record. Every other field is the empty or `false` one.
    #[must_use]
    pub fn empty(write_back: fn(&mut Ctx)) -> Self {
        Self {
            id: String::new(),
            entries: std::collections::BTreeMap::new(),
            touched: std::collections::BTreeSet::new(),
            long: std::collections::BTreeSet::new(),
            short_token: None,
            long_until: 0,
            by_hand: false,
            cleared_short: false,
            cleared_long: false,
            opened_by: "",
            dirty: false,
            stored: false,
            write_back,
        }
    }
}

impl Ctx {
    /// How many objects this context has allocated and not yet dismantled —
    /// what [`crate::object::sweep`] would have to take apart if the context
    /// ended now.
    #[cfg(test)]
    pub(crate) fn live_objects(&self) -> usize {
        self.live.count()
    }

    /// A context writing to the given sink, with nothing pending, every flag
    /// clear, and [`RunKind::Command`] as its kind of run.
    ///
    /// A debug build arms the shipped default memory ceiling
    /// (`nvs_config::defaults::LIMITS_MEMORY`), so every Rust test that makes a
    /// context runs the overshoot backstop. A test that needs more memory, or
    /// none, calls [`Self::set_memory_limit`] with a comment saying why. A
    /// release build arms no ceiling here.
    #[must_use]
    pub fn new(output: OutputSink) -> Self {
        // Both are allocated before `memory_base` is read, as they always
        // were, so neither is on the new context's own balance.
        let tree = std::sync::Arc::new(TreeState::default());
        let deadline = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let mut ctx = Self::sharing(output, tree, deadline);
        let turn = crate::budget::running_turn();
        if !turn.is_null() {
            ctx.made = Some(super::limits::Made {
                turn,
                tree: std::sync::Arc::as_ptr(&ctx.tree),
                deadline: std::sync::Arc::as_ptr(&ctx.deadline),
                live: std::rc::Rc::as_ptr(&ctx.live),
            });
        }
        ctx
    }

    /// [`Self::new`] over a tree and a deadline the caller already has.
    ///
    /// [`Ctx::child`] passes its parent's, so a task allocates neither and
    /// frees neither. Everything else is fresh, the trace ids included, so a
    /// task's exported spans are its own as before. The context records no
    /// [`super::limits::Made`]: it made nothing that
    /// [`Self::hand_over_its_making`] could credit.
    ///
    /// **What it spends:** nothing. It saves two allocations and two frees
    /// per spawned task.
    pub(super) fn sharing(
        output: OutputSink,
        tree: std::sync::Arc<TreeState>,
        deadline: std::sync::Arc<std::sync::atomic::AtomicU64>,
    ) -> Self {
        // The stack pointer in *this* frame, which is the closest thing to
        // "where the request starts" that needs no platform call. It
        // under-reports the true base by however deep the caller already is,
        // which shrinks the ceiling rather than stretching it — the safe
        // direction.
        let base = stack_pointer();
        // The tree's shared state is made before the struct that names its
        // word: what the hot slot holds is an address inside that allocation,
        // and moving the handle into the field below leaves the allocation
        // exactly where it is.
        //
        // The trace ids are drawn before `memory_base` is read. The draw makes
        // the thread's random generator on its first use, and that generator is
        // the thread's for as long as it runs. Charged to the first context
        // made on a thread, its bytes would be on one run's balance and not on
        // the next, so a ceiling would be crossed at a point that depends on
        // what this thread did before: a compile draws first, a cache load does
        // not (`rule:testing/bench-counters`).
        let trace_context = crate::trace_context::TraceContext::started();
        crate::budget::keep_footprint_outside();
        let mut ctx = Self {
            safepoint: std::ptr::from_ref(&tree.word),
            debug: super::trace::starting_debug_flags(),
            deadline,
            stack_limit: 0,
            stack_floor: 0,
            statics: std::ptr::null_mut(),
            deprecated: 0,
            deprecated_logged: std::collections::HashSet::new(),
            frames: std::ptr::null(),
            stepping: 0,
            step: None,
            returned: None,
            debug_stopping: false,
            throw_reported: false,
            tree,
            cancelled: false,
            exit_code: 0,
            memory_base: crate::budget::live_bytes(),
            memory_peak_saved: crate::budget::rebase_peak(),
            // A context with no ceiling arms none, and takes the thread's
            // arming with it either way: what it displaced comes back as it
            // drops, so the enclosing request is measured against its own
            // ceiling again rather than against a child's.
            memory_ceiling_saved: crate::budget::displace(crate::budget::Armed::NONE),
            // The verdict beside the threshold, taken the same way: a context
            // begins under no refusal whatever the thread was carrying, and
            // hands back what it took as it drops.
            memory_refused_saved: crate::budget::take_refusal(),
            memory_off_stack: false,
            made: None,
            output_base: crate::budget::written_bytes(),
            // A context is on its tree's root core until something says
            // otherwise, and the only thing that can is `Ctx::join_tree` on the
            // core the child was placed on.
            on_root_core: true,
            idles_tree: false,
            tree_root: true,
            tree_share: None,
            memory_limit: 0,
            output_limit: 0,
            limit_handler: Value::null(),
            limit_reached: None,
            fatal_reserve: 0,
            cpu_limit: 0,
            fatal_reserve_time: 0,
            cpu_slice_open: false,
            uncaught_handler: Value::null(),
            shutdown_handler: Value::null(),
            exit_hooks: Vec::new(),
            exit_hooks_drained: false,
            ending: None,
            exit_drain: None,
            max_script_depth: Self::DEFAULT_MAX_SCRIPT_DEPTH,
            spawn_clamp: None,
            script_depth: 0,
            task_slot: None,
            deferred: Some(Vec::new()),
            holds_deferred_slot: false,
            later: None,
            in_later: false,
            in_deferred: false,
            task_child: false,
            main_ended: false,
            slotted: false,
            pending: None,
            site_frame_pending: false,
            pending_fatal: false,
            runtime_error_class: None,
            core_classes: None,
            output,
            diagnostic: OutputSink::Stderr,
            log: LogTarget::Unread,
            log_minimum: Level::Debug,
            log_format: LogFormat::Json,
            origin: None,
            module_links: None,
            commands: None,
            routes: None,
            arguments: Vec::new(),
            run_input: None,
            program_name: String::new(),
            program_id: None,
            config: None,
            grant_filter: None,
            own_file: 0,
            trace_context,
            fixed_clock: None,
            test_env: None,
            test_records: None,
            owned_by_test: false,
            random_state: None,
            test_server: None,
            scripted_answers: std::collections::VecDeque::new(),
            faked_http: crate::ctx::AnswerTable::default(),
            captures: Vec::new(),
            body_chunks: Vec::new(),
            content_type: None,
            inline_debug: Vec::new(),
            style_nonce: None,
            file_body: None,
            file_body_bytes: 0,
            body_stream: None,
            stream_window: None,
            stream_must_end: None,
            stream_ended: false,
            event_stream: None,
            status: None,
            headers: DeclaredHeaders::default(),
            inbound: None,
            run_kind: RunKind::Command,
            stmt_hits: None,
            run_hits: None,
            query_sites: None,
            copy_sites: None,
            copy_counts: None,
            run_profile: None,
            debug_ceiling: DebugFlags::empty(),
            debug_sinks: DebugSinks::default(),
            counted_stmts: 0,
            counted_calls: 0,
            counted_gc_cycles: 0,
            trace: Vec::new(),
            yielder: std::ptr::null(),
            fault: None,
            helper_calls: 0,
            statics_store: Vec::new().into_boxed_slice(),
            unit_statics: None,
            isolate_argument: Value::null(),
            assertions: Vec::new(),
            unowned_failures: 0,
            snapshot_mismatches: Vec::new(),
            started_scripts: Vec::new(),
            open_files: Vec::new(),
            rosters: std::collections::HashMap::new(),
            spawned_children: Vec::new(),
            open_sockets: Vec::new(),
            open_readers: Vec::new(),
            open_connections: Vec::new(),
            temporary_dirs: Vec::new(),
            session: None,
            lent_session: None,
            peer: None,
            deliveries: None,
            late_inits: Vec::new(),
            late_table: std::cell::OnceCell::new(),
            live: std::rc::Rc::new(crate::object::LiveList::default()),
        };
        ctx.arm_stack_limit(base, STACK_CEILING);
        // A debug build starts every context under the shipped default memory
        // ceiling, the limit a request gets when its configuration sets none.
        // So a Rust test that calls a member runs the overshoot backstop
        // (`abi.rs`'s `overshoot_backstop`), and a member that holds past its
        // bound panics in the test that called it. `set_config` and
        // `set_memory_limit` replace it, so a served request, `nvs test` and
        // the conformance runs keep their configured limit. A test that needs
        // more memory, or none, calls `set_memory_limit` itself and says why;
        // the default is never raised for it. A release build keeps no
        // ceiling here and spends nothing.
        #[cfg(debug_assertions)]
        ctx.set_memory_limit(
            usize::try_from(nvs_config::defaults::LIMITS_MEMORY).unwrap_or(usize::MAX),
        );
        ctx
    }

    /// Open `session` on this request, replacing whatever it had open.
    ///
    /// Replacing rather than refusing, because `rule:core-api/session-roster`'s
    /// `clearAll` and a `regenerate` that finds its session gone both leave the
    /// request a fresh session this way. The refusal of a second open belongs
    /// to `openWithIdentifier`, the one member that opens explicitly.
    ///
    /// **In a `Core\Html::later` slot it fills the lent cell**, so every slot
    /// and the request read the one session the first slot opened. A slot that
    /// opened it while a sibling was parked on the same load keeps the first
    /// one, which is the same record.
    pub fn open_session(&mut self, session: Session) {
        if self.session.is_none()
            && let Some(lent) = &self.lent_session
        {
            // A sibling that finished its load first already filled it, with
            // the same record, so this copy is dropped.
            let _ = lent.open.set(session);
            return;
        }
        self.session = Some(session);
    }

    /// The session this request opened, or `None` before the first
    /// `Core\Session` call, which opens it
    /// (`rule:core-classes/a-session-opens-on-its-first-use`). In a
    /// `Core\Html::later` slot it is the request's session, lent to the slot.
    #[must_use]
    pub fn session(&self) -> Option<&Session> {
        self.session
            .as_ref()
            .or_else(|| self.lent_session.as_ref().and_then(|lent| lent.open.get()))
    }

    /// The same, to write: the members that change a value reach the record
    /// and its dirty flag through this, and nothing else may.
    ///
    /// `None` in a `Core\Html::later` slot, whose session is lent read-only:
    /// every member that writes checks that the head is open first, and throws
    /// there before it gets here.
    pub fn session_mut(&mut self) -> Option<&mut Session> {
        self.session.as_mut()
    }

    /// The copy of the request a `Core\Html::later` slot reads, and `None`
    /// outside a slot. [`Self::inbound`] falls back to it.
    pub(super) fn lent_head(&self) -> Option<&Inbound> {
        self.lent_session.as_ref().map(|lent| &lent.head)
    }

    /// Lends this request's session to the `Core\Html::later` slots about to
    /// run, open or not: [`Self::child`] copies the cell into each slot. Pair
    /// it with [`Self::reclaim_session`] once the slots have ended.
    ///
    /// A request with no inbound lends nothing, so a slot of a program
    /// answering no request still throws at its first session or request call.
    /// A slot's own context has no inbound either, so a nested `later` keeps
    /// the cell and the request its parent slot was lent.
    pub fn lend_session(&mut self) {
        let Some(head) = self.inbound.as_deref().map(Inbound::head_alone) else {
            return;
        };
        let open = std::cell::OnceCell::new();
        if let Some(session) = self.session.take() {
            let _ = open.set(session);
        }
        self.lent_session = Some(std::rc::Rc::new(LentSession { open, head }));
    }

    /// Takes back what [`Self::lend_session`] lent: the request's session,
    /// or the one a slot opened, so the write-back at the end sees it.
    pub fn reclaim_session(&mut self) {
        let Some(lent) = self.lent_session.take() else {
            return;
        };
        let open = match std::rc::Rc::try_unwrap(lent) {
            Ok(lent) => lent.open.into_inner(),
            // A slot's context not dropped yet still holds the cell. Its task
            // has ended, so nothing reads the copy again.
            Err(shared) => shared.open.get().cloned(),
        };
        if let Some(session) = open {
            self.session = Some(session);
        }
    }

    /// Close the session this request had open, leaving it with none, so the
    /// next `Core\Session` call opens it again. No member calls it: a test
    /// uses it to return a request to the state before its first use.
    pub fn close_session(&mut self) {
        self.session = None;
    }

    /// Send this request's session record, if anything changed it — `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`
    /// 's write-back, at the end of the program that opened it.
    ///
    /// Called by whoever ends a program, and called **unconditionally** by
    /// each of them: a program that started no session, or started one and
    /// only read it, is the no-op this returns on. That is the rule's "writing only
    /// when the record changed" decided in the one place that can see the
    /// flag, rather than a condition every caller would have to restate.
    ///
    /// The two callers are `nvs-host`'s isolate teardown — which is where an
    /// HTTP request ends, since a request is a root isolate — and `nvs run`'s
    /// root task, after `rule:observability/script-on-exit`'s exit hooks, because a hook is user code
    /// that may still write. Neither can reach the store itself, which is what
    /// [`Session::write_back`] is for.
    ///
    /// **Not on a cancelled task.** The send parks on the store, and a task
    /// being torn down may not park ([`crate::HelperFrame`]) — so a caller
    /// that got here through a cancellation skips this and the record is lost.
    /// That is the same answer a request whose process died gives, and the rule's
    /// last-write-wins already declines to repair a lost write.
    ///
    /// Sending clears the flag, so a second call sends nothing: the send is
    /// idempotent even where two ends could both reach it.
    pub fn end_session(&mut self) {
        let Some(write_back) = self
            .session
            .as_ref()
            .filter(|session| session.dirty)
            .map(|session| session.write_back)
        else {
            return;
        };
        if let Some(session) = self.session.as_mut() {
            session.dirty = false;
        }
        write_back(self);
    }

    /// The process status `exit`/`exit(n)` named, `0` if none ran.
    ///
    /// Read once, at the request boundary, after a [`crate::EXITED`] status
    /// came back — that constant owns why an `exit` is not a failure.
    #[must_use]
    pub fn exit_code(&self) -> i64 {
        self.exit_code
    }

    /// Records the status `exit(n)` named — `nvs_exit`'s one side effect.
    pub fn set_exit_code(&mut self, code: i64) {
        self.exit_code = code;
    }

    /// The ending the isolate's program recorded, spelled the way the seam that
    /// routes it spells one: `Ok(())` where nothing was recorded.
    ///
    /// [`Self::set_ending`] owns why a program records at all, and
    /// `nvs_stdlib::script::run_exit_hooks` is the table this is read against.
    pub fn ending(&self) -> Result<(), i32> {
        match self.ending {
            Some(status) => Err(status),
            None => Ok(()),
        }
    }

    /// Records the ABI status the isolate's program answered with.
    ///
    /// Called by the program itself, on the one path where its frame came back
    /// with a status: [`crate::script::Program`] answers a [`crate::Value`]
    /// alone, so a status not recorded here is one the classifier across the
    /// boundary cannot see. [`crate::EXITED`] is the ending that needs it — it
    /// leaves no pending message, and [`Self::exit_code`] reads `0` both for
    /// `exit(0)` and for a script that never called `exit`.
    pub fn set_ending(&mut self, status: i32) {
        self.ending = Some(status);
    }

    /// Fills the seam the exit queue is drained through.
    ///
    /// `Core\Script::onExit` is the one caller; the field's own doc owns why
    /// the pointer travels on the context, and why a registration is what fills
    /// it.
    pub fn set_exit_drain(
        &mut self,
        drain: fn(&mut Self, Result<(), i32>, Option<&crate::Thrown>),
    ) {
        self.exit_drain = Some(drain);
    }

    /// Drains the exit queue for `outcome`, through the seam a registration
    /// filled — and does nothing at all where no hook was ever registered.
    ///
    /// **Which endings run hooks is not decided here.** This end knows only
    /// that the program ended and with what;
    /// `nvs_stdlib::script::run_exit_hooks` is the one place that reads
    /// `rule:observability/three-endings-fire-the-exit-queue`'s table, builds
    /// the report and refuses the two endings that fire nothing, and it is what
    /// the pointer holds.
    pub fn drain_exit_hooks(&mut self, outcome: Result<(), i32>, thrown: Option<&crate::Thrown>) {
        let Some(drain) = self.exit_drain else {
            return;
        };
        drain(self, outcome, thrown);
    }

    /// `rule:routing/an-absolute-link-takes-a-configured-origin`'s configured origin, or `None` when this unit resolves
    /// none — see [`Self::origin`]'s field docs for why it is never sniffed.
    #[must_use]
    pub fn origin(&self) -> Option<&str> {
        self.origin.as_deref()
    }

    /// Configures this request's origin, dropping a trailing `/` so a link is
    /// the origin and the path concatenated and nothing has to decide which
    /// side owns the separator.
    ///
    /// Written **before** the request runs, by whoever resolved it: `nvs run`
    /// from the configuration, and the mount that accepted the request once
    /// there is a server. Nothing on the request path calls this, which is
    /// what makes `rule:routing/an-absolute-link-takes-a-configured-origin`'s
    /// "configured, never sniffed" a property of the shape
    /// rather than of a review.
    pub fn set_origin(&mut self, origin: &str) {
        self.origin = Some(origin.trim_end_matches('/').into());
    }

    /// Where `handler`, a route of another module, is served from the mount
    /// this request came through, or `None` where no mount serves it or no
    /// door wrote a table (`rule:routing/a-link-reaches-another-module`).
    #[must_use]
    pub fn module_link(&self, handler: &str) -> Option<&nvs_config::mount::Link> {
        self.module_links.as_deref()?.get(handler)
    }

    /// Gives this request the link table of the mount it came through. Written
    /// before the request runs, by the door, as [`Self::set_origin`] is.
    pub fn set_module_links(&mut self, links: std::sync::Arc<nvs_config::mount::Links>) {
        self.module_links = Some(links);
    }

    /// This request's configuration, or `None` on a context nobody configured —
    /// see [`Self::config`]'s field docs.
    #[must_use]
    pub fn config(&self) -> Option<&nvs_config::Request> {
        self.config.as_ref()
    }

    /// The `[http]` block this request's response is built from, or `None`
    /// where it set no `[http.headers]` or `[http.cors]` setting, or nobody
    /// configured the context
    /// (`rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`).
    /// [`nvs_config::Request::response_http`] owns the fold and its cost.
    #[must_use]
    pub fn response_http(&self) -> Option<crate::host::ResponseHttp> {
        self.config.as_ref()?.response_http().map(Box::new)
    }

    /// The same, for the two members that write the overlay
    /// (`Core\Config::set` and `restore`).
    pub fn config_mut(&mut self) -> Option<&mut nvs_config::Request> {
        self.config.as_mut()
    }

    /// Hands this request the snapshot it will read for its whole life —
    /// `rule:config/the-config-is-an-immutable-snapshot`'s one clone, taken before the program runs.
    ///
    /// Written by whoever resolved the tree, exactly as [`Self::set_origin`] is
    /// and for the same reason: nothing on the request path may re-read the
    /// configuration, or two reads in one request could disagree.
    ///
    /// The configuration is server work where this context already is — its
    /// kind of run, or the configuration it replaces — because that decides
    /// the CPU ceiling an unwritten `[limits] cpu_time` gives
    /// ([`nvs_config::Request::serves`]).
    ///
    /// The app folder the configuration it replaces carried stays, for the same reason
    /// ([`Self::set_app_folder`]).
    ///
    /// The same answer decides the probes (`rule:testing/debug-mode-directive`). A context that is
    /// not server work arms the [`DebugFlags`] `[debug] mode` names, and drops the ones the
    /// configuration it replaces armed; a bit its host turned on itself stays on. The ceiling
    /// `Core\Debug` starts a probe under moves the same way ([`Self::start_probes`]). Server work arms
    /// none, because a server refuses a tree that names any (`nvs_config::debug`).
    pub fn set_config(&mut self, snapshot: std::sync::Arc<nvs_config::Snapshot>) {
        let serving = self.run_kind().is_server_work()
            || self
                .config
                .as_ref()
                .is_some_and(nvs_config::Request::serves);
        let mut config = nvs_config::Request::new(snapshot);
        if serving {
            config.serve();
        }
        if let Some(folder) = self.config.as_ref().and_then(nvs_config::Request::folder) {
            config.set_folder(std::sync::Arc::clone(folder));
        }
        let replaced = self.config.as_ref().map_or(DebugFlags::empty(), armed_by);
        self.debug = (self.debug - replaced) | armed_by(&config);
        self.debug_ceiling = (self.debug_ceiling - replaced) | armed_by(&config);
        self.config = Some(config);
        self.refresh_limits();
        // `[debug] store` may have turned on or off with this snapshot, and it decides whether
        // this context counts copies.
        self.arm_copy_counts();
        // The log target is read out of the snapshot that just arrived, not out
        // of the one this context was built with. Dropping whatever was
        // resolved is what makes the read happen once *after* configuration
        // rather than once per context, and it closes the sink a previous
        // snapshot opened.
        self.log = LogTarget::Unread;
    }

    /// This request's place in a distributed trace — `rule:observability/a-trace-id-exists-for-every-request`, and never
    /// `None`, because that rule has an id exist for every request.
    #[must_use]
    pub fn trace_context(&self) -> &crate::trace_context::TraceContext {
        &self.trace_context
    }

    /// Continues the trace an inbound request arrived carrying, replacing the
    /// root [`Self::new`] drew.
    ///
    /// Written before the program runs, exactly as [`Self::set_config`] is: the
    /// id appears in log records and on the response, so a second write
    /// mid-request would split one request across two traces.
    pub fn set_trace_context(&mut self, trace: crate::trace_context::TraceContext) {
        self.trace_context = trace;
    }

    /// This program's command table, or `None` for one that declares no
    /// `#[Command]` — see [`Self::commands`]'s field docs, and
    /// [`crate::commands`] for why the two absences are one case.
    #[must_use]
    pub fn commands(&self) -> Option<&crate::commands::CommandTable> {
        self.commands.as_deref()
    }

    /// Hands this program the table the compiler built for it — `rule:tooling/commands-are-compiled`,
    /// written before the program runs exactly as [`Self::set_config`] is.
    pub fn set_commands(&mut self, table: std::sync::Arc<crate::commands::CommandTable>) {
        self.commands = Some(table);
    }

    /// This program's route table, or `None` for one that declares no
    /// `#[Route]` — see [`Self::routes`]'s field docs, and [`crate::routes`]
    /// for why the rows cross as a runtime value.
    #[must_use]
    pub fn routes(&self) -> Option<&crate::routes::Routes> {
        self.routes.as_deref()
    }

    /// Hands this program the route table the compiler built for it — `rule:routing/matched-once-before-the-handler`
    /// , written before the program runs exactly as [`Self::set_commands`]
    /// is.
    pub fn set_routes(&mut self, table: std::sync::Arc<crate::routes::Routes>) {
        self.routes = Some(table);
    }

    /// The `Core`-class resolver this context was booted with, or `None` where
    /// no embedder installed one — what [`Self::set_core_classes`] wrote, and
    /// what a derived context copies so a child resolves the names its parent
    /// could.
    #[must_use]
    pub fn core_classes(&self) -> Option<crate::ctx::CoreClasses> {
        self.core_classes
    }

    /// Hands this program the `Core` classes, written before it runs exactly as
    /// [`Self::set_commands`] is — the difference being that this table is the
    /// **process's** rather than this unit's compile product, so what crosses
    /// is a function pointer and not an `Arc`.
    ///
    /// Installing it is what makes an encoded `Core` instance resolvable on the
    /// way back in ([`Self::class_desc`], and `crate::graph`'s `decode`); a
    /// context that never gets one still runs, and refuses such a payload by
    /// name as it always did.
    pub fn set_core_classes(&mut self, resolve: crate::ctx::CoreClasses) {
        self.core_classes = Some(resolve);
    }

    /// This process's argument vector past the program itself — see
    /// [`Self::arguments`]'s field docs.
    #[must_use]
    pub fn command_line(&self) -> &[String] {
        &self.arguments
    }

    /// Hands this program the words it was started with, before it runs.
    pub fn set_command_line(&mut self, arguments: Vec<String>) {
        self.arguments = arguments;
    }

    /// Makes this context the child of a `Core\Test::run`, whose standard
    /// input is `input` — see [`Self::run_input`]'s field docs.
    pub fn set_run_input(&mut self, input: Vec<u8>) {
        self.run_input = Some(Box::new(std::io::Cursor::new(input)));
    }

    /// Whether this context is the child of a `Core\Test::run`.
    #[must_use]
    pub fn in_run(&self) -> bool {
        self.run_input.is_some()
    }

    /// The rest of a run child's standard input, for `Core\IO::stdin` and
    /// `Core\IO::stdinStream` to read from. `None` outside a run.
    pub fn run_input_mut(&mut self) -> Option<&mut std::io::Cursor<Vec<u8>>> {
        self.run_input.as_deref_mut()
    }

    /// The name a completion script registers this program against — what
    /// `rule:tooling/commands-are-compiled`'s
    /// `Core\Command::completions` writes into `complete -F … <name>`,
    /// `complete -c <name>` and `-CommandName <name>`.
    ///
    /// **Which name that is, the launcher decides**, because only the site that
    /// starts a program can tell the two invocations apart:
    ///
    /// * `nvs run script.nvs` — the **script's** own stem, `script`, and never
    ///   `nvs`. The word the shell saw is `nvs`, but a script registered
    ///   against it would answer for the toolchain: every other `nvs run` would
    ///   then complete against this program's command table.
    /// * `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s
    ///   single-file executable — the **executable's** own stem, because there
    ///   the binary *is* the program, and its entry file is a synthetic path
    ///   inside the payload that no shell has ever seen.
    ///
    /// Empty for every context nobody wrote one onto, which is every served
    /// request — an HTTP request is not something a shell completes — and the
    /// member refuses rather than inventing a name. [`Self::command_line`] is
    /// empty there for the same reason.
    #[must_use]
    pub fn program_name(&self) -> &str {
        &self.program_name
    }

    /// Hands this program the name the shell knows it by, before it runs.
    pub fn set_program_name(&mut self, name: String) {
        self.program_name = name;
    }

    /// `rule:programs/no-runtime-autoload`'s `Core\Program::id()`: the 64 lowercase hex characters naming
    /// this program's exact code and environment, or empty for a context no
    /// host wrote one onto — see [`Self::program_id`]'s field docs for the
    /// formula and for why nothing here recomputes or truncates it.
    #[must_use]
    pub fn program_id(&self) -> &str {
        self.program_id.as_deref().unwrap_or_default()
    }

    /// Hands this program its identity, before it runs — computed by
    /// `nvs_config::cache::program_id` where the resolved graph and the
    /// environment digest are both in hand, which is the only place they are.
    pub fn set_program_id(&mut self, id: std::sync::Arc<str>) {
        self.program_id = Some(id);
    }

    /// `rule:testing/determinism-declared-on-the-test`'s fixed clock in nanoseconds since the Unix epoch, or
    /// `None` for a context that reads the host's — see [`Self::fixed_clock`]'s
    /// field docs.
    #[must_use]
    pub fn fixed_clock(&self) -> Option<i128> {
        self.fixed_clock
            .as_deref()
            .map(crate::test_clock::TestClock::now)
    }

    /// The shared clock behind [`Self::fixed_clock`], for a member that parks
    /// on one of its deadlines: a sleep or a timeout of the test's task tree.
    #[must_use]
    pub fn test_clock(&self) -> Option<&std::rc::Rc<crate::test_clock::TestClock>> {
        self.fixed_clock.as_ref()
    }

    /// Fixes this context's wall clock at `nanos` nanoseconds since the Unix
    /// epoch.
    ///
    /// Called twice for two reasons that are deliberately one method: the test
    /// runner arming a `#[Test(at: …)]` isolate before its program runs, and
    /// `Core\Test::advance` moving that reading forward from inside the test.
    /// **Neither of them is on a request path** — there is no way to reach this
    /// from a program that is not a test, because the member that reaches it
    /// throws without a clock already fixed.
    ///
    /// A context that already has a clock moves it, so every task of the tree
    /// reads the move and every deadline it passes wakes; one with none gets a
    /// clock of its own.
    ///
    /// This crate makes no claim about `nanos` being a representable instant:
    /// the range belongs to the calendar library, which is `nvs-stdlib`'s
    /// (`nvs_stdlib::time`), and both callers check it there before calling.
    pub fn set_fixed_clock(&mut self, nanos: i128) {
        match &self.fixed_clock {
            Some(clock) => clock.set(nanos),
            None => self.fixed_clock = Some(crate::test_clock::TestClock::new(nanos)),
        }
    }

    /// `#[Test(env:)]`'s variables, or `None` outside a test that set some — see
    /// [`Self::test_env`]'s field docs.
    #[must_use]
    pub fn test_env(&self) -> Option<&std::rc::Rc<[(String, String)]>> {
        self.test_env.as_ref()
    }

    /// Arms this context with a test's `env:` variables, before its program runs. The runner
    /// calls it on the test's isolate and on each request and run that test makes.
    pub fn set_test_env(&mut self, vars: std::rc::Rc<[(String, String)]>) {
        self.test_env = Some(vars);
    }

    /// The records of the test that owns this context, or `None` outside `nvs test` — see
    /// [`Self::test_records`]'s field docs.
    #[must_use]
    pub fn test_records(&self) -> Option<&std::rc::Rc<crate::test_records::TestRecords>> {
        self.test_records.as_ref()
    }

    /// Arms this context with a test's records, before its program runs. The runner calls it on
    /// the suite's context, on each test's isolate, and on each request and run that test makes.
    pub fn set_test_records(&mut self, records: std::rc::Rc<crate::test_records::TestRecords>) {
        self.test_records = Some(records);
    }

    /// Whether a `#[Test]` method owns this context — see
    /// [`Self::owned_by_test`]'s field docs.
    #[must_use]
    pub fn owned_by_test(&self) -> bool {
        self.owned_by_test
    }

    /// Marks this context as one `#[Test]` method's isolate, before its
    /// program runs. The one caller is `nvs_cli::runner`.
    pub fn set_owned_by_test(&mut self) {
        self.owned_by_test = true;
    }

    /// `rule:testing/in-process-request`'s ephemeral listener, as the base URL a test reaches it
    /// at — `None` for every context no `#[Test(server: true)]` armed, which
    /// is every context but one. See [`Self::test_server`]'s field docs.
    #[must_use]
    pub fn test_server(&self) -> Option<&str> {
        self.test_server.as_deref()
    }

    /// Names the listener this test's runner bound, before its program runs.
    ///
    /// The one caller is `nvs_cli::runner`, arming a `#[Test(server: true)]`
    /// isolate on the child's own context — the same place and the same moment
    /// [`Self::set_fixed_clock`] is called from, and for the same reason. There
    /// is no way to reach this from a program: the member that reads it answers
    /// `null` where nothing was armed, so a program cannot invent an address
    /// nothing is listening on.
    pub fn set_test_server(&mut self, url: String) {
        self.test_server = Some(url);
    }

    /// `rule:testing/determinism-declared-on-the-test`'s seeded generator's live state, or `None` for a context
    /// that draws from the operating system — see [`Self::random_state`]'s
    /// field docs for why this is unreachable outside a test.
    #[must_use]
    pub fn random_state(&self) -> Option<u64> {
        self.random_state
    }

    /// Puts this context's draws on a seeded generator starting at `state`.
    ///
    /// Called by the test runner arming a `#[Test(seed: …)]` isolate, and then
    /// once per draw by `nvs_stdlib::random` writing the advanced state back.
    /// Those are deliberately one method: a seed *is* a starting state, so a
    /// second entry point would be a second place for the two to disagree about
    /// which draw a sequence begins at.
    pub fn set_random_state(&mut self, state: u64) {
        self.random_state = Some(state);
    }

    /// Adds `answers` to the tail of this context's scripted answer queue —
    /// `rule:tooling/a-prompt-is-a-core-member`'s last paragraph, as `Core\Test::answerCli`.
    ///
    /// The tail rather than a replacement, because a queue that discarded what
    /// it had not reached yet would make two calls scripting two halves of one
    /// flow depend on how far the first half got. There is no member that
    /// empties it: a queue nothing drained dies with the isolate that holds it,
    /// which is the same lifetime [`Self::fixed_clock`] has.
    pub fn script_answers(&mut self, answers: impl IntoIterator<Item = String>) {
        self.scripted_answers.extend(answers);
    }

    /// Whether a prompt asked right now would be answered from the queue rather
    /// than from a terminal.
    ///
    /// Read by `nvs_stdlib::cli` where a prompt decides whether it has anyone
    /// to ask *before* it asks — `select` builds no menu for a question nobody
    /// can answer — so the two questions have to be askable separately from
    /// [`Self::take_scripted_answer`], which consumes.
    #[must_use]
    pub fn has_scripted_answer(&self) -> bool {
        !self.scripted_answers.is_empty()
    }

    /// Takes the oldest scripted answer, or `None` for a context with none.
    ///
    /// `None` is the ordinary state and not a failure: every context outside a
    /// test has one, and a test that scripted fewer answers than its subject
    /// asks questions gets `rule:tooling/a-prompt-never-blocks`'s unattended answer for the rest — the default,
    /// or `Core\Cli\NotInteractive` — which is what makes "too few answers" a
    /// thing a test can assert rather than a hang.
    pub fn take_scripted_answer(&mut self) -> Option<String> {
        self.scripted_answers.pop_front()
    }

    /// `rule:testing/an-outbound-call-is-answered-from-a-table`'s table, as
    /// `Core\Test::answerHttp` fills it and `Core\Http\Client` reads it — see
    /// [`crate::ctx::AnswerTable`] for the matching, and [`Self::faked_http`]'s
    /// field docs for why it is scoped to one test's isolate.
    #[must_use]
    pub fn faked_http(&self) -> &crate::ctx::AnswerTable {
        &self.faked_http
    }

    /// The same table, to register an answer in or to write a call down in.
    ///
    /// One accessor for both writes rather than a method per operation: a
    /// registration and a record are the two halves of one mechanism, and the
    /// decisions in either — which row answers a URL, what order the calls are
    /// handed back in — belong to the table rather than to the context holding
    /// it.
    pub fn faked_http_mut(&mut self) -> &mut crate::ctx::AnswerTable {
        &mut self.faked_http
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
}

/// The probes `config`'s `[debug] mode` arms: each name `nvs_config::debug::NAMES` accepts, as its
/// bit, and none for a configuration that serves.
fn armed_by(config: &nvs_config::Request) -> DebugFlags {
    if config.serves() {
        return DebugFlags::empty();
    }
    nvs_config::debug::named(&config.snapshot().config)
        .iter()
        .map(|name| match name.as_str() {
            "coverage" => DebugFlags::COVERAGE,
            "branch" => DebugFlags::BRANCH,
            "trace" => DebugFlags::TRACE,
            "profile" => DebugFlags::PROFILE,
            _ => DebugFlags::empty(),
        })
        .fold(DebugFlags::empty(), |armed, bit| armed | bit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_context_is_under_the_default_memory_ceiling() {
        let ctx = Ctx::new(OutputSink::Stdout);
        assert_eq!(
            ctx.memory_limit() as u64,
            nvs_config::defaults::LIMITS_MEMORY,
            "a debug build arms the shipped default, so a member past its bound panics in its test"
        );
        assert!(
            ctx.overshoot_bound().is_some(),
            "the backstop judges every member call a test makes"
        );
    }

    fn written(names: &[&str]) -> std::sync::Arc<nvs_config::Snapshot> {
        let mut snapshot = nvs_config::Snapshot::default();
        snapshot.config.debug = Some(nvs_config::tree::Debug {
            mode: Some(names.iter().map(ToString::to_string).collect()),
            ..nvs_config::tree::Debug::default()
        });
        std::sync::Arc::new(snapshot)
    }

    #[test]
    fn a_command_line_run_arms_the_probes_its_configuration_names() {
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::COUNT);
        ctx.set_config(written(&["coverage", "trace"]));
        assert_eq!(
            ctx.debug_flags(),
            DebugFlags::COUNT | DebugFlags::COVERAGE | DebugFlags::TRACE
        );
        // A later configuration drops what the first armed, and keeps the host's own bit.
        ctx.set_config(written(&["profile"]));
        assert_eq!(ctx.debug_flags(), DebugFlags::COUNT | DebugFlags::PROFILE);
    }

    #[test]
    fn server_work_arms_no_probe_whatever_its_configuration_names() {
        let mut ctx = Ctx::buffered();
        ctx.set_run_kind(RunKind::Job);
        ctx.set_config(written(&["coverage", "branch", "trace", "profile"]));
        assert!(ctx.debug_flags().is_empty());
    }

    /// `rule:programs/no-runtime-autoload`'s id is host-written like everything else in this file: a
    /// context nobody handed one to has none, and one that was handed one
    /// answers with exactly those characters — this layer neither computes nor
    /// shortens an id.
    #[test]
    fn a_program_id_is_written_by_the_host_and_never_shortened() {
        let mut ctx = Ctx::buffered();
        assert!(ctx.program_id().is_empty());

        let id = "0123456789abcdef".repeat(4);
        ctx.set_program_id(id.as_str().into());
        assert_eq!(ctx.program_id(), id);
    }

    #[test]
    fn a_fresh_context_has_nothing_set() {
        let ctx = Ctx::buffered();
        assert!(ctx.safepoint_flags().is_empty());
        assert!(ctx.debug_flags().is_empty());
        assert!(!ctx.deadline_expired());
        assert!(ctx.pending().is_none());
    }
}
