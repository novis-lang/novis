//! A test-only global allocator that counts live — and, separately, total —
//! bytes on the calling thread, in front of the allocator MWL actually ships.
//!
//! It exists for one guard: `object::tests::an_acyclic_object_graph_releases_
//! every_allocation`, the leak check `docs/agent/loop-goal.md` Stage 5 names. A
//! hand-written refcount protocol is exactly where a leak hides, and asserting
//! that a refcount reached zero only proves the *bookkeeping* balanced — not
//! that the allocation was handed back. Measuring the allocator proves both.
//!
//! # What it wraps, and why the counts still mean what they did
//!
//! Every method below forwards to [`Pooled`](crate::alloc::Pooled) rather than
//! to the platform heap, so a test build measures the allocator an optimized
//! build actually runs on instead of the one it replaced. `Counting` sits
//! *outside* the size-class cache: a request served from a recycled block is
//! still exactly one `alloc` here, and returning that block to the cache is
//! still exactly one `dealloc`, so both counters carry the same meaning they
//! carried over [`System`](std::alloc::System) — [`live_bytes`] a balance and
//! [`allocated_bytes`] a monotonic total, both in *requested* bytes rather
//! than in the class-rounded block a request lands in. What changes is the
//! shape underneath them: a leak or a transient allocation is now caught
//! against the code path the release binary takes.
//!
//! # Why the counter is thread-local
//!
//! `cargo test` runs this crate's tests concurrently in one process, so a
//! process-wide counter would fold every other test's allocations into the
//! delta and produce a flaky assertion. Every allocation a test makes happens
//! on that test's own thread, so a thread-local counter is both exact and
//! immune to its neighbours. The cell is `const`-initialized and holds an
//! `isize`, so it needs no destructor — a `thread_local!` with one would
//! allocate from inside the allocator.
//!
//! A block allocated on one thread and freed on another makes the freeing
//! thread's count go negative. That is why the counter is signed rather than
//! wrapping: it stays a readable number instead of a huge one, and no guard
//! here spans threads.

use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;

use crate::alloc::Pooled;

thread_local! {
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static TOTAL: Cell<usize> = const { Cell::new(0) };
}

/// How many bytes this thread has allocated and not yet freed.
pub(crate) fn live_bytes() -> isize {
    LIVE.with(Cell::get)
}

/// How many bytes this thread has ever allocated, never decreasing.
///
/// [`live_bytes`] cannot see an allocation that is freed again before the
/// call under test returns, and a *transient* allocation is exactly what
/// `array::tests::an_integer_subscript_allocates_no_key` exists to catch: the
/// key-taking primitives build an `MwlStr` and drop it inside one call, so
/// their live delta is zero and their total delta is not.
pub(crate) fn allocated_bytes() -> usize {
    TOTAL.with(Cell::get)
}

fn add(bytes: isize) {
    LIVE.with(|live| live.set(live.get().wrapping_add(bytes)));
    if bytes > 0 {
        let grew = usize::try_from(bytes).unwrap_or(0);
        TOTAL.with(|total| total.set(total.get().wrapping_add(grew)));
    }
}

/// [`Pooled`], plus the per-thread byte count above.
#[derive(Debug)]
pub(crate) struct Counting;

#[expect(
    unsafe_code,
    reason = "a global allocator's contract is inherently unsafe to implement; \
              every method below forwards to `Pooled` unchanged and only adds \
              arithmetic on a thread-local Cell"
)]
unsafe impl GlobalAlloc for Counting {
    #[expect(
        unsafe_code,
        reason = "the caller's `layout` obligations are forwarded to `Pooled` \
                  verbatim"
    )]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
        let ptr = unsafe { Pooled.alloc(layout) };
        if !ptr.is_null() {
            add(isize::try_from(layout.size()).unwrap_or(isize::MAX));
        }
        ptr
    }

    #[expect(
        unsafe_code,
        reason = "the caller's `ptr`/`layout` obligations are forwarded to \
                  `Pooled` verbatim"
    )]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        add(-isize::try_from(layout.size()).unwrap_or(isize::MAX));
        #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
        unsafe {
            Pooled.dealloc(ptr, layout);
        }
    }

    #[expect(
        unsafe_code,
        reason = "the caller's `layout` obligations are forwarded to `Pooled` \
                  verbatim"
    )]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
        let ptr = unsafe { Pooled.alloc_zeroed(layout) };
        if !ptr.is_null() {
            add(isize::try_from(layout.size()).unwrap_or(isize::MAX));
        }
        ptr
    }

    #[expect(
        unsafe_code,
        reason = "the caller's `ptr`/`layout`/`new_size` obligations are \
                  forwarded to `Pooled` verbatim"
    )]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
        let fresh = unsafe { Pooled.realloc(ptr, layout, new_size) };
        if !fresh.is_null() {
            add(isize::try_from(new_size).unwrap_or(isize::MAX));
            add(-isize::try_from(layout.size()).unwrap_or(isize::MAX));
        }
        fresh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_count_returns_to_where_it_started() {
        let before = live_bytes();
        let held = vec![0_u8; 4096];
        assert!(live_bytes() >= before + 4096);
        drop(held);
        assert_eq!(live_bytes(), before);
    }

    #[test]
    fn the_total_counts_a_block_that_was_already_freed() {
        let before = allocated_bytes();
        drop(vec![0_u8; 4096]);
        assert!(allocated_bytes() >= before + 4096);
    }
}
