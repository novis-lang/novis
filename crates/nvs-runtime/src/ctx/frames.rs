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
//! **What it spends:** under the switch, one record of [`FRAME_RECORD_SIZE`]
//! bytes per call in flight, on the task's own stack, and one byte per
//! statement per unit. A normal run spends one null pointer and one empty
//! handler per context.

use std::sync::atomic::{AtomicU8, Ordering};

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

/// What a context runs when compiled code reaches an armed stop site. It
/// returns a status: [`crate::OK`] runs on, and any other ends the request the
/// way a safepoint's does.
pub type DebugStop = Box<dyn FnMut(&mut Ctx) -> i32>;

/// The [`DebugStop`] a context holds, if any. A type of its own so the context
/// can print itself: a closure has no `Debug`.
#[derive(Default)]
pub(crate) struct StopSlot(Option<DebugStop>);

impl std::fmt::Debug for StopSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() { "Some(..)" } else { "None" })
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
    /// poll run, and returns the one it replaces. With none set, both run on.
    pub fn set_debug_stop(&mut self, stop: Option<DebugStop>) -> Option<DebugStop> {
        std::mem::replace(&mut self.debug_stop.0, stop)
    }

    /// Runs the [`DebugStop`] this context has, and returns its status, or
    /// [`crate::OK`] when none is set. Both ways a task stops come here: an
    /// armed site ([`nvs_debug_site`]) and a `DEBUG_BREAK` poll
    /// ([`super::nvs_safepoint`]).
    ///
    /// The handler runs inside a [`crate::HelperFrame`], because it may park
    /// the task with compiled frames above it, and a host may not unwind such
    /// a stack.
    pub(crate) fn run_debug_stop(&mut self) -> i32 {
        let Some(mut stop) = self.debug_stop.0.take() else {
            return crate::OK;
        };
        let status = {
            let _frame = crate::HelperFrame::enter();
            stop(self)
        };
        // A handler that set another one while it ran keeps that one.
        if self.debug_stop.0.is_none() {
            self.debug_stop.0 = Some(stop);
        }
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
        crate::abi::contain_panic(ctx, Ctx::run_debug_stop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
