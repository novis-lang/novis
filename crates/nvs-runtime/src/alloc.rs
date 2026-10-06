//! Novis's own allocator: a per-thread cache of small blocks in front of
//! [`System`].
//!
//! A small allocation's round trip to the platform heap is the dominant cost
//! of making it — [`docs/perf/userland-gap.md`](/docs/perf/userland-gap.md) § A
//! holds that measurement and the case-by-case attribution behind it. A string,
//! an array header and a small object are all in that size range, so the
//! platform heap is on the request path several times per statement, and
//! fronting it with a free list is the largest single move on the userland
//! suite's median that this tree has measured.
//!
//! [`docs/plan/design.md`](/docs/plan/design.md) § *Per-request isolation* is
//! the home of the decision that a request allocates from an arena of its own;
//! this is the half of it that needs no per-request accounting and no
//! [`Ctx`](crate::Ctx). The `[limits.hard]` ceiling attaches here at M6, where
//! [`affordable`](crate::affordable)'s own doc comment says it does.
//!
//! # What it spends
//!
//! Per `rule:programs/memory-priority`'s *Say what
//! you spend*: a bounded per-**thread** cache of freed blocks — 16 classes of
//! 16 bytes up to 256, each holding at most 512 blocks, so at most ~2 MB on a
//! thread that has touched every class. It is never per request and never
//! grows with requests served, which is the O(in-flight) rule AGENTS.md's
//! priority ordering states. Blocks are never returned to the platform on
//! thread exit, because the cache holds no `Drop` type (see below); a thread
//! that ends hands its ~2 MB back to the OS with the rest of the process.
//!
//! # The one trap
//!
//! The `thread_local!` below **must** be `const`-initialized and hold no `Drop`
//! type. A lazily-initialized thread local allocates its own state, and one
//! with a destructor registers that destructor — both from inside the
//! allocator, which is the recursion [`counting_alloc`](crate::counting_alloc)'s
//! own header describes. `Cell<*mut u8>` and `Cell<u32>` are neither.
//!
//! # Why every cached block carries one canonical layout
//!
//! A block is allocated from [`System`] with its **class's** layout — the full
//! class size, aligned to 16 — rather than the caller's. That is what makes
//! recycling sound: a popped block satisfies any request in its class, and the
//! layout handed back to [`System`] when the cache is full is the one it was
//! allocated with, which [`GlobalAlloc`] requires. It is also why an alignment
//! over 16 is never pooled.
//!
//! # Why it is registered only in optimized builds
//!
//! `crates/nvs-runtime/src/lib.rs` registers this as the `#[global_allocator]`
//! for non-test builds **that have `debug_assertions` off**. A recycled block
//! is a block valgrind never sees freed, so pooling in a debug build would
//! blunt exactly the tool this repository checks its refcount protocol with —
//! `tools/leak-check.sh` and `tools/wsl-acceptance.sh`'s memory leg both build
//! `-p nvs-cli` in debug, and the playbook's "exit 127 is a double release"
//! signal is the same instrument. A leak is unaffected either way (a value
//! that is never released never enters the cache, so it is still *definitely
//! lost*), but a use-after-free inside a recycled block is not, and security
//! outranks latency. The gain above is a release-build effect, the guard
//! that holds it (`benches/abi-probe/tests/perf_guards.rs`) is a
//! release-only test, and the `Pooled` type itself always pools, so this
//! module's own tests exercise the free list in either profile. A `cfg(test)`
//! build goes further: [`counting_alloc::Counting`](crate::counting_alloc) is
//! registered there and forwards to this allocator, so every guard that
//! measures bytes measures the shape the release build has. The counters sit
//! outside the cache, so a leak is still a block that never reaches `dealloc`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::ptr;

/// Bytes between one size class and the next.
const CLASS_STEP: usize = 16;

/// How many size classes the cache holds.
const CLASS_COUNT: usize = 16;

/// The largest allocation the cache serves.
const MAX_POOLED: usize = CLASS_STEP * CLASS_COUNT;

/// The alignment every cached block is allocated with, and therefore the
/// largest alignment a request can carry and still be pooled.
const MAX_ALIGN: usize = 16;

/// How many freed blocks one class keeps before handing the rest back to
/// [`System`].
const CLASS_CAPACITY: u32 = 512;

/// One class's free list: an intrusive singly-linked list whose next pointer
/// lives in the first word of each free block, plus its length.
///
/// The length is not derivable from the list without walking it, and the cap
/// is checked on every `dealloc`, so it is stored rather than counted.
struct Class {
    head: Cell<*mut u8>,
    len: Cell<u32>,
}

thread_local! {
    /// The calling thread's cache. `const`-initialized and `Drop`-free, per the
    /// module doc's *The one trap*.
    ///
    /// The empty class is an inline `const` block rather than a named constant:
    /// a `Class` holds `Cell`s, and a named constant with interior mutability
    /// is copied at each use, which is what `clippy::declare_interior_mutable_
    /// const` refuses.
    static CACHE: [Class; CLASS_COUNT] = const {
        [const {
            Class {
                head: Cell::new(ptr::null_mut()),
                len: Cell::new(0),
            }
        }; CLASS_COUNT]
    };
}

/// The class that serves `layout`, or `None` when the platform heap does.
///
/// A zero-sized layout never reaches a global allocator, but answering `None`
/// for one keeps the `size - 1` below out of reach of an underflow rather than
/// relying on that.
fn class_of(layout: Layout) -> Option<usize> {
    let size = layout.size();
    if size == 0 || size > MAX_POOLED || layout.align() > MAX_ALIGN {
        return None;
    }
    Some((size - 1) / CLASS_STEP)
}

/// The layout every block of `class` is allocated from [`System`] with.
fn block_layout(class: usize) -> Layout {
    let size = (class + 1) * CLASS_STEP;
    debug_assert!(class < CLASS_COUNT && size <= MAX_POOLED);
    #[expect(
        unsafe_code,
        reason = "the size is a non-zero multiple of 16 at most 256 and the \
                  alignment is a power of two, so the checked constructor's \
                  two failure modes cannot arise -- and a panic raised from \
                  inside the allocator would have nowhere to unwind to"
    )]
    unsafe {
        Layout::from_size_align_unchecked(size, MAX_ALIGN)
    }
}

/// Takes a block off `class`'s free list, or null when it is empty.
fn pop(class: usize) -> *mut u8 {
    CACHE.with(|cache| {
        let entry = &cache[class];
        let head = entry.head.get();
        if head.is_null() {
            return head;
        }
        #[expect(
            unsafe_code,
            reason = "every block on this list was pushed by `push` below, so \
                      it is at least 16 bytes and 16-aligned, and its first \
                      word holds the next pointer written there"
        )]
        let next = unsafe { ptr::read(head.cast::<*mut u8>()) };
        entry.head.set(next);
        entry.len.set(entry.len.get().saturating_sub(1));
        head
    })
}

/// Puts `block` on `class`'s free list, or answers `false` when it is full.
///
/// # Safety
///
/// `block` must be a live allocation made with `block_layout(class)` that the
/// caller is giving up ownership of.
#[expect(
    unsafe_code,
    reason = "the caller's ownership of `block` is what makes writing the next \
              pointer into its first word sound"
)]
unsafe fn push(class: usize, block: *mut u8) -> bool {
    CACHE.with(|cache| {
        let entry = &cache[class];
        if entry.len.get() >= CLASS_CAPACITY {
            return false;
        }
        #[expect(
            unsafe_code,
            reason = "a block of any class is at least 16 bytes and 16-aligned, \
                      so its first word holds a pointer"
        )]
        unsafe {
            ptr::write(block.cast::<*mut u8>(), entry.head.get());
        }
        entry.head.set(block);
        entry.len.set(entry.len.get() + 1);
        true
    })
}

/// How many freed blocks the calling thread's cache holds in the class that
/// serves `layout`, or `0` for a layout no class serves.
///
/// Reads the length `push` and `pop` already keep, so it costs the allocation
/// path nothing. [`crate::budget::pooled_blocks`] is its public face.
pub(crate) fn cached_blocks(layout: Layout) -> u32 {
    class_of(layout).map_or(0, |class| CACHE.with(|cache| cache[class].len.get()))
}

/// [`System`], with the per-thread size-class cache above in front of it.
#[derive(Debug)]
pub(crate) struct Pooled;

#[expect(
    unsafe_code,
    reason = "a global allocator's contract is inherently unsafe to implement; \
              every method below either forwards to `System` unchanged or \
              serves the request from a block of the same class this allocator \
              itself took from `System`"
)]
unsafe impl GlobalAlloc for Pooled {
    #[expect(
        unsafe_code,
        reason = "the caller's `layout` obligations are forwarded to `System` \
                  verbatim, or met by a cached block of `layout`'s own class"
    )]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let Some(class) = class_of(layout) else {
            #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
            return unsafe { System.alloc(layout) };
        };
        let cached = pop(class);
        if !cached.is_null() {
            return cached;
        }
        #[expect(
            unsafe_code,
            reason = "the class layout is at least as large and as aligned as \
                      the caller's, and is the layout `dealloc` will hand back"
        )]
        unsafe {
            System.alloc(block_layout(class))
        }
    }

    #[expect(
        unsafe_code,
        reason = "`ptr` came from `alloc` above with this same `layout`, so it \
                  is either a block of `layout`'s class or a `System` \
                  allocation of `layout` itself"
    )]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let Some(class) = class_of(layout) else {
            #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
            unsafe {
                System.dealloc(ptr, layout);
            }
            return;
        };
        #[expect(
            unsafe_code,
            reason = "the caller has given up `ptr`, and it is a block of this \
                      class because `alloc` allocated it from `block_layout`"
        )]
        let cached = unsafe { push(class, ptr) };
        if !cached {
            #[expect(
                unsafe_code,
                reason = "the class layout is the one this block was allocated \
                          with, which is what `System` requires"
            )]
            unsafe {
                System.dealloc(ptr, block_layout(class));
            }
        }
    }

    #[expect(
        unsafe_code,
        reason = "a cached block carries whatever the last owner left in it, so \
                  the zeroing below is not optional; the uncached path forwards \
                  the caller's contract verbatim"
    )]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let Some(class) = class_of(layout) else {
            #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
            return unsafe { System.alloc_zeroed(layout) };
        };
        let cached = pop(class);
        if !cached.is_null() {
            #[expect(
                unsafe_code,
                reason = "the block is at least `layout.size()` bytes, being of \
                          `layout`'s own class"
            )]
            unsafe {
                ptr::write_bytes(cached, 0, layout.size());
            }
            return cached;
        }
        #[expect(
            unsafe_code,
            reason = "the class layout is at least as large and as aligned as \
                      the caller's, and is the layout `dealloc` will hand back"
        )]
        unsafe {
            System.alloc_zeroed(block_layout(class))
        }
    }

    #[expect(
        unsafe_code,
        reason = "`System::realloc` may only be handed a pointer `System` \
                  itself allocated with the layout passed, which is true only \
                  when neither the old nor the new size is pooled; every other \
                  case goes through this allocator's own `alloc`/`dealloc`"
    )]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let Ok(fresh_layout) = Layout::from_size_align(new_size, layout.align()) else {
            return ptr::null_mut();
        };
        let old = class_of(layout);
        let new = class_of(fresh_layout);
        if old.is_none() && new.is_none() {
            #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
            return unsafe { System.realloc(ptr, layout, new_size) };
        }
        if old.is_some() && old == new {
            // The block already spans the whole class, so a resize inside one
            // is the identity — and this is the case `$s .= $piece` hits.
            return ptr;
        }
        #[expect(
            unsafe_code,
            reason = "the grow-and-copy fallback `GlobalAlloc` documents, with \
                      both halves going through this allocator"
        )]
        let fresh = unsafe { self.alloc(fresh_layout) };
        if !fresh.is_null() {
            #[expect(
                unsafe_code,
                reason = "both blocks are live, distinct, and at least the \
                          copied length"
            )]
            unsafe {
                ptr::copy_nonoverlapping(ptr, fresh, layout.size().min(new_size));
                self.dealloc(ptr, layout);
            }
        }
        fresh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many blocks `class` is currently caching.
    fn cached(class: usize) -> u32 {
        CACHE.with(|cache| cache[class].len.get())
    }

    /// Allocates and immediately frees one block, answering its address — two
    /// round trips of one class answer the same address iff the cache served
    /// the second.
    fn round_trip(layout: Layout) -> *mut u8 {
        #[expect(unsafe_code, reason = "the layout is non-zero-sized")]
        let block = unsafe { Pooled.alloc(layout) };
        #[expect(unsafe_code, reason = "the block was just allocated with it")]
        unsafe {
            Pooled.dealloc(block, layout);
        }
        block
    }

    fn layout(size: usize) -> Layout {
        Layout::from_size_align(size, 8).expect("a valid test layout")
    }

    #[test]
    fn a_freed_block_serves_the_next_request_of_its_class() {
        let first = round_trip(layout(32));
        let second = round_trip(layout(32));
        assert_eq!(first, second, "the class's free list was not consulted");
    }

    #[test]
    fn a_class_is_chosen_by_size_and_alignment() {
        assert_eq!(class_of(layout(1)), Some(0));
        assert_eq!(class_of(layout(16)), Some(0));
        assert_eq!(class_of(layout(17)), Some(1));
        assert_eq!(class_of(layout(MAX_POOLED)), Some(CLASS_COUNT - 1));
        assert_eq!(class_of(layout(MAX_POOLED + 1)), None);
        assert_eq!(class_of(layout(0)), None);
        let over_aligned = Layout::from_size_align(32, MAX_ALIGN * 2).expect("a valid layout");
        assert_eq!(class_of(over_aligned), None);
    }

    #[test]
    fn a_full_class_hands_the_rest_back() {
        // The last class is the largest, and nothing else in this test binary
        // allocates `MAX_POOLED` bytes often enough to move the count under us.
        const CLASS: usize = CLASS_COUNT - 1;
        let held: Vec<*mut u8> = (0..CLASS_CAPACITY + 8)
            .map(|_| {
                #[expect(unsafe_code, reason = "freed in the loop below")]
                unsafe {
                    Pooled.alloc(layout(MAX_POOLED))
                }
            })
            .collect();
        for block in held {
            #[expect(unsafe_code, reason = "allocated with this layout above")]
            unsafe {
                Pooled.dealloc(block, layout(MAX_POOLED));
            }
        }
        assert_eq!(
            cached(CLASS),
            CLASS_CAPACITY,
            "the cache grew past its cap, so it is not O(in-flight)"
        );
    }

    #[test]
    fn a_resize_inside_one_class_keeps_the_block() {
        #[expect(unsafe_code, reason = "freed at the end of the test")]
        let block = unsafe { Pooled.alloc(layout(20)) };
        #[expect(
            unsafe_code,
            reason = "the block is live and was allocated with `layout(20)`"
        )]
        let grown = unsafe { Pooled.realloc(block, layout(20), 30) };
        assert_eq!(block, grown, "a resize inside class 1 reallocated");
        #[expect(unsafe_code, reason = "the block now carries the 30-byte layout")]
        unsafe {
            Pooled.dealloc(grown, layout(30));
        }
    }

    #[test]
    fn a_resize_across_classes_copies() {
        #[expect(unsafe_code, reason = "freed at the end of the test")]
        let block = unsafe { Pooled.alloc_zeroed(layout(16)) };
        #[expect(unsafe_code, reason = "the block is live and 16 bytes")]
        unsafe {
            ptr::write_bytes(block, 0xAB, 16);
        }
        #[expect(
            unsafe_code,
            reason = "the block is live and was allocated with `layout(16)`"
        )]
        let grown = unsafe { Pooled.realloc(block, layout(16), 200) };
        assert!(!grown.is_null());
        #[expect(unsafe_code, reason = "the copy covers the first 16 bytes")]
        let kept = unsafe { ptr::read(grown) };
        assert_eq!(kept, 0xAB, "the grow-and-copy fallback lost the contents");
        #[expect(unsafe_code, reason = "the block now carries the 200-byte layout")]
        unsafe {
            Pooled.dealloc(grown, layout(200));
        }
    }

    #[test]
    fn a_cached_block_is_handed_back_zeroed() {
        #[expect(unsafe_code, reason = "freed on the next line")]
        let dirty = unsafe { Pooled.alloc(layout(48)) };
        #[expect(unsafe_code, reason = "the block is live and 48 bytes")]
        unsafe {
            ptr::write_bytes(dirty, 0xFF, 48);
            Pooled.dealloc(dirty, layout(48));
        }
        #[expect(unsafe_code, reason = "freed at the end of the test")]
        let fresh = unsafe { Pooled.alloc_zeroed(layout(48)) };
        assert_eq!(fresh, dirty, "the free list was not consulted");
        #[expect(unsafe_code, reason = "the block is live and 48 bytes")]
        let bytes = unsafe { std::slice::from_raw_parts(fresh, 48) };
        assert!(
            bytes.iter().all(|byte| *byte == 0),
            "a recycled block reached `alloc_zeroed`'s caller unzeroed"
        );
        #[expect(unsafe_code, reason = "allocated with this layout above")]
        unsafe {
            Pooled.dealloc(fresh, layout(48));
        }
    }
}
