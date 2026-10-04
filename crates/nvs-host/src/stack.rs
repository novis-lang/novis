//! A task's stack: reserved wide, resident narrow, pooled per worker.
//!
//! `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`
//! is this policy's only home; what follows is how it is spelled here and
//! the facts about the platforms that make it true.
//!
//! # Reserved is not resident
//!
//! [`TASK_STACK_SIZE`] is a *reservation*. `corosensei`'s `DefaultStack` maps
//! the whole width with no access rights and then makes the usable part
//! writable (`mmap` + `mprotect` on Unix, `VirtualAlloc(MEM_RESERVE)` plus a
//! commit of the first page and the guard pages on Windows) — so a page a task
//! never touches is never backed by memory on any platform this runs on. A
//! shallow handler costs the handful of pages its frames actually wrote, and
//! 100k concurrent tasks is 100 GiB of *address space* in a 64-bit process
//! against a resident cost measured in what the handlers touched.
//!
//! **Linux bounds the mappings rather than the bytes**, and that is the limit
//! this width meets first. Each reservation leaves more than one VMA behind —
//! the `mmap` and the `mprotect` that opens the usable part — and a process may
//! hold only `vm.max_map_count` of them, a sysctl whose stock setting is well
//! under what a hundred thousand stacks needs. Nothing here can raise it, so
//! `scheduler`'s release-only width test demands it of the kernel and names the
//! sysctl when it is short, rather than shrinking to a width that would prove
//! the claim only where nobody doubted it. `docs/setup.md` § *Linux and macOS*
//! is what an operator does about it.
//!
//! What it spends, in the terms
//! `rule:programs/memory-priority` asks for: one
//! reservation per in-flight task, charged to the request that owns it, plus at
//! most [`MAX_POOLED_STACKS`] reservations per worker held idle between tasks.
//! Both are O(in-flight) and neither grows with requests served.
//!
//! # Why the width is 1 MiB and not chosen freely
//!
//! `nvs_runtime`'s `STACK_RESERVE` is already 256 KiB of unwinding room between
//! the soft recursion limit and the hard floor
//! (`rule:errors/on-limit`). A stack
//! narrower than several times that would put the soft limit so close to its
//! own base that a handler would be refused before it had done anything; 1 MiB
//! leaves 768 KiB of ordinary depth above a reserve that is itself sized for
//! the throw. [`bounds`] is where the two meet.
//!
//! # Why a pool
//!
//! A stack is an `mmap`/`VirtualAlloc` pair and a request is not a thing to do
//! one of those for. A finished task's stack goes back to the worker's pool and
//! the next task takes it, so the syscall is paid once per concurrency level
//! rather than once per request. The pool is per worker and therefore never
//! shared, never locked and never `Send` — it lives in that worker's
//! [`Scheduler`](crate::Scheduler), which is `!Send` for the same reason
//! everything else on a core is.
//!
//! **[`MAX_POOLED_STACKS`] is a compiled-in stand-in for the worker's in-flight
//! cap**, which does not exist yet: admission arithmetic and the configurable
//! `[limits]` that bound it are goal `governance`'s, and this constant is what that work
//! replaces. Without it the pool's only bound would be the high-water mark of
//! concurrency, so one burst would leave its stacks — and the pages they
//! touched — resident for the life of the worker. Handing a stack back to the
//! OS past the cap is the deliberate trade: the burst pays a syscall on the way
//! down so the steady state does not pay a footprint forever.
//!
//! # The spare stack
//!
//! A parse whose recursion depth is set by its input — `serde_json` takes one
//! native frame per nesting level, and `Core\Json::decode` allows 1024 of them
//! — cannot be run on the task's own stack, because what is left of that stack
//! depends on how deep the program already is and on how fat the build's frames
//! are. [`on_spare_stack`] runs such a parse on a second stack instead: one per
//! thread, [`SPARE_STACK_SIZE`] reserved, taken the first time a thread needs it
//! and kept until the thread exits.
//!
//! What it spends: one reservation per worker thread, O(workers) and never
//! O(requests), plus the pages the deepest parse on that thread touched, which
//! stay resident for the life of the thread.

use std::cell::RefCell;

use corosensei::stack::{DefaultStack, Stack as _};

/// The address space one task's stack reserves, guard page excluded.
///
/// Reserved, not committed — this module's docs own why that distinction is the
/// whole footprint answer, and why the number is 1 MiB rather than anything
/// else. Public because it is the quantity an admission decision counts ahead
/// of time (`rule:http-server/a-wedged-core-is-detected-by-its-deadline`), which is the only thing outside this crate that has
/// a reason to know it.
pub const TASK_STACK_SIZE: usize = 1 << 20;

/// How many idle stacks one worker's pool keeps before handing one back to the
/// OS.
///
/// A compiled-in stand-in for that worker's in-flight cap, exactly as every
/// other limit in this goal is compiled in: 1 GiB of reserved address space per
/// worker, which is unremarkable, against the resident pages those stacks
/// touched, which is what the cap actually bounds.
pub const MAX_POOLED_STACKS: usize = 1024;

/// The address space a thread's spare stack reserves, guard page excluded.
///
/// 8 MiB, the default Linux thread stack. It is sized for the debug profile,
/// whose frames are several times the release profile's, and
/// `nvs_stdlib::json`'s tests decode the deepest allowed document on it from a
/// thread whose own stack could not hold that parse.
pub const SPARE_STACK_SIZE: usize = 8 << 20;

thread_local! {
    /// This thread's spare stack, `None` until the first [`on_spare_stack`]
    /// and while one is running.
    static SPARE: RefCell<Option<DefaultStack>> = const { RefCell::new(None) };
}

/// Runs `f` on this thread's spare stack and returns what it returns.
///
/// `f` must not call compiled Novis code. The context's recursion limit is
/// armed for the task's stack, so a Novis frame on this one would be checked
/// against the wrong bounds. A call made while the spare stack is already in
/// use reserves another one for itself, and the thread keeps one of the two
/// when both have returned. A panic in `f` is propagated to the caller, and the
/// stack it ran on is freed.
///
/// # Panics
///
/// If `f` panics, or if the OS refuses to reserve the stack, which is the same
/// `expect` [`StackPool::take`] makes.
pub fn on_spare_stack<R>(f: impl FnOnce() -> R) -> R {
    let mut stack = SPARE
        .with(|spare| spare.borrow_mut().take())
        .unwrap_or_else(|| {
            DefaultStack::new(SPARE_STACK_SIZE).expect("a thread could not reserve its spare stack")
        });
    let result = corosensei::on_stack(&mut stack, f);
    SPARE.with(|spare| {
        let mut slot = spare.borrow_mut();
        if slot.is_none() {
            *slot = Some(stack);
        }
    });
    result
}

/// The `(base, ceiling)` pair `Ctx::arm_stack_limit` wants for a task running
/// on `stack`.
///
/// The ceiling is [`TASK_STACK_SIZE`] and **not** `base - limit`, which is the
/// mistake this function exists to make impossible: `corosensei`'s `limit()` is
/// documented to include the guard pages, so arming from it would put the hard
/// floor *below* the guard and the overflow would fault before the limit ever
/// compared unequal. Every platform rounds its allocation up from the requested
/// size, so `base - TASK_STACK_SIZE` is at or above the first usable byte and
/// the floor is always inside memory the task may write.
pub(crate) fn bounds(stack: &DefaultStack) -> (usize, usize) {
    (stack.base().get(), TASK_STACK_SIZE)
}

/// One worker's supply of task stacks.
///
/// Not `Send`: it holds no lock because it is never reached from a second
/// thread, which is the same structural property [`crate::Scheduler`] carries
/// and for the same reason.
pub(crate) struct StackPool {
    free: Vec<DefaultStack>,
    cap: usize,
}

impl std::fmt::Debug for StackPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // A stack's address says nothing a reader can use and would differ
        // between runs; how many are idle is the only interesting fact.
        f.debug_struct("StackPool")
            .field("pooled", &self.free.len())
            .field("cap", &self.cap)
            .finish()
    }
}

impl StackPool {
    /// An empty pool bounded by [`MAX_POOLED_STACKS`].
    pub(crate) const fn new() -> Self {
        Self::with_cap(MAX_POOLED_STACKS)
    }

    /// An empty pool with an explicit bound, so a test can reach the cap
    /// without reserving a gigabyte to do it.
    pub(crate) const fn with_cap(cap: usize) -> Self {
        Self {
            free: Vec::new(),
            cap,
        }
    }

    /// A stack for a task, recycled if the pool has one and freshly reserved if
    /// it does not.
    ///
    /// # Panics
    ///
    /// If the OS refuses the reservation. Nothing softer is available here —
    /// `Scheduler::spawn` has no error to return and a task with no stack
    /// cannot be queued — and nothing softer is wanted either: address space is
    /// precisely the resource `rule:http-server/a-wedged-core-is-detected-by-its-deadline`'s admission arithmetic counts before
    /// it admits a request, so a refusal at this point means admission let in
    /// work the worker could not hold. That arithmetic is goal `governance`'s; until it
    /// exists this is the same `expect` `corosensei`'s own default stack does.
    pub(crate) fn take(&mut self) -> DefaultStack {
        match self.free.pop() {
            Some(stack) => stack,
            None => {
                DefaultStack::new(TASK_STACK_SIZE).expect("a worker could not reserve a task stack")
            }
        }
    }

    /// Takes a finished task's stack back, keeping it for the next task unless
    /// the pool is already at its cap.
    pub(crate) fn give(&mut self, stack: DefaultStack) {
        if self.free.len() < self.cap {
            self.free.push(stack);
        }
    }

    /// How many stacks are idle in this pool.
    pub(crate) fn pooled(&self) -> usize {
        self.free.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_finished_task_s_stack_is_handed_to_the_next_one() {
        let mut pool = StackPool::new();
        let first = pool.take();
        let base = first.base();
        assert_eq!(pool.pooled(), 0, "a stack in use is not idle");

        pool.give(first);
        assert_eq!(pool.pooled(), 1);

        let second = pool.take();
        assert_eq!(
            second.base(),
            base,
            "the next task must get the stack back rather than a new mapping"
        );
        assert_eq!(pool.pooled(), 0);
    }

    #[test]
    fn a_stack_past_the_cap_goes_back_to_the_os() {
        // The property `MAX_POOLED_STACKS` exists for, at a width a test can
        // afford: a burst that ends does not leave its whole peak resident.
        let mut pool = StackPool::with_cap(2);
        for _ in 0..4 {
            pool.give(DefaultStack::new(TASK_STACK_SIZE).expect("a test stack"));
        }
        assert_eq!(pool.pooled(), 2);
    }

    /// Where a local of `on_spare_stack`'s closure lives.
    fn a_local_s_address() -> usize {
        let local = 0_u8;
        std::ptr::from_ref(std::hint::black_box(&local)) as usize
    }

    /// The base of the spare stack this thread keeps, if it keeps one.
    fn kept_base() -> Option<usize> {
        SPARE.with(|spare| spare.borrow().as_ref().map(|stack| stack.base().get()))
    }

    #[test]
    fn the_callable_runs_on_the_thread_s_spare_stack_and_the_stack_is_kept() {
        let first = on_spare_stack(a_local_s_address);
        let base = kept_base().expect("the thread keeps its spare stack after a call");
        assert!(
            first < base && first > base - SPARE_STACK_SIZE,
            "the closure's local is inside the spare stack"
        );

        let second = on_spare_stack(a_local_s_address);
        assert_eq!(second, first, "the second call runs on the same stack");
        assert_eq!(kept_base(), Some(base));
    }

    #[test]
    fn a_nested_call_gets_a_stack_of_its_own_and_one_is_kept() {
        let (outer, inner) =
            on_spare_stack(|| (a_local_s_address(), on_spare_stack(a_local_s_address)));
        assert!(
            outer.abs_diff(inner) > SPARE_STACK_SIZE / 2,
            "the inner call is not on the outer call's stack"
        );
        assert!(kept_base().is_some(), "one stack is kept after both return");
    }

    #[test]
    fn a_panic_in_the_callable_reaches_the_caller() {
        let caught = std::panic::catch_unwind(|| on_spare_stack(|| panic!("inside")));
        assert!(caught.is_err());
        assert_eq!(on_spare_stack(|| 7), 7, "the next call still runs");
    }

    #[test]
    fn the_armed_floor_lands_inside_the_task_s_own_stack() {
        // `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`'s last bullet, as arithmetic: the hard floor is below
        // the base and at or above the first byte the task may write, so it is
        // crossed by a deep call before the guard page is.
        let stack = DefaultStack::new(TASK_STACK_SIZE).expect("a test stack");
        let (base, ceiling) = bounds(&stack);
        assert_eq!(base, stack.base().get());
        assert_eq!(ceiling, TASK_STACK_SIZE);

        let floor = base - ceiling;
        assert!(floor < base, "the floor is below the base");
        assert!(
            floor >= stack.limit().get(),
            "the floor must not sit below the guard page"
        );
        assert!(
            ceiling > 2 * nvs_runtime::STACK_RESERVE,
            "a stack has to be several times the unwinding reserve to be coherent with it"
        );
    }
}
