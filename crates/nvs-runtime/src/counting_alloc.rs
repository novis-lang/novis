//! A test-only global allocator that counts live — and, separately, total —
//! bytes on the calling thread, in front of the allocator Novis actually ships.
//!
//! It exists for one guard: `object::tests::an_acyclic_object_graph_releases_
//! every_allocation`, the runtime's leak check. A
//! hand-written refcount protocol is exactly where a leak hides, and asserting
//! that a refcount reached zero only proves the *bookkeeping* balanced — not
//! that the allocation was handed back. Measuring the allocator proves both.
//!
//! # What it wraps, and what the counts mean
//!
//! Every method below forwards to `Backing` — [`Pooled`](crate::alloc::Pooled)
//! normally — rather than to the platform heap, so a test build measures the
//! allocator an optimized build actually runs on. `Counting` sits *outside*
//! the size-class cache: a request served from a recycled block is exactly one
//! `alloc` here, and returning that block to the cache is exactly one
//! `dealloc`, so the counters mean what they say over
//! [`System`](std::alloc::System) and over the pool alike — [`live_bytes`] a
//! balance and [`allocated_bytes`] a monotonic total, both in *requested*
//! bytes rather than in the class-rounded block a request lands in. A leak or
//! a transient allocation is therefore caught against the code path the
//! release binary takes.
//!
//! # Why a sanitizer needs `Backing` to be the platform heap
//!
//! That recycling is invisible to a memory checker, and this crate's own test
//! binary is the one build where it matters. ASAN finds a use-after-free by
//! poisoning freed memory and holding it in quarantine, which it can only do
//! for a block that reaches `free`. A block Novis frees goes onto the size-class
//! cache instead, so ASAN never poisons it and a read through a dangling
//! pointer lands in live, legitimately-mapped memory and says nothing. That is
//! exactly the refcount bug the sanitizer is there for.
//!
//! Hence the crate's `sanitizer` feature: it swaps `Backing` for
//! [`System`](std::alloc::System) so every free is a real one. The counters
//! keep both their meanings — `Counting` sits outside either backing allocator
//! — so the leak guards above still assert what they assert; what is given up
//! while it is on is that they measure the platform heap rather than the code
//! path a release binary takes. The CI job named `asan` is the only caller.
//!
//! **No other leg needs it.** The valgrind sweep runs
//! `target/debug/nvs`, and a debug build is deliberately left on the platform
//! heap ([`crate`] § *the allocator itself*), so that sweep sees every free
//! already. `nvs-codegen` and `nvs-stdlib` link this crate with `cfg(test)`
//! off, which is the same story. `cfg(test)` here is the one place the pool is
//! installed under a checker.
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

#[cfg(not(feature = "sanitizer"))]
use crate::alloc::Pooled as Backing;
#[cfg(feature = "sanitizer")]
use std::alloc::System as Backing;

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
/// key-taking primitives build an `NvsStr` and drop it inside one call, so
/// their live delta is zero and their total delta is not.
pub(crate) fn allocated_bytes() -> usize {
    TOTAL.with(Cell::get)
}

fn add(bytes: isize) {
    // The request budget counts in every build, and a test build registers
    // `Counting` instead of `budget::Accounting` — so without this line the
    // one profile the unit tests run in would be the only one not counting.
    crate::budget::add(bytes);
    LIVE.with(|live| live.set(live.get().wrapping_add(bytes)));
    if bytes > 0 {
        let grew = usize::try_from(bytes).unwrap_or(0);
        TOTAL.with(|total| total.set(total.get().wrapping_add(grew)));
    }
}

/// `Backing`, plus the per-thread byte count above.
#[derive(Debug)]
pub(crate) struct Counting;

#[expect(
    unsafe_code,
    reason = "a global allocator's contract is inherently unsafe to implement; \
              every method below forwards to `Backing` unchanged and only adds \
              arithmetic on a thread-local Cell"
)]
unsafe impl GlobalAlloc for Counting {
    #[expect(
        unsafe_code,
        reason = "the caller's `layout` obligations are forwarded to `Backing` \
                  verbatim"
    )]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
        let ptr = unsafe { Backing.alloc(layout) };
        if !ptr.is_null() {
            add(isize::try_from(layout.size()).unwrap_or(isize::MAX));
        }
        ptr
    }

    #[expect(
        unsafe_code,
        reason = "the caller's `ptr`/`layout` obligations are forwarded to \
                  `Backing` verbatim"
    )]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        add(-isize::try_from(layout.size()).unwrap_or(isize::MAX));
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
            add(isize::try_from(layout.size()).unwrap_or(isize::MAX));
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
