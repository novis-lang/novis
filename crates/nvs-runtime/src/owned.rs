//! The owned-subject protocol: a `Core` member that may take its first
//! argument's reference instead of borrowing it.
//!
//! A member whose first parameter is an `array<…>`, a `string` or a `bytes`,
//! and whose result is the same kind, owns that subject
//! (`rule:types/a-replaced-value-is-changed-in-place`), and so does
//! `Core\Str::split`, whose last piece may be the subject. [`crate::nvs_helper!`]'s
//! owning form writes such a member as two halves, and the split is what makes
//! the protocol structural rather than a convention each body keeps:
//!
//! - **`check`** reads every argument borrowed, runs every callback, and
//!   returns a plan or a [`Fault`]. On a `Fault` the subject is still the
//!   caller's and unchanged, because nothing has taken it yet.
//! - **`apply`** takes the plan and the owned subject ([`NvsArray`] or
//!   [`NvsStr`]) and has no error in its type. It runs no user code. It may
//!   change the subject in place only where [`OwnedSubject::is_sole`] holds,
//!   and asserts that in a debug build before it does. On a shared subject it
//!   builds only what it keeps, and its handle's drop releases the reference.
//!
//! # Two entry points, one body
//!
//! The form emits the member's own symbol, which keeps the helper ABI every
//! caller uses today: the arguments are borrowed, and `apply` gets a handle of
//! its own ([`OwnedSubject::share`]), so the subject's count is at least two
//! and nothing is changed in place. It also emits `<symbol>_owned`, which takes
//! the reference the caller passed in the first slot: on `OK` that reference is
//! consumed, by the result or by the release, and on `THROWN` or `FATAL` from
//! `check` it is still the caller's.
//!
//! The lowering reaches the owning symbol only through [`Entry::owned`], which
//! each stdlib module's `address` returns beside the borrowed address. So a
//! member the table does not name as owning is only ever called borrowed: a
//! wrong row can miss a move, and can never release a reference twice.
//!
//! **What it spends:** the borrowed entry takes one retain and one release per
//! call, for the handle `apply` gets. Each owning member is compiled twice, once
//! per entry point.

use crate::array::NvsArray;
use crate::string::NvsStr;
use crate::{Ctx, Fault, HelperResult, Value};

/// A value an owning member's `apply` can receive by ownership: an array, or
/// the one heap form `string` and `bytes` share.
pub trait OwnedSubject: Sized {
    /// Whether `value` has this kind, read without taking anything.
    fn fits(value: &Value) -> bool;

    /// Takes the reference `value` carries.
    ///
    /// # Safety
    ///
    /// The caller must have transferred exactly one reference of `value` to
    /// this call, and must not release it again.
    #[expect(
        unsafe_code,
        reason = "taking a transferred reference is the caller's obligation to state"
    )]
    unsafe fn take(value: &Value) -> Option<Self>;

    /// A handle of its own on `value`, which retains it once.
    fn share(value: &Value) -> Option<Self>;

    /// Whether this handle is the only owner, so a change in place is
    /// invisible to everyone else. An immortal literal is never sole.
    fn is_sole(&self) -> bool;
}

impl OwnedSubject for NvsArray {
    fn fits(value: &Value) -> bool {
        value.array_ptr().is_some()
    }

    #[expect(
        unsafe_code,
        reason = "the caller transferred this value's one reference, which the \
                  handle now owns"
    )]
    unsafe fn take(value: &Value) -> Option<Self> {
        value.array_ptr().map(|ptr| unsafe { Self::from_raw(ptr) })
    }

    fn share(value: &Value) -> Option<Self> {
        let ptr = value.array_ptr()?;
        #[expect(
            unsafe_code,
            reason = "an array value owns a reference to a live allocation, and \
                      the borrowed handle is never dropped: the clone takes the \
                      one reference the caller gets"
        )]
        let held = std::mem::ManuallyDrop::new(unsafe { Self::from_raw(ptr) });
        Some(Self::clone(&held))
    }

    fn is_sole(&self) -> bool {
        self.refcount() == 1
    }
}

impl OwnedSubject for NvsStr {
    fn fits(value: &Value) -> bool {
        value.buffer_ptr().is_some()
    }

    #[expect(
        unsafe_code,
        reason = "the caller transferred this value's one reference, which the \
                  handle now owns"
    )]
    unsafe fn take(value: &Value) -> Option<Self> {
        value.buffer_ptr().map(|ptr| unsafe { Self::from_raw(ptr) })
    }

    fn share(value: &Value) -> Option<Self> {
        let ptr = value.buffer_ptr()?;
        #[expect(
            unsafe_code,
            reason = "a string or bytes value owns a reference to a live \
                      allocation, and the borrowed handle is never dropped: the \
                      clone takes the one reference the caller gets"
        )]
        let held = std::mem::ManuallyDrop::new(unsafe { Self::from_raw(ptr) });
        Some(Self::clone(&held))
    }

    fn is_sole(&self) -> bool {
        self.refcount() == 1
    }
}

/// One `Core` symbol's address, and its owning entry where it has one.
///
/// Each stdlib module's `address` returns this, so the borrowed address and
/// the owning one are found in one lookup and cannot disagree.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// The borrowed entry: today's helper ABI.
    pub address: *const u8,
    /// The entry that takes the first argument's reference, for a member
    /// written in [`crate::nvs_helper!`]'s owning form.
    pub owned: Option<Owned>,
}

/// The owning entry of a member written in [`crate::nvs_helper!`]'s owning
/// form.
#[derive(Debug, Clone, Copy)]
pub struct Owned {
    /// The entry's symbol, the member's own symbol with `_owned` after it.
    pub symbol: &'static str,
    /// The entry's address.
    pub address: *const u8,
    /// The argument slot of the subject whose reference the entry takes: `0`
    /// for a static member, and `1` for an instance member, whose receiver is
    /// slot `0`.
    pub slot: usize,
}

impl Entry {
    /// A symbol with only the borrowed entry.
    #[must_use]
    pub const fn borrowed(address: *const u8) -> Self {
        Self {
            address,
            owned: None,
        }
    }
}

/// The message a subject of the wrong kind faults with. A checked program
/// never reaches it, because the first parameter's type is declared.
const NOT_THE_SUBJECT: &str = "internal error: an owning member's subject is not its declared kind";

/// The borrowed entry's body: a handle of its own on the subject, then
/// `check`, then `apply` over that handle.
///
/// The handle is taken before `check`, because `check` runs the callbacks.
/// The caller lends the subject with no reference of its own, as a property
/// read is passed. So a callback that writes the subject's slot would
/// otherwise find a count of 1 and change the array or string under the walk
/// in place, or free it. With the handle, that write copies, and the walk
/// reads what it started with. A failing `check` drops the handle again.
///
/// Kept public because [`crate::nvs_helper!`]'s expansion names it.
///
/// # Errors
///
/// What `check` returns, or a fatal fault for a subject of the wrong kind.
pub fn run_borrowed<S: OwnedSubject, P>(
    ctx: &mut Ctx,
    args: &[Value],
    slot: usize,
    check: impl FnOnce(&mut Ctx, &[Value]) -> Result<P, Fault>,
    apply: impl FnOnce(P, S) -> Value,
) -> HelperResult {
    if !args.get(slot).is_some_and(S::fits) {
        return Err(Fault::fatal(NOT_THE_SUBJECT));
    }
    let subject = args
        .get(slot)
        .and_then(S::share)
        .ok_or_else(|| Fault::fatal(NOT_THE_SUBJECT))?;
    let plan = check(ctx, args)?;
    Ok(apply(plan, subject))
}

/// The owning entry's body: `check` over the borrowed arguments, then `apply`
/// over the reference the caller passed in the subject's slot.
///
/// Kept public because [`crate::nvs_helper!`]'s expansion names it.
///
/// # Errors
///
/// What `check` returns, or a fatal fault for a subject of the wrong kind. On
/// either, the subject's reference is still the caller's.
///
/// # Safety
///
/// The caller must have transferred the reference in `args[slot]` to this
/// call, and must release it again only when this returns an error.
#[expect(
    unsafe_code,
    reason = "the caller's contract is exactly `OwnedSubject::take`'s"
)]
pub unsafe fn run_owned<S: OwnedSubject, P>(
    ctx: &mut Ctx,
    args: &[Value],
    slot: usize,
    check: impl FnOnce(&mut Ctx, &[Value]) -> Result<P, Fault>,
    apply: impl FnOnce(P, S) -> Value,
) -> HelperResult {
    if !args.get(slot).is_some_and(S::fits) {
        return Err(Fault::fatal(NOT_THE_SUBJECT));
    }
    let plan = check(ctx, args)?;
    // `fits` held above and `check` read the arguments borrowed, so the slot
    // still carries the reference the caller transferred.
    let subject = args
        .get(slot)
        .and_then(|value| unsafe { S::take(value) })
        .ok_or_else(|| Fault::fatal(NOT_THE_SUBJECT))?;
    Ok(apply(plan, subject))
}

/// Stops the request where a compiled read finds a lent property's poison
/// tag ([`crate::Tag::LENT_POISON`]). Only a debug build calls it: the read
/// ran while a call had the property's value, so the ownership pass lent a
/// slot this read reaches. `source` is the reading statement's carrier, or
/// null where the read has none.
///
/// Returns `FATAL`, and the compiled frame returns it unchanged. The frame
/// skips its own releases on that return, which leaks only in a debug build
/// that has already found a compiler bug.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the duration of the call,
/// and `source` must be null or a carrier a compiled unit baked.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer and its own baked carrier"
)]
#[deny(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::unreachable,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_lend_trap(ctx: *mut Ctx, source: *const u8) -> i32 {
    // The caller's contract, passed on unchanged.
    let at = unsafe { crate::source::decode(source) }.map_or_else(
        || "an unknown statement".to_owned(),
        |s| crate::source::location(&s),
    );
    // The caller's contract, passed on unchanged.
    unsafe {
        (*ctx).set_pending(format!(
            "a property was read at {at} while a call had its value, \
             so the compiler lent a slot this read reaches"
        ));
    }
    crate::FATAL
}
