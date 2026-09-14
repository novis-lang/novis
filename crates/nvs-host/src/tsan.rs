//! ThreadSanitizer's fiber annotations: how the sanitizer is told that the
//! stack beneath a thread has changed.
//!
//! A task is a stackful coroutine ([`crate::scheduler`]), so resuming one moves
//! the thread's stack pointer onto a mapping the sanitizer has never seen it
//! standing on. ThreadSanitizer keeps one history of accesses per *thread*, and
//! a stack is the region it expects to belong to exactly one such history; a
//! scheduler that switches stacks underneath it therefore produces a report at
//! every task boundary, because two tasks that never ran at the same instant —
//! touching the same recycled stack slot, or the same arena through a pointer
//! one of them parked on — are indistinguishable from two threads racing over
//! it. The sanitizer's own answer is a **fiber**: one history per stack, moved
//! from one to the next at the moment the stack actually moves.
//!
//! [`Fiber`] is a task's, made when its coroutine is built and retired with it;
//! the scheduler's own is whatever the thread already had, read back at each
//! switch rather than held. [`Fiber::around`] is the whole surface, and the
//! shape of it is the point — see § *Why it is one call and not two*. **A
//! task's own `suspend` needs no annotation**: it is the far end of the same
//! switch, and it reappears here as that resume returning.
//!
//! # Why it is one call and not two
//!
//! The obvious spelling is a `switch_to` before the resume and a `switch_back`
//! after it. It corrupts the sanitizer's heap. ThreadSanitizer instruments
//! every function it compiles with an entry and an exit that push and pop a
//! shadow stack belonging to *the current fiber*, so a function that is entered
//! on one fiber and returns on another pushes onto one and pops from the other
//! — and the first `switch_to` pops a fiber whose shadow stack is still empty.
//! The pointer goes below its buffer, the next entry writes outside it, and the
//! process dies in `__tsan_func_entry` some distance away from the annotation
//! that caused it.
//!
//! So the rule this module is built to keep is: **no function of ours returns
//! on a fiber other than the one it was entered on.** [`Fiber::around`] leaves
//! and comes back within one call, which makes it balanced whether or not it is
//! inlined; the raw `__tsan_*` calls inside it are the sanitizer runtime's own
//! and carry no instrumentation of their own. Nothing else here switches.
//!
//! # Why the switch synchronizes
//!
//! `__tsan_switch_to_fiber`'s flag word can suppress the happens-before edge a
//! switch would otherwise draw; this module never passes it. Two tasks on one
//! core are ordered by the scheduler resuming them one at a time, so everything
//! a task did before it suspended really does happen-before what the next one
//! does, and the edge states a fact about this runtime rather than quieting an
//! inconvenient one — `rule:concurrency/a-wake-never-moves-a-task` is why the
//! fact holds, and it is the same property that buys the non-atomic refcount.
//! Suppressing it would have the sanitizer report a core's own run queue.
//!
//! # What is compiled, and what it spends
//!
//! The gate is this crate's `tsan` feature, which `tools/tsan.sh` turns on and
//! nothing else does, so the `__tsan_*` calls below exist in that leg and in no
//! other build. **The feature is not a switch a build may flip on its own**: it
//! names symbols only the ThreadSanitizer runtime exports, so a build that
//! enables it without `-Zsanitizer=thread` does not link, and that script is the
//! one caller for the same reason it is the one home of the flags.
//!
//! The gate is a feature rather than `cfg(sanitize = "thread")`, which is what
//! `rustc` itself sets under that flag: the predicate is unstable, and naming it
//! is an error on a stable compiler whether or not the branch is taken — so the
//! spelling that reads as exactly what it means would cost this crate its stable
//! build entirely.
//!
//! Off the leg [`Fiber`] is zero-sized and [`Fiber::around`] is the call it
//! wraps, which costs the scheduler nothing and still type-checks its call sites
//! — the failure this shape exists to stop is an annotation that stopped
//! compiling long before anyone next ran the leg.
//!
//! What it spends (`rule:programs/memory-priority`), in that leg alone: one
//! sanitizer fiber per task, alongside the stack the task already holds, for as
//! long as the task is alive. O(in-flight), like the stack it annotates.

#[cfg(feature = "tsan")]
use std::ffi::c_void;

// The sanitizer's fiber interface, as the ThreadSanitizer runtime exports it.
// A plain comment because rustdoc documents nothing on an extern block and
// warns about a doc comment there.
//
// The flag word both switching calls take selects a variant of the operation;
// a `0` is the plain one, which is the module doc's § *Why the switch
// synchronizes*.
#[cfg(feature = "tsan")]
#[expect(
    unsafe_code,
    reason = "the only way to tell ThreadSanitizer that a stack switch happened is its own runtime \
              interface, and there is no safe wrapper for it anywhere in the ecosystem. Every \
              argument below is a fiber handle this module itself received from that runtime, and \
              nothing crosses the boundary but those handles and a flag word."
)]
unsafe extern "C" {
    fn __tsan_get_current_fiber() -> *mut c_void;
    fn __tsan_create_fiber(flags: u32) -> *mut c_void;
    fn __tsan_destroy_fiber(fiber: *mut c_void);
    fn __tsan_switch_to_fiber(fiber: *mut c_void, flags: u32);
}

/// One task's fiber: the access history that follows its stack rather than the
/// thread the stack is currently under.
///
/// Created with the task's coroutine and destroyed with it, so a stack handed
/// back to [`crate::stack`]'s pool never carries the previous task's history
/// into the next one.
#[derive(Debug)]
pub(crate) struct Fiber {
    #[cfg(feature = "tsan")]
    raw: *mut c_void,
}

impl Fiber {
    /// A fiber for a stack nothing has run on yet.
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(feature = "tsan")]
            raw: {
                #[expect(
                    unsafe_code,
                    reason = "`__tsan_create_fiber` takes a flag word and returns a handle owned \
                              by this value until `Drop` hands it back. It touches no memory of \
                              ours, cannot fail, and leaves the caller on the fiber it found it on."
                )]
                unsafe {
                    __tsan_create_fiber(0)
                }
            },
        }
    }

    /// Runs `switch` — the one call that moves this thread onto this fiber's
    /// stack and back again — with the sanitizer told about both halves.
    ///
    /// The caller passes the stack switch itself and nothing else: everything
    /// before and after it stays outside, because a frame either side of one of
    /// these annotations is a frame the sanitizer accounts to the wrong stack.
    /// Leaving and returning within this one call is also what keeps its own
    /// instrumentation balanced — the module doc's § *Why it is one call and
    /// not two* is the whole of that reasoning, and it is not a style
    /// preference.
    pub(crate) fn around<R>(&self, switch: impl FnOnce() -> R) -> R {
        #[cfg(feature = "tsan")]
        #[expect(
            unsafe_code,
            reason = "`__tsan_get_current_fiber` reads the sanitizer's own current handle and \
                      `__tsan_switch_to_fiber` moves it, first to a handle this value owns and has \
                      not destroyed and then back to the one just read. Both touch no memory of \
                      ours and neither can fail; the thread's own handle outlives the call because \
                      the thread does."
        )]
        let here = unsafe {
            let here = __tsan_get_current_fiber();
            __tsan_switch_to_fiber(self.raw, 0);
            here
        };
        let out = switch();
        #[cfg(feature = "tsan")]
        #[expect(
            unsafe_code,
            reason = "the other half of the switch above, with the handle it read: the thread is \
                      back on the stack it was called on and the sanitizer is told so."
        )]
        unsafe {
            __tsan_switch_to_fiber(here, 0);
        }
        out
    }
}

#[cfg(feature = "tsan")]
impl Drop for Fiber {
    fn drop(&mut self) {
        #[expect(
            unsafe_code,
            reason = "`__tsan_destroy_fiber` retires a handle this value owns, once, at the end of \
                      its life. The scheduler is standing on its own fiber here — a task is never \
                      dropped from its own stack — so this never retires the current one."
        )]
        unsafe {
            __tsan_destroy_fiber(self.raw);
        }
    }
}
