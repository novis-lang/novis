//! Reaching a compiled instance member from native code, by name.
//!
//! A helper that has to ask an *object* something — `Comparable::compareTo`
//! for [ADR 0013](../../../docs/adr/0013-comparable-interface.md), the
//! `iterate`/`advance`/`current` trio for
//! [ADR 0053](../../../docs/adr/0053-iteration-and-generators.md) — cannot
//! name a compiled function: the class is one this crate and `mwl-stdlib` know
//! nothing about, and the interface member is a bodiless declaration
//! (`mwl_types::iter_lib`), so no symbol exists for a call site to resolve.
//! The object's own descriptor carries the table that answers it, and
//! [`crate::call_closure`] already reaches a closure's `invoke` through
//! exactly this lookup with the name fixed.
//!
//! So this is that lookup with the name as an argument, and it is the same
//! dispatch a `foreach` or a `$value->compareTo($other)` in source performs —
//! one indirect call through the class descriptor, no allocation, and nothing
//! cached, because a descriptor's method table is a compile-time constant of
//! the unit that declared it.
//!
//! # What a caller owes
//!
//! [`call_method`] retains the receiver and every argument before it calls,
//! because a compiled MWL function releases its parameters — the same
//! reconciliation [`crate::call_closure`] performs, and for the same reason.
//! The [`Value`] it hands back is a fresh reference this frame owns.

use crate::abi::{Fault, MwlFn, OK};
use crate::ctx::Ctx;
use crate::object::{ClassDesc, MwlObj};
use crate::value::Value;

/// The compiled address of `receiver`'s `name`, or `None` when its class
/// declares no such method.
///
/// `what` names the caller for a fault message and is never shown to a program
/// that is behaving.
///
/// # Errors
///
/// [`Fault::Fatal`] when `receiver` is not an object, or is one with no class
/// descriptor — both engine faults rather than anything a program can cause.
pub fn method_address(receiver: Value, name: &str, what: &str) -> Result<Option<*const u8>, Fault> {
    let ptr = receiver.obj_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: {what} looked for `{name}` on tag {}",
            receiver.tag_byte()
        ))
    })?;
    #[expect(
        unsafe_code,
        reason = "the caller owns a reference to this object, so the \
                  allocation and its descriptor are both live for this read"
    )]
    let desc: *const ClassDesc = unsafe { MwlObj::class_of(ptr) };
    if desc.is_null() {
        return Err(Fault::fatal(format!(
            "internal error: {what} was handed an object with no class descriptor"
        )));
    }
    #[expect(
        unsafe_code,
        reason = "just checked the descriptor is non-null, and it is owned by \
                  the compiled unit's class table for that unit's whole life"
    )]
    Ok(unsafe { &*desc }.method(name))
}

/// Calls `receiver`'s `name` with `args` past the receiver, or answers `None`
/// when its class declares no such member.
///
/// The one entry point from outside this crate: the address form below takes a
/// raw pointer, which is this crate's own currency and nobody else's.
///
/// # Errors
///
/// [`method_address`]'s, plus [`Fault::Pending`] when the member throws.
pub fn call_method(
    ctx: &mut Ctx,
    receiver: Value,
    name: &str,
    args: &[Value],
    what: &str,
) -> Result<Option<Value>, Fault> {
    let Some(target) = method_address(receiver, name, what)? else {
        return Ok(None);
    };
    call_at(ctx, receiver, target, args).map(Some)
}

/// Calls a member on `receiver` at the address [`method_address`] answered,
/// with `args` past the receiver, returning whatever it produced.
///
/// # Errors
///
/// [`Fault::Pending`] when the member throws, carrying that call's own status
/// so the exception reaches the request unchanged.
pub(crate) fn call_at(
    ctx: &mut Ctx,
    receiver: Value,
    target: *const u8,
    args: &[Value],
) -> Result<Value, Fault> {
    #[expect(
        unsafe_code,
        reason = "the address came out of a live descriptor's method table, \
                  which `mwl-codegen` fills only with compiled functions of \
                  exactly this signature"
    )]
    let target: MwlFn = unsafe { std::mem::transmute::<*const u8, MwlFn>(target) };

    let mut slots = Vec::with_capacity(args.len() + 1);
    slots.push(receiver);
    slots.extend_from_slice(args);
    #[expect(
        unsafe_code,
        reason = "every value here is one the caller already owns a reference \
                  to, so each payload is live for the length of this call"
    )]
    unsafe {
        for slot in &slots {
            slot.retain();
        }
    }

    crate::abi::call(target, ctx, &slots).map_err(|status| {
        debug_assert_ne!(status, OK, "call reports Err only for a non-OK status");
        Fault::Pending(status)
    })
}
