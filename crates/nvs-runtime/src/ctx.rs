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
//! **One word per request *tree*, not per context.** The field is a shared
//! handle rather than the word itself, so a `spawn script` child polls the same
//! word its root does:
//! [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md) gives a tree
//! one ceiling to divide and charges a child's CPU to the root, and the timer
//! that expires a request only ever holds the root to fire at. [`Ctx::isolate`]
//! owns what the sharing costs.
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

use nvs_config::log::Format as LogFormat;
use nvs_render::{Level, Record};

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
    /// A file on disk, under [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
    /// § 10's rotation and retention bound.
    ///
    /// What `[log] target = "file:…"` selects, built by
    /// [`Ctx::write_log_record`]'s reader and reachable directly through
    /// [`Ctx::set_diagnostic_sink`], which is how the floor's own bound is
    /// asserted without a configuration in front of it.
    File(crate::logfile::LogFile),
    /// Discarded.
    Sink,
}

/// Where a record goes when `[log] target` names no destination — which is a
/// different channel for each of [ADR 0092](../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
/// § 6's two writers, and the same one for both as soon as it does name one.
///
/// [`Ctx::write_log_record`] is the whole of the routing and its doc comment is
/// the home of why the unconfigured default is a split rather than a single
/// channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogChannel {
    /// The program's own output, through [`Ctx::write_output`] and so through
    /// ADR 0088 § 5's capture stack — `Core\Log::write`'s, because a record a
    /// program chose to write is something it said.
    Output,
    /// The diagnostic channel, through [`Ctx::write_diagnostic`] — the engine
    /// floor's, because a record about a program that has already stopped is
    /// not that program's output.
    Diagnostic,
}

/// What `[log] target` resolved to, read once per context.
#[derive(Debug)]
enum LogTarget {
    /// The directive has not been read yet. Every context starts here and
    /// returns here at [`Ctx::set_config`], so the read happens after the
    /// configuration is in place and never twice.
    Unread,
    /// Read, and the configuration names no destination this build can open.
    /// Each writer keeps [`LogChannel`]'s own channel.
    Unnamed,
    /// Read: both writers land here.
    Named(OutputSink),
}

/// A database connection a request is holding open — `nvs_db`'s `Connection`,
/// and the seam that lets a [`Ctx`] hold one without naming it.
///
/// Declared here for [`Running`](crate::host::Running)'s reason and one more.
/// A `Core\Db\Connection` is an object with no native drop, so the connection
/// itself is a key into a table the request owns
/// ([`Ctx::hold_open_connection`]); the table has to live in this crate,
/// because this is the crate that learns when a request ends. And the edge
/// cannot run the other way: [ADR 0132](../../../docs/adr/0132-database-driver-shape.md)
/// § 1 has `nvs-db` depending on this crate, so a field typed
/// `nvs_db::Connection` would close a cycle.
///
/// The one method is the downcast a holder needs to get its own type back,
/// which `dyn Trait` cannot do on its own. Nothing in this crate calls it —
/// what this crate wants from a connection is that it is dropped with the
/// request, which is [`Drop`]'s job and needs no method at all.
pub trait HeldConnection: std::fmt::Debug + std::any::Any {
    /// This connection as the concrete type its driver crate knows it by.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
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
    ///
    /// **Shared with every context in the request tree, which is why it is a
    /// handle and not the word.**
    /// [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md) gives a
    /// tree "one ceiling to divide" and charges a child's CPU to the root. A
    /// copied flag satisfied that only in one direction — a child built *after*
    /// the timer fired was born expired, while one built a microsecond before
    /// it ran on with a zero of its own, and nothing would ever have set that
    /// zero, because the timer holds the root and no registry of live children
    /// exists for it to walk. So [`Self::child`] and [`Self::isolate`] clone the
    /// handle, and the tree stops on the one store.
    ///
    /// **What it spends:** one allocation per request *tree* — the children
    /// share the root's — and one pointer hop on a poll already amortised over
    /// [`crate::bounded_loop`]'s batch. The handle is in the hot line the stack
    /// check loads; the word it names is not, so the hop is a second cache line
    /// and the batch is what makes it affordable.
    deadline: std::sync::Arc<std::sync::atomic::AtomicU64>,
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
    /// The thread's live-byte balance when this context was made — the zero
    /// point [`Self::memory_used`] measures this request's own allocation
    /// from. [`crate::budget`] owns why the counter is kept per *thread* and
    /// read per *request*, and what maintaining it costs.
    ///
    /// **Cold, and here rather than beside the stack limit it reads like.**
    /// Compiled code never loads it: the first five words are an arrangement
    /// this crate's tests pin by offset, so a field added among them moves
    /// `statics` and fails them. Nothing below `statics` has that constraint.
    memory_base: isize,
    /// The thread's output-byte count when this context was made — the zero
    /// point [`Self::output_used`] measures this request's own writing from.
    /// [`Self::memory_base`]'s twin in every respect, the reason it sits below
    /// `statics` included.
    output_base: usize,
    /// `[limits] memory` as a byte count, or `0` for a request under no cap —
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's
    /// first resource limit.
    ///
    /// **Cached, not re-derived.** The value on disk is a string with a suffix
    /// (`"256M"`), and parsing one per poll would put a string parse on the
    /// path a runaway loop is stopped from. So it is resolved whenever the
    /// request's configuration changes — [`Self::set_config`] at start, and
    /// [`Self::refresh_limits`] after `Core\Config::set` or `::restore` moves
    /// the overlay — and read as a bare integer everywhere else.
    memory_limit: usize,
    /// `[limits] max_output` as a byte count, or `0` for a request under no cap
    /// — the response-size ceiling beside the memory one, cached for
    /// [`Self::memory_limit`]'s reason.
    ///
    /// **No reserved slice, unlike its two siblings.**
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1 carves
    /// one out of `memory` and one out of `cpu_time` because a tier-1 handler
    /// cannot run without allocating and cannot run without taking time. It can
    /// run without writing, and nothing refuses a write in the first place —
    /// this ceiling is *noticed* at the safepoint poll and never enforced at
    /// [`Self::write_output`]. So there is no `fatal_reserve_output` for
    /// [`Self::refresh_limits`] to carve, and this one field is the whole of
    /// what the directive becomes.
    ///
    /// **What it spends:** one word per request.
    output_limit: usize,
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's
    /// tier-1 handler: the closure `Core\Fatal::onLimit` registered, owned, or
    /// `null` for a request that registered none.
    ///
    /// **Here, beside the ceiling, because this is where the breach is asked.**
    /// § 1 makes the registration request-local and puts it next to the pending
    /// slot for the reason this field is a field at all: it dies with the
    /// request, exactly as [ADR 0008](../../../docs/adr/0008-static-and-global.md)
    /// and [ADR 0012](../../../docs/adr/0012-no-superglobals.md) require of
    /// everything a request holds, so there is no process-wide table for a
    /// second request to inherit one from.
    ///
    /// Cold: nothing loads it inline, and only the slow path that has already
    /// decided a limit was breached reads it.
    ///
    /// **What it spends:** two words per request, and one reference to the
    /// closure for a request that registers one — O(in-flight requests), per
    /// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md).
    limit_handler: Value,
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's
    /// reserved slice, in bytes: what [`Ctx::memory_limit`] was *reduced by* so
    /// that the tier-1 handler has somewhere to run once ordinary execution has
    /// spent everything it may.
    ///
    /// Carved at request start rather than found later, because the point of a
    /// reserve is that nothing else could have taken it. Cold in the same way
    /// the handler slot above is: only the slow path that has already decided a
    /// limit was breached reads it.
    ///
    /// **What it spends:** nothing beyond a word per request — the bytes
    /// themselves are the operator's own `[limits] memory`, moved from one side
    /// of the ceiling to the other, so a request's total is unchanged.
    fatal_reserve: usize,
    /// `[limits] cpu_time` in nanoseconds, or `0` for a request under no cap —
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's
    /// second resource limit, cached for [`Self::memory_limit`]'s reason.
    ///
    /// **What measures it is the request thread's own CPU clock, and never the
    /// wall clock [`Self::deadline`] is written from.** The two are different
    /// directives — `[limits] wall_time` is the other one — and they answer
    /// different questions: wall time bounds how long a client waits, and CPU
    /// time bounds how much of this machine one request may burn. Charging a
    /// request for the time it spent descheduled or blocked on a socket would
    /// make a runaway loop and a slow database indistinguishable, and § 1's
    /// limits exist to stop the first without touching the second.
    ///
    /// **The sampling is the host's, not this crate's.** A thread-per-core host
    /// runs many requests in one process, so the clock has to be per-thread
    /// (`CLOCK_THREAD_CPUTIME_ID` on Unix, `GetThreadTimes` on Windows) rather
    /// than per-process, and reading another thread's is a platform call
    /// `nvs-runtime` has no dependency to make. So the split is the one
    /// [`Self::deadline`] already uses: this crate holds the ceiling and the
    /// flag, and whatever timer watches the request raises
    /// [`SafepointFlags::CPU_LIMIT`] when the clock passes it. Until that timer
    /// exists the flag is raised by tests alone, which is the gap
    /// [`nvs_safepoint`]'s CPU branch describes.
    ///
    /// **What it spends:** one word per request.
    cpu_limit: u64,
    /// The nanoseconds carved out of [`Self::cpu_limit`] and left for the tier-1
    /// handler — [`Self::fatal_reserve`]'s other half, and the same slice.
    ///
    /// Held rather than re-derived for the reason its sibling is: the number is
    /// read once by [`Self::refresh_limits`] and then only by the slow path that
    /// has already decided a limit was breached.
    ///
    /// **What it spends:** one word per request. The time itself is the
    /// operator's own `[limits] cpu_time`, moved from one side of the ceiling to
    /// the other, so a request's total is unchanged.
    fatal_reserve_time: u64,
    /// Takes ownership of the closure `Core\Fatal::onUncaughtThrow` registered
    /// — [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 2's
    /// tier 2, and the second handler slot beside [`Self::limit_handler`].
    ///
    /// **Nothing is reserved for it, and that is § 2's own decision**: a throw
    /// that reached the root means execution was healthy up to the moment it
    /// was raised, so the request's ordinary remaining budget is what this
    /// handler runs under. The two words [`Self::fatal_reserve`] and
    /// [`Self::fatal_reserve_time`] hold for tier 1 have no twin here, and
    /// [`Self::run_uncaught_handler`] widens no ceiling on the way in.
    ///
    /// Request-local for [`Self::limit_handler`]'s reason, and released in the
    /// same place for it: the request ending is the only unregistration.
    ///
    /// **What it spends:** one word per request, and one reference to the
    /// closure for a request that registers one — O(in-flight requests), per
    /// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md).
    uncaught_handler: Value,
    /// [ADR 0127](../../../docs/adr/0127-the-end-of-a-script-is-observable.md)
    /// § 1's end-of-script queue, in registration order — what
    /// `Core\Script::onExit` appends to and [`Self::run_exit_hooks`] drains
    /// once, as the last user code of the script.
    ///
    /// **A queue rather than a slot**, which is the one way it differs from the
    /// two handlers above it: a hook does not replace the hook before it, so
    /// registering twice registers twice and § 1's FIFO order is this vec's
    /// order. A hook registered *by* a hook joins the tail of the same drain,
    /// which is why [`Self::run_exit_hooks`] walks by index rather than
    /// draining the vec it is iterating.
    ///
    /// Request-local for [`Self::limit_handler`]'s reason, and released in the
    /// same place for it: a script stopped by a `FATAL` reaches [`Drop`] with
    /// its queue unrun, exactly as its deferred work does, because § 3 says a
    /// limit breach runs none of it.
    ///
    /// **What it spends:** one reference per registration, plus whatever each
    /// hook captured, held from the registration to the end of the script —
    /// per request, O(registrations), which is the spend ADR 0127 § 1 states
    /// and [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md) asks
    /// for.
    exit_hooks: Vec<Value>,
    /// Whether [`Self::run_exit_hooks`] has already run — ADR 0127 § 2's "the
    /// queue runs once, at most once per script".
    ///
    /// A flag rather than "the vec is empty": a hook that registers a hook
    /// empties neither, and an ending that ran an empty queue has still had its
    /// one drain.
    exit_hooks_drained: bool,
    /// How deep a chain of `spawn script` may nest — `[limits] max_script_depth`,
    /// [`Self::DEFAULT_MAX_SCRIPT_DEPTH`] where nothing states one, and `0`
    /// (no ceiling) only where an operator wrote `false`.
    ///
    /// **The one ceiling here with a default rather than an unstated "no cap",**
    /// and the asymmetry is the point. A request left uncapped for CPU or memory
    /// is a request an operator chose not to bound; a recursion of isolates left
    /// unbounded does not run on forever, it exhausts the tree's heap, so
    /// "unstated" would mean the breach is reported as an out-of-memory — the
    /// confusion `docs/plan/m6.md`'s *Verify* asks this directive to remove. A
    /// malformed value takes the default for the same reason, where its siblings
    /// read malformed as no cap: on is the safe direction for a net.
    ///
    /// **Counting the depth is the host's, not this crate's**, the same split
    /// [`Self::cpu_limit`] describes for the clock: `nvs-runtime` has no isolate
    /// to number, so it holds the ceiling and `nvs_host::Isolate` compares its
    /// own depth against it.
    ///
    /// **What it spends:** one word per request.
    max_script_depth: u32,
    /// How many `spawn script` boundaries stand between this context and the
    /// request that started the tree — `0` for the request itself, and one more
    /// for each isolate beneath it ([`Self::isolate`] is where it is counted).
    ///
    /// Held here rather than on `nvs_host::Isolate` because the depth has to
    /// survive the crossing, and a [`Ctx`] is the only thing that does: an
    /// isolate is built, run and dropped, while its context is what its own
    /// children are built from. That is also why the ceiling beside it is not
    /// re-read from the configuration by [`Self::isolate`] — see that
    /// constructor.
    ///
    /// **What it spends:** one word per request, and one per in-flight isolate.
    script_depth: u32,
    /// [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
    /// § 6's after-response work, in registration order — `None` once the
    /// queue has been drained, which is the encoding of
    /// [`crate::deferred::DeferError::Sealed`].
    ///
    /// Request-local for [`Self::limit_handler`]'s reason, and the one thing
    /// that makes § 6's "memory, CPU and tasks stay charged to the request
    /// tree" true by construction rather than by remembering to: the
    /// registrations are the request's, so the request ending is what releases
    /// them. [`mod@crate::deferred`] is the one home for when the queue runs
    /// and why the cap beside it counts trees.
    ///
    /// **What it spends:** four words per request, and two words plus one
    /// closure reference per registration — no allocation at all for a request
    /// that defers nothing.
    deferred: Option<Vec<crate::deferred::Deferred>>,
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
    /// Where `[log] target` sends a record — [`Self::write_log_record`]'s
    /// destination, resolved from the directive on first use and held here
    /// because [`OutputSink::File`] is stateful: the rotation bound is counted
    /// against a handle, so re-deriving the sink per record would re-open the
    /// file per record and count nothing.
    ///
    /// **What it spends:** one discriminant per context while the directive is
    /// unset, and one `LogFile` — a path, a descriptor and two counters — for a
    /// context that writes to a configured file. Nothing is O(records).
    log: LogTarget,
    /// The quietest level `[log] level` writes — [`Self::write_log_record`]'s
    /// floor, resolved beside [`Self::log`] on the same first use because one
    /// call reads both directives and a second "have I read it yet" flag would
    /// be a second thing to keep in step with it.
    ///
    /// **What it spends:** one discriminant per context, and nothing per
    /// record beyond the comparison. [`Level::Debug`] is every level, so a
    /// context whose configuration names none writes what it was handed.
    log_minimum: Level,
    /// Which of ADR 0092 § 3's two renderings the target emits — resolved on
    /// the same first use as the two above, for the same reason, and read at
    /// every record [`Self::write_log_record`] does not drop.
    ///
    /// **What it spends:** one discriminant per context, and one `String` per
    /// written record, which is the allocation the caller made when it rendered
    /// for itself.
    log_format: LogFormat,
    /// [ADR 0102](../../../docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
    /// § 6's configured origin: the scheme and authority
    /// `Core\Router::urlAbsolute` puts in front of a link, with no trailing
    /// `/`.
    ///
    /// **Configured, never sniffed.** § 6 refuses `Host` and
    /// `X-Forwarded-Host` outright, which is why this is written *before* the
    /// request runs and nothing during it can move it — `nvs run` reads
    /// `nvs.toml`'s `[[app]] origin` today, and the mount that accepted the
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
    /// [ADR 0078](../../../docs/adr/0078-config-reload-and-control-socket.md)
    /// § 1's configuration, as this request sees it: the snapshot it cloned at
    /// start and the copy-on-write overlay `Core\Config::set` writes over it.
    ///
    /// **The clone happens once, here, and never per read.** `nvs run` builds
    /// the snapshot before the program starts and hands it over with
    /// [`Self::set_config`]; every `Core\Config` member then reads this field.
    /// A reload landing mid-request replaces what `Current::load` would hand
    /// out next and cannot touch the `Arc` this one already holds, which is the
    /// whole of § 1's "a request reads one tree to completion".
    ///
    /// `None` is a context nobody configured — every test context, and any
    /// caller that has not built a snapshot. The members answer as they do for
    /// a directive nothing set, rather than throwing: an unconfigured host is
    /// ADR 0103 § 1 step 3's shipped defaults and not an error.
    ///
    /// **What it spends:** one `Arc` clone per request, plus a `String` pair
    /// per key that request actually set. O(in-flight requests), per
    /// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md) — the tree
    /// itself is shared and is charged to the snapshot, not to the request.
    config: Option<nvs_config::Request>,
    /// This request's place in a distributed trace —
    /// [ADR 0076](../../../docs/adr/0076-observability-export.md) § 2, whose id
    /// is Novis's only request identifier.
    ///
    /// **Not [`Self::trace`]**, which is ADR 0018's per-call-site event list;
    /// [`crate::trace_context`]'s module doc opens on why the two are separate
    /// and what each is for.
    ///
    /// Drawn eagerly in [`Self::new`] rather than on first ask, because § 2 has
    /// an id exist for every request whatever the sampling decision — a lazy one
    /// would be the same id in the end and a second place for two readers to
    /// disagree about whether there is one at all. A request that arrived with a
    /// `traceparent` gets [`Self::set_trace_context`] before it runs, exactly as
    /// [`Self::config`] does.
    ///
    /// **What it spends:** 25 bytes per request and one CSPRNG draw.
    trace_context: crate::trace_context::TraceContext,
    /// [ADR 0086](../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 6's
    /// command table, or `None` for a program that declared no `#[Command]` —
    /// [`crate::commands`] owns why the rows cross into the runtime at all and
    /// why both absences answer alike.
    ///
    /// **Isolate configuration, written before the program runs**, exactly as
    /// [`Self::origin`] and [`Self::config`] are: the table is a compile
    /// product, so re-reading it mid-request could only ever produce the same
    /// answer or a wrong one.
    ///
    /// **What it spends:** one `Arc` clone per request; the rows themselves are
    /// shared and charged to whoever compiled them.
    commands: Option<std::sync::Arc<crate::commands::CommandTable>>,
    /// The process argument vector past the program itself — what
    /// [ADR 0086](../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 6's
    /// `Core\Command::run` matches against the table above, and what § 13's
    /// `Core\Cli::arguments` will hand back unchanged.
    ///
    /// **Isolate configuration, written before the program runs**, for
    /// [`Self::commands`]'s reason and one more: a command line is a fact about
    /// how this process was started, so a request served over HTTP has none and
    /// gets the empty vector rather than the launcher's own words.
    ///
    /// **What it spends:** one `String` per word a `nvs run` was given, and one
    /// empty `Vec` — no allocation — for every context nobody wrote one onto,
    /// which is every served request. O(in-flight requests), per
    /// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md).
    arguments: Vec<String>,
    /// The name the shell knows this program by — what
    /// [ADR 0086](../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 6's
    /// `Core\Command::completions` registers its script against. See
    /// [`Self::program_name`] for which name that is, which is the whole of
    /// what the member can be wrong about.
    ///
    /// **Isolate configuration, written before the program runs**, for
    /// [`Self::arguments`]'s two reasons — and it is separate from that vector
    /// rather than a zeroth word of it, because the two do not always come from
    /// the same place: a bundled program's name is its executable's while its
    /// words are everything past that executable.
    ///
    /// **What it spends:** one short `String` per `nvs run`, and one empty
    /// `String` — no allocation — for every context nobody wrote one onto.
    /// O(in-flight requests), per
    /// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md).
    program_name: String,
    /// [ADR 0079](../../../docs/adr/0079-testing-is-a-language-feature.md)
    /// § 12's fixed clock: the wall-clock reading `Core\Time::now` answers
    /// with, in nanoseconds since the Unix epoch, or `None` for a context that
    /// reads the host's clock.
    ///
    /// **Isolate configuration, written before the program runs**, exactly as
    /// [`Self::origin`] is — the test runner reads `#[Test(at: …)]` and writes
    /// it onto the isolate's own context, and a context nobody wrote it onto
    /// is every context outside a test. That is why this does not reopen
    /// [ADR 0008](../../../docs/adr/0008-static-and-global.md)'s "nothing holds
    /// state behind a function's back": the one thing that moves it afterwards
    /// is `Core\Test::advance`, which § 12 declares beside the clock and which
    /// exists nowhere but inside a test.
    ///
    /// Nanoseconds since the epoch rather than a `jiff::Timestamp` because this
    /// crate is the one every compiled unit links and `jiff` is `nvs-stdlib`'s
    /// dependency, not this one's; `nvs_stdlib::time` owns both conversions and
    /// is the only reader.
    ///
    /// **What it spends:** two words per request, and nothing at all on the
    /// `Core\Time::now` path beyond one predictable not-taken branch.
    fixed_clock: Option<i128>,
    /// [ADR 0079](../../../docs/adr/0079-testing-is-a-language-feature.md)
    /// § 12's seeded generator, as its **live state** rather than as the seed
    /// it started from, or `None` for a context whose draws come from the
    /// operating system.
    ///
    /// The state and not the seed because every draw has to move it: two
    /// `Core\Random::int` calls in one test are two different numbers, and the
    /// *sequence* is what a seed reproduces. `nvs_stdlib::random` owns the step
    /// that turns one state into the next and into a drawn value; this crate
    /// holds the word and knows nothing about how it is stirred, for the same
    /// reason [`Self::fixed_clock`] holds a count rather than a timestamp.
    ///
    /// **Unreachable outside a test**, which is the whole of why a seedable
    /// generator does not weaken `Core\Random`: the only writer is the test
    /// runner arming a `#[Test(seed: …)]` isolate, so every context a request
    /// or a `nvs run` ever gets has `None` here and draws from the CSPRNG
    /// `nvs_stdlib::random`'s module docs describe. ADR 0079 § 12 puts
    /// `Core\Random\Seeded` behind a separate *type* in production for exactly
    /// this reason, and this field does not reopen that — it adds no spelling a
    /// program outside a test can write.
    ///
    /// **What it spends:** two words per request, and one predictable
    /// not-taken branch per draw.
    random_state: Option<u64>,
    /// [ADR 0086](../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 4's
    /// scripted answer queue: what the next `Core\Cli` prompts read instead of
    /// a terminal, oldest first, and empty for every context outside a test.
    ///
    /// **Beside [`Self::fixed_clock`] because it is the same idea** — a test
    /// declares the world its subject runs in, and the isolate is what scopes
    /// the declaration ([ADR 0079](../../../docs/adr/0079-testing-is-a-language-feature.md)
    /// § 2 gives each test its own context). A queue held in a `thread_local`
    /// would outlive the test that filled it and answer the next one's prompt,
    /// which is the failure a fixed clock avoids the same way.
    ///
    /// The only writer is `Core\Test::scriptAnswers`, so a request or a
    /// `nvs run` never has one — and a program cannot script its own prompts
    /// into a state nobody typed, which is what keeps § 4's "it never blocks"
    /// a statement about the terminal rather than about this field.
    ///
    /// **What it spends:** three words per request for the empty queue, and the
    /// answers themselves only where a test wrote them.
    scripted_answers: std::collections::VecDeque<String>,
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
    /// The value that crossed into this isolate, owned — see
    /// [`Ctx::set_isolate_argument`].
    ///
    /// Cold, and null for every context that is not an isolate's, which is
    /// every request. It is a field here rather than a slot the program keeps
    /// because it is the isolate's ownership *root*
    /// ([ADR 0116](../../../docs/adr/0116-an-isolates-arena-is-an-ownership-root.md)
    /// § 2): what releases it is dropping this context, and nothing else knows
    /// when that happens.
    isolate_argument: Value,
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
    /// The isolates this request has started and not yet awaited, by the key
    /// its `Core\Script\Handle` carries — see [`Ctx::hold_started_script`].
    started_scripts: Vec<Option<Box<dyn crate::host::Running>>>,
    /// The files this request has opened and not yet closed, by the key its
    /// `Core\IO\File` carries — see [`Ctx::hold_open_file`].
    open_files: Vec<Option<std::fs::File>>,
    /// The database connections this request has opened, each with the
    /// memoization key it was reached by — see [`Ctx::hold_open_connection`].
    open_connections: Vec<(Option<String>, Box<dyn HeldConnection>)>,
    /// Every object this context has allocated and not yet dismantled — ADR
    /// 0116 § 2's live list, whose sweep in [`Drop`] reclaims the cyclic graph
    /// the root drain could not. [`crate::object`]'s own docs are the home of
    /// the mechanism and of what it spends, including why the list is a
    /// separate allocation rather than a word of this struct.
    ///
    /// Last, and never in the hot line: nothing on the request path reads it,
    /// and the one write per object allocation reaches it through
    /// [`current_live_list`] rather than through the context at all.
    live: std::rc::Rc<crate::object::LiveList>,
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
            // decides which slot this promotion seeds — ADR 0071 § 5's
            // `issues`, ADR 0067 § 7's `reason` — see `Thrown::new_as`.
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
        // An isolate's argument is one of its roots and is released with them
        // — `Ctx::set_isolate_argument` owns why it is held here at all.
        self.set_isolate_argument(Value::null());
        // ADR 0020 § 1's handler is request-local, so the request ending is
        // what unregisters it — see `Ctx::set_limit_handler`.
        self.set_limit_handler(Value::null());
        // ADR 0020 § 2's handler is request-local for the same reason and is
        // unregistered the same way — see `Ctx::set_uncaught_handler`.
        self.set_uncaught_handler(Value::null());
        // ADR 0127 § 1's exit hooks are request-local for the same reason. A
        // script that ended at a `FATAL` or a cancellation reaches here with the
        // queue unrun — § 3 — so this is where those registrations are given
        // back, exactly as an unrun deferred registration is below.
        #[expect(
            unsafe_code,
            reason = "the queue holds exactly one reference per registration and \
                      nothing else points at it"
        )]
        for hook in std::mem::take(&mut self.exit_hooks) {
            // SAFETY: `Ctx::push_exit_hook` was handed that reference and this
            // is the only other place it is given back.
            unsafe { hook.release() };
        }
        // ADR 0072 § 6's deferred work is request-local for the same reason,
        // and a request that never returned ordinarily reaches here with its
        // registrations unrun — `crate::deferred`'s module doc owns why they
        // are released rather than run. `take_deferred` also takes this tree
        // back out of the core's count.
        for work in self.take_deferred() {
            // SAFETY: the queue holds exactly one reference per registration
            // and nothing else points at it.
            #[expect(
                unsafe_code,
                reason = "the queue owned the reference it is handing over as it \
                          goes down"
            )]
            unsafe {
                work.closure.release();
            }
        }
        // A failure that ended the request still owns its exception object,
        // and `Thrown`'s own `Drop` is what releases it. Taken here rather
        // than left to the field drop below, because a field is dropped
        // *after* this body: the sweep would otherwise reach an object that is
        // about to be released a second time.
        drop(self.pending.take());
        // Last, and only once every root above is gone: what is still on the
        // live list is then exactly the cyclic garbage the refcounts could not
        // free. ADR 0116 § 2 is the decision and `crate::object::sweep` the
        // mechanism.
        crate::object::sweep(&self.live);
    }
}

/// Byte offset of the safepoint word within [`Ctx`] — see the module docs.
pub const SAFEPOINT_OFFSET: usize = std::mem::offset_of!(Ctx, safepoint);

/// Byte offset of the debug-flags word within [`Ctx`] — see the module docs.
pub const DEBUG_FLAGS_OFFSET: usize = std::mem::offset_of!(Ctx, debug);

/// Byte offset of the soft call-stack limit within [`Ctx`] — see the module
/// docs.
pub const STACK_LIMIT_OFFSET: usize = std::mem::offset_of!(Ctx, stack_limit);

/// Byte offset of the deadline handle within [`Ctx`] — see the module docs'
/// *The request's deadline* section, which owns why the word itself is the
/// tree's and this line holds only the way to it.
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

/// Which of [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
/// § 1's resource limits stopped the request, as the tier-1 handler is told it.
///
/// § 1 spells that handler's parameter `LimitReport`, and this is what the
/// report is built from: [`Ctx::run_limit_handler`] hands the closure an array
/// whose `limit` key is [`Limit::name`]. **An array and not a class**, because
/// the report is built where the breach is — in this crate, which holds no
/// `Core` class descriptor to instantiate one from and would have to reach into
/// `nvs-stdlib` to get one — and because a keyed array is the one shape a later
/// field can be added to without changing the signature of a handler already
/// written. What it spends is two allocations out of § 1's reserved slice, once
/// per request that both registers a handler and is stopped.
///
/// **Four variants, because four limits are enforced.** § 1 lists six; wall
/// time and call-stack depth each gain a variant in the slice that gives them a
/// breach to report, since a variant nothing can produce is a word in this
/// report's vocabulary that no handler could see.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    /// `[limits] memory`, reached at a helper boundary ([`crate::run_helper`])
    /// or at the safepoint poll.
    Memory,
    /// `[limits] cpu_time`, reached at the safepoint poll.
    CpuTime,
    /// `[limits] max_output`, reached at the safepoint poll — the one place it
    /// is reached, since [`Ctx::output_limit`]'s field doc has the write itself
    /// refusing nothing.
    Output,
    /// `[limits] max_script_depth`, reached where a `spawn script` would build
    /// a child past the ceiling — the one limit in this list that is not
    /// reached in flight, since nothing is over it until an isolate is asked
    /// for. [`Ctx::script_depth_breach`] is where the question is asked.
    ScriptDepth,
}

impl Limit {
    /// The `nvs.toml` directive's own spelling, which is what the report
    /// carries.
    ///
    /// The directive's name rather than a sentence: the one thing a handler can
    /// do with the report that reading the `FATAL`'s message cannot is branch on
    /// it, and the name an operator would raise is the name they already know.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::CpuTime => "cpu_time",
            Self::Output => "max_output",
            Self::ScriptDepth => "max_script_depth",
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
            deadline: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            stack_limit: 0,
            stack_floor: 0,
            statics: std::ptr::null_mut(),
            exit_code: 0,
            memory_base: crate::budget::live_bytes(),
            output_base: crate::budget::written_bytes(),
            memory_limit: 0,
            output_limit: 0,
            limit_handler: Value::null(),
            fatal_reserve: 0,
            cpu_limit: 0,
            fatal_reserve_time: 0,
            uncaught_handler: Value::null(),
            exit_hooks: Vec::new(),
            exit_hooks_drained: false,
            max_script_depth: Self::DEFAULT_MAX_SCRIPT_DEPTH,
            script_depth: 0,
            deferred: Some(Vec::new()),
            pending: None,
            runtime_error_class: None,
            output,
            diagnostic: OutputSink::Stderr,
            log: LogTarget::Unread,
            log_minimum: Level::Debug,
            log_format: LogFormat::Json,
            origin: None,
            commands: None,
            arguments: Vec::new(),
            program_name: String::new(),
            config: None,
            trace_context: crate::trace_context::TraceContext::started(),
            fixed_clock: None,
            random_state: None,
            scripted_answers: std::collections::VecDeque::new(),
            captures: Vec::new(),
            stmt_hits: Vec::new(),
            trace: Vec::new(),
            yielder: std::ptr::null(),
            fault: None,
            helper_calls: 0,
            statics_store: Vec::new().into_boxed_slice(),
            isolate_argument: Value::null(),
            assertions: Vec::new(),
            started_scripts: Vec::new(),
            open_files: Vec::new(),
            open_connections: Vec::new(),
            live: std::rc::Rc::new(crate::object::LiveList::default()),
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
    /// ADR 0078 § 1's one clone, taken before the program runs.
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

    /// This request's place in a distributed trace — ADR 0076 § 2, and never
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

    /// Hands this program the table the compiler built for it — ADR 0086 § 6,
    /// written before the program runs exactly as [`Self::set_config`] is.
    pub fn set_commands(&mut self, table: std::sync::Arc<crate::commands::CommandTable>) {
        self.commands = Some(table);
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
    /// [ADR 0086](../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 6's
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
    /// * [ADR 0048](../../../docs/adr/0048-portable-single-file-executables.md)'s
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

    /// How many bytes this request has allocated and not yet freed.
    ///
    /// The difference between the thread's balance now and what it was when
    /// this context was made, floored at zero: a request that frees more than
    /// it allocated — because it released what it inherited — has used none of
    /// its own budget rather than a negative amount of it. [`crate::budget`]
    /// says why the underlying counter is per thread.
    #[must_use]
    pub fn memory_used(&self) -> usize {
        usize::try_from(crate::budget::live_bytes().saturating_sub(self.memory_base)).unwrap_or(0)
    }

    /// The ceiling **ordinary execution** is held to, in bytes, `0` for no cap.
    ///
    /// `[limits] memory` less [`Self::fatal_reserve`], because ADR 0020 § 1's
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

    /// Whether this request has allocated past its ceiling.
    ///
    /// Two loads and a compare, and the second load is the thread-local
    /// [`crate::budget::live_bytes`] reads. An uncapped request answers `false`
    /// on the first compare without reading the counter at all.
    #[must_use]
    pub fn over_memory_limit(&self) -> bool {
        self.memory_limit != 0 && self.memory_used() > self.memory_limit
    }

    /// How many bytes this request has written to its response, in the sense
    /// `[limits] max_output` means.
    ///
    /// This request's share of the thread's count, taken against
    /// [`Self::output_base`] — so a root's reading holds every isolate spawned
    /// beneath it and each isolate's holds only its own, which is
    /// [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md)'s
    /// "child output against the root's `max_output`" and the same arrangement
    /// [`Self::memory_used`] already has.
    #[must_use]
    pub fn output_used(&self) -> usize {
        crate::budget::written_bytes().saturating_sub(self.output_base)
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

    /// Takes ownership of the closure `Core\Fatal::onLimit` registered —
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's
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
    /// — [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 2's
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

    /// Runs [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 2's
    /// tier 2 over `thrown`, if this request registered one.
    ///
    /// **The handler is handed the real exception object**, not a report built
    /// from it, which is the one way this differs from
    /// [`Self::run_limit_handler`]'s array. § 2 says why: this is the request's
    /// own root rather than an isolate boundary
    /// [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md) has to
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
    /// [ADR 0127](../../../docs/adr/0127-the-end-of-a-script-is-observable.md)
    /// § 1's end-of-script queue — what `Core\Script::onExit` does, which is
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

    /// Whether the queue has already had its one drain — ADR 0127 § 2.
    #[must_use]
    pub fn exit_hooks_drained(&self) -> bool {
        self.exit_hooks_drained
    }

    /// Runs the end-of-script queue FIFO, handing each hook `report` — ADR 0127
    /// §§ 2 and 5.
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
    /// is the home of § 5's three failure readings.
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
    /// ADR 0127 § 5.
    ///
    /// Three readings, and only the last stops the queue:
    ///
    /// - **A throw** is written to the same record `Core\Log` writes, through
    ///   [`crate::floor`], and abandoned — § 5's "logged with the request's
    ///   trace id rather than swallowed", which is
    ///   [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
    ///   § 4's rule for a second throw and [`crate::deferred`]'s reading of it
    ///   for after-response work.
    /// - **An `exit`** is § 5's refusal: a hook that could end the script would
    ///   suppress every hook behind it, so the status it named is dropped and
    ///   the `RuntimeError` that section names is reported in its place.
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
    /// [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
    /// § 6, and [`mod@crate::deferred`] owns when that is on a host with no
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
    /// **`false` is the one refusal**: the queue is already draining, and
    /// deferred work may not defer more. The caller turns it into the
    /// `RuntimeError` § 6's last bullet names and keeps the reference.
    pub fn defer(&mut self, closure: Value, deadline_nanos: u64) -> bool {
        let Some(queue) = self.deferred.as_mut() else {
            return false;
        };
        queue.push(crate::deferred::Deferred {
            closure,
            deadline_nanos,
        });
        true
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

    /// Runs [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
    /// § 1's tier-1 handler, if this request registered one — the last thing a
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
    /// to a program that does not read it beyond the two allocations building it.
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
        let stopped_for_cpu = self.safepoint.contains(SafepointFlags::CPU_LIMIT);
        if cpu_ordinary != 0 {
            self.cpu_limit = cpu_ordinary.saturating_add(cpu_reserve);
            // Spent, not merely lent, for [`Self::fatal_reserve`]'s reason.
            self.fatal_reserve_time = 0;
        }
        if stopped_for_cpu {
            self.safepoint.remove(SafepointFlags::CPU_LIMIT);
        }
        // Built here rather than by either caller, and *after* the reserve is
        // in force: it allocates, and a report the ladder could not afford to
        // build would be a tier that says nothing for the same reason a handler
        // that cannot allocate is.
        let mut report = crate::NvsArray::new();
        report.set(
            crate::NvsStr::new(b"limit"),
            Value::str(crate::NvsStr::new(limit.name().as_bytes())),
        );
        let report = Value::array(report);
        let answer = crate::call_closure(self, handler, &[report]);
        self.memory_limit = ordinary;
        self.fatal_reserve = reserve;
        self.cpu_limit = cpu_ordinary;
        self.fatal_reserve_time = cpu_reserve;
        if stopped_for_cpu {
            self.safepoint.insert(SafepointFlags::CPU_LIMIT);
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

    /// The [`crate::Fault`] a request past its memory ceiling owes, or `None`
    /// while it is inside it.
    ///
    /// [`crate::Fault::fatal`] and never a throw:
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1 makes
    /// every resource-limit breach a `FATAL`, so no `catch` sees this and a
    /// fixture that wraps the loop in one has found the rule rather than a bug.
    /// The message names the ceiling as well as the reading, because the two
    /// together are what tells an operator whether to raise the limit or to fix
    /// the program.
    #[must_use]
    pub fn memory_breach(&self) -> Option<crate::Fault> {
        if !self.over_memory_limit() {
            return None;
        }
        // The ceiling named is the request's **whole** budget, not the reduced
        // one it was measured against: `[limits] memory` is the number the
        // operator wrote and the only one they can recognise. Where a slice of
        // it is ADR 0020 § 1's reserve, saying so is what keeps the sentence
        // from reading as a reading below its own ceiling.
        let reserved = match self.fatal_reserve {
            0 => String::new(),
            bytes => format!(", of which {bytes} is reserved for the limit handler"),
        };
        Some(crate::Fault::fatal(format!(
            "the request exceeded its memory limit — {} bytes held against a ceiling of {}{reserved}",
            self.memory_used(),
            self.memory_limit.saturating_add(self.fatal_reserve),
        )))
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
    /// A ceiling of `0` is [ADR 0005]'s no-ceiling-at-all and answers `None`
    /// however deep the chain already is — see [`Self::max_script_depth`]'s
    /// field doc, which owns why only an explicit `false` reads that way here.
    ///
    /// [ADR 0005]: ../../../docs/adr/0005-config-changeability.md
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
        // ADR 0020 § 1: the slice is *carved out of* the request's own budget
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
        // Read in the same pass and for the same reason, though there is nothing
        // to carve out of it: a request's ceilings are one reading of one
        // configuration.
        self.max_script_depth = self.configured_max_script_depth();
        // The response ceiling, in the same pass and for the same reason.
        // Nothing is carved out of it — [`Self::output_limit`]'s field doc owns
        // why a ceiling on writing needs no slice reserved from it.
        self.output_limit = self.configured_output_limit();
    }

    /// `[limits] cpu_time` in nanoseconds, or `0` for a request under no cap.
    ///
    /// See [`Self::cpu_limit`]'s field doc for what the number measures. A
    /// malformed value answers "no cap" for the reason
    /// [`Self::configured_memory_limit`] does, and `false` — [ADR 0005]'s
    /// spelling of no ceiling at all — answers the same `0`, because a request
    /// that may burn any amount of CPU and one whose ceiling nothing states are
    /// the same request to everything downstream.
    ///
    /// [ADR 0005]: ../../../docs/adr/0005-config-changeability.md
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
    /// no cap — [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
    /// § 1.
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
    /// [`Self::fatal_reserve`] is the sibling that has a *spender*: the memory
    /// half is added back for the length of the call in
    /// [`Self::run_limit_handler`], because a handler that cannot allocate is a
    /// tier that says nothing. Nothing hands this half back yet, because nothing
    /// samples a clock against `cpu_limit` in the first place — the gap
    /// [`nvs_safepoint`]'s CPU branch describes, and the reason a handler
    /// entered under [`SafepointFlags::CPU_LIMIT`] still stops at its own first
    /// back edge.
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
    /// `false` — [ADR 0005]'s spelling of no ceiling at all — is the one value
    /// that answers `0` and turns the net off. A malformed one takes the default
    /// instead, which is where this reader parts company with
    /// [`Self::configured_cpu_time`]; the field doc owns why. Either way the
    /// file was already parsed and refused at the boundary that could name the
    /// line, so this is not a second place to refuse it.
    ///
    /// [ADR 0005]: ../../../docs/adr/0005-config-changeability.md
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
    fn configured_handler_reserve(&self) -> Option<usize> {
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
    fn configured_handler_reserve_time(&self) -> Option<u64> {
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
    /// mostly program. ADR 0020 § 1 names `fatal_reserve_time` and states no
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
    /// than mostly program. ADR 0020 § 1 states that the slice exists and that
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
    /// can name the file and the line ([ADR 0064](../../../docs/adr/0064-configuration-file-format.md)
    /// § 3), and a second refusal from inside a running request could only be
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

    /// One `[limits]` directive read as a byte count — the arithmetic the two
    /// readers above share.
    ///
    /// Written once rather than per ceiling because a second size directive
    /// growing its own parse is how the two would come to disagree about what
    /// `"32M"` means, and `nvs_config::Quantity` is the one place that question
    /// is answered ([ADR 0064](../../../docs/adr/0064-configuration-file-format.md)
    /// § 5).
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

    /// ADR 0079 § 12's fixed clock in nanoseconds since the Unix epoch, or
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

    /// ADR 0079 § 12's seeded generator's live state, or `None` for a context
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
    /// ADR 0086 § 4's last paragraph, as `Core\Test::scriptAnswers`.
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

    /// A context for a **child task of this request** — what `nvs-host` hands
    /// [`crate::host::Job`] when it runs a group.
    ///
    /// [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
    /// § 1's children "share the request", and this is the one place that
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
    /// the yielder and the stack bounds — the last two because the child will
    /// run on a stack of its own that this context has never seen. So does the
    /// diagnostic sink, which starts at [`OutputSink::Stderr`] like any fresh
    /// context's: an [`OutputSink`] is not `Clone`, and a redirected one is a
    /// test reading its own dumps back rather than a property of the request.
    ///
    /// **What it spends:** one `Ctx` per in-flight child, freed when that child
    /// ends, plus the origin's own bytes copied once. O(in-flight) and not
    /// O(children ever spawned), per
    /// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md).
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
        let mut child = Self::new(OutputSink::Buffer(Vec::new()));
        // Request-wide, and therefore shared or copied.
        child.statics = self.statics;
        child.debug = self.debug;
        child.origin = self.origin.clone();
        child.runtime_error_class = self.runtime_error_class.clone();
        // The word, not its value: a task of this request is bounded by this
        // request's wall time and by no clock of its own. See the field doc.
        child.deadline = std::sync::Arc::clone(&self.deadline);
        // Sealed rather than empty: ADR 0072 § 6's queue is the *request's*, and
        // one on a child would be drained by nobody and released when the child
        // ended. `crate::deferred` is the one home for that rule and for why a
        // refusal is the only honest answer to a registration nothing would run.
        child.deferred = None;
        child
    }

    /// A context for an **isolate** — the other half of the pair
    /// [`Ctx::child`] opens, and the one place the two part.
    ///
    /// [ADR 0116](../../../docs/adr/0116-an-isolates-arena-is-an-ownership-root.md)
    /// § 4: an isolate's arena is an ownership root of its own, so its
    /// static-property base is **its own** rather than an alias of this
    /// request's. That single difference is the whole of ADR 0006's "globals,
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
    ///
    /// What crosses is what ADR 0006's table calls request-wide and immutable:
    /// the debug flags, the origin, the runtime error class table (compiled
    /// code, shared by design) and the deadline word, since a budget is
    /// accounted at the root of the request tree and never per isolate. The
    /// deadline crosses as the *word* and not as its value — one store expires
    /// the whole tree, whenever in the child's life the timer fires — and
    /// [`Self::deadline`]'s field doc owns why a copy was the wrong half of
    /// that. The output sink is the caller's, because `output: 'capture'` and
    /// `output: 'inherit'` differ in nothing else.
    ///
    /// **No ceiling crosses, and that is what makes the budget the tree's.**
    /// ADR 0006's table charges a child's memory and a child's output to the
    /// root, and both counters are the thread's ([`crate::budget`]) with the
    /// child's own zero point taken here by [`Self::new`]: a child reads back
    /// its own share, the root's base predates every child so its reading holds
    /// all of them at once, and the ceiling that stops the tree is the root's.
    /// A child handed a ceiling of its own — [`Self::set_memory_limit`],
    /// [`Self::set_output_limit`] — narrows itself further and can never widen
    /// the tree.
    ///
    /// **What it spends:** one `Ctx` per in-flight isolate plus its own statics
    /// store once armed, both freed when that isolate ends. O(in-flight), per
    /// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md).
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
        // the file. That is the direction ADR 0006's table wants: a parent that
        // narrowed a limit for itself has narrowed it for the tree beneath it,
        // and a child re-reading the snapshot would silently widen it back.
        isolate.config = self.config.clone();
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
        // Sealed for [`Self::child`]'s reason and not for a reason of its own:
        // nothing drains an isolate's queue, and ADR 0072 § 6's work is the
        // request's. `crate::deferred`'s known gap is where an isolate gets one.
        isolate.deferred = None;
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
    /// under [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md)'s
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

    /// A context for [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
    /// § 3's **tier-3 handler** — [`Self::isolate`] with the failing request's
    /// budget left behind.
    ///
    /// § 3's one deliberate exception to [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md):
    /// an ordinary isolate spends the tree's budget, which is exactly wrong for
    /// the one isolate whose job is to report that the tree ran out of it.
    /// Everything ADR 0006 calls request-wide still crosses — the sibling above
    /// is the one home of that list — and three things part from it:
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

    /// The base of the static-property storage compiled code loads inline —
    /// the word at [`STATICS_OFFSET`], handed out rather than re-derived.
    ///
    /// Null before [`Ctx::install_statics`] has run, which is safe because a
    /// unit declaring no static emits no instruction that reads it. Two callers
    /// want it and neither can reach the field: `nvs-host`'s group runner,
    /// which gives a child the *same* base so a request has one copy of every
    /// static rather than one per task ([`Ctx::child`]), and a test asserting
    /// that it did.
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
    fn release_statics(&mut self) {
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
    /// somewhere [ADR 0116](../../../docs/adr/0116-an-isolates-arena-is-an-ownership-root.md)
    /// § 2's wholesale release will reach; this context *is* that ownership
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

    /// Files a started isolate against this request and answers the key that
    /// takes it back out — what a `Core\Script\Handle`'s one slot holds.
    ///
    /// A handle is an object and a `Core` instance has no native drop, so a
    /// key into a table is the only representation available and *whose* table
    /// it is is the whole question. It is the request's, so that the footprint
    /// is O(isolates this request has started and not awaited) and is released
    /// with the request — a process-wide table would be O(spawns served),
    /// which [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md) calls
    /// a leak rather than a trade. `crates/nvs-stdlib/src/channel.rs` records
    /// the same reasoning for the queue it keeps in slots instead.
    ///
    /// **What it spends:** one pointer pair per live handle, and nothing at all
    /// for a request that spawns none. A key is never reused, so a handle
    /// awaited twice reads an empty slot rather than another request's isolate.
    pub fn hold_started_script(&mut self, running: Box<dyn crate::host::Running>) -> u64 {
        self.started_scripts.push(Some(running));
        // The index, one-based, so that a handle slot never holds a key a
        // zeroed value could be mistaken for.
        self.started_scripts.len() as u64
    }

    /// Takes the isolate `key` names back out, or `None` when it has already
    /// been taken — an `await` of a handle a previous `await` consumed.
    #[must_use]
    pub fn take_started_script(&mut self, key: u64) -> Option<Box<dyn crate::host::Running>> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.started_scripts.get_mut(index)?.take()
    }

    /// Files an open file against this request and answers the key that reads
    /// it back — what a `Core\IO\File`'s one slot holds.
    ///
    /// The same shape and the same reasoning as [`Ctx::hold_started_script`],
    /// which is the one home of *why* a `Core` handle is a key into a
    /// request-owned table rather than the native thing itself: a `Core`
    /// instance has no native drop, so a table this module does not own would
    /// never learn that the last reference had gone. What this adds is the
    /// close: a descriptor is scarce in a way an awaited isolate is not, so
    /// `Core\IO\File::close` takes the handle out and drops it, and a request
    /// that forgets closes everything it opened when its `Ctx` goes.
    ///
    /// **What it spends:** one `Option<File>` — a descriptor and a niche — per
    /// `open` this request performed, *including* the ones it has since closed,
    /// because a key is never reused. That is O(opens by one request), released
    /// with the request and charged to its memory limit, and it is the price of
    /// the safety property: a stale handle reads an empty slot and throws,
    /// where a recycled key would silently address whatever file the same slot
    /// now holds. A loop opening and closing a million paths spends a few
    /// megabytes for it, which
    /// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md)'s ordering
    /// spends without hesitating to keep a descriptor from being confused for
    /// another.
    pub fn hold_open_file(&mut self, file: std::fs::File) -> u64 {
        self.open_files.push(Some(file));
        // The index, one-based, so that a handle slot never holds a key a
        // zeroed value could be mistaken for.
        self.open_files.len() as u64
    }

    /// The file `key` names, borrowed for one read or write, or `None` once it
    /// has been closed or if it was never this request's.
    pub fn open_file_mut(&mut self, key: u64) -> Option<&mut std::fs::File> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_files.get_mut(index)?.as_mut()
    }

    /// Takes the file `key` names back out, or `None` when it has already been
    /// taken — a `close` of a handle a previous `close` consumed.
    #[must_use]
    pub fn take_open_file(&mut self, key: u64) -> Option<std::fs::File> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_files.get_mut(index)?.take()
    }

    /// Files an open database connection against this request and answers the
    /// key that reads it back — what a `Core\Db\Connection`'s first slot holds.
    ///
    /// The same shape and the same reasoning as [`Ctx::hold_open_file`], and
    /// [`Ctx::hold_started_script`] is the one home of *why* a `Core` handle is
    /// a key into a request-owned table. What this adds is `memo`, which is
    /// [ADR 0067](../../../docs/adr/0067-core-db.md) § 2's memoization key —
    /// the block's name for `Core\Db::connect`, and `None` for a
    /// `{shared: false}` call, which is exactly what "bypasses memoization"
    /// means: an entry no [`Ctx::memoized_connection`] lookup can match. The
    /// key is held beside the connection rather than in a map of its own
    /// because a request opens a handful of connections at most, so a linear
    /// scan is the whole lookup and an empty request pays no allocation for it.
    ///
    /// There is **no reuse across requests here**, and there is not meant to be
    /// yet: § 13's per-core pool is what makes a connection outlive the request
    /// that opened it, and it may only do so behind that section's reset. Until
    /// it exists, a connection is opened by the request that asks for one and
    /// closed when this context drops, which is the isolating answer rather
    /// than the fast one.
    ///
    /// **What it spends:** one connection — a socket, a TLS session and its
    /// statement cache — per distinct `connect` a request performs, released
    /// with the request. A key is never reused.
    pub fn hold_open_connection(
        &mut self,
        memo: Option<String>,
        connection: Box<dyn HeldConnection>,
    ) -> u64 {
        self.open_connections.push((memo, connection));
        // The index, one-based, so that a handle slot never holds a key a
        // zeroed value could be mistaken for.
        self.open_connections.len() as u64
    }

    /// The connection `key` names, borrowed for one statement, or `None` for a
    /// key this request never filed.
    ///
    /// [`Ctx::open_file_mut`]'s shape, and one difference that is
    /// [`HeldConnection`]'s whole reason: what comes back is the trait object
    /// rather than a driver's own type, because
    /// [ADR 0132](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)
    /// § 1 has `nvs-db` depending on this crate and naming `nvs_db::Connection`
    /// here would close a cycle. The caller that knows which crate opened it
    /// gets its own type back through
    /// [`HeldConnection::as_any_mut`](crate::HeldConnection::as_any_mut) — one
    /// downcast, at the one place a statement is written.
    ///
    /// There is no `take_open_connection` beside it and there is not meant to
    /// be one yet: spec § 18's `Core\Db\Connection::close` is what would take a
    /// connection back out, and until ADR 0067 § 13's pool exists a closed
    /// connection has nowhere to go that dropping it with the request does not
    /// already reach.
    pub fn open_connection_mut(&mut self, key: u64) -> Option<&mut dyn HeldConnection> {
        let index = usize::try_from(key.checked_sub(1)?).ok()?;
        self.open_connections
            .get_mut(index)
            .map(|(_, held)| &mut **held)
    }

    /// The key of the connection this request already opened under `memo`, or
    /// `None` for a name it has not reached yet — § 2's memoization, asked.
    #[must_use]
    pub fn memoized_connection(&self, memo: &str) -> Option<u64> {
        self.open_connections
            .iter()
            .position(|(held, _)| held.as_deref() == Some(memo))
            .map(|index| index as u64 + 1)
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

    /// Asks the next safepoint poll to act.
    pub fn request_safepoint(&mut self, flags: SafepointFlags) {
        self.safepoint |= flags;
    }

    /// Whether this request has been cancelled — the flag [`Ctx::cancel`] sets.
    ///
    /// What it distinguishes is a context that *failed* from one that was
    /// stopped: both carry a pending message afterwards, and only one of them
    /// is a `Throwable` anybody may see. `nvs_host::group` is the caller, for
    /// exactly that question about a child.
    #[must_use]
    pub fn cancelled(&self) -> bool {
        self.safepoint.contains(SafepointFlags::CANCEL)
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
    /// [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
    /// § 5's teardown: no `catch`, no cleanup, no user code at all.
    ///
    /// That is already exactly what [`SafepointFlags::CANCEL`] means, so this
    /// sets the flag and asks [`nvs_safepoint`] for the answer rather than
    /// inventing a second one — the status and its message keep one home, and
    /// what comes back is what the poll compiled code was going to make anyway
    /// would have said, only without the statements in between.
    pub fn cancel(&mut self) -> crate::Fault {
        self.request_safepoint(SafepointFlags::CANCEL);
        #[expect(
            unsafe_code,
            reason = "the pointer is a reborrow of this `&mut self`, which is \
                      live for the whole call"
        )]
        let status = unsafe { nvs_safepoint(&raw mut *self) };
        crate::Fault::Pending(status)
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

    /// The descriptor for the class `name` spells in **this program's** table,
    /// or `None` for a name it does not declare.
    ///
    /// The one route from a `Core` member to a class the *program* wrote, and
    /// it exists for [`crate::graph::decode`]: ADR 0023 § 3 refuses a payload
    /// naming a class the receiving side cannot resolve, which is a question
    /// only the compiled unit's own table can answer. It reads the table
    /// [`Self::set_runtime_error_class`] installed rather than a second
    /// registration, for that method's own reason — the six § 10 classes and
    /// every class the program declares are all rows of one table, and a
    /// second handle on it would be a second thing to keep in step.
    ///
    /// The pointer is live for as long as this context is: the handle shares
    /// ownership of the table ([`ErrorClass`]).
    #[must_use]
    pub fn class_desc(&self, name: &str) -> Option<*const ClassDesc> {
        Some(self.runtime_error_class.as_ref()?.sibling(name)?.desc())
    }

    /// The same table as a **handle that keeps it alive by itself** — one
    /// [`ErrorClass`] is a handle on the whole table, since `sibling` reaches
    /// any class in it by name.
    ///
    /// [`Self::class_desc`] answers for a caller holding this context; this
    /// answers for one that will still be asking after it has let go of it.
    /// Its caller is the isolate boundary: a child's answer is copied out on
    /// the child's own stack, where the *parent's* context is borrowed by the
    /// frame parked on the join, so the receiving side has to have been taken
    /// before the child started. Cloning it is an `Rc` bump and holding it
    /// keeps one class table alive for the length of one isolate, which is
    /// bounded by what is in flight.
    #[must_use]
    pub fn class_table(&self) -> Option<ErrorClass> {
        self.runtime_error_class.clone()
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

    /// Records a `THROWN` of `class` carrying `message`, with `value` written
    /// into slot `slot` of the object it builds —
    /// [`crate::Fault::ThrownWithSlot`]'s one destination, and so the whole of
    /// how a native member fills a property below `Throwable`'s four
    /// (`ParseError::$issues`, `Core\Db\DbError::$kind`).
    ///
    /// The object is built **here** rather than left as a [`Pending::Message`]
    /// to be promoted later, which is what keeps the pending state free of an
    /// owned reference: every path that replaces or discards a pending failure
    /// would otherwise have to release one, and exactly one of those paths
    /// being missed is the shape a refcount leak takes. Only a member with
    /// something to put in the slot reaches this, so the eager allocation is on
    /// a path that has already allocated.
    ///
    /// A `class` whose descriptor is too narrow for `slot` releases `value` and
    /// throws without it — [`Thrown::new_as`] owns that, and it is the same
    /// "nothing installed" case a null descriptor is.
    ///
    /// # Safety
    ///
    /// `value` must be a value whose reference is being transferred here.
    #[expect(
        unsafe_code,
        reason = "the value's reference and the installed descriptor's liveness \
                  are both obligations the signature cannot express"
    )]
    pub unsafe fn raise_with_slot(
        &mut self,
        class: ThrownClass,
        message: &str,
        slot: usize,
        value: crate::Value,
    ) {
        let desc = self.error_desc(class);
        #[expect(
            unsafe_code,
            reason = "the descriptor comes from the `Rc`-shared table this \
                      context holds, so it outlives the instance; the value's \
                      reference is forwarded"
        )]
        let thrown = unsafe { Thrown::new_as(desc, class, message, Some((slot, value))) };
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

    /// One extra slot of the pending failure's exception object, read
    /// **without clearing it** — [`Self::take_thrown`]'s borrowing half, for a
    /// caller that has to decide something about a throw it may still re-raise
    /// unchanged.
    ///
    /// [ADR 0067](../../../docs/adr/0067-core-db.md) § 8's retry loop is why
    /// this exists: it has to know whether the closure's own refusal was a
    /// deadlock or a serialization failure before it decides to run the
    /// closure again, and `take_thrown` would clear the very failure it is
    /// still deciding about — a decision that came out "do not retry" would
    /// then have to re-raise a throw it had already consumed.
    ///
    /// `class` is not decoration. `KIND_SLOT` and `ISSUES_SLOT` are the same
    /// number, so a slot index alone would read a `ParseError`'s issue array
    /// as a `Core\Db\ErrorKind`; the answer is `None` unless the pending
    /// failure is an instance of `class`, by [`Self::pending_conforms_to`]'s
    /// reading of that word — the one a `catch` naming it would bind.
    ///
    /// `None` also where nothing is pending, where the failure is a bare
    /// [`Pending::Message`] with no object behind it at all, and where the
    /// class declares too few fields to hold that slot
    /// ([`Thrown::field`]). The value is borrowed, not retained: it is good
    /// only while the failure is still pending.
    #[must_use]
    pub fn pending_slot(&self, class: &str, slot: usize) -> Option<Value> {
        if !self.pending_conforms_to(class) {
            return None;
        }
        match self.pending.as_ref()? {
            Pending::Message(_, _) => None,
            Pending::Thrown(thrown) => thrown.field(slot),
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
        // `[limits] max_output` is bytes written to the *response*, so the
        // charge is here: below the capture, above the sink. What a capture
        // swallowed is not a response yet and is already bounded by
        // `[limits] memory`, the capture buffer being heap `crate::budget`
        // counts; it is charged when the program writes the captured text back
        // out, and charging it here as well would bill the same bytes twice.
        //
        // Charged whether or not *this* context has a ceiling, because the
        // counter is the thread's and the context holding the ceiling may be a
        // parent two levels up. The compare is `Ctx::over_output_limit`'s and
        // happens at the safepoint poll.
        crate::budget::wrote(bytes.len());
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

    /// Writes one rendered record where `[log] target` says, and where
    /// `unconfigured` says when the directive names nothing.
    ///
    /// **This is the only reader of that directive**, and both of
    /// [ADR 0092](../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
    /// § 6's writers reach it: `Core\Log::write` with [`LogChannel::Output`]
    /// and [`crate::floor::report`] with [`LogChannel::Diagnostic`]. § 6's
    /// claim is about *sameness* — one serialiser, two callers — and a
    /// destination each caller resolved for itself is the second way that
    /// sameness could be lost after the record's shape.
    ///
    /// **A named target wins over `Core\Out::capture`.** The record leaves
    /// through the sink rather than through [`Self::write_output`], so ADR 0088
    /// § 5's capture stack does not see it and `[limits] max_output` is not
    /// charged for it. Both follow from what the directive means: an operator
    /// naming a destination is saying where the deployment's records go, and a
    /// program capturing its own output has said nothing about that. With no
    /// target configured the record is still the program's output and both
    /// rules apply to it exactly as before.
    ///
    /// **`[log] level` is the floor, and it is read here for the same reason.**
    /// ADR 0092 § 2's last paragraph makes the directive the minimum level
    /// written, so a record quieter than it is dropped and answers `Ok`: it was
    /// not written, and nothing failed. Asked at this one call rather than at
    /// each writer, so the two of them cannot come to disagree about which
    /// records a deployment collects — which is § 6's sameness a second time,
    /// after the record's shape and its destination.
    ///
    /// The comparison is `<` over [`Level`]'s own ordering, which is § 2's
    /// roster quietest-first, and so is that section's `<=` over the syslog
    /// severities read the other way round — those run *downward*, `Debug` at 7
    /// and `Critical` at 2. Written as the enum ordering because that is the
    /// one of the two spellings a reader cannot get backwards.
    ///
    /// **`[log] format` picks the rendering, which is why this takes a
    /// [`Record`] and not bytes.** ADR 0092 § 3 gives a log target two of its
    /// three renderings — JSON Lines and plaintext — and § 6's producers name
    /// none of them, so the choice belongs at the sink and nowhere else. A
    /// caller that rendered first would be a caller that had chosen, and the
    /// two of them would have chosen separately: the same drift the record's
    /// shape, its destination and its floor are each held here to avoid. The
    /// price is one `String` per written record, which is what the caller
    /// allocated before.
    ///
    /// The three directives are read once — see [`Self::set_config`] — and the
    /// sink `target` names is held for the life of the context, because a
    /// rotation bound counted against a handle needs the handle to survive the
    /// record.
    ///
    /// # Errors
    ///
    /// Whatever the sink returns; a file target's failure is the caller's to
    /// swallow, which is ADR 0020 § 4's answer at the floor.
    pub fn write_log_record(
        &mut self,
        record: &Record,
        unconfigured: LogChannel,
    ) -> io::Result<()> {
        if matches!(self.log, LogTarget::Unread) {
            self.log = self.resolve_log_target();
            self.log_minimum = self.resolve_log_minimum();
            self.log_format = self.resolve_log_format();
        }
        if record.envelope.level < self.log_minimum {
            return Ok(());
        }
        let rendered = match self.log_format {
            LogFormat::Json => nvs_render::json::line(record),
            LogFormat::Text => nvs_render::plain::render(record),
        };
        let line = rendered.as_bytes();
        if let LogTarget::Named(sink) = &mut self.log {
            return write_to(sink, line);
        }
        match unconfigured {
            LogChannel::Output => self.write_output(line),
            LogChannel::Diagnostic => self.write_diagnostic(line),
        }
    }

    /// `[log] target` as the sink it names, through ADR 0020 § 4's grammar and
    /// not through a second reading of it.
    ///
    /// [`nvs_config::log::Target`] is that grammar and it has two readers:
    /// this one, and the boot check that refuses a tree naming a target § 4
    /// does not spell. So a value that reached here is one of three, and
    /// [`LogTarget::Unnamed`] covers two facts rather than one:
    ///
    /// - **`syslog` is spelled and not yet transported.** A syslog sink is a
    ///   datagram to a platform endpoint carrying ADR 0092 § 2's severity in a
    ///   priority field — a transport, a framing and an argument the
    ///   byte-oriented sinks here do not take. Routing it to `stderr` instead
    ///   would be this module claiming a destination it does not reach, so it
    ///   routes nowhere new and each writer's own channel still carries the
    ///   record.
    /// - **A target nobody spelled** never boots, so reaching it here means a
    ///   context was configured by something other than a resolved tree — a
    ///   test, in practice. It is not a diagnostic at the one moment the
    ///   engine has a failure to report; it is the unconfigured routing.
    fn resolve_log_target(&self) -> LogTarget {
        let Some(written) = self
            .config
            .as_ref()
            .and_then(|config| config.get("log.target"))
        else {
            return LogTarget::Unnamed;
        };
        match nvs_config::log::Target::of(&written) {
            Some(nvs_config::log::Target::Stderr) => LogTarget::Named(OutputSink::Stderr),
            Some(nvs_config::log::Target::File(path)) => LogTarget::Named(OutputSink::File(
                crate::logfile::LogFile::new(std::path::PathBuf::from(path)),
            )),
            Some(nvs_config::log::Target::Syslog) | None => LogTarget::Unnamed,
        }
    }

    /// What `[log] level` names, or [`Level::Debug`] where it names nothing —
    /// [`Self::write_log_record`]'s floor, resolved with the target above.
    ///
    /// `Debug` for an unset directive rather than ADR 0091 § 3's per-mode
    /// `Info`: that default is applied to the *tree*, so a resolved
    /// configuration already carries it here, and a context configured by
    /// something other than a resolved tree has said nothing about which
    /// records it wants. The safe answer to that is all of them. A word the
    /// grammar does not carry reads the same way and never boots — `E0614`
    /// refuses it at the file, for the reason `nvs_config::log`'s module doc
    /// gives about doing this at boot rather than at the first record.
    fn resolve_log_minimum(&self) -> Level {
        self.config
            .as_ref()
            .and_then(|config| config.get("log.level"))
            .and_then(|written| Level::of(&written))
            .unwrap_or(Level::Debug)
    }

    /// What `[log] format` names, or [`LogFormat::Json`] where it names
    /// nothing — [`Self::write_log_record`]'s rendering, resolved with the two
    /// directives above.
    ///
    /// One record per line for an unset directive, which is both ADR 0091 § 3's
    /// per-mode default and [`LogFormat`]'s own: a pipeline reading a target
    /// nobody configured can find the record boundaries without being told, and
    /// the plaintext rendering's are a blank-line-free block. A word the grammar
    /// does not carry reads the same way and never boots — `E0615` refuses it at
    /// the file.
    fn resolve_log_format(&self) -> LogFormat {
        self.config
            .as_ref()
            .and_then(|config| config.get("log.format"))
            .and_then(|written| LogFormat::of(&written))
            .unwrap_or(LogFormat::Json)
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
            OutputSink::Stdout | OutputSink::Stderr | OutputSink::File(_) | OutputSink::Sink => {
                None
            }
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
            // A `LogFile` writes straight through — there is no buffer of its
            // own between `write_all` and the descriptor.
            OutputSink::Buffer(_) | OutputSink::File(_) | OutputSink::Sink => Ok(()),
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
            OutputSink::Stdout
            | OutputSink::Stderr
            | OutputSink::Buffer(_)
            | OutputSink::File(_)
            | OutputSink::Sink => CARRIER_CLI_TEXT,
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

    /// Whether what this request writes reaches the process's own standard
    /// streams, rather than a buffer, a response body or nothing at all.
    ///
    /// ADR 0086 § 4's prompts are the caller: a question is only a question if
    /// the person answering can see it, so `Core\Cli::ask` under `nvs serve`,
    /// inside a `Core\Out::capture` or under a test's [`OutputSink::Buffer`]
    /// is not interactive however many terminals the process has. Without
    /// this, a prompt in a request handler would write into the response body
    /// and then block the core waiting for a keystroke.
    #[must_use]
    pub fn output_reaches_the_terminal(&self) -> bool {
        self.captures.is_empty() && matches!(self.output, OutputSink::Stdout | OutputSink::Stderr)
    }

    /// Takes everything written so far, if this context buffers its output.
    #[must_use]
    pub fn take_buffered_output(&mut self) -> Option<Vec<u8>> {
        match &mut self.output {
            OutputSink::Buffer(buffer) => Some(std::mem::take(buffer)),
            OutputSink::Stdout | OutputSink::Stderr | OutputSink::File(_) | OutputSink::Sink => {
                None
            }
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
        OutputSink::File(file) => file.write(bytes),
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

    /// The live list of that same context, or null — see [`CurrentCtx`].
    ///
    /// A second word rather than a hop through [`CURRENT`], because the two
    /// readers want different things: the release path wants the context, and
    /// [`crate::object::NvsObj::alloc`] wants a list it may write to while a
    /// helper above it holds `&mut Ctx`. Reaching the list *through* the
    /// context would make every object allocation write into an allocation
    /// that reference claims exclusively; the `Rc` it names does not.
    static CURRENT_LIVE: std::cell::Cell<*const crate::object::LiveList> =
        const { std::cell::Cell::new(std::ptr::null()) };
}

/// The live list of this thread's current context, or null when no compiled
/// frame is running — [`crate::object::NvsObj::alloc`]'s one reader.
pub(crate) fn current_live_list() -> *const crate::object::LiveList {
    CURRENT_LIVE.get()
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
pub(crate) struct CurrentCtx(*mut Ctx, *const crate::object::LiveList);

impl CurrentCtx {
    /// Makes `ctx` this thread's current context until the guard drops.
    pub(crate) fn install(ctx: &mut Ctx) -> Self {
        let live = std::rc::Rc::as_ptr(&ctx.live);
        Self(CURRENT.replace(&raw mut *ctx), CURRENT_LIVE.replace(live))
    }
}

impl Drop for CurrentCtx {
    fn drop(&mut self) {
        CURRENT_LIVE.set(self.1);
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
                  unwind out of the JIT frame above — the one thing reached \
                  from here that is not a load and a compare is ADR 0020 § 1's \
                  handler, whose own helper calls are each contained by \
                  `run_helper`"
    )]
    let ctx = unsafe { &mut *ctx };

    if ctx.safepoint.contains(SafepointFlags::CPU_LIMIT) {
        // ADR 0020 § 1 lists CPU time beside memory, so the ladder is the same
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
    // ADR 0020 § 1's memory limit, asked here as well as at every helper
    // boundary ([`crate::run_helper`]): this poll sits between two Novis
    // statements, which is the one place a limit can stop a program that is
    // allocating without calling anything. `crate::budget`'s module doc owns
    // which allocations that reaches today and which it does not.
    if let Some(crate::Fault::Fatal(message)) = ctx.memory_breach() {
        // ADR 0020 § 1's tier 1, ahead of the status this returns: the handler
        // is the last thing the program gets to run, and it runs before the
        // breach becomes the message the ladder prints — so a throw of its own
        // is overwritten by `set_pending` below rather than reported in place
        // of the limit that stopped the request. `Ctx::run_limit_handler` owns
        // the zero-retry rule.
        ctx.run_limit_handler(Limit::Memory);
        ctx.set_pending(message);
        return crate::FATAL;
    }
    // ADR 0020 § 1's response ceiling, asked here and nowhere else. A program
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

    /// ADR 0067 § 8's retry loop reads a refusal's `kind` off a failure it has
    /// not decided about yet, so the read leaves the pending exactly as it
    /// found it — and the class it names is the whole of what keeps
    /// `KIND_SLOT` from reading a `ParseError`'s `issues` back as an
    /// `ErrorKind`, the two being the same slot number.
    #[test]
    fn a_pending_refusals_kind_reads_back_without_disturbing_it() {
        const WIDE: [&str; 5] = ["message", "previous", "backtrace", "location", "kind"];
        const NARROW: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut table = ClassTable::new();
        let root = table.define("RuntimeError", &NARROW, &[]);
        table.define("Core\\Db\\DbError", &WIDE, &[root]);
        table.define("ParseError", &WIDE, &[root]);

        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::rc::Rc::new(table), root));
        #[expect(
            unsafe_code,
            reason = "an `int` carries no reference for the slot to take over"
        )]
        unsafe {
            ctx.raise_with_slot(
                ThrownClass::DbError,
                "refused",
                crate::KIND_SLOT,
                Value::int(5),
            );
        }

        assert_eq!(
            ctx.pending_slot("Core\\Db\\DbError", crate::KIND_SLOT)
                .and_then(Value::as_int),
            Some(5)
        );
        assert!(
            ctx.pending_slot("ParseError", crate::KIND_SLOT).is_none(),
            "the slot number is the same one; only the class tells them apart"
        );
        assert_eq!(
            ctx.pending_class().as_deref(),
            Some("Core\\Db\\DbError"),
            "reading a slot decides nothing and clears nothing"
        );

        let taken = ctx.take_thrown();
        assert_eq!(taken.message(), "refused");
        assert!(
            ctx.pending_slot("Core\\Db\\DbError", crate::KIND_SLOT)
                .is_none(),
            "nothing is pending once it has been taken"
        );
    }

    /// ADR 0020 § 1's first resource limit, as far as this slice goes: the
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

    /// ADR 0006's "child output against the root's `max_output`", as the two
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

    /// `[limits] max_output` bounds the *response*: what a capture swallowed is
    /// not one yet, and is charged when the program writes it back out rather
    /// than at both points.
    #[test]
    fn captured_bytes_are_charged_when_they_reach_the_sink_and_not_before() {
        let mut ctx = Ctx::buffered();
        ctx.begin_capture();
        ctx.write_output(b"inside").expect("a buffer");
        let taken = ctx.end_capture().expect("a capture was open");
        assert_eq!(ctx.output_used(), 0);

        ctx.write_output(&taken).expect("a buffer");
        assert_eq!(ctx.output_used(), 6);
    }

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

    /// ADR 0006's "one ceiling to divide": the flag is the tree's own word, so
    /// the store reaches a child built before the timer fired. A copy answered
    /// the other order correctly and this one not at all, which is why the
    /// child here is spawned first.
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
