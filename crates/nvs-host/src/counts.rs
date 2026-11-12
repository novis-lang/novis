//! Per-thread counts of the system calls a request's I/O makes, for tests in any crate.
//!
//! Three counts, each bumped where the work is done: a socket read that answered `WouldBlock`
//! ([`crate::NvsStream`]'s `read` and `poll_read`), a poke at a core's wake descriptor, the
//! `eventfd` write or its kqueue and completion-port counterparts ([`crate::RemoteWake`]'s
//! delivery), and a streak check of a stream whose reads did not park ([`crate::NvsStream`]'s
//! `read` and `write`). `nvs-server`'s keep-alive count test reads the first two per request, and
//! the bounds it asserts are what a change to either path has to keep or lower.
//!
//! **Debug builds only.** The module exists under `debug_assertions` and nowhere else, so the release
//! binary has no counter on its request path. A `cfg(test)` counter would be the usual shape, but a
//! crate's `cfg(test)` is invisible to another crate's tests. A cargo feature reaches across, and it
//! would build every crate above this one twice, once with it and once without. A wrapper under
//! [`crate::NvsStream`] cannot reach a server's socket, because [`crate::NvsConnection`] names its
//! source types (that type's doc says why). Debug builds and the tests share one set of artifacts,
//! so this costs no second build.
//!
//! The counts are thread-local: the core that serves a connection is the thread that reads and pokes
//! for it, and a test running beside it on another thread adds nothing. A poke made from another
//! thread is counted on that thread.
//!
//! [`StackPages::now`] is a reading beside the counts: the pages resident on the stacks this
//! thread's tasks run on, which `nvs-cli`'s waiting-request test divides per request. Its list of
//! live stacks is `crate::stack`'s, kept under the same `debug_assertions`.
//!
//! Memory: three `u64` per thread, and two words per live task stack, in debug builds.

use std::cell::Cell;

thread_local! {
    static WOULD_BLOCK_READS: Cell<u64> = const { Cell::new(0) };
    static WAKE_POKES: Cell<u64> = const { Cell::new(0) };
    static STREAK_CHECKS: Cell<u64> = const { Cell::new(0) };
}

/// What this thread has counted since it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Socket reads that answered `WouldBlock`.
    pub would_block_reads: u64,
    /// Pokes at a core's wake descriptor.
    pub wake_pokes: u64,
    /// Streak checks of a stream whose reads and writes did not park.
    pub streak_checks: u64,
}

impl Counts {
    /// This thread's counts now.
    #[must_use]
    pub fn now() -> Self {
        Self {
            would_block_reads: WOULD_BLOCK_READS.with(Cell::get),
            wake_pokes: WAKE_POKES.with(Cell::get),
            streak_checks: STREAK_CHECKS.with(Cell::get),
        }
    }

    /// What was counted between `earlier` and `self`.
    #[must_use]
    pub fn since(self, earlier: Self) -> Self {
        Self {
            would_block_reads: self.would_block_reads - earlier.would_block_reads,
            wake_pokes: self.wake_pokes - earlier.wake_pokes,
            streak_checks: self.streak_checks - earlier.streak_checks,
        }
    }
}

/// The pages resident on the stacks this thread's tasks run on now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackPages {
    /// On the stacks of the server's own tasks: the accept loop and each connection.
    pub host: usize,
    /// On the stacks of compiled Novis code: each isolate and the tasks beneath it.
    pub program: usize,
}

impl StackPages {
    /// This thread's reading now. A query of the working set on Windows and `mincore` on Linux;
    /// elsewhere every page of a live stack counts as resident.
    #[must_use]
    pub fn now() -> Self {
        let (host, program) = crate::stack::live_resident_pages();
        Self { host, program }
    }
}

pub(crate) fn would_block_read() {
    WOULD_BLOCK_READS.with(|count| count.set(count.get() + 1));
}

pub(crate) fn wake_poke() {
    WAKE_POKES.with(|count| count.set(count.get() + 1));
}

pub(crate) fn streak_check() {
    STREAK_CHECKS.with(|count| count.set(count.get() + 1));
}
