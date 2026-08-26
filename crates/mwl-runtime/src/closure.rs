//! Calling an [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md)
//! closure value from native code.
//!
//! A closure is an ordinary MWL object whose class declares exactly one
//! method, [`CLOSURE_INVOKE`], and one field per captured binding —
//! `mwl_ir::lower::lower_closure` owns that representation and says why it
//! reuses the object machinery rather than adding a second heap shape. So
//! everything here is already available: [`crate::mwl_class_method`] finds
//! the compiled address, and [`crate::call`] reaches it under exactly the
//! [ADR 0002](../../../docs/adr/0002-error-propagation.md) signature every
//! other compiled function has.
//!
//! # Why this exists at all
//!
//! `Core\Arr::filter` and its siblings are native Rust helpers, and a helper
//! *borrows* its arguments while a compiled method *owns* its parameters —
//! the two halves of the convention `mwl_stdlib`'s own module docs state.
//! [`call_closure`] is the one place that mismatch is reconciled: it retains
//! the receiver and every argument on the way in, so the callee's exit sweep
//! releases references this function paid for rather than the caller's.
//! Getting that wrong in each `Core` member separately is exactly the kind of
//! hand-written refcount protocol Stage 6's valgrind leg exists to catch, so
//! there is one implementation and no second one.

use crate::abi::{Fault, MwlFn, OK};
use crate::ctx::Ctx;
use crate::object::{ClassDesc, MwlObj};
use crate::value::Value;

/// The one method a closure's captured-environment class answers. Must agree
/// with `mwl_ir::lower`'s own constant; `mwl-codegen`'s
/// `a_closure_is_reachable_through_the_method_table` holds the two together.
pub const CLOSURE_INVOKE: &str = "invoke";

/// The field slot holding how many parameters a closure declares, not
/// counting the receiver — always the first, since a descriptor carries no
/// field names for a native caller to search.
///
/// `mwl_ir::lower`'s `FN_ARITY` is the definition side and owns the reason
/// the arity is stored per object at all; `mwl-codegen`'s
/// `a_closure_object_carries_its_own_arity_in_slot_zero` holds the two
/// together.
pub const CLOSURE_ARITY_SLOT: usize = 0;

/// Calls the closure `closure` with as many leading `args` as it declares
/// parameters, borrowing every one of them.
///
/// The trailing arguments a shorter closure does not want are dropped rather
/// than passed, which is
/// [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
/// § 2's "every callback receives `($value, $key)` and may declare fewer
/// parameters" — the rule that removes PHP's `ARRAY_FILTER_USE_KEY`/
/// `ARRAY_FILTER_USE_BOTH` flags. A caller therefore passes every argument it
/// already holds and lets the trimming happen here; the one reason to ask
/// [`closure_arity`] first is an argument that would have to be *built* — see
/// that function.
///
/// The receiver and each argument actually passed are retained before the
/// call and released by the callee, so the caller keeps owning exactly what
/// it owned before — the borrowing convention every `Core` helper is written
/// against.
///
/// # Errors
///
/// [`Fault::Pending`] carrying the callee's own status when the closure
/// throws or faults, so the exception the callee recorded in `ctx` reaches
/// the request unchanged rather than being replaced by a message from here.
/// [`Fault::Fatal`] when `closure` is not a closure value at all, or declares
/// more parameters than the caller has to offer — both engine faults: the
/// checker only admits an ADR 0027 closure value where a `callable` is
/// expected, and no `Core` member offers fewer than the spec says it does.
pub fn call_closure(ctx: &mut Ctx, closure: Value, args: &[Value]) -> Result<Value, Fault> {
    let target = invoke_address(closure)?;
    let arity = closure_arity(closure)?;
    let args = args.get(..arity).ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `callable` declaring {arity} parameters was called with only \
             {} available",
            args.len()
        ))
    })?;
    #[expect(
        unsafe_code,
        reason = "the address came out of a live descriptor's method table, \
                  which `mwl-codegen` fills only with compiled functions of \
                  exactly this signature"
    )]
    let target: MwlFn = unsafe { std::mem::transmute::<*const u8, MwlFn>(target) };

    let mut slots = Vec::with_capacity(args.len() + 1);
    slots.push(closure);
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

/// How many parameters `closure` declares — [`CLOSURE_ARITY_SLOT`], read
/// straight off the object with no call made.
///
/// [`call_closure`] uses it to trim the argument list, and a caller asks it
/// directly to avoid *building* an argument that trimming would throw away:
/// `Core\Arr::map`'s `$key` costs a rendered decimal and an `MwlStr` per
/// entry on a list, which is `docs/perf/userland-gap.md` § D. That is the
/// only reason to inspect an arity — a caller that already holds every
/// argument still passes them all and lets the trimming happen here.
///
/// # Errors
///
/// [`Fault::Fatal`] when `closure` is not a closure value at all, the same
/// engine fault [`call_closure`] answers with.
pub fn closure_arity(closure: Value) -> Result<usize, Fault> {
    let ptr = closure.obj_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `callable` argument carried tag {} rather than an object",
            closure.tag_byte()
        ))
    })?;
    #[expect(
        unsafe_code,
        reason = "the caller owns a reference to this object, and the slot \
                  index is one every closure class has by construction"
    )]
    let slot = unsafe { crate::object::mwl_object_field_get(ptr, CLOSURE_ARITY_SLOT) };
    let arity = slot.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `callable`'s arity slot carried tag {} rather than an int",
            slot.tag_byte()
        ))
    })?;
    usize::try_from(arity)
        .map_err(|_| Fault::fatal("internal error: a `callable` recorded a negative arity"))
}

/// The compiled address of `closure`'s [`CLOSURE_INVOKE`], or a [`Fault`]
/// naming what was passed instead.
fn invoke_address(closure: Value) -> Result<*const u8, Fault> {
    let ptr = closure.obj_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `callable` argument carried tag {} rather than an object",
            closure.tag_byte()
        ))
    })?;
    #[expect(
        unsafe_code,
        reason = "the caller owns a reference to this object, so the \
                  allocation and its descriptor are both live for this read"
    )]
    let desc: *const ClassDesc = unsafe { MwlObj::class_of(ptr) };
    if desc.is_null() {
        return Err(Fault::fatal(
            "internal error: a `callable` argument's object has no class descriptor".to_owned(),
        ));
    }
    #[expect(
        unsafe_code,
        reason = "just checked the descriptor is non-null, and it is owned by \
                  the compiled unit's class table for that unit's whole life"
    )]
    let found = unsafe { &*desc }.method(CLOSURE_INVOKE);
    found.ok_or_else(|| {
        #[expect(
            unsafe_code,
            reason = "same descriptor, still live — read only to name what was \
                      passed"
        )]
        let name = unsafe { &*desc }.name().to_owned();
        Fault::fatal(format!(
            "internal error: `{name}` was passed where a `callable` was expected, and declares \
             no `{CLOSURE_INVOKE}`"
        ))
    })
}
