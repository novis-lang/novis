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
//! most the worker's cap of reservations held idle in each of its two pools, which is its
//! share of `[server] max_in_flight` on a serving core. Both are O(in-flight)
//! and neither grows with requests served.
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
//! **A serving core caps its pool at its share of `max_in_flight`**:
//! `nvs serve` hands [`Scheduler::bound_pool_by`](crate::Scheduler::bound_pool_by)
//! a function reading admission's live number divided by the worker count, so
//! a reload that moves `max_in_flight` moves the cap at the next stack that
//! comes back, and a lowered cap frees the excess there. Without a cap the
//! pool's only bound is the most tasks the worker has had in flight at once,
//! so one burst would leave its stacks, and the pages they touched, resident
//! for the life of the worker. Handing a stack back to the OS past the cap is
//! the deliberate trade: the burst pays a syscall on the way down so the
//! steady state does not pay a footprint forever.
//!
//! A scheduler that is not a serving core has no admission number and keeps
//! that high-water mark. Each program's tasks are bounded by its `max_tasks`,
//! and `nvs run`'s scheduler ends with its one program, so a compiled-in
//! number here would bound nothing the work does not.
//!
//! # One pool per role
//!
//! A worker keeps two pools ([`Stacks`]): one for the server's own work — the
//! accept loop, a connection, a ticker — and one for compiled Novis code — an
//! isolate's task, a handler script, and every task beneath one of them
//! ([`Role`]). A child takes the pool of the task that spawned it. A pooled
//! stack keeps the pages its deepest task touched, so with one pool a
//! connection that took a stack a program had grown deep kept the program's
//! pages for a task that needs a few. With two, a connection's stack stays at a
//! connection's depth. Each pool is capped as the one pool was, so a serving
//! core may hold twice its share of `max_in_flight` idle: what a waiting request
//! holds in flight, a connection stack and a program stack.
//!
//! # The idle trim
//!
//! A pooled stack keeps every page it was given, so the pages of a burst stay
//! resident after it. When the core is about to wait with nothing ready and
//! has taken no stack for a whole [`TRIM_INTERVAL`], each idle stack that came
//! back since the last trim gives back its pages, except its top
//! [`KEPT_RESIDENT`] bytes: `madvise(MADV_DONTNEED)` on Unix and
//! `DiscardVirtualMemory` on Windows. A stack keeps its mapping and its place in
//! the pool, so the next task pays page faults and no `mmap`.
//!
//! The trim never runs on a give, which would be a system call per request, and
//! never while the core is busy, which would fault the pages of a hot stack back
//! in once a second. While a stack waits for its trim, the core's wait is
//! bounded by the interval, so a core that then goes quiet wakes at most twice
//! more to trim. A trim is not a scrubbing guarantee: on macOS the old bytes may
//! stay. Novis code cannot read them, because the language is memory-safe.
//!
//! What it spends: one system call per idle stack per trim on Unix and a few on
//! Windows, and up to two wakes of a quiet core. It gives back all but 8 KiB of
//! each trimmed stack.
//!
//! # What a sleeping program touches
//!
//! An isolate's task that sleeps as `Core\Time::sleep` does, on a fresh stack,
//! has 2 pages resident in a release build on Windows and 7 in a debug one. The
//! largest frame is the task's entry: `corosensei`'s start and the closure in
//! `Scheduler::start`. The task's `Ctx` stays in the box `start` carries it
//! in, so that frame holds a pointer to it. No frame of the host's path is
//! worth splitting, and [`KEPT_RESIDENT`] covers it whole.
//! `a_sleeping_program_touches_the_pages_its_own_frames_need` holds the host's
//! part to its count. A pooled stack also keeps the pages a deeper call
//! touched before it returned, until the idle trim.
//!
//! # What a waiting request holds
//!
//! `nvs-cli`'s `a_waiting_request_holds_its_connection_and_its_isolate_on_the_heap`
//! opens 32 connections to one core and then sends a request on each whose
//! program sleeps. It reads the heap and, in a debug build,
//! [`crate::counts::StackPages`] at each step. A release build measured with
//! that reading turned on by hand gives the stack figures below; the test holds
//! the debug ones.
//!
//! A waiting request has about 10 pages of stack resident in release, 31 in
//! debug:
//!
//! - **An idle connection, 4.1 pages (13.5 in debug)** on its connection's
//!   stack: the accept loop's spawn, `hyper`'s connection future and the read
//!   it is parked in.
//! - **Its request, 3.9 more pages (9.6)** on the connection's stack, while the
//!   connection waits for the isolate's answer.
//! - **Its program, 2.0 pages (8)** on the program's own stack, which is the
//!   host's path above and little more.
//!
//! So three quarters of a waiting request's stack is its connection's. Kept by
//! value in the task's entry, the `Ctx` cost one more page on every stack: an
//! idle connection held 5.0. What the box spends: one heap block of a `Ctx`'s
//! size per live task. It was allocated per task before too, and freed when the
//! task started.
//!
//! Its heap on the serving core is about 26 KiB, measured in a debug build:
//!
//! - **The connection, about 22 KiB.** `hyper`'s read buffer and its header
//!   write buffer, 8 KiB each, sized inside `hyper` with no setting to change
//!   them. The future that serves the connection's requests, about 2.4 KiB,
//!   which `hyper` boxes. The connection task's `Ctx`, 1,872 bytes, and about
//!   1 KiB of small cells and handles.
//! - **The request, about 3.7 KiB.** The isolate's `Ctx`, 1,872 bytes and
//!   about 550 more of its own, and the request's parsed head and slots.
//!
//! Two 8 KiB parts were not needed per request and are gone. The read scratch
//! of `nvs_server::io` is one per core, because no read parks while it holds
//! it. The access line copies the request path out of the head, so `hyper`
//! reuses its read buffer: a clone of the `Uri` kept that buffer for the whole
//! request, and `hyper` read the next bytes into a second one.
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
use std::rc::Rc;
use std::time::{Duration, Instant};

use corosensei::stack::{DefaultStack, Stack as _};

/// The address space one task's stack reserves, guard page excluded.
///
/// Reserved, not committed — this module's docs own why that distinction is the
/// whole footprint answer, and why the number is 1 MiB rather than anything
/// else. Public because it is the quantity an admission decision counts ahead
/// of time (`rule:http-server/a-wedged-core-is-detected-by-its-deadline`), which is the only thing outside this crate that has
/// a reason to know it.
pub const TASK_STACK_SIZE: usize = 1 << 20;

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
/// If `f` panics, or if the OS refuses to reserve the stack.
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

/// How long a core must go without taking a stack before its pools give their
/// pages back, and so the most often a trim runs. The module doc's *The idle
/// trim* owns why.
pub(crate) const TRIM_INTERVAL: Duration = Duration::from_secs(1);

/// The top of a pooled stack a trim leaves resident: the pages the next task's
/// first frames write.
pub(crate) const KEPT_RESIDENT: usize = 8 << 10;

/// Which pool a task's stack comes from. The module doc's *One pool per role*
/// owns the split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    /// The server's own work: the accept loop, a connection, a ticker.
    Host,
    /// Compiled Novis code: an isolate's task, a handler script, and every
    /// task beneath one of them.
    Program,
}

/// One worker's two pools and the trim that serves both.
///
/// Not `Send`, for [`StackPool`]'s reason.
#[derive(Debug)]
pub(crate) struct Stacks {
    host: StackPool,
    program: StackPool,
    /// When the core last asked whether it was idle a whole interval.
    checked: Instant,
    /// Whether a stack was taken since [`Self::checked`].
    taken: bool,
}

impl Stacks {
    /// Two empty pools with no cap.
    pub(crate) fn new() -> Self {
        Self {
            host: StackPool::new(),
            program: StackPool::new(),
            checked: Instant::now(),
            taken: false,
        }
    }

    fn pool(&mut self, role: Role) -> &mut StackPool {
        match role {
            Role::Host => &mut self.host,
            Role::Program => &mut self.program,
        }
    }

    /// A stack for a task of `role`, from that role's pool.
    ///
    /// # Errors
    ///
    /// [`StackPool::take`]'s.
    pub(crate) fn take(&mut self, role: Role) -> std::io::Result<DefaultStack> {
        self.taken = true;
        let stack = self.pool(role).take()?;
        #[cfg(debug_assertions)]
        LIVE.with_borrow_mut(|live| live.push((role, stack.base().get())));
        Ok(stack)
    }

    /// A finished task's stack, back to the pool of the role it was taken for.
    pub(crate) fn give(&mut self, role: Role, stack: DefaultStack) {
        #[cfg(debug_assertions)]
        LIVE.with_borrow_mut(|live| {
            let base = stack.base().get();
            if let Some(at) = live.iter().position(|&(_, live)| live == base) {
                live.swap_remove(at);
            }
        });
        self.pool(role).give(stack);
    }

    /// Caps each pool at what `cap` returns, read each time a stack comes back.
    pub(crate) fn bound_by(&mut self, cap: Rc<dyn Fn() -> usize>) {
        self.host.cap = Some(Rc::clone(&cap));
        self.program.cap = Some(cap);
    }

    /// How many stacks are idle in both pools.
    pub(crate) fn pooled(&self) -> usize {
        self.host.pooled() + self.program.pooled()
    }

    /// How many stacks are idle in `role`'s pool.
    #[cfg(test)]
    pub(crate) fn pooled_for(&self, role: Role) -> usize {
        match role {
            Role::Host => self.host.pooled(),
            Role::Program => self.program.pooled(),
        }
    }

    /// Called when the core is about to wait with nothing ready. Trims every
    /// idle stack a trim has not reached yet once the core has taken no stack
    /// for a whole [`TRIM_INTERVAL`], and returns how long the wait may last
    /// before the next call is due. `None` when no stack is left to trim, so
    /// the wait is not bounded for it.
    pub(crate) fn idle_wait(&mut self, now: Instant) -> Option<Duration> {
        if !self.host.untrimmed() && !self.program.untrimmed() {
            return None;
        }
        let since = now.saturating_duration_since(self.checked);
        if since < TRIM_INTERVAL {
            return Some(TRIM_INTERVAL - since);
        }
        if !self.taken {
            self.host.trim();
            self.program.trim();
        }
        self.taken = false;
        self.checked = now;
        (self.host.untrimmed() || self.program.untrimmed()).then_some(TRIM_INTERVAL)
    }

    /// Makes every reservation either pool would make from now on fail.
    #[cfg(test)]
    pub(crate) fn refuse_fresh(&mut self) {
        self.host.refuse_fresh();
        self.program.refuse_fresh();
    }
}

#[cfg(debug_assertions)]
thread_local! {
    /// The role and base of every stack this thread's [`Stacks`] handed to a
    /// task and has not taken back, for [`live_resident_pages`].
    static LIVE: RefCell<Vec<(Role, usize)>> = const { RefCell::new(Vec::new()) };
}

/// How many pages are resident on the stacks this thread's tasks run on now,
/// the host's and the programs', in that order. A stack whose task was dropped
/// with its scheduler is still listed, and its range reads as not resident.
#[cfg(debug_assertions)]
pub(crate) fn live_resident_pages() -> (usize, usize) {
    LIVE.with_borrow(|live| {
        live.iter().fold((0, 0), |(host, program), &(role, base)| {
            let pages = resident_pages(base - TASK_STACK_SIZE, TASK_STACK_SIZE);
            match role {
                Role::Host => (host + pages, program),
                Role::Program => (host, program + pages),
            }
        })
    })
}

/// How many pages of the `len` bytes at `start` are resident. A range that is
/// not mapped has none.
#[cfg(all(any(test, debug_assertions), target_os = "linux"))]
pub(crate) fn resident_pages(start: usize, len: usize) -> usize {
    let page = 4096;
    let mut flags = vec![0_u8; len.div_ceil(page)];
    #[expect(
        unsafe_code,
        reason = "`mincore` has no `std` spelling; the vector has one byte per page of the range"
    )]
    // SAFETY: the range is page aligned, and `flags` has one byte per page of it.
    // An unmapped range fails with `ENOMEM` and writes nothing.
    let read = unsafe {
        libc::mincore(
            std::ptr::with_exposed_provenance_mut::<libc::c_void>(start),
            len,
            flags.as_mut_ptr(),
        )
    };
    if read != 0 {
        return 0;
    }
    flags.iter().filter(|flag| **flag & 1 == 1).count()
}

/// [`resident_pages`] on Windows, through the working set.
#[cfg(all(any(test, debug_assertions), windows))]
pub(crate) fn resident_pages(start: usize, len: usize) -> usize {
    use windows_sys::Win32::System::ProcessStatus::{
        K32QueryWorkingSetEx, PSAPI_WORKING_SET_EX_INFORMATION,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let page = 4096;
    let mut pages: Vec<PSAPI_WORKING_SET_EX_INFORMATION> = (0..len.div_ceil(page))
        .map(|n| PSAPI_WORKING_SET_EX_INFORMATION {
            VirtualAddress: std::ptr::with_exposed_provenance_mut(start + n * page),
            ..Default::default()
        })
        .collect();
    let bytes = u32::try_from(size_of_val(pages.as_slice())).expect("a small query");
    #[expect(
        unsafe_code,
        reason = "`QueryWorkingSetEx` has no `std` spelling; the buffer is `bytes` long"
    )]
    // SAFETY: the buffer is live and `bytes` long, and the handle is this process's.
    let read =
        unsafe { K32QueryWorkingSetEx(GetCurrentProcess(), pages.as_mut_ptr().cast(), bytes) };
    assert_ne!(read, 0, "QueryWorkingSetEx failed");
    #[expect(
        unsafe_code,
        reason = "both fields of the union are a `usize` of flags"
    )]
    // SAFETY: every bit pattern of the union is valid.
    pages
        .iter()
        .filter(|page| unsafe { page.VirtualAttributes.Flags } & 1 == 1)
        .count()
}

/// [`resident_pages`] where there is no query: it counts every page.
#[cfg(all(any(test, debug_assertions), not(any(target_os = "linux", windows))))]
pub(crate) fn resident_pages(_start: usize, len: usize) -> usize {
    len / 4096
}

/// A stack idle in a pool, and whether its pages may still be resident.
struct Pooled {
    stack: DefaultStack,
    /// `false` once a trim gave its pages back, until a task takes it again.
    resident: bool,
}

/// One role's supply of task stacks on one worker.
///
/// Not `Send`: it holds no lock because it is never reached from a second
/// thread, which is the same structural property [`crate::Scheduler`] carries
/// and for the same reason.
pub(crate) struct StackPool {
    free: Vec<Pooled>,
    /// How many idle stacks the pool keeps, read each time one comes back.
    /// `None` keeps every one, so the pool is bounded by the most tasks its
    /// scheduler has had in flight at once. The module doc owns which
    /// schedulers have a cap.
    cap: Option<Rc<dyn Fn() -> usize>>,
    /// Whether a fresh reservation fails, which only a test sets.
    #[cfg(test)]
    refuse_fresh: bool,
}

impl std::fmt::Debug for StackPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // A stack's address says nothing a reader can use and would differ
        // between runs; how many are idle is the only interesting fact.
        f.debug_struct("StackPool")
            .field("pooled", &self.free.len())
            .field("cap", &self.cap.as_ref().map(|cap| cap()))
            .finish()
    }
}

impl StackPool {
    /// An empty pool with no cap of its own.
    pub(crate) fn new() -> Self {
        Self {
            free: Vec::new(),
            cap: None,
            #[cfg(test)]
            refuse_fresh: false,
        }
    }

    /// An empty pool that keeps at most `cap` idle stacks.
    #[cfg(test)]
    pub(crate) fn with_cap(cap: usize) -> Self {
        let mut pool = Self::new();
        pool.bound_by(Rc::new(move || cap));
        pool
    }

    /// Keeps at most what `cap` returns idle, from the next stack given back.
    #[cfg(test)]
    pub(crate) fn bound_by(&mut self, cap: Rc<dyn Fn() -> usize>) {
        self.cap = Some(cap);
    }

    /// A stack for a task, recycled if the pool has one and freshly reserved if
    /// it does not.
    ///
    /// # Errors
    ///
    /// The OS error, when the pool is empty and the OS refuses the reservation.
    /// [`crate::Scheduler`] ends the task the stack was for as a limit
    /// (`rule:errors/on-limit`), so a refusal fails that task's request and
    /// the worker goes on serving the others.
    pub(crate) fn take(&mut self) -> std::io::Result<DefaultStack> {
        if let Some(pooled) = self.free.pop() {
            return Ok(pooled.stack);
        }
        #[cfg(test)]
        if self.refuse_fresh {
            return Err(std::io::Error::other("the test refused the reservation"));
        }
        DefaultStack::new(TASK_STACK_SIZE)
    }

    /// Makes every reservation this pool would make from now on fail, so a
    /// test can reach the refusal without exhausting the address space.
    #[cfg(test)]
    pub(crate) fn refuse_fresh(&mut self) {
        self.refuse_fresh = true;
    }

    /// Takes a finished task's stack back, keeping it for the next task unless
    /// the pool is already at its cap. A cap lowered since the last call frees
    /// the idle stacks past it here.
    pub(crate) fn give(&mut self, stack: DefaultStack) {
        let pooled = Pooled {
            stack,
            resident: true,
        };
        let Some(cap) = self.cap.as_ref().map(|cap| cap()) else {
            self.free.push(pooled);
            return;
        };
        if self.free.len() < cap {
            self.free.push(pooled);
        }
        self.free.truncate(cap);
    }

    /// How many stacks are idle in this pool.
    pub(crate) fn pooled(&self) -> usize {
        self.free.len()
    }

    /// Whether an idle stack here may still hold pages a trim would give back.
    fn untrimmed(&self) -> bool {
        self.free.iter().any(|pooled| pooled.resident)
    }

    /// Gives back the pages of every idle stack a trim has not reached since
    /// it came back, except its top [`KEPT_RESIDENT`] bytes. One system call
    /// per such stack on Unix, and a few on Windows.
    fn trim(&mut self) {
        for pooled in self.free.iter_mut().filter(|pooled| pooled.resident) {
            let base = pooled.stack.base().get();
            // `bounds`' floor: at or above the first byte the task may write,
            // so the range is inside the stack's writable part.
            let floor = base - TASK_STACK_SIZE;
            discard(floor, base - KEPT_RESIDENT - floor);
            pooled.resident = false;
        }
    }
}

/// Tells the OS the `len` bytes at `start`, inside an idle pooled stack, hold
/// nothing, so it may take their pages back. A failure leaves them resident,
/// which costs memory and nothing else, so it is ignored.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn discard(start: usize, len: usize) {
    // `madvise` rounds a length up to whole pages. On a host whose pages are
    // larger than 4 KiB that would reach into the kept top, so the range ends
    // at the last whole page below it instead.
    #[expect(
        unsafe_code,
        reason = "`sysconf` has no `std` spelling; it reads a constant of the process"
    )]
    // SAFETY: `_SC_PAGESIZE` is a valid name, and the call writes no memory.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let page = usize::try_from(page).unwrap_or(4096).max(1);
    let len = ((start + len) / page * page).saturating_sub(start);
    if len == 0 {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "`madvise` has no `std` spelling. The range is inside the writable part of a \
                  stack this pool owns and no task runs on, so no live frame is in it. A page \
                  read after `MADV_DONTNEED` reads as zero on Linux and keeps or loses its old \
                  bytes on macOS, and the next task writes every frame before it reads one."
    )]
    // SAFETY: as the reason above says, nothing reads these bytes until a new
    // task's frames write them.
    let _ = unsafe {
        libc::madvise(
            std::ptr::with_exposed_provenance_mut::<libc::c_void>(start),
            len,
            libc::MADV_DONTNEED,
        )
    };
}

/// [`discard`] on Windows: `DiscardVirtualMemory` over each committed,
/// read-write region of the range. The pages stay committed, so the guard page
/// `corosensei` moves down a growing stack still works. The reserved pages
/// below it are skipped, because the call fails on a range that holds one.
#[cfg(windows)]
fn discard(start: usize, len: usize) {
    use windows_sys::Win32::System::Memory::{
        DiscardVirtualMemory, MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_READWRITE, VirtualQuery,
    };
    let end = start + len;
    let mut at = start;
    while at < end {
        let mut info = MEMORY_BASIC_INFORMATION::default();
        #[expect(
            unsafe_code,
            reason = "`VirtualQuery` has no `std` spelling. It reads the attributes of an address \
                      this pool's stack reserved, into a stack struct this call owns."
        )]
        // SAFETY: the out-parameter is a live, correctly sized struct.
        let filled = unsafe {
            VirtualQuery(
                std::ptr::with_exposed_provenance::<std::ffi::c_void>(at),
                &raw mut info,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if filled == 0 {
            return;
        }
        let region_end = (info.BaseAddress.addr() + info.RegionSize).min(end);
        if region_end <= at {
            return;
        }
        if info.State == MEM_COMMIT && info.Protect == PAGE_READWRITE {
            #[expect(
                unsafe_code,
                reason = "`DiscardVirtualMemory` has no `std` spelling. The region is committed, \
                          read-write and inside a stack this pool owns and no task runs on, so \
                          no live frame is in it, and the next task writes every frame before it \
                          reads one."
            )]
            // SAFETY: as the reason above says.
            let _ = unsafe {
                DiscardVirtualMemory(
                    std::ptr::with_exposed_provenance_mut::<std::ffi::c_void>(at),
                    region_end - at,
                )
            };
        }
        at = region_end;
    }
}

/// [`discard`] where neither call exists: the pages stay resident.
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn discard(_start: usize, _len: usize) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_finished_task_s_stack_is_handed_to_the_next_one() {
        let mut pool = StackPool::new();
        let first = pool.take().expect("a test stack");
        let base = first.base();
        assert_eq!(pool.pooled(), 0, "a stack in use is not idle");

        pool.give(first);
        assert_eq!(pool.pooled(), 1);

        let second = pool.take().expect("a test stack");
        assert_eq!(
            second.base(),
            base,
            "the next task must get the stack back rather than a new mapping"
        );
        assert_eq!(pool.pooled(), 0);
    }

    #[test]
    fn a_stack_past_the_cap_goes_back_to_the_os() {
        // A burst that ends does not leave its whole peak resident.
        let mut pool = StackPool::with_cap(2);
        for _ in 0..4 {
            pool.give(DefaultStack::new(TASK_STACK_SIZE).expect("a test stack"));
        }
        assert_eq!(pool.pooled(), 2);
    }

    #[test]
    fn a_lowered_cap_frees_the_idle_stacks_past_it_at_the_next_give() {
        let cap = std::rc::Rc::new(std::cell::Cell::new(4));
        let mut pool = StackPool::new();
        let read = std::rc::Rc::clone(&cap);
        pool.bound_by(Rc::new(move || read.get()));
        for _ in 0..4 {
            pool.give(DefaultStack::new(TASK_STACK_SIZE).expect("a test stack"));
        }
        assert_eq!(pool.pooled(), 4);
        cap.set(1);
        pool.give(DefaultStack::new(TASK_STACK_SIZE).expect("a test stack"));
        assert_eq!(pool.pooled(), 1, "a reload's lower cap was not met");
    }

    #[test]
    fn a_connection_stack_and_a_program_stack_come_from_their_own_pools() {
        let mut stacks = Stacks::new();
        let program = stacks.take(Role::Program).expect("a test stack");
        let program_base = program.base();
        stacks.give(Role::Program, program);
        assert_eq!(stacks.pooled_for(Role::Program), 1);
        assert_eq!(stacks.pooled_for(Role::Host), 0);

        let host = stacks.take(Role::Host).expect("a test stack");
        assert_ne!(
            host.base(),
            program_base,
            "a connection took the stack a program ran on"
        );
        assert_eq!(
            stacks.pooled_for(Role::Program),
            1,
            "the program's stack stays in its pool"
        );
        stacks.give(Role::Host, host);

        let again = stacks.take(Role::Program).expect("a test stack");
        assert_eq!(
            again.base(),
            program_base,
            "the next program gets the program's stack back"
        );
    }

    #[test]
    fn each_pool_is_held_to_the_cap_on_its_own() {
        let mut stacks = Stacks::new();
        stacks.bound_by(Rc::new(|| 2));
        for role in [Role::Host, Role::Program] {
            for _ in 0..4 {
                stacks.give(
                    role,
                    DefaultStack::new(TASK_STACK_SIZE).expect("a test stack"),
                );
            }
        }
        assert_eq!(stacks.pooled_for(Role::Host), 2);
        assert_eq!(
            stacks.pooled_for(Role::Program),
            2,
            "the program pool kept its burst"
        );
        assert_eq!(stacks.pooled(), 4, "a core keeps the cap once per pool");
    }

    #[test]
    fn a_pooled_stack_gives_its_pages_back_when_its_core_is_idle() {
        let mut stacks = Stacks::new();
        let mut stack = stacks.take(Role::Program).expect("a test stack");
        corosensei::on_stack(&mut stack, || {
            let frame = std::hint::black_box([1_u8; 256 << 10]);
            std::hint::black_box(&frame);
        });
        let base = stack.base().get();
        let floor = base - TASK_STACK_SIZE;
        let trimmed = base - KEPT_RESIDENT - floor;
        stacks.give(Role::Program, stack);
        let touched = resident_pages(floor, trimmed);
        assert!(
            touched >= 32,
            "the deep frame left {touched} pages resident"
        );

        let start = stacks.checked;
        assert!(
            stacks.idle_wait(start).is_some(),
            "a stack waiting for its trim bounds the core's wait"
        );
        // The core took a stack in its first interval, so it is not idle yet.
        assert!(stacks.idle_wait(start + TRIM_INTERVAL).is_some());
        assert!(
            resident_pages(floor, trimmed) >= 32,
            "a core that took a stack this interval trimmed"
        );
        assert_eq!(
            stacks.idle_wait(start + 2 * TRIM_INTERVAL),
            None,
            "a trimmed pool still bounds the wait"
        );
        if cfg!(any(target_os = "linux", windows)) {
            let left = resident_pages(floor, trimmed);
            assert!(
                left <= 2,
                "the idle trim left {left} of {touched} pages resident"
            );
        }

        let again = stacks.take(Role::Program).expect("a test stack");
        assert_eq!(again.base().get(), base, "a trimmed stack is still pooled");
    }

    /// The module doc's *What a sleeping program touches*, as a count: an
    /// isolate's task that sleeps the way `Core\Time::sleep` does, on a fresh
    /// program stack, read by a neighbour while it is parked. The bound is the
    /// figure measured on Windows in each profile, with one page of margin.
    #[test]
    fn a_sleeping_program_touches_the_pages_its_own_frames_need() {
        use crate::isolate::{Isolate, Output, Program};
        use crate::reactor::{Reactor, install, run_until_idle};
        use crate::scheduler::Scheduler;
        use nvs_runtime::{Ctx, OutputSink, TaskRoot, Value};
        use std::cell::Cell;

        let floor = Rc::new(Cell::new(0_usize));
        let resident = Rc::new(Cell::new(0_usize));
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let armed = Rc::clone(&floor);
        let program: Program = Box::new(move |ctx: &mut Ctx, _args| {
            armed.set(ctx.stack_bounds().1);
            let _ = nvs_runtime::peer::sleep(ctx, Duration::from_millis(200));
            Value::null()
        });
        let mut cell = Some(program);
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |ctx| {
            let program = cell.take().expect("run once");
            let _ = Isolate::new(program, Value::null(), Output::Capture).run(ctx);
        });
        let (read, seen) = (Rc::clone(&floor), Rc::clone(&resident));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            while read.get() == 0 {
                let _ = crate::timer::sleep(Duration::from_millis(10));
            }
            let _ = crate::timer::sleep(Duration::from_millis(20));
            seen.set(resident_pages(read.get(), TASK_STACK_SIZE));
        });
        run_until_idle(&mut sched).expect("the loop failed");

        assert_ne!(floor.get(), 0, "the program never ran");
        if cfg!(any(target_os = "linux", windows)) {
            // 7 pages in a debug build and 2 in a release one.
            let bound = if cfg!(debug_assertions) { 8 } else { 3 };
            let pages = resident.get();
            assert!(
                (1..=bound).contains(&pages),
                "a sleeping program left {pages} pages of its stack resident"
            );
        }
    }

    /// Where a local of the function `on_spare_stack` runs lives.
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
            "the function's local is inside the spare stack"
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
