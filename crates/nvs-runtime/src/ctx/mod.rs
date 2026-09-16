//! The per-request context: the first argument of every compiled function and
//! every helper.
//!
//! `Ctx` is where everything that is "ambient" to running Novis code lives,
//! because `rule:statements/no-host-populated-variables` means nothing
//! is ambient to the *language*: no variable is host-populated, so the host's
//! state has to travel somewhere, and it travels here.
//!
//! # Layout is part of the ABI
//!
//! The hot words come first, in a `#[repr(C)]` struct, because compiled
//! code loads them inline rather than calling anything:
//!
//! * [`SAFEPOINT_OFFSET`] — the *address* of the word the safepoint poll
//!   `nvs-codegen` emits at every function entry and loop back edge reads
//!   (`docs/adr/README.md`'s project-start decisions). The address is loaded
//!   once in the ABI entry block, which dominates every block below it, so the
//!   poll itself is a load of the word, a test, and a predicted-not-taken
//!   branch to the [`nvs_safepoint`] slow path. The word is out of line
//!   because the threads that raise a bit in it — a CPU sampler, a
//!   cancellation, the allocator — are not the one running the request, and a
//!   word inside this struct could only be written through the `&mut Ctx` a
//!   helper body is already holding.
//! * [`DEBUG_FLAGS_OFFSET`] — `rule:testing/debug-probes`'s probe check, at every statement boundary and every call site. Same
//!   shape, same cost class, and present in every compiled unit whether or not
//!   any request ever sets a bit — that is what makes coverage and tracing
//!   start/stoppable *mid-request*, which the rejected instrumented-tier
//!   design could not do.
//! * [`STACK_LIMIT_OFFSET`] — `rule:errors/on-limit`'s call-stack ceiling, compared against the stack pointer at the same
//!   emit site the safepoint poll uses. It sits in this line rather than
//!   anywhere colder precisely so the compare costs a load that is already
//!   paid for.
//! * [`DEADLINE_OFFSET`] — `rule:http-server/time-is-bounded-inside-a-helper`
//!   's deadline flag, polled from *inside* a helper whose runtime scales
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
//! `rule:errors/propagation`'s checked returns
//! could carry. `rule:errors/on-limit`'s answer is a bounds pair, armed per request and compared at every
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
//! correct on a stack at least that deep and permissive on a shallower one,
//! where the guard page is still reached first and `nvs-codegen`'s
//! `enable_probestack` still turns that into a clean crash rather than a stack
//! clash. Reading a thread's true bounds needs a platform call this crate has
//! no dependency for; the request's stack becomes Novis's own to size at M6,
//! and until then an embedder that knows its bounds calls
//! [`Ctx::arm_stack_limit`] with them.
//! — owner: unowned
//!
//! # The request's deadline
//!
//! The safepoint word above bounds Novis code because compiled code polls it
//! between calls. A helper is *one* call, so a helper whose runtime is O(its
//! input) — a sort, a scan, an encode, a hash over a large value — runs
//! entirely inside the gap that poll leaves.
//! `rule:http-server/time-is-bounded-inside-a-helper`
//! 's answer is [`Ctx::deadline_expired`]: a flag, in the line the stack
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
//! `rule:security/isolate-shares-nothing` gives a tree
//! one ceiling to divide and charges a child's CPU to the root, and the timer
//! that expires a request only ever holds the root to fire at. [`Ctx::isolate`]
//! owns what the sharing costs.
//!
//! **Who reads it is not a member's decision.** `rule:http-server/time-is-bounded-inside-a-helper`'s first constraint
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
use std::net::IpAddr;

use nvs_config::log::Format as LogFormat;
use nvs_render::{Level, Record};

use crate::object::{ClassDesc, ClassId, ClassTable, FieldDefault};
use crate::throwable::{Thrown, ThrownClass};

/// How the exit queue is drained — [`Ctx::exit_drain`]'s one word, named
/// because the seam is spelled out at both ends of it and `nvs-stdlib`'s end is
/// in another crate.
pub(crate) type ExitDrain = fn(&mut Ctx, Result<(), i32>, Option<&Thrown>);

/// The `Core` half of [`Ctx::class_desc`]'s answer: a name to the descriptor
/// `nvs-stdlib` leaked for it, or `None` for a name that crate does not own.
///
/// A plain `fn` and not a boxed closure, because the table behind it is the
/// **process's** — one leaked `ClassTable`, built on first use and never
/// dropped — so there is nothing per request to capture and installing this
/// costs one word.
pub type CoreClasses = fn(&str) -> Option<*const ClassDesc>;
use crate::value::Value;

mod answers;
mod current;
mod error;
mod held;
mod hooks;
mod inbound;
mod isolate;
mod limits;
mod output;
mod safepoint;
mod trace;
mod wiring;

// A glob re-export takes each item at its own visibility, so what these
// lines decide is only the ceiling. `limits` and `hooks` have no line at all:
// everything in them is a method on `Ctx`, found through the type rather than
// through a module path. `isolate` is named for its one exception, the seed a
// context crosses a thread as.
pub(crate) use self::current::*;
// The one item of that module a host needs: `nvs_host`'s stack switch is what
// carries the pair off the thread and back — see [`CurrentStack`].
pub use self::answers::*;
pub use self::current::CurrentStack;
pub use self::error::*;
pub use self::held::*;
pub use self::inbound::*;
pub use self::isolate::PlacedIsolate;
pub use self::output::*;
pub use self::safepoint::*;
pub use self::trace::*;
pub use self::wiring::*;

bitflags::bitflags! {
    /// What a safepoint poll has been asked to do.
    ///
    /// The word is checked, not the individual bits: compiled code branches on
    /// "is this non-zero", and only the [`nvs_safepoint`] slow path looks at
    /// which bit is set.
    ///
    /// **Every bit here is the request tree's.** The word lives outside [`Ctx`]
    /// and a child polls the one its root does, so a bit raised in it is raised
    /// for every context in the tree — which is what a ceiling the tree divides
    /// needs, and what stops a runaway wherever in the tree it is running. A
    /// cancellation is not one of those: cancelling one task is not stopping
    /// the request, and `nvs_host::group` asks a child about its own
    /// cancellation exactly to tell a child that *failed* from one that was
    /// stopped. So it is [`Ctx::cancel`]'s own per-context flag, and bit 1 is
    /// absent here rather than reused.
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    #[repr(transparent)]
    pub struct SafepointFlags: u64 {
        /// The request has exceeded its CPU-time budget.
        const CPU_LIMIT = 1 << 0;
        /// The cycle collector wants to stop the world.
        const COLLECT = 1 << 2;
        /// A debugger wants to break here.
        const DEBUG_BREAK = 1 << 3;
        /// A growing allocation has carried this request past
        /// `rule:errors/on-limit`'s memory ceiling.
        ///
        /// Raised by [`crate::budget`]'s threshold, at the allocation, and it
        /// is a request to *poll* rather than the verdict: the branch it wakes
        /// asks [`Ctx::memory_breach`] against the counter, so a crossing that
        /// has been given back again by the time the poll arrives lowers this
        /// and carries on. That is what makes an allocation's share of the work
        /// one compare and a store — the ceiling is read where it was already
        /// being read, between two statements.
        const MEMORY_LIMIT = 1 << 5;
        /// A terminating signal has been delivered to this process and the
        /// handler `Core\Signal::onShutdown` registered has not run yet.
        ///
        /// The one flag whose branch is not a stop: it runs user code and
        /// returns [`crate::OK`], because a graceful shutdown asks the request
        /// to finish rather than ends it where it stands
        /// (`rule:concurrency/a-drain-closes-a-connection-cleanly`). Raising it
        /// is what makes the delivery — which is a store in a signal context and
        /// nothing else — reach Novis code at a safepoint.
        const SHUTDOWN = 1 << 4;
    }
}

bitflags::bitflags! {
    /// `rule:testing/debug-probes`'s per-request debug-flags word.
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

/// Per-request state, passed to every compiled Novis function and every helper.
#[repr(C)]
#[derive(Debug)]
pub struct Ctx {
    /// Hot. The address of `safepoint_word`, loaded inline by every safepoint
    /// poll; see the module docs. **Only compiled code follows it** — every
    /// reader on this side goes through the handle beside it, which is why
    /// nothing here is `unsafe`.
    ///
    /// A pointer is the eight bytes the flags word was, so the arrangement
    /// `tests::the_hot_words_come_first_and_are_a_word_apart` pins is the one
    /// it always was.
    ///
    /// **What it spends:** one allocation per request *tree* — [`Self::child`]
    /// and [`Self::isolate`] share the root's — and one pointer hop, which the
    /// poll does not pay: the address is fixed for the life of the context, so
    /// `nvs-codegen` binds it once at function entry and a back edge keeps the
    /// single load it always had.
    safepoint: *const std::sync::atomic::AtomicU64,
    /// Hot. Read inline by every `rule:testing/debug-probes` probe site; see the module docs.
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
    /// `rule:security/isolate-shares-nothing` gives a
    /// tree "one ceiling to divide" and charges a child's CPU to the root. A
    /// copied flag would satisfy that only in one direction — a child built
    /// *after* the timer fired would be born expired, while one built a
    /// microsecond before it would run on with a zero of its own that nothing
    /// would ever set, because the timer holds the root and no registry of live
    /// children exists for it to walk. So [`Self::child`] and [`Self::isolate`]
    /// clone the handle, and the tree stops on the one store.
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
    /// The state this request tree shares — the safepoint word [`Self::safepoint`]
    /// names, and the counters a member of the tree running on another core
    /// charges into. [`TreeState`] owns what it holds and what it costs.
    ///
    /// Cold, and below `statics` for [`Self::memory_base`]'s reason: compiled
    /// code loads the address above and never this field, and the hot line is
    /// an arrangement this crate's tests pin by offset.
    ///
    /// **Shared with every context in the request tree.**
    /// `rule:security/isolate-shares-nothing` gives a tree one ceiling to
    /// divide, so it gives it one word to be stopped by and one pair to be
    /// counted in: a child built after a flag was raised must not be born clean,
    /// and the raiser holds the root and has no registry of live children to
    /// walk. [`Self::child`] and [`Self::isolate`] therefore clone the handle
    /// rather than copy what it holds, through [`Self::share_safepoint_with`] —
    /// the one place the handle and the address are written together, which
    /// [`Self::join_tree`] is across a thread boundary.
    tree: std::sync::Arc<TreeState>,
    /// Whether this context has been cancelled, which [`Self::cancel`] is the
    /// only writer of.
    ///
    /// Per context rather than a bit in the word above, because the word is the
    /// whole tree's and a cancellation is one context's: a task cancelled by
    /// its group has not stopped the request that spawned it, and the group
    /// runner asks each child this to tell one that *failed* from one that was
    /// stopped.
    ///
    /// It needs no poll to be delivered, which is why nothing is lost by
    /// keeping it out of the word compiled code reads. [`Self::cancel`] takes
    /// `&mut self`, so the only party that can raise it is one holding
    /// exclusive access — and the thread running this context holds that for as
    /// long as it runs, so a cancellation is always handed to a frame that is
    /// parked or to the frame that asked for it, and that frame is the one that
    /// stops.
    cancelled: bool,
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
    /// Compiled code never loads it: the hot words at the top are an
    /// arrangement this crate's tests pin by offset, so a field added among
    /// them moves `statics` and fails them. Nothing below `statics` has that
    /// constraint.
    memory_base: isize,
    /// The high-water mark this context displaced when it rebased
    /// [`crate::budget::peak_bytes`] to its own baseline, republished as the
    /// larger of the two when it drops.
    /// `rule:observability/a-memory-peak-is-recorded-not-asked-for` owns why a
    /// nested context saves and restores rather than clobbering: an isolate
    /// that allocated little would otherwise erase the peak of the request that
    /// spawned it. Cold, and beside [`Self::memory_base`] for that field's own
    /// reason.
    memory_peak_saved: isize,
    /// The allocator threshold this context displaced when it armed its own,
    /// armed again as it drops.
    ///
    /// [`Self::memory_peak_saved`]'s arrangement applied to
    /// [`crate::budget::armed_ceiling`], and for a sharper reason: the thread's
    /// armed pair names a word by address, so a context that took the arming
    /// and did not give it back would leave the allocator publishing into a
    /// request that has ended. Restoring here is what keeps the armed address
    /// one a live context holds — [`crate::budget::arm`] states the obligation
    /// this field discharges. Cold, and beside its twin for that field's own
    /// reason.
    memory_ceiling_saved: crate::budget::Armed,
    /// Whether the thread was already carrying a refused allocation when this
    /// context took the arming, handed back as it drops.
    ///
    /// [`Self::memory_ceiling_saved`]'s arrangement applied to the verdict that
    /// travels with the threshold. A refusal is recorded per thread because the
    /// allocators that hit one hold no `Ctx` to record it on, and this field is
    /// what confines it to the request it belongs to: a request born on a
    /// worker whose last request was refused starts clear, and an isolate gives
    /// its parent's answer back rather than the one it was stopped by.
    memory_refused_saved: bool,
    /// The thread's output-byte count when this context was made — the zero
    /// point [`Self::output_used`] measures this request's own writing from.
    /// [`Self::memory_base`]'s twin in every respect, the reason it sits below
    /// `statics` included.
    output_base: usize,
    /// Whether this context runs on the core its request tree's root runs on,
    /// which is every context of a tree that has placed no child elsewhere.
    ///
    /// What it decides is one branch in [`Self::memory_used`] and
    /// [`Self::output_used`]: a context on the root's core adds the tree's
    /// off-core counters to its thread-local share, and one off it adds nothing,
    /// because its own thread's balance already holds every context beneath it
    /// there. [`Self::join_tree`] is the only writer and
    /// [`Self::share_safepoint_with`] carries the answer down.
    on_root_core: bool,
    /// What this context has already published into its tree's off-core
    /// counters, or `None` for one that publishes nothing — which is every
    /// context on the root's core and every descendant of a member off it.
    ///
    /// `Some` is the whole of what makes a context a *publisher*, so an
    /// ordinary request pays one predictable branch per poll and no atomic.
    /// [`Self::publish_off_core`] is the step and [`TreeShare`] the pair.
    tree_share: Option<TreeShare>,
    /// `[limits] memory` as a byte count, or `0` for a request under no cap —
    /// `rule:errors/on-limit`'s
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
    /// **No reserved slice, unlike its siblings.**
    /// `rule:errors/on-limit` carves
    /// one out of `memory` and one out of `cpu_time` because a tier-1 handler
    /// cannot run without allocating and cannot run without taking time. It can
    /// run without writing, and nothing refuses a write in the first place —
    /// this ceiling is *noticed* at the safepoint poll and never enforced at
    /// [`Self::write_output`]. So there is no `fatal_reserve_output` for
    /// [`Self::refresh_limits`] to carve, and this one field is the whole of
    /// what the directive becomes.
    ///
    /// **Read twice, in two units.** [`Self::output_limit`] reads it as the
    /// running total above; [`Self::intake_limit`] reads the same number as the
    /// ceiling on one buffer a `Core` member fills from outside the request,
    /// which is what `rule:core-classes/process-run` reuses rather than adding a
    /// second directive. Those two readings fail differently and that method's
    /// doc owns why.
    ///
    /// **What it spends:** one word per request.
    output_limit: usize,
    /// `rule:errors/on-limit`'s
    /// tier-1 handler: the closure `Core\Fatal::onLimit` registered, owned, or
    /// `null` for a request that registered none.
    ///
    /// **Here, beside the ceiling, because this is where the breach is asked.**
    /// § 1 makes the registration request-local and puts it next to the pending
    /// slot for the reason this field is a field at all: it dies with the
    /// request, exactly as `rule:statements/static-is-a-member-modifier`
    /// and `rule:statements/no-host-populated-variables` require of
    /// everything a request holds, so there is no process-wide table for a
    /// second request to inherit one from.
    ///
    /// Cold: nothing loads it inline, and only the slow path that has already
    /// decided a limit was breached reads it.
    ///
    /// **What it spends:** two words per request, and one reference to the
    /// closure for a request that registers one — O(in-flight requests), per
    /// `rule:programs/memory-priority`.
    limit_handler: Value,
    /// `rule:errors/on-limit`'s
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
    /// `rule:errors/on-limit`'s
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
    /// — `rule:errors/on-uncaught-throw`'s
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
    /// `rule:programs/memory-priority`.
    uncaught_handler: Value,
    /// Takes ownership of the closure `Core\Signal::onShutdown` registered —
    /// what this request runs when the process is asked to stop.
    ///
    /// The third handler slot, and the one whose *firing* is not a failure:
    /// [`Self::run_shutdown_handler`] widens no ceiling and returns the request
    /// to what it was doing, because a drain asks a request to finish rather
    /// than stops it ([`crate::drain`]).
    ///
    /// Request-local for [`Self::limit_handler`]'s reason, and released in the
    /// same place for it: the request ending is the only unregistration. That a
    /// process-wide event is answered by a per-request slot is the point —
    /// `rule:security/no-cross-request-state` leaves no table for a second
    /// request to inherit a handler from, so the delivery raises a flag on each
    /// running request and each of them runs its own handler or none.
    ///
    /// **What it spends:** one word per request, and one reference to the
    /// closure for a request that registers one — O(in-flight requests), per
    /// `rule:programs/memory-priority`.
    shutdown_handler: Value,
    /// `rule:observability/script-on-exit`
    /// 's end-of-script queue, in registration order — what
    /// `Core\Script::onExit` appends to and [`Self::run_exit_hooks`] drains
    /// once, as the last user code of the script.
    ///
    /// **A queue rather than a slot**, which is the one way it differs from the
    /// handlers above it: a hook does not replace the hook before it, so
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
    /// per request, O(registrations), which is the spend `rule:observability/script-on-exit` states
    /// and `rule:programs/memory-priority` asks
    /// for.
    exit_hooks: Vec<Value>,
    /// Whether [`Self::run_exit_hooks`] has already run — `rule:observability/three-endings-fire-the-exit-queue`'s "the
    /// queue runs once, at most once per script".
    ///
    /// A flag rather than "the vec is empty": a hook that registers a hook
    /// empties neither, and an ending that ran an empty queue has still had its
    /// one drain.
    exit_hooks_drained: bool,
    /// The ABI status the isolate's program answered with, where it recorded
    /// one — `None` for a program that ran to its end, and for one written in
    /// Rust, which records nothing.
    ///
    /// [`crate::script::Program`] answers a [`crate::Value`] and no status, so
    /// an ending that leaves nothing else behind is invisible to the classifier
    /// on the other side of the boundary: a [`crate::THROWN`] leaves
    /// [`Self::pending`], but a [`crate::EXITED`] leaves only
    /// [`Self::exit_code`], and `0` is what that reads both for `exit(0)` and
    /// for a script that never called `exit`. Recording the status is what lets
    /// one classifier tell every ending apart, which is what
    /// `rule:observability/three-endings-fire-the-exit-queue`'s table is read
    /// against. Four bytes per request, in the word
    /// [`Self::exit_hooks_drained`] is already holding.
    ending: Option<i32>,
    /// How the exit queue is drained when the program that registered a hook
    /// ends — `rule:observability/script-on-exit`'s routing half, travelling
    /// **on the context** rather than being reached for at the end.
    ///
    /// It travels for the reason `Session::write_back` does: the report each
    /// hook is handed is a `Core` instance, so only `nvs-stdlib` can build one,
    /// while the two places a program ends are `nvs-host`'s isolate teardown
    /// and `nvs run`'s root task. `nvs-stdlib` already depends on `nvs-host`,
    /// so that edge cannot run the other way; this crate is the one all three
    /// rest on, so the seam is inverted through it and `Core\Script::onExit` —
    /// the one member that registers anything — fills the pointer.
    ///
    /// `None` until a hook is registered, so a script that registers none costs
    /// one word and no call.
    exit_drain: Option<ExitDrain>,
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
    /// `rule:concurrency/after-response-outlives-the-connection`
    /// 's after-response work, in registration order — `None` once the
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
    /// Whether this tree is one of the ones this core is counting against
    /// § 7's `max_concurrent`.
    ///
    /// A flag rather than a reading of the queue above, because the two say
    /// different things at the one moment that matters: the drain takes the
    /// queue at its start and the slot is not given back until its end, so a
    /// tree running its last registration has an empty queue and is still very
    /// much being held. [`mod@crate::deferred`] owns what the count means.
    ///
    /// **What it spends:** one word per request.
    holds_deferred_slot: bool,
    /// What is behind a pending `THROWN` or `FATAL` status — see [`Pending`]
    /// for why one field carries both shapes rather than two sitting beside
    /// each other.
    pending: Option<Pending>,
    /// Whether the innermost frame of [`Self::pending`]'s backtrace is the one
    /// the raise rendered from its own site, rather than one a compiled frame
    /// pushed — [`Ctx::raise_sited`] sets it and [`Ctx::push_frame`] spends it,
    /// and between them they are why a frame the throw unwinds out of is named
    /// once rather than twice.
    ///
    /// **What it spends:** one word per request.
    site_frame_pending: bool,
    /// Where `echo` writes.
    output: OutputSink,
    /// Where a **diagnostic** writes — `rule:errors/debug-dump`'s destination for a CLI `Core\Debug::dump`, and later for the log
    /// target's own records.
    ///
    /// A second sink rather than another [`OutputSink`] variant, because the
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
    /// Which of `rule:errors/renderings`'s two renderings the target emits — resolved on
    /// the same first use as the two above, for the same reason, and read at
    /// every record [`Self::write_log_record`] does not drop.
    ///
    /// **What it spends:** one discriminant per context, and one `String` per
    /// written record, which is the allocation the caller made when it rendered
    /// for itself.
    log_format: LogFormat,
    /// `rule:routing/an-absolute-link-takes-a-configured-origin`
    /// 's configured origin: the scheme and authority
    /// `Core\Router::urlAbsolute` puts in front of a link, with no trailing
    /// `/`.
    ///
    /// **Configured, never sniffed.** § 6 refuses `Host` and
    /// `X-Forwarded-Host` outright, which is why this is written *before* the
    /// request runs and nothing during it can move it — `nvs run` reads
    /// `nvs.toml`'s `[[app]] origin` today, and the mount that accepted the
    /// request will write it once there is a server, since
    /// `rule:http-server/a-mount-table-expands-at-boot`
    /// makes a mount's own origin win over the application's. `None` is a
    /// unit that resolves neither, and the link helper throws rather than
    /// answering with an empty authority in it.
    ///
    /// **What it spends:** two words per request, plus the origin's own bytes
    /// once — tens of them, not thousands — and nothing at all for a program
    /// that configures none.
    origin: Option<Box<str>>,
    /// `rule:config/the-config-is-an-immutable-snapshot`
    /// 's configuration, as this request sees it: the snapshot it cloned at
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
    /// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 3's shipped defaults and not an error.
    ///
    /// **What it spends:** one `Arc` clone per request, plus a `String` pair
    /// per key that request actually set. O(in-flight requests), per
    /// `rule:programs/memory-priority` — the tree
    /// itself is shared and is charged to the snapshot, not to the request.
    config: Option<nvs_config::Request>,
    /// The capabilities a spawn site's `grants:` left this context able to ask
    /// for, and `None` for one no spawn narrowed —
    /// `rule:security/isolate-shares-nothing`.
    ///
    /// A whitelist laid **over** [`Self::config`] rather than an edit of it, and
    /// that is the direction the option is defined in: a grant is a list and not
    /// a quantity, so narrowing it is an intersection with what the parent
    /// already held, and a name the parent lacks stays lacking because the
    /// overlay below is still asked. It is inherited by everything this context
    /// spawns ([`Ctx::isolate`]), so a second `grants:` deeper in the tree can
    /// only shorten the list again.
    ///
    /// An [`Arc`](std::sync::Arc) because it crosses to the core a placed child
    /// runs on ([`crate::ctx::PlacedIsolate`]) and is read on every capability
    /// question either side. **What it spends**
    /// (`rule:programs/memory-priority`): one allocation per spawn that wrote
    /// `grants:`, shared by every descendant of that child.
    grant_filter: Option<std::sync::Arc<[nvs_config::capability::Cap]>>,
    /// This request's place in a distributed trace —
    /// `rule:observability/a-trace-id-exists-for-every-request`, whose id
    /// is Novis's only request identifier.
    ///
    /// **Not [`Self::trace`]**, which is `rule:testing/debug-probes`'s per-call-site event list;
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
    /// `rule:tooling/commands-are-compiled`'s
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
    /// `rule:routing/matched-once-before-the-handler`
    /// 's route table, as the compiler built it — what the door matched this
    /// request against, and what `Core\Router`'s own members ask a second
    /// question of.
    ///
    /// The table, and never this request's *match*: that is a fact about the
    /// request and lives on [`Inbound`], because § 1's rule is that it is taken
    /// once before any application code and travels from there.
    ///
    /// Written before the program runs and never rewritten, on
    /// [`Self::commands`]' argument exactly — and `None` is a program that
    /// declared no `#[Route]`, which is `rule:routing/table-is-opt-in`'s opt-in rule as a member
    /// sees it.
    ///
    /// **What it spends:** one `Arc` clone per request; the rows themselves are
    /// shared and charged to whoever compiled them.
    routes: Option<std::sync::Arc<crate::routes::Routes>>,
    /// The process argument vector past the program itself — what
    /// `rule:tooling/commands-are-compiled`'s
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
    /// `rule:programs/memory-priority`.
    arguments: Vec<String>,
    /// The name the shell knows this program by — what
    /// `rule:tooling/commands-are-compiled`'s
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
    /// `rule:programs/memory-priority`.
    program_name: String,
    /// The identity of the whole program this context runs — what
    /// `rule:programs/no-runtime-autoload`'s
    /// `Core\Program::id()` answers, as 64 lowercase hex characters, or empty
    /// for a context no host wrote one onto.
    ///
    /// **Written before the program runs and never during it**, like every
    /// other value in `ctx/wiring.rs`, and this one could not be written any
    /// later: it is `BLAKE3(each unit's content hash in program order ‖
    /// env_hash)` — `nvs_config::cache::program_id` owns the formula — and only
    /// a host still holding the resolved graph can compute it. The member reads
    /// this slot and hashes nothing, so a first call cannot be the request that
    /// pays for every unit digest, and two reads in one run cannot disagree.
    ///
    /// **What it spends:** 64 bytes per context that was handed one, and one
    /// empty `String` — no allocation — for every context that was not.
    /// O(in-flight requests), per
    /// `rule:programs/memory-priority`.
    program_id: String,
    /// `rule:testing/determinism-declared-on-the-test`'s fixed clock: the wall-clock reading `Core\Time::now` answers
    /// with, in nanoseconds since the Unix epoch, or `None` for a context that
    /// reads the host's clock.
    ///
    /// **Isolate configuration, written before the program runs**, exactly as
    /// [`Self::origin`] is — the test runner reads `#[Test(at: …)]` and writes
    /// it onto the isolate's own context, and a context nobody wrote it onto
    /// is every context outside a test. That is why this does not reopen
    /// `rule:statements/static-is-a-member-modifier`'s "nothing holds
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
    /// `rule:testing/determinism-declared-on-the-test`'s seeded generator, as its **live state** rather than as the seed
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
    /// `nvs_stdlib::random`'s module docs describe. `rule:testing/determinism-declared-on-the-test` puts
    /// `Core\Random\Seeded` behind a separate *type* in production for exactly
    /// this reason, and this field does not reopen that — it adds no spelling a
    /// program outside a test can write.
    ///
    /// **What it spends:** two words per request, and one predictable
    /// not-taken branch per draw.
    random_state: Option<u64>,
    /// `rule:testing/in-process-request`'s second mechanism, as the one thing a test can observe of it: the
    /// base URL of the ephemeral listener the runner bound for a
    /// `#[Test(server: true)]` case, or `None` for every other context there
    /// has ever been.
    ///
    /// **Beside [`Self::fixed_clock`] for that field's own reason** — a test
    /// declares the world its subject runs in and the isolate is what scopes
    /// the declaration (§ 2), so the address lives on the child's context and
    /// dies with it. A `thread_local` would outlive the listener it names and
    /// answer the next test with a port nothing is bound to.
    ///
    /// A `String` and not a `SocketAddr`: what a program does with it is write
    /// it into a URL, this crate has no URL type, and the scheme is a fact the
    /// runner knows and the address does not carry (§ 18's listener is `http`,
    /// `rule:http-server/two-deployments-and-nothing-a-proxy-owns`
    /// having dropped the TLS listener). `nvs_stdlib::test`'s `serverUrl` is
    /// the only reader and `nvs_cli::runner` the only writer.
    ///
    /// **What it spends:** three words per request, and the URL itself only in
    /// the tests that asked for a listener.
    test_server: Option<String>,
    /// `rule:tooling/a-prompt-is-a-core-member`'s
    /// scripted answer queue: what the next `Core\Cli` prompts read instead of
    /// a terminal, oldest first, and empty for every context outside a test.
    ///
    /// **Beside [`Self::fixed_clock`] because it is the same idea** — a test
    /// declares the world its subject runs in, and the isolate is what scopes
    /// the declaration (`rule:testing/isolate-per-test` gives each test its own context). A queue held in a `thread_local`
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
    /// `rule:testing/an-outbound-call-is-answered-from-a-table`'s table: the
    /// answers a test registered for outbound calls, and the calls it has made
    /// since — empty for every context outside a test.
    ///
    /// **Beside [`Self::scripted_answers`] because it is the same idea one
    /// door over**: a test declares the world its subject runs in, and the
    /// isolate is what scopes the declaration. The only writer of the answers
    /// half is `Core\Test::answerHttp`, so a request or a `nvs run` never has
    /// one — and the switch it arms only ever *removes* a program's reach,
    /// since a call answered from here opens no socket at all.
    ///
    /// **What it spends:** four words per request for the two empty vectors,
    /// then a row per registered answer and a record per call taken, both
    /// charged to the test that wrote them and released with its isolate.
    /// [`AnswerTable`] is the home of the rest.
    faked_http: AnswerTable,
    /// `Core\Out::capture`'s buffers, innermost last —
    /// `rule:security/capture-answers-the-carrier`
    /// .
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
    /// giving [`OutputSink`] another variant that every other writer would
    /// have to match on.
    captures: Vec<Vec<u8>>,
    /// The media type this request's output has been *declared* to be —
    /// `rule:security/response-body-is-one-typed-member`
    /// 's body members, each of which owns one body shape and sets its
    /// own `Content-Type`. `None` for a request that only echoed, which § 4's
    /// last bullet reads as `text/html`.
    ///
    /// **A declaration and not the bytes.** The bytes are
    /// [`Self::write_output`]'s, because § 3's table already binds a request's
    /// `echo` to the response body and a second buffer beside it would be two
    /// answers to the question of what the body is. What a body member adds
    /// over an `echo` is this one string, so this one string is what it
    /// records.
    ///
    /// Nothing here adjudicates a disagreement between two declarations: § 4's
    /// sixth row makes mixing writers a **compile** error, so the last one
    /// wins by construction rather than by a rule this field enforces.
    ///
    /// **What it spends:** one word per request, and one short allocation per
    /// request that declares — never per write.
    content_type: Option<Box<str>>,
    /// What this request's `Core\Debug::dump` calls rendered for its body, held
    /// until the body is finished — `rule:errors/debug-dump`'s `[debug] inline`
    /// row, and empty on every context that dumped nothing.
    ///
    /// Collected rather than written where the dump was made, because both
    /// questions a block's placement turns on are answered only at the end.
    /// Whether the body is HTML at all is [`Self::content_type`], which the body
    /// member that declares it may not have run yet; and markup a block is
    /// appended *after* cannot be markup the block landed in the middle of.
    /// [`Self::flush_inline_debug`] is the one reader.
    ///
    /// **What it spends:** the rendered blocks of one request's dumps, released
    /// with the request. Nothing at all outside a mode whose `[debug] inline`
    /// ceiling is open, which a host that wrote no configuration does not have.
    inline_debug: Vec<u8>,
    /// The file this response's body **is** — `Core\Response::sendFile`, the one
    /// body member whose bytes never pass through this context.
    ///
    /// Beside [`Self::content_type`] because it is the same kind of fact and is
    /// taken on the same path: a name the finish path lifts onto a
    /// [`crate::host::Completion`] for whoever is answering. What it holds is a
    /// path and never the bytes at it, so a response carrying a file of any size
    /// costs this context one name — the read belongs to the server, which
    /// streams it under the static policy and answers a range request over it.
    ///
    /// `None` on every context that never called that member, which is every
    /// request answering with what it wrote.
    ///
    /// **What it spends:** one word per request, and one short allocation per
    /// request that sends a file.
    file_body: Option<Box<std::path::Path>>,
    /// The writing half of a response body being written **over time**, for the
    /// request that opened one — `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
    /// streaming response, whose other half the connection is draining.
    ///
    /// Beside [`Self::content_type`] because it answers the same question the
    /// field above does and the buffer below does: where this response's bytes
    /// go. A request that opened a stream writes through this; every other
    /// request writes through [`Self::write_output`], and the member that writes
    /// a chunk reads this field to tell which it is.
    ///
    /// `None` on every context that never opened one, which is every request
    /// that answered whole. Dropped with this context, and dropping it is what
    /// ends the body — `crate::stream::Emit`'s own `Drop` owns why an isolate
    /// that ended can never leave a response open.
    ///
    /// **What it spends:** one pointer per request, and — only for a request
    /// that streams — the one chunk in flight, which is the writer's own
    /// allocation on its way to the wire rather than a copy of it.
    body_stream: Option<crate::stream::Emit>,
    /// Which door opened the **event stream** this context writes, and `None`
    /// for a program that opened neither — the one thing `Core\Sse::current()`
    /// separates from every other program.
    ///
    /// Beside [`Self::body_stream`] because it says what that half *is* where
    /// the field above says where the bytes go, and neither that field nor the
    /// peer can answer it: a request streaming an ordinary body holds an
    /// [`crate::stream::Emit`] too, and an event stream that outlives its
    /// request was handed no socket, `rule:concurrency/two-doors-one-isolate`'s
    /// hand-over taking nothing. That rule makes both doors answer one handle,
    /// and the door itself is held here because the two are not the same
    /// program: one ends with its response and the other is a connection, so
    /// `Core\Topic` admits the second beside a peer and refuses the first.
    ///
    /// Written by whoever opened the stream and never cleared: a stream that has
    /// ended is still the only thing this context was.
    ///
    /// **What it spends:** one byte per request, and never an allocation.
    event_stream: Option<EventStreamDoor>,
    /// What this request's response says it *is* — spec § 15's `setStatus`,
    /// or `None` where nothing set one and the answer is whatever the server's
    /// own default is.
    ///
    /// Beside [`Self::content_type`] and not folded into it: a status and a
    /// media type are declared by different members — every body member sets
    /// the second and only `setStatus` sets the first — so one field holding
    /// both would make a body member that did not mean to touch the status
    /// able to.
    ///
    /// The last declaration wins here too, and for a weaker reason than
    /// `content_type`'s: § 4's sixth row does not reach `setStatus` at all,
    /// since setting a status twice writes no body twice. Two calls are a
    /// program saying two things about one response, and the later one is the
    /// one it meant.
    ///
    /// **What it spends:** one half-word per request, and never an allocation.
    status: Option<u16>,
    /// The headers this request's response carries beyond the ones whoever is
    /// answering wrote for itself — spec § 15's `setHeader` and `addCookie`, in
    /// the order they were first declared.
    ///
    /// A **list** where the two fields above are words, because a header is a
    /// map rather than a property of the response:
    /// `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`
    /// makes `setHeader` an override of *one* policy-owned header on one
    /// response, so what has to cross is every pair a program set rather than
    /// one of them.
    ///
    /// A `Vec` and not a map: a response carries a handful of these, the order
    /// a program set them in is the order the peer sees them in, and a hash
    /// over three entries costs more than the scan that replaces one — and a
    /// map keyed by name could not hold a second `Set-Cookie` at all, which is
    /// why a name declared twice stays two rows here and each row says for
    /// itself how it joins. [`Self::declare_header`] owns the comparison and
    /// [`DeclaredHeader::append`] the distinction.
    ///
    /// **What it spends:** nothing for a request that sets none — an empty
    /// `Vec` does not allocate — and two short allocations per declared header
    /// for one that does, charged to that request's own budget like every
    /// other allocation it makes.
    headers: Vec<DeclaredHeader>,
    /// The request this context is answering, as it arrived — spec § 15's
    /// `Core\Request`, and `None` in every process that is not serving one.
    ///
    /// The fields above are the response half of the same channel and
    /// this is the inbound half, which is why it sits here rather than beside
    /// the configuration: what is on it is per *request*, written once before
    /// the program runs and never again.
    ///
    /// **`None` is an answer, not a missing value.** A CLI program, a scheduled
    /// script and a test all run with no request, and
    /// `rule:security/request-state-throws-in-an-isolate` makes reading
    /// `Core\Request` there a **throw** rather than an empty string — "there is
    /// no request here" and "the request sent nothing" are different facts, and
    /// an `Option` is what keeps them different this far down.
    ///
    /// **Boxed, and that is load-bearing rather than tidy.** A whole `Ctx` is
    /// handed back from a coroutine inside `nvs_host::scheduler::Finished`, and
    /// `corosensei` refuses to transfer anything over **1024 bytes** across a
    /// stack switch — a limit this struct is already close to, and one whose
    /// breach reads as `type is too big to transfer` from an unrelated test
    /// rather than as anything about this line.
    /// `nvs_host::scheduler`'s `a_finished_task_fits_the_stack_switch` is the
    /// guard, and it is why a per-request aggregate belongs behind one pointer
    /// here: everything else `Core\Request` still owes — the headers, the
    /// cookies, the mount captures — grows [`Inbound`] and not this struct.
    ///
    /// **What it spends:** one word per request that has none, and one
    /// allocation holding an [`Inbound`] — itself a few short allocations — for
    /// one that does.
    inbound: Option<Box<Inbound>>,
    /// `rule:testing/debug-probes`'s statement-boundary hit counters, indexed by `nvs_ir::StmtId`.
    ///
    /// Written only from [`nvs_probe_stmt`], which compiled code reaches only
    /// when the [`DebugFlags`] word above is non-zero — so a request with no
    /// probe enabled never touches this vector and never allocates it.
    ///
    /// **A stand-in, not the final shape.** `nvs_ir::StmtId` numbers from zero
    /// within *each* function, so two functions' statements collide in this
    /// one table. `rule:testing/debug-probes` wants path → line → count, which needs the unit and
    /// function a statement belongs to; that qualification arrives with
    /// `Core\Debug` and the Clover/lcov exporters in M10. What this table is
    /// for now is proving the mechanism: the probe fires at exactly the
    /// statements a request executed, and nowhere else.
    stmt_hits: Vec<u64>,
    /// `rule:testing/debug-probes`'s call-site trace, in the order the probes fired.
    ///
    /// Written only from [`nvs_probe_call_enter`]/[`nvs_probe_call_exit`],
    /// under the same "the flags word was non-zero" gate `stmt_hits` is under.
    ///
    /// **A stand-in, not the final shape**, for the same reason `stmt_hits`
    /// is one, plus a second: `rule:testing/debug-probes` has trace and profile data *stream to
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
    /// The `Core` classes, as the resolver an embedder installs at boot — see
    /// [`Ctx::set_core_classes`], and [`Ctx::class_desc`] for why a name is
    /// asked of the program's own table first and of this second.
    ///
    /// **What it spends:** one word per context, and nothing per request
    /// beyond it: the descriptors are `nvs_stdlib::instance`'s process-wide
    /// table, shared by every core rather than built per boot.
    core_classes: Option<CoreClasses>,
    /// The coroutine yielder of the task this request is running inside, or
    /// null on the main stack — `nvs-host`'s scheduler publishes it on entry
    /// and clears it before the context leaves the coroutine.
    ///
    /// **This is what removes async colouring.** A helper that has to wait
    /// reaches the yielder through the context it was already handed rather
    /// than through its own signature, so no caller up the chain is marked
    /// `async` and Novis needs no such marker at all
    /// (`rule:http-server/a-core-is-never-blocked-on-a-syscall`
    /// , `docs/plan/design.md` § *Thread-per-core, shared-nothing runtime*).
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
    /// The **recipes** the slots above were materialized from — the compiled
    /// unit whose code this context is running, as the one thing a child of it
    /// needs that its own class table does not already carry.
    ///
    /// `rule:security/isolate-shares-nothing`'s method entry
    /// is what reads it: a `Class::method` isolate runs code out of the
    /// *parent's* unit, so it has to arm a fresh store against that unit's slot
    /// numbering and there is no path for a resolver to compile. Shared rather
    /// than copied — one atomic increment per isolate, against a list whose
    /// length is the unit's static-property count — and `None` for a context
    /// nobody armed through [`Ctx::install_statics`], which is every hand-built
    /// fixture. `crate::script`'s module doc owns why this is the whole handle.
    unit_statics: Option<std::sync::Arc<[Option<FieldDefault>]>>,
    /// The value that crossed into this isolate, owned — see
    /// [`Ctx::set_isolate_argument`].
    ///
    /// Cold, and null for every context that is not an isolate's, which is
    /// every request. It is a field here rather than a slot the program keeps
    /// because it is the isolate's ownership *root*
    /// (`rule:security/isolate-teardown-is-a-drain-then-a-sweep`
    /// ): what releases it is dropping this context, and nothing else knows
    /// when that happens.
    isolate_argument: Value,
    /// `rule:testing/failure-ledger`'s per-test assertion ledger, in the order the assertions ran.
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
    /// § 14's inline snapshots that did not hold, in the order they ran — the
    /// material `nvs test --update` splices from, and empty for every run that
    /// asserted no snapshot or whose snapshots all held.
    ///
    /// Beside the ledger rather than inside it: an entry here is not a verdict
    /// and nothing reads it to decide one. What it spends is one pair of
    /// strings per *failed* snapshot, so an ordinary green run allocates
    /// nothing at all for it ([`SnapshotMismatch`] owns the rest).
    snapshot_mismatches: Vec<SnapshotMismatch>,
    /// The isolates this request has started and not yet awaited, by the key
    /// its `Core\Script\Handle` carries — see [`Ctx::hold_started_script`].
    started_scripts: Vec<Option<Box<dyn crate::host::Running>>>,
    /// The files this request has opened and not yet closed, by the key its
    /// `Core\IO\File` carries — see [`Ctx::hold_open_file`].
    open_files: Vec<Option<std::fs::File>>,
    /// The children this task has spawned, by the key its `Core\Process\Handle`
    /// carries — see [`Ctx::hold_spawned_child`], and [`HeldChild`] for the
    /// kill that makes none of them outlive this context.
    spawned_children: Vec<Option<HeldChild>>,
    /// The sockets this request has opened and not yet closed, by the key its
    /// `Core\Net\Stream` or `Core\Net\Listener` carries — see
    /// [`Ctx::hold_open_socket`].
    open_sockets: Vec<Option<Box<dyn HeldSocket>>>,
    /// The streamed reply bodies this request is still reading, by the key its
    /// `Core\Http\Stream` — and then the one walk that took the body — carries;
    /// see [`Ctx::hold_open_reader`].
    open_readers: Vec<Option<Box<dyn HeldReader>>>,
    /// The database connections this request has opened, each with the
    /// memoization key it was reached by and the pool lease it goes home on —
    /// see [`Ctx::hold_open_connection`].
    open_connections: Vec<OpenConnection>,
    /// The temporary directories `Core\IO::temporaryDir` has handed this script,
    /// in the order it handed them out — see [`Ctx::track_temporary_dir`], and
    /// `rule:core-classes/temporary-dir-sweep` for the sweep that reads it.
    ///
    /// Unlike its neighbours this is a list and not a table: a directory
    /// is a path rather than a handle, nothing in Novis holds a key to one, and
    /// a program removing its own directory is § 3's goal state reached early
    /// rather than a slot to empty. So an entry is never taken back out
    /// individually and the sweep drains the whole list once.
    temporary_dirs: Vec<std::path::PathBuf>,
    /// The session `Core\Session::start` opened, or `None` for a request that
    /// started none — see [`Session`].
    ///
    /// `rule:core-api/session-roster` makes every other member of that class throw while this is `None`,
    /// which is the whole of what
    /// `rule:core-classes/session-is-started-explicitly` buys:
    /// "this request uses sessions" is a line in the source, and it is worth
    /// nothing if the first `get` can silently start one.
    session: Option<Session>,
    /// `rule:concurrency/a-connection-is-a-root-isolate`'s
    /// socket, on the connection isolate the upgrade opened and on nothing else
    /// — [`crate::peer`] is the home of the seam and of why it is a trait
    /// object.
    ///
    /// `None` for every other kind of context, which is what makes
    /// `Core\Socket::current()` a refusal outside a connection rather than a
    /// rule to remember. It is dropped with this context, and dropping it is
    /// what closes the descriptor.
    peer: Option<Box<dyn crate::peer::PeerSocket>>,
    /// `rule:concurrency/a-connection-is-a-loop`'s
    /// **second source**: values published to topics this connection
    /// subscribed to, in arrival order, waiting for the next `receive()`.
    ///
    /// `None` on every context that is not a connection's, and on most that
    /// are — see [`Ctx::deliver`], which owns why the queue is here beside
    /// [`Self::peer`] rather than anywhere else. Each entry holds one owned
    /// reference, given back in [`Drop`] for whatever is still queued when the
    /// connection ends.
    ///
    /// **An [`Option`] because the allocation is on the request path.** The
    /// queue is a separate allocation because § 4's subscriber table shares it
    /// ([`crate::peer::Inbox`]), and an eager one would charge every request
    /// ever served for a connection's facility; [`Ctx::inbox`] makes it the
    /// first time something asks, which is a subscribe or a delivery and
    /// therefore a connection.
    deliveries: Option<std::rc::Rc<crate::peer::Inbox>>,
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

/// The request ends here, and so does everything its static properties held.
///
/// This is the whole of what makes a static's storage request-scoped rather
/// than process-global: nothing outside the [`Ctx`] ever points at a slot, so
/// dropping the context is what returns the memory and drops the references —
/// there is no second owner to coordinate with and no table to clear.
impl Drop for Ctx {
    fn drop(&mut self) {
        // Before the teardown below, so what that allocates on this core is no
        // longer being charged to a tree whose root is on another one —
        // `Ctx::end_off_core_share` owns why the memory share is given back
        // whole and the written one is not.
        self.end_off_core_share();
        // First, because what the teardown below allocates is not part of what
        // this request held: republishing here leaves those bytes to raise the
        // enclosing context's mark, where they belong. `Ctx::memory_peak_saved`
        // is the field, and the rule it names owns why the larger of the two
        // wins.
        crate::budget::publish_peak(self.memory_peak_saved);
        // Beside it, and first for the same reason twice over: what the
        // teardown below allocates belongs to whatever context encloses this
        // one, and the word this context armed stops being a word anything may
        // publish into the moment it drops. `Ctx::memory_ceiling_saved` is the
        // field and `crate::budget::arm` the obligation.
        crate::budget::arm(self.memory_ceiling_saved);
        // And the verdict that travels with it, for the same reason in the
        // other direction: a refusal this request was stopped by is not one the
        // thread's next request inherits. `Ctx::memory_refused_saved` is the
        // field.
        crate::budget::restore_refusal(self.memory_refused_saved);
        self.release_statics();
        // An isolate's argument is one of its roots and is released with them
        // — `Ctx::set_isolate_argument` owns why it is held here at all.
        self.set_isolate_argument(Value::null());
        // `rule:errors/on-limit`'s handler is request-local, so the request ending is
        // what unregisters it — see `Ctx::set_limit_handler`.
        self.set_limit_handler(Value::null());
        // `rule:errors/on-uncaught-throw`'s handler is request-local for the same reason and is
        // unregistered the same way — see `Ctx::set_uncaught_handler`.
        self.set_uncaught_handler(Value::null());
        // And the shutdown handler, which is request-local for that reason and
        // no other: a signal is the process's, but nothing outlives the request
        // that registered what to run — see `Ctx::set_shutdown_handler`.
        self.set_shutdown_handler(Value::null());
        // `rule:observability/script-on-exit`'s exit hooks are request-local for the same reason. A
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
        // `rule:concurrency/a-connection-is-a-loop`'s undelivered topic values, for a connection that ended
        // with the queue non-empty — `crate::peer::Delivery` carries one owned
        // reference and deliberately has no `Drop` of its own, so this is where
        // the ones no `receive()` reached are given back.
        // Taking the handle is also what unsubscribes this connection from
        // every topic it joined: `Core\Topic`'s table holds a `Weak` onto this
        // queue and nothing else, so dropping the last strong reference is the
        // subscription ending — see `crate::peer::Inbox`.
        if let Some(inbox) = self.deliveries.take() {
            while let Some(delivery) = inbox.pop() {
                #[expect(
                    unsafe_code,
                    reason = "the queue holds exactly one reference per delivery and \
                              nothing else points at it"
                )]
                // SAFETY: `Ctx::deliver` was handed that reference and this is
                // the only other place it is given back.
                unsafe {
                    delivery.into_value().release();
                }
            }
        }
        // `rule:concurrency/after-response-outlives-the-connection`'s deferred work is request-local for the same reason,
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
        // The other end of § 7's count, for a tree that ended without draining
        // — and a no-op for one that drained, which gave its slot back there.
        self.release_deferred_slot();
        // `rule:security/db-pool-reset-is-a-boundary`: a connection the request is still holding is released
        // to this core's pool under the lease it was filed with, rather than
        // closed here — `crate::pool` decides which of those two happens, and
        // its module doc owns why the reset is the acquiring request's job and
        // not this one's. Either way the lease is consumed, which is what gives
        // the key's `max` slot back. The clock is read once for the whole set
        // and not at all for a request that opened no connection.
        //
        // An entry a `Core\Db\Connection::close` already emptied is skipped:
        // `Ctx::close_open_connection` ran these same two lines for it then,
        // and took its lease with it.
        if !self.open_connections.is_empty() {
            let now = std::time::Instant::now();
            for held in std::mem::take(&mut self.open_connections) {
                let Some(connection) = held.connection else {
                    continue;
                };
                match held.lease {
                    Some(lease) => crate::pool::release(lease, now, connection),
                    None => drop(connection),
                }
            }
        }
        // `rule:core-classes/temporary-dir-sweep`'s sweep, and this is the only place it is called from —
        // `crate::sweep`'s module doc owns why a context's teardown *is* that
        // section's "after the last user code" for every ending at once.
        //
        // The files come first, and deliberately: a `Core\IO\File` the program
        // opened inside its own temporary directory and never closed is dropped
        // as a field a moment after this body either way, but on Windows a held
        // handle is what makes a deletion fail — so closing them here is the
        // difference between a program that forgot to close being swept and
        // being logged about. Nothing else observes the order; a descriptor has
        // no teardown but the close.
        drop(std::mem::take(&mut self.open_files));
        // `rule:core-classes/process-spawn`'s lifetime, and it is here rather
        // than left to the field drop for the line above's reason twice over: a
        // child still running holds its working directory and whatever it has
        // open, so on Windows it is what makes the sweep's deletion fail, and
        // it is also the one resource whose release is a *kill* rather than a
        // close. [`HeldChild`]'s `Drop` is that kill.
        drop(std::mem::take(&mut self.spawned_children));
        crate::sweep::at_script_end(self);
        // A failure that ended the request still owns its exception object,
        // and `Thrown`'s own `Drop` is what releases it. Taken here rather
        // than left to the field drop below, because a field is dropped
        // *after* this body: the sweep would otherwise reach an object that is
        // about to be released a second time.
        drop(self.pending.take());
        // `Core\Request::json`'s decoded document, taken here for that same
        // reason: the carrier holding it is a field too, so leaving the
        // reference to `HeldValue`'s own `Drop` would give it back after the
        // sweep below had already reached what it points at.
        drop(
            self.inbound
                .as_deref_mut()
                .and_then(Inbound::take_decoded_body),
        );
        // Last, and only once every root above is gone: what is still on the
        // live list is then exactly the cyclic garbage the refcounts could not
        // free. `rule:security/isolate-teardown-is-a-drain-then-a-sweep` is the decision and `crate::object::sweep` the
        // mechanism.
        crate::object::sweep(&self.live);
    }
}

/// Byte offset of the handle to the safepoint word within [`Ctx`] — see the
/// module docs, which own why the word itself is the request tree's and this
/// line holds only the way to it.
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

/// `rule:errors/on-limit`'s
/// call-stack ceiling: **8 MiB of reserved address space per request**, of
/// which only the touched pages are ever resident.
///
/// About 65,000 frames — the same order as what PHP permits, and the default
/// Linux thread stack. Stated as
/// `rule:programs/memory-priority` requires: what
/// the number buys is how deep a program may recurse and how much one runaway
/// commits before it is stopped, and at `benches/abi-probe`'s per-call cost
/// that depth is microseconds of work either way.
pub const STACK_CEILING: usize = 8 << 20;

/// The slice between [`Ctx::arm_stack_limit`]'s soft address and its hard one.
///
/// It is what a [`ThrownClass::Recursion`] unwinds in, and it is also the
/// slack that lets a **leaf** function skip the check entirely: its caller
/// passed the compare with this much stack still under it, so a frame that
/// allocates no further calls cannot cross the floor.
pub const STACK_RESERVE: usize = 256 << 10;

/// Which of `rule:errors/on-limit`'s resource limits stopped the request, as the tier-1 handler is told it.
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
/// **A variant per enforced limit.** § 1 lists more than these; wall time and
/// call-stack depth each gain a variant in the slice that gives them a breach
/// to report, since a variant nothing can produce is a word in this report's
/// vocabulary that no handler could see.
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

impl Default for Ctx {
    fn default() -> Self {
        Self::stdout()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hot_words_come_first_and_are_a_word_apart() {
        assert_eq!(SAFEPOINT_OFFSET, 0);
        assert_eq!(DEBUG_FLAGS_OFFSET, 8);
        assert_eq!(DEADLINE_OFFSET, 16);
        assert_eq!(STACK_LIMIT_OFFSET, 24);
        assert_eq!(STATICS_OFFSET, 40);
        // The offsets above are the arrangement; this is the property `rule:http-server/time-is-bounded-inside-a-helper`
        // actually buys with it, and it is what fails first when a field
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
}
