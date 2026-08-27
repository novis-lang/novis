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
//!
//! # Why the parameter types are checked here, of all places
//!
//! [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md) § 1
//! gives `callable` no parameter list, so **no checker can compare a call site
//! against the body it will reach**, and the compiled `invoke` reads argument
//! slot *i* at its own declared representation. Hand it a mismatch and the
//! callee reinterprets the payload — an `int` read as an `MwlStr` pointer is
//! an arbitrary dereference, not a fault, and
//! `Core\Arr::map($ints, fn (string $s): string => $s)` over an `array<int>`
//! is all it takes to write one.
//!
//! So the closure object carries its parameter tags
//! ([`CLOSURE_PARAM_TAGS_SLOT`]) the way it already carries its arity
//! ([`CLOSURE_ARITY_SLOT`]), written at the literal by
//! `mwl_ir::lower::lower_closure_literal` from the declared types, and
//! [`check_param_tags`] compares one against each argument on the way in —
//! throwing the [`crate::ThrownClass::Logic`] `LogicError` [`mwl_call_closure`]
//! answers a bad arity with. It sits in [`call_closure`] because that is the
//! one path *both* callers take, a `Core` member's callback and ADR 0031's
//! `$fn(...)` alike; putting it in either caller would leave the other one
//! holding the hole. What it costs, and the one argument it converts rather
//! than compares — ADR 0007 § 2's `int`-into-`float` widening, which no checker
//! was there to insert — are that function's own doc comment.

use crate::abi::{Fault, MwlFn, OK};
use crate::ctx::Ctx;
use crate::object::{ClassDesc, MwlObj};
use crate::value::{Tag, Value};

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

/// The field slot holding which tag each of a closure's parameters requires —
/// always the second, and read by index for the same reason
/// [`CLOSURE_ARITY_SLOT`] is.
///
/// The payload is an `int` carrying one nibble per parameter, parameter 0 in
/// the least significant four bits, and each nibble is the [`Tag`]
/// discriminant an argument in that position must carry — so reading one costs
/// a shift and a mask and needs no table here. `mwl_ir::lower`'s
/// `FN_PARAM_TAGS` is the definition side and owns why the object carries this
/// at all; `mwl-codegen`'s `param_tag_nibbles_are_the_runtime_tag_bytes` holds
/// its map against the tag bytes compiled code actually writes.
pub const CLOSURE_PARAM_TAGS_SLOT: usize = 1;

/// The one [`CLOSURE_PARAM_TAGS_SLOT`] nibble that is not a [`Tag`]: the
/// parameter is `mixed`, `?T` or another union, whose representation *is* a
/// tag chosen at run time, so no argument can be wrong for it.
///
/// Twelve is the first number past the tag roster and can therefore never
/// collide with one — [`Tag::from_byte`] answering `None` for it is half of
/// `mwl-codegen`'s `the_any_nibble_denotes_no_tag_at_all`.
pub const CLOSURE_PARAM_TAG_ANY: u8 = 12;

/// How many parameters [`CLOSURE_PARAM_TAGS_SLOT`] can describe: one nibble
/// each in a 64-bit payload. A closure declaring more cannot be called —
/// [`check_param_tags`] says why refusing is the answer.
const CLOSURE_PARAM_TAGS_CAPACITY: usize = 16;

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
/// [`Fault::Thrown`] for an argument whose tag is not the one the closure
/// declares in that position, and which ADR 0007 § 2's `int`-into-`float`
/// widening does not reconcile — [`check_param_tags`], which runs before
/// anything is retained or passed.
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
    // Over this frame's own copy rather than the caller's slice, because the
    // check both refuses and *converts*: a widened `int` must reach the callee
    // as a `float` while the caller keeps owning the `int` it passed. Neither
    // tag is refcounted, so the retain below is unaffected by the substitution.
    check_param_tags(closure, &mut slots[1..])?;
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

/// `mwl_ir::Helper::CallClosure` — ADR 0031's `$fn(...)`, which is compiled
/// code's own way into [`call_closure`]. `args[0]` is the closure and
/// `args[1..argc]` the arguments it was called with, in written order.
///
/// **The one helper that takes a count.** Every other one's arity is a
/// literal in its [`crate::mwl_helper!`] expansion, because a conversion or a
/// comparison has the same shape at every call site; a closure call's arity is
/// the *call site's*, so it travels beside the slot and this function is
/// written out rather than generated. `mwl-codegen`'s `Signatures::helper_variadic`
/// is the other half of that ABI.
///
/// Too *many* arguments is not an error: [`call_closure`] trims to the
/// arity the closure recorded, which is spec § 2's "every callback receives
/// `($value, $key)` and may declare fewer parameters" and is also PHP's own
/// answer for extra positional arguments to a userland function. Too *few* is
/// the catchable `LogicError` below rather than the engine fault
/// [`call_closure`] answers a native caller with: a `callable` carries no
/// parameter list for the checker to count against
/// ([ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md)
/// § 1), so a program can reach it, and a program-reachable failure is a
/// throw ([ADR 0002](../../../docs/adr/0002-error-propagation.md)).
///
/// # Safety
///
/// `ctx`, `args` and `out` must each be non-null, aligned and valid for the
/// duration of the call; `args` must point at `argc` initialized values, of
/// which there must be at least one; and `out` must be writable. Compiled MWL
/// code satisfies all of it by construction.
#[expect(
    unsafe_code,
    reason = "the helper ABI's pointer contract, discharged exactly where \
              `mwl_helper!` discharges it for every fixed-arity helper"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_call_closure(
    ctx: *mut Ctx,
    args: *const Value,
    argc: usize,
    out: *mut Value,
) -> i32 {
    let body = |ctx: &mut Ctx, args: &[Value]| {
        let Some((closure, passed)) = args.split_first() else {
            return Err(Fault::fatal(
                "internal error: a closure call reached the runtime with no closure at all",
            ));
        };
        let arity = closure_arity(*closure)?;
        if passed.len() < arity {
            return Err(Fault::thrown_as(
                crate::ThrownClass::Logic,
                format!(
                    "too few arguments to a `callable`: it declares {arity} parameter(s), \
                     {} given",
                    passed.len()
                ),
            ));
        }
        call_closure(ctx, *closure, passed)
    };
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, args, argc, out, body)
    }
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

/// Refuses `args` unless every one of them carries the tag the closure's
/// corresponding parameter declares — the check that stands in for the one no
/// checker can make.
///
/// [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md) § 1
/// gives `callable` no parameter list, so a call site has nothing to compare
/// against and the compiled `invoke` reads argument slot *i* at its own
/// declared representation — an `int` handed to a `string` parameter is
/// dereferenced as an `MwlStr` pointer. This is the one place that can still
/// tell, because the closure object carries what the literal declared
/// (`mwl_ir::lower`'s `FN_PARAM_TAGS`), and it is on the path *both* callers
/// take: a `Core` member's callback and ADR 0031's `$fn(...)` alike.
///
/// One shift, one mask and one byte comparison per argument, on the callback
/// path — priority 3 spent on priority 1, which is the direction AGENTS.md's
/// ordering names, and the only design available while `callable` stays
/// unparameterized.
///
/// # The one conversion, rather than a refusal
///
/// The comparison is exact everywhere except the single position
/// [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) § 2 admits an
/// implicit conversion: an `int` or `uint` arriving at a `float` parameter is
/// *widened in place* rather than refused, through
/// [`crate::helpers::widen_to_float`] and therefore through the same row a
/// written `as float` takes. Above 2^53 that row refuses, and so does this —
/// as `ArithmeticError`, the class ADR 0007 § 4 names for a numeric overflow,
/// which a helper failure cannot reach (see `crate::helpers`'s `does_not_fit`)
/// but this function can.
///
/// Because it converts, `args` is the caller's *copy* rather than the caller's
/// slice; [`call_closure`] owns that distinction.
///
/// # Errors
///
/// [`Fault::Thrown`] carrying [`crate::ThrownClass::Arithmetic`] for an
/// integer argument to a `float` parameter that the widening above cannot
/// represent exactly, and [`crate::ThrownClass::Logic`] for a mismatched
/// argument, and for a closure declaring more parameters than
/// [`CLOSURE_PARAM_TAGS_CAPACITY`] can record — a program can reach both and a
/// program-reachable failure is a throw
/// ([ADR 0002](../../../docs/adr/0002-error-propagation.md)). Refusing the
/// call in the second case is deliberate: passing an argument whose declared
/// tag was never written down is exactly the read this function exists to
/// prevent, and no spec callback comes close to sixteen parameters.
///
/// [`Fault::Fatal`] when the closure is not a closure value, when its tag slot
/// does not hold an `int`, or when either side carries a byte that denotes no
/// representation at all — each of those is a compiler or runtime bug rather
/// than something a program can write.
fn check_param_tags(closure: Value, args: &mut [Value]) -> Result<(), Fault> {
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
    let slot = unsafe { crate::object::mwl_object_field_get(ptr, CLOSURE_PARAM_TAGS_SLOT) };
    let word = slot.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `callable`'s parameter-tag slot carried tag {} rather than an int",
            slot.tag_byte()
        ))
    })?;
    // The sixteenth nibble sits in the sign bit; the slot holds the same 64
    // bits either way, and only the nibbles are ever read.
    let word = u64::from_ne_bytes(word.to_ne_bytes());

    for (i, arg) in args.iter_mut().enumerate() {
        if i >= CLOSURE_PARAM_TAGS_CAPACITY {
            return Err(Fault::thrown_as(
                crate::ThrownClass::Logic,
                format!(
                    "a `callable` declaring more than {CLOSURE_PARAM_TAGS_CAPACITY} parameters \
                     cannot be called: nothing recorded what its parameter {} requires",
                    i + 1
                ),
            ));
        }
        // The mask leaves four bits, which is what a nibble is.
        let nibble = ((word >> (i * 4)) & 0xf) as u8;
        if nibble == CLOSURE_PARAM_TAG_ANY {
            continue;
        }
        let required = Tag::from_byte(nibble).ok_or_else(|| {
            Fault::fatal(format!(
                "internal error: a `callable` recorded nibble {nibble} for parameter {}, which \
                 denotes no representation",
                i + 1
            ))
        })?;
        let given = arg.tag().ok_or_else(|| {
            Fault::fatal(format!(
                "internal error: argument {} to a `callable` carried tag {}, which denotes no \
                 representation",
                i + 1,
                arg.tag_byte()
            ))
        })?;
        if given != required {
            // ADR 0007 § 2's one implicit conversion, and there is no second:
            // an `int` or `uint` arriving at a `float` parameter widens under
            // that ADR's 2^53 rule instead of being refused. It is applied
            // here because no checker saw this call site to insert it — a
            // `callable` has no parameter list (ADR 0031 § 1) — and out of
            // `crate::helpers`'s own row, so the boundary is the same one a
            // written `as float` lands on.
            // `tests/conformance/core/arr-a-callback-float-parameter-widens-an-int-and-stops-at-2-53.mwlt`
            // pins both sides of it from MWL.
            if required == Tag::Float && matches!(given, Tag::Int | Tag::Uint) {
                *arg = crate::helpers::widen_to_float(*arg).ok_or_else(|| {
                    Fault::thrown_as(
                        crate::ThrownClass::Arithmetic,
                        format!(
                            "cannot convert argument {} to a `callable` from `{}` to `float`",
                            i + 1,
                            given.describe()
                        ),
                    )
                })?;
                continue;
            }
            return Err(Fault::thrown_as(
                crate::ThrownClass::Logic,
                format!(
                    "argument {} to a `callable` must be of type {}, {} given",
                    i + 1,
                    required.describe(),
                    given.describe()
                ),
            ));
        }
    }
    Ok(())
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
