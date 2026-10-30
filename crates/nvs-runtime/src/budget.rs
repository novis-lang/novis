//! The per-request resource budget: a live-byte counter and an output-byte
//! counter the runtime keeps in every build, and the `[limits] memory` and
//! `[limits] max_output` ceilings a request is measured against.
//!
//! `rule:errors/on-limit` names
//! memory among the resource limits whose breach is a `FATAL`,
//! and [`crate::affordable`]'s own doc comment says why the count lives
//! here rather than at the members that allocate: a guard written per call site
//! is a guard the next call site forgets, which is the shape of PHP's own
//! history — its `memory_limit` is enforced in the allocator precisely because
//! per-function checks did not hold. So the counter sits in the one place every
//! allocation passes through, and [`Ctx`](crate::Ctx) holds the request's share
//! of it.
//!
//! # Why this is not `counting_alloc`
//!
//! That module counts too, and it is `#[cfg(test)]`: it exists for the leak
//! guards, its counters are read by assertions and by nothing else, and a build
//! that ships never has it. This one is the opposite in every one of those —
//! it is compiled into every build, it is read by the request path, and a tree
//! without it has no enforceable cap. They share an arithmetic and nothing
//! else, so `Counting` feeds `add` rather than keeping a count of its own: a
//! test build registers `Counting` as its global allocator, so without that
//! call the very builds the unit tests run in would be the only ones not
//! counting.
//!
//! # Per *thread*, read as per *request*
//!
//! [`live_bytes`] is a thread-local balance, for the reason `counting_alloc`'s
//! own header gives: a process-wide counter would fold every other worker's
//! allocations into the answer, and every allocation a request makes happens on
//! the thread running it. A request is charged the *difference* between that
//! balance now and the balance when its [`Ctx`](crate::Ctx) was made —
//! [`Ctx::memory_used`](crate::Ctx::memory_used).
//!
//! # Per request *tree* across switches
//!
//! A core interleaves many requests: each one parks at an await and at every
//! time slice, and its neighbours allocate and free while it is parked. So the
//! balance a task reads is its request *tree's*, not the thread's: every turn
//! is charged to the tree that ran it. [`TreeState::balance`](crate::TreeState)
//! is that tree's balance as its last turn on its root core left it, and
//! [`FOREIGN`] is how far the thread's balance stands from it, which
//! [`live_bytes`] subtracts. A park stores the running tree's balance, and
//! every resume, a task's first turn included, sets [`FOREIGN`] from the
//! balance of the tree it resumes. That is O(1) per switch and allocates
//! nothing. A child's context is dropped as one more turn of its tree
//! ([`Ctx::end_in_its_turn`](crate::Ctx::end_in_its_turn)), and a connection
//! that starts the next request's tree hands its running turn to it
//! ([`Ctx::reroot`](crate::Ctx::reroot)), so a byte is given back on the tree
//! it was charged to. A spawn that hands a new tree its context moves the
//! context's own allocations and the task's body off the spawning tree
//! ([`Ctx::hand_over_its_making`](crate::Ctx::hand_over_its_making)), because
//! the new tree frees them. Otherwise an accept loop is charged once for every
//! connection it serves.
//!
//! **The unit is the tree and never one task's stack**
//! (`rule:security/isolate-budget-is-the-trees`). A `Core\Task` child charges
//! its stack to its parent and files results the parent then holds, so the
//! tasks of one tree read one balance, and children that each stay under the
//! tree's ceiling still cross it together. Only turns of *other* trees move a
//! parked tree's offset, so two requests that take turns on one core are each
//! charged their own bytes, and only the one past its own limit stops
//! (`rule:security/isolate-shares-nothing`, `rule:errors/on-limit`). The
//! ceiling, the mark and the refusal's figure are read in the same units, and
//! the allocation path is unchanged: [`CEILING`] is stored against the
//! thread's [`LIVE`], and moves with the offset at each resume.
//!
//! A task is in its tree's units from its first turn to its end, whatever
//! context it makes inline. A context running off its tree's root core has no
//! tree balance to share there, and its stack keeps the offset it had, which
//! is the thread's balance as before. A block one request allocates and
//! another frees is still charged to the one that frees it, as a block that
//! crosses threads is below. The bytes a request hands to a neighbour are rare
//! and bounded by what it was allowed to hold.
//!
//! A block allocated on one thread and freed on another makes the freeing
//! thread's balance go negative, which is why the counter is signed: it stays a
//! readable number rather than a huge one, and a request's *used* figure
//! saturates at zero rather than wrapping. A negative balance is also a credit
//! the freeing request could spend past its ceiling, so a block that crosses
//! threads on purpose — the answer of a job on the blocking pool — is moved
//! from one balance to the other with [`carry`].
//!
//! [`written_bytes`] is that same arrangement for `[limits] max_output`, and it
//! is here rather than as a field on [`Ctx`](crate::Ctx) for the one property a
//! per-context field could not have. An isolate writes on the thread that
//! spawned it, so a thread-local count puts a child's bytes on the root's
//! reading as well as on the child's own — which is what
//! `rule:security/isolate-shares-nothing` already says
//! the directive means, and what makes the tree's output budget a budget for
//! the tree rather than one per isolate. Monotonic where [`live_bytes`] is a
//! balance, because bytes written to a response are never given back, so it
//! needs no sign.
//!
//! # The part a thread cannot hold
//!
//! A request tree is one core's until a child is placed on another one
//! (`rule:concurrency/on-worker-runs-the-child-on-another-core`), and from that
//! moment the counters above hold only part of the answer: the child's bytes
//! are on the far core's balance, where the root's reading cannot see them and
//! the root's ceiling cannot stop them. [`OffCore`] is the rest — an atomic
//! pair, one per request tree, that a context running off the root's core
//! charges its own share into and every context on the root's core adds to its
//! reading.
//!
//! **It is the tree's and not the placing context's.** A pair made by whichever
//! context happened to place the child would be one that context's own
//! ancestors never learn about, and the ceiling that stops a tree is the root's
//! (`rule:security/isolate-budget-is-the-trees`) — the root has no registry of
//! live descendants to walk, which is the same argument that puts the safepoint
//! word in one allocation per tree. So the pair sits in that allocation, and a
//! tree that places nothing off its core pays two words nothing ever writes and
//! one relaxed load per reading.
//!
//! A member off the root's core publishes **at its own polls**, not at its
//! allocations, so the root's reading of it is as fresh as that member's last
//! poll. Nothing enforced rests on that freshness: what bounds such a child is
//! the sub-cap it is handed where it is placed, which is what remains of the
//! tree's budget there and never more.
//!
//! # Where the breach is noticed
//!
//! Counting is universal; *noticing* is not, and the difference is worth
//! knowing before trusting the cap.
//! [`Ctx::memory_breach`](crate::Ctx::memory_breach) is asked by
//! [`crate::run_helper`], on the way in to every `Core` member, and by
//! [`crate::nvs_safepoint`], the poll compiled code makes between statements.
//! Together those bound any program that calls anything.
//!
//! **The allocator itself is what bounds the rest.** A request whose ceiling
//! is armed carries it here as an absolute balance — [`armed_ceiling`] — and
//! [`add`] compares every *growing* allocation against it. A crossing raises
//! [`SafepointFlags::MEMORY_LIMIT`](crate::SafepointFlags) — and beside it
//! [`SafepointFlags::COLLECT`](crate::SafepointFlags), which is where the
//! in-flight cycle collector runs and the only place it does ([`publish`]) —
//! in the word that
//! request's tree polls, which is why a loop allocating only through the
//! ctx-less helpers — `nvs_str_concat`, `nvs_array_append` — stops at its own
//! next back edge rather than at whatever member it happens to call next. The
//! flag is a *request to poll*, not the verdict: what it wakes asks
//! [`Ctx::memory_breach`](crate::Ctx::memory_breach) against these counters, so
//! a crossing given back before the poll arrives is lowered there rather than
//! reported.
//!
//! **Bounding one operation rather than a loop means refusing in front of the
//! allocation**, where the size is known and the caller is ours. [`affords`] is
//! that question, and a `false` from it is the refusal itself: the bytes are
//! never asked for, so the request goes on holding *less* than its ceiling and
//! no counter here can say what happened. The verdict is therefore kept beside
//! the counters rather than derived from them, and
//! [`Ctx::over_memory_limit`](crate::Ctx::over_memory_limit) is where the poll
//! reads it. The seam that asks is the value allocators in [`crate::string`]
//! and [`crate::array`], never this module's own `GlobalAlloc`, whose null
//! reaches `handle_alloc_error` and aborts the process.
//!
//! Every allocator whose size is a **count off a call site** asks it: the value
//! allocators named above, and [`crate::affordable`], which is the one seam
//! every count-shaped `Core` argument is checked at. So a request cannot ask
//! for its whole ceiling twice over in a single operation and hold the bytes
//! until its next poll — the ask is refused in front of the allocation, where
//! [`add`]'s compare could only have caught the *second* such operation.
//!
//! # Whose bytes they are
//!
//! A per-*thread* balance reads as a per-*request* one only while every byte on
//! the thread belongs to the request running there, and a cross-request store
//! is where that stops being true: a `Core\Cache` entry outlives the request
//! that wrote it and is freed by whichever request evicts it. Left on
//! [`live_bytes`], that is a ceiling a program widens at will — fill the store
//! in one request, free it in the next, and the second request's balance falls
//! below the baseline its ceiling was armed against, buying headroom the
//! operator never granted.
//!
//! [`Detached`] is the bracket that answers it. An allocation or a release made
//! while one is held moves [`detached_bytes`] instead, so the bytes stay
//! counted against the process and leave the running request's reading exactly
//! where they found it. What bounds them is the store's own limit, because they
//! are not the request's to cap.
//!
//! **What a bracket owes is symmetry**, and that is the one part the shape
//! cannot enforce: a block allocated inside one and freed outside it lowers a
//! balance its allocation never raised, which is this same hole pointing the
//! other way. So the bracket belongs to the store, around every path that
//! allocates or frees what it holds, rather than to a call site that happens to
//! be storing. Charging a free to whoever allocated the block instead needs
//! per-request provenance, which is what a request arena
//! gives; until then, a store that is not bracketed is one whose frees are the
//! freeing request's.
//!
//! # What it spends
//!
//! Per `rule:programs/memory-priority`'s *say what
//! you spend*: the counter cells below, one set per thread — never per request,
//! and never growing with requests served — plus one `isize` on each
//! [`Ctx`](crate::Ctx) for the mark it displaced, which
//! `rule:observability/a-memory-peak-is-recorded-not-asked-for` states. On the
//! allocation path it is a thread-local read-modify-write of the live balance
//! per `dealloc`, and of each memory counter per `alloc` — the high-water mark
//! a compare that stores only when it moves. Each is a register-relative load, an add
//! and a store against a `const`-initialized cell. Together they are a part of
//! a pooled round trip and a small fraction of a platform heap one, and
//! `the_memory_mark_stays_small_beside_the_allocation_it_fronts` in
//! `benches/abi-probe/tests/perf_guards.rs` holds the second against a bound. It
//! is bought deliberately: AGENTS.md's priority ordering puts request
//! isolation above latency, and a cap nothing counts against is not a cap.
//!
//! The ceiling above it is three more cells of the same set — the threshold,
//! the address a crossing publishes into, and the bit a refusal is remembered
//! in — and on the allocation path one thread-local load and one compare per
//! *growing* allocation. A request under no ceiling arms `0` and stops at that
//! compare, which is why the sentinel is zero rather than a maximum: the
//! uncapped case is the one that must stay shortest. [`affords`] is one more
//! load and an add, paid only by an allocator that asks it, and each
//! [`Ctx`](crate::Ctx) carries the bit it displaced beside the threshold it
//! displaced.
//!
//! The accounting boundary is two more cells of that set and one predictable
//! branch per allocation, taken by the store that opened a bracket and by
//! nothing else.
//!
//! [`OffCore`] is two words per request *tree*, inside an allocation the tree
//! already makes, plus one relaxed load per reading taken on its root core and
//! one read-modify-write per poll taken by a member off it. Nothing on the
//! allocation path reads or writes them.
//!
//! The per-core gauge admission reads is `gauge`'s, and its module doc says
//! what it spends: one compare per balance change on the allocation path, and
//! a relaxed store once the balance has moved a mebibyte.
//!
//! A memory `FATAL` reports the peak, through
//! [`Ctx::memory_held`](crate::Ctx), and it adds no cell and no work on the
//! allocation path: the mark it reads is the one [`PEAK`] already keeps.
//! Before a breach the request was under its ceiling, so the mark since its
//! context began is also the mark since the breach. The read happens only
//! once a poll has found the breach. A debug build also checks each member
//! call against `bun nv proofs --overshoot`'s bound (`run_helper`'s
//! `overshoot_backstop`). That is two thread-local loads per call, and a
//! release build does not compile it.

#[cfg(not(test))]
use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;

mod gauge;

pub use gauge::{
    CoreGauge, STEP as GAUGE_STEP, give_share_back, held_by_cores, held_by_this_core, hold_share,
    process_limit, publish_held, set_process_limit,
};

/// What [`Accounting`] forwards to: Novis's own pooled allocator wherever it is
/// registered, and the platform heap everywhere else.
///
/// The split is `crate::alloc`'s, not this module's — a debug build is left
/// on the platform heap so a recycled block is never a block valgrind fails to
/// see freed, and the `sanitizer` feature takes the pool out for the same
/// reason. Counting is orthogonal to both: it sits outside whichever backing
/// allocator is chosen, so [`live_bytes`] means the same thing in every build.
#[cfg(all(not(test), not(debug_assertions), not(feature = "sanitizer")))]
use crate::alloc::Pooled as Backing;
#[cfg(all(not(test), any(debug_assertions, feature = "sanitizer")))]
use std::alloc::System as Backing;

thread_local! {
    /// Const-initialized and holding no `Drop` type, for `crate::alloc`'s
    /// *one trap*: a lazily-initialized thread local allocates its own state
    /// from inside the allocator.
    static LIVE: Cell<isize> = const { Cell::new(0) };
    /// The highest [`LIVE`] has reached since the innermost live
    /// [`Ctx`](crate::Ctx) rebased this mark —
    /// `rule:observability/a-memory-peak-is-recorded-not-asked-for`'s recorded
    /// high-water mark. It moves in [`add`], inside the branch that already
    /// tests for a positive delta, rather than being sampled by whoever asks:
    /// deterministic release means a spike is gone by the time a reader
    /// arrives, and the spike is what the mark exists for.
    static PEAK: Cell<isize> = const { Cell::new(0) };
    /// How far [`LIVE`] stands from the running tree's own balance: the
    /// module doc's *per request tree across switches*. [`live_bytes`]
    /// subtracts it, and [`ArmedStack`] carries it with the stack.
    static FOREIGN: Cell<isize> = const { Cell::new(0) };
    /// The request tree whose turn the running stack is, or `None` on a stack
    /// that runs no task of a tree on its root core.
    ///
    /// A handle rather than an address, because the scheduler takes it off the
    /// thread after a task's last turn, when the task's context and maybe the
    /// tree's last other holder are gone. Only moved at a switch, so no
    /// reference count changes there: one clone at a task's first turn, one
    /// drop after its last. Never read by the allocator, which is what lets it
    /// hold a type with `Drop`.
    static TURN: Cell<Option<std::sync::Arc<crate::TreeState>>> = const { Cell::new(None) };
    /// Monotonic: how many allocation requests this thread has made.
    static REQUESTS: Cell<usize> = const { Cell::new(0) };
    /// Monotonic: how many bytes those requests asked for.
    static TOTAL: Cell<usize> = const { Cell::new(0) };
    /// Monotonic: how many bytes this thread has written to a request's output
    /// sink — the module doc's second counter.
    static WRITTEN: Cell<usize> = const { Cell::new(0) };
    /// The [`LIVE`] balance the running request may not pass, or `0` for one
    /// under no ceiling.
    ///
    /// `rule:errors/on-limit`'s memory limit as an **absolute** balance rather
    /// than as the request's own reading, so that the compare in [`add`]
    /// subtracts nothing first. [`Ctx::refresh_limits`](crate::Ctx) is the one
    /// pass that computes the ceiling and [`arm`] the one writer of this, but
    /// for [`credit_turn`], which moves it with [`FOREIGN`] so the ceiling in
    /// the tree's own units stays where it is.
    static CEILING: Cell<isize> = const { Cell::new(0) };
    /// The safepoint word the request that armed [`CEILING`] polls, or null
    /// where nothing is armed.
    ///
    /// Read by an allocation that has already crossed the ceiling and by
    /// nothing else, which is why it is a second cell rather than a field
    /// beside the threshold: the ordinary path loads [`CEILING`] alone.
    static POLLED: Cell<*const std::sync::atomic::AtomicU64> =
        const { Cell::new(std::ptr::null()) };
    /// The first allocation this thread's running request was refused, or
    /// `None` — [`refuse`]'s verdict, which no counter can be read for.
    ///
    /// Per thread because the allocators that hit a refusal hold no
    /// [`Ctx`](crate::Ctx) to record it on, and confined to one request because
    /// the context that is refused displaces this cell exactly as it displaces
    /// [`CEILING`].
    static REFUSED: Cell<Option<Refusal>> = const { Cell::new(None) };
    /// Whether this thread is inside `rule:errors/on-limit`'s reserve, where
    /// [`affords`] answers without asking the balance — see [`Reporting`].
    static REPORTING: Cell<bool> = const { Cell::new(false) };
    /// Whether this thread is inside a [`Detached`] bracket, where an
    /// allocation is the process's rather than the running request's.
    static DETACHING: Cell<bool> = const { Cell::new(false) };
    /// The balance a [`Detached`] bracket moves in place of [`LIVE`]: how many
    /// bytes this thread has allocated on the process's behalf and not yet
    /// given back.
    ///
    /// Signed for [`LIVE`]'s reason, and more plainly so — a store filled by a
    /// request on one core and emptied by a request on another is the ordinary
    /// case for cross-request state rather than the exception it is for a
    /// request's own values.
    static DETACHED: Cell<isize> = const { Cell::new(0) };
}

/// `rule:errors/on-limit`'s reserve, open for as long as this value lives: the
/// pre-check in front of an allocation answers `true` while one is held.
///
/// The bytes a *report* costs are bytes the request has already been told it
/// cannot afford — it is over its ceiling, which is the entire reason there is
/// a report — so a pre-check applied to the runtime building one is a tier that
/// says nothing, for the same reason a handler that cannot allocate is. What
/// still bounds it is the counting ceiling: [`add`] compares the widened
/// threshold behind every allocation, and the poll that follows stops a report
/// that overran the slice it was lent.
///
/// It covers the runtime's **own** construction and never user code. A limit
/// handler's body runs outside it, so a program lent the reserve is still
/// refused a single allocation past the widened ceiling. Nesting is safe: each
/// guard puts back what it found rather than closing the reserve outright.
#[derive(Debug)]
pub struct Reporting(bool);

impl Reporting {
    /// Opens the reserve, and answers the guard that closes it again.
    #[must_use]
    pub fn begin() -> Self {
        Self(REPORTING.with(|open| open.replace(true)))
    }
}

impl Drop for Reporting {
    fn drop(&mut self) {
        REPORTING.with(|open| open.set(self.0));
    }
}

/// The accounting boundary of a cross-request store, open for as long as this
/// value lives: an allocation or a release made while one is held moves
/// [`detached_bytes`] and leaves [`live_bytes`] — and with it every request's
/// reading and every armed ceiling — where it found it.
///
/// The module doc's *whose bytes they are* says which bytes these are and why a
/// request must not be charged or credited for them. What a bracket owes is
/// symmetry, so it is held by the **store**, around every path that allocates
/// or frees what it holds, and never by a call site that happens to be storing.
/// `Drop` closes it on an unwind as well as on a return, which is why this is a
/// guard rather than a pair of calls.
///
/// Nesting is safe: each guard puts back what it found rather than closing the
/// bracket outright. [`allocations`] and [`allocated_bytes`] keep moving inside
/// one, because an allocation made on the process's behalf is still an
/// allocation this thread made, and a guard asserting that a member allocates
/// nothing has to fail on it.
#[derive(Debug)]
pub struct Detached(bool);

impl Detached {
    /// Opens the bracket, and answers the guard that closes it again.
    #[must_use]
    pub fn begin() -> Self {
        Self(DETACHING.with(|open| open.replace(true)))
    }
}

impl Drop for Detached {
    fn drop(&mut self) {
        DETACHING.with(|open| open.set(self.0));
    }
}

/// What a request tree holds, and has written, on cores other than the one its
/// root runs on — the module doc's *the part a thread cannot hold*.
///
/// One per request tree, in the allocation its safepoint word already occupies.
/// [`Ctx::memory_used`](crate::Ctx::memory_used) and
/// [`Ctx::output_used`](crate::Ctx::output_used) add these figures to the
/// thread-local share when the context asking runs on the tree's root core, and
/// add nothing when it does not: a context off that core reads its own thread's
/// balance, which already holds every context beneath it there.
///
/// `Relaxed` on both sides, exactly as the safepoint word is. The number is the
/// whole of what is published — nothing is ordered behind it — and a reading one
/// poll out of date is a reading of a tree that is still allocating anyway.
#[derive(Debug, Default)]
pub struct OffCore {
    /// A balance: charged as a member off the root's core grows, and given back
    /// whole when that member's context ends.
    ///
    /// Signed for [`live_bytes`]'s reason and one sharper — a child that
    /// releases what it was handed drives its own share below zero, and the
    /// reading that adds it floors rather than wrapping.
    memory: std::sync::atomic::AtomicIsize,
    /// Monotonic, as [`written_bytes`] is: bytes written to a response are never
    /// given back, so a member's share stays on the tree's count after that
    /// member has ended.
    output: std::sync::atomic::AtomicUsize,
}

impl OffCore {
    /// A tree with nothing placed off its root's core.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            memory: std::sync::atomic::AtomicIsize::new(0),
            output: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// How many bytes this tree holds on cores other than its root's.
    #[must_use]
    #[inline]
    pub fn memory(&self) -> isize {
        self.memory.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// How many bytes this tree has written from cores other than its root's.
    #[must_use]
    pub fn output(&self) -> usize {
        self.output.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Charges `delta` to the balance — negative where a member is giving its
    /// share back.
    pub fn charge_memory(&self, delta: isize) {
        self.memory
            .fetch_add(delta, std::sync::atomic::Ordering::Relaxed);
    }

    /// Charges `bytes` to the written count.
    ///
    /// Saturating for [`wrote`]'s reason: a count that wrapped would answer
    /// "under the ceiling" for the one tree that most certainly is not.
    pub fn charge_output(&self, bytes: usize) {
        let _ = self.output.try_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |written| Some(written.saturating_add(bytes)),
        );
    }
}

/// A ceiling and the word a crossing of it publishes into — what [`arm`] sets
/// and [`displace`] hands back.
///
/// The two travel together because arming one without the other is either a
/// threshold nothing can report or an address nothing will reach.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Armed {
    ceiling: isize,
    word: *const std::sync::atomic::AtomicU64,
}

impl Armed {
    /// Nothing armed: no threshold, and no word to publish a crossing into —
    /// the state a thread running no request is left in. A request under no
    /// ceiling arms a threshold of `0` with its word, so the server's memory
    /// running out can still stop it (`gauge`'s module doc).
    pub(crate) const NONE: Self = Self {
        ceiling: 0,
        word: std::ptr::null(),
    };

    /// The threshold `ceiling` published into `word`.
    ///
    /// `word` must be the address of an `AtomicU64` that outlives the arming —
    /// [`arm`] owns what the caller owes for that.
    pub(crate) const fn new(ceiling: isize, word: *const std::sync::atomic::AtomicU64) -> Self {
        Self { ceiling, word }
    }
}

/// Arms `next` for this thread.
///
/// **What the caller owes:** the word `next` names must still be there at every
/// allocation until something arms over it. Two arrangements discharge that
/// together. [`Ctx`](crate::Ctx) holds the pair it displaced in a field and arms
/// it again as it drops, which is [`rebase_peak`]'s arrangement applied to a
/// second number, and covers contexts that nest on one stack. [`ArmedStack`]
/// takes the pair off the thread whenever a task's stack stops running, which
/// covers contexts on different stacks: a context made while another task is
/// parked displaces nothing of that task's. So the address armed always belongs
/// to a context that is still running, on the stack the thread is running.
///
/// `next.ceiling` is in the running stack's own units, as [`live_bytes`] is,
/// and [`CEILING`] is stored against the thread's [`LIVE`] so the compare in
/// [`carry`] subtracts nothing. A threshold that lands on the `0` sentinel is
/// moved one byte up, so a capped request never reads as uncapped.
pub(crate) fn arm(next: Armed) {
    let ceiling = if next.ceiling == 0 {
        0
    } else {
        match next.ceiling.saturating_add(FOREIGN.with(Cell::get)) {
            0 => 1,
            raw => raw,
        }
    };
    CEILING.with(|cell| cell.set(ceiling));
    POLLED.with(|polled| polled.set(next.word));
}

/// Arms `next` and hands back the pair it displaced.
///
/// [`Ctx::new`](crate::Ctx)'s half of the nesting above: a context made inside
/// another takes the thread's arming with its own and carries the enclosing
/// pair until it drops.
pub(crate) fn displace(next: Armed) -> Armed {
    let displaced = Armed {
        ceiling: armed_ceiling(),
        word: POLLED.with(Cell::get),
    };
    arm(next);
    displaced
}

/// The arming, the refusal and the reserve of the stack that is about to stop
/// running, carried off the thread until that stack runs again.
///
/// `crate::CurrentStack` is the model and its doc the argument. All four cells
/// describe one *task*, while the thread-local they live in is shared by every
/// coroutine the core runs. Without this, a task that makes a context while
/// another task is parked saves *that* task's arming and writes it back as its
/// context drops. Two requests interleaved on one core are then each measured
/// against the other's ceiling. If the parked task ended in between, the
/// restore leaves [`POLLED`] naming the safepoint word of a tree that is gone,
/// and the next allocation past the ceiling writes into freed memory.
/// `rule:security/isolate-shares-nothing` is the rule this holds.
///
/// `nvs_host`'s `yield_on` is the one caller, beside `CurrentStack`:
/// [`take`](ArmedStack::take) on the way out of a stack, and
/// [`restore`](ArmedStack::restore) on the way back in. A task that never comes
/// back simply drops this, and its contexts' own drops put back what they
/// displaced on that stack, which is nothing.
///
/// It also carries the stack's share of the balance: the tree whose turn it
/// is ([`TURN`]), its [`FOREIGN`] offset and its [`PEAK`] mark. The module
/// doc's *per request tree across switches* is what they are for. [`LIVE`]
/// itself stays the thread's, because the core's gauge publishes it.
/// [`WRITTEN`] stays on the thread too: output is the tree's budget, and a
/// tree's tasks share one core.
///
/// **Cost:** seven thread-local moves per park and seven per resume, one
/// relaxed store at a park and one relaxed load at a resume, against a switch
/// that already costs a stack change, and nothing on the allocation path.
/// **Memory:** seven words per parked task, O(in-flight).
#[derive(Debug)]
#[must_use = "the task's arming is off the thread until this is restored"]
pub struct ArmedStack {
    armed: Armed,
    refused: Option<Refusal>,
    reporting: bool,
    /// The tree whose turn the stack was, its balance stored as it parked.
    turn: Option<std::sync::Arc<crate::TreeState>>,
    /// [`FOREIGN`] when the stack parked, which is put back only on a stack
    /// with no tree.
    foreign: isize,
    /// The stack's mark in its own units, as [`peak_bytes`] reads it.
    peak: isize,
}

impl ArmedStack {
    /// Takes the cells off the thread and leaves it under nothing: no
    /// threshold, no word, no refusal, no reserve, no tree, no offset, and a
    /// mark at the balance as it stands.
    ///
    /// The tree whose turn ends here is given the balance it ends at, which
    /// is how a task's last turn reaches its tree as well: the scheduler takes
    /// the stack once more after the task returns.
    pub fn take() -> Self {
        let turn = TURN.with(Cell::take);
        if let Some(tree) = &turn {
            let balance = live_bytes();
            tree.balance
                .store(balance, std::sync::atomic::Ordering::Relaxed);
            settle_idle(tree, balance);
        }
        let live = LIVE.with(Cell::get);
        let taken = Self {
            armed: displace(Armed::NONE),
            refused: take_refusal(),
            reporting: REPORTING.with(|open| open.replace(false)),
            turn,
            foreign: FOREIGN.with(|cell| cell.replace(0)),
            peak: 0,
        };
        let peak = PEAK.with(|cell| cell.replace(live));
        Self {
            peak: peak.wrapping_sub(taken.foreign),
            ..taken
        }
    }

    /// Puts back what [`take`](ArmedStack::take) answered.
    ///
    /// The scheduler's side of a resume. A scheduler that runs on no tree's
    /// turn is charged for every task it resumes, as a caller is charged for
    /// what it calls, and its mark keeps the higher of its own and the balance
    /// the task left.
    pub fn restore(self) {
        self.resume();
        PEAK.with(|cell| cell.set(cell.get().max(LIVE.with(Cell::get))));
    }

    /// Puts back what [`take`](ArmedStack::take) answered, in the units of
    /// the tree whose turn this stack is.
    ///
    /// The task's side of a resume, in `nvs_host`'s `yield_on`: a request
    /// parked here reads, and is measured against its ceiling by, its own
    /// tree's allocations alone, however many other trees ran on the core
    /// meanwhile.
    pub fn resume(self) {
        let foreign = match &self.turn {
            Some(tree) => enter(tree),
            None => self.foreign,
        };
        TURN.with(|cell| cell.set(self.turn));
        FOREIGN.with(|cell| cell.set(foreign));
        arm(self.armed);
        restore_refusal(self.refused);
        REPORTING.with(|open| open.set(self.reporting));
        PEAK.with(|cell| cell.set(self.peak.wrapping_add(foreign)));
    }

    /// Whether the stack this came off had a safepoint word armed.
    ///
    /// For the assertion a caller cannot otherwise make: a thread between
    /// tasks names no word, and a restore that crossed a switch is what would
    /// make it name one that is gone.
    #[must_use]
    pub fn is_armed(&self) -> bool {
        !self.armed.word.is_null()
    }
}

/// The [`FOREIGN`] offset that puts this thread in `tree`'s units: its
/// balance as the last turn left it is what [`live_bytes`] reads next.
fn enter(tree: &crate::TreeState) -> isize {
    LIVE.with(Cell::get)
        .wrapping_sub(tree.balance.load(std::sync::atomic::Ordering::Relaxed))
}

/// Starts a turn of `tree` on a stack that runs no tree yet: a task's first
/// turn, in [`Ctx::arm_on_this_stack`](crate::Ctx::arm_on_this_stack). The
/// threshold armed next is read in the tree's units.
pub(crate) fn begin_turn(tree: &std::sync::Arc<crate::TreeState>) {
    FOREIGN.with(|cell| cell.set(enter(tree)));
    TURN.with(|cell| cell.set(Some(std::sync::Arc::clone(tree))));
}

/// Hands the running turn from `old` to `new` where the running stack is
/// `old`'s turn: [`Ctx::reroot`](crate::Ctx::reroot) on a connection that
/// serves one request after another. Both trees are given the balance as it
/// stands, so the stack's units do not move. A carrier the connection parsed
/// under the last request's tree and the next request frees is given back on
/// the tree it was charged to. Without this, the connection's turns charge a
/// tree no request reads, and each request's freed carrier stays on it.
pub(crate) fn reroot_turn(
    old: &std::sync::Arc<crate::TreeState>,
    new: &std::sync::Arc<crate::TreeState>,
) {
    TURN.with(|cell| {
        let running = cell.take();
        let next = match running {
            Some(tree) if std::sync::Arc::ptr_eq(&tree, old) => {
                let live = live_bytes();
                tree.balance
                    .store(live, std::sync::atomic::Ordering::Relaxed);
                new.balance
                    .store(live, std::sync::atomic::Ordering::Relaxed);
                Some(std::sync::Arc::clone(new))
            }
            other => other,
        };
        cell.set(next);
    });
}

/// The tree whose turn the running stack is, by address, or null on a stack
/// that runs no tree's turn. [`Ctx::new`](crate::Ctx::new) notes it, so a
/// spawn can tell whether the context was made on the turn that hands it over.
pub(crate) fn running_turn() -> *const crate::TreeState {
    TURN.with(|cell| {
        let turn = cell.take();
        let at = turn
            .as_ref()
            .map_or(std::ptr::null(), std::sync::Arc::as_ptr);
        cell.set(turn);
        at
    })
}

/// Takes `bytes` off the running tree's balance and leaves the thread's
/// [`LIVE`] where it is.
///
/// For bytes this turn allocated that another tree's last turn frees: a
/// context a spawn hands to a task of its own tree
/// ([`Ctx::hand_over_its_making`](crate::Ctx::hand_over_its_making)). Left on
/// this tree, they are a charge nobody frees, one per task spawned. The mark
/// and the ceiling move with the balance, so in the tree's units only the
/// balance moves.
pub(crate) fn credit_turn(bytes: isize) {
    FOREIGN.with(|cell| cell.set(cell.get().wrapping_add(bytes)));
    PEAK.with(|cell| cell.set(cell.get().wrapping_add(bytes)));
    CEILING.with(|cell| {
        let raw = cell.get();
        if raw != 0 {
            cell.set(match raw.saturating_add(bytes) {
                0 => 1,
                moved => moved,
            });
        }
    });
}

/// Starts counting `tree` as a request's from the balance as it stands:
/// [`Ctx::reroot`](crate::Ctx::reroot), in front of each request a connection
/// serves. What the tree holds above this is what [`begin_idle`] later sets
/// aside.
pub(crate) fn count_from_here(tree: &crate::TreeState) {
    use std::sync::atomic::Ordering::Relaxed;
    tree.base.store(live_bytes(), Relaxed);
    tree.idling.store(COUNTED, Relaxed);
}

/// [`TreeState::idling`](crate::TreeState) of a tree whose base nothing has
/// set: a fresh tree, whose balance starts where the stack that made it stood.
const UNBASED: u8 = 0;
/// [`TreeState::idling`](crate::TreeState) of a tree that counts as a
/// request's and has a base.
const COUNTED: u8 = 1;
/// [`TreeState::idling`](crate::TreeState) of a connection's tree between
/// requests.
const IDLE: u8 = 2;

/// Marks `tree` as a connection's between requests, so what it holds above its
/// base leaves this core's published balance from its next park on
/// (`gauge`'s *a connection between requests is not a request*).
///
/// Answers whether the tree is idle now. It is not where the running stack is
/// not `tree`'s turn, because only a turn keeps the tree's balance current.
pub(crate) fn begin_idle(tree: &std::sync::Arc<crate::TreeState>) -> bool {
    let running = TURN.with(|cell| {
        let turn = cell.take();
        let running = turn
            .as_ref()
            .is_some_and(|turn| std::sync::Arc::ptr_eq(turn, tree));
        cell.set(turn);
        running
    });
    if !running {
        return false;
    }
    let balance = live_bytes();
    // A connection waiting for its first request measures from here.
    if tree.idling.swap(IDLE, std::sync::atomic::Ordering::Relaxed) == UNBASED {
        tree.base
            .store(balance, std::sync::atomic::Ordering::Relaxed);
    }
    settle_idle(tree, balance);
    true
}

/// Counts `tree` as a request's again: the next request on its connection has
/// started, or the connection's context is ending. What it had set aside goes
/// back onto the published balance.
pub(crate) fn end_idle(tree: &crate::TreeState) {
    if tree
        .idling
        .compare_exchange(
            IDLE,
            COUNTED,
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
        )
        .is_ok()
    {
        let held = tree.idle_held.swap(0, std::sync::atomic::Ordering::Relaxed);
        gauge::move_idle(held.wrapping_neg());
    }
}

/// Sets aside what an idle `tree` holds at `balance`, in its own units, above
/// its base. Nothing while a child of the tree is still running, and never
/// less than zero: a tree that freed more than it allocated since its base
/// gave back bytes another turn charged, and sets none of its own aside.
fn settle_idle(tree: &crate::TreeState, balance: isize) {
    use std::sync::atomic::Ordering::Relaxed;
    if tree.idling.load(Relaxed) != IDLE {
        return;
    }
    let held = if tree.tasks.load(Relaxed) > 0 {
        0
    } else {
        balance.wrapping_sub(tree.base.load(Relaxed)).max(0)
    };
    let before = tree.idle_held.swap(held, Relaxed);
    gauge::move_idle(held.wrapping_sub(before));
}

/// Raises [`SafepointFlags::MEMORY_LIMIT`](crate::SafepointFlags) and
/// [`SafepointFlags::COLLECT`](crate::SafepointFlags) in the word the armed
/// request's tree polls.
///
/// **Both bits, because this is the only place that knows the ceiling was
/// crossed**, and the collector is the one that runs only there: the poll the
/// first bit brings the request to is where the second is answered, so a cycle
/// holding the bytes is reclaimed before the counter decides whether the
/// request stops. `crate::object::collect` owns what that walk costs and
/// `Ctx::collect_if_asked` is the door it goes through.
///
/// Out of line from [`add`], because it is reached only by an allocation that
/// has already crossed the ceiling. It allocates nothing and takes no lock,
/// which is what lets the global allocator be the caller.
fn publish() {
    let word = POLLED.with(Cell::get);
    if word.is_null() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "the word is reached by address because the allocator has no \
                  `Ctx` to reach it through; the reference made here is shared \
                  and the store below is atomic, so this is the access the \
                  watchdog thread already makes through `SafepointView`"
    )]
    // SAFETY: `POLLED` is non-null only between an `arm` and the arming that
    // replaces it, and every armed address is the `Arc` allocation a live
    // `Ctx` holds — the word lives in that allocation rather than inside the
    // context, so a `&mut Ctx` on this thread grants no unique access to it
    // and this shared reference does not alias one.
    let word = unsafe { &*word };
    word.fetch_or(
        (crate::SafepointFlags::MEMORY_LIMIT | crate::SafepointFlags::COLLECT).bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
}

/// Records that an allocation was refused, and asks for the poll that reports
/// it.
///
/// Both halves are needed and neither is the other. The flag alone is what
/// brings compiled code to a poll at all, and the poll asks the counters —
/// which, after a refusal, say the request is comfortably inside its ceiling,
/// because the bytes it asked for were never handed over. So the verdict is
/// remembered here, where
/// [`Ctx::over_memory_limit`](crate::Ctx::over_memory_limit) reads it.
///
/// Private, so that a refusal cannot be recorded without the arithmetic in
/// [`affords`] that justifies it. Only the first refusal is kept: it is the
/// breach, and anything the request asks for after it is a consequence.
fn refuse(asked: isize, live: isize) {
    REFUSED.with(|refused| {
        if refused.get().is_none() {
            refused.set(Some(Refusal {
                asked,
                live,
                full: false,
            }));
        }
    });
    publish();
}

/// Why the running request was stopped by the allocator: an allocation
/// [`affords`] refused, or the server's memory running out while it grew. It
/// carries what was asked for and the thread's [`live_bytes`] balance at that
/// moment.
///
/// Kept because the poll that reports the breach comes after the refusal, and
/// by then the reading has moved — an operation refused half way, such as a
/// concatenation, has already given back the operand it was growing. The
/// `FATAL` names these numbers instead, which is what tells an operator
/// how close the program was to fitting.
///
/// The server's memory running out is kept here rather than in a cell of its
/// own because it travels exactly as a refusal does: a context takes the
/// thread's verdict as it starts, gives it back as it ends, and a limit handler
/// runs without it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Refusal {
    /// The bytes the refused allocation asked for, saturated at `isize::MAX`,
    /// and `0` where `full` is set.
    pub(crate) asked: isize,
    /// The running stack's absolute balance when it asked, as [`live_bytes`]
    /// reads it.
    pub(crate) live: isize,
    /// Whether what ran out was the server's memory and not the request's own
    /// ceiling: a publish took the sum of every core's balance past
    /// [`set_process_limit`]'s limit while this request was growing.
    pub(crate) full: bool,
}

/// Whether the running request has been refused an allocation.
///
/// Sticky: the request that is refused is over, and nothing it does afterwards
/// brings it back under a ceiling it never held the bytes against.
/// [`take_refusal`] is the only way back, and its two callers are the context
/// that ends and the handler that is lent a slice to report with.
///
/// Public for a `Core` member's walk: a refused write stores nothing, so a
/// walk that keeps going after one spends its time for no result. Reading
/// this once per row or per nested array, and returning, is what stops it.
#[inline]
#[must_use]
pub fn refused() -> bool {
    REFUSED.with(Cell::get).is_some()
}

/// Whether the running request was stopped because the server's memory is
/// full — the verdict [`Refusal::full`] names. Sticky as [`refused`] is.
#[inline]
pub(crate) fn server_full() -> bool {
    REFUSED.with(Cell::get).is_some_and(|refusal| refusal.full)
}

/// The first allocation the running request was refused, or `None` while it
/// has been refused nothing.
pub(crate) fn refusal() -> Option<Refusal> {
    REFUSED.with(Cell::get)
}

/// Hands the refusal back and clears it.
///
/// [`displace`]'s arrangement for the verdict that travels with the threshold:
/// [`Ctx::new`](crate::Ctx::new) takes what the thread was carrying so a
/// request starts under no refusal of anyone else's, and
/// [`Ctx::run_limit_handler`](crate::Ctx) takes it for the length of the call
/// so `rule:errors/on-limit`'s handler can allocate the report it exists to
/// write.
#[must_use]
pub(crate) fn take_refusal() -> Option<Refusal> {
    REFUSED.with(|cell| cell.replace(None))
}

/// Puts back what [`take_refusal`] handed out.
pub(crate) fn restore_refusal(refused: Option<Refusal>) {
    REFUSED.with(|cell| cell.set(refused));
}

/// How many bytes the running stack has allocated on this thread and not yet
/// freed.
///
/// The absolute balance, not a request's share — [`Ctx::memory_used`](crate::Ctx::memory_used)
/// is the per-request reading and the module doc says how the two relate.
/// What other tasks moved the thread's balance by while this stack was parked
/// is left out, so on a stack that never parked this is the thread's whole
/// balance.
#[must_use]
#[inline]
pub fn live_bytes() -> isize {
    LIVE.with(Cell::get).wrapping_sub(FOREIGN.with(Cell::get))
}

/// How many bytes this thread has allocated and not yet freed, whatever task
/// allocated them.
///
/// [`live_bytes`] with every parked task's share left in: the balance the
/// core's gauge publishes. A test reads it to count what many waiting requests
/// on one core hold together.
#[must_use]
pub fn thread_live_bytes() -> isize {
    LIVE.with(Cell::get)
}

/// How many bytes this thread has allocated inside a [`Detached`] bracket and
/// not yet given back inside one.
///
/// The process's share of what the thread holds, which [`live_bytes`] and every
/// per-request reading derived from it exclude on purpose. It is a figure to
/// attribute bytes with rather than a ceiling to enforce: what bounds a store's
/// entries is the store's own limit, `[cache.memory] max_size` for
/// `Core\Cache::memory()`.
#[must_use]
pub fn detached_bytes() -> isize {
    DETACHED.with(Cell::get)
}

/// The highest [`live_bytes`] has reached since the innermost live
/// [`Ctx`](crate::Ctx) rebased the mark.
///
/// The absolute mark, not a request's share —
/// [`Ctx::memory_peak`](crate::Ctx::memory_peak) is the per-request reading and
/// stands to this exactly as [`Ctx::memory_used`](crate::Ctx::memory_used)
/// stands to [`live_bytes`].
/// `rule:observability/a-memory-peak-is-recorded-not-asked-for` is why the
/// runtime keeps this at all rather than leaving a program to sample the
/// balance.
#[must_use]
pub fn peak_bytes() -> isize {
    PEAK.with(Cell::get).wrapping_sub(FOREIGN.with(Cell::get))
}

/// Starts a fresh mark at the current balance, and hands back the one it
/// displaced.
///
/// [`Ctx::new`](crate::Ctx::new)'s half of the nesting the rule above states:
/// a context created inside another rebases the mark to its own baseline and
/// carries the enclosing value, so it measures its own allocation and no
/// caller's.
pub(crate) fn rebase_peak() -> isize {
    let base = LIVE.with(Cell::get);
    PEAK.with(|peak| peak.replace(base))
        .wrapping_sub(FOREIGN.with(Cell::get))
}

/// Starts the mark again at the current balance, keeping the one
/// [`rebase_peak`] displaced where its caller holds it.
///
/// [`Ctx::rebase_memory`](crate::Ctx::rebase_memory)'s half: the context's
/// baseline moves, so the mark it measures from moves with it.
pub(crate) fn restart_peak() {
    let base = LIVE.with(Cell::get);
    PEAK.with(|peak| peak.set(base));
}

/// Restores `enclosing` as the mark, keeping whichever of the two is higher.
///
/// The other half, run as a context drops: publishing `max(enclosing, reached)`
/// is what stops an isolate that allocated little from erasing the peak of the
/// request that spawned it.
pub(crate) fn publish_peak(enclosing: isize) {
    let enclosing = enclosing.saturating_add(FOREIGN.with(Cell::get));
    PEAK.with(|peak| peak.set(peak.get().max(enclosing)));
}
/// How many allocation requests this thread has made, never decreasing.
///
/// One per `alloc`, `alloc_zeroed` and `realloc`, and none per `dealloc` —
/// which is what a guard asserting "this member allocates nothing" counts. It
/// answers where [`live_bytes`] cannot: a call that allocates and frees again
/// before it returns balances to zero and still made the allocation.
#[must_use]
pub fn allocations() -> usize {
    REQUESTS.with(Cell::get)
}

/// How many bytes this thread has ever allocated, never decreasing.
///
/// The other half of [`allocations`]: a run that allocated nothing has a zero
/// balance too, so a guard reading [`live_bytes`] alone cannot tell "released
/// everything" from "never ran".
#[must_use]
pub fn allocated_bytes() -> usize {
    TOTAL.with(Cell::get)
}

/// Takes `bytes` off [`allocated_bytes`]'s total: what a debug build allocates
/// for a check that a release build does not make. A counted run then prints
/// the same `bytes` on every build (`rule:testing/bench-counters`). The live
/// balance keeps them, because they are held, so a ceiling still sees them.
#[cfg(debug_assertions)]
pub(crate) fn uncount_debug_bytes(bytes: usize) {
    TOTAL.with(|total| total.set(total.get().saturating_sub(bytes)));
}

/// How many freed blocks this thread's allocator cache holds for a request of
/// `layout`, or `0` in a build that has no cache and for a layout it does not
/// serve.
///
/// The count falls by one for each allocation the cache serves and rises by
/// one for each block freed back into it, so a guard reads it to show that the
/// registered allocator served a request from the cache and not from the
/// platform heap. A debug build has no cache, for the reason `crate::alloc`
/// gives, and a `sanitizer` build keeps one that no allocation reaches, so its
/// count never moves.
#[must_use]
pub fn pooled_blocks(layout: std::alloc::Layout) -> u32 {
    #[cfg(any(test, not(debug_assertions)))]
    let blocks = crate::alloc::cached_blocks(layout);
    #[cfg(not(any(test, not(debug_assertions))))]
    let blocks = {
        let _ = layout;
        0
    };
    blocks
}

/// How many bytes this thread has written to a request's output, never
/// decreasing.
///
/// The absolute count, not a request's share:
/// [`Ctx::output_used`](crate::Ctx::output_used) is the per-request reading and
/// relates to this exactly as [`Ctx::memory_used`](crate::Ctx::memory_used)
/// relates to [`live_bytes`]. The module doc says why the counter is per
/// *thread* and what that buys an isolate.
#[must_use]
pub fn written_bytes() -> usize {
    WRITTEN.with(Cell::get)
}

/// Charges `bytes` to this thread's output count.
///
/// Called from [`Ctx::write_output`](crate::Ctx::write_output), at the point
/// the bytes reach the sink rather than at the point a program hands them over,
/// which is the distinction that doc comment owns. Saturating, because a count
/// that wrapped would answer "under the ceiling" for the one request that most
/// certainly is not.
pub(crate) fn wrote(bytes: usize) {
    WRITTEN.with(|written| written.set(written.get().saturating_add(bytes)));
}

/// The [`live_bytes`] balance the running request may not pass, or `0` for one
/// under no ceiling.
///
/// The absolute threshold [`add`] compares a growing allocation against, which
/// is [`Ctx::memory_limit`](crate::Ctx::memory_limit) plus the balance the
/// request started from. The `0` is not a small ceiling but the sentinel the
/// allocator short-circuits on, and reading it is how a case asks whether a
/// request armed anything at all.
///
/// Read in the running stack's units, as [`live_bytes`] is, so a task that
/// parks reads the same threshold when it resumes.
#[must_use]
pub fn armed_ceiling() -> isize {
    match CEILING.with(Cell::get) {
        0 => 0,
        raw => raw.wrapping_sub(FOREIGN.with(Cell::get)),
    }
}

/// Whether the running request can still afford `bytes` — and the refusal
/// itself where it cannot.
///
/// **A `false` answer has already recorded the breach.** What the caller owes
/// is to allocate nothing and hand back a value that cost nothing: the request
/// is over, its next poll will say so, and between here and there it may build
/// wrong values and compare them but can write no output and reach no `Core`
/// member, because every one of those passes [`crate::run_helper`]'s question
/// first.
///
/// Asked *in front of* the allocation, which is the only position from which
/// one operation is bounded rather than a loop of them — [`add`] compares after
/// `Backing` has already handed the block over. It lives here rather than at
/// the call site for [`crate::affordable`]'s reason: a guard written per call
/// site is a guard the next call site forgets.
///
/// An uncapped request answers on the sentinel without reading the balance at
/// all, and so do a thread inside [`Reporting`]'s reserve and one inside a
/// [`Detached`] bracket, whose bytes are not the request's to be refused.
#[must_use]
pub fn affords(bytes: usize) -> bool {
    let ceiling = CEILING.with(Cell::get);
    if ceiling == 0 {
        return true;
    }
    if REPORTING.with(Cell::get) || DETACHING.with(Cell::get) {
        return true;
    }
    // Saturating in the direction that refuses: an ask too large to count in
    // the balance's own type is one no request can afford, so it must not wrap
    // into a number that fits.
    let ask = isize::try_from(bytes).unwrap_or(isize::MAX);
    let live = LIVE.with(Cell::get);
    if live.saturating_add(ask) <= ceiling {
        return true;
    }
    refuse(ask, live.wrapping_sub(FOREIGN.with(Cell::get)));
    false
}

/// What the running request may still allocate before it crosses its ceiling,
/// or `None` where nothing holds it to one.
///
/// For work this thread hands to another thread while it waits. A job on
/// `nvs_host::blocking`'s pool allocates on a thread armed with no ceiling, and
/// what it builds is carried onto this balance only when it returns. So the job
/// is handed this figure before it starts and stops once it holds more, and the
/// caller then asks [`affords`] for what the job reached, which records the
/// refusal. `None` in the cases [`affords`] answers without reading the
/// balance: an uncapped request, and a thread inside [`Reporting`] or
/// [`Detached`].
#[must_use]
pub fn headroom() -> Option<usize> {
    let ceiling = CEILING.with(Cell::get);
    if ceiling == 0 || REPORTING.with(Cell::get) || DETACHING.with(Cell::get) {
        return None;
    }
    Some(usize::try_from(ceiling.saturating_sub(LIVE.with(Cell::get))).unwrap_or(0))
}

/// Whether the running request now holds more than its ceiling.
///
/// For a loop that builds one value out of many allocations and reaches no
/// poll on the way, such as a buffered database read appending its rows.
/// [`add`] has already raised the poll flag at the crossing, and this is how
/// the loop learns of it before its next row. The `FATAL` is
/// [`Ctx::memory_breach`](crate::Ctx::memory_breach)'s to write.
#[must_use]
pub fn over_ceiling() -> bool {
    let ceiling = CEILING.with(Cell::get);
    ceiling != 0
        && !REPORTING.with(Cell::get)
        && !DETACHING.with(Cell::get)
        && LIVE.with(Cell::get) > ceiling
}

/// The bytes [`ByteWriter::read_from`] takes from its reader in one read.
pub const READ_STEP: usize = 16 * 1024;

/// Why a [`ByteWriter`] stopped writing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stopped {
    /// The request's memory ceiling refused the buffer's growth. The breach is
    /// recorded, and the request's next poll stops it.
    Refused,
    /// The bytes grew past the writer's own limit, which its caller set.
    Over,
}

/// A byte buffer that asks the running request's budget in front of every
/// growth, for every `Core` member that writes a payload of bytes.
///
/// **Asked before the allocation, not charged after it**, for [`affords`]'s
/// reason: the live balance sees a `Vec`'s block only once it is handed over,
/// so a payload of ten gigabytes was written whole before the copy into a
/// `bytes` value was refused. The ask is the whole new block, because a
/// reallocation holds the old one and the new one at once. One writer here and
/// not a check per member, because a guard written per call site is a guard
/// the next call site forgets.
///
/// After a refusal or a write past the limit, nothing more is written and
/// nothing more is allocated, and [`Self::finish`] says which one stopped it.
/// What it spends: one comparison per write that fits, and the buffer grows by
/// doubling, so it holds at most twice its payload.
#[derive(Debug)]
pub struct ByteWriter {
    /// The bytes written so far.
    out: Vec<u8>,
    /// The longest payload the caller keeps, and `usize::MAX` for any.
    limit: usize,
    /// What stopped the writes, once something has.
    stopped: Option<Stopped>,
}

impl Default for ByteWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteWriter {
    /// An empty writer that only the request's memory ceiling stops.
    #[must_use]
    pub const fn new() -> Self {
        Self::within(usize::MAX)
    }

    /// An empty writer that also stops once its payload would pass `limit`
    /// bytes. A write past the limit is checked first and allocates nothing,
    /// so a caller that would throw a long payload away never pays for it.
    #[must_use]
    pub const fn within(limit: usize) -> Self {
        Self {
            out: Vec::new(),
            limit,
            stopped: None,
        }
    }

    /// Appends `bytes`, or nothing once the writer has stopped.
    pub fn put(&mut self, bytes: &[u8]) {
        if self.stopped.is_some() {
            return;
        }
        let needed = self.out.len().saturating_add(bytes.len());
        if needed > self.limit {
            self.stopped = Some(Stopped::Over);
            return;
        }
        if needed > self.out.capacity() {
            let grown = needed.max(self.out.capacity().saturating_mul(2).min(self.limit));
            if !affords(grown) {
                self.stopped = Some(Stopped::Refused);
                return;
            }
            self.out.reserve_exact(grown - self.out.len());
        }
        self.out.extend_from_slice(bytes);
    }

    /// Appends one byte, as [`Self::put`] does.
    pub fn put_byte(&mut self, byte: u8) {
        self.put(&[byte]);
    }

    /// Appends everything `reader` gives until its end, or until the writer
    /// stops. A file, a pipe and standard input are read this way, so a source
    /// larger than the request can hold stops at the growth that does not fit.
    ///
    /// What it spends: one [`READ_STEP`] buffer on the stack, and one copy of
    /// each step into the payload.
    ///
    /// # Errors
    ///
    /// Whatever `reader` said, other than an interrupted read, which is asked
    /// again.
    pub fn read_from(&mut self, reader: &mut impl std::io::Read) -> std::io::Result<()> {
        let mut step = [0_u8; READ_STEP];
        while self.stopped.is_none() {
            match reader.read(&mut step) {
                Ok(0) => break,
                Ok(read) => self.put(&step[..read]),
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    /// How many bytes are written.
    #[must_use]
    pub fn len(&self) -> usize {
        self.out.len()
    }

    /// Whether nothing is written.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.out.is_empty()
    }

    /// What stopped the writes, or `None` while every write has landed. A
    /// caller whose work has no other end checks this and stops its work too.
    #[must_use]
    pub const fn stopped(&self) -> Option<Stopped> {
        self.stopped
    }

    /// The bytes written, or what stopped the writes. A writer that stopped
    /// frees its partial payload here.
    ///
    /// # Errors
    ///
    /// [`Stopped`], where a write was refused or passed the limit.
    pub fn finish(self) -> Result<Vec<u8>, Stopped> {
        match self.stopped {
            None => Ok(self.out),
            Some(stopped) => Err(stopped),
        }
    }
}

/// [`ByteWriter::put`] for a writer that takes `io::Write`, such as
/// `serde_json`'s formatter. A write never fails: after a stop the bytes are
/// discarded, and the caller reads [`ByteWriter::stopped`] to end its work.
impl std::io::Write for ByteWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.put(bytes);
        Ok(bytes.len())
    }

    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.put(bytes);
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The counters that say an allocation was made, whoever ends up holding the
/// bytes.
///
/// Out of line from [`add`] because both of its balances move them: which
/// balance a delta lands on is a question of attribution, and whether the
/// thread allocated is not.
fn record(bytes: isize) {
    REQUESTS.with(|count| count.set(count.get().wrapping_add(1)));
    let grew = usize::try_from(bytes).unwrap_or(0);
    TOTAL.with(|total| total.set(total.get().wrapping_add(grew)));
}

/// Charges `bytes` to this thread's counters — negative for a release.
///
/// `pub(crate)`, and called from [`Accounting`] below and from
/// `counting_alloc`'s own arithmetic in a test build. A positive delta is one
/// allocation *request*, which is why the memory counters move together here
/// rather than at each allocating call site — [`PEAK`] among them, so the
/// mark is exact for every allocation rather than approximate between two
/// reads. A release moves the balance and nothing else: bytes given back
/// cannot raise a high-water mark, and the branch is the one the monotonic
/// counters already needed.
///
/// Inside a [`Detached`] bracket the delta lands on the process's balance
/// instead, and every question this asks of the request — its mark, its
/// ceiling — is one the bytes are not the request's to answer.
pub(crate) fn add(bytes: isize) {
    if bytes > 0 {
        record(bytes);
    }
    carry(bytes);
}

/// Moves `bytes` onto this thread's balance without counting an allocation —
/// negative to move them off it.
///
/// For a block that crosses threads while it is alive. The module doc's
/// *Per thread* section says what a cross-thread free does to two balances
/// left alone: the allocating thread keeps a charge nobody frees and the
/// freeing thread goes negative, which is a credit its request can spend past
/// its own ceiling. The thread that hands a block over moves its size off
/// with a negative delta, and the thread that takes it moves the same size on,
/// so each block is charged to the thread that will free it. A positive delta
/// raises the mark and meets the ceiling exactly as an allocation does,
/// because the bytes are this thread's to hold from here on;
/// `nvs_host::blocking::run` is the caller.
pub fn carry(bytes: isize) {
    if DETACHING.with(Cell::get) {
        let held = DETACHED.with(|held| {
            let now = held.get().wrapping_add(bytes);
            held.set(now);
            now
        });
        if gauge::far_from(held, &gauge::DETACHED_MARK) {
            gauge::report(bytes > 0);
        }
        return;
    }
    let live = LIVE.with(|live| {
        let now = live.get().wrapping_add(bytes);
        live.set(now);
        now
    });
    // The core's published balance, which admission sums across cores
    // (`gauge`'s module doc). One compare here, and a publish only once the
    // balance has moved a whole step.
    if gauge::far_from(live, &gauge::LIVE_MARK) {
        gauge::report(bytes > 0);
    }
    if bytes > 0 {
        PEAK.with(|peak| {
            if live > peak.get() {
                peak.set(live);
            }
        });
        // `rule:errors/on-limit`'s memory ceiling, asked here because this is
        // the one place a growing allocation passes: a loop growing a string
        // through the ctx-less primitives reaches no other question until it
        // calls something. A release is deliberately outside this branch —
        // bytes given back cannot cross a ceiling, exactly as they cannot raise
        // the mark above.
        let ceiling = CEILING.with(Cell::get);
        if ceiling != 0 && live > ceiling {
            publish();
        }
    }
}

/// The size a layout charges, saturating rather than wrapping.
#[cfg(not(test))]
fn charge(bytes: usize) -> isize {
    isize::try_from(bytes).unwrap_or(isize::MAX)
}

/// `Backing`, plus the per-thread byte count above — the global allocator every
/// build that is not a test build registers.
#[cfg(not(test))]
#[derive(Debug)]
pub(crate) struct Accounting;

#[cfg(not(test))]
#[expect(
    unsafe_code,
    reason = "a global allocator's contract is inherently unsafe to implement; \
              every method below forwards to `Backing` unchanged and only adds \
              arithmetic on a thread-local Cell"
)]
unsafe impl GlobalAlloc for Accounting {
    #[expect(
        unsafe_code,
        reason = "the caller's `layout` obligations are forwarded to `Backing` \
                  verbatim"
    )]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
        let ptr = unsafe { Backing.alloc(layout) };
        if !ptr.is_null() {
            add(charge(layout.size()));
        }
        ptr
    }

    #[expect(
        unsafe_code,
        reason = "the caller's `ptr`/`layout` obligations are forwarded to \
                  `Backing` verbatim"
    )]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        add(-charge(layout.size()));
        #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
        unsafe {
            Backing.dealloc(ptr, layout);
        }
    }

    #[expect(
        unsafe_code,
        reason = "the caller's `layout` obligations are forwarded to `Backing` \
                  verbatim"
    )]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
        let ptr = unsafe { Backing.alloc_zeroed(layout) };
        if !ptr.is_null() {
            add(charge(layout.size()));
        }
        ptr
    }

    #[expect(
        unsafe_code,
        reason = "the caller's `ptr`/`layout`/`new_size` obligations are \
                  forwarded to `Backing` verbatim"
    )]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
        let fresh = unsafe { Backing.realloc(ptr, layout, new_size) };
        if !fresh.is_null() {
            add(charge(new_size));
            add(-charge(layout.size()));
        }
        fresh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_balance_returns_to_where_it_started() {
        let before = live_bytes();
        let held = vec![0_u8; 4096];
        assert!(live_bytes() >= before + 4096);
        drop(held);
        assert_eq!(live_bytes(), before);
    }

    /// The mark is what the balance cannot say: it survives the release.
    ///
    /// This is the whole case
    /// `rule:observability/a-memory-peak-is-recorded-not-asked-for` exists for
    /// — a spike deterministic release has already given back — so both halves
    /// are asserted together. A reader sampling [`live_bytes`] after the drop
    /// sees the baseline and would report a request that never grew.
    ///
    /// The mark is asserted against the *balance at the spike* and not against
    /// its own earlier reading: with no [`Ctx`](crate::Ctx) to rebase it, this
    /// is whatever the test thread has ever reached, which a fresh megabyte
    /// need not pass. [`Ctx::memory_peak`](crate::Ctx::memory_peak) is where
    /// the figure becomes a request's, and the case below is where the
    /// rebasing is asserted.
    #[test]
    fn the_mark_keeps_a_spike_the_balance_has_already_given_back() {
        let spike = 1 << 20;
        let baseline = live_bytes();
        let held = vec![0_u8; spike];
        let raised = live_bytes();
        assert!(
            raised >= baseline + isize::try_from(spike).expect("a megabyte fits"),
            "the balance did not record an allocation, so this asserts nothing"
        );
        let reached = peak_bytes();
        assert!(
            reached >= raised,
            "the mark is below a balance the allocator reached"
        );
        drop(held);
        assert!(
            live_bytes() < reached,
            "the release did not lower the balance, so this asserts nothing"
        );
        assert_eq!(peak_bytes(), reached, "the release lowered the mark");
    }

    /// A tree whose balance starts where this stack stands, so its first turn
    /// reads what the stack read.
    fn tree_here() -> std::sync::Arc<crate::TreeState> {
        let tree = std::sync::Arc::new(crate::TreeState::default());
        tree.balance
            .store(live_bytes(), std::sync::atomic::Ordering::Relaxed);
        tree
    }

    /// The module doc's *per request tree across switches*, without a
    /// scheduler. A stack that parks reads the balance and the ceiling it
    /// parked with, whatever a task of another tree allocated meanwhile. The
    /// stack that ran both still counts the neighbour's bytes.
    #[test]
    fn a_parked_stack_reads_its_own_balance_and_ceiling_when_it_resumes() {
        let neighbour_size = 2 << 20;
        let outside = ArmedStack::take();
        let outside_before = live_bytes();
        let tree = tree_here();
        begin_turn(&tree);
        arm(Armed::new(live_bytes() + (1 << 20), std::ptr::null()));
        let before = live_bytes();
        let ceiling = armed_ceiling();
        let parked = ArmedStack::take();
        let neighbour = vec![0_u8; neighbour_size];
        parked.resume();
        assert_eq!(
            live_bytes(),
            before,
            "a neighbour's bytes reached a parked stack"
        );
        assert_eq!(
            armed_ceiling(),
            ceiling,
            "the ceiling moved across the switch"
        );
        assert!(
            !over_ceiling(),
            "a neighbour's bytes took a parked stack past its own ceiling"
        );
        drop(ArmedStack::take());
        outside.restore();
        assert!(
            live_bytes() >= outside_before + isize::try_from(neighbour_size).expect("fits"),
            "the stack that ran both lost the neighbour's bytes"
        );
        drop(neighbour);
    }

    /// The other half of the same section: two stacks of one tree read one
    /// balance, so what one task holds counts against its sibling's ceiling.
    #[test]
    fn the_tasks_of_one_tree_read_one_balance() {
        let sibling_size: isize = 2 << 20;
        let outside = ArmedStack::take();
        let tree = tree_here();
        begin_turn(&tree);
        let before = live_bytes();
        let first = ArmedStack::take();
        begin_turn(&tree);
        let sibling = vec![0_u8; 2 << 20];
        drop(ArmedStack::take());
        first.resume();
        assert!(
            live_bytes() >= before + sibling_size,
            "a task did not see the bytes its sibling holds: {before} then {}",
            live_bytes()
        );
        drop(ArmedStack::take());
        outside.restore();
        drop(sibling);
    }

    /// A context created inside another measures its own allocation, and gives
    /// the enclosing mark back no smaller than it found it.
    ///
    /// Both directions are the rule's *nesting saves and restores*: without the
    /// rebase the inner reading would inherit the outer's spike, and without
    /// the `max` on drop the outer's would come back as whatever the inner
    /// happened to reach.
    #[test]
    fn a_nested_context_measures_its_own_allocation_and_restores_what_it_displaced() {
        let spike = 1 << 20;
        let outer = crate::Ctx::new(crate::OutputSink::Sink);
        let held = vec![0_u8; spike];
        let reached = outer.memory_peak();
        assert!(
            reached >= spike,
            "the outer context did not see its own spike"
        );
        {
            let inner = crate::Ctx::new(crate::OutputSink::Sink);
            assert!(
                inner.memory_peak() < spike,
                "a nested context inherited the mark of the one that made it"
            );
        }
        assert!(
            outer.memory_peak() >= reached,
            "the nested context erased the peak of the one that made it"
        );
        drop(held);
    }

    /// The `ceiling-backstop` check's cases: what a request past its ceiling
    /// reports, here, and what a debug build does to a member that grows past
    /// it inside one call, in `abi.rs`.
    mod past_its_ceiling {
        /// The poll comes after the growth. A request that grew far past its
        /// ceiling and gave most of it back before the poll reports what it
        /// held at the top, because that is what the ceiling was for.
        #[test]
        fn a_breach_reports_the_peak_the_request_held_not_the_balance_at_the_poll() {
            let ceiling = 1 << 20;
            let spike = 16 << 20;
            let mut ctx = crate::Ctx::buffered();
            ctx.set_memory_limit(ceiling);
            let kept = vec![1_u8; 2 << 20];
            drop(vec![1_u8; spike]);
            assert!(
                ctx.memory_used() < spike,
                "the spike was not given back, so this asserts nothing"
            );
            let Some(crate::Fault::Fatal(message)) = ctx.memory_breach() else {
                panic!("a request holding twice its ceiling was not stopped");
            };
            let held: usize = message
                .split(" bytes held")
                .next()
                .and_then(|head| head.rsplit(' ').next())
                .and_then(|figure| figure.parse().ok())
                .unwrap_or_else(|| panic!("the message names no reading: {message}"));
            assert!(
                held >= spike,
                "the message reads {held} bytes, the balance at the poll, and not the {spike}-byte peak: {message}"
            );
            drop(kept);
        }
    }

    /// Stage 3's two growth seams under a 1 MiB ceiling. Each is asked to
    /// grow to 64 MiB in one call, stops near its ceiling, and records the
    /// breach the request's next poll reports.
    mod a_buffer_stops_at_its_ceiling {
        use crate::budget::{ByteWriter, Stopped};

        const CEILING: usize = 1 << 20;
        const ASKED: usize = 64 << 20;

        #[test]
        fn an_array_a_producer_fills_stops_growing_at_the_ceiling() {
            let mut ctx = crate::Ctx::buffered();
            ctx.set_memory_limit(CEILING);
            let mut out = crate::NvsArray::new();
            let entries = ASKED / size_of::<crate::Value>();
            for at in 0..entries {
                out.append(crate::Value::int(i64::try_from(at).expect("fits")));
            }
            let kept = out.count() * size_of::<crate::Value>();
            assert!(
                kept <= CEILING,
                "the array kept {kept} bytes of entries under a {CEILING}-byte ceiling"
            );
            assert!(
                ctx.memory_peak() <= CEILING + CEILING / 2,
                "the array's growth peaked at {} bytes",
                ctx.memory_peak()
            );
            assert!(
                matches!(ctx.memory_breach(), Some(crate::Fault::Fatal(_))),
                "the refused growth recorded no breach"
            );
            drop(out);
        }

        #[test]
        fn a_byte_writer_stops_writing_at_the_ceiling() {
            let mut ctx = crate::Ctx::buffered();
            ctx.set_memory_limit(CEILING);
            let mut out = ByteWriter::new();
            let chunk = [7_u8; 4096];
            for _ in 0..ASKED / chunk.len() {
                out.put(&chunk);
            }
            assert_eq!(out.stopped(), Some(Stopped::Refused));
            assert!(
                out.len() <= CEILING,
                "the writer kept {} bytes under a {CEILING}-byte ceiling",
                out.len()
            );
            assert!(
                ctx.memory_peak() <= CEILING + CEILING / 2,
                "the writer's growth peaked at {} bytes",
                ctx.memory_peak()
            );
            assert!(
                matches!(ctx.memory_breach(), Some(crate::Fault::Fatal(_))),
                "the refused growth recorded no breach"
            );
            assert_eq!(out.finish(), Err(Stopped::Refused));
        }
    }
}
