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

/// `$m->name(...)` on a **`mixed`** receiver — `nvs_ir::Helper::CallErasedMethod`'s
/// whole answer, and the one dispatch here that a *program* reaches.
///
/// [ADR 0036](../../../docs/adr/0036-anonymous-object-shapes.md) § 4 defers
/// which class is behind the handle and whether there is one at all, so every
/// question a checker would have answered is answered here instead, from the
/// receiver's own [`ClassDesc`]: its [`crate::MethodRow`] carries the callee's
/// arity and the tag each parameter requires, and
/// [`crate::closure`]'s `check_param_tags` judges the arguments against them
/// through the one implementation this path and `callable`'s share.
/// `docs/adr/README.md` § *Decisions taken at project start* owns the
/// convention and why it is a descriptor row rather than a per-method thunk.
///
/// Nothing is marshalled in either direction: ADR 0002 makes one calling
/// convention normative, so the tagged values this site holds are already the
/// slots a compiled callee reads, and its return arrives tagged for the
/// `mixed` the call's own type is.
///
/// Ownership is [`call_at`]'s: the receiver and every argument passed are
/// retained here and released by the callee's own exit sweep, so the caller
/// keeps owning exactly what it lowered and the [`Value`] handed back is a
/// fresh reference.
///
/// # Errors
///
/// Every refusal below is a **catchable** throw, a `mixed` receiver being a
/// mistake a program can write rather than one a compiler can make
/// ([ADR 0002](../../../docs/adr/0002-error-propagation.md)):
///
/// - a receiver whose tag is not an object at all, worded as ADR 0036 § 4 says
///   the erased property fetch words its own;
/// - a class whose table has no such member;
/// - a member that is not `public` — the one visibility question this site can
///   ask, since a `mixed` receiver is outside every class by construction;
/// - a `Core`-owned member ([`crate::MethodRow::native`]), whose native
///   convention *borrows* argument 0 where a compiled method owns its
///   parameters, and for which no signature reached this crate at all;
/// - too few arguments for the arity the row records, which is
///   [`crate::closure`]'s own wording for a `callable`;
/// - an argument whose tag is not the one the parameter requires, and which
///   ADR 0007 § 2's `int`-into-`float` widening does not reconcile.
///
/// A variadic or `inout` parameter list is **not** refused here and needs no
/// row of its own: both are packed and written back at the *call site*, which
/// is a shape `nvs_types` refuses where it is written (`E0714`) or a callee
/// this row simply describes by its declared count.
///
/// Plus [`Fault::Pending`] when the member itself throws, carrying that call's
/// own status so the exception reaches the request unchanged.
pub fn call_erased_method(
    ctx: &mut Ctx,
    receiver: Value,
    name: &str,
    args: &[Value],
) -> Result<Value, Fault> {
    let Some(ptr) = receiver.obj_ptr() else {
        return Err(Fault::thrown_as(
            crate::ThrownClass::Logic,
            format!(
                "cannot call `{name}()` on {}, which is not an object",
                receiver
                    .tag()
                    .map_or("a value of no representation", crate::Tag::describe)
            ),
        ));
    };
    #[expect(
        unsafe_code,
        reason = "the caller owns a reference to this object, so the \
                  allocation and its descriptor are both live for this read"
    )]
    let desc: *const ClassDesc = unsafe { NvsObj::class_of(ptr) };
    if desc.is_null() {
        return Err(Fault::fatal(
            "internal error: a call through a `mixed` receiver was handed an object with no \
             class descriptor"
                .to_owned(),
        ));
    }
    #[expect(
        unsafe_code,
        reason = "just checked the descriptor is non-null, and it is owned by \
                  the compiled unit's class table for that unit's whole life"
    )]
    let desc = unsafe { &*desc };
    let class = desc.name();
    let Some(row) = desc.method_row(name) else {
        // A `Core`-owned class has no *compiled* method table at all — only
        // the engine-protocol rows `nvs_stdlib::instance` puts there — so
        // "has no method" would be a plausible answer and a wrong one for a
        // member the spec plainly gives it. `Core` is a reserved namespace
        // (ADR 0011 § 2), which is what makes the name enough to tell, and
        // this only runs on the failing edge.
        if class.starts_with("Core\\") {
            return Err(Fault::thrown_as(
                crate::ThrownClass::Logic,
                format!(
                    "`{class}::{name}` cannot be called through a `mixed` receiver: a `Core` \
                     member borrows its receiver where a compiled method owns its parameters, \
                     and no signature for one reaches the runtime — convert the receiver to \
                     `{class}` first"
                ),
            ));
        }
        return Err(Fault::thrown_as(
            crate::ThrownClass::Logic,
            format!("`{class}` has no method `{name}()`"),
        ));
    };
    let callee = format!("`{class}::{name}`");
    if !row.public {
        return Err(Fault::thrown_as(
            crate::ThrownClass::Logic,
            format!("{callee} is not public, and a `mixed` receiver is outside every class"),
        ));
    }
    if row.native {
        return Err(Fault::thrown_as(
            crate::ThrownClass::Logic,
            format!(
                "{callee} cannot be called through a `mixed` receiver: it is a native member, \
                 which borrows its receiver where a compiled method owns its parameters, and \
                 neither its arity nor its parameter tags reached the runtime"
            ),
        ));
    }
    let arity = row.arity as usize;
    if args.len() < arity {
        return Err(Fault::thrown_as(
            crate::ThrownClass::Logic,
            format!(
                "too few arguments to {callee}: it declares {arity} parameter(s), {} given",
                args.len()
            ),
        ));
    }
    // Over this frame's own copy rather than the caller's slice, because the
    // check both refuses and *converts*: a widened `int` must reach the callee
    // as a `float` while the caller keeps owning the `int` it passed. Trimmed
    // to the declared arity first, exactly as a closure call trims — a site
    // that wrote more than the callee takes is PHP's own answer for a
    // userland call, and an argument nothing declares has no tag to check.
    let mut passed = args[..arity].to_vec();
    crate::closure::check_param_tags(&callee, row.param_tags, &mut passed)?;
    call_at(ctx, receiver, row.code, &passed)
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

/// [ADR 0079](../../../docs/adr/0079-testing-is-a-language-feature.md) § 20's
/// one test: a fresh instance of `class`, its `method` called on that instance
/// with no arguments and no result, and the instance released.
///
/// **One instance per call** is the whole of what a runner buys from this
/// entry point — § 2's isolation begins at "no test observes another's
/// receiver", and an instance shared across two methods would lose that before
/// the isolates that finish it exist. The constructor is § 7's `setUp` and is
/// run where the class declares one; a class that declares none carries no
/// `constructor` row at all (`nvs_ir` lowers such a `new` with no target), so
/// the allocation and its armed defaults are the whole of construction.
///
/// The status is answered rather than a [`Fault`], because there is no helper
/// frame around this call to translate one: the runner is the outermost caller
/// and `ctx` is where the detail is left, exactly as [`crate::abi::call`]
/// leaves it for `nvs run`.
///
/// # Errors
///
/// The status of whichever call failed, with its message on `ctx`:
///
/// - [`crate::THROWN`] when the constructor or the method threw, which is the
///   test failing — a failed assertion arrives here as ADR 0079 § 5's
///   `Core\Test\Failure` like any other throw;
/// - [`crate::THROWN`] for a constructor that declares parameters, which
///   §§ 8-9's `#[Fixture]` injection is what will supply and nothing does yet.
///   A throw rather than a [`crate::FATAL`]: it is a limit of this runner, so
///   the suite reports that test and carries on;
/// - [`crate::FATAL`] when the class declares no such method, which is an
///   internal inconsistency for a roster that came out of the same compile;
/// - [`crate::EXITED`] when the test called `exit`.
///
/// # Safety
///
/// `class` must refer to a live descriptor whose method table `nvs-codegen`
/// has already filled.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
pub unsafe fn construct_and_call(
    ctx: &mut Ctx,
    class: *const ClassDesc,
    method: &str,
) -> Result<(), i32> {
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    let desc = unsafe { &*class };
    let receiver = match desc.method_row(crate::object::CONSTRUCTOR) {
        Some(row) if row.arity > 0 => {
            return Err(crate::abi::record_fault(
                ctx,
                Fault::thrown(format!(
                    "`{}`'s constructor declares {} parameter(s), and this runner supplies \
                     none: ADR 0079 §§ 8-9's `#[Fixture]` injection is not built yet",
                    desc.name(),
                    row.arity
                )),
            ));
        }
        Some(_) => {
            #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
            match unsafe { crate::object::construct(ctx, class, &[]) } {
                Ok(value) => value,
                Err(fault) => return Err(crate::abi::record_fault(ctx, fault)),
            }
        }
        // No `constructor` row at all: ADR 0022 § 2 gives such a class nothing
        // to run, so the allocation with its armed defaults *is* the instance.
        None => {
            #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
            let object = unsafe { NvsObj::new(class) };
            Value::object(object)
        }
    };

    let outcome = call_method(ctx, receiver, method, &[], "the test runner");
    #[expect(
        unsafe_code,
        reason = "this frame holds the one reference construction handed it; \
                  `call_method` retained its own for the callee to release"
    )]
    unsafe {
        receiver.release();
    }
    match outcome {
        // A `#[Test]` method returns `void` (`code::E_TEST_METHOD_SHAPE`), so
        // the value is `null` — released anyway rather than trusting that
        // check from here, this being the runtime and not the checker.
        Ok(Some(value)) => {
            #[expect(
                unsafe_code,
                reason = "a returned value is a fresh reference this frame owns \
                          and nothing else will read"
            )]
            unsafe {
                value.release();
            }
            Ok(())
        }
        Ok(None) => Err(crate::abi::record_fault(
            ctx,
            Fault::fatal(format!(
                "internal error: `{}` declares no `{method}()` for the test runner to call",
                desc.name()
            )),
        )),
        Err(fault) => Err(crate::abi::record_fault(ctx, fault)),
    }
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

crate::nvs_helper! {
    /// `nvs_ir::Helper::CallErasedMethod` — compiled code's own way into
    /// [`call_erased_method`], which is ADR 0036 § 4's deferral for a call.
    ///
    /// `args[0]` is the receiver, still **tagged**: nothing proved it holds an
    /// object, so the test is made below rather than here. `args[1]` is the
    /// member name, an immortal `string` constant in the calling unit's data
    /// section rather than an allocation per call. `args[2]` is **one array**
    /// holding every argument in written order, which is why this is a
    /// fixed-arity helper at all: a site's own count is not the fact the call
    /// turns on — a `...` argument's count is its subject's own length, and
    /// what the arguments are judged against is a callee chosen when this
    /// runs. `nvs_ir::Helper::CallClosureArray` is the same shape for the same
    /// reason.
    ///
    /// The entries are **borrowed** from an array the caller owns for the
    /// length of this call, exactly as [`crate::nvs_call_closure_array`]'s
    /// are; [`call_at`] retains each one it actually passes.
    fn nvs_call_erased_method(ctx, args: [3]) {
        let name = args[1]
            .as_text()
            .ok_or_else(|| crate::helpers::wrong_tag("nvs_call_erased_method", crate::Tag::Str, args[1]))?;
        let array = args[2].array_ptr().ok_or_else(|| {
            crate::helpers::wrong_tag("nvs_call_erased_method", crate::Tag::Array, args[2])
        })?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for this read"
        )]
        // The caller's reference is the caller's: this handle reads the table
        // and must not run its own drop, exactly as `nvs_call_closure_array`'s
        // subject handle does.
        let entries = unsafe {
            let source = std::mem::ManuallyDrop::new(crate::array::NvsArray::from_raw(array));
            let mut entries = Vec::new();
            let mut from = 0;
            while let Some(slot) = source.next_slot(from) {
                from = slot + 1;
                entries.push(source.value_at(slot).expect("next_slot names a live entry"));
            }
            entries
        };
        call_erased_method(ctx, args[0], name, &entries)
    }
}
