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
//! [`Ctx::memory_used`](crate::Ctx::memory_used) — which is exact for the one
//! request a thread runs at a time and is what
//! `rule:security/isolate-shares-nothing`'s isolate boundary will
//! sharpen when a core runs several.
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
//! per-request provenance, which is what the request arena in `docs/plan/m6.md`
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
//! and a store against a `const`-initialized cell, a few instructions in front
//! of an allocation that costs far more than they do even out of the pool. It
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

#[cfg(not(test))]
use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;

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
    /// pass that computes the ceiling and [`arm`] the one writer of this.
    static CEILING: Cell<isize> = const { Cell::new(0) };
    /// The safepoint word the request that armed [`CEILING`] polls, or null
    /// where nothing is armed.
    ///
    /// Read by an allocation that has already crossed the ceiling and by
    /// nothing else, which is why it is a second cell rather than a field
    /// beside the threshold: the ordinary path loads [`CEILING`] alone.
    static POLLED: Cell<*const std::sync::atomic::AtomicU64> =
        const { Cell::new(std::ptr::null()) };
    /// Whether an allocation this thread's running request asked for was
    /// refused — [`refuse`]'s verdict, which no counter can be read for.
    ///
    /// Per thread because the allocators that hit a refusal hold no
    /// [`Ctx`](crate::Ctx) to record it on, and confined to one request because
    /// the context that is refused displaces this cell exactly as it displaces
    /// [`CEILING`].
    static REFUSED: Cell<bool> = const { Cell::new(false) };
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
        let _ = self.output.fetch_update(
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
    /// what a request under no ceiling arms, and the state a thread running no
    /// request is left in.
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
/// allocation until something arms over it. [`Ctx`](crate::Ctx) discharges that
/// by holding the pair it displaced in a field and arming it again as it drops,
/// which is [`rebase_peak`]'s arrangement applied to a second number — so the
/// address armed always belongs to a context that is still running.
pub(crate) fn arm(next: Armed) {
    CEILING.with(|ceiling| ceiling.set(next.ceiling));
    POLLED.with(|polled| polled.set(next.word));
}

/// Arms `next` and hands back the pair it displaced.
///
/// [`Ctx::new`](crate::Ctx)'s half of the nesting above: a context made inside
/// another takes the thread's arming with its own and carries the enclosing
/// pair until it drops.
pub(crate) fn displace(next: Armed) -> Armed {
    let displaced = Armed {
        ceiling: CEILING.with(Cell::get),
        word: POLLED.with(Cell::get),
    };
    arm(next);
    displaced
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
/// [`affords`] that justifies it.
fn refuse() {
    REFUSED.with(|refused| refused.set(true));
    publish();
}

/// Whether the running request has been refused an allocation.
///
/// Sticky: the request that is refused is over, and nothing it does afterwards
/// brings it back under a ceiling it never held the bytes against.
/// [`take_refusal`] is the only way back, and its two callers are the context
/// that ends and the handler that is lent a slice to report with.
pub(crate) fn refused() -> bool {
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
pub(crate) fn take_refusal() -> bool {
    REFUSED.with(|cell| cell.replace(false))
}

/// Puts back what [`take_refusal`] handed out.
pub(crate) fn restore_refusal(refused: bool) {
    REFUSED.with(|cell| cell.set(refused));
}

/// How many bytes this thread has allocated and not yet freed.
///
/// The absolute balance, not a request's share — [`Ctx::memory_used`](crate::Ctx::memory_used)
/// is the per-request reading and the module doc says how the two relate.
#[must_use]
pub fn live_bytes() -> isize {
    LIVE.with(Cell::get)
}

/// How many bytes this thread has allocated inside a [`Detached`] bracket and
/// not yet given back inside one.
///
/// The process's share of what the thread holds, which [`live_bytes`] and every
/// per-request reading derived from it exclude on purpose. It is a figure to
/// attribute bytes with rather than a ceiling to enforce: what bounds a store's
/// entries is the store's own limit, the local cache tier's `max_size` for
/// `Core\Cache`.
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
    PEAK.with(Cell::get)
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
}

/// Restores `enclosing` as the mark, keeping whichever of the two is higher.
///
/// The other half, run as a context drops: publishing `max(enclosing, reached)`
/// is what stops an isolate that allocated little from erasing the peak of the
/// request that spawned it.
pub(crate) fn publish_peak(enclosing: isize) {
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
#[must_use]
pub fn armed_ceiling() -> isize {
    CEILING.with(Cell::get)
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
    if LIVE.with(Cell::get).saturating_add(ask) <= ceiling {
        return true;
    }
    refuse();
    false
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
        DETACHED.with(|held| held.set(held.get().wrapping_add(bytes)));
        return;
    }
    let live = LIVE.with(|live| {
        let now = live.get().wrapping_add(bytes);
        live.set(now);
        now
    });
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
}
