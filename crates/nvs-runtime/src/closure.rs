//! Calling an `rule:types/closure-literal`
//! closure value from native code.
//!
//! A closure is an ordinary Novis object whose class declares exactly one
//! method, [`CLOSURE_INVOKE`], and one field per captured binding —
//! `nvs_ir::lower::lower_closure` owns that representation and says why it
//! reuses the object machinery rather than adding a second heap shape. So
//! everything here is already available: [`crate::nvs_class_method`] finds
//! the compiled address, and [`crate::call`] reaches it under exactly the
//! `rule:errors/propagation` signature every
//! other compiled function has.
//!
//! # Why this exists at all
//!
//! `Core\Arr::filter` and its siblings are native Rust helpers, and a helper
//! *borrows* its arguments while a compiled method *owns* its parameters —
//! the two halves of the convention `nvs_stdlib`'s own module docs state.
//! [`call_closure`] is the one place that mismatch is reconciled: it retains
//! the receiver and every argument on the way in, so the callee's exit sweep
//! releases references this function paid for rather than the caller's.
//! Getting that wrong in each `Core` member separately is exactly the kind of
//! hand-written refcount protocol Stage 6's valgrind leg exists to catch, so
//! there is one implementation and no second one.
//!
//! # Why the parameter types are checked here, of all places
//!
//! `rule:types/closure-literal`
//! gives `callable` no parameter list, so **no checker can compare a call site
//! against the body it will reach**, and the compiled `invoke` reads argument
//! slot *i* at its own declared representation. Hand it a mismatch and the
//! callee reinterprets the payload — an `int` read as an `NvsStr` pointer is
//! an arbitrary dereference, not a fault, and
//! `Core\Arr::map($ints, fn (string $s): string => $s)` over an `array<int>`
//! is all it takes to write one.
//!
//! So the closure object carries its parameter tags
//! ([`CLOSURE_PARAM_TAGS_SLOT`]) the way it already carries its arity
//! ([`CLOSURE_ARITY_SLOT`]), written at the literal by
//! `nvs_ir::lower::lower_closure_literal` from the declared types, and
//! [`check_param_tags`] compares one against each argument on the way in —
//! throwing the [`crate::ThrownClass::Logic`] `LogicError` [`nvs_call_closure`]
//! answers a bad arity with. It sits in [`call_closure`] because that is the
//! one path *both* callers take, a `Core` member's callback and `rule:types/closure-literal`'s
//! `$fn(...)` alike; putting it in either caller would leave the other one
//! holding the hole. What it costs, and the one argument it converts rather
//! than compares — `rule:types/conversion`'s `int`-into-`float` widening, which no checker
//! was there to insert — are that function's own doc comment.
//!
//! One caller does have a checker in front of it. Where a call site's callee
//! carries `rule:types/callable-signature`'s written signature, every argument
//! was proven against a declared parameter type where it was written, and the
//! widening the paragraph above names was inserted there rather than here — so
//! that site reaches [`nvs_call_closure_proven`] and pays nothing per argument.
//! [`TagCheck`] is which of the two a call is, and it is the only difference
//! between them: the metadata stays on every closure object, because a closure
//! does not know at its literal which kind of site will call it.

use crate::abi::{Fault, NvsFn, OK};
use crate::ctx::Ctx;
use crate::object::{ClassDesc, NvsObj};
use crate::value::{Tag, Value};

/// The one method a closure's captured-environment class answers. Must agree
/// with `nvs_ir::lower`'s own constant; `nvs-codegen`'s
/// `a_closure_is_reachable_through_the_method_table` holds the two together.
pub const CLOSURE_INVOKE: &str = "invoke";

/// The field slot holding how many parameters a closure declares, not
/// counting the receiver — always the first, since a descriptor carries no
/// field names for a native caller to search.
///
/// `nvs_ir::lower`'s `FN_ARITY` is the definition side and owns the reason
/// the arity is stored per object at all; `nvs-codegen`'s
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
/// a shift and a mask and needs no table here. `nvs_ir::lower`'s
/// `FN_PARAM_TAGS` is the definition side and owns why the object carries this
/// at all; `nvs-codegen`'s `param_tag_nibbles_are_the_runtime_tag_bytes` holds
/// its map against the tag bytes compiled code actually writes.
pub const CLOSURE_PARAM_TAGS_SLOT: usize = 1;

/// The field a **first-class callable**'s object records its target's
/// parameter names under, comma-separated in declaration order.
///
/// Read by name rather than by index, and that is the whole safety argument:
/// `nvs_ir::lower`'s `FN_PARAM_NAMES` writes this field at a `Class::method(...)`
/// and at nothing else, so a `fn` literal's closure — whose third field is its
/// first *capture* — answers [`closure_param_names`] `None` rather than having
/// a captured string read as a parameter list. [`CLOSURE_PARAM_NAMES_SLOT`] is
/// the hint that keeps the lookup one comparison.
///
/// Must agree with that constant; `nvs-ir`'s
/// `a_first_class_callable_records_its_targets_parameter_names` is the writing
/// side asserted on its own — the field list and the joined names — and it
/// stands alone rather than beside a behavioural end-to-end case for the
/// reason that file's module doc gives: the one member that reads these needs
/// a connection's slot, which no corpus case is offered.
pub const CLOSURE_PARAM_NAMES: &str = "fn#names";

/// Where [`CLOSURE_PARAM_NAMES`] sits on a first-class callable's object —
/// third, after the arity and the tags.
///
/// A **hint** rather than an index, unlike [`CLOSURE_ARITY_SLOT`] and
/// [`CLOSURE_PARAM_TAGS_SLOT`], because this field is not on every closure
/// class: [`ClassDesc::field_slot`] takes it, checks that one slot, and falls
/// back to a search that answers `None` for a class without the field. So the
/// common case costs a comparison and the absent case cannot be mistaken for
/// a hit.
pub const CLOSURE_PARAM_NAMES_SLOT: usize = 2;

/// The one [`CLOSURE_PARAM_TAGS_SLOT`] nibble that is not a [`Tag`]: the
/// parameter is `mixed`, `?T` or another union, whose representation *is* a
/// tag chosen at run time, so no argument can be wrong for it.
///
/// Fifteen is the **top** of the nibble rather than the first number past the
/// tag roster, and that is deliberate: the roster grows — `rule:classes/an-unwritten-property-read-throws`'s
/// never-written storage state ([`Tag::Unset`]) is one of its tags — so a
/// nibble chosen as "one past the last tag" is one a later tag collides with.
/// [`Tag::from_byte`] answering `None` for it is half of `nvs-codegen`'s
/// `the_any_nibble_denotes_no_tag_at_all`, which holds the two apart;
/// `nvs_ir::lower::FN_PARAM_TAG_ANY` is the other end of the same number and
/// the two are held together by that test.
pub const CLOSURE_PARAM_TAG_ANY: u8 = 15;

/// How many parameters [`CLOSURE_PARAM_TAGS_SLOT`] can describe: one nibble
/// each in a 64-bit payload. A closure declaring more cannot be called —
/// [`check_param_tags`] says why refusing is the answer.
const CLOSURE_PARAM_TAGS_CAPACITY: usize = 16;

/// Whether a call still owes [`check_param_tags`], which is the whole of what
/// `rule:types/callable-signature`'s proof buys.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TagCheck {
    /// Nothing proved what the arguments hold: the callee is reached through
    /// bare `callable` or from a `Core` member's own roster, where the callback
    /// arrived as a value and its declared parameters are the closure object's
    /// metadata alone.
    Required,
    /// The call site's callee carried a written signature, so `nvs_types`
    /// checked every argument against a declared parameter type and `nvs-ir`
    /// inserted the one conversion the tag check would otherwise have
    /// performed. Reading the tags again could only agree.
    Proven,
}

/// Calls the closure `closure` with as many leading `args` as it declares
/// parameters, borrowing every one of them.
///
/// The trailing arguments a shorter closure does not want are dropped rather
/// than passed, which is
/// [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
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
/// declares in that position, and which `rule:types/conversion`'s `int`-into-`float`
/// widening does not reconcile — [`check_param_tags`], which runs before
/// anything is retained or passed.
/// [`Fault::Fatal`] when `closure` is not a closure value at all, or declares
/// more parameters than the caller has to offer — both engine faults: the
/// checker only admits an `rule:types/callable-is-a-closure` closure value where a `callable` is
/// expected, and no `Core` member offers fewer than the spec says it does.
pub fn call_closure(ctx: &mut Ctx, closure: Value, args: &[Value]) -> Result<Value, Fault> {
    call_closure_with(ctx, closure, args, TagCheck::Required)
}

/// [`call_closure`], plus which of [`TagCheck`]'s two kinds of call site this
/// is — the one function both spellings run through, so a caller that skips the
/// per-argument check still gets the arity trim, the retains and the error edge
/// unchanged.
fn call_closure_with(
    ctx: &mut Ctx,
    closure: Value,
    args: &[Value],
    tags: TagCheck,
) -> Result<Value, Fault> {
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
                  which `nvs-codegen` fills only with compiled functions of \
                  exactly this signature"
    )]
    let target: NvsFn = unsafe { std::mem::transmute::<*const u8, NvsFn>(target) };

    let mut slots = Vec::with_capacity(args.len() + 1);
    slots.push(closure);
    slots.extend_from_slice(args);
    // Over this frame's own copy rather than the caller's slice, because the
    // check both refuses and *converts*: a widened `int` must reach the callee
    // as a `float` while the caller keeps owning the `int` it passed. Neither
    // tag is refcounted, so the retain below is unaffected by the substitution.
    // A proven site had both done for it where the call was written, and the
    // slots are already what the callee reads.
    if tags == TagCheck::Required {
        check_param_tags(
            "a `callable`",
            closure_param_tags(closure)?,
            &mut slots[1..],
        )?;
    }
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

/// `nvs_ir::Helper::CallClosure` — `rule:types/closure-literal`'s `$fn(...)`, which is compiled
/// code's own way into [`call_closure`]. `args[0]` is the closure and
/// `args[1..argc]` the arguments it was called with, in written order.
///
/// **The one helper that takes a count.** Every other one's arity is a
/// literal in its [`crate::nvs_helper!`] expansion, because a conversion or a
/// comparison has the same shape at every call site; a closure call's arity is
/// the *call site's*, so it travels beside the slot and this function is
/// written out rather than generated. `nvs-codegen`'s `Signatures::helper_variadic`
/// is the other half of that ABI.
///
/// Too *many* arguments is not an error: [`call_closure`] trims to the
/// arity the closure recorded, which is spec § 2's "every callback receives
/// `($value, $key)` and may declare fewer parameters" and is also PHP's own
/// answer for extra positional arguments to a userland function. Too *few* is
/// the catchable `LogicError` below rather than the engine fault
/// [`call_closure`] answers a native caller with: a `callable` carries no
/// parameter list for the checker to count against
/// (`rule:types/closure-literal`), so a program can reach it, and a program-reachable failure is a
/// throw (`rule:errors/propagation`).
///
/// # Safety
///
/// `ctx`, `args` and `out` must each be non-null, aligned and valid for the
/// duration of the call; `args` must point at `argc` initialized values, of
/// which there must be at least one; and `out` must be writable. Compiled Novis
/// code satisfies all of it by construction.
#[expect(
    unsafe_code,
    reason = "the helper ABI's pointer contract, discharged exactly where \
              `nvs_helper!` discharges it for every fixed-arity helper"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_call_closure(
    ctx: *mut Ctx,
    args: *const Value,
    argc: usize,
    out: *mut Value,
) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, args, argc, out, |ctx, args| {
            closure_call_body(ctx, args, TagCheck::Required)
        })
    }
}

/// `nvs_ir::Helper::CallClosureProven` — [`nvs_call_closure`] for a call site
/// whose callee carried `rule:types/callable-signature`'s written signature.
///
/// Identical in every respect but one: the arguments are not compared against
/// the closure's recorded parameter tags, because `nvs_types` compared them
/// against the *declared* ones where the call was written and `nvs-ir` inserted
/// the conversion [`check_param_tags`] would have performed. That is what the
/// signature is for — the check is a `rule:programs/memory-priority` priority 1
/// guard, and a proven site discharges it at compile time rather than per
/// argument, per call.
///
/// The arity trim, the retains, the borrowed arguments and the error edge are
/// all [`nvs_call_closure`]'s unchanged, including the catchable `LogicError`
/// for too few arguments: the checker counts a call site's list against the
/// *type's* parameters, and the closure the value actually holds may declare
/// any number up to that (`rule:types/callable-arity`), so the arity read here
/// is still the closure's own.
///
/// # Safety
///
/// [`nvs_call_closure`]'s, unchanged.
#[expect(
    unsafe_code,
    reason = "the helper ABI's pointer contract, discharged exactly where \
              `nvs_helper!` discharges it for every fixed-arity helper"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_call_closure_proven(
    ctx: *mut Ctx,
    args: *const Value,
    argc: usize,
    out: *mut Value,
) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, args, argc, out, |ctx, args| {
            closure_call_body(ctx, args, TagCheck::Proven)
        })
    }
}

/// The body both closure-call helpers hand [`crate::run_helper`]: split the
/// closure off the front of the slot run, and call it.
fn closure_call_body(ctx: &mut Ctx, args: &[Value], tags: TagCheck) -> Result<Value, Fault> {
    let Some((closure, passed)) = args.split_first() else {
        return Err(Fault::fatal(
            "internal error: a closure call reached the runtime with no closure at all",
        ));
    };
    call_closure_from_nvs(ctx, *closure, passed, tags)
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::CallClosureArray` — `rule:types/closure-literal`'s `$fn(...)` where the
    /// call site wrote a `...` argument, so how many arguments there are is the
    /// spread subject's own run-time length rather than the site's own count.
    ///
    /// `args[0]` is the closure and `args[1]` **one array** holding every
    /// argument in call order: the array `nvs_ir::lower::call` already builds
    /// for a variadic parameter's tail, with each spread flattened into it by
    /// [`crate::array::nvs_array_spread`]. That is the whole reason this is a second
    /// helper rather than a wider [`nvs_call_closure`] — that one's argument
    /// count is a literal in the emitted call, which is exactly the fact a `...`
    /// does not have.
    ///
    /// Everything after the unpacking is [`nvs_call_closure`]'s, through the
    /// one [`call_closure_from_nvs`] they share: entries are passed positionally
    /// in key order, extra ones are trimmed by [`call_closure`], and too few is
    /// the same catchable `LogicError`. The entries are **borrowed** from an
    /// array the caller owns for the length of this call, and `call_closure`
    /// retains each one it actually passes.
    ///
    /// It has no [`nvs_call_closure_proven`] twin, and needs none: what a
    /// signature proves is each argument against the parameter it fills, and a
    /// `...` is precisely the shape where which argument fills which parameter
    /// is not known until this function unpacks it. So a spread keeps
    /// [`TagCheck::Required`] whatever the callee's type says.
    fn nvs_call_closure_array(ctx, args: [2]) {
        let array = args[1].array_ptr().ok_or_else(|| {
            crate::helpers::wrong_tag("nvs_call_closure_array", Tag::Array, args[1])
        })?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for this read"
        )]
        // The caller's reference is the caller's: this handle reads the table
        // and must not run its own drop, exactly as `nvs_array_spread`'s
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
        call_closure_from_nvs(ctx, args[0], &entries, TagCheck::Required)
    }
}

/// The field a closure that uses `$this` stores it under. `nvs_ir::lower`'s
/// capture list names it, and [`bind_closure`] finds it by this name because
/// a capture has no fixed slot.
pub const CLOSURE_THIS_FIELD: &str = "this";

crate::nvs_helper! {
    /// `nvs_ir::Helper::BindClosure` — `$fn->bindTo($obj)` and `$fn->bind($obj)`,
    /// and the first half of `$fn->call($obj, ...)`. [`bind_closure`] is the
    /// whole operation.
    fn nvs_closure_bind(_ctx, args: [2]) {
        bind_closure(args[0], args[1])
    }
}

/// `rule:types/callable-absorbs-closure`'s rebind: `closure` with `$this`
/// replaced by `this`, as a fresh reference the caller owns.
///
/// A closure whose body does not use `$this` has no [`CLOSURE_THIS_FIELD`]
/// (`rule:statements/a-closure-binds-this-only-where-it-uses-it`), so it comes
/// back unchanged, one more reference to the same object, whatever `this` is.
///
/// One that does is copied with [`crate::object::nvs_object_clone`], and the
/// copy's `this` slot takes the new object. The compiled body reads `$this` at
/// the layout of the class it was checked against, and that class is the
/// slot's one entry in [`ClassDesc::field_classes`]. So the new object must
/// conform to it, which a subclass does because its slots extend its parent's.
/// Anything else, `null` among them, throws a `LogicError` and copies nothing:
/// reading an unrelated class at that layout would be a memory-safety hole.
///
/// **Cost:** one class test by name and one closure copy per rebind, freed like
/// any object. A `$this`-free closure pays only the field lookup.
///
/// # Errors
///
/// The `LogicError` above, and [`require_closure`]'s for a receiver that is
/// not a closure.
pub fn bind_closure(closure: Value, this: Value) -> Result<Value, Fault> {
    require_closure(closure)?;
    let ptr = closure
        .obj_ptr()
        .ok_or_else(|| Fault::fatal("internal error: a closure reached a rebind untagged"))?;
    #[expect(
        unsafe_code,
        reason = "`require_closure` just read this object's descriptor, and the \
                  caller owns a reference to the object for this whole call"
    )]
    let desc: &ClassDesc = unsafe { &*NvsObj::class_of(ptr) };
    let Some(slot) = desc.field_slot(CLOSURE_THIS_FIELD, 2) else {
        #[expect(
            unsafe_code,
            reason = "the caller's reference keeps the object live, and the \
                      retain is the reference the returned value owns"
        )]
        unsafe {
            closure.retain();
        }
        return Ok(closure);
    };
    let admitted = desc.field_classes(slot).unwrap_or(&[]);
    let refused = match this.obj_ptr() {
        Some(target) if !target.is_null() => {
            #[expect(
                unsafe_code,
                reason = "the caller owns a reference to this value, so the \
                          object is live and its descriptor is too"
            )]
            let class = unsafe { &*NvsObj::class_of(target) };
            if admitted.iter().any(|label| class.conforms_to_name(label)) {
                None
            } else {
                Some(format!("an object of class `{}`", class.name()))
            }
        }
        _ => Some(match this.tag() {
            Some(Tag::Null) => "`null`".to_owned(),
            Some(tag) => format!("a value of type `{}`", tag.describe()),
            None => "a malformed value".to_owned(),
        }),
    };
    if let Some(given) = refused {
        let wanted = admitted.first().map_or("its own class", String::as_str);
        return Err(Fault::thrown_as(
            crate::ThrownClass::Logic,
            format!(
                "this closure uses `$this`, so it can only be bound to an object of class \
                 `{wanted}` or a subclass of it. The argument is {given}"
            ),
        ));
    }
    #[expect(
        unsafe_code,
        reason = "the caller's reference keeps the closure live for the copy, \
                  and `this` is live for the retain the copy's slot takes over"
    )]
    let copy = unsafe {
        let copy = NvsObj::from_raw(crate::object::nvs_object_clone(ptr));
        this.retain();
        copy
    };
    copy.set_field(slot, this);
    Ok(Value::object(copy))
}

/// [`call_closure`] under the one check a *program* can reach, shared by the
/// two helpers compiled Novis code calls a closure through.
///
/// A native caller has no arity mistake to make — a `Core` member offers every
/// argument the spec says it does, so [`call_closure`] answers it with an
/// engine fault. An Novis call site's list is whatever was written there, and
/// `rule:types/closure-literal`
/// gives the checker no parameter list to count it against, so too few is
/// program-reachable and therefore a throw
/// (`rule:errors/propagation`).
///
/// The callee itself is checked first, for the same reason: a callee typed
/// `mixed` or `?callable` reaches here unproven, so a value that is not a
/// closure is program-reachable and throws ([`require_closure`]) rather than
/// reaching the engine faults below.
///
/// # Errors
///
/// The catchable `LogicError`s above, plus everything [`call_closure`] itself
/// answers with.
fn call_closure_from_nvs(
    ctx: &mut Ctx,
    closure: Value,
    passed: &[Value],
    tags: TagCheck,
) -> Result<Value, Fault> {
    require_closure(closure)?;
    let arity = closure_arity(closure)?;
    if passed.len() < arity {
        return Err(Fault::thrown_as(
            crate::ThrownClass::Logic,
            format!(
                "too few arguments to a `callable`: it declares {arity} parameter(s), {} given",
                passed.len()
            ),
        ));
    }
    call_closure_with(ctx, closure, passed, tags)
}

/// `rule:types/callable-is-a-closure`'s run-time half: a compiled `$f(...)`
/// whose callee is not a closure throws a `LogicError` naming what it is.
///
/// # Errors
///
/// The catchable `LogicError` for a value whose tag or class is not a
/// closure's, and [`Fault::Fatal`] for an object with no class descriptor.
fn require_closure(closure: Value) -> Result<(), Fault> {
    let called = match closure.obj_ptr() {
        Some(ptr) if !ptr.is_null() => {
            #[expect(
                unsafe_code,
                reason = "the caller owns a reference to this object, so the \
                          allocation and its descriptor are both live for this read"
            )]
            let desc: *const ClassDesc = unsafe { NvsObj::class_of(ptr) };
            if desc.is_null() {
                return Err(Fault::fatal(
                    "internal error: a called object has no class descriptor".to_owned(),
                ));
            }
            #[expect(
                unsafe_code,
                reason = "just checked the descriptor is non-null, and it is owned by \
                          the compiled unit's class table for that unit's whole life"
            )]
            let class = unsafe { &*desc };
            if class.is_closure() {
                return Ok(());
            }
            format!("an object of class `{}`", class.name())
        }
        _ => match closure.tag() {
            Some(Tag::Null) => "`null`".to_owned(),
            Some(tag) => format!("a value of type `{}`", tag.describe()),
            None => "a malformed value".to_owned(),
        },
    };
    Err(Fault::thrown_as(
        crate::ThrownClass::Logic,
        format!("{called} cannot be called: only a closure is a `callable`"),
    ))
}

/// How many parameters `closure` declares — [`CLOSURE_ARITY_SLOT`], read
/// straight off the object with no call made.
///
/// [`call_closure`] uses it to trim the argument list, and a caller asks it
/// directly to avoid *building* an argument that trimming would throw away:
/// `Core\Arr::map`'s `$key` costs a rendered decimal and an `NvsStr` per
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
    let slot = unsafe { crate::object::nvs_object_field_get(ptr, CLOSURE_ARITY_SLOT) };
    let arity = slot.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `callable`'s arity slot carried tag {} rather than an int",
            slot.tag_byte()
        ))
    })?;
    usize::try_from(arity)
        .map_err(|_| Fault::fatal("internal error: a `callable` recorded a negative arity"))
}

/// The [`CLOSURE_PARAM_TAGS_SLOT`] word `closure` recorded — the nibbles
/// [`check_param_tags`] judges its arguments against, read straight off the
/// object with no call made.
///
/// Split out of that check so the check itself takes a word: an erased method
/// call reads the same encoding off [`crate::MethodRow::param_tags`] instead,
/// and only *where the word comes from* differs between the two paths.
///
/// # Errors
///
/// [`Fault::Fatal`] when `closure` is not a closure value at all, or when its
/// tag slot does not hold an `int` — both compiler or runtime bugs rather than
/// anything a program can write.
fn closure_param_tags(closure: Value) -> Result<u64, Fault> {
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
    let slot = unsafe { crate::object::nvs_object_field_get(ptr, CLOSURE_PARAM_TAGS_SLOT) };
    let word = slot.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `callable`'s parameter-tag slot carried tag {} rather than an int",
            slot.tag_byte()
        ))
    })?;
    // The sixteenth nibble sits in the sign bit; the slot holds the same 64
    // bits either way, and only the nibbles are ever read.
    Ok(u64::from_ne_bytes(word.to_ne_bytes()))
}

/// The parameter names `closure`'s target declares, in declaration order, or
/// `None` for a closure that records none.
///
/// `None` is the honest answer for an `fn` literal and is not a failure:
/// [`CLOSURE_PARAM_NAMES`] is written at a `Class::method(...)` and nowhere
/// else, so a literal's closure genuinely has no such field. A caller that
/// *needs* names — [ADR 0006](/docs/decisions/0006.md)
/// § *Decision*'s `args:` binding, which is by name — turns that `None` into
/// its own refusal naming the form the program wrote, because only the caller
/// knows which member it is refusing on behalf of.
///
/// An empty name list is `Some(&[][..])`-shaped rather than `None`: a target
/// declaring no parameters is a callable an `args:`-less entry may open, and
/// it is not the same fact as a closure that never recorded any.
///
/// # Errors
///
/// [`Fault::Fatal`] when `closure` is not an object, when its class has no
/// descriptor, or when the field is present and does not hold text. Each is a
/// compiler or embedder bug rather than a program's: this lowering writes a
/// `ConstStr` into that slot or writes no field at all.
pub fn closure_param_names(closure: Value) -> Result<Option<Vec<String>>, Fault> {
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
    let desc: *const ClassDesc = unsafe { NvsObj::class_of(ptr) };
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
    let found = unsafe { &*desc }.field_slot(CLOSURE_PARAM_NAMES, CLOSURE_PARAM_NAMES_SLOT);
    let Some(slot) = found else {
        return Ok(None);
    };
    #[expect(
        unsafe_code,
        reason = "the slot index came from this object's own descriptor, and \
                  the caller owns a reference keeping the object live"
    )]
    let value = unsafe { crate::object::nvs_object_field_get(ptr, slot) };
    let text = value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `callable`'s parameter-name slot carried tag {} rather than text",
            value.tag_byte()
        ))
    })?;
    // An entry declaring no parameters writes the empty string, which splits
    // to one empty name — the filter is what makes that the empty list the
    // caller means. No declared name can be empty, so nothing else is lost.
    Ok(Some(
        text.split(',')
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect(),
    ))
}

/// Refuses `args` unless every one of them carries the tag the callee's
/// corresponding parameter declares — the check that stands in for the one no
/// checker can make — over the nibble `word` that callee recorded, naming it
/// `callee` in whatever it has to report.
///
/// `rule:types/closure-literal`
/// gives `callable` no parameter list, so a call site has nothing to compare
/// against and the compiled `invoke` reads argument slot *i* at its own
/// declared representation — an `int` handed to a `string` parameter is
/// dereferenced as an `NvsStr` pointer. This is the one place that can still
/// tell, because the closure object carries what the literal declared
/// (`nvs_ir::lower`'s `FN_PARAM_TAGS`), and it is on the path *both* callers
/// take: a `Core` member's callback and `rule:types/closure-literal`'s `$fn(...)` alike.
///
/// **It is the erased *method* call's check too**, which is why it takes a
/// word rather than a closure object. A `mixed` receiver defers the same
/// question one storage kind along (`rule:types/erased-member-access`), and
/// [`crate::MethodRow::param_tags`] carries the callee's nibbles in this very
/// encoding so that `rule:types/conversion`'s one implicit conversion is written once —
/// two copies of it are two places for it to stop agreeing.
/// `docs/adr/README.md` § *Decisions taken at project start* owns that
/// decision, and [`crate::dispatch::call_erased_method`] is the other caller.
///
/// One shift, one mask and one byte comparison per argument, on the callback
/// path — priority 3 spent on priority 1, which is the direction AGENTS.md's
/// ordering names, and the only design available while `callable` stays
/// unparameterized.
///
/// # The one conversion, rather than a refusal
///
/// The comparison is exact everywhere except the single position
/// `rule:types/conversion` admits an
/// implicit conversion: an `int` or `uint` arriving at a `float` parameter is
/// *widened in place* rather than refused, through
/// [`crate::helpers::widen_to_float`] and therefore through the same row a
/// written `as float` takes. Above 2^53 that row refuses, and so does this —
/// as `ArithmeticError`, the class `rule:types/arithmetic` names for a numeric overflow
/// and the one `crate::helpers`' `numeric_does_not_fit` raises for the written
/// `as float`.
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
/// (`rule:errors/propagation`). Refusing the
/// call in the second case is deliberate: passing an argument whose declared
/// tag was never written down is exactly the read this function exists to
/// prevent, and no spec callback comes close to sixteen parameters.
///
/// [`Fault::Fatal`] when either side carries a byte that denotes no
/// representation at all — a compiler or runtime bug rather than something a
/// program can write. Where the word itself comes from is the caller's, and so
/// are the faults reading it can raise ([`closure_param_tags`]).
pub(crate) fn check_param_tags(callee: &str, word: u64, args: &mut [Value]) -> Result<(), Fault> {
    for (i, arg) in args.iter_mut().enumerate() {
        if i >= CLOSURE_PARAM_TAGS_CAPACITY {
            return Err(Fault::thrown_as(
                crate::ThrownClass::Logic,
                format!(
                    "{callee} declares more than {CLOSURE_PARAM_TAGS_CAPACITY} parameters and \
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
                "internal error: {callee} recorded nibble {nibble} for parameter {}, which \
                 denotes no representation",
                i + 1
            ))
        })?;
        let given = arg.exact_tag().ok_or_else(|| {
            Fault::fatal(format!(
                "internal error: argument {} to {callee} carried tag {}, which denotes no \
                 representation",
                i + 1,
                arg.tag_byte()
            ))
        })?;
        if given != required {
            // `rule:types/conversion`'s one implicit conversion, and there is no second:
            // an `int` or `uint` arriving at a `float` parameter widens under
            // that ADR's 2^53 rule instead of being refused. It is applied
            // here because no checker saw this call site to insert it — a
            // `callable` has no parameter list (`rule:types/closure-literal`) — and out of
            // `crate::helpers`'s own row, so the boundary is the same one a
            // written `as float` lands on.
            // `tests/conformance/core/arr-a-callback-float-parameter-widens-an-int-and-stops-at-2-53.nvst`
            // pins both sides of it from Novis.
            if required == Tag::Float && matches!(given, Tag::Int | Tag::Uint) {
                *arg = crate::helpers::widen_to_float(*arg).ok_or_else(|| {
                    Fault::thrown_as(
                        crate::ThrownClass::Arithmetic,
                        format!(
                            "cannot convert argument {} to {callee} from `{}` to `float`",
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
                    "argument {} to {callee} must be of type {}, {} given",
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
///
/// What makes a value a closure is its class's
/// [`ClassDesc::is_closure()`] bit, not the `invoke` in its method table: an
/// ordinary class may declare that name, and calling into one would jump
/// through a method the caller never type-checked against
/// `rule:types/callable-is-a-closure`'s literal. The method lookup that
/// follows the bit can therefore only fail on a descriptor built wrong, which
/// is why it reports an internal error rather than a mismatch.
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
    let desc: *const ClassDesc = unsafe { NvsObj::class_of(ptr) };
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
    let class = unsafe { &*desc };
    if !class.is_closure() {
        return Err(Fault::fatal(format!(
            "internal error: `{}` was passed where a `callable` was expected, and is not a closure",
            class.name()
        )));
    }
    class.method(CLOSURE_INVOKE).ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: `{}` is marked a closure and declares no `{CLOSURE_INVOKE}`",
            class.name()
        ))
    })
}
