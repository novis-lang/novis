//! The per-request context: the first argument of every compiled function and
//! every helper.
//!
//! `Ctx` is where everything that is "ambient" to running Novis code lives,
//! because [ADR 0012](../../../docs/adr/0012-no-superglobals.md) means nothing
//! is ambient to the *language*: no variable is host-populated, so the host's
//! state has to travel somewhere, and it travels here.
//!
//! # Layout is part of the ABI
//!
//! The three hot words come first, in a `#[repr(C)]` struct, because compiled
//! code loads them inline rather than calling anything:
//!
//! * [`SAFEPOINT_OFFSET`] — the safepoint poll `nvs-codegen` emits at every
//!   function entry and loop back edge (`docs/adr/README.md`'s project-start
//!   decisions). Load, test, predicted-not-taken branch to the
//!   [`nvs_safepoint`] slow path.
//! * [`DEBUG_FLAGS_OFFSET`] — [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
//!   § 1's probe check, at every statement boundary and every call site. Same
//!   shape, same cost class, and present in every compiled unit whether or not
//!   any request ever sets a bit — that is what makes coverage and tracing
//!   start/stoppable *mid-request*, which the rejected instrumented-tier
//!   design could not do.
//! * [`STACK_LIMIT_OFFSET`] — [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
//!   § 1's call-stack ceiling, compared against the stack pointer at the same
//!   emit site the safepoint poll uses. It sits in this line rather than
//!   anywhere colder precisely so the compare costs a load that is already
//!   paid for.
//! * [`DEADLINE_OFFSET`] — [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//!   § 5's deadline flag, polled from *inside* a helper whose runtime scales
//!   with its input. See *The request's deadline* below.
//! * [`STATICS_OFFSET`] — the base of this request's static-property storage,
//!   loaded inline by every `Class::$prop` read and write. See *Static
//!   properties are request-scoped* below.
//!
//! All of them are exposed as `offset_of!` constants rather than restated
//! numbers, so adding a field can never silently desynchronise codegen from
//! this struct. They fit one [`HOT_LINE_BYTES`] line together, and
//! `ctx::tests::the_hot_words_come_first_and_are_a_word_apart` is what says so.
//!
//! # The call-stack limit
//!
//! Novis compiles natively, so a user call is a real machine frame and
//! exhausting the stack is a `SIGSEGV` rather than something
//! [ADR 0002](../../../docs/adr/0002-error-propagation.md)'s checked returns
//! could carry. [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
//! § 1's answer is a bounds pair, armed per request and compared at every
//! non-leaf function entry:
//!
//! * `stack_limit` is the **soft** address. Crossing it is a catchable
//!   [`ThrownClass::Recursion`], so a recursive-descent parser over
//!   untrusted-depth input degrades instead of killing the request.
//! * `stack_floor` is the **hard** address, [`STACK_RESERVE`] further down.
//!   Crossing it is a [`crate::FATAL`] no `catch` sees, like every other
//!   resource limit. The reserve between the two is the room the throw has to
//!   unwind in.
//!
//! Compiled code compares against the soft address only; [`nvs_stack_check`]
//! decides which of the two it is. [`Ctx::arm_stack_limit`] is the one place
//! the pair is computed, so they cannot be written inconsistently.
//!
//! **Known gap: the ceiling is asserted, not discovered.** [`Ctx::new`] arms
//! from the stack pointer at construction and [`STACK_CEILING`], which is
//! correct on a stack at least that deep and permissive — behaving exactly as
//! the runtime did before this existed — on a shallower one, where the guard
//! page is still reached first and `nvs-codegen`'s `enable_probestack` still
//! turns that into a clean crash rather than a stack clash. Reading a thread's
//! true bounds needs a platform call this crate has no dependency for; the
//! request's stack becomes Novis's own to size at M6, and until then an embedder
//! that knows its bounds calls [`Ctx::arm_stack_limit`] with them.
//!
//! # The request's deadline
//!
//! The safepoint word above bounds Novis code because compiled code polls it
//! between calls. A helper is *one* call, so a helper whose runtime is O(its
//! input) — a sort, a scan, an encode, a hash over a large value — runs
//! entirely inside the gap that poll leaves.
//! [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//! § 5's answer is [`Ctx::deadline_expired`]: a flag, in the line the stack
//! check has already loaded, polled from inside the loop and amortised over a
//! batch of iterations. Reading a flag rather than a clock is the whole of why
//! it is affordable — a clock read amortised over the same batch would cost
//! more than the iteration it protects.
//!
//! **It is a separate word from `safepoint` rather than another bit in it**,
//! and that is the one decision worth stating here. The safepoint word is
//! written through `&mut self` by the request's own thread; this flag's writer
//! is a timer that by construction is *not* that thread, because a helper that
//! has not returned cannot have run one. So it is an
//! [`std::sync::atomic::AtomicU64`], written through `&self`, and folding it
//! into `safepoint` would have made every safepoint write atomic to serve the
//! one word that needs it.
//!
//! `Relaxed` on both sides: no data is published behind the flag, and a poll
//! that observes it one batch late is inside the amortisation window the batch
//! already grants.
//!
//! **Who reads it is not a member's decision.** ADR 0106 § 5's first constraint
//! puts the poll in a combinator rather than in every helper that remembers to
//! ask, and that combinator is [`crate::bounded_loop`] — it owns the batch
//! ([`crate::DEADLINE_POLL_BATCH`]) and what a fired poll returns. Nothing here
//! polls on its own.
//!
//! # Static properties are request-scoped
//!
//! A `public static int $total;` has exactly one storage slot **per request**,
//! not per process: [`Ctx::install_statics`] materializes every slot from its
//! declared initializer when the request's context is armed, and [`Ctx`]'s
//! `Drop` releases them when the request ends. `docs/adr/README.md`
//! § *Decisions taken at project start* owns the decision and its reasoning;
//! what belongs here is the shape it takes.
//!
//! The slots are one flat `[Value]`, indexed by a slot number `nvs-codegen`
//! resolves at compile time from `nvs_ir::ir::Program::statics` — the same
//! "the label is resolved once, the machine sees an index" arrangement a
//! field slot already has. Compiled code loads the base out of
//! [`STATICS_OFFSET`] and indexes it, so a static read is two loads and no
//! call, exactly like a `FieldGet` after its receiver.
//!
//! **What it spends:** 16 bytes per *accessed* static property per in-flight
//! request, plus whatever a `string` or array initializer allocates — one
//! [`crate::NvsStr`] per request per string-valued static. O(in-flight
//! requests), never O(requests served), which is what `AGENTS.md`'s
//! priority-5 rule asks of any per-request allocation.
//!
//! # Output
//!
//! `Ctx` owns where `echo` writes, rather than the runtime writing to the
//! process's stdout directly. Two reasons, in `AGENTS.md`'s priority order:
//! under `nvs serve` a request's output is its HTTP response body, not a
//! process-wide stream (priority 1, request isolation); and a test can assert
//! on [`OutputSink::Buffer`] without capturing the process's real stdout
//! (priority 4).

use std::borrow::Cow;
use std::io::{self, Write};

use crate::object::{ClassDesc, ClassId, ClassTable, FieldDefault};
use crate::throwable::{Thrown, ThrownClass};
use crate::value::Value;

bitflags::bitflags! {
    /// What a safepoint poll has been asked to do.
    ///
    /// The word is checked, not the individual bits: compiled code branches on
    /// "is this non-zero", and only the [`nvs_safepoint`] slow path looks at
    /// which bit is set.
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    #[repr(transparent)]
    pub struct SafepointFlags: u64 {
        /// The request has exceeded its CPU-time budget.
        const CPU_LIMIT = 1 << 0;
        /// The client disconnected, or the request was cancelled.
        const CANCEL = 1 << 1;
        /// The cycle collector wants to stop the world.
        const COLLECT = 1 << 2;
        /// A debugger wants to break here.
        const DEBUG_BREAK = 1 << 3;
    }
}

bitflags::bitflags! {
    /// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// § 1's per-request debug-flags word.
    ///
    /// Setting a bit on a request that is already running is the whole
    /// mechanism: no recompilation, no re-resolution, no second cache key.
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    #[repr(transparent)]
    pub struct DebugFlags: u64 {
        /// Count a hit per line at every statement boundary.
        const COVERAGE = 1 << 0;
        /// Count a hit per conditional CFG edge, keyed by `nvs_ir::EdgeId`.
        const BRANCH = 1 << 1;
        /// Emit an entry/exit probe around every call.
        const TRACE = 1 << 2;
        /// Accumulate self/inclusive time around every call.
        const PROFILE = 1 << 3;
    }
}

/// The `Core` class a captured terminal sink hands its bytes back as —
/// [ADR 0088](../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
/// § 3's default row, and § 5's carrier.
///
/// Named here rather than in `nvs-stdlib`, where the class itself is declared,
/// because the *sink* is what decides the carrier and the sink lives in this
/// crate. `nvs_stdlib::cli::TEXT` takes its `name` from this constant, so the
/// class a program writes and the class [`crate::value_to_string`] renders
/// cannot drift apart.
pub const CARRIER_CLI_TEXT: &str = r"Core\Cli\Text";

/// The carrier of the **HTML** sink — ADR 0088 § 3's HTTP-request row.
///
/// Declared beside [`CARRIER_CLI_TEXT`] and unreachable until M8 attaches that
/// sink: no [`OutputSink`] variant selects it yet. It is here so the pair is
/// one fact in one file, and so [`crate::value_to_string`]'s carrier row is
/// written against the *set* of carriers rather than against the one that
/// happens to exist.
pub const CARRIER_HTML_MARKUP: &str = r"Core\Html\Markup";

/// The field slot every sink carrier holds its already-escaped bytes in.
///
/// Both carriers declare exactly one slot and this is it, so
/// [`crate::value_to_string`] can render either without asking `nvs-stdlib`
/// anything — which it could not do anyway, the dependency running
/// `nvs-stdlib` → `nvs-runtime` and not back. `nvs_stdlib::cli`'s
/// `the_carrier_slot_matches_the_registered_layout` is the check that the
/// class's own registered layout agrees with this number.
pub const CARRIER_TEXT_SLOT: usize = 0;

/// Whether `name` is a sink carrier — [`CARRIER_CLI_TEXT`] or
/// [`CARRIER_HTML_MARKUP`].
#[must_use]
pub fn is_carrier(name: &str) -> bool {
    name == CARRIER_CLI_TEXT || name == CARRIER_HTML_MARKUP
}

/// Where a request's `echo` output goes.
#[derive(Debug)]
#[non_exhaustive]
pub enum OutputSink {
    /// The process's standard output — `nvs run`'s destination.
    Stdout,
    /// The process's standard error — the *diagnostic* channel's destination,
    /// and never a request's `echo`.
    ///
    /// [ADR 0092](../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
    /// § 4 sends a CLI `Core\Debug::dump` here rather than to stdout, so
    /// `prog | jq` and `prog > out.txt` keep working while a program is being
    /// debugged. `var_dump` writing to stdout is a small thing that makes PHP
    /// CLI tools unpipeable, and there is no reason to inherit it.
    Stderr,
    /// An in-memory buffer, read back with [`Ctx::take_buffered_output`].
    ///
    /// This is what a test uses, and the shape an HTTP response body will
    /// reuse in M7.
    Buffer(Vec<u8>),
    /// Discarded.
    Sink,
}

/// Per-request state, passed to every compiled Novis function and every helper.
#[repr(C)]
#[derive(Debug)]
pub struct Ctx {
    /// Hot. Read inline by every safepoint poll; see the module docs.
    safepoint: SafepointFlags,
    /// Hot. Read inline by every ADR 0018 probe site; see the module docs.
    debug: DebugFlags,
    /// Hot. Polled from inside a helper whose runtime scales with its input —
    /// see the module docs' *The request's deadline* section. Zero while the
    /// request may keep running; any other value means its deadline has
    /// passed.
    ///
    /// Atomic because it is the one word in this struct whose writer is not
    /// the thread reading it: a helper that has not returned cannot have run
    /// the timer that expires it. `Relaxed` on both sides, because nothing is
    /// published behind the flag and a poll that observes it one batch late is
    /// already inside the amortisation window.
    deadline: std::sync::atomic::AtomicU64,
    /// Hot. Read inline by every non-leaf function entry; see the module docs'
    /// call-stack-limit section. The **soft** address: below it, a
    /// [`ThrownClass::Recursion`] throws.
    stack_limit: usize,
    /// The **hard** address beneath [`Self::stack_limit`], read only by
    /// [`nvs_stack_check`]'s slow path: below it, the request is over.
    ///
    /// Cold as far as compiled code is concerned — it is never loaded inline —
    /// but it shares the hot line anyway, because the slow path that reads it
    /// has just read the word beside it.
    stack_floor: usize,
    /// Hot. The base of [`Self::statics_store`], loaded inline by every
    /// static-property read and write — see the module docs.
    ///
    /// Null until [`Ctx::install_statics`] runs, which is safe because a unit
    /// declaring no static property emits no instruction that loads it: the
    /// slot index compiled code carries comes from `nvs_ir::Program::statics`,
    /// so there is an index only where there is a slot.
    statics: *mut Value,
    /// The process status `exit`/`exit(n)` named, `0` until one runs.
    ///
    /// Cold: written once by `nvs_exit` on the way out, read once at the
    /// request boundary. It sits beside [`Self::pending`] rather than inside
    /// it because an `exit` is not a failure and carries no message — see
    /// [`crate::EXITED`] for why it is its own status.
    exit_code: i64,
    /// What is behind a pending `THROWN` or `FATAL` status — see [`Pending`]
    /// for why one field carries both shapes rather than two sitting beside
    /// each other.
    pending: Option<Pending>,
    /// Where `echo` writes.
    output: OutputSink,
    /// Where a **diagnostic** writes — [ADR 0092](../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
    /// § 4's destination for a CLI `Core\Debug::dump`, and later for the log
    /// target's own records.
    ///
    /// A second sink rather than a fourth [`OutputSink`] variant, because the
    /// two channels differ in *where they go* and not in what is written to
    /// them: a request may capture its output ([`Self::captures`]) without
    /// capturing its diagnostics, and a dump must reach the developer whether
    /// or not a `Core\Out::capture` is in force. [`Self::write_diagnostic`] is
    /// therefore deliberately not routed through the capture stack.
    ///
    /// [`OutputSink::Stderr`] for every context by default, including a
    /// buffered one: a test that wants to *read* a dump asks for
    /// [`Self::set_diagnostic_sink`] explicitly, so a test that does not is
    /// never quietly swallowing one.
    diagnostic: OutputSink,
    /// [ADR 0102](../../../docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
    /// § 6's configured origin: the scheme and authority
    /// `Core\Router::urlAbsolute` puts in front of a link, with no trailing
    /// `/`.
    ///
    /// **Configured, never sniffed.** § 6 refuses `Host` and
    /// `X-Forwarded-Host` outright, which is why this is written *before* the
    /// request runs and nothing during it can move it — `nvs run` reads
    /// `nvs.toml`'s `[app] origin` today, and the mount that accepted the
    /// request will write it once there is a server, since
    /// [ADR 0097](../../../docs/adr/0097-development-server-and-proxied-origin.md)
    /// § 3 makes a mount's own origin win over the application's. `None` is a
    /// unit that resolves neither, and the link helper throws rather than
    /// answering with an empty authority in it.
    ///
    /// **What it spends:** two words per request, plus the origin's own bytes
    /// once — tens of them, not thousands — and nothing at all for a program
    /// that configures none.
    origin: Option<Box<str>>,
    /// `Core\Out::capture`'s buffers, innermost last —
    /// [ADR 0088](../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
    /// § 5.
    ///
    /// A **stack**, because a capture is scoped to a closure and therefore
    /// nests by call nesting; PHP's global `ob_*` stack, which can be started
    /// in one function and ended in another, is exactly what
    /// `docs/spec/01-core-library.md` § 12 removed. While it is non-empty
    /// [`Self::write_output`] appends to its last entry and the sink below
    /// sees nothing, which is that section's "`capture` always swallows".
    ///
    /// **What it spends:** nothing until a capture begins — an empty `Vec` is
    /// three words in the [`Ctx`] and no allocation — then one buffer per
    /// nesting level, holding what that level has captured, charged to the
    /// request and freed when the level ends. The cost on the `echo` path is
    /// one predictable not-taken branch, which is the priority-3 price of not
    /// giving [`OutputSink`] a fourth variant that every other writer would
    /// have to match on.
    captures: Vec<Vec<u8>>,
    /// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// § 1's statement-boundary hit counters, indexed by `nvs_ir::StmtId`.
    ///
    /// Written only from [`nvs_probe_stmt`], which compiled code reaches only
    /// when the [`DebugFlags`] word above is non-zero — so a request with no
    /// probe enabled never touches this vector and never allocates it.
    ///
    /// **A stand-in, not the final shape.** `nvs_ir::StmtId` numbers from zero
    /// within *each* function, so two functions' statements collide in this
    /// one table. ADR 0018 wants path → line → count, which needs the unit and
    /// function a statement belongs to; that qualification arrives with
    /// `Core\Debug` and the Clover/lcov exporters in M10. What this table is
    /// for now is proving the mechanism: the probe fires at exactly the
    /// statements a request executed, and nowhere else.
    stmt_hits: Vec<u64>,
    /// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// § 1's call-site trace, in the order the probes fired.
    ///
    /// Written only from [`nvs_probe_call_enter`]/[`nvs_probe_call_exit`],
    /// under the same "the flags word was non-zero" gate `stmt_hits` is under.
    ///
    /// **A stand-in, not the final shape**, for the same reason `stmt_hits`
    /// is one, plus a second: ADR 0018 has trace and profile data *stream to
    /// a sink* rather than accumulate, precisely because a long-running
    /// request's trace is call-count-proportional. This vector is bounded by
    /// nothing, which is why it exists only until `Core\Debug` names a sink —
    /// it is proving the probe fires at the right places, not serving a
    /// request. `PROFILE`'s self/inclusive timing shares these two sites and
    /// lands with that sink.
    trace: Vec<TraceEvent>,
    /// The class a bare-message failure is promoted to, if one was
    /// installed — see [`Ctx::set_runtime_error_class`].
    runtime_error_class: Option<ErrorClass>,
    /// The coroutine yielder of the task this request is running inside, or
    /// null on the main stack — `nvs-host`'s scheduler publishes it on entry
    /// and clears it before the context leaves the coroutine.
    ///
    /// **This is what removes async colouring.** A helper that has to wait
    /// reaches the yielder through the context it was already handed rather
    /// than through its own signature, so no caller up the chain is marked
    /// `async` and Novis needs no such marker at all
    /// ([ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
    /// § 6, `docs/plan/design.md` § *Thread-per-core, shared-nothing runtime*).
    ///
    /// **Opaque on purpose.** It is a `*const ()` rather than a
    /// `*const corosensei::Yielder<…>` so that the coroutine crate stays out
    /// of the crate every compiled unit links; `nvs-host` is the only code
    /// that knows what it points at, and the `unsafe` that dereferences it
    /// lives there. Writing it here is safe — a stored pointer is inert — and
    /// [`Ctx::set_yielder`] carries the contract the reader relies on.
    ///
    /// Cold as far as compiled code is concerned: nothing loads it inline, so
    /// it sits below the hot line and costs one word per request.
    yielder: *const (),
    /// The armed fault-injection site, if any. See [`FaultSite`].
    ///
    /// A request with nothing armed — every request that is not a
    /// `nvs run --fault-inject=…` — pays one `Option` test per helper entry
    /// and touches `helper_calls` never.
    fault: Option<FaultSite>,
    /// How many runtime helpers this request has entered, counted only while
    /// `fault` is armed. See [`FaultSite::HelperPanic`].
    helper_calls: u32,
    /// This request's static-property slots, owned. Cold: compiled code
    /// reaches them through [`Self::statics`], never through this field.
    ///
    /// A boxed slice rather than a `Vec` on purpose — the pointer beside it is
    /// only sound while nothing can reallocate the buffer, and a boxed slice
    /// has no `push`. [`Ctx::install_statics`] is the one place the two are
    /// written, so they cannot disagree.
    statics_store: Box<[Value]>,
    /// [ADR 0079](../../../docs/adr/0079-testing-is-a-language-feature.md)
    /// § 5's per-test assertion ledger, in the order the assertions ran.
    ///
    /// It lives here, and nowhere a program can name, because that is the
    /// whole of what makes it work: the runner reads the ledger rather than
    /// the exception state, so `try { … } catch (Throwable $t) {}` around an
    /// assertion cannot erase the fact that it failed. A `Core` member that
    /// answered "how many assertions failed" would hand the test back the very
    /// eraser this exists to take away, so there is none, and the only member
    /// that *writes* to it beyond appending is
    /// `Core\Test::expectFailure(callable)` — § 5's single greppable spelling
    /// for "this failure was on purpose".
    ///
    /// **What it spends:** one entry per assertion executed, charged to the
    /// request and freed with it, and nothing at all for a request that runs
    /// no assertion — an empty `Vec` is three words in the [`Ctx`] and no
    /// allocation. A passing assertion allocates nothing beyond the entry
    /// itself, [`AssertionOutcome::member`] being a `&'static str` the member
    /// names rather than a built string.
    assertions: Vec<AssertionOutcome>,
}

/// One entry of [ADR 0079](../../../docs/adr/0079-testing-is-a-language-feature.md)
/// § 5's ledger: an assertion that ran, and how it came out.
///
/// The outcome is the *message*, not a `bool`, because the ledger is what the
/// runner reports from — reading the exception state instead is exactly the
/// silently-passing test § 5 abolishes, and a `bool` would send it back there
/// for the wording.
#[derive(Clone, Debug)]
pub struct AssertionOutcome {
    /// Which `Core\Test` member ran — a literal the member itself names, so a
    /// passing assertion costs no allocation.
    pub member: &'static str,
    /// `None` where the assertion held; the failure's message where it did
    /// not.
    pub failure: Option<String>,
}

/// One class descriptor, plus the table that owns it.
///
/// The safe way to hand a [`Ctx`] a descriptor that must outlive it: a
/// [`ClassDesc`]'s *address* is its identity (`crate::object`), so a context
/// cannot make one up and still match a `catch` clause's — it has to be handed
/// the compiled unit's. Carrying the [`Rc`](std::rc::Rc)-shared
/// [`ClassTable`] along with the id is what turns "the table must outlive the
/// context" from a contract into a fact, which is why
/// [`Ctx::set_runtime_error_class`] needs no `unsafe` at all.
#[derive(Clone, Debug)]
pub struct ErrorClass {
    table: std::rc::Rc<ClassTable>,
    id: ClassId,
}

impl ErrorClass {
    /// A handle on `id` within `table`.
    #[must_use]
    pub fn new(table: std::rc::Rc<ClassTable>, id: ClassId) -> Self {
        Self { table, id }
    }

    /// The descriptor's address, live for as long as this handle is.
    #[must_use]
    pub fn desc(&self) -> *const ClassDesc {
        self.table.desc(self.id)
    }

    /// A handle on another class of the *same* table, by name — how
    /// [`Ctx::error_desc`] reaches spec § 10's `ParseError` from the
    /// `RuntimeError` the embedder installed.
    ///
    /// Sharing the table is the point: the returned handle keeps it alive by
    /// itself, so a `catch` clause's descriptor and this one are two addresses
    /// in the same table and compare by pointer the way `crate::object`'s
    /// conformance test requires.
    #[must_use]
    pub fn sibling(&self, name: &str) -> Option<Self> {
        Some(Self::new(
            std::rc::Rc::clone(&self.table),
            self.table.id_of(name)?,
        ))
    }
}

/// What is behind a pending non-[`crate::OK`] status.
///
/// One field on [`Ctx`], not two: the exception object **subsumes** the
/// message a [`crate::Fault`] used to leave behind, rather than sitting beside
/// it. Two fields would mean two places to ask "what failed", and every read
/// would have to state which one wins.
///
/// The two variants are not two kinds of failure — they are the same failure
/// at two levels of detail:
///
/// * [`Pending::Message`] is what a runtime helper's [`crate::Fault`] and
///   every [`crate::FATAL`] produce. It allocates nothing when the message is
///   `'static`, which is the property
///   [ADR 0002](../../../docs/adr/0002-error-propagation.md) § *Measured cost*
///   depends on: `benches/abi-probe` measured a throw at 2.8x a normal return
///   with an allocating message and *cheaper* than a return without one, and
///   PHP code throws on ordinary control-flow paths.
/// * [`Pending::Thrown`] is what Novis's own `throw` produces, and the only one
///   carrying a backtrace. A `Message` is promoted to one on demand — by
///   [`Ctx::take_thrown`] when a `catch` dispatch takes it, or by
///   [`Ctx::push_frame`] when a `THROWN` unwinds a compiled frame — so a
///   helper-raised throw is catchable and traceable without every helper
///   paying for an allocation it usually does not need.
///
/// Promotion needs a class to build the object from, which only the compiled
/// unit has — see [`Ctx::set_runtime_error_class`]. With none installed, a
/// `Message` stays a message: it is still reported, and still ends the
/// request, but no `catch` clause matches it and it grows no backtrace.
///
/// A [`crate::FATAL`] never becomes a `Thrown`: compiled code only ever pushes
/// a frame for a `THROWN` status, and no `catch` is ever entered for a
/// `FATAL` ([ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)).
#[derive(Debug)]
enum Pending {
    /// A message alone, with no exception object behind it yet, plus the
    /// class it will be promoted to — see [`ThrownClass`].
    Message(ThrownClass, Cow<'static, str>),
    /// Novis's own exception object.
    Thrown(Thrown),
}

impl Pending {
    /// The message, whichever shape this is.
    fn message(&self) -> Cow<'_, str> {
        match self {
            Self::Message(_, message) => Cow::Borrowed(message),
            Self::Thrown(thrown) => Cow::Owned(thrown.message()),
        }
    }

    /// The class a promotion would build, or `None` for a failure that is
    /// already an object.
    fn class(&self) -> Option<ThrownClass> {
        match self {
            Self::Message(class, _) => Some(*class),
            Self::Thrown(_) => None,
        }
    }

    /// This failure as an exception object, allocating one of `class` around a
    /// bare message if that is all there is.
    ///
    /// # Safety
    ///
    /// `class` must be null or refer to a live class descriptor that outlives
    /// every instance made from it.
    #[expect(
        unsafe_code,
        reason = "the descriptor's liveness is the caller's obligation and \
                  cannot be expressed in the signature"
    )]
    unsafe fn into_thrown(self, class: *const ClassDesc) -> Thrown {
        match self {
            // The class is passed through rather than dropped: it is what
            // decides whether ADR 0071 § 5's `issues` slot exists to fill —
            // see `Thrown::new_as`.
            #[expect(unsafe_code, reason = "forwarding this function's own contract")]
            Self::Message(thrown, message) => unsafe {
                Thrown::new_as(class, thrown, &message, None)
            },
            Self::Thrown(thrown) => thrown,
        }
    }
}

/// A failure a run can be *asked* to produce, for a mode that by definition
/// has no user-facing trigger.
///
/// The set is closed on purpose, and reachable only through `nvs run
/// --fault-inject=<site>`: it must never be reachable from a served request
/// (`nvs serve`, M7), and nothing in Novis source can arm one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FaultSite {
    /// The request's **second** runtime helper call panics, which
    /// [`crate::run_helper`]'s `catch_unwind` contains into a `FATAL`.
    ///
    /// Deliberately not the *first*: `echo` is itself a helper, so faulting
    /// the very first call would give a run that produced no output at all —
    /// and half of what containing an engine panic means is that what the
    /// request already produced survives it. Two is the smallest count that
    /// leaves room for a byte to have been written.
    ///
    /// It is a fixed count rather than a condition on the output for one
    /// reason: an injected fault that can silently never fire is worse than
    /// one whose site reads slightly arbitrarily. A request that enters fewer
    /// than two helpers does nothing observable at all, so there is no
    /// program this leaves un-faulted that anyone would want to fault.
    HelperPanic,
}

/// One [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
/// § 1 call-site trace record.
///
/// [ADR 0041](../../../docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md)
/// adds a `call`/`gc`/`spawn`/`query` kind alongside this; every event here is
/// a `call`, since the GC, isolate-spawn and `Core\Db` statement routines it
/// also instruments do not exist yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceEvent {
    /// The callee's `Class::method` label.
    pub callee: String,
    /// `None` on entry; on exit, the status the call site is about to branch
    /// on — so a trace records a thrown or `FATAL` exit exactly as it
    /// happened rather than as a reconstruction.
    pub status: Option<i32>,
}

/// The request ends here, and so does everything its static properties held.
///
/// This is the whole of what makes a static's storage request-scoped rather
/// than process-global: nothing outside the [`Ctx`] ever points at a slot, so
/// dropping the context is what returns the memory and drops the references —
/// there is no second owner to coordinate with and no table to clear.
impl Drop for Ctx {
    fn drop(&mut self) {
        self.release_statics();
    }
}

/// Byte offset of the safepoint word within [`Ctx`] — see the module docs.
pub const SAFEPOINT_OFFSET: usize = std::mem::offset_of!(Ctx, safepoint);

/// Byte offset of the debug-flags word within [`Ctx`] — see the module docs.
pub const DEBUG_FLAGS_OFFSET: usize = std::mem::offset_of!(Ctx, debug);

/// Byte offset of the soft call-stack limit within [`Ctx`] — see the module
/// docs.
pub const STACK_LIMIT_OFFSET: usize = std::mem::offset_of!(Ctx, stack_limit);

/// Byte offset of the deadline flag within [`Ctx`] — see the module docs'
/// *The request's deadline* section.
pub const DEADLINE_OFFSET: usize = std::mem::offset_of!(Ctx, deadline);

/// The bytes an x86-64 or AArch64 cache line holds, which is what
/// [`DEADLINE_OFFSET`] and its neighbours have to fit inside together.
///
/// Not a portability claim: a target with a wider line still holds them, and
/// one with a narrower line would cost a second load rather than be wrong.
pub const HOT_LINE_BYTES: usize = 64;

/// Byte offset of the static-property base pointer within [`Ctx`] — see the
/// module docs' *Static properties are request-scoped* section.
pub const STATICS_OFFSET: usize = std::mem::offset_of!(Ctx, statics);

/// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's
/// call-stack ceiling: **8 MiB of reserved address space per request**, of
/// which only the touched pages are ever resident.
///
/// About 65,000 frames — the same order as what PHP permits, and the default
/// Linux thread stack. Stated as
/// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md) requires: what
/// the number buys is how deep a program may recurse and how much one runaway
/// commits before it is stopped, and at `benches/abi-probe`'s measured 1.32 ns
/// per call that is ≈86 µs either way.
pub const STACK_CEILING: usize = 8 << 20;

/// The slice between [`Ctx::arm_stack_limit`]'s soft address and its hard one.
///
/// It is what a [`ThrownClass::Recursion`] unwinds in, and it is also the
/// slack that lets a **leaf** function skip the check entirely: its caller
/// passed the compare with this much stack still under it, so a frame that
/// allocates no further calls cannot cross the floor.
pub const STACK_RESERVE: usize = 256 << 10;

impl Ctx {
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
        let mut ctx = Self {
            safepoint: SafepointFlags::empty(),
            debug: DebugFlags::empty(),
            deadline: std::sync::atomic::AtomicU64::new(0),
            stack_limit: 0,
            stack_floor: 0,
            statics: std::ptr::null_mut(),
            exit_code: 0,
            pending: None,
            runtime_error_class: None,
            output,
            diagnostic: OutputSink::Stderr,
            origin: None,
            captures: Vec::new(),
            stmt_hits: Vec::new(),
            trace: Vec::new(),
            yielder: std::ptr::null(),
            fault: None,
            helper_calls: 0,
            statics_store: Vec::new().into_boxed_slice(),
            assertions: Vec::new(),
        };
        ctx.arm_stack_limit(base, STACK_CEILING);
        ctx
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

    /// ADR 0102 § 6's configured origin, or `None` when this unit resolves
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
    /// static ADR 0022 § 2 required no default of (a nullable or `lateinit`
    /// one) and starts the request at `null`.
    pub fn install_statics(&mut self, defaults: &[Option<FieldDefault>]) {
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
    }

    /// How many static-property slots this request holds — the length
    /// [`Ctx::install_statics`] was last armed with.
    #[must_use]
    pub fn statics_len(&self) -> usize {
        self.statics_store.len()
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
    fn release_statics(&mut self) {
        let store = std::mem::replace(&mut self.statics_store, Vec::new().into_boxed_slice());
        self.statics = std::ptr::null_mut();
        for value in store.into_vec() {
            unsafe { value.release() };
        }
    }

    /// Arms [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
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

    /// The pending safepoint requests.
    #[must_use]
    pub fn safepoint_flags(&self) -> SafepointFlags {
        self.safepoint
    }

    /// Whether this request's deadline has passed —
    /// [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
    /// § 5's poll, and the module docs' *The request's deadline* section owns
    /// what it costs and who may call it.
    ///
    /// One relaxed load from a line the stack check has already brought in,
    /// which is the whole reason a bounded loop can afford to ask.
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

    /// The active [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
    /// probes.
    #[must_use]
    pub fn debug_flags(&self) -> DebugFlags {
        self.debug
    }

    /// Turns probes on or off for a request that may already be running.
    pub fn set_debug_flags(&mut self, flags: DebugFlags) {
        self.debug = flags;
    }

    /// Counts one hit for the statement `stmt` names — [`nvs_probe_stmt`]'s
    /// whole effect under [`DebugFlags::COVERAGE`].
    pub fn record_stmt_hit(&mut self, stmt: u32) {
        let index = stmt as usize;
        if self.stmt_hits.len() <= index {
            self.stmt_hits.resize(index + 1, 0);
        }
        self.stmt_hits[index] += 1;
    }

    /// The per-statement hit counters gathered so far, indexed by
    /// `nvs_ir::StmtId` — empty for a request that ran with
    /// [`DebugFlags::COVERAGE`] off throughout. See the field's own doc
    /// comment for why this is a stand-in for ADR 0018's path → line → count
    /// shape rather than that shape itself.
    #[must_use]
    pub fn stmt_hits(&self) -> &[u64] {
        &self.stmt_hits
    }

    /// Records one call-site trace event — [`nvs_probe_call_enter`]/
    /// [`nvs_probe_call_exit`]'s whole effect under [`DebugFlags::TRACE`].
    pub fn record_trace(&mut self, callee: &str, status: Option<i32>) {
        self.trace.push(TraceEvent {
            callee: callee.to_owned(),
            status,
        });
    }

    /// The call-site trace gathered so far, in the order the probes fired —
    /// empty for a request that ran with [`DebugFlags::TRACE`] off
    /// throughout. See the field's own doc comment for why this accumulates
    /// in memory today and will not once ADR 0018's sink exists.
    #[must_use]
    pub fn trace(&self) -> &[TraceEvent] {
        &self.trace
    }

    /// Arms a fault-injection site for this request — see [`FaultSite`] for
    /// what each one does and why the set is closed.
    pub fn inject_fault(&mut self, site: FaultSite) {
        self.fault = Some(site);
    }

    /// Whether [`FaultSite::HelperPanic`] is armed *and* this is the helper
    /// call it names, disarming it if so. Called once per helper entry from
    /// [`crate::run_helper`]; a request with nothing armed pays one
    /// already-loaded `Option` test and nothing else.
    pub(crate) fn take_armed_helper_panic(&mut self) -> bool {
        if self.fault != Some(FaultSite::HelperPanic) {
            return false;
        }
        self.helper_calls += 1;
        if self.helper_calls < 2 {
            return false;
        }
        self.fault = None;
        true
    }

    /// Records the message behind a `THROWN` or `FATAL` status, as spec
    /// § 10's `RuntimeError`.
    pub fn set_pending(&mut self, message: impl Into<Cow<'static, str>>) {
        self.set_pending_as(ThrownClass::Runtime, message);
    }

    /// Records the message behind a `THROWN`, naming which of spec § 10's
    /// classes a `catch` will see — [`ThrownClass`] owns the roster.
    pub fn set_pending_as(&mut self, class: ThrownClass, message: impl Into<Cow<'static, str>>) {
        self.pending = Some(Pending::Message(class, message.into()));
    }

    /// Installs the class a bare-message failure is promoted to — spec
    /// § 10's `RuntimeError`, "the world said no", which is what a runtime
    /// helper's failure is.
    ///
    /// It is also this context's *anchor into the compiled unit's class
    /// table*: every other § 10 class is reached from it by name
    /// ([`ErrorClass::sibling`]), so a helper that throws a
    /// [`ThrownClass::Parse`] needs no second installation call. One handle
    /// rather than six because the six are not independent — they all come
    /// from the one table a `Unit` owns, and installing a subset would make
    /// "which classes can this request throw" a property of the embedder.
    ///
    /// A caller that never installs one gets the degraded behaviour
    /// [`Pending`] describes, never a crash.
    pub fn set_runtime_error_class(&mut self, class: ErrorClass) {
        self.runtime_error_class = Some(class);
    }

    /// The descriptor `class` names, or the installed `RuntimeError`'s if the
    /// table holds no such class, or null if none was installed at all.
    ///
    /// Falling back rather than failing is deliberate: a compiled unit always
    /// carries the whole seeded tree (`nvs_hir::errors::TREE`), so a miss here
    /// means an embedder built a table by hand — and a failure that arrives as
    /// a `RuntimeError` is strictly better than one that arrives as no object
    /// at all.
    fn error_desc(&self, class: ThrownClass) -> *const ClassDesc {
        let Some(installed) = self.runtime_error_class.as_ref() else {
            return std::ptr::null();
        };
        installed
            .sibling(class.name())
            .map_or_else(|| installed.desc(), |found| found.desc())
    }

    /// Runs `body` with this context's pending failure set aside, discarding
    /// anything `body` raised and putting the saved one back.
    ///
    /// # Decision: a throw out of a dying generator's `finally` is dropped
    ///
    /// Its one caller is [`crate::object::dismantle`], and a release has no
    /// error edge to propagate on: `nvs_object_release` answers nothing, and
    /// the release itself is very often *already* running under an exception —
    /// a landing pad dropping its locals on the way out. Leaving the throw in
    /// the pending slot would therefore either replace the exception actually
    /// in flight with one raised by a `finally` the program never resumed into
    /// by hand, or attach itself to whatever call returns next; both are worse
    /// than losing it, and the first loses the original as well. So the
    /// `finally` **runs** — which is what PHP compatibility asks for
    /// ([ADR 0053](../../../docs/adr/0053-iteration-and-generators.md) § 4) —
    /// and a throw escaping it is where this differs from PHP, which reports
    /// one as uncaught. Surfacing it wants
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)'s ladder,
    /// which does not exist yet; until it does, the safe half is the half that
    /// is kept.
    pub(crate) fn with_pending_set_aside<R>(&mut self, body: impl FnOnce(&mut Self) -> R) -> R {
        let saved = self.pending.take();
        let out = body(self);
        // Dropping a `Pending::Thrown` releases the exception object's own
        // reference, which is why this is a replace rather than an assignment.
        drop(std::mem::replace(&mut self.pending, saved));
        out
    }

    /// Records an already-built exception as the pending `THROWN` — what
    /// [`crate::nvs_raise`] does for Novis's own `throw`, taking ownership of the
    /// reference it was handed.
    pub fn raise(&mut self, thrown: Thrown) {
        self.pending = Some(Pending::Thrown(thrown));
    }

    /// Records a `THROWN` of `class` carrying `message` and
    /// [ADR 0071](../../../docs/adr/0071-derived-codecs.md) § 5's `issues`
    /// list — [`crate::Fault::ThrownWithIssues`]'s one destination.
    ///
    /// The object is built **here** rather than left as a [`Pending::Message`]
    /// to be promoted later, which is what keeps the pending state free of an
    /// owned reference: every path that replaces or discards a pending failure
    /// would otherwise have to release one, and exactly one of those paths
    /// being missed is the shape a refcount leak takes. Only a member that
    /// actually recorded an issue reaches this, so the eager allocation is on
    /// a path that has already allocated.
    ///
    /// # Safety
    ///
    /// `issues` must be a value whose reference is being transferred here.
    #[expect(
        unsafe_code,
        reason = "the value's reference and the installed descriptor's liveness \
                  are both obligations the signature cannot express"
    )]
    pub unsafe fn raise_with_issues(
        &mut self,
        class: ThrownClass,
        message: &str,
        issues: crate::Value,
    ) {
        let desc = self.error_desc(class);
        #[expect(
            unsafe_code,
            reason = "the descriptor comes from the `Rc`-shared table this \
                      context holds, so it outlives the instance; the value's \
                      reference is forwarded"
        )]
        let thrown = unsafe { Thrown::new_as(desc, class, message, Some(issues)) };
        self.raise(thrown);
    }

    /// The pending message, if any, without clearing it.
    #[must_use]
    pub fn pending(&self) -> Option<Cow<'_, str>> {
        self.pending.as_ref().map(Pending::message)
    }

    /// The class descriptor a pending failure would be caught through, or
    /// `None` where nothing is pending.
    ///
    /// The pointer may still be null: a [`Pending::Message`] raised before
    /// [`Self::set_runtime_error_class`] installed anything has no class to
    /// resolve against, which is that method's documented "no exception class
    /// installed" state.
    fn pending_desc(&self) -> Option<*const ClassDesc> {
        Some(match self.pending.as_ref()? {
            Pending::Message(class, _) => self.error_desc(*class),
            Pending::Thrown(thrown) => thrown.class_desc(),
        })
    }

    /// The name of the class a pending failure would be caught as, without
    /// clearing it — [`Self::pending`]'s companion, for a reader that has to
    /// report *what* was thrown rather than what it said.
    #[must_use]
    pub fn pending_class(&self) -> Option<String> {
        let desc = self.pending_desc()?;
        if desc.is_null() {
            return self
                .pending
                .as_ref()?
                .class()
                .map(ThrownClass::name)
                .map(str::to_owned);
        }
        #[expect(
            unsafe_code,
            reason = "the descriptor came out of the `Rc`-shared table this \
                      context holds, or off a live exception object's own \
                      header, so it outlives this borrow"
        )]
        Some(unsafe { (*desc).name() }.to_owned())
    }

    /// Whether a pending failure is an instance of the class `name` spells —
    /// **its own class or any ancestor**, which is exactly what a `catch`
    /// clause naming that class would bind.
    ///
    /// # Decision: an ancestor matches
    ///
    /// ADR 0079 § 4's `Core\Test::assertThrows` is the caller, and a test
    /// naming `RuntimeError` is claiming no more than that the failure is one
    /// — matching a subclass is what PHP's own `expectException` does, and it
    /// is the reading under which the assertion agrees with the `catch` a
    /// reader would have written by hand instead. The narrower "this exact
    /// class" reading is available to a test that wants it, by asserting the
    /// name: there is no spelling of the wider one if this answers narrowly.
    ///
    /// The ancestry itself is read off the descriptor
    /// ([`crate::ClassDesc::conforms_to_name`]) rather than off a second copy
    /// of `nvs_hir::errors::TREE` here, so the two cannot disagree about what
    /// `ParseError` descends from. `false` where nothing is pending, and where
    /// no exception class was ever installed to resolve one against.
    #[must_use]
    pub fn pending_conforms_to(&self, name: &str) -> bool {
        let Some(desc) = self.pending_desc() else {
            return false;
        };
        if desc.is_null() {
            return false;
        }
        #[expect(
            unsafe_code,
            reason = "the descriptor came out of the `Rc`-shared table this \
                      context holds, or off a live exception object's own \
                      header, so it outlives this borrow"
        )]
        unsafe {
            (*desc).conforms_to_name(name)
        }
    }

    /// Takes the pending message, clearing it — and dropping the exception
    /// object behind it, if there was one.
    #[must_use]
    pub fn take_pending(&mut self) -> Option<Cow<'static, str>> {
        Some(match self.pending.take()? {
            Pending::Message(_, message) => message,
            Pending::Thrown(thrown) => Cow::Owned(thrown.message()),
        })
    }

    /// Appends one entry to
    /// [ADR 0079](../../../docs/adr/0079-testing-is-a-language-feature.md)
    /// § 5's ledger — what every `Core\Test` assertion does, whether it held
    /// or not.
    ///
    /// Recording the *passing* ones as well is not bookkeeping for its own
    /// sake: § 20's "a test that asserts nothing fails" is a question about
    /// how many entries a test produced, and a ledger holding only failures
    /// cannot answer it.
    pub fn record_assertion(&mut self, member: &'static str, failure: Option<String>) {
        self.assertions.push(AssertionOutcome { member, failure });
    }

    /// How many entries the ledger holds — the mark
    /// [`Self::discharge_failures_from`] is given.
    #[must_use]
    pub fn assertion_count(&self) -> usize {
        self.assertions.len()
    }

    /// Removes every **failed** entry recorded at or after `mark`, answering
    /// how many there were — `Core\Test::expectFailure(callable)`'s half of
    /// ADR 0079 § 5.
    ///
    /// The passing entries in that range stay: they are assertions that really
    /// ran, and § 20 counts them. Only the failure is discharged, and only
    /// where the caller has said it expected one.
    pub fn discharge_failures_from(&mut self, mark: usize) -> usize {
        let mark = mark.min(self.assertions.len());
        let mut discharged = 0;
        let mut index = mark;
        while index < self.assertions.len() {
            if self.assertions[index].failure.is_some() {
                self.assertions.remove(index);
                discharged += 1;
            } else {
                index += 1;
            }
        }
        discharged
    }

    /// The whole ledger, taken — what the runner reads at the end of a test.
    ///
    /// Taking rather than borrowing is what makes one test's ledger that
    /// test's: the runner clears it between tests by consuming it, so a
    /// context reused across a file cannot report an earlier test's entries
    /// against a later one.
    #[must_use]
    pub fn take_assertions(&mut self) -> Vec<AssertionOutcome> {
        std::mem::take(&mut self.assertions)
    }

    /// Takes the pending failure as an exception object, clearing it — what a
    /// `catch` binds to its variable, and what `nvs run` reports a backtrace
    /// from.
    ///
    /// Promotes a bare [`Pending::Message`] rather than returning `None` for
    /// one: a helper-raised `THROWN` is as catchable as Novis's own, it just has
    /// no backtrace to show.
    #[must_use]
    pub fn take_thrown(&mut self) -> Thrown {
        let Some(pending) = self.pending.take() else {
            return Thrown::none();
        };
        let desc = pending
            .class()
            .map_or(std::ptr::null(), |class| self.error_desc(class));
        #[expect(
            unsafe_code,
            reason = "`set_runtime_error_class`'s own contract makes the \
                      installed descriptor outlive every instance built here"
        )]
        unsafe {
            pending.into_thrown(desc)
        }
    }

    /// Records one more frame a pending `THROWN` has unwound out of.
    ///
    /// A bare message is promoted to a real exception here, so a helper-raised
    /// throw accumulates a backtrace from the first compiled frame it leaves.
    pub fn push_frame(&mut self, label: &str) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        // Captured before promotion, so the message survives the one case
        // promotion cannot produce an object for — see `Pending`'s note on an
        // uninstalled class.
        let message = pending.message().into_owned();
        let class = pending.class();
        let desc = class.map_or(std::ptr::null(), |class| self.error_desc(class));
        #[expect(
            unsafe_code,
            reason = "`set_runtime_error_class`'s own contract makes the \
                      installed descriptor outlive every instance built here"
        )]
        let thrown = unsafe { pending.into_thrown(desc) };
        if thrown.is_none() {
            self.pending = Some(Pending::Message(
                class.unwrap_or_default(),
                Cow::Owned(message),
            ));
            return;
        }
        thrown.push_frame(label);
        self.pending = Some(Pending::Thrown(thrown));
    }

    /// Writes raw bytes to this request's output, unescaped.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns. [`OutputSink::Buffer`] and
    /// [`OutputSink::Sink`] never fail.
    pub fn write_output(&mut self, bytes: &[u8]) -> io::Result<()> {
        // ADR 0088 § 5: while a `Core\Out::capture` is in force, the innermost
        // one takes the bytes and the sink below sees nothing.
        if let Some(capture) = self.captures.last_mut() {
            capture.extend_from_slice(bytes);
            return Ok(());
        }
        write_to(&mut self.output, bytes)
    }

    /// Writes raw bytes to this request's **diagnostic** channel — ADR 0092
    /// § 4's destination for a CLI `Core\Debug::dump`.
    ///
    /// Deliberately **not** routed through [`Self::captures`]: a
    /// `Core\Out::capture` redirects what a program `echo`s, and a dump is not
    /// that. Capturing one would make `echo Core\Out::capture(fn () =>
    /// Core\Debug::dump($x))` swallow the dump it was meant to make visible.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns. [`OutputSink::Buffer`] and
    /// [`OutputSink::Sink`] never fail.
    pub fn write_diagnostic(&mut self, bytes: &[u8]) -> io::Result<()> {
        write_to(&mut self.diagnostic, bytes)
    }

    /// Points this context's diagnostic channel somewhere else — what a test
    /// that wants to read a dump back calls, and the one way to move it off
    /// [`OutputSink::Stderr`].
    pub fn set_diagnostic_sink(&mut self, sink: OutputSink) {
        self.diagnostic = sink;
    }

    /// Takes everything written to the diagnostic channel so far, if it
    /// buffers.
    #[must_use]
    pub fn take_buffered_diagnostic(&mut self) -> Option<Vec<u8>> {
        match &mut self.diagnostic {
            OutputSink::Buffer(buffer) => Some(std::mem::take(buffer)),
            OutputSink::Stdout | OutputSink::Stderr | OutputSink::Sink => None,
        }
    }

    /// Flushes this request's output.
    ///
    /// `nvs run` calls this once the script's frame returns: Rust's standard
    /// output is line-buffered, and a script whose last `echo` has no trailing
    /// newline would otherwise depend on the process-exit flush.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns.
    pub fn flush_output(&mut self) -> io::Result<()> {
        match &mut self.output {
            OutputSink::Stdout => io::stdout().flush(),
            OutputSink::Stderr => io::stderr().flush(),
            OutputSink::Buffer(_) | OutputSink::Sink => Ok(()),
        }
    }

    /// The `Core` class this request's sink hands captured bytes back as —
    /// [ADR 0088](../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
    /// § 3's table, read as a class name.
    ///
    /// [`CARRIER_CLI_TEXT`] for every sink that exists today, because every
    /// one of them is a terminal or a stand-in for one: `nvs run`'s stdout, a
    /// test's buffer, a discarded run. [`CARRIER_HTML_MARKUP`] arrives with
    /// M8's HTTP request, which is the only context that attaches the HTML
    /// sink, and it is a new [`OutputSink`] variant plus one arm here rather
    /// than a rule any call site states.
    #[must_use]
    pub fn carrier(&self) -> &'static str {
        match &self.output {
            OutputSink::Stdout | OutputSink::Stderr | OutputSink::Buffer(_) | OutputSink::Sink => {
                CARRIER_CLI_TEXT
            }
        }
    }

    /// Opens a capture level: from here until the matching [`Self::end_capture`],
    /// everything written to this request's output is buffered instead.
    pub fn begin_capture(&mut self) {
        self.captures.push(Vec::new());
    }

    /// Closes the innermost capture level and answers what it captured, or
    /// `None` when none was open.
    ///
    /// A caller that opened one **must** close it on every edge, the throwing
    /// one included — `nvs_stdlib::out` is the only such caller, and it does.
    pub fn end_capture(&mut self) -> Option<Vec<u8>> {
        self.captures.pop()
    }

    /// How many captures are open — a test's window onto the invariant that
    /// [`Self::begin_capture`] and [`Self::end_capture`] pair on every edge.
    #[must_use]
    pub fn capture_depth(&self) -> usize {
        self.captures.len()
    }

    /// Takes everything written so far, if this context buffers its output.
    #[must_use]
    pub fn take_buffered_output(&mut self) -> Option<Vec<u8>> {
        match &mut self.output {
            OutputSink::Buffer(buffer) => Some(std::mem::take(buffer)),
            OutputSink::Stdout | OutputSink::Stderr | OutputSink::Sink => None,
        }
    }
}

/// Writes `bytes` to one sink — the body [`Ctx::write_output`] and
/// [`Ctx::write_diagnostic`] share, so a sink variant added later cannot be
/// handled at one channel and forgotten at the other.
fn write_to(sink: &mut OutputSink, bytes: &[u8]) -> io::Result<()> {
    match sink {
        OutputSink::Stdout => io::stdout().write_all(bytes),
        OutputSink::Stderr => io::stderr().write_all(bytes),
        OutputSink::Buffer(buffer) => {
            buffer.extend_from_slice(bytes);
            Ok(())
        }
        OutputSink::Sink => Ok(()),
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Self::stdout()
    }
}

thread_local! {
    /// The context whose compiled frames are running on this thread, or null
    /// between runs — see [`CurrentCtx`].
    ///
    /// A raw pointer and a `Cell` so it is `const`-initialized and carries no
    /// destructor, which is what `crate::alloc`'s own thread-local requires of
    /// every one in this crate and costs nothing here.
    static CURRENT: std::cell::Cell<*mut Ctx> = const { std::cell::Cell::new(std::ptr::null_mut()) };
}

/// Installs a context as this thread's current one for as long as the guard
/// lives, restoring whatever was there before when it drops.
///
/// # Decision: the release path reaches its context through here
///
/// [`crate::abi::call`] is the one door from Rust into compiled code, so this
/// is set there and nowhere else. Its one reader is
/// [`crate::object::dismantle`], which has to call a dying generator's unwind
/// entry point (`nvs_ir::lower::generator`'s transform) and reaches it from a
/// `nvs_object_release` whose `extern "C"` signature is one pointer wide:
/// threading a context through every release primitive would put a parameter
/// on the hot path of every decrement in the language to serve the one release
/// in ten thousand that frees a suspended generator, which AGENTS.md's
/// priority 3 rules out.
///
/// **A release performs no other context access**, which is what makes this
/// sound: the `&mut Ctx` frames above a release are dormant for the length of
/// it, so the reborrow [`with_current`] hands out is the only live one.
/// **Cost:** two thread-local word stores per Rust-to-compiled call boundary —
/// not per compiled call, which passes the context in a register.
pub(crate) struct CurrentCtx(*mut Ctx);

impl CurrentCtx {
    /// Makes `ctx` this thread's current context until the guard drops.
    pub(crate) fn install(ctx: &mut Ctx) -> Self {
        Self(CURRENT.replace(&raw mut *ctx))
    }
}

impl Drop for CurrentCtx {
    fn drop(&mut self) {
        CURRENT.set(self.0);
    }
}

/// Runs `body` against this thread's current context, or answers `None` when
/// no compiled frame is running — see [`CurrentCtx`].
pub(crate) fn with_current<R>(body: impl FnOnce(&mut Ctx) -> R) -> Option<R> {
    let ptr = CURRENT.get();
    if ptr.is_null() {
        return None;
    }
    #[expect(
        unsafe_code,
        reason = "the pointer was installed by `CurrentCtx::install` from a \
                  live `&mut Ctx` whose guard is still on this thread's stack, \
                  and every frame holding one is dormant for the length of a \
                  release"
    )]
    Some(body(unsafe { &mut *ptr }))
}

/// The safepoint poll's slow path — reached only when the word compiled code
/// loaded was non-zero.
///
/// Returns [`crate::FATAL`] for a request that must stop, and [`crate::OK`]
/// otherwise. A resource-limit stop is deliberately not a `THROWN`:
/// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) makes it not
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
                  unwind out of the JIT frame above"
    )]
    let ctx = unsafe { &mut *ctx };

    if ctx.safepoint.contains(SafepointFlags::CPU_LIMIT) {
        ctx.set_pending("the request exceeded its CPU-time limit");
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
/// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's two
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

/// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
/// § 1's statement-boundary probe — the slow path behind the debug-flags
/// check, reached only when the word compiled code loaded was non-zero.
///
/// Deliberately the same *shape* as [`nvs_safepoint`]: one cached load and one
/// predicted-not-taken branch at the site, everything else out of line. It
/// differs in returning nothing — coverage bookkeeping cannot fail, and ADR
/// 0018 puts a debugger break at a safepoint, not at a probe — so a compiled
/// probe site has no status to check and no error edge to emit.
///
/// Only [`DebugFlags::COVERAGE`] acts here. `BRANCH` needs the per-edge probe
/// site that lands with `nvs_ir::Terminator::Branch`'s lowering; `TRACE` and
/// `PROFILE` are the call-site pair below.
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
pub unsafe extern "C" fn nvs_probe_stmt(ctx: *mut Ctx, stmt: u32) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ctx` is valid for this call; the only \
                  thing that can panic here is the allocator, which aborts \
                  rather than unwinding into the JIT frame above"
    )]
    let ctx = unsafe { &mut *ctx };

    if ctx.debug.contains(DebugFlags::COVERAGE) {
        ctx.record_stmt_hit(stmt);
    }
}

/// Reads a callee label a compiled call site passed as a pointer/length pair
/// into the unit's own data section.
///
/// Lossy rather than fallible: the bytes come from an Novis identifier the
/// compiler wrote there, so they are already valid UTF-8, and a trace record
/// is not a place to fail a request from if that assumption were ever wrong.
///
/// # Safety
///
/// `name`/`len` must describe a live, readable byte range, or `len` must be
/// zero.
#[expect(
    unsafe_code,
    reason = "compiled code passes a pointer and a length; the contract cannot \
              be expressed in the signature"
)]
unsafe fn callee_label<'a>(name: *const u8, len: usize) -> Cow<'a, str> {
    if len == 0 {
        return Cow::Borrowed("");
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the range is readable; the zero-length \
                  case is split out because `from_raw_parts` rejects a null \
                  pointer even for an empty slice"
    )]
    let bytes = unsafe { std::slice::from_raw_parts(name, len) };
    String::from_utf8_lossy(bytes)
}

/// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
/// § 1's call-site **entry** probe — the slow path behind the debug-flags
/// check compiled code emits before every call.
///
/// The callee is passed as a pointer and length into the compiled unit's own
/// data section rather than as an index into a side table: the name is
/// already a static constant of the unit, so there is nothing for a table to
/// add and nothing to keep in sync.
///
/// Like [`nvs_probe_stmt`], it returns nothing — a trace record cannot fail —
/// so the site has no status to check.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the call, and `name`/`len`
/// must describe a readable byte range.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer and a static byte \
              range; neither contract can be expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_probe_call_enter(ctx: *mut Ctx, name: *const u8, len: usize) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both are valid for this call; the only \
                  thing that can panic here is the allocator, which aborts \
                  rather than unwinding into the JIT frame above"
    )]
    let (ctx, label) = unsafe { (&mut *ctx, callee_label(name, len)) };
    if ctx.debug.contains(DebugFlags::TRACE) {
        ctx.record_trace(&label, None);
    }
}

/// [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
/// § 1's call-site **exit** probe, carrying the checked-return `status` the
/// call site is about to branch on — which is why a trace shows a thrown or
/// `FATAL` exit as it happened rather than as a reconstruction.
///
/// See [`nvs_probe_call_enter`] for the rest, including why the flags word is
/// re-read here rather than the entry probe's answer being reused: a request
/// may turn tracing on or off *during* the call, and an exit whose flag state
/// differs from its entry's is the honest record of that.
///
/// # Safety
///
/// The same contract as [`nvs_probe_call_enter`].
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer and a static byte \
              range; neither contract can be expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_probe_call_exit(
    ctx: *mut Ctx,
    name: *const u8,
    len: usize,
    status: i32,
) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both are valid for this call; the only \
                  thing that can panic here is the allocator, which aborts \
                  rather than unwinding into the JIT frame above"
    )]
    let (ctx, label) = unsafe { (&mut *ctx, callee_label(name, len)) };
    if ctx.debug.contains(DebugFlags::TRACE) {
        ctx.record_trace(&label, Some(status));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0088 § 5's "always swallows": while a capture is open the sink below
    /// it sees nothing at all, and it sees everything again once it closes.
    #[test]
    fn a_capture_takes_the_output_and_the_sink_below_sees_none_of_it() {
        let mut ctx = Ctx::buffered();
        ctx.write_output(b"before").unwrap();
        ctx.begin_capture();
        ctx.write_output(b"inside").unwrap();
        assert_eq!(ctx.end_capture().as_deref(), Some(&b"inside"[..]));
        ctx.write_output(b"after").unwrap();
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"beforeafter"[..])
        );
    }

    /// A capture is scoped to a closure, so captures nest by call nesting: the
    /// innermost one takes the bytes, and what it re-emits afterwards lands in
    /// the one outside it.
    #[test]
    fn captures_nest_innermost_first() {
        let mut ctx = Ctx::buffered();
        ctx.begin_capture();
        ctx.write_output(b"outer<").unwrap();
        ctx.begin_capture();
        ctx.write_output(b"inner").unwrap();
        let inner = ctx.end_capture().expect("the inner capture was open");
        assert_eq!(inner, b"inner");
        assert_eq!(ctx.capture_depth(), 1);
        ctx.write_output(&inner).unwrap();
        ctx.write_output(b">").unwrap();
        assert_eq!(ctx.end_capture().as_deref(), Some(&b"outer<inner>"[..]));
        assert_eq!(ctx.capture_depth(), 0);
        assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
    }

    /// Closing a capture nobody opened answers `None` rather than corrupting
    /// the stack — the shape a helper's error edge relies on.
    #[test]
    fn ending_a_capture_that_was_never_begun_answers_nothing() {
        let mut ctx = Ctx::buffered();
        assert!(ctx.end_capture().is_none());
        assert_eq!(ctx.capture_depth(), 0);
    }

    /// Every sink that exists today is a terminal or a stand-in for one, so
    /// each names the same carrier — ADR 0088 § 3's default row.
    #[test]
    fn every_sink_today_carries_cli_text() {
        assert_eq!(Ctx::stdout().carrier(), CARRIER_CLI_TEXT);
        assert_eq!(Ctx::buffered().carrier(), CARRIER_CLI_TEXT);
        assert_eq!(Ctx::new(OutputSink::Sink).carrier(), CARRIER_CLI_TEXT);
        assert!(is_carrier(CARRIER_CLI_TEXT) && is_carrier(CARRIER_HTML_MARKUP));
        assert!(!is_carrier(r"Core\Str"));
    }

    #[test]
    fn the_hot_words_come_first_and_are_a_word_apart() {
        assert_eq!(SAFEPOINT_OFFSET, 0);
        assert_eq!(DEBUG_FLAGS_OFFSET, 8);
        assert_eq!(DEADLINE_OFFSET, 16);
        assert_eq!(STACK_LIMIT_OFFSET, 24);
        assert_eq!(STATICS_OFFSET, 40);
        // The offsets above are the arrangement; this is the property ADR 0106
        // § 5 actually buys with it, and it is what fails first when a field
        // is added rather than appended.
        for (name, offset) in [
            ("safepoint", SAFEPOINT_OFFSET),
            ("debug", DEBUG_FLAGS_OFFSET),
            ("deadline", DEADLINE_OFFSET),
            ("stack_limit", STACK_LIMIT_OFFSET),
            ("statics", STATICS_OFFSET),
        ] {
            assert!(
                offset + size_of::<u64>() <= HOT_LINE_BYTES,
                "`{name}` at {offset} has left the hot line"
            );
        }
    }

    #[test]
    fn installing_statics_materializes_one_slot_per_declared_default() {
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        assert_eq!(ctx.statics_len(), 0);
        ctx.install_statics(&[
            Some(FieldDefault::Int(3)),
            Some(FieldDefault::Str("hi".to_owned())),
            None,
        ]);
        assert_eq!(ctx.statics_len(), 3);
        // Re-arming is what a second request on a reused context does: the
        // previous slots are released, never leaked, and the initializers run
        // again rather than the writes of the request before carrying over.
        ctx.install_statics(&[
            Some(FieldDefault::Int(3)),
            Some(FieldDefault::Str("hi".to_owned())),
            None,
        ]);
        assert_eq!(ctx.statics_len(), 3);
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
    fn a_fresh_context_has_nothing_set() {
        let ctx = Ctx::buffered();
        assert!(ctx.safepoint_flags().is_empty());
        assert!(ctx.debug_flags().is_empty());
        assert!(!ctx.deadline_expired());
        assert!(ctx.pending().is_none());
    }

    #[test]
    fn buffered_output_accumulates_and_is_taken_once() {
        let mut ctx = Ctx::buffered();
        ctx.write_output(b"Hello, ").unwrap();
        ctx.write_output(b"World!").unwrap();
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"Hello, World!"[..])
        );
        assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
    }

    #[test]
    fn a_discarding_sink_reports_nothing_buffered() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.write_output(b"gone").unwrap();
        assert!(ctx.take_buffered_output().is_none());
    }

    #[test]
    fn a_pending_message_is_taken_once() {
        let mut ctx = Ctx::buffered();
        ctx.set_pending("boom");
        assert_eq!(ctx.pending().as_deref(), Some("boom"));
        assert_eq!(ctx.take_pending().as_deref(), Some("boom"));
        assert!(ctx.pending().is_none());
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
    fn a_probe_with_coverage_off_records_nothing() {
        let mut ctx = Ctx::buffered();
        for stmt in 0..4 {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            unsafe {
                nvs_probe_stmt(&raw mut ctx, stmt);
            }
        }
        assert!(ctx.stmt_hits().is_empty());
    }

    #[test]
    fn a_probe_counts_a_hit_per_statement_once_coverage_is_on() {
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        for stmt in [2_u32, 0, 2] {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            unsafe {
                nvs_probe_stmt(&raw mut ctx, stmt);
            }
        }
        // Statement 1 never ran; 2 ran twice. The table is dense, so an
        // unexecuted statement between two executed ones reads back as zero
        // rather than as absent.
        assert_eq!(ctx.stmt_hits(), [1, 0, 2]);
    }

    #[test]
    fn nothing_is_armed_by_default_and_the_site_fires_exactly_once() {
        let mut ctx = Ctx::buffered();
        for _ in 0..4 {
            assert!(!ctx.take_armed_helper_panic());
        }

        ctx.inject_fault(FaultSite::HelperPanic);
        // The first call is let through; the second is the site. Nothing
        // after it fires again — one injected fault, not a poisoned request.
        assert!(!ctx.take_armed_helper_panic());
        assert!(ctx.take_armed_helper_panic());
        assert!(!ctx.take_armed_helper_panic());
        assert!(!ctx.take_armed_helper_panic());
    }

    #[test]
    fn a_call_probe_with_tracing_off_records_nothing() {
        let mut ctx = Ctx::buffered();
        let name = b"Math::double";
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
            nvs_probe_call_exit(&raw mut ctx, name.as_ptr(), name.len(), crate::OK);
        }
        // Coverage on, tracing still off: the two flags are independent, and
        // the call probe reads its own bit rather than "any bit set".
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
        }
        assert!(ctx.trace().is_empty());
    }

    #[test]
    fn a_call_probe_records_the_callee_and_the_status_it_is_handed() {
        // The exit probe carries the checked-return status the call site is
        // about to branch on, so a thrown or `FATAL` exit is recorded as it
        // happened rather than reconstructed — ADR 0018 § 1.
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::TRACE);
        let name = b"Boom::inner";
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, name.as_ptr(), name.len());
            nvs_probe_call_exit(&raw mut ctx, name.as_ptr(), name.len(), crate::THROWN);
        }
        assert_eq!(
            ctx.trace(),
            [
                TraceEvent {
                    callee: "Boom::inner".to_owned(),
                    status: None,
                },
                TraceEvent {
                    callee: "Boom::inner".to_owned(),
                    status: Some(crate::THROWN),
                },
            ]
        );
    }

    #[test]
    fn a_call_probe_accepts_an_empty_label_without_reading_the_pointer() {
        let mut ctx = Ctx::buffered();
        ctx.set_debug_flags(DebugFlags::TRACE);
        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            nvs_probe_call_enter(&raw mut ctx, std::ptr::null(), 0);
        }
        assert_eq!(ctx.trace().len(), 1);
        assert_eq!(ctx.trace()[0].callee, "");
    }

    #[test]
    fn coverage_can_be_turned_on_and_off_mid_request() {
        // ADR 0018's whole argument for a runtime-checked flag over a second
        // compiled tier: a harness brackets one test inside a running request.
        let mut ctx = Ctx::buffered();
        let probe = |ctx: &mut Ctx, stmt: u32| {
            #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
            unsafe {
                nvs_probe_stmt(&raw mut *ctx, stmt);
            }
        };

        probe(&mut ctx, 0);
        ctx.set_debug_flags(DebugFlags::COVERAGE);
        probe(&mut ctx, 1);
        ctx.set_debug_flags(DebugFlags::empty());
        probe(&mut ctx, 2);

        assert_eq!(ctx.stmt_hits(), [0, 1]);
    }

    #[test]
    fn expect_failure_discharges_only_the_failures_inside_its_own_body() {
        // ADR 0079 § 5: the discharge is what `Core\Test::expectFailure` does,
        // and it is deliberately narrow in both directions — a failure recorded
        // *before* the mark is another test's problem and stays, and a passing
        // assertion inside the body really ran, so § 20 still counts it.
        let mut ctx = Ctx::buffered();
        ctx.record_assertion("assertSame", Some("an earlier failure".to_owned()));
        let mark = ctx.assertion_count();
        ctx.record_assertion("assertSame", None);
        ctx.record_assertion("assertEquals", Some("the expected one".to_owned()));
        ctx.record_assertion("assertEqualsDeep", Some("and a second".to_owned()));

        assert_eq!(ctx.discharge_failures_from(mark), 2);
        let left = ctx.take_assertions();
        assert_eq!(left.len(), 2);
        assert_eq!(left[0].failure.as_deref(), Some("an earlier failure"));
        assert_eq!(left[1].member, "assertSame");
        assert!(left[1].failure.is_none());
        // Taken rather than borrowed: the next test starts empty.
        assert_eq!(ctx.assertion_count(), 0);
        assert_eq!(ctx.discharge_failures_from(mark), 0);
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
