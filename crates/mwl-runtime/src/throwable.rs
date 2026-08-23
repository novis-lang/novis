//! The runtime-owned exception value, and the primitives compiled code calls
//! to build, inspect and raise one.
//!
//! # Why the runtime owns it rather than the language
//!
//! [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1 makes
//! `Throwable`/`Exception`/`Error` global, PHP-shaped names that exist without
//! a source declaration. Nothing in MWL declares their fields, so nothing
//! should be able to reach one: a [`ThrowableHeader`] is **opaque** to
//! compiled code, which only ever holds the pointer. `getMessage()` and
//! `getTraceAsString()` are the entry points below, not property reads through
//! an object layout — which is why an exception works in M3 while a
//! user-declared class still waits for M4's object representation.
//!
//! # The backtrace is built as the throw propagates
//!
//! A frame label is pushed by [`mwl_trace_push`] from the *error* path of each
//! compiled frame the throw travels out of — never from a push/pop record kept
//! on the way in. That is the whole reason
//! [ADR 0002](../../../docs/adr/0002-error-propagation.md) can claim a call
//! costs a compare-and-branch: a frame-record scheme would move the cost onto
//! the success path, which is the path that runs. The consequence is visible
//! and deliberate: the trace holds exactly the frames the exception *unwound
//! out of*, so a `catch` in the frame that called the thrower sees the
//! thrower's frame and nothing below it. PHP instead snapshots the whole stack
//! at construction; matching that needs a walk of MWL's own frame chain, which
//! arrives with `Throwable::getTrace()`'s `array<…>` form in M4.
//!
//! # Refcounting
//!
//! [`Rc`] rather than the hand-rolled header [`crate::MwlStr`] uses: a
//! `Throwable` is allocated at most once per throw, never on a hot path, so
//! the second indirection a `Rc<T>` costs buys back the whole of that module's
//! manual `alloc`/`dealloc` surface. `mwl_throwable_retain`/`_release` are
//! `Rc::increment_strong_count`/`decrement_strong_count`, so compiled code's
//! retain/release policy for `mwl_ir::Ty::Throwable` is the same one it
//! already applies to a string.

use std::cell::RefCell;
use std::rc::Rc;

use crate::ctx::Ctx;
use crate::string::{MwlStr, StrHeader};

/// One exception value: its message, and the frames it has unwound out of so
/// far.
///
/// Opaque to compiled code — see the module docs. Both fields are private and
/// there is no public constructor taking them apart, because every path into
/// one goes through [`mwl_exception_new`] or [`crate::Ctx`].
#[derive(Debug)]
pub struct ThrowableHeader {
    /// The message `getMessage()` returns.
    ///
    /// A Rust `String`, not an [`MwlStr`], so [`crate::Ctx::pending`] can hand
    /// back a `&str` without a fallible UTF-8 step on a failure path. The one
    /// cost is that `getMessage()` allocates a fresh MWL string per call,
    /// which is not a hot path by construction.
    message: String,
    /// `#0`-first frame labels, in the order [`mwl_trace_push`] pushed them.
    ///
    /// `RefCell` because a throw in flight may be reachable from a still-live
    /// local (`throw $e;` retains it) as well as from [`crate::Ctx`], so the
    /// push cannot take a unique borrow.
    trace: RefCell<Vec<String>>,
}

impl ThrowableHeader {
    /// A fresh exception with `message` and an empty backtrace.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Rc<Self> {
        Rc::new(Self {
            message: message.into(),
            trace: RefCell::new(Vec::new()),
        })
    }

    /// The message `getMessage()` returns.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Records one more frame the throw has unwound out of.
    pub fn push_frame(&self, label: impl Into<String>) {
        self.trace.borrow_mut().push(label.into());
    }

    /// The backtrace `getTraceAsString()` renders, `#0` first.
    ///
    /// The `#N ` prefix is applied here rather than stored, so a frame label
    /// never has to know its own depth at the point it is pushed.
    #[must_use]
    pub fn trace_as_string(&self) -> String {
        let frames = self.trace.borrow();
        let mut out = String::new();
        for (index, frame) in frames.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            out.push('#');
            out.push_str(&index.to_string());
            out.push(' ');
            out.push_str(frame);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// The primitives compiled code calls
// ---------------------------------------------------------------------------
//
// Same split `crate::string`'s own primitives are on, for the same reason:
// none of these can fail, so none of them wears ADR 0002's checked-return
// shape. Every one is `extern "C"` and never `extern "C-unwind"`.

/// Allocates a fresh exception carrying `message`'s bytes — what
/// `new Exception("…")` lowers to.
///
/// `message` is only read; the caller keeps its own reference.
///
/// # Safety
///
/// `message` must refer to a live MWL string allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw string pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_exception_new(message: *const StrHeader) -> *const ThrowableHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the pointee is live; the borrow ends \
                  before the owned `String` is built from it"
    )]
    let bytes = unsafe { MwlStr::bytes_of(message) };
    Rc::into_raw(ThrowableHeader::new(String::from_utf8_lossy(bytes)))
}

/// Adds a reference — `mwl_ir::InstKind::Retain` for a `Ty::Throwable`
/// operand.
///
/// # Safety
///
/// `ptr` must refer to a live exception allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw pointer whose liveness the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_throwable_retain(ptr: *const ThrowableHeader) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ptr` came from `Rc::into_raw` and is \
                  still live"
    )]
    unsafe {
        Rc::increment_strong_count(ptr);
    }
}

/// Drops a reference, freeing the exception if it was the last —
/// `mwl_ir::InstKind::Release` for a `Ty::Throwable` operand.
///
/// # Safety
///
/// `ptr` must refer to a live exception allocation whose reference has not
/// already been released.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw pointer whose liveness the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_throwable_release(ptr: *const ThrowableHeader) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ptr` came from `Rc::into_raw` and \
                  that this reference has not already been dropped"
    )]
    unsafe {
        Rc::decrement_strong_count(ptr);
    }
}

/// `getMessage()`: a fresh MWL string holding the exception's message.
///
/// # Safety
///
/// `ptr` must refer to a live exception allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw pointer whose liveness the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_throwable_message(ptr: *const ThrowableHeader) -> *mut StrHeader {
    #[expect(unsafe_code, reason = "the caller guarantees the pointee is live")]
    let header = unsafe { &*ptr };
    MwlStr::new(header.message.as_bytes()).into_raw()
}

/// `getTraceAsString()`: a fresh MWL string holding the backtrace built so
/// far, `#0` first.
///
/// # Safety
///
/// `ptr` must refer to a live exception allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw pointer whose liveness the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_throwable_trace(ptr: *const ThrowableHeader) -> *mut StrHeader {
    #[expect(unsafe_code, reason = "the caller guarantees the pointee is live")]
    let header = unsafe { &*ptr };
    MwlStr::new(header.trace_as_string().as_bytes()).into_raw()
}

/// Makes `thrown` this request's pending exception — what MWL's `throw`
/// lowers to, immediately before the frame branches to its own cleanup path.
///
/// Takes ownership of the reference it is handed: `mwl_ir::lower` retains an
/// aliasing `throw $e;` operand first, exactly the way it retains any other
/// value copied into a second durable slot.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the duration of the call, and
/// `thrown` must refer to a live exception allocation whose reference is being
/// transferred here.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context and exception pointers; the \
              contract cannot be expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_raise(ctx: *mut Ctx, thrown: *const ThrowableHeader) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both pointers are valid, and that the \
                  exception's reference is being transferred"
    )]
    unsafe {
        (*ctx).raise(Rc::from_raw(thrown));
    }
}

/// Records one more frame a pending `THROWN` has unwound out of — the
/// backtrace's whole mechanism, called only from a compiled frame's error
/// path.
///
/// A non-`THROWN` `status` is ignored: a resource-limit or internal failure is
/// not a `Throwable` at all
/// ([ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)), so it has
/// no backtrace to grow.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the duration of the call, and
/// `label` must be valid for reads of `len` bytes, or `len` must be zero.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer plus a pointer and a \
              length into its own data section"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_trace_push(ctx: *mut Ctx, label: *const u8, len: usize, status: i32) {
    if status != crate::THROWN {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ctx` is valid and `label` is valid \
                  for `len` bytes; the zero-length case is split out because \
                  `from_raw_parts` rejects a null pointer even for an empty \
                  slice"
    )]
    unsafe {
        let bytes = if len == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(label, len)
        };
        (*ctx).push_frame(&String::from_utf8_lossy(bytes));
    }
}

/// Hands the pending exception to a `catch` clause's bound variable, clearing
/// it from the context — `mwl_ir::InstKind::TakeThrown`'s entry point.
///
/// The caller owns the returned reference and must eventually release it;
/// `mwl_ir::lower` binds it to the clause's local, which the frame's ordinary
/// scope-exit sweep releases.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the duration of the call.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer; the contract cannot be \
              expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_take_thrown(ctx: *mut Ctx) -> *const ThrowableHeader {
    #[expect(unsafe_code, reason = "the caller guarantees `ctx` is valid")]
    let thrown = unsafe { (*ctx).take_thrown() };
    // A `catch` is only ever entered on a `THROWN` status, which by
    // construction leaves something pending — but a null here would be a
    // pointer compiled code then releases, so the defensive case allocates an
    // empty exception rather than trusting that.
    Rc::into_raw(thrown.unwrap_or_else(|| ThrowableHeader::new("")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_exception_has_its_message_and_an_empty_trace() {
        let e = ThrowableHeader::new("boom");
        assert_eq!(e.message(), "boom");
        assert_eq!(e.trace_as_string(), "");
    }

    #[test]
    fn frames_are_numbered_in_push_order() {
        let e = ThrowableHeader::new("traced");
        e.push_frame("Deep::inner() at t.mwl:4");
        e.push_frame("Deep::outer() at t.mwl:8");
        assert_eq!(
            e.trace_as_string(),
            "#0 Deep::inner() at t.mwl:4\n#1 Deep::outer() at t.mwl:8"
        );
    }

    #[test]
    fn the_primitives_round_trip_a_message_through_an_mwl_string() {
        let raw = MwlStr::new(b"from a literal").into_raw();
        #[expect(unsafe_code, reason = "exercising the primitives' own contract")]
        unsafe {
            let thrown = mwl_exception_new(raw);
            let back = MwlStr::from_raw(mwl_throwable_message(thrown));
            assert_eq!(back.as_bytes(), b"from a literal");
            mwl_throwable_release(thrown);
            drop(MwlStr::from_raw(raw));
        }
    }

    #[test]
    fn a_retain_keeps_the_allocation_alive_past_one_release() {
        let e = ThrowableHeader::new("shared");
        let raw = Rc::into_raw(e);
        #[expect(unsafe_code, reason = "exercising the retain/release pair")]
        unsafe {
            mwl_throwable_retain(raw);
            mwl_throwable_release(raw);
            assert_eq!((*raw).message(), "shared");
            mwl_throwable_release(raw);
        }
    }
}
