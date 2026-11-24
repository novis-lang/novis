//! Novis's own allocator: a per-thread cache of small blocks in front of
//! [`System`].
//!
//! A small allocation's round trip to the platform heap is the dominant cost
//! of making it. A string, an array header and a small object are all in that
//! size range, so the platform heap is on the request path several times per
//! statement. `benches/abi-probe`'s
//! `an_allocation_round_trip_stays_in_the_pooled_cost_class` holds the cached
//! round trip below the platform heap's.
//!
//! `rule:security/arena-is-an-ownership-root` is the home of the decision
//! that a request allocates from an arena of its own;
//! this is the half of it that needs no per-request accounting and no
//! [`Ctx`](crate::Ctx). The per-request memory ceiling is enforced at
//! [`affordable`](crate::affordable), not here.
//!
//! # What it spends
//!
//! Per `rule:programs/memory-priority`'s *Say what
//! you spend*: a bounded per-**thread** cache of freed blocks — 16 classes of
//! 16 bytes up to 256, each holding at most 512 blocks, so at most 1,114,112
//! bytes (about 1.06 MiB) on a thread that has filled every class. It is
//! never per request and never grows with requests served, which is the
//! O(in-flight) rule AGENTS.md's priority ordering states. Blocks are not
//! returned to the platform on thread exit by themselves, because the cache
//! holds no `Drop` type (see
//! below). A thread that ends keeps its blocks until the process ends, unless
//! it calls [`release_cache`] first, through
//! [`crate::budget::release_pooled_blocks`] or
//! [`crate::budget::run_then_release_pooled_blocks`]. Every thread that ends
//! before the process does calls one of them, or the blocks of every such
//! thread it ever ran add up.
//!
//! The cache's free-list heads are 256 bytes of `budget`'s per-thread
//! storage, beside the memory counters, and add no lookup of their own: the
//! registered allocator looks that storage up once per allocation, free or
//! resize, and takes the cache and the counters from the same lookup.
//!
//! # The one trap
//!
//! The `thread_local!` that holds the cache (`budget`'s `THREAD`) **must** be
//! `const`-initialized and hold no `Drop` type. A lazily-initialized thread local allocates its own state, and one
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

/// One thread's cache: a free list per class.
///
/// It is a field of `budget`'s per-thread storage, beside the memory
/// counters, so the registered allocator looks the thread's storage up once
/// and hands this to the free lists and the counters to the arithmetic. That
/// thread-local's doc carries the module doc's *The one trap*.
/// [`budget::with_cache`](crate::budget::with_cache) is how [`Pooled`] and the
/// functions below reach it.
#[derive(Debug)]
pub(crate) struct Cache([Class; CLASS_COUNT]);

impl std::fmt::Debug for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Class")
            .field("len", &self.len.get())
            .finish_non_exhaustive()
    }
}

impl Cache {
    /// An empty cache, for a `const`-initialized thread-local.
    ///
    /// The empty class is an inline `const` block rather than a named
    /// constant: a `Class` holds `Cell`s, and a named constant with interior
    /// mutability is copied at each use, which is what
    /// `clippy::declare_interior_mutable_const` refuses.
    pub(crate) const fn new() -> Self {
        Self(
            [const {
                Class {
                    head: Cell::new(ptr::null_mut()),
                    len: Cell::new(0),
                }
            }; CLASS_COUNT],
        )
    }
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

impl Cache {
    /// Takes a block off `class`'s free list, or null when it is empty.
    #[inline]
    fn pop(&self, class: usize) -> *mut u8 {
        let entry = &self.0[class];
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
    #[inline]
    unsafe fn push(&self, class: usize, block: *mut u8) -> bool {
        let entry = &self.0[class];
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
    }
}

/// How many freed blocks the calling thread's cache holds in the class that
/// serves `layout`, or `0` for a layout no class serves.
///
/// Reads the length `push` and `pop` already keep, so it costs the allocation
/// path nothing. [`crate::budget::pooled_blocks`] is its public face.
pub(crate) fn cached_blocks(layout: Layout) -> u32 {
    class_of(layout).map_or(0, |class| {
        crate::budget::with_cache(|cache| cache.0[class].len.get())
    })
}

/// Hands every block the calling thread's cache keeps back to [`System`].
///
/// A thread that is about to end calls it, through
/// [`crate::budget::release_pooled_blocks`]. Without it the blocks of a thread
/// that ended stay allocated and no other thread can use them, because the
/// cache holds no `Drop` type (the module doc's *The one trap*).
pub(crate) fn release_cache() {
    crate::budget::with_cache(|cache| {
        for class in 0..CLASS_COUNT {
            loop {
                let block = cache.pop(class);
                if block.is_null() {
                    break;
                }
                #[expect(
                    unsafe_code,
                    reason = "every cached block of `class` was allocated with \
                              `block_layout(class)`, which is what `System` requires"
                )]
                unsafe {
                    System.dealloc(block, block_layout(class));
                }
            }
        }
    });
}

/// [`System`], with the calling thread's size-class cache in front of it.
///
/// Each method looks the thread's [`Cache`] up once and runs the cache's own
/// method on it. The registered allocator, `budget::Accounting`, takes the
/// cache from the same lookup as its counters and does not come through here.
#[derive(Debug)]
pub(crate) struct Pooled;

#[expect(
    unsafe_code,
    reason = "a global allocator's contract is inherently unsafe to implement; \
              every method below forwards to the calling thread's `Cache` \
              unchanged"
)]
unsafe impl GlobalAlloc for Pooled {
    #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        crate::budget::with_cache(|cache| unsafe { cache.alloc(layout) })
    }

    #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        crate::budget::with_cache(|cache| unsafe { cache.dealloc(ptr, layout) });
    }

    #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        crate::budget::with_cache(|cache| unsafe { cache.alloc_zeroed(layout) })
    }

    #[expect(unsafe_code, reason = "forwarding the caller's own contract")]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        crate::budget::with_cache(|cache| unsafe { cache.realloc(ptr, layout, new_size) })
    }
}

#[expect(
    unsafe_code,
    reason = "a global allocator's contract is inherently unsafe to implement; \
              every method below either forwards to `System` unchanged or \
              serves the request from a block of the same class this cache \
              itself took from `System`"
)]
unsafe impl GlobalAlloc for Cache {
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
        let cached = self.pop(class);
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
        let cached = unsafe { self.push(class, ptr) };
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
        let cached = self.pop(class);
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
        crate::budget::with_cache(|cache| cache.0[class].len.get())
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

    /// The best of five timed runs of `rounds` calls of `step`, in
    /// nanoseconds per call.
    fn best_ns(rounds: u32, step: &dyn Fn()) -> f64 {
        (0..5)
            .map(|_| {
                let start = std::time::Instant::now();
                for _ in 0..rounds {
                    step();
                }
                start.elapsed().as_secs_f64() * 1e9 / f64::from(rounds)
            })
            .fold(f64::INFINITY, f64::min)
    }

    /// A timing probe, not a guard: it prints to standard error the counted round trip the
    /// registered allocator makes next to the platform heap's, and asserts
    /// nothing. The goal that merged the cache's and the counters' thread
    /// locals ran it before and after, to catch a merge that got slower.
    /// Run it in a release build, by name, with `--ignored -- --nocapture`.
    #[test]
    #[ignore = "a timing probe: run it by name in a release build"]
    #[expect(
        clippy::print_stderr,
        reason = "a probe's figures are its output, and stdout is the language server's"
    )]
    fn the_counted_round_trip_is_timed_beside_the_platform_heap() {
        const ROUNDS: u32 = 2_000_000;
        let small = layout(48);
        let counted = best_ns(ROUNDS, &|| {
            #[expect(unsafe_code, reason = "freed on the next line, with its layout")]
            unsafe {
                let block = crate::budget::Accounting.alloc(std::hint::black_box(small));
                crate::budget::Accounting.dealloc(std::hint::black_box(block), small);
            }
        });
        let grown = best_ns(ROUNDS, &|| {
            #[expect(
                unsafe_code,
                reason = "grown across classes, then freed with the grown layout"
            )]
            unsafe {
                let block = crate::budget::Accounting.alloc(std::hint::black_box(layout(16)));
                let block = crate::budget::Accounting.realloc(block, layout(16), 200);
                crate::budget::Accounting.dealloc(std::hint::black_box(block), layout(200));
            }
        });
        let platform = best_ns(ROUNDS, &|| {
            #[expect(unsafe_code, reason = "freed on the next line, with its layout")]
            unsafe {
                let block = System.alloc(std::hint::black_box(small));
                System.dealloc(std::hint::black_box(block), small);
            }
        });
        eprintln!(
            "counted round trip {counted:.2} ns, counted grow {grown:.2} ns, \
             platform round trip {platform:.2} ns, ratio {:.2}",
            counted / platform
        );
    }

    #[test]
    fn a_released_cache_keeps_no_block() {
        let _ = round_trip(layout(48));
        let class = class_of(layout(48)).expect("48 bytes is pooled");
        assert!(cached(class) > 0, "the freed block was not cached");
        release_cache();
        assert!(
            (0..CLASS_COUNT).all(|class| cached(class) == 0),
            "a class kept a block after the cache was released"
        );
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
