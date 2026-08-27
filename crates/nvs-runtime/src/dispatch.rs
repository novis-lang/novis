//! Reaching a compiled instance member from native code, by name.
//!
//! A helper that has to ask an *object* something — `Comparable::compareTo`
//! for [ADR 0013](../../../docs/adr/0013-comparable-interface.md), the
//! `iterate`/`advance`/`current` trio for
//! [ADR 0053](../../../docs/adr/0053-iteration-and-generators.md) — cannot
//! name a compiled function: the class is one this crate and `nvs-stdlib` know
//! nothing about, and the interface member is a bodiless declaration
//! (`nvs_types::iter_lib`), so no symbol exists for a call site to resolve.
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
//! because a compiled Novis function releases its parameters — the same
//! reconciliation [`crate::call_closure`] performs, and for the same reason.
//! The [`Value`] it hands back is a fresh reference this frame owns.

use crate::abi::{Fault, NvsFn, OK};
use crate::ctx::Ctx;
use crate::object::{ClassDesc, NvsObj};
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
    let desc = descriptor_of(receiver, what, &format!("`{name}`"))?;
    #[expect(
        unsafe_code,
        reason = "`descriptor_of` answers only a non-null descriptor, and one \
                  is owned by the compiled unit's class table for that unit's \
                  whole life"
    )]
    Ok(unsafe { &*desc }.method(name))
}

/// `receiver`'s class descriptor, never null.
///
/// `what` names the caller and `looked_for` what it wanted, both for a fault
/// message neither of the two callers can reach from behaving code.
///
/// # Errors
///
/// [`Fault::Fatal`] when `receiver` is not an object, or is one with no class
/// descriptor — both engine faults rather than anything a program can cause.
fn descriptor_of(receiver: Value, what: &str, looked_for: &str) -> Result<*const ClassDesc, Fault> {
    let ptr = receiver.obj_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: {what} looked for {looked_for} on tag {}",
            receiver.tag_byte()
        ))
    })?;
    #[expect(
        unsafe_code,
        reason = "the caller owns a reference to this object, so the \
                  allocation and its descriptor are both live for this read"
    )]
    let desc: *const ClassDesc = unsafe { NvsObj::class_of(ptr) };
    if desc.is_null() {
        return Err(Fault::fatal(format!(
            "internal error: {what} was handed an object with no class descriptor"
        )));
    }
    Ok(desc)
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

/// Renders `receiver` through the **native** renderer its class carries, or
/// answers `None` when its class carries none — which is every class a program
/// declares.
///
/// Ownership is the opposite of [`call_method`]'s, and that is the whole
/// reason the address sits in its own descriptor field rather than in the
/// method table ([`ClassDesc::renderer`]): a native `Core` member is an ADR
/// 0002 helper, so it *borrows* argument 0, and this neither retains on the
/// way in nor releases on the way out. The `string` it answers with carries
/// the one reference every helper's result does.
///
/// # Errors
///
/// [`method_address`]'s two engine faults, plus [`Fault::Pending`] when the
/// renderer itself faults.
pub fn call_render(ctx: &mut Ctx, receiver: Value, what: &str) -> Result<Option<Value>, Fault> {
    let desc = descriptor_of(receiver, what, "a renderer")?;
    #[expect(
        unsafe_code,
        reason = "`descriptor_of` answers only a non-null descriptor, and one \
                  is owned by its class table for that table's whole life"
    )]
    let Some(target) = (unsafe { &*desc }).renderer() else {
        return Ok(None);
    };
    #[expect(
        unsafe_code,
        reason = "the address came out of `ClassTable::set_render`, which \
                  `nvs-stdlib` calls only with a registered `Core` member's \
                  own address, and every one of those has this signature"
    )]
    let target: NvsFn = unsafe { std::mem::transmute::<*const u8, NvsFn>(target) };
    crate::abi::call(target, ctx, &[receiver])
        .map(Some)
        .map_err(|status| {
            debug_assert_ne!(status, OK, "call reports Err only for a non-OK status");
            Fault::Pending(status)
        })
}

/// Resumes the dying generator `receiver` into its unwind entry point, at the
/// address its class carries ([`crate::ClassDesc::unwind_entry`]).
///
/// Ownership is [`call_render`]'s rather than [`call_at`]'s, and for the same
/// reason stated the other way round: `gen#unwind` is the one *compiled*
/// method that **borrows** argument 0, because its caller is a release that
/// has no reference left to hand over. `nvs_ir::lower::generator`'s
/// `lower_generator_unwind` argues that inversion in full; here it means this
/// neither retains on the way in nor releases on the way out.
///
/// # Errors
///
/// [`Fault::Pending`] when a `finally` the unwind ran threw — which
/// [`crate::object::dismantle`] discards, [`crate::Ctx::with_pending_set_aside`]
/// owning why.
pub(crate) fn call_unwind(
    ctx: &mut Ctx,
    receiver: Value,
    target: *const u8,
) -> Result<Value, Fault> {
    #[expect(
        unsafe_code,
        reason = "the address came out of a live descriptor's own method table, \
                  which `nvs-codegen` fills only with compiled functions of \
                  exactly this signature"
    )]
    let target: NvsFn = unsafe { std::mem::transmute::<*const u8, NvsFn>(target) };
    crate::abi::call(target, ctx, &[receiver]).map_err(|status| {
        debug_assert_ne!(status, OK, "call reports Err only for a non-OK status");
        Fault::Pending(status)
    })
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
                  which `nvs-codegen` fills only with compiled functions of \
                  exactly this signature"
    )]
    let target: NvsFn = unsafe { std::mem::transmute::<*const u8, NvsFn>(target) };

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
