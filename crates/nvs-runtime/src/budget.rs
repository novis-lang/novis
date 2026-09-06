//! The per-request resource budget: a live-byte counter and an output-byte
//! counter the runtime keeps in every build, and the `[limits] memory` and
//! `[limits] max_output` ceilings a request is measured against.
//!
//! `rule:errors/on-limit` names
//! memory as the first of the five resource limits whose breach is a `FATAL`,
//! and [`crate::affordable`]'s own doc comment already says why the count lives
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
//! else, so `Counting` feeds `add` rather than keeping a third count: a test
//! build registers `Counting` as its global allocator, so without that call the
//! very builds the unit tests run in would be the only ones not counting.
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
//! [ADR 0006](/docs/adr/0006-isolated-script-execution.md)'s isolate boundary will
//! sharpen when a core runs several.
//!
//! A block allocated on one thread and freed on another makes the freeing
//! thread's balance go negative, which is why the counter is signed: it stays a
//! readable number rather than a huge one, and a request's *used* figure
//! saturates at zero rather than wrapping.
//!
//! [`written_bytes`] is that same arrangement for `[limits] max_output`, and it
//! is here rather than as a field on [`Ctx`](crate::Ctx) for the one property a
//! per-context field could not have. An isolate writes on the thread that
//! spawned it, so a thread-local count puts a child's bytes on the root's
//! reading as well as on the child's own — which is what
//! [ADR 0006](/docs/adr/0006-isolated-script-execution.md) already says
//! the directive means, and what makes the tree's output budget a budget for
//! the tree rather than one per isolate. Monotonic where [`live_bytes`] is a
//! balance, because bytes written to a response are never given back, so it
//! needs no sign.
//!
//! # Where the breach is noticed — and the one shape it does not reach yet
//!
//! Counting is universal; *noticing* is not, and the difference is worth
//! knowing before trusting the cap. Two places ask
//! [`Ctx::memory_breach`](crate::Ctx::memory_breach): [`crate::run_helper`], on
//! the way in to every `Core` member, and [`crate::nvs_safepoint`], the poll
//! compiled code makes between statements. Together those bound any program
//! that calls anything.
//!
//! **Known gap.** A compiled loop that allocates only through the ctx-less
//! runtime helpers — `nvs_str_concat`, `nvs_array_append` — reaches neither
//! until it next calls a member, because the safepoint's *fast* path branches
//! on `Ctx`'s flags word and nothing sets that word for a breach. Closing it
//! means giving the allocator a way to publish into that word, and the two
//! candidates are a thread-local `*const AtomicU64` armed for the running
//! request, and a poll word compiled code reads out of thread-local storage.
//! The first is the small change and it is the one with a real question under
//! it: the allocator runs on the same thread as the `&mut Ctx` that reborrows
//! that word, so the stashed pointer is not obviously sound in the way
//! [`Ctx`](crate::Ctx)'s cross-thread `deadline` is. That question is why the
//! gap is recorded here rather than closed in passing.
//!
//! # What it spends
//!
//! Per `rule:programs/memory-priority`'s *say what
//! you spend*: three words per thread — never per request, and never growing
//! with requests served — and on the allocation path one thread-local
//! read-modify-write per `dealloc` and three per `alloc`. Each is a
//! register-relative load, an add and a store against a `const`-initialized
//! cell, a few instructions in front of an allocation that costs tens of
//! nanoseconds even out of the pool. It is bought deliberately: AGENTS.md's
//! priority ordering puts request isolation above latency, and a cap nothing
//! counts against is not a cap.

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
    /// Monotonic: how many allocation requests this thread has made.
    static REQUESTS: Cell<usize> = const { Cell::new(0) };
    /// Monotonic: how many bytes those requests asked for.
    static TOTAL: Cell<usize> = const { Cell::new(0) };
    /// Monotonic: how many bytes this thread has written to a request's output
    /// sink — the module doc's second counter.
    static WRITTEN: Cell<usize> = const { Cell::new(0) };
}

/// How many bytes this thread has allocated and not yet freed.
///
/// The absolute balance, not a request's share — [`Ctx::memory_used`](crate::Ctx::memory_used)
/// is the per-request reading and the module doc says how the two relate.
#[must_use]
pub fn live_bytes() -> isize {
    LIVE.with(Cell::get)
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
/// One caller — [`Ctx::write_output`](crate::Ctx::write_output), at the point
/// the bytes reach the sink rather than at the point a program hands them over,
/// which is the distinction that doc comment owns. Saturating, because a count
/// that wrapped would answer "under the ceiling" for the one request that most
/// certainly is not.
pub(crate) fn wrote(bytes: usize) {
    WRITTEN.with(|written| written.set(written.get().saturating_add(bytes)));
}

/// Charges `bytes` to this thread's counters — negative for a release.
///
/// `pub(crate)` and called from exactly two places: [`Accounting`] below, and
/// `counting_alloc`'s own arithmetic in a test build. A positive delta is one
/// allocation *request*, which is why the three counters move together here
/// rather than at four call sites each.
pub(crate) fn add(bytes: isize) {
    LIVE.with(|live| live.set(live.get().wrapping_add(bytes)));
    if bytes > 0 {
        REQUESTS.with(|count| count.set(count.get().wrapping_add(1)));
        let grew = usize::try_from(bytes).unwrap_or(0);
        TOTAL.with(|total| total.set(total.get().wrapping_add(grew)));
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
}
