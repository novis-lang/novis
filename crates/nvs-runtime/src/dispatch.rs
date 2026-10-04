//! Reaching a compiled member from native code, by name — an instance member
//! through its receiver's descriptor, and a `static` one through its class's.
//!
//! A helper that has to ask an *object* something — `Comparable::compareTo`
//! for `rule:classes/comparable`, the
//! `iterate`/`advance`/`current` trio for
//! `rule:iteration/two-interfaces` — cannot
//! name a compiled function: the class is one this crate and `nvs-stdlib` know
//! nothing about, and the interface member is a bodiless declaration
//! (`nvs_types::iter_lib`), so no symbol exists for a call site to resolve.
//! The object's own descriptor carries the table that answers it, and
//! [`crate::call_callable`] already reaches a callable's `invoke` through
//! exactly this lookup with the name fixed.
//!
//! So this is that lookup with the name as an argument, and it is the same
//! dispatch a `foreach` or a `$value->compareTo($other)` in source performs —
//! one indirect call through the class descriptor, no allocation, and nothing
//! cached, because a descriptor's method table is a compile-time constant of
//! the unit that declared it.
//!
//! # What the call retains, and what it hands back
//!
//! [`call_method`] retains the receiver and every argument before it calls,
//! because a compiled Novis function releases its parameters — the same
//! reconciliation [`crate::call_callable`] performs, and for the same reason.
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
/// message no caller can reach from behaving code.
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

/// The `static` method `label` — a `Class::method` string — names, called with
/// `args`, or `None` where this program declares no such class or no such
/// method on it.
///
/// `rule:tooling/commands-are-compiled`'s
/// dispatch is the caller: a `#[Command]` handler is named by a string the
/// compiler put in the table and reached from a native member, which is the one
/// shape the receiver-keyed [`call_method`] above cannot serve — there is no
/// instance to key on. The route is the one the *program's* own
/// `Class::method()` call site takes: `nvs_types::layout::ClassLayout::methods`
/// lists every method with a body, `static` ones included, so the class
/// descriptor's method table already holds the address and nothing new has to
/// be installed beside the table for a handler to be reachable.
///
/// Slot 0 is the **called class**, per [`Value::class_desc`] — late static
/// binding's whole mechanism, and the reason this fills it from the descriptor
/// it just looked the address up in rather than leaving it `null`.
///
/// # Errors
///
/// [`Fault::Pending`] when the method throws, carrying that call's own status
/// so the exception reaches the request unchanged.
pub fn call_static(ctx: &mut Ctx, label: &str, args: &[Value]) -> Result<Option<Value>, Fault> {
    let Some((class, method)) = label.rsplit_once("::") else {
        return Ok(None);
    };
    let Some(desc) = ctx.class_desc(class) else {
        return Ok(None);
    };
    #[expect(
        unsafe_code,
        reason = "`Ctx::class_desc` answers out of the compiled unit's own class \
                  table, which the context shares ownership of for its whole life"
    )]
    unsafe {
        call_static_on(ctx, desc, method, args)
    }
}

/// [`call_static`] for a caller that is **already holding the descriptor**,
/// answering `None` where that class declares no method `name`.
///
/// The label form above exists because its caller has a string and nothing
/// else. A native member that reached a class through a call site's own type
/// argument — `nvs_stdlib::db::row`'s hydration, which takes the descriptor
/// `queryAs<T>` wrote — has the table in hand, and rendering its name to look
/// it up again would make the call depend on a second resolution of a question
/// already answered.
///
/// Slot 0 is the **called class**, per [`Value::class_desc`], exactly as above.
///
/// # Safety
///
/// `desc` must refer to a live descriptor whose method table `nvs-codegen` has
/// filled.
///
/// # Errors
///
/// [`Fault::Pending`] when the method throws, carrying that call's own status
/// so the exception reaches the request unchanged.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
pub unsafe fn call_static_on(
    ctx: &mut Ctx,
    desc: *const ClassDesc,
    name: &str,
    args: &[Value],
) -> Result<Option<Value>, Fault> {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the descriptor is live for this read"
    )]
    let Some(target) = (unsafe { &*desc }).method(name) else {
        return Ok(None);
    };
    call_at(ctx, Value::class_desc(desc), target, args).map(Some)
}

/// [`call_static`], for arguments that came out of a **program's own map** —
/// [ADR 0006](/docs/decisions/0006.md) § *Decision*'s
/// method entry, called with `args:`'s entries already bound to its parameters
/// positionally by `nvs_stdlib::script`.
///
/// The target is the same and the route is the same; what differs is who wrote
/// the argument list. [`call_static`]'s own caller builds one from a table the
/// compiler filled, so arity and each parameter's representation are settled
/// before the call exists. Here the list is as long as the map the program
/// passed and holds whatever that map held, and a compiled callee reads its
/// slots without asking — so both questions are asked *here*, through
/// [`crate::callable`]'s `check_param_tags`, which is the one implementation
/// `rule:types/conversion`'s `int`-into-`float` widening lives in and which `callable`
/// and an erased method call already share. Another copy of that comparison is
/// exactly what the shared helper exists to prevent.
///
/// A **native** row is refused rather than called: a `Core`-owned member
/// borrows argument 0 where a compiled method owns its parameters
/// ([`crate::MethodRow::native`]), and no signature for one ever reached this
/// crate. `nvs_types` resolves the entry against the program's own classes, so
/// this is a shape only a mismatched class table can produce.
///
/// # Errors
///
/// [`Fault::Thrown`] carrying [`crate::ThrownClass::Logic`] for an argument
/// count the entry does not declare, for a native row, and for an argument
/// whose tag the parameter does not admit — every one of them the ordinary
/// named-argument error `rule:security/isolate-shares-nothing` says a bad `args:` map is, raised at the
/// spawn. [`Fault::Pending`] when the entry itself throws.
pub fn call_static_bound(
    ctx: &mut Ctx,
    label: &str,
    args: &mut [Value],
) -> Result<Option<Value>, Fault> {
    let Some((class, method)) = label.rsplit_once("::") else {
        return Ok(None);
    };
    let Some(desc) = ctx.class_desc(class) else {
        return Ok(None);
    };
    #[expect(
        unsafe_code,
        reason = "`Ctx::class_desc` answers out of the compiled unit's own class \
                  table, which the context shares ownership of for its whole life"
    )]
    let Some(row) = (unsafe { &*desc }).method_row(method) else {
        return Ok(None);
    };
    // Copied out rather than held: the row lives in the class table and the
    // call below takes the context mutably.
    let (code, arity, param_tags, native) =
        (row.code, row.arity as usize, row.param_tags, row.native);
    if native {
        return Err(Fault::thrown_as(
            crate::ThrownClass::Logic,
            format!("`{label}` is a `Core`-owned member and cannot be an isolate's entry"),
        ));
    }
    if args.len() != arity {
        return Err(Fault::thrown_as(
            crate::ThrownClass::Logic,
            format!(
                "`{label}` declares {arity} parameter(s) and was called with {}",
                args.len()
            ),
        ));
    }
    crate::callable::check_param_tags(label, param_tags, args)?;
    call_at(ctx, Value::class_desc(desc), code, args).map(Some)
}

/// `$m->name(...)` on a **`mixed`** receiver — `nvs_ir::Helper::CallErasedMethod`'s
/// whole answer, and the one dispatch here that a *program* reaches.
///
/// `rule:types/erased-member-access` defers
/// which class is behind the handle and whether there is one at all, so every
/// question a checker would have answered is answered here instead, from the
/// receiver's own [`ClassDesc`]: its [`crate::MethodRow`] carries the callee's
/// arity and the tag each parameter requires, and
/// [`crate::callable`]'s `check_param_tags` judges the arguments against them
/// through the one implementation this path and `callable`'s share.
/// `docs/adr/README.md` § *Decisions taken at project start* owns the
/// convention and why it is a descriptor row rather than a per-method thunk.
///
/// Nothing is marshalled in either direction: `rule:errors/propagation` makes one calling
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
/// (`rule:errors/propagation`):
///
/// - a receiver whose tag is not an object at all, worded as `rule:types/erased-member-access` says
///   the erased property fetch words its own;
/// - a class whose table has no such member;
/// - a member that is not `public` — the one visibility question this site can
///   ask, since a `mixed` receiver is outside every class by construction;
/// - a `Core`-owned member ([`crate::MethodRow::native`]), whose native
///   convention *borrows* argument 0 where a compiled method owns its
///   parameters, and for which no signature reached this crate at all;
/// - too few arguments for the arity the row records, which is
///   [`crate::callable`]'s own wording for a `callable`;
/// - an argument whose tag is not the one the parameter requires, and which
///   `rule:types/conversion`'s `int`-into-`float` widening does not reconcile.
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
    call_erased_method_from(ctx, receiver, name, args, None)
}

/// [`call_erased_method`], with the class the call is written inside named —
/// `rule:security/reflection-enforces-visibility`'s check asked from somewhere
/// rather than from nowhere.
///
/// `site` is `None` for every erased call a program writes, which is what the
/// entry point above passes: a `mixed` receiver is outside every class by
/// construction, and that is the whole of what the visibility question can be
/// asked against there. It is `Some` for one caller — `Core\Reflect\ClassInfo`'s
/// acting members, which the compiler hands their own call site through
/// `nvs_stdlib::registry::CALL_SITE_MEMBERS`.
///
/// Which sites a non-`public` member is then reachable from is
/// [`crate::Ctx::method_is_visible_from`]'s question rather than this
/// function's, and it is the hierarchy walk `nvs_types::signatures`'s
/// `is_visible_from` makes where a receiver's class is written down: a
/// `private` method answers to the class it belongs to and a `protected` one
/// to every class in that hierarchy declaring it. A site naming a class
/// outside it is refused the way the anonymous one is.
///
/// # Errors
///
/// [`call_erased_method`]'s, whose doc comment lists them.
pub fn call_erased_method_from(
    ctx: &mut Ctx,
    receiver: Value,
    name: &str,
    args: &[Value],
    site: Option<&str>,
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
        // (`rule:core-api/reserved-namespace`), which is what makes the name enough to tell, and
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
    if !ctx.method_is_visible_from(desc, row, site) {
        let outside = site.map_or_else(
            || "a `mixed` receiver is outside every class".to_owned(),
            |site| format!("the call is written inside `{site}`"),
        );
        return Err(Fault::thrown_as(
            crate::ThrownClass::Logic,
            format!("{callee} is not public, and {outside}"),
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
    // to the declared arity first, exactly as a callable call trims — a site
    // that wrote more than the callee takes is PHP's own answer for a
    // userland call, and an argument nothing declares has no tag to check.
    let mut passed = args[..arity].to_vec();
    crate::callable::check_param_tags(&callee, row.param_tags, &mut passed)?;
    call_at(ctx, receiver, row.code, &passed)
}

/// A fresh instance of `class`, built by running its own
/// [`crate::object::CONSTRUCTOR`] with `args` under the visibility check code
/// written inside `site` would face — `Core\Reflect\ClassInfo`'s constructing
/// member, and nothing else reaches this.
///
/// **The check is [`call_erased_method_from`]'s**, not a second one written
/// here, because a constructor is a method of the class and
/// `rule:security/reflection-enforces-visibility` asks for the *same* check
/// rather than one that agrees: a `private` constructor is reached from its own
/// class's bodies, which is what leaves a singleton's own `load()` working, and
/// refused from every other site the way the `new` written there is refused.
/// The argument list is judged against the constructor's declared parameters by
/// that same path, so `$arguments` meets one arity rule and one tag rule across
/// both doors. A refusal therefore arrives in that path's own words, including
/// its reading of a `None` site: the erased door has one sentence for a call
/// that is outside every class, and a second one written for this caller would
/// be the second visibility rule this reuse exists to avoid.
///
/// The allocation is made ahead of the check rather than behind it, which costs
/// one object on the refusing edge and buys the single check: every failing
/// path below releases the one reference this frame holds, so nothing is
/// abandoned. A class declaring no constructor carries no row for one —
/// `nvs_ir` lowers such a `new` with no target — so the allocation with its
/// armed defaults *is* the instance, and there is no member whose visibility
/// could be asked about.
///
/// # Errors
///
/// [`call_erased_method`]'s, whose doc comment lists them, raised against the
/// constructor as the callee.
///
/// # Safety
///
/// `class` must refer to a live descriptor whose method table `nvs-codegen`
/// has already filled.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
pub unsafe fn construct_erased_from(
    ctx: &mut Ctx,
    class: *const ClassDesc,
    args: &[Value],
    site: Option<&str>,
) -> Result<Value, Fault> {
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    let desc = unsafe { &*class };
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    let receiver = Value::object(unsafe { NvsObj::new(class) });
    if desc.method_row(crate::object::CONSTRUCTOR).is_none() {
        return Ok(receiver);
    }
    match call_erased_method_from(ctx, receiver, crate::object::CONSTRUCTOR, args, site) {
        Ok(returned) => {
            #[expect(
                unsafe_code,
                reason = "a constructor returns `void`, so this is the `null` the \
                          call handed back as a fresh reference nothing will read"
            )]
            unsafe {
                returned.release();
            }
            Ok(receiver)
        }
        Err(fault) => {
            #[expect(
                unsafe_code,
                reason = "this frame holds the one reference the allocation was \
                          made with, and the instance is now unreachable"
            )]
            unsafe {
                receiver.release();
            }
            Err(fault)
        }
    }
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
/// [`method_address`]'s engine faults, plus [`Fault::Pending`] when the
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

/// Orders `left` against `right` through `rule:classes/comparable`'s
/// `compareTo`, whichever of the two conventions `left`'s class answers it
/// under — or `None` when it answers it under neither.
///
/// The one entry point for ordering two objects, because the two conventions
/// are the thing a caller must not have to choose between. A class a program
/// declares carries the member on its method table and is called through
/// [`call_method`], which transfers a reference per slot. A `Core` class
/// carries it on [`crate::ClassDesc::comparer`] instead, because a native
/// member is an `rule:errors/propagation` helper that **borrows** its
/// arguments — so this neither retains on the way in nor releases on the way
/// out, exactly as [`call_render`] does not. The `int` it answers with carries
/// the one reference either call's result does.
///
/// The descriptor is asked first: a `Core` class has no method row to find, and
/// no class carries both.
///
/// # Errors
///
/// [`method_address`]'s engine faults, plus [`Fault::Pending`] when the
/// comparison itself throws.
pub fn call_compare_to(
    ctx: &mut Ctx,
    left: Value,
    right: Value,
    what: &str,
) -> Result<Option<Value>, Fault> {
    let desc = descriptor_of(left, what, crate::object::COMPARE_TO)?;
    #[expect(
        unsafe_code,
        reason = "`descriptor_of` answers only a non-null descriptor, and one \
                  is owned by its class table for that table's whole life"
    )]
    let Some(target) = (unsafe { &*desc }).comparer() else {
        return call_method(ctx, left, crate::object::COMPARE_TO, &[right], what);
    };
    #[expect(
        unsafe_code,
        reason = "the address came out of `ClassTable::set_compare`, which \
                  `nvs-stdlib` calls only with a registered `Core` member's \
                  own address, and every one of those has this signature"
    )]
    let target: NvsFn = unsafe { std::mem::transmute::<*const u8, NvsFn>(target) };
    crate::abi::call(target, ctx, &[left, right])
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
/// [`crate::object::dismantle`] reports through the escalation ladder rather
/// than propagating, [`crate::Ctx::with_pending_set_aside`] owning why.
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

/// `rule:testing/fixtures`'s
/// fixtures, built once and owned until the class they belong to is done with.
///
/// It exists to put the **ownership** of a built fixture in one place. A
/// fixture is a value the runner holds across many calls — every test of the
/// class borrows it, and a compiled callee releases its parameters — so
/// somebody has to hold exactly one reference per built value and drop it at
/// the end. That somebody is this type rather than the runner, because
/// `nvs-cli` forbids `unsafe` outright and a released [`Value`] has no safe
/// spelling.
///
/// The order values are built in is the *checker's*: `nvs_types::testing`
/// resolved each parameter to the fixture supplying it and refused a cycle
/// (`nvs_diagnostics::code::E_FIXTURE_CYCLE`), so a caller walking
/// dependencies before dependants always finds what [`Self::build`] asks for
/// already here.
///
/// **Built once here and copied into each test**, which is § 8's own bargain:
/// the expensive half runs once per class, and what a test is handed is
/// [`CrossedFixtures`] — its own graph copy, sharing no mutable state with
/// this set or with the test before it. A test that mutates what it was given
/// therefore mutates a copy, and § 2's isolation holds over a fixture as it
/// does over a static.
#[derive(Debug, Default)]
pub struct Fixtures {
    built: Vec<(String, Value)>,
}

impl Fixtures {
    /// An empty set — one class's worth, built as its tests are reached.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Calls the `static` fixture method compiled at `target`, with the
    /// already-built values `needs` names, and keeps the result under `name`.
    ///
    /// `receiver` is the **called class**, which is what a `static` method's
    /// slot 0 carries (`rule:statements/static-is-a-member-modifier`'s late static binding, [`Value::class_desc`]) —
    /// a caller that left it `null` would hand the body a descriptor of zero
    /// for any `static::` inside it.
    ///
    /// Each argument is **retained** on the way in, because a compiled Novis
    /// function releases its parameters ([`call_at`] reconciles the same way):
    /// this set keeps its own reference and hands the callee one of its own.
    /// The receiver slot is not one: a descriptor is not an Novis value and
    /// carries no reference at all.
    ///
    /// # Errors
    ///
    /// The status of the call — [`crate::THROWN`] for a fixture whose body
    /// threw, with its message on `ctx`, or [`crate::EXITED`] for one that
    /// called `exit`. Nothing is recorded for a call that failed, so a caller
    /// that reports and carries on is asking for a value that is not here
    /// rather than one that is half-built.
    pub fn build(
        &mut self,
        ctx: &mut Ctx,
        name: &str,
        target: NvsFn,
        receiver: Value,
        needs: &[String],
    ) -> Result<(), i32> {
        let Some(args) = self.values(needs) else {
            return Err(crate::abi::record_fault(
                ctx,
                Fault::fatal(format!(
                    "internal error: `{name}` was built before a fixture it declares"
                )),
            ));
        };
        #[expect(
            unsafe_code,
            reason = "every value here is one this set already owns a reference \
                      to, so each payload is live for the length of this call"
        )]
        unsafe {
            for arg in &args {
                arg.retain();
            }
        }
        let mut slots = Vec::with_capacity(args.len() + 1);
        slots.push(receiver);
        slots.extend_from_slice(&args);
        let value = crate::abi::call(target, ctx, &slots)?;
        self.built.push((name.to_owned(), value));
        Ok(())
    }

    /// The values `needs` names, in that order — the argument list a test
    /// method or a fixture is called with — or `None` when one has not been
    /// built, which is an internal inconsistency for a roster that came out of
    /// the same compile.
    ///
    /// The values are **borrowed**: this set keeps its references, and the
    /// call the caller makes retains its own.
    #[must_use]
    pub fn values(&self, needs: &[String]) -> Option<Vec<Value>> {
        needs
            .iter()
            .map(|name| {
                self.built
                    .iter()
                    .find(|(built, _)| built == name)
                    .map(|(_, value)| *value)
            })
            .collect()
    }
}

impl Drop for Fixtures {
    fn drop(&mut self) {
        #[expect(
            unsafe_code,
            reason = "this set holds exactly one reference per built value, \
                      handed over by the call that produced it, and nothing \
                      else reads them after this"
        )]
        unsafe {
            for (_, value) in self.built.drain(..) {
                value.release();
            }
        }
    }
}

/// `rule:testing/data-rows`'s
/// data row, materialized into the values one call takes.
///
/// It sits beside [`Fixtures`] for that type's own reason: a row's `string` is
/// a fresh allocation somebody has to release, `nvs-cli` forbids `unsafe`
/// outright, and a released [`Value`] has no safe spelling. So the runner asks
/// for a constant by the shape the checker folded it to and is handed back a
/// [`Value`] it may pass but never owns.
///
/// Where `Fixtures` is a **per-class** owner, this is a **per-call** one, and
/// that is § 9's own rule rather than a convenience: each row is its own case,
/// so its values are built where the call is made and released when it
/// returns, and no two cases ever share one. A row holds only the constants
/// `nvs_types::defaults::literal_default` folds — the scalars and a
/// `string` — which is why there is one method per shape and no general
/// [`Value`] way in: the one reference each carries is this type's, and a
/// caller that could hand one over could hand over a second.
///
/// The values are **borrowed** by the call, exactly as a built fixture is:
/// [`call_at`] retains what it passes, so the callee releases its own
/// reference and this set still holds the one it made.
#[derive(Debug, Default)]
pub struct RowValues {
    built: Vec<Value>,
}

impl RowValues {
    /// An empty row — one call's worth, filled as the argument list is built.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `nvs_types::ConstArg::Bool`.
    pub fn bool(&mut self, value: bool) -> Value {
        self.keep(Value::bool(value))
    }

    /// `nvs_types::ConstArg::Int`.
    pub fn int(&mut self, value: i64) -> Value {
        self.keep(Value::int(value))
    }

    /// `nvs_types::ConstArg::Uint`.
    pub fn uint(&mut self, value: u64) -> Value {
        self.keep(Value::uint(value))
    }

    /// `nvs_types::ConstArg::Float`.
    pub fn float(&mut self, value: f64) -> Value {
        self.keep(Value::float(value))
    }

    /// `nvs_types::ConstArg::Str`, already cooked by the checker — one fresh
    /// allocation per call, which is what a written string literal costs at
    /// its own site too (`nvs-codegen`'s known gap 4).
    pub fn str(&mut self, text: &str) -> Value {
        self.keep(Value::str(crate::string::NvsStr::new(text.as_bytes())))
    }

    /// Records the one reference `value` carries and hands back a borrow of
    /// it — the single place this set takes ownership, so the count it keeps
    /// and the count [`Drop`] releases cannot disagree.
    fn keep(&mut self, value: Value) -> Value {
        self.built.push(value);
        value
    }
}

impl Drop for RowValues {
    fn drop(&mut self) {
        #[expect(
            unsafe_code,
            reason = "this row holds exactly one reference per materialized                       value, made here, and the call that borrowed them has                       returned"
        )]
        unsafe {
            for value in self.built.drain(..) {
                value.release();
            }
        }
    }
}

/// One test isolate's own copies of the fixtures it asked for — `rule:testing/fixtures`'s
/// "built once in the parent, copied into each test", which is
/// `rule:classes/graph-copy`'s graph copy and nothing else.
///
/// It is [`RowValues`]'s shape over a different source, and it is here for that
/// type's reason: the copy carries one reference each and somebody has to
/// release it once the call has returned, which `nvs-cli` cannot spell. Where
/// `Fixtures` is the **per-class** owner, this is the **per-test** one, and
/// that split is § 2 rather than a convenience — the set outlives the class's
/// last test, a copy does not outlive the isolate it was made for.
///
/// The copy is made on the **parent's** stack, before the isolate exists, and
/// is moved into it. That is the same ordering `nvs_host::Isolate::start`
/// gives its argument, and for the same reason `rule:security/arena-is-an-ownership-root` gives: an arena is
/// an ownership root and not an address range, so where the walk runs decides
/// nothing and who releases the result decides everything.
#[derive(Debug, Default)]
pub struct CrossedFixtures {
    built: Vec<Value>,
}

impl CrossedFixtures {
    /// Copies the values `needs` names out of `from`, in that order.
    ///
    /// `None` when one of them was never built, which is the same internal
    /// inconsistency [`Fixtures::values`] answers `None` for.
    ///
    /// # Errors
    ///
    /// [`crate::GraphError`] naming the fixture value that has no meaning on
    /// the other side — a callable, or an object holding a host handle. That is
    /// the **parent's** fault in `nvs_host::Isolate`'s own sense: the value was
    /// built before any test isolate existed, so nothing has run yet and the
    /// caller reports it against the test that asked for it.
    pub fn copy(from: &Fixtures, needs: &[String]) -> Option<Result<Self, crate::GraphError>> {
        let borrowed = from.values(needs)?;
        let mut built = Vec::with_capacity(borrowed.len());
        for value in borrowed {
            #[expect(
                unsafe_code,
                reason = "`from` holds a reference to each of these for the whole \
                          of this call, so the retain the walk then consumes is \
                          made against a live payload — and the source is left \
                          with the one it started with, which is what keeps the \
                          copy a copy rather than an adoption"
            )]
            unsafe {
                value.retain();
            }
            match crate::graph::copy_graph(value) {
                Ok(crossed) => built.push(crossed),
                // Whatever crossed already is this type's, so the partial copy
                // is dropped rather than leaked: `built` is moved into a `Self`
                // that releases it.
                Err(refused) => {
                    drop(Self { built });
                    return Some(Err(refused));
                }
            }
        }
        Some(Ok(Self { built }))
    }

    /// The copies, in the order `needs` named them — **borrowed**, exactly as
    /// [`Fixtures::values`] is: the call retains its own.
    #[must_use]
    pub fn values(&self) -> &[Value] {
        &self.built
    }
}

impl Drop for CrossedFixtures {
    fn drop(&mut self) {
        #[expect(
            unsafe_code,
            reason = "this set holds exactly one reference per copy, made by the \
                      walk in `copy`, and the call that borrowed them has returned"
        )]
        unsafe {
            for value in self.built.drain(..) {
                value.release();
            }
        }
    }
}

/// `rule:testing/runner-is-strict`'s
/// one test: a fresh instance of `class`, its `method` called on that instance
/// with `args` — § 8's fixtures, in the order the checker resolved them — and
/// the instance released.
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
///   test failing — a failed assertion arrives here as `rule:testing/failure-ledger`'s
///   `Core\Test\Failure` like any other throw;
/// - [`crate::THROWN`] for a constructor that declares parameters, which
///   nothing supplies: § 7 makes the constructor `setUp` and § 8 injects into
///   the *test method's* parameters, so there is no roster a constructor
///   argument could come out of. A throw rather than a [`crate::FATAL`]: it is
///   a limit of this runner, so the suite reports that test and carries on;
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
    args: &[Value],
) -> Result<(), i32> {
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    let desc = unsafe { &*class };
    // Checked **before** anything is constructed rather than trusted from the
    // roster: the checker resolved one fixture per declared parameter, so a
    // disagreement is an internal inconsistency, and a call made at the wrong
    // arity reads slots the callee's frame does not own. Ahead of construction
    // because a bail-out after it would abandon the instance.
    if let Some(row) = desc.method_row(method)
        && usize::try_from(row.arity).unwrap_or(usize::MAX) != args.len()
    {
        return Err(crate::abi::record_fault(
            ctx,
            Fault::fatal(format!(
                "internal error: `{}`'s `{method}()` declares {} parameter(s) and the test \
                 runner supplied {}",
                desc.name(),
                row.arity,
                args.len()
            )),
        ));
    }
    let receiver = match desc.method_row(crate::object::CONSTRUCTOR) {
        Some(row) if row.arity > 0 => {
            return Err(crate::abi::record_fault(
                ctx,
                Fault::thrown(format!(
                    "`{}`'s constructor declares {} parameter(s), and this runner supplies \
                     none: `rule:testing/constructor-is-setup` makes the constructor `setUp`, and § 8's fixtures \
                     fill the test method's own parameters",
                    desc.name(),
                    row.arity
                )),
            ));
        }
        Some(_) =>
        {
            #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
            match unsafe { crate::object::construct(ctx, class, &[]) } {
                Ok(value) => value,
                Err(fault) => return Err(crate::abi::record_fault(ctx, fault)),
            }
        }
        // No `constructor` row at all: `rule:classes/definite-property-initialization` gives such a class nothing
        // to run, so the allocation with its armed defaults *is* the instance.
        None => {
            #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
            let object = unsafe { NvsObj::new(class) };
            Value::object(object)
        }
    };

    let outcome = call_method(ctx, receiver, method, args, "the test runner");
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
    /// [`call_erased_method`], which is `rule:types/erased-member-access`'s deferral for a call.
    ///
    /// `args[0]` is the receiver, still **tagged**: nothing proved it holds an
    /// object, so the test is made below rather than here. `args[1]` is the
    /// member name, an immortal `string` constant in the calling unit's data
    /// section rather than an allocation per call. `args[2]` is **one array**
    /// holding every argument in written order, which is why this is a
    /// fixed-arity helper at all: a site's own count is not the fact the call
    /// turns on — a `...` argument's count is its subject's own length, and
    /// what the arguments are judged against is a callee chosen when this
    /// runs. `nvs_ir::Helper::CallCallableArray` is the same shape for the same
    /// reason.
    ///
    /// The entries are **borrowed** from an array the caller owns for the
    /// length of this call, exactly as [`crate::callable::nvs_call_callable_array`]'s
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
        // and must not run its own drop, exactly as `nvs_call_callable_array`'s
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
