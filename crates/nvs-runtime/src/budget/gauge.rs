//! What the whole process holds, published one core at a time, and the server's memory limit a
//! crossing of it is measured against.
//!
//! `rule:http-server/admission-is-arithmetic-not-a-number` admits a request by the memory the
//! process holds now, so something has to say how much that is without a lock and without an
//! atomic on the allocation path. The per-thread balances in the parent module are exact and
//! private to their threads; a sum of per-*request* readings is not, because one core runs several
//! requests interleaved and each reading includes the others' bytes. So each core owns one
//! [`Gauge`] and publishes its thread's [`LIVE`](super::LIVE) plus
//! [`DETACHED`](super::DETACHED), less what its connections hold between requests ([`IDLE`]), into
//! it, and [`held_by_cores`] is the sum of every gauge.
//!
//! **A connection between requests is not a request.** `hyper`'s buffers and the head of the next
//! request are allocated on the connection's turns, and an idle keep-alive connection keeps them.
//! Counted, enough idle connections would refuse every request while no request held anything. So
//! a connection whose place has gone back marks its tree idle
//! ([`Ctx::between_requests`](crate::Ctx::between_requests)), and what that tree holds above the
//! balance its last request started at is moved into [`IDLE`] at each of its parks. The next
//! request's start ([`Ctx::reroot`](crate::Ctx::reroot)) takes it out again, so those bytes count
//! while a request is in flight on the connection. A tree with a child still running, such as
//! after-response work, moves nothing, because that child is a request's work. What an idle tree
//! holds is bounded by `hyper`'s buffer size, and how many connections are open is the proxy's to
//! bound (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`).
//!
//! **A core publishes when its balance has moved by [`STEP`] since its last publish**, and at a
//! request's start and end ([`publish_held`]). The allocation path compares the balance against the
//! last published value, one thread-local load and one compare, and writes nothing shared until
//! the step is crossed. So the sum is stale by at most one step per core, and the store a publish
//! makes goes to a cache line only that core writes.
//!
//! **A thread that is not a core publishes nothing.** A core claims its gauge as it starts
//! ([`CoreGauge::claim`], which `nvs_host::Worker::spawn` calls) and gives it back as it ends. A
//! blocking-pool thread hands what it built to the core that waits for it
//! ([`carry`](super::carry)), so the core's gauge counts those bytes from then on.
//!
//! **A crossing after admission stops the request that grew.** The server sets the limit with
//! [`set_process_limit`]. A publish that raised the sum past it, on a thread where a request is
//! armed, records the verdict beside the request's own refusal and raises the same poll flag its
//! own ceiling does. Work with no request armed is never stopped, because there is nobody to stop:
//! the engine's own bytes are what the reserve under the limit exists for.
//!
//! **A request admitted and not yet grown counts one [`STEP`].** Before it allocates, an admitted
//! request looks like no request at all, so a burst on one core would be admitted whole before any
//! of it allocated. Admission takes a share ([`hold_share`]) for each request it admits, and the
//! core publishes one [`STEP`] per share on top of its balance. A publish where a request armed on
//! this thread grew a whole step gives one share back, and so does a request's end
//! ([`give_share_back`]). The shares are the core's, not each request's: a growth gives back a
//! share whichever request grew, so a request that has not grown can go uncounted by one step,
//! which is the staleness the sum already has. The count never passes the requests admitted on
//! the core, and it is zero once they end. The shares count against the server's limit too, so a
//! request that grows can be stopped up to one step per share early.
//!
//! # What it spends
//!
//! Per `rule:programs/memory-priority`: one padded atomic per core, in blocks of [`CHUNK`] that are
//! never freed and are reused as cores come and go, and three thread-local cells per thread. That
//! is O(cores) and nothing per request. On the allocation path it is one thread-local load and one
//! compare per balance change; a publish is one relaxed store, plus one relaxed load per core when
//! a request is armed and the balance grew. [`IDLE`] is two more thread-local cells, and an idle
//! tree's park pays two relaxed loads, a swap and a compare. Each tree carries three words for it.
//! [`SHARES`] is one more thread-local cell, and admitting or ending a request one more publish.

use std::cell::Cell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};
use std::thread::LocalKey;

use super::{DETACHED, LIVE, POLLED, REFUSED, Refusal};

/// How far a thread's balance moves before its core publishes again: one mebibyte.
pub const STEP: isize = 1 << 20;

/// How many gauges one block holds. The first block is a `static`, so a process with this many
/// cores or fewer allocates nothing for its gauges.
const CHUNK: usize = 16;

/// The limit [`set_process_limit`] stores where there is none. A sum never passes it.
const NO_LIMIT: isize = isize::MAX;

/// One core's published balance, on a cache line of its own so a publish never invalidates a
/// neighbour's line.
#[repr(align(128))]
#[derive(Debug)]
struct Gauge {
    /// The owning thread's balance at its last publish.
    held: AtomicIsize,
    /// Whether a live core owns this gauge.
    owned: AtomicBool,
}

impl Gauge {
    const fn new() -> Self {
        Self {
            held: AtomicIsize::new(0),
            owned: AtomicBool::new(false),
        }
    }
}

/// A block of gauges and the block after it, appended when every gauge before it is owned.
#[derive(Debug)]
struct Chunk {
    gauges: [Gauge; CHUNK],
    next: OnceLock<&'static Chunk>,
}

impl Chunk {
    const fn new() -> Self {
        Self {
            gauges: [const { Gauge::new() }; CHUNK],
            next: OnceLock::new(),
        }
    }
}

/// The first block of gauges.
static FIRST: Chunk = Chunk::new();

/// How many gauges have ever been claimed at once: the reader sums this many and no more.
static SPAN: AtomicUsize = AtomicUsize::new(0);

/// The sum past which a growing request is stopped, or [`NO_LIMIT`].
static LIMIT: AtomicIsize = AtomicIsize::new(NO_LIMIT);

thread_local! {
    /// The gauge this thread publishes into, or `None` for a thread that is not a core.
    static GAUGE: Cell<Option<&'static Gauge>> = const { Cell::new(None) };
    /// [`LIVE`] at this thread's last publish.
    pub(super) static LIVE_MARK: Cell<isize> = const { Cell::new(0) };
    /// [`DETACHED`] at this thread's last publish.
    pub(super) static DETACHED_MARK: Cell<isize> = const { Cell::new(0) };
    /// What the trees of this thread's connections between requests hold, as each last parked:
    /// the module doc's *a connection between requests is not a request*. A publish subtracts it.
    static IDLE: Cell<isize> = const { Cell::new(0) };
    /// [`IDLE`] at this thread's last publish.
    static IDLE_MARK: Cell<isize> = const { Cell::new(0) };
    /// How many requests admitted on this core have not yet grown a whole [`STEP`]: the module
    /// doc's *a request admitted and not yet grown counts one step*. A publish adds a step for each.
    static SHARES: Cell<isize> = const { Cell::new(0) };
}

/// Moves [`IDLE`] by `by`, and publishes once it has moved a whole [`STEP`] since the last
/// publish. Called at a switch, never on the allocation path.
pub(super) fn move_idle(by: isize) {
    if by == 0 {
        return;
    }
    IDLE.with(|idle| idle.set(idle.get().wrapping_add(by)));
    if far_from(IDLE.with(Cell::get), &IDLE_MARK) {
        report(false);
    }
}

/// Whether `now` has moved a whole [`STEP`] away from the value `mark` holds. The allocation
/// path's one compare.
#[inline]
pub(super) fn far_from(now: isize, mark: &'static LocalKey<Cell<isize>>) -> bool {
    now.wrapping_sub(mark.with(Cell::get)).unsigned_abs() >= STEP.unsigned_abs()
}

/// Publishes this thread's balance into its gauge, and stops the armed request where the publish
/// took the sum past the server's limit.
///
/// Out of line, because the allocation path reaches it once per [`STEP`]. It allocates nothing and
/// takes no lock, which is what lets the global allocator be the caller. A thread with no gauge
/// still moves its marks, so it also reaches this once per step and not once per allocation.
#[cold]
#[inline(never)]
pub(super) fn report(grew: bool) {
    let live = LIVE.with(Cell::get);
    let detached = DETACHED.with(Cell::get);
    let idle = IDLE.with(Cell::get);
    LIVE_MARK.with(|mark| mark.set(live));
    DETACHED_MARK.with(|mark| mark.set(detached));
    IDLE_MARK.with(|mark| mark.set(idle));
    // A request that grew a step has published what it holds, so a share goes back.
    if grew && !POLLED.with(Cell::get).is_null() {
        SHARES.with(|shares| shares.set((shares.get() - 1).max(0)));
    }
    let Some(gauge) = GAUGE.with(Cell::get) else {
        return;
    };
    let shares = SHARES.with(Cell::get).wrapping_mul(STEP);
    gauge.held.store(
        live.wrapping_add(detached)
            .wrapping_sub(idle)
            .wrapping_add(shares),
        Ordering::Relaxed,
    );
    if grew {
        stop_if_full(super::live_bytes());
    }
}

/// Records the server's memory as full for the request armed on this thread, when the sum is past
/// the limit.
fn stop_if_full(live: isize) {
    let limit = LIMIT.load(Ordering::Relaxed);
    if limit == NO_LIMIT || POLLED.with(Cell::get).is_null() || held_by_cores() <= limit {
        return;
    }
    REFUSED.with(|refused| {
        if refused.get().is_none() {
            refused.set(Some(Refusal {
                asked: 0,
                live,
                full: true,
            }));
        }
    });
    super::publish();
}

/// Publishes this thread's balance now, whatever it moved by. A core calls it as a request starts
/// and as one ends, so the sum is fresh at the moment admission reads it. It never stops a
/// request.
pub fn publish_held() {
    report(false);
}

/// Counts one more admitted request on this core at [`STEP`] until it grows, and publishes. The
/// server's admission calls it for each request it admits.
pub fn hold_share() {
    SHARES.with(|shares| shares.set(shares.get() + 1));
    report(false);
}

/// Gives back a share [`hold_share`] took, where one is still held, and publishes. The server
/// calls it as an admitted request ends.
pub fn give_share_back() {
    SHARES.with(|shares| shares.set((shares.get() - 1).max(0)));
    report(false);
}

/// What every core holds, as each last published it: the sum of their gauges, in bytes.
///
/// Each gauge is stale by at most [`STEP`]. A gauge whose core has ended reads `0`.
#[must_use]
pub fn held_by_cores() -> isize {
    let mut left = SPAN.load(Ordering::Relaxed);
    let mut chunk = &FIRST;
    let mut sum: isize = 0;
    loop {
        for gauge in chunk.gauges.iter().take(left) {
            sum = sum.wrapping_add(gauge.held.load(Ordering::Relaxed));
        }
        left = left.saturating_sub(CHUNK);
        match chunk.next.get() {
            Some(next) if left > 0 => chunk = next,
            _ => return sum,
        }
    }
}

/// What this thread last published into its gauge, in bytes, or `None` on a thread that is not a
/// core. One term of [`held_by_cores`], which a test reads to leave other cores out.
#[must_use]
pub fn held_by_this_core() -> Option<isize> {
    GAUGE
        .with(Cell::get)
        .map(|gauge| gauge.held.load(Ordering::Relaxed))
}

/// Sets the sum past which a growing request is stopped, or removes it with `None`.
///
/// The server sets it at boot and again at each reload. A limit too large for the balance's type
/// is no limit.
pub fn set_process_limit(bytes: Option<u64>) {
    let limit = bytes.map_or(NO_LIMIT, |bytes| isize::try_from(bytes).unwrap_or(NO_LIMIT));
    LIMIT.store(limit, Ordering::Relaxed);
}

/// The limit [`set_process_limit`] last set, or `None`.
#[must_use]
pub fn process_limit() -> Option<u64> {
    match LIMIT.load(Ordering::Relaxed) {
        NO_LIMIT => None,
        limit => u64::try_from(limit).ok(),
    }
}

/// This thread's gauge, held for as long as the thread runs as a core.
///
/// Not `Send`: the gauge belongs to the thread that claimed it. Dropping it sets the gauge to `0`
/// and gives it back for the next core to claim.
#[derive(Debug)]
pub struct CoreGauge {
    /// `None` where this thread already had a gauge when it claimed again.
    gauge: Option<&'static Gauge>,
    _thread: std::marker::PhantomData<*const ()>,
}

impl CoreGauge {
    /// Claims a free gauge for this thread and publishes its balance into it.
    ///
    /// A thread that already has one keeps it, and the guard this returns gives nothing back.
    #[must_use]
    pub fn claim() -> Self {
        let mut held = Self {
            gauge: None,
            _thread: std::marker::PhantomData,
        };
        if GAUGE.with(Cell::get).is_some() {
            return held;
        }
        let gauge = free_gauge();
        GAUGE.with(|mine| mine.set(Some(gauge)));
        held.gauge = Some(gauge);
        publish_held();
        held
    }
}

impl Drop for CoreGauge {
    fn drop(&mut self) {
        let Some(gauge) = self.gauge else {
            return;
        };
        GAUGE.with(|mine| mine.set(None));
        gauge.held.store(0, Ordering::Relaxed);
        gauge.owned.store(false, Ordering::Release);
    }
}

/// The first gauge no core owns, owned now, appending a block where every one is taken.
fn free_gauge() -> &'static Gauge {
    let mut chunk = &FIRST;
    let mut index = 0;
    loop {
        for gauge in &chunk.gauges {
            index += 1;
            if gauge
                .owned
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                SPAN.fetch_max(index, Ordering::Relaxed);
                return gauge;
            }
        }
        chunk = chunk.next.get_or_init(|| Box::leak(Box::new(Chunk::new())));
    }
}
