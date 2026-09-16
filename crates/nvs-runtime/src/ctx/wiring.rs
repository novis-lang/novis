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

use super::*;

/// The session one request has open — `rule:http-server/a-session-is-loaded-once-and-written-whole`'s "loaded once and written
/// whole", as what a request holds between `Core\Session::start` and the
/// write-back at its end.
///
/// **Deliberately not a [`Value`].** The record crosses the store boundary as
/// `rule:classes/two-copy-depths`'s
/// byte carrier in both directions — `rule:http-server/a-session-store-answers-four-operations` says so, for the same reason
/// a `Core\Cache` entry does — so holding the bytes means the context has
/// nothing to release at teardown and holds no object that could name a
/// `ClassDesc` an `rule:config/an-edit-reaches-the-next-request-without-a-restart`
/// unit swap has retired. Decoding is [`crate::decode`]'s, once per member that
/// reads, over bytes this struct already owns.
///
/// What it spends: one identifier and one decoded record per **in-flight**
/// request that started a session, freed with the request — never O(sessions
/// served).
#[derive(Debug)]
pub struct Session {
    /// The identifier the store issued, as it rides in the cookie.
    pub id: String,
    /// The record under [`Self::id`], as the byte carrier wrote it.
    ///
    /// **Empty is the empty session.** A record with no keys is zero bytes
    /// rather than the encoding of an empty map, so `start` can mint one
    /// without building a `Value` to encode, and a store that answered an empty
    /// entry and one that answered a freshly minted one are the same record.
    pub record: Vec<u8>,
    /// Whether anything has changed [`Self::record`] since it was loaded.
    ///
    /// § 4's "writing only when the record changed" is what keeps a read-only
    /// request off the write path: a request that starts a session and reads it
    /// makes one round trip, not two.
    pub dirty: bool,
    /// How [`Self::record`] is sent when the program that opened it ends —
    /// § 4's write-back, travelling **with the record** rather than being
    /// reached for at the end.
    ///
    /// It has to travel, because none of the three crates involved can name
    /// the other two. The store is `nvs-stdlib`'s — § 2's four operations are
    /// over `Core\Cache`'s wire — while the two places a program *ends* are
    /// `nvs-host`'s isolate teardown, which an HTTP request is
    /// (`rule:security/isolate-shares-nothing`), and
    /// `nvs run`'s root task; neither of those crates depends on `nvs-stdlib`,
    /// and `nvs-stdlib` may not depend on either. This crate is the one all of
    /// them already rest on, so the seam is inverted through it exactly as
    /// [`crate::host`]'s own § 1 inverts the scheduler's, and the pointer is
    /// filled in by `Core\Session::start`, the only member that opens a record
    /// at all.
    ///
    /// A bare `fn` rather than a boxed closure: there is one implementation
    /// and it captures nothing — everything it needs is on the [`Ctx`] it is
    /// handed — so a `dyn FnOnce` would be an allocation per request to say
    /// what one word already says.
    pub write_back: fn(&mut Ctx),
}

impl Ctx {
    /// How many objects this context has allocated and not yet dismantled —
    /// what [`crate::object::sweep`] would have to take apart if the context
    /// ended now.
    #[cfg(test)]
    pub(crate) fn live_objects(&self) -> usize {
        self.live.count()
    }

    /// A context writing to the given sink, with nothing pending and every
    /// flag clear.
    #[must_use]
    pub fn new(output: OutputSink) -> Self {
        // The address of a local is a stack address in *this* frame, which is
        // the closest thing to "where the request starts" that needs no
        // platform call. It under-reports the true base by however deep the
        // caller already is, which shrinks the ceiling rather than stretching
        // it — the safe direction.
        let anchor = 0_u8;
        let base = std::ptr::from_ref(&anchor) as usize;
        // The tree's shared state before the struct that names its word: what
        // the hot slot holds is an address inside this allocation, and moving
        // the handle into the field below leaves the allocation exactly where it
        // is.
        let tree = std::sync::Arc::new(TreeState::default());
        let mut ctx = Self {
            safepoint: std::ptr::from_ref(&tree.word),
            debug: DebugFlags::empty(),
            deadline: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            stack_limit: 0,
            stack_floor: 0,
            statics: std::ptr::null_mut(),
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
            output_base: crate::budget::written_bytes(),
            // A context is on its tree's root core until something says
            // otherwise, and the only thing that can is `Ctx::join_tree` on the
            // core the child was placed on.
            on_root_core: true,
            tree_share: None,
            memory_limit: 0,
            output_limit: 0,
            limit_handler: Value::null(),
            fatal_reserve: 0,
            cpu_limit: 0,
            fatal_reserve_time: 0,
            uncaught_handler: Value::null(),
            shutdown_handler: Value::null(),
            exit_hooks: Vec::new(),
            exit_hooks_drained: false,
            ending: None,
            exit_drain: None,
            max_script_depth: Self::DEFAULT_MAX_SCRIPT_DEPTH,
            script_depth: 0,
            deferred: Some(Vec::new()),
            holds_deferred_slot: false,
            pending: None,
            runtime_error_class: None,
            core_classes: None,
            output,
            diagnostic: OutputSink::Stderr,
            log: LogTarget::Unread,
            log_minimum: Level::Debug,
            log_format: LogFormat::Json,
            origin: None,
            commands: None,
            routes: None,
            arguments: Vec::new(),
            program_name: String::new(),
            program_id: String::new(),
            config: None,
            grant_filter: None,
            trace_context: crate::trace_context::TraceContext::started(),
            fixed_clock: None,
            random_state: None,
            test_server: None,
            scripted_answers: std::collections::VecDeque::new(),
            faked_http: crate::ctx::AnswerTable::default(),
            captures: Vec::new(),
            content_type: None,
            file_body: None,
            body_stream: None,
            event_stream: None,
            status: None,
            headers: Vec::new(),
            inbound: None,
            stmt_hits: Vec::new(),
            trace: Vec::new(),
            yielder: std::ptr::null(),
            fault: None,
            helper_calls: 0,
            statics_store: Vec::new().into_boxed_slice(),
            unit_statics: None,
            isolate_argument: Value::null(),
            assertions: Vec::new(),
            snapshot_mismatches: Vec::new(),
            started_scripts: Vec::new(),
            open_files: Vec::new(),
            spawned_children: Vec::new(),
            open_sockets: Vec::new(),
            open_readers: Vec::new(),
            open_connections: Vec::new(),
            temporary_dirs: Vec::new(),
            session: None,
            peer: None,
            deliveries: None,
            live: std::rc::Rc::new(crate::object::LiveList::default()),
        };
        ctx.arm_stack_limit(base, STACK_CEILING);
        ctx
    }

    /// Open `session` on this request, replacing whatever it had open.
    ///
    /// Replacing rather than refusing, because `rule:core-api/session-roster`'s `regenerate` is
    /// exactly this call under a new identifier. The refusal of a *second*
    /// `start` belongs to that member, which is the only one that can tell an
    /// accidental re-open from a deliberate one.
    pub fn open_session(&mut self, session: Session) {
        self.session = Some(session);
    }

    /// The session this request started, or `None` before `Core\Session::start`
    /// — which is what every other member of that class throws on.
    #[must_use]
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// The same, to write: `set`, `remove` and `clear` reach the record and its
    /// dirty flag through this, and nothing else may.
    pub fn session_mut(&mut self) -> Option<&mut Session> {
        self.session.as_mut()
    }

    /// Close the session this request had open, leaving it with none.
    ///
    /// `rule:core-api/session-roster`'s `destroy` is the only caller, and closing rather than
    /// emptying is the honest answer: the record is gone from the store, so a
    /// record left on the request would be a copy of something that no longer
    /// exists — and § 4's write-back would put it straight back. What a member
    /// sees afterwards is exactly what it sees before `start`, which is the
    /// throw that names it.
    pub fn close_session(&mut self) {
        self.session = None;
    }

    /// Send this request's session record, if anything changed it — `rule:http-server/a-session-is-loaded-once-and-written-whole`
    /// 's write-back, at the end of the program that opened it.
    ///
    /// Called by whoever ends a program, and called **unconditionally** by
    /// each of them: a program that started no session, or started one and
    /// only read it, is the no-op this returns on. That is § 4's "writing only
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
    /// That is the same answer a request whose process died gives, and § 4's
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
    /// what makes § 6's "configured, never sniffed" a property of the shape
    /// rather than of a review.
    pub fn set_origin(&mut self, origin: &str) {
        self.origin = Some(origin.trim_end_matches('/').into());
    }

    /// This request's configuration, or `None` on a context nobody configured —
    /// see [`Self::config`]'s field docs.
    #[must_use]
    pub fn config(&self) -> Option<&nvs_config::Request> {
        self.config.as_ref()
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
    pub fn set_config(&mut self, snapshot: std::sync::Arc<nvs_config::Snapshot>) {
        self.config = Some(nvs_config::Request::new(snapshot));
        self.refresh_limits();
        // The log target is read out of the snapshot that just arrived, not out
        // of the one this context was built with. Dropping whatever was
        // resolved is what makes the read happen once *after* configuration
        // rather than once per context, and it closes the sink a previous
        // snapshot opened.
        self.log = LogTarget::Unread;
    }

    /// This request's place in a distributed trace — `rule:observability/a-trace-id-exists-for-every-request`, and never
    /// `None`, because § 2 has an id exist for every request.
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
        &self.program_id
    }

    /// Hands this program its identity, before it runs — computed by
    /// `nvs_config::cache::program_id` where the resolved graph and the
    /// environment digest are both in hand, which is the only place they are.
    pub fn set_program_id(&mut self, id: String) {
        self.program_id = id;
    }

    /// `rule:testing/determinism-declared-on-the-test`'s fixed clock in nanoseconds since the Unix epoch, or
    /// `None` for a context that reads the host's — see [`Self::fixed_clock`]'s
    /// field docs.
    #[must_use]
    pub fn fixed_clock(&self) -> Option<i128> {
        self.fixed_clock
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
    /// This crate makes no claim about `nanos` being a representable instant:
    /// the range belongs to the calendar library, which is `nvs-stdlib`'s
    /// (`nvs_stdlib::time`), and both callers check it there before calling.
    pub fn set_fixed_clock(&mut self, nanos: i128) {
        self.fixed_clock = Some(nanos);
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
    /// `rule:tooling/a-prompt-is-a-core-member`'s last paragraph, as `Core\Test::scriptAnswers`.
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
    /// asks questions gets § 4's unattended answer for the rest — the default,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `rule:programs/no-runtime-autoload`'s id is host-written like everything else in this file: a
    /// context nobody handed one to has none, and one that was handed one
    /// answers with exactly those characters — this layer neither computes nor
    /// shortens an id.
    #[test]
    fn a_program_id_is_written_by_the_host_and_never_shortened() {
        let mut ctx = Ctx::buffered();
        assert!(ctx.program_id().is_empty());

        let id = "0123456789abcdef".repeat(4);
        ctx.set_program_id(id.clone());
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
