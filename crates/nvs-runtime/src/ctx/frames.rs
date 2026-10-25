//! The frame records and stop sites of a program compiled under the debug
//! switch (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`).
//!
//! Each function compiled with the switch keeps one [`FrameRecord`] in its own
//! stack frame. On entry it links the record to the head [`Ctx`] holds, and on
//! every return it puts the head back to its caller's record, a throw and a
//! stop included. So the chain from [`Ctx::frames`] is the task's call stack,
//! innermost first, read the same way on every platform and with no unwinding
//! of native frames.
//!
//! Each statement is a stop site, numbered as `nvs_ir::Program::stmt_spans`
//! numbers it. The site writes its number into the record, tests its byte of
//! the unit's [`SiteTable`] and calls [`nvs_debug_site`] when the byte is set,
//! then polls the safepoint word. A unit compiled without the switch has no
//! table, no record and no site.
//!
//! A function the lowering builds itself, such as an exception's constructor
//! or a method a `by` delegation forwards, has no slot table, so it keeps no record and
//! is not in the call stack. That is the view a debugger wants, because it
//! is not code the user wrote.
//!
//! A step is one context's, not the tree's
//! (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). [`Ctx::set_step`]
//! raises a byte on that context alone, and a site calls [`nvs_debug_site`]
//! when its armed byte or that one is set, so a step stops no other task. A
//! return under the switch tests the same byte and calls [`nvs_debug_return`]
//! while it is set. When the frame the step started in returns, that call
//! keeps the value it returned ([`Ctx::take_returned`]) and moves the step to
//! the caller.
//!
//! The stop handler is the request tree's, not one context's
//! (`rule:testing/a-stop-pauses-the-whole-task-tree`). Every context of the
//! tree runs the same one, a child on its parent's core and a `spawn worker`
//! child on another, because each of them shares the tree's state.
//!
//! **What it spends:** under the switch, one record of [`FRAME_RECORD_SIZE`]
//! bytes per call in flight, on the task's own stack, and one byte per
//! statement per unit. A step spends one byte load per site and per return,
//! and while a step is set, one walk of the frame chain per site and per
//! return of the task that steps. A normal run spends one null pointer, two
//! flags and one empty step per context, and one empty lock per request tree.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use super::Ctx;
use crate::Value;

/// One call in flight of a function compiled under the debug switch. Compiled
/// code writes every field, and this crate only reads them.
#[repr(C)]
#[derive(Debug)]
pub struct FrameRecord {
    /// The caller's record, or null for the outermost one.
    caller: *const FrameRecord,
    /// The first of the function's sixteen-byte variable slots, or null for a
    /// function with no variables.
    slots: *const Value,
    /// The function's index in `nvs_ir::Program::functions`.
    function: u32,
    /// The program-wide number of the stop site the call last reached.
    site: u32,
    /// How many slots `slots` points at.
    slot_count: u32,
}

/// The bytes compiled code reserves for one [`FrameRecord`].
pub const FRAME_RECORD_SIZE: usize = std::mem::size_of::<FrameRecord>();
/// Byte offset of [`FrameRecord`]'s link to its caller.
pub const FRAME_CALLER_OFFSET: usize = std::mem::offset_of!(FrameRecord, caller);
/// Byte offset of [`FrameRecord`]'s slot base.
pub const FRAME_SLOTS_OFFSET: usize = std::mem::offset_of!(FrameRecord, slots);
/// Byte offset of [`FrameRecord`]'s function index.
pub const FRAME_FUNCTION_OFFSET: usize = std::mem::offset_of!(FrameRecord, function);
/// Byte offset of [`FrameRecord`]'s current stop site.
pub const FRAME_SITE_OFFSET: usize = std::mem::offset_of!(FrameRecord, site);
/// Byte offset of [`FrameRecord`]'s slot count.
pub const FRAME_SLOT_COUNT_OFFSET: usize = std::mem::offset_of!(FrameRecord, slot_count);

/// What one [`FrameRecord`] says, read out of it while its call is in flight.
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    /// The function's index in `nvs_ir::Program::functions`.
    pub function: u32,
    /// The program-wide number of the stop site the call last reached.
    pub site: u32,
    /// The function's variable slots, one per entry of its slot table. A
    /// slot whose variable has no value at this statement is
    /// [`Value::unset`]. The values are borrowed: reading one takes no
    /// reference.
    pub slots: &'a [Value],
}

/// The records linked to one context, innermost first — see [`Ctx::frames`].
#[derive(Debug)]
pub struct Frames<'a> {
    next: *const FrameRecord,
    _ctx: std::marker::PhantomData<&'a Ctx>,
}

impl<'a> Iterator for Frames<'a> {
    type Item = Frame<'a>;

    fn next(&mut self) -> Option<Frame<'a>> {
        if self.next.is_null() {
            return None;
        }
        #[expect(
            unsafe_code,
            reason = "a non-null head or caller link is a record compiled code linked on entry and \
                      unlinks before its frame returns, so every record on the chain is in a frame \
                      still on this task's stack. The borrow of the context keeps compiled code \
                      from running while the walk reads it."
        )]
        let record: &'a FrameRecord = unsafe { &*self.next };
        self.next = record.caller;
        let slots = if record.slots.is_null() {
            &[][..]
        } else {
            #[expect(
                unsafe_code,
                reason = "compiled code points `slots` at the function's own area of `slot_count` \
                          sixteen-byte values, in the same frame as the record, and sets every \
                          slot to an unset `Value` before it links the record. The \
                          borrow of the context keeps compiled code from writing them while the \
                          slice is read."
            )]
            unsafe {
                std::slice::from_raw_parts(record.slots, record.slot_count as usize)
            }
        };
        Some(Frame {
            function: record.function,
            site: record.site,
            slots,
        })
    }
}

/// Where a step stops, measured from the frame it started in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepKind {
    /// At the next site of the same frame or of a caller.
    Over,
    /// At the next site reached, in any frame.
    In,
    /// At the next site of a caller.
    Out,
}

/// One context's step in progress: its kind, and how deep the frame chain was
/// where it started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// Where it stops.
    pub kind: StepKind,
    /// How many frame records were linked where the step started.
    pub depth: usize,
}

impl Step {
    /// Whether a site reached with `depth` records linked is where this step
    /// stops.
    #[must_use]
    pub fn stops_at(self, depth: usize) -> bool {
        match self.kind {
            StepKind::In => true,
            StepKind::Over => depth <= self.depth,
            StepKind::Out => depth < self.depth,
        }
    }
}

/// The value the frame a step started in returned, kept until the next stop
/// shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Returned {
    /// The returning function's index in `nvs_ir::Program::functions`.
    pub function: u32,
    /// Its value as a record tree, or [`nvs_render::Node::Redacted`] for a
    /// `secret` one.
    pub value: nvs_render::Node,
}

/// Why a context runs its tree's [`DebugStop`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopCause {
    /// Compiled code reached an armed stop site ([`nvs_debug_site`]).
    Site,
    /// A poll found [`super::SafepointFlags::DEBUG_BREAK`] raised on the
    /// tree's word ([`super::nvs_safepoint`]).
    Break,
}

/// What every context of a request tree runs when it reaches an armed stop
/// site, or polls with [`super::SafepointFlags::DEBUG_BREAK`] raised. It
/// returns a status: [`crate::OK`] runs on, and any other ends the request the
/// way a safepoint's does.
///
/// Shared and `Send`, because every context of the tree runs the same one, on
/// whichever core the context is.
///
/// With a handler set, the poll leaves `DEBUG_BREAK` raised, so every task of
/// the tree stops at its next poll. The handler lowers it, through
/// [`super::WeakSafepointView::release_debug_stop`], when the stop ends.
pub type DebugStop = Arc<dyn Fn(&mut Ctx, StopCause) -> i32 + Send + Sync>;

/// The debug state a request tree shares: its [`DebugStop`], and how long the
/// tree has been stopped, which its wall deadline is moved by.
#[derive(Default)]
pub(crate) struct TreeDebug(Mutex<TreeDebugState>);

#[derive(Default)]
struct TreeDebugState {
    stop: Option<DebugStop>,
    /// When the stop in progress began, or `None` while the tree runs.
    held_since: Option<Instant>,
    /// The length of every stop that has ended.
    held_for: Duration,
}

impl TreeDebug {
    fn lock(&self) -> MutexGuard<'_, TreeDebugState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn stop(&self) -> Option<DebugStop> {
        self.lock().stop.clone()
    }

    fn set_stop(&self, stop: Option<DebugStop>) -> Option<DebugStop> {
        std::mem::replace(&mut self.lock().stop, stop)
    }

    /// Starts a hold on the tree's deadlines, and returns whether one was not
    /// already in progress.
    pub(super) fn hold(&self) -> bool {
        let mut state = self.lock();
        if state.held_since.is_some() {
            return false;
        }
        state.held_since = Some(Instant::now());
        true
    }

    /// Ends the hold in progress and adds its length to the total. Returns
    /// whether there was one.
    pub(super) fn release(&self) -> bool {
        let mut state = self.lock();
        let Some(since) = state.held_since.take() else {
            return false;
        };
        state.held_for = state.held_for.saturating_add(since.elapsed());
        true
    }

    /// The length of every hold that has ended, or `None` while one is in
    /// progress, when the tree has no deadline to read.
    pub(super) fn held_for(&self) -> Option<Duration> {
        let state = self.lock();
        state.held_since.is_none().then_some(state.held_for)
    }
}

impl std::fmt::Debug for TreeDebug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.lock();
        f.debug_struct("TreeDebug")
            .field("stop", &state.stop.is_some())
            .field("held_since", &state.held_since)
            .field("held_for", &state.held_for)
            .finish()
    }
}

impl Ctx {
    /// The records of every call in flight compiled under the debug switch,
    /// innermost first. Empty for a context running no such code.
    #[must_use]
    pub fn frames(&self) -> Frames<'_> {
        Frames {
            next: self.frames,
            _ctx: std::marker::PhantomData,
        }
    }

    /// Sets what an armed stop site and a [`super::SafepointFlags::DEBUG_BREAK`]
    /// poll run in every context of this request tree, and returns the one it
    /// replaces. With none set, both run on.
    pub fn set_debug_stop(&mut self, stop: Option<DebugStop>) -> Option<DebugStop> {
        self.tree.debug.set_stop(stop)
    }

    /// How many frame records are linked: the depth of the call in flight.
    #[must_use]
    pub fn frame_depth(&self) -> usize {
        self.frames().count()
    }

    /// Sets this context's step, or ends it with `None`. Only this context's
    /// sites read it, so no other task of the tree stops for it.
    pub fn set_step(&mut self, step: Option<Step>) {
        self.step = step;
        self.stepping = u8::from(step.is_some());
    }

    /// This context's step in progress.
    #[must_use]
    pub fn step(&self) -> Option<Step> {
        self.step
    }

    /// The value the step's frame returned since the last call, if it has
    /// returned.
    pub fn take_returned(&mut self) -> Option<Returned> {
        self.returned.take()
    }

    /// Whether this tree has a [`DebugStop`] set.
    pub(crate) fn has_debug_stop(&self) -> bool {
        self.tree.debug.stop().is_some()
    }

    /// Holds this tree's deadlines while it is stopped: from here until
    /// [`super::WeakSafepointView::release_debug_stop`], its wall deadline is
    /// not reached, and the time between is added to it
    /// (`rule:testing/a-stop-pauses-the-whole-task-tree`).
    ///
    /// CPU time needs no hold: a stopped task is parked or blocked, so its
    /// thread's clock does not charge it.
    pub fn hold_deadlines(&self) {
        self.tree.hold_deadlines();
    }

    /// Runs the tree's [`DebugStop`], and returns its status, or
    /// [`crate::OK`] when none is set or this context is already inside it.
    /// Both ways a task stops come here: an armed site ([`nvs_debug_site`])
    /// and a `DEBUG_BREAK` poll ([`super::nvs_safepoint`]).
    ///
    /// The handler runs inside a [`crate::HelperFrame`], because it may park
    /// the task with compiled frames above it, and a host may not unwind such
    /// a stack.
    pub(crate) fn run_debug_stop(&mut self, cause: StopCause) -> i32 {
        if self.debug_stopping {
            return crate::OK;
        }
        let Some(stop) = self.tree.debug.stop() else {
            return crate::OK;
        };
        self.debug_stopping = true;
        let status = {
            let _frame = crate::HelperFrame::enter();
            stop(self, cause)
        };
        self.debug_stopping = false;
        status
    }
}

/// The armed byte of every stop site in one unit, indexed by the program-wide
/// statement number. Compiled code reads a byte at each site, and a debug
/// controller on any thread sets and clears them.
#[derive(Debug)]
pub struct SiteTable {
    armed: Box<[AtomicU8]>,
}

impl SiteTable {
    /// A table of `sites` bytes, none of them armed.
    #[must_use]
    pub fn new(sites: usize) -> Self {
        Self {
            armed: (0..sites).map(|_| AtomicU8::new(0)).collect(),
        }
    }

    /// How many sites the table has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.armed.len()
    }

    /// Whether the table has no sites.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.armed.is_empty()
    }

    /// Arms or disarms `site`. Returns `false` for a site past the table.
    pub fn arm(&self, site: u32, armed: bool) -> bool {
        let Some(byte) = usize::try_from(site)
            .ok()
            .and_then(|site| self.armed.get(site))
        else {
            return false;
        };
        byte.store(u8::from(armed), Ordering::Relaxed);
        true
    }

    /// Whether `site` is armed.
    #[must_use]
    pub fn is_armed(&self, site: u32) -> bool {
        usize::try_from(site)
            .ok()
            .and_then(|site| self.armed.get(site))
            .is_some_and(|byte| byte.load(Ordering::Relaxed) != 0)
    }

    /// The address compiled code adds a site number to. An `AtomicU8` has the
    /// layout of a `u8`, so a plain byte load there reads the armed byte.
    #[must_use]
    pub fn base(&self) -> usize {
        self.armed.as_ptr().expose_provenance()
    }
}

/// The slow path of an armed stop site: runs the context's [`DebugStop`], and
/// returns its status. Returns [`crate::OK`] when none is set.
///
/// # Safety
///
/// `ctx` must be non-null, aligned, and valid for the duration of the call.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer; the contract cannot be \
              expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_debug_site(ctx: *mut Ctx) -> i32 {
    #[expect(unsafe_code, reason = "the caller's contract, passed on unchanged")]
    unsafe {
        crate::abi::contain_panic(ctx, |ctx| ctx.run_debug_stop(StopCause::Site))
    }
}

/// A return under the debug switch while [`Ctx::set_step`]'s byte is set,
/// before the frame unlinks its record. When the returning frame is the one
/// the step started in, keeps the value at `out` ([`Ctx::take_returned`]) and
/// moves the step to the caller: a step over or out then stops at the
/// caller's next site. `secret` is nonzero for a function that may return a
/// `secret` value, which is kept redacted.
///
/// # Safety
///
/// `ctx` must be non-null, aligned, and valid for the duration of the call,
/// and `out` must point at the initialized [`Value`] the function returns.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context and its out-slot; the contract cannot be \
              expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_debug_return(ctx: *mut Ctx, out: *const Value, secret: i32) {
    #[expect(unsafe_code, reason = "the caller's contract, passed on unchanged")]
    let (ctx, value) = unsafe { (&mut *ctx, *out) };
    let Some(step) = ctx.step else {
        return;
    };
    let mut frames = ctx.frames();
    let Some(function) = frames.next().map(|frame| frame.function) else {
        return;
    };
    let depth = 1 + frames.count();
    if depth != step.depth {
        return;
    }
    let value = if secret == 0 {
        crate::record::node(value)
    } else {
        nvs_render::Node::Redacted
    };
    ctx.returned = Some(Returned { function, value });
    let kind = match step.kind {
        StepKind::In => StepKind::In,
        StepKind::Over | StepKind::Out => StepKind::Over,
    };
    ctx.step = Some(Step {
        kind,
        depth: depth - 1,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_stops_in_its_frame_a_deeper_one_or_a_caller_by_its_kind() {
        let step = |kind| Step { kind, depth: 3 };
        assert!(step(StepKind::In).stops_at(4));
        assert!(step(StepKind::Over).stops_at(3) && step(StepKind::Over).stops_at(2));
        assert!(!step(StepKind::Over).stops_at(4));
        assert!(step(StepKind::Out).stops_at(2) && !step(StepKind::Out).stops_at(3));
    }

    #[test]
    fn a_site_table_arms_and_disarms_one_site() {
        let table = SiteTable::new(3);
        assert!(table.arm(1, true));
        assert!(table.is_armed(1) && !table.is_armed(0) && !table.is_armed(2));
        assert!(table.arm(1, false));
        assert!(!table.is_armed(1));
        assert!(!table.arm(3, true));
    }
}
