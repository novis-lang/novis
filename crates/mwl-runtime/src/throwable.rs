//! The pending-exception value, and the primitives compiled code calls to
//! raise, inspect and re-take one.
//!
//! # An exception is an ordinary object
//!
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) § 10
//! makes `Throwable` the root of a small class tree whose members are
//! *readonly properties* — `message`, `previous`, `backtrace`, `location` —
//! not `getX()` accessors, and makes user classes extend it directly. So an
//! exception is a [`crate::MwlObj`] like any other: allocated by
//! `mwl_object_new`, refcounted by `mwl_object_retain`/`mwl_object_release`,
//! read by an ordinary `FieldGet`. There is no second representation here any
//! more, and no `Ty::Throwable` in the IR.
//!
//! What survives is the part the *runtime* owns: which object is pending, and
//! growing its backtrace as a throw travels. Both need to reach two slots of
//! an object they otherwise know nothing about, which is what [`MESSAGE_SLOT`]
//! and friends are for.
//!
//! # The slot indices are load-bearing
//!
//! `crate::object` lays a subclass's slots out *after* its parent's, and
//! `Throwable` is the root of every exception class in existence — so
//! `message` is slot 0 and `backtrace` slot 2 for `LogicError`, for a user's
//! `ConfigError extends Throwable`, and for anything else that can be thrown.
//! `mwl_hir::errors::PROPERTIES` is the one home for that order; the constants
//! below restate the two indices this crate needs because `mwl-runtime`
//! depends on nothing (see [`crate`]'s own docs), and
//! `mwl-codegen`'s `the_runtime_and_the_compiler_agree_on_every_throwable_slot`
//! is the test that holds the two together.
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
//! at construction; matching that needs a walk of MWL's own frame chain.

use crate::ctx::Ctx;
use crate::object::{ClassDesc, MwlObj, ObjHeader};
use crate::{MwlArray, MwlStr, Value};

/// The slot `Throwable::$message` occupies — see this module's docs.
pub const MESSAGE_SLOT: usize = 0;
/// The slot `Throwable::$previous` occupies.
pub const PREVIOUS_SLOT: usize = 1;
/// The slot `Throwable::$backtrace` occupies.
pub const BACKTRACE_SLOT: usize = 2;
/// The slot `Throwable::$location` occupies.
pub const LOCATION_SLOT: usize = 3;

/// How many slots any exception class has at minimum — every one of the four
/// above. A descriptor with fewer is not an exception class, and every
/// operation here refuses it rather than reading past the allocation.
pub const SLOT_COUNT: usize = 4;

/// The slot `ParseError::$issues` occupies — the one property any class in the
/// tree declares beyond the root's four
/// ([ADR 0071](../../../docs/adr/0071-derived-codecs.md) § 5).
///
/// `ParseError` inherits exactly [`SLOT_COUNT`] slots and adds this one, so a
/// descriptor with more than [`SLOT_COUNT`] fields is the only shape it can
/// take. `mwl_hir::errors::ISSUES_SLOT` is the compiler's copy, and
/// `mwl-codegen`'s `the_runtime_and_the_compiler_agree_on_every_throwable_slot`
/// is what holds the two together.
pub const ISSUES_SLOT: usize = SLOT_COUNT;

/// Which of [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md)
/// § 10's classes a runtime helper's failure lands in.
///
/// A closed enum rather than a `&'static str` a helper writes, for
/// `mwl_stdlib::registry::CoreTy`'s reason: a misspelled class name would be
/// a silent *runtime* miss — the `catch` clause that was meant to handle it
/// simply would not match — rather than a compile error. The roster is spec
/// § 10's tree minus its root, since a helper that means "anything at all"
/// means [`Self::Runtime`].
///
/// `mwl-runtime` depends on nothing (see [`crate`]'s own docs), so the names
/// below restate `mwl_hir::errors::TREE`'s; `mwl-codegen`'s
/// `every_thrown_class_is_in_the_compiler_s_exception_tree` is the test that
/// holds the two together, exactly as
/// `the_runtime_and_the_compiler_agree_on_every_throwable_slot` holds the slot
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ThrownClass {
    /// `RuntimeError` — "the world said no", which is what a helper failure
    /// with nothing more specific to say is, and the class
    /// [`Ctx::set_runtime_error_class`] installs.
    #[default]
    Runtime,
    /// `LogicError` — a bug in the program: a bad argument, a bad state, a
    /// bad index.
    Logic,
    /// `IOError` — a file, socket or process failed.
    Io,
    /// `ParseError` — input did not match a format this code declared, which
    /// is what `Core\Json::decode` and `Core\Time::parse` answer with.
    Parse,
    /// `TimeoutError` — a deadline passed.
    Timeout,
    /// `ArithmeticError` — overflow (ADR 0007), division by zero.
    Arithmetic,
}

impl ThrownClass {
    /// The class's name, as `mwl_hir::errors::TREE` spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Runtime => "RuntimeError",
            Self::Logic => "LogicError",
            Self::Io => "IOError",
            Self::Parse => "ParseError",
            Self::Timeout => "TimeoutError",
            Self::Arithmetic => "ArithmeticError",
        }
    }

    /// Every class in the roster — what a guard test iterates.
    pub const ALL: &'static [Self] = &[
        Self::Runtime,
        Self::Logic,
        Self::Io,
        Self::Parse,
        Self::Timeout,
        Self::Arithmetic,
    ];
}

/// One owned reference to a pending exception object.
///
/// Exists so [`crate::Ctx`] can hold a raw `*mut ObjHeader` without either
/// leaking it or hand-writing a release at every early return — the same job
/// [`MwlObj`] does, except that this one may be *null*, which is the state a
/// failure with no object behind it (a helper fault whose class was never
/// installed) leaves.
#[derive(Debug)]
pub struct Thrown {
    /// Null, or one owned reference.
    ptr: *mut ObjHeader,
}

impl Thrown {
    /// Builds a fresh exception of `class` carrying `message`, with an empty
    /// backtrace — what a helper's bare-message failure is promoted to, and
    /// what `mwl run --fault-inject` produces.
    ///
    /// Returns a null [`Thrown`] if `class` is null or describes fewer than
    /// [`SLOT_COUNT`] slots, which is the "no exception class was installed"
    /// case [`Ctx::set_runtime_error_class`] documents.
    ///
    /// # Safety
    ///
    /// `class` must be null or refer to a live class descriptor that outlives
    /// every instance made from it.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the descriptor's liveness is the caller's obligation and \
                  cannot be expressed in the signature"
    )]
    pub unsafe fn new(class: *const ClassDesc, message: &str) -> Self {
        #[expect(unsafe_code, reason = "forwarding this function's own contract")]
        unsafe {
            Self::new_as(class, ThrownClass::Runtime, message, None)
        }
    }

    /// [`Self::new`], plus the one property a class below the root declares:
    /// [ADR 0071](../../../docs/adr/0071-derived-codecs.md) § 5's `issues` on
    /// `ParseError`, which is filled with `issues` — or with an empty array
    /// when a thrower has none to report, since the property is declared
    /// `array<Issue>` rather than `?array<Issue>` and reading `null` out of it
    /// would be a type the checker ruled out.
    ///
    /// `thrown` rather than the descriptor's name decides that: a name compare
    /// on every promotion would put a string equality on the throw path, and a
    /// user's `class ConfigError extends Throwable { public int $code; }` also
    /// has a fifth slot — one that must **not** be written here.
    ///
    /// Takes over `issues`' reference; releases it if there is no slot to put
    /// it in (a null or too-narrow descriptor, which is
    /// [`Ctx::set_runtime_error_class`]'s "nothing installed" case).
    ///
    /// # Safety
    ///
    /// `class` must be null or refer to a live class descriptor that outlives
    /// every instance made from it, and `issues` must be a value whose
    /// reference is being transferred here.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the descriptor's liveness and the value's reference are both \
                  the caller's obligation and cannot be expressed in the signature"
    )]
    pub unsafe fn new_as(
        class: *const ClassDesc,
        thrown: ThrownClass,
        message: &str,
        issues: Option<Value>,
    ) -> Self {
        let count = if class.is_null() {
            0
        } else {
            #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
            unsafe {
                (*class).field_count()
            }
        };
        if count < SLOT_COUNT {
            if let Some(issues) = issues {
                #[expect(
                    unsafe_code,
                    reason = "the caller transferred this reference and there is \
                              no slot to hand it on to"
                )]
                unsafe {
                    issues.release();
                }
            }
            return Self::none();
        }
        #[expect(
            unsafe_code,
            reason = "the caller guarantees the descriptor outlives the instance"
        )]
        let obj = unsafe { MwlObj::new(class) };
        obj.set_field(MESSAGE_SLOT, Value::str(MwlStr::new(message.as_bytes())));
        obj.set_field(BACKTRACE_SLOT, Value::array(MwlArray::new()));
        obj.set_field(LOCATION_SLOT, Value::str(MwlStr::new(b"")));
        if thrown == ThrownClass::Parse && count > ISSUES_SLOT {
            obj.set_field(
                ISSUES_SLOT,
                issues.unwrap_or_else(|| Value::array(MwlArray::new())),
            );
        } else if let Some(issues) = issues {
            #[expect(
                unsafe_code,
                reason = "the caller transferred this reference and this class \
                          declares no slot to hand it on to"
            )]
            unsafe {
                issues.release();
            }
        }
        Self {
            ptr: obj.into_raw(),
        }
    }

    /// The absent exception — what a failure with no object behind it holds.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
        }
    }

    /// Whether there is no object here at all.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.ptr.is_null()
    }

    /// Takes over one reference to `ptr`, which may be null.
    ///
    /// # Safety
    ///
    /// `ptr` must be null or refer to a live object allocation whose
    /// reference is being transferred here.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "reclaiming a reference is the caller's obligation to state"
    )]
    pub const unsafe fn from_raw(ptr: *mut ObjHeader) -> Self {
        Self { ptr }
    }

    /// Gives the reference back, leaving nothing behind to release.
    #[must_use]
    pub fn into_raw(self) -> *mut ObjHeader {
        let ptr = self.ptr;
        std::mem::forget(self);
        ptr
    }

    /// A borrowed handle on the object, or `None` if there is none.
    ///
    /// [`std::mem::ManuallyDrop`] because [`MwlObj::from_raw`] takes over a
    /// reference this value still owns.
    fn borrow(&self) -> Option<std::mem::ManuallyDrop<MwlObj>> {
        if self.ptr.is_null() {
            return None;
        }
        #[expect(
            unsafe_code,
            reason = "the pointer is non-null and this value owns a reference \
                      to it; the handle is never dropped, so the reference is \
                      not released twice"
        )]
        Some(std::mem::ManuallyDrop::new(unsafe {
            MwlObj::from_raw(self.ptr)
        }))
    }

    /// The exception's `message` property, as an owned string.
    ///
    /// Empty for an absent exception, or for one whose slot somehow does not
    /// hold a string — a diagnostic path never gets to be the thing that
    /// crashes.
    #[must_use]
    pub fn message(&self) -> String {
        let Some(obj) = self.borrow() else {
            return String::new();
        };
        if obj.field_count() < SLOT_COUNT {
            return String::new();
        }
        let slot = obj.field(MESSAGE_SLOT);
        slot.as_str_bytes()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
            .unwrap_or_default()
    }

    /// The `backtrace` property rendered `#0`-first, the form
    /// `mwl run`'s uncaught report prints.
    ///
    /// The `#N ` prefix is applied here rather than stored, so a frame label
    /// never has to know its own depth at the point it is pushed.
    #[must_use]
    pub fn trace_as_string(&self) -> String {
        let mut out = String::new();
        for (index, frame) in self.frames().iter().enumerate() {
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

    /// Every backtrace frame label, in push order.
    #[must_use]
    pub fn frames(&self) -> Vec<String> {
        let Some(obj) = self.borrow() else {
            return Vec::new();
        };
        if obj.field_count() < SLOT_COUNT {
            return Vec::new();
        }
        let Some(array) = obj.field(BACKTRACE_SLOT).array_ptr() else {
            return Vec::new();
        };
        #[expect(
            unsafe_code,
            reason = "the slot holds one reference the object owns; the handle \
                      is never dropped, so that reference is not released here"
        )]
        let handle = std::mem::ManuallyDrop::new(unsafe { MwlArray::from_raw(array) });
        let mut out = Vec::new();
        let mut slot = 0;
        while let Some(found) = handle.next_slot(slot) {
            if let Some(value) = handle.value_at(found)
                && let Some(bytes) = value.as_str_bytes()
            {
                out.push(String::from_utf8_lossy(bytes).into_owned());
            }
            slot = found + 1;
        }
        out
    }

    /// Appends one frame label to the `backtrace` property, in place.
    ///
    /// The field's own reference is *moved out* of the slot and back in, so
    /// the array's refcount stays at one and copy-on-write never separates —
    /// retaining it first would copy the whole trace once per frame the throw
    /// unwinds through.
    pub fn push_frame(&self, label: &str) {
        let Some(obj) = self.borrow() else {
            return;
        };
        if obj.field_count() < SLOT_COUNT {
            return;
        }
        let held = obj.take_field(BACKTRACE_SLOT);
        let Some(array) = held.array_ptr() else {
            // Not an array: put back exactly what was there, unchanged.
            obj.set_field(BACKTRACE_SLOT, held);
            return;
        };
        #[expect(
            unsafe_code,
            reason = "`take_field` transferred the slot's own reference here, \
                      and `into_raw` hands it straight back to the slot"
        )]
        let mut handle = unsafe { MwlArray::from_raw(array) };
        handle.append(Value::str(MwlStr::new(label.as_bytes())));
        obj.set_field(BACKTRACE_SLOT, Value::from_array_ptr(handle.into_raw()));
    }
}

impl Drop for Thrown {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        #[expect(
            unsafe_code,
            reason = "this value owned exactly one reference to a live \
                      allocation, and this is the one place it is given up"
        )]
        unsafe {
            drop(MwlObj::from_raw(self.ptr));
        }
    }
}

// ---------------------------------------------------------------------------
// The primitives compiled code calls
// ---------------------------------------------------------------------------
//
// Same split `crate::string`'s own primitives are on, for the same reason:
// none of these can fail, so none of them wears ADR 0002's checked-return
// shape. Every one is `extern "C"` and never `extern "C-unwind"`.

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
/// `thrown` must be null or refer to a live object allocation whose reference
/// is being transferred here.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context and exception pointers; the \
              contract cannot be expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_raise(ctx: *mut Ctx, thrown: *mut ObjHeader) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees both pointers are valid, and that the \
                  exception's reference is being transferred"
    )]
    unsafe {
        (*ctx).raise(Thrown::from_raw(thrown));
    }
}

/// Builds an exception of `class` carrying `message` and makes it this
/// request's pending one — [`mwl_raise`] for a throw compiled code raises by
/// itself, with no MWL `new` behind it.
///
/// The one caller today is `mwl-codegen`'s integer `%`, whose zero divisor
/// must throw spec § 10's `ArithmeticError` rather than trap the process. That
/// site has no MWL expression to construct the exception from, and cannot go
/// through a helper's [`crate::Fault`] either: a helper failure carries a bare
/// message, which `run_helper` promotes to `RuntimeError` and only
/// `RuntimeError` (see [`Ctx::set_runtime_error_class`]). Naming the class is
/// the whole point here, and compiled code already knows the descriptor's
/// address as a constant — so it passes it.
///
/// A null or too-small `class` leaves the pending failure with no object
/// behind it, exactly as [`Thrown::new`] documents; the status the caller
/// returns is unaffected.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the duration of the call;
/// `class` must be null or refer to a live class descriptor that outlives the
/// instance made from it; and `message` must be valid for reads of `len`
/// bytes, or `len` must be zero.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context and descriptor pointers plus a \
              pointer and a length into its own data section"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_raise_new(
    ctx: *mut Ctx,
    class: *const ClassDesc,
    message: *const u8,
    len: usize,
) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `message` is valid for `len` bytes; \
                  the zero-length case is split out because `from_raw_parts` \
                  rejects a null pointer even for an empty slice"
    )]
    let bytes = unsafe {
        if len == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(message, len)
        }
    };
    let text = String::from_utf8_lossy(bytes);
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the descriptor outlives the instance"
    )]
    let thrown = unsafe { Thrown::new(class, &text) };
    #[expect(unsafe_code, reason = "the caller guarantees `ctx` is valid")]
    unsafe {
        (*ctx).raise(thrown);
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

/// Hands the pending exception to a `catch` clause's dispatch, clearing it
/// from the context — `mwl_ir::InstKind::TakeThrown`'s entry point.
///
/// The caller owns the returned reference and must eventually release it.
/// It may be **null**: a helper's bare-message failure has no object behind
/// it unless [`Ctx::set_runtime_error_class`] installed a class to build one
/// from. Every operation a `catch` dispatch performs on the result is
/// null-tolerant — `mwl_object_instanceof` answers `false`, so no clause
/// matches and the throw is re-raised unchanged.
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
pub unsafe extern "C" fn mwl_take_thrown(ctx: *mut Ctx) -> *mut ObjHeader {
    #[expect(unsafe_code, reason = "the caller guarantees `ctx` is valid")]
    let thrown = unsafe { (*ctx).take_thrown() };
    thrown.into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ClassTable;

    /// The [`SLOT_COUNT`] slots in slot order, spelled the way
    /// `mwl_types::error_lib` declares them.
    const SLOT_NAMES: [&str; SLOT_COUNT] = ["message", "previous", "backtrace", "location"];

    /// A class table shaped like the seeded exception tree: `Throwable` with
    /// its four slots, and one subclass of it with none of its own.
    fn tree() -> (ClassTable, *const ClassDesc, *const ClassDesc) {
        let mut table = ClassTable::new();
        let root = table.define("Throwable", &SLOT_NAMES, &[]);
        let leaf = table.define("LogicError", &SLOT_NAMES, &[root]);
        let (root, leaf) = (table.desc(root), table.desc(leaf));
        (table, root, leaf)
    }

    #[test]
    fn a_fresh_exception_has_its_message_and_an_empty_trace() {
        let (_table, root, _) = tree();
        #[expect(unsafe_code, reason = "the table outlives the instance")]
        let e = unsafe { Thrown::new(root, "boom") };
        assert_eq!(e.message(), "boom");
        assert_eq!(e.trace_as_string(), "");
    }

    #[test]
    fn frames_are_numbered_in_push_order() {
        let (_table, _, leaf) = tree();
        #[expect(unsafe_code, reason = "the table outlives the instance")]
        let e = unsafe { Thrown::new(leaf, "traced") };
        e.push_frame("Deep::inner() at t.mwl:4");
        e.push_frame("Deep::outer() at t.mwl:8");
        assert_eq!(
            e.trace_as_string(),
            "#0 Deep::inner() at t.mwl:4\n#1 Deep::outer() at t.mwl:8"
        );
        assert_eq!(e.frames().len(), 2);
    }

    #[test]
    fn growing_the_backtrace_never_separates_the_array() {
        // The append path moves the slot's own reference out and back, so the
        // array stays uniquely owned — otherwise every frame would copy the
        // whole trace.
        let (_table, root, _) = tree();
        #[expect(unsafe_code, reason = "the table outlives the instance")]
        let e = unsafe { Thrown::new(root, "m") };
        for i in 0..64 {
            e.push_frame(&format!("f{i}"));
        }
        let frames = e.frames();
        assert_eq!(frames.len(), 64);
        assert_eq!(frames[0], "f0");
        assert_eq!(frames[63], "f63");
    }

    #[test]
    fn an_absent_exception_answers_every_query_without_reading_anything() {
        let none = Thrown::none();
        assert!(none.is_none());
        assert_eq!(none.message(), "");
        assert_eq!(none.trace_as_string(), "");
        assert!(none.frames().is_empty());
        none.push_frame("ignored");
    }

    #[test]
    fn a_class_with_too_few_slots_is_refused_rather_than_written_past() {
        let mut table = ClassTable::new();
        let narrow = table.define("NotAnException", &["message"], &[]);
        #[expect(unsafe_code, reason = "the table outlives the call")]
        let e = unsafe { Thrown::new(table.desc(narrow), "boom") };
        assert!(e.is_none());
    }
}
