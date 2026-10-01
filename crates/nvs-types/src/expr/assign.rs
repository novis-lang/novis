//! Where a value must fit a declared type: `rule:types/unions-and-mixed`'s assignability
//! relation, and the three positions that apply it.
//!
//! [`is_assignable`] is the relation itself, and the one place `rule:types/unions-and-mixed`'s
//! table is written down. It carries two amendments from `rule:types/object-top`: every class
//! or shape type is `<: object` (§ 1), and a shape target is checked
//! structurally by width subtyping plus ordinary field assignability (§ 3,
//! [`shape_satisfied`]) rather than nominally — Novis's one deliberate exception
//! to otherwise fully nominal typing. **`array<T>` is covariant in its element
//! type**, and it is the only generic name in the language that is;
//! [`is_assignable`]'s own doc comment owns that rule and why `rule:types/arrays`'s
//! copy-on-write value semantics make it sound where an aliasing language
//! could not. A qualifier widens but never narrows across it — see
//! [`super::quals`]. `rule:types/literal-types` adds the last amendment: a literal type, an
//! enum-case type, and any union of them widen to their base for free, which
//! [`is_assignable`] answers by one recursion through
//! [`TypeInterner::literal_base`] rather than by four table rows of its own.
//! `rule:types/callable-signature` adds the last, and the first relation here
//! that is not invariant: a written signature satisfies bare `callable`, and
//! two signatures compare by `rule:types/callable-arity`'s prefix match with
//! `rule:types/callable-variance`'s contravariant parameters and covariant
//! return.
//!
//! The positions are [`check_assign`] (`$x = e`, and the reads a target is
//! made of), [`check_compound_assign`] (`$x ⊕= e`, typed as the `$x = $x ⊕ e`
//! it means) and [`check_return`].
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// Whether a value of type `from` may be used where `to` is declared —
/// `to == mixed` always accepts; `from == mixed` never implicitly satisfies
/// a non-`mixed` target (`rule:types/unions-and-mixed`: "`mixed` never absorbs implicitly in
/// the other direction"); otherwise `from` must equal `to`, or `to` must be
/// a union `from` is (or, if `from` is itself a union, every member is) a
/// member of. `rule:types/object-top` amends this with real `object` subtyping (every
/// class or shape is `<: object`), and § 3 with a shape target's structural
/// check (see [`shape_satisfied`]) — the two amendments this ADR makes to
/// `rule:types/unions-and-mixed`'s table, needing `graph`/`signatures` only to resolve a
/// class receiver's own property types against a shape target. `rule:security/taint-propagation`
/// and `rule:security/secret-propagation` add one more: a same-base `string`/`bytes` value widens
/// freely on its `tainted`/`secret` axes (see the qualifier check just above
/// [`shape_satisfied`]'s call), never narrows. `rule:types/literal-types` adds the last:
/// `"a" → string`, `Mode::Read → Mode`, and each of those over a union, are
/// free — see the widening step below for why one recursion states all four
/// rows and why the reverse direction needs no rule to refuse it. `rule:types/conversion`'s own amendment is the last: `int`/`uint` widen into a `float`
/// position, that being the one implicit conversion the language has, and a
/// union source is therefore satisfied member-wise against any target —
/// which is how § 4's `int|float` quotient reaches a declared `float`.
///
/// Takes the interner by `&mut` for that one step: asking whether `from`
/// widens to `to` means naming the type it widens *to*, and naming a type in
/// this crate is interning it. Every caller already holds `Env::interner`
/// mutably, so no call site changes shape.
#[must_use]
pub(crate) fn is_assignable(
    from: TypeId,
    to: TypeId,
    interner: &mut TypeInterner,
    graph: &ClassGraph,
    signatures: &SignatureTable,
) -> bool {
    assignable(from, to, Widening::Converts, interner, graph, signatures)
}

/// Whether the `int`/`uint` → `float` row of [`is_assignable`] applies.
///
/// The row is a conversion: lowering turns the `int` into a `float` where it
/// is stored. That works for a value that is copied into the position, and it
/// does not work for the field of a shape value or object that already exists,
/// because that value is shared and its field keeps the representation it was
/// built with. [`shape_satisfied`] compares fields with [`Widening::InPlace`],
/// at every depth of the field type, so a `{w: int}` value never satisfies
/// `{w: float}`, `{w: ?float}` or `{w: array<float>}`. An array's elements are
/// compared the same way, for the same reason: the elements of an array that
/// already exists keep the representation they were stored with, so an
/// `array<int>` never satisfies `array<float>`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Widening {
    /// The value is converted where it is stored, so `int` fills a `float`.
    Converts,
    /// The value is read where it is, so `int` does not fill a `float`.
    InPlace,
}

/// Marks `value` for conversion to `float` where it is stored, when a storing
/// position accepted it at `to` only through [`is_assignable`]'s `int`/`uint`
/// → `float` row inside a union.
///
/// That is a `to` that is a union naming `float`, and a value whose integer
/// type the union does not name: `3` at `?float` or `float|string`, or a
/// `?int` at `?float`. Such a union erases to a tagged value, so `nvs-ir`
/// cannot tell from the representation that the integer must become a
/// `float`. It reads the mark instead
/// ([`crate::expr_table::ExprTypeTable::widens_to_float`]). A union source
/// converts by its run-time tag, so it is marked only when the target names
/// neither of its integer members: the conversion applies to both tags. A
/// plain `float` target is not marked, because its erased type already
/// converts.
///
/// Called by every position that stores a value at a declared type, after
/// that position has accepted it.
pub(crate) fn note_float_widening(value: &Expr, from: TypeId, to: TypeId, env: &mut Env<'_>) {
    note_float_widening_at(value.unparenthesized().span, from, to, env);
}

/// [`note_float_widening`] for a value with no expression of its own: the
/// result of `$x ??= e`, which `nvs-ir` lowers as the `$x ?? e` it means, at
/// the assignment's span.
pub(crate) fn note_float_widening_at(span: Span, from: TypeId, to: TypeId, env: &mut Env<'_>) {
    let Ty::Union(members) = env.interner.get(to) else {
        return;
    };
    let members = members.clone();
    let float = env.interner.float();
    if !members.contains(&float) {
        return;
    }
    let from = env.interner.literal_base(from);
    let integer = |ty: &TypeId| matches!(env.interner.get(*ty), Ty::Int | Ty::Uint);
    let widens = match env.interner.get(from) {
        Ty::Union(from_members) => {
            let mut integers = from_members.iter().filter(|member| integer(member));
            integers.clone().next().is_some() && integers.all(|member| !members.contains(member))
        }
        _ => integer(&from) && !members.contains(&from),
    };
    if widens {
        env.exprs.record_float_widening(span);
    }
}

/// [`is_assignable`] with the `int` → `float` row on or off — see [`Widening`].
fn assignable(
    from: TypeId,
    to: TypeId,
    widen: Widening,
    interner: &mut TypeInterner,
    graph: &ClassGraph,
    signatures: &SignatureTable,
) -> bool {
    if from == to {
        return true;
    }
    if matches!(interner.get(to), Ty::Mixed) {
        return true;
    }
    if matches!(interner.get(from), Ty::Mixed) {
        return false;
    }
    // `rule:types/literal-types`'s first four rows, all at once. `literal_base` widens a
    // literal atom to its base, an enum-case atom to its enum, and a union
    // member-wise — so `"a"|"b" → string` and `Mode::Read|Mode::Write → Mode`
    // fall out of the same call the two atom rows do, and the recursion then
    // composes each with every rule below it (`"a"` into `string|null`, into
    // `tainted string`, into a `Stringable` target). `literal_base` is
    // idempotent, so the recursion is one level deep.
    //
    // The reverse direction — `string → "a"` — needs no rule to refuse it:
    // nothing below widens a base type *down*, and § 4's last three rows make
    // that direction a checked `as`, never an assignment.
    let widened = interner.literal_base(from);
    if widened != from && assignable(widened, to, widen, interner, graph, signatures) {
        return true;
    }
    // A union target is satisfied member-wise, and **membership is checked
    // first because it is the common case** — a union built out of the same
    // interned ids answers with one `contains` and no recursion at all.
    //
    // The recursion behind it is what makes every *other* rule in this
    // function apply inside a union, and it is not a special case for one
    // caller: `?Animal` refusing a `Dog` was the same hole as
    // `array<T>|Iterable<T>|Iterator<T>` (`nvs_stdlib::registry::CoreTy::Iterated`)
    // refusing a class that implements `Iterable<int>`, since neither the
    // nominal rule nor the shape rule nor the widening rule was ever reached
    // for a member. Unions are flattened and canonicalised, so a member is
    // never itself a union and the recursion is one level deep.
    if let Ty::Union(members) = interner.get(to) {
        let members = members.clone();
        let satisfies = |from: TypeId, interner: &mut TypeInterner| {
            members.contains(&from)
                || members
                    .iter()
                    .any(|member| assignable(from, *member, widen, interner, graph, signatures))
        };
        return match interner.get(from).clone() {
            Ty::Union(from_members) => from_members
                .into_iter()
                .all(|member| satisfies(member, interner)),
            _ => satisfies(from, interner),
        };
    }
    // `rule:types/conversion`'s implicit conversion, and the whole of it: "Implicit
    // conversion happens in exactly one place: **`int` or `uint` widening into
    // a `float` position**, which is the one coercion PHP's own
    // `strict_types` permits, and it throws above 2^53 rather than rounding."
    // The throw is `nvs_ir::lower::Lowering::coerce`'s half; here it is only
    // the accepting.
    //
    // A *union* source is checked member-wise against a non-union target for
    // this row's sake rather than as a rule of its own — `rule:types/arithmetic`'s
    // `int|float` quotient reaching a declared `float` is the one shape that
    // needs it, and it is the ADR's own worked example (`float $avg = $sum /
    // $n;`). Written as a general member-wise check because that is what the
    // relation means, and because narrowing the rule to the quotient's exact
    // two unions would make `is_assignable` name an operator.
    if let Ty::Union(from_members) = interner.get(from) {
        let from_members = from_members.clone();
        return from_members
            .into_iter()
            .all(|member| assignable(member, to, widen, interner, graph, signatures));
    }
    if widen == Widening::Converts
        && matches!(interner.get(to), Ty::Float)
        && matches!(interner.get(from), Ty::Int | Ty::Uint)
    {
        return true;
    }
    if matches!(interner.get(to), Ty::Object)
        && matches!(interner.get(from), Ty::Class(..) | Ty::Shape(_))
    {
        return true;
    }
    if let (Ty::Class(from_q, from_args), Ty::Class(to_q, to_args)) =
        (interner.get(from), interner.get(to))
    {
        let (from_q, from_args) = (from_q.clone(), from_args.clone());
        let (to_q, to_args) = (to_q.clone(), to_args.clone());
        return class_satisfied(
            &from_q, &from_args, &to_q, &to_args, interner, graph, signatures,
        );
    }
    if let Ty::Shape(to_fields) = interner.get(to) {
        let to_fields = to_fields.clone();
        return shape_satisfied(from, &to_fields, interner, graph, signatures);
    }
    // **`array<T>` is covariant in its element type**, and it is the one
    // generic name in the language that is — see [`class_satisfied`] for why
    // `Iterator<T>` stays invariant beside it.
    //
    // The usual objection does not apply: covariant arrays are unsound in a
    // language where the target *aliases* the source, because a write through
    // the widened view lands in storage the narrow view still reads. `rule:types/arrays` makes an Novis array a copy-on-write **value** instead, so the widened
    // binding is a separate array the moment anything writes to it, and the
    // narrow one can never observe the write. What covariance buys is every
    // signature the spec writes over a union — `Core\Arr::sum`'s
    // `array<int|float|decimal>` takes an `array<int>`, which is what a caller
    // means by it — and `Core\Arr::flip`'s `array<T>` binding a `T` it could
    // not otherwise reach.
    //
    // The elements are compared with [`Widening::InPlace`]: an array that
    // already exists keeps the representation its elements were stored with,
    // so an `array<int>` is not an `array<float>` at any depth. `as
    // array<float>` is the conversion, and it builds a new array. An array
    // *literal* at `array<float>` is not compared here at all: its elements
    // are checked one by one against `float`, and lowering stores each one
    // converted (`super::literals::check_array_literal`).
    if let (Ty::Array(from_elem), Ty::Array(to_elem)) = (interner.get(from), interner.get(to)) {
        let (from_elem, to_elem) = (*from_elem, *to_elem);
        return assignable(
            from_elem,
            to_elem,
            Widening::InPlace,
            interner,
            graph,
            signatures,
        );
    }
    // `rule:types/class-reference-variance`: **`class<T>` is covariant in its argument, and only
    // upward** — `class<Dog>` widens to `class<Animal>` wherever `Dog` widens
    // to `Animal`, and a narrowing is written `as class<Dog>` and checked
    // against the descriptor at run time.
    //
    // The paragraph above earns `array<T>`'s covariance with copy-on-write.
    // Nothing of that argument is needed here, because the trap it answers
    // cannot be set: a class descriptor has no write side, so the argument is a
    // pure output position and there is nothing a widened view could store for
    // the narrow one to read back.
    if let (Ty::ClassRef(from_arg), Ty::ClassRef(to_arg)) = (interner.get(from), interner.get(to)) {
        let (from_arg, to_arg) = (*from_arg, *to_arg);
        return assignable(from_arg, to_arg, widen, interner, graph, signatures);
    }
    // **Bare `callable` is the top of `rule:types/callable-signature`'s
    // lattice**, so a written signature always satisfies it. The reverse is
    // refused by falling through: a value whose signature is unknown cannot
    // fill a position that promises one, and that refusal is the whole of what
    // writing the annotation buys.
    if matches!(interner.get(to), Ty::Callable)
        && matches!(interner.get(from), Ty::CallableSig { .. })
    {
        return true;
    }
    // `rule:concurrency/all-answers-a-typed-shape`'s one parameter, and the
    // whole of what it accepts: a shape, every field of which is a `callable`.
    // Width subtyping does not enter it — the argument's field names are the
    // answer's field names, so every field is read and none is surplus.
    //
    // Checked here rather than at the binding pass because it is a question
    // about the argument's *type*, which is what `rule:types/callable-signature`
    // put within reach: a field is refused for declaring no callable at all,
    // never for being written somewhere other than at the call.
    if let Ty::ShapeOfCallables(_) = interner.get(to) {
        let Ty::Shape(fields) = interner.get(from) else {
            return false;
        };
        return fields.iter().all(|field| {
            matches!(
                interner.get(field.ty),
                Ty::Callable | Ty::CallableSig { .. }
            )
        });
    }
    // `rule:types/callable-arity` and `rule:types/callable-variance` are one
    // comparison. Arity is a **prefix** match — `n ≤ m`, only the first `n`
    // parameters compared — which describes `nvs_runtime::closure`'s own
    // behaviour rather than overruling it, since a callee is already handed
    // just the arguments it declares. Parameters are then contravariant and the
    // return type covariant, each refusing the one unsound direction: the slot
    // may be handed any `User`, and its caller was promised a `string`.
    //
    // The variance here is free for a reason `rule:types/arrays`' covariance
    // has to argue for: the widening that costs an O(n) restamp is `as
    // array<U>`, and a callable conversion restamps nothing, copies nothing
    // and emits nothing at all.
    if let (
        Ty::CallableSig {
            params: from_params,
            ret: from_ret,
        },
        Ty::CallableSig {
            params: to_params,
            ret: to_ret,
        },
    ) = (interner.get(from), interner.get(to))
    {
        let (from_params, from_ret) = (from_params.clone(), *from_ret);
        let (to_params, to_ret) = (to_params.clone(), *to_ret);
        if from_params.len() > to_params.len() {
            return false;
        }
        let params_ok = from_params
            .iter()
            .zip(&to_params)
            .all(|(from_p, to_p)| assignable(*to_p, *from_p, widen, interner, graph, signatures));
        // `never` is the bottom of the return position and the one place the
        // relation meets it: `rule:types/grammar` makes the atom return-only,
        // and a body that never comes back satisfies whatever its caller was
        // promised because the caller never reads it.
        let ret_ok = matches!(interner.get(from_ret), Ty::Never)
            || assignable(from_ret, to_ret, widen, interner, graph, signatures);
        return params_ok && ret_ok;
    }
    // `rule:security/taint-propagation` / `rule:security/secret-propagation`: `tainted` and `secret` are two independent
    // bits on the same `string`/`bytes` base, and each may only ever widen
    // through ordinary assignment — a plain value is always a safe
    // over-approximation of "may be tainted"/"may be secret," but never the
    // reverse. `from` is assignable to a same-base `to` exactly when every
    // qualifier bit `from` carries, `to` carries too (a strict superset is
    // fine; a missing bit is this whole mechanism's point).
    if let (Some(from_is_bytes), Some(to_is_bytes)) = (
        qualifiable_base(from, interner),
        qualifiable_base(to, interner),
    ) {
        let tainted_ok = !is_tainted(from, interner) || is_tainted(to, interner);
        let secret_ok = !is_secret(from, interner) || is_secret(to, interner);
        if from_is_bytes == to_is_bytes && tainted_ok && secret_ok {
            return true;
        }
    }
    false
}

/// Whether a value of class `from_q` may be used where the class or
/// interface `to_q` (at `to_args`) is declared — Novis's one nominal subtyping
/// rule, and deliberately the whole of it.
///
/// `from_q` satisfies `to_q` when it reaches it through `extends`/
/// `implements`; the two [`QName`]s being equal is already handled by
/// [`is_assignable`]'s interning check, since a class type is interned
/// structurally. There is **no variance**: a generic target (`rule:iteration/concrete-generic-implements`'s
/// `Iterable<T>`/`Iterator<T>`, which are the only generic names user code
/// can write) additionally requires the arguments `from_q` fixed for it to
/// equal `to_args` exactly, so `Iterator<int>` never satisfies
/// `Iterator<mixed>`. Widening a cursor's element type is not obviously
/// sound in either direction — `current()` returns `T` while a future
/// `Sink<T>` would consume one — and nothing on `rule:iteration/two-interfaces`'s path needs it, so
/// the invariant rule is what is committed to here rather than a covariant
/// one that would be expensive to take back.
pub(crate) fn class_satisfied(
    from_q: &QName,
    from_args: &[TypeId],
    to_q: &QName,
    to_args: &[TypeId],
    interner: &mut TypeInterner,
    graph: &ClassGraph,
    signatures: &SignatureTable,
) -> bool {
    // **Two rosters answer "does this class implement that interface", and a
    // `Core` class is only in the second.** `nvs_hir`'s graph holds what a
    // *program* declared; the one thing a `Core` class says about a hierarchy
    // is seeded straight into the signature table (`crate::core_lib`'s `seed`,
    // off `nvs_stdlib::registry::ITERABLES`), so it reaches `implements` and
    // never the graph. Gating on the graph alone is what made
    // `Core\Arr::from($set)` an `E0401` for a set `foreach` walks.
    let fixed = crate::signatures::resolve_interface_args(from_q, to_q, signatures, graph);
    if !nvs_hir::hierarchy::implements_interface(from_q, to_q, graph) && fixed.is_none() {
        return false;
    }
    if to_args.is_empty() {
        return true;
    }
    // Invariant in the argument, as ever — but the arguments compared are the
    // ones the *receiver* fixed, since what a class wrote for an interface is
    // written in its own type variables. `crate::generics::with_class_args`
    // owns that step and its two other callers.
    fixed.is_some_and(|fixed| {
        fixed.len() == to_args.len()
            && fixed.into_iter().zip(to_args).all(|(arg, want)| {
                crate::generics::with_class_args(from_q, from_args, arg, interner) == *want
            })
    })
}

/// `rule:types/shape-type`'s structural check for a shape target: `from` must have at
/// least every field `to_fields` names, each satisfying the field's declared
/// type by [`field_fits`] — [`is_assignable`] without the `int` → `float`
/// row, because `from` is shared and its fields are not converted (width
/// subtyping — an extra field on `from` is never a problem). A class receiver's field types come
/// from [`resolve_property`], the same ancestor walk an ordinary `$obj->prop`
/// access already uses; any other `from` (a scalar, `object`, a mismatched
/// shape) never satisfies a shape target.
///
/// A field the target marks optional ([`ShapeField::required`] false) relaxes
/// exactly one half of that: the key may be **absent** from `from`. When it is
/// present it is checked like any other, so `{a?: int}` still refuses a source
/// whose `a` is a `string`. The relaxation does not run the other way — a
/// source whose own `a` is optional is not proven to carry one, so it fills a
/// required `a` no better than a source with no `a` at all, and only another
/// optional field accepts it.
pub(crate) fn shape_satisfied(
    from: TypeId,
    to_fields: &[ShapeField],
    interner: &mut TypeInterner,
    graph: &ClassGraph,
    signatures: &SignatureTable,
) -> bool {
    // Cloned rather than borrowed: [`is_assignable`] needs the interner
    // mutably for `rule:types/literal-types`'s widening step, and a shape's field list is
    // read while it recurses. A shape type is a handful of fields and this
    // path runs once per shape-typed assignment.
    match interner.get(from).clone() {
        Ty::Shape(from_fields) => to_fields.iter().all(|to_field| {
            match from_fields.iter().find(|from| from.name == to_field.name) {
                Some(from) => {
                    (from.required || !to_field.required)
                        && field_fits(from.ty, to_field.ty, interner, graph, signatures)
                }
                None => !to_field.required,
            }
        }),
        // A declared property is always present, so nothing here asks about
        // `from.required`: a class satisfies an optional field by carrying it
        // at an assignable type, and by not carrying it at all.
        Ty::Class(qname, _) => to_fields.iter().all(|to_field| {
            match resolve_property(&qname, &to_field.name, signatures, graph) {
                Some(from_field_ty) => {
                    field_fits(from_field_ty, to_field.ty, interner, graph, signatures)
                }
                None => !to_field.required,
            }
        }),
        _ => false,
    }
}

/// Whether a field of type `from` that already exists satisfies a field
/// declared `to`: [`is_assignable`] without the `int` → `float` row, at any
/// depth of `to` — see [`Widening`].
pub(crate) fn field_fits(
    from: TypeId,
    to: TypeId,
    interner: &mut TypeInterner,
    graph: &ClassGraph,
    signatures: &SignatureTable,
) -> bool {
    assignable(from, to, Widening::InPlace, interner, graph, signatures)
}

pub(crate) fn report_mismatch(span: Span, expected: TypeId, actual: TypeId, env: &mut Env<'_>) {
    let diag = mismatch(span, expected, actual, env);
    env.diags.report(diag);
}

/// [`report_mismatch`]'s diagnostic, built and not yet reported — for the one
/// caller that has something to add to it.
///
/// The wording is the same either way on purpose: a mismatch that reads
/// differently depending on which pass noticed it is two diagnostics for one
/// mistake. What a caller adds is a `help:`, which is the part of a diagnostic
/// that is allowed to know more than the types do —
/// `crate::expr::args::check_shape_field` knows the refused key is a sink and
/// can therefore name the way through.
///
/// A shape target adds its own help here rather than at a caller, for that same
/// reason and one more: [`missing_required_keys`] answers the question the
/// reader actually has — which key is absent — and *every* position that
/// assigns into a shape asks it, so a caller that had to opt in would leave a
/// `return`, an attribute payload and a `foreach` binding printing both shapes
/// whole for the reader to diff. It stays [`code::E_TYPE_MISMATCH`] because it
/// is the same mistake, a value that does not fit its declared type; a code of
/// its own would fire on this one arm and say nothing the help line does not.
pub(crate) fn mismatch(
    span: Span,
    expected: TypeId,
    actual: TypeId,
    env: &mut Env<'_>,
) -> Diagnostic {
    let expected_desc = env.interner.describe(expected);
    let actual_desc = env.interner.describe(actual);
    let diag = Diagnostic::error(
        code::E_TYPE_MISMATCH,
        format!("expected `{expected_desc}`, found `{actual_desc}`"),
    )
    .with_primary(span, format!("this is `{actual_desc}`"));
    match missing_required_keys(expected, actual, env).as_slice() {
        [] => match unconverted_field(expected, actual, env) {
            Some((name, from, to)) => {
                let (from, to) = (env.interner.describe(from), env.interner.describe(to));
                diag.with_help(format!(
                    "the field `{name}` is `{from}` in this value, and a value that already \
                     exists keeps its field types, so it is not converted to `{to}`. Build a new \
                     value instead: `{{{name}: $value->{name}}}`"
                ))
            }
            None => match unconverted_array_help(expected, actual, env) {
                Some(help) => diag.with_help(help),
                None => diag,
            },
        },
        [one] => diag.with_help(format!(
            "`{one}` is required here, and this value does not supply it"
        )),
        several => {
            let names: Vec<String> = several.iter().map(|name| format!("`{name}`")).collect();
            diag.with_help(format!(
                "these keys are required here, and this value supplies none of them: {}",
                names.join(", ")
            ))
        }
    }
}

/// The keys `expected`'s shape requires that `actual` does not supply, in the
/// shape's own declaration order. Empty whenever `expected` is not a shape, and
/// empty when the source carries every required key — a mismatch on a field's
/// *type* has both shapes printed and no key to name.
///
/// "Does not supply" is [`shape_satisfied`]'s test rather than a weaker one, so
/// the two cannot disagree about which key is at fault: a source field that is
/// itself optional is not proven to carry a value, fills a required key no
/// better than an absent one, and is named here. That asymmetry, and the one
/// that lets a class answer presence alone, are [`shape_satisfied`]'s to own.
fn missing_required_keys(expected: TypeId, actual: TypeId, env: &mut Env<'_>) -> Vec<String> {
    let (signatures, graph) = (env.signatures, env.graph);
    let Ty::Shape(to_fields) = env.interner.get(expected).clone() else {
        return Vec::new();
    };
    let required = to_fields.into_iter().filter(|field| field.required);
    match env.interner.get(actual).clone() {
        Ty::Shape(from_fields) => required
            .filter(|to| {
                !from_fields
                    .iter()
                    .any(|from| from.name == to.name && from.required)
            })
            .map(|to| to.name)
            .collect(),
        Ty::Class(qname, _) => required
            .filter(|to| resolve_property(&qname, &to.name, signatures, graph).is_none())
            .map(|to| to.name)
            .collect(),
        _ => Vec::new(),
    }
}

/// The first field `expected`'s shape declares that `actual` fails only
/// because [`field_fits`] leaves out the `int` → `float` row: an `int` or
/// `uint` field that [`is_assignable`] accepts and [`field_fits`] does not.
/// Building a new value converts that field. Gives the field's
/// name, its type in `actual` and its declared type. `None` when `expected` is
/// not a shape or no field fails that way.
fn unconverted_field(
    expected: TypeId,
    actual: TypeId,
    env: &mut Env<'_>,
) -> Option<(String, TypeId, TypeId)> {
    let (signatures, graph) = (env.signatures, env.graph);
    let Ty::Shape(to_fields) = env.interner.get(expected).clone() else {
        return None;
    };
    let from_fields: Vec<(String, TypeId)> = match env.interner.get(actual).clone() {
        Ty::Shape(from_fields) => from_fields
            .into_iter()
            .map(|field| (field.name, field.ty))
            .collect(),
        Ty::Class(qname, _) => to_fields
            .iter()
            .filter_map(|to| {
                resolve_property(&qname, &to.name, signatures, graph)
                    .map(|ty| (to.name.clone(), ty))
            })
            .collect(),
        _ => return None,
    };
    to_fields.into_iter().find_map(|to| {
        let (_, from_ty) = from_fields.iter().find(|(name, _)| *name == to.name)?;
        let scalar = matches!(env.interner.get(*from_ty), Ty::Int | Ty::Uint);
        let converts = scalar && is_assignable(*from_ty, to.ty, env.interner, graph, signatures);
        let fits = field_fits(*from_ty, to.ty, env.interner, graph, signatures);
        (converts && !fits).then_some((to.name, *from_ty, to.ty))
    })
}

/// The help line for an `array<int>` or `array<uint>` that fails an
/// `array<float>` target only because [`field_fits`] leaves out the `int` →
/// `float` row for its elements, at any depth of nesting and under a `?`.
/// `as` is the conversion, so the help names it with the target type. `None`
/// for every other mismatch.
fn unconverted_array_help(expected: TypeId, actual: TypeId, env: &mut Env<'_>) -> Option<String> {
    let (signatures, graph) = (env.signatures, env.graph);
    let target = env.interner.without_null(expected);
    let (mut from, mut to) = (actual, target);
    let mut nested = false;
    while let (Ty::Array(from_elem), Ty::Array(to_elem)) =
        (env.interner.get(from), env.interner.get(to))
    {
        (from, to) = (*from_elem, *to_elem);
        nested = true;
    }
    let scalar = matches!(env.interner.get(from), Ty::Int | Ty::Uint);
    let converts = scalar && is_assignable(from, to, env.interner, graph, signatures);
    if !nested || !converts || field_fits(from, to, env.interner, graph, signatures) {
        return None;
    }
    let (actual, target) = (env.interner.describe(actual), env.interner.describe(target));
    Some(format!(
        "an `{actual}` that already exists keeps the type of its elements, so it is not \
         converted to `{target}`. Convert it with `as`, which builds a new array: \
         `$value as {target}`"
    ))
}

/// Checks a `return expr;`'s value against the method's declared return
/// type, reporting `E_BAD_RETURN_TYPE` — distinct wording from the generic
/// `E_TYPE_MISMATCH` [`check_expr`] itself reports, for what is structurally
/// the same assignability question.
pub(crate) fn check_return(
    expr: &Expr,
    return_ty: TypeId,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    // `rule:iteration/one-way-only`: a generator is a lazy sequence and nothing more, so a
    // bare `return;` (stop here) is the only form its body may write. Its own
    // diagnostic rather than the mismatch below, which would report the
    // `void` `crate::check::check_method` checks a generator body against and
    // never mention why.
    if ctx.generator_elem.is_some() {
        infer(expr, None, live, scope, ctx, env);
        env.diags.report(
            Diagnostic::error(
                code::E_GENERATOR_RETURNS_A_VALUE,
                "a generator cannot return a value",
            )
            .with_primary(expr.span, "this value has nowhere to go")
            .with_help(
                "`rule:iteration/one-way-only`: there is no generator return value to retrieve — write \
                 `return;` to stop the sequence, or `yield` this value",
            ),
        );
        return;
    }
    // A value that already reported an error inside it is one mistake, not
    // two: its type is a stand-in, often with `mixed` in it, and a second
    // `E_BAD_RETURN_TYPE` about that stand-in names no mistake of its own.
    let errors_before = env.diags.error_count();
    let actual = infer(expr, Some(return_ty), live, scope, ctx, env);
    if env.diags.error_count() != errors_before {
        return;
    }
    if !is_assignable(actual, return_ty, env.interner, env.graph, env.signatures) {
        let expected_desc = env.interner.describe(return_ty);
        let actual_desc = env.interner.describe(actual);
        let diag = Diagnostic::error(
            code::E_BAD_RETURN_TYPE,
            format!("this method declares `{expected_desc}` but returns `{actual_desc}`"),
        )
        .with_primary(expr.span, format!("this is `{actual_desc}`"));
        let diag = match unconverted_array_help(return_ty, actual, env) {
            Some(help) => diag.with_help(help),
            None => diag,
        };
        env.diags.report(diag);
    } else {
        note_float_widening(expr, actual, return_ty, env);
    }
}

/// `E0701` — `$a = &$b;`, refused rather than lowered. The diagnostic is
/// [`nvs_syntax::by_reference_assignment`], which the parser reports for the
/// declaration spelling, `int $a = &$b;`.
pub(crate) fn report_by_reference_assignment(span: Span, value: Span, env: &mut Env<'_>) {
    let value_text = span_text(env.src, value).to_owned();
    env.diags
        .report(nvs_syntax::by_reference_assignment(span, &value_text));
}

#[expect(
    clippy::too_many_arguments,
    reason = "the same context [`check_compound_assign`] states, with the \n              assignment operator in place of the binary one it maps to"
)]
pub(crate) fn check_assign(
    op: AssignOp,
    span: Span,
    target: &Expr,
    value: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    if let Some(binop) = op.binary_op() {
        return check_compound_assign(binop, span, target, value, live, scope, ctx, env);
    }
    if let (AssignOp::Assign, ExprKind::Variable(span)) = (op, &target.kind) {
        let name = strip_sigil(span_text(env.src, *span)).to_owned();
        // `overwrite`, not `declared_ty`: a write is checked against what the
        // binding was *declared* as, and drops whatever a `!== null` test
        // narrowed it to — see `crate::locals`' narrowing docs.
        let declared = scope.overwrite(&name);
        let value_ty = check_expr(value, declared, live, scope, ctx, env);
        match declared {
            Some(ty) => {
                live.insert(name);
                ty
            }
            None => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_UNDEFINED_VARIABLE,
                        format!("`${name}` is assigned to but was never declared"),
                    )
                    .with_primary(target.span, "no `LocalDecl` for this name")
                    .with_help("declare it first: `T $name = ...;`"),
                );
                value_ty
            }
        }
    } else {
        mark_write_target_levels(target, true, env);
        let target_ty = check_expr(target, None, live, scope, ctx, env);
        check_write_target(target, ctx, env);
        let before = env.diags.len();
        let value_ty = check_expr(value, Some(target_ty), live, scope, ctx, env);
        // `rule:security/secret-qualifier`'s container axis at `$a["k"] =
        // $secret`, guarded on the diagnostic count for the reason
        // [`super::literals::check_array_literal`]'s element is: a value that
        // already failed against the element type is one mistake.
        if env.diags.len() == before {
            reject_secret_element_write(target, target_ty, value, value_ty, env);
        }
        target_ty
    }
}

/// Records every `$a[]` level of a plain `=`'s target as a legal append, so
/// that [`super::check_expr`]'s `ExprKind::Index` arm can report `E0481` for
/// every *other* `index: None` it meets.
///
/// PHP's `[]` names the key one past the highest integer key, which is an
/// answer only where a value is being put there: PHP refuses `echo $a[];`
/// with *"Cannot use [] for reading"* and `unset($a[])` with *"for
/// unsetting"*, and Novis refuses `$a[] .= "x"` alongside them, which is ADR
/// 0007 § 7 row 10 — PHP appends there only because the element that is not
/// there yet reads as `""`. Marking the legal spans is therefore the whole
/// rule, and this walk is where they all are:
/// the target chain of a plain assignment, every level of it, since
/// `$a[][0] = 1` appends a fresh row and writes into it (`rule:types/arrays`'s
/// separation applies at each level, and `nvs_ir::lower::stmt`'s flatten
/// walks the same chain).
///
/// Called **before** the target is checked, unlike [`check_write_target`],
/// because the arm it speaks to is inside that check. A subscript's own
/// *index* expression is not walked — `$a[$b[]] = 1` reads `$b[]`, and is
/// refused for it.
///
/// The marks are read a second time, by `super`'s own
/// `refused_as_a_write_target`, which is why all four write
/// spellings call this and not just the one that has a `$a[]` to legalise:
/// `plain` is `false` for a compound assignment, an increment and an
/// `unset()`, none of which may append, and the *presence* of the mark is
/// what tells a subscript that the refusal its holder is about to take is
/// the one mistake to report.
pub(crate) fn mark_write_target_levels(target: &Expr, plain: bool, env: &mut Env<'_>) {
    let mut level = target.unparenthesized();
    while let ExprKind::Index { base, .. } = &level.kind {
        env.write_target_levels.insert(level.span, plain);
        level = base.unparenthesized();
    }
}

/// The four assignment targets that have nowhere to write to, refused where
/// they are written rather than lowered into something that quietly drops the
/// write. The first two match PHP, which refuses the same two spellings.
///
/// A **nullsafe** target (`$a?->b = v`) is refused for the operator's own
/// reason: `?->` yields `null` where the receiver is `null`, and `null` is not
/// a place. PHP says "can't use nullsafe operator in write context"; the
/// alternative is an assignment that silently does nothing on one path.
///
/// An **element write through an `rule:classes/property-hooks` hooked property**
/// (`$obj->hooked[0] = v`) is refused because a hooked property is a pair of
/// accessors and not a slot. `rule:types/arrays` separates the array the `get` hook
/// answered with, and no rule pushes the separated copy back through `set` —
/// PHP raises "indirect modification of overloaded property" and discards the
/// write, so refusing *is* the PHP-compatible answer rather than a divergence.
/// Read the array into a local, write the element, assign it back.
///
/// An **element write through an erased property** — an `rule:types/object-top` shape's
/// field, or any property of `rule:types/grammar`'s plain `object` — is refused
/// because `rule:types/erased-member-access` resolves one by *name* at run time and stopped there:
/// a read needs only the name, a write needs a slot for the separated array to
/// land in. This is the write half of the question `E0477` answers for a
/// method call, and the answer is the same one — narrow the receiver. It is
/// the one of the three PHP would have allowed (on a `stdClass`), and it is
/// refused for the language's own reason rather than PHP's: the property reads
/// as `mixed`, and `mixed` is not indexable anywhere else either.
///
/// A root that is not a **place** at all — `$h->rows()["a"] = "y"`,
/// `[1, 2]["0"] = "z"` — is the fourth and the widest, `E0700`. It is the same
/// missing slot the two above are about, arrived at from the other side: those
/// two name a holder that turns out not to be storage, this one names no
/// holder in the first place. [`is_a_place`] is the test, and it is exactly
/// the set of roots `nvs_ir::lower::Lowering::write_back_array` can re-point.
/// This is the one of the four PHP does *not* refuse — 8.5 lowers the write
/// into the temporary and discards it, silently — so it is a deliberate
/// divergence, `rule:php-migration/every-divergence-is-deliberate-and-listed` row 15, taken because the only statement it costs
/// is one that could never have done anything.
///
/// Called *after* the target is checked, because the hooked half reads the
/// [`ExprInfo::HookedProperty`] entry [`super::members`] records while
/// checking the access. All four spellings that write through a target go
/// through it — a plain `=`, a compound `⊕=`, `$x++`/`--$x`, which
/// `nvs_ir::lower` desugars into the same `$x = $x ± 1` a compound assignment
/// becomes and which therefore has exactly the same nowhere to write to, and
/// `unset($a[$k])`, which `rule:types/arrays` separates the array for exactly as a
/// write does (`super::members`' `check_unset_target`). All four give the
/// same answer on the same target and each takes exactly one diagnostic for
/// it, which
/// `tests/conformance/lang/every-write-spelling-agrees-on-a-refused-element-target.nvst`
/// asks of all four at once — a spelling that grew its own answer, or a
/// second diagnostic for the subscript the refused holder made unreadable,
/// fails there while still reading right on its own line.
/// Only the root of a subscript chain is examined:
/// `nvs_ir::lower::Lowering::lower_reassignment` flattens a nested element
/// write down to its root holder and writes every level back through that, so
/// the root is the only level with a holder at all — which is also why
/// `$obj->hooked[0][1] = v` is this same refusal and not a deeper one.
pub(crate) fn check_write_target(target: &Expr, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let mut root = target.unparenthesized();
    let mut through_subscript = false;
    while let ExprKind::Index { base, .. } = &root.kind {
        root = base.unparenthesized();
        through_subscript = true;
    }
    if reject_readonly_write(root, ctx, env) {
        return;
    }
    if matches!(root.kind, ExprKind::PropertyAccess { nullsafe: true, .. }) {
        env.diags.report(
            Diagnostic::error(
                code::E_NULLSAFE_WRITE_TARGET,
                "`?->` cannot be written through",
            )
            .with_primary(root.span, "this yields `null` when the receiver is `null`")
            .with_help(
                "`null` is not a place to assign to — test the receiver instead: \
                 `if ($x !== null) { $x->p = …; }`",
            ),
        );
        return;
    }
    if !through_subscript {
        reject_get_only_hook_write(root, ctx, env);
        return;
    }
    if !is_a_place(&root.kind) {
        env.diags.report(
            Diagnostic::error(
                code::E_ELEMENT_WRITE_ROOT_NOT_A_PLACE,
                "an array element cannot be written through a temporary",
            )
            .with_primary(root.span, "this value is not stored anywhere")
            .with_help(
                "`rule:types/arrays` separates the array before the element is written, and the \
                 separated copy has to go back into whatever held it — a temporary holds it \
                 nowhere, so the write would be discarded. Bind it first, write the element \
                 through the binding, and assign that back if it has an owner",
            ),
        );
        return;
    }
    match env.exprs.lookup(root.span) {
        Some(ExprInfo::HookedProperty { name, .. }) => {
            let name = name.clone();
            env.diags.report(
                Diagnostic::error(
                    code::E_ELEMENT_WRITE_THROUGH_HOOK,
                    format!(
                        "an array element cannot be written through the hooked property `{name}`"
                    ),
                )
                .with_primary(root.span, "reading this runs its `get` hook")
                .with_help(
                    "`rule:classes/property-hooks` makes a hooked property a pair of accessors, not a slot, so the \
                     separated array would have nowhere to go — read it into a local, write the \
                     element there, and assign the local back through the property",
                ),
            );
        }
        Some(ExprInfo::ShapeProperty { name, .. }) => {
            let name = name.clone();
            env.diags.report(
                Diagnostic::error(
                    code::E_ELEMENT_WRITE_THROUGH_ERASED_PROPERTY,
                    format!(
                        "an array element cannot be written through the erased property `{name}`"
                    ),
                )
                .with_primary(root.span, "this receiver is a shape or a plain `object`")
                .with_help(
                    "`rule:types/erased-member-access` resolves such a property by *name* at run time, which gives the \
                     separated array no slot to be written back into — convert the receiver to \
                     the class that declares it first (`var $c = $x as ClassName;`), or read the \
                     property into a typed local, write the element there, and assign it back",
                ),
            );
        }
        _ => {}
    }
}

/// `rule:classes/lateinit-restrictions`'s contract for `readonly`, at the one place it can be broken:
/// a write to such a property from anywhere but the declaring class's own
/// `constructor` is `E0782`. Answers whether it reported, so its caller stops
/// rather than adding a second diagnostic about the same target.
///
/// The **root** of the write is what is examined, so an element write
/// (`$w->tags[0] = "x"`) is refused alongside the plain one: `rule:types/arrays`
/// separates the array and `nvs_ir::lower` writes the separated copy back
/// through the property, which is a write to the property whatever the
/// spelling suggests. All four write spellings reach this through
/// [`check_write_target`], so `$w->id++` and `unset($w->id)` answer here too.
///
/// Two conditions, not one: the write must be inside the constructor *and*
/// inside the class that declared the property. A subclass constructor writing
/// an inherited `readonly` property is refused for the same reason PHP refuses
/// it — the declaring class's own constructor is the one that promised the
/// value, and a second writer is a second chance to write.
///
/// A hooked property is not reachable here: it records
/// [`ExprInfo::HookedProperty`] instead, and a hook's `set` accessor is the
/// write. An erased receiver records [`ExprInfo::ShapeProperty`] and names no
/// declaring class, so nothing can be asked of it — the same gap every other
/// rule stated over a class has there. A *keyed* access
/// ([`ExprInfo::KeyedProperty`]) does name a class, so it can be asked, and
/// [`reject_readonly_write_through_key`] asks it ahead of this.
fn reject_readonly_write(root: &Expr, ctx: &Ctx<'_>, env: &mut Env<'_>) -> bool {
    if reject_readonly_write_through_key(root, env) {
        return true;
    }
    let Some(ExprInfo::Property { class, name, .. }) = env.exprs.lookup(root.span) else {
        return false;
    };
    let (class, name) = (class.clone(), name.clone());
    let Some((owner, _)) =
        crate::signatures::resolve_property_owned(&class, &name, env.signatures, env.graph)
    else {
        return false;
    };
    if !crate::signatures::property_is_readonly(&owner, &name, env.signatures) {
        return false;
    }
    if ctx.in_constructor && ctx.current_class == Some(&owner) {
        return false;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_READONLY_WRITE_AFTER_CONSTRUCTION,
            format!("`{owner}::${name}` is `readonly`, so only `{owner}`'s constructor writes it"),
        )
        .with_primary(root.span, "this write happens after construction")
        .with_help(
            "`readonly` promises the value is assigned exactly once, while the object is being \
             built (`rule:classes/lateinit-restrictions`) — assign it in the constructor, take it as a constructor \
             parameter (`public readonly T $x`), or drop the modifier if the property is meant \
             to change",
        ),
    );
    true
}

/// `rule:types/property-key-access`'s last paragraph: a write *through a property key* is refused
/// where `T`'s public set holds a `readonly` property, naming it. The code and
/// the headline are [`reject_readonly_write`]'s, because it is the same rule of
/// `rule:classes/lateinit-restrictions` being broken — only the question is asked one step less
/// specifically.
///
/// The question is asked of the **set** rather than of one resolved property
/// because *which* name the key holds is exactly what the access does not know
/// (`rule:types/property-key-access`). One `readonly` member is therefore enough to refuse: the
/// write might name it. § 5 also decides that this is compile-time rather than
/// a `readonly` bit on `ClassDesc` and a throw the program has to reach —
/// where a request-controlled name selects the field to write, the earlier
/// report is the direction priority 1 points in.
///
/// It sits here, off the recorded [`ExprInfo::KeyedProperty`], rather than in
/// the [`super::members`] checker that records that entry, because this is the
/// one place the access is known to be a **write**: inference sees the same
/// `$obj->$key` either way, and all four write spellings reach this through
/// [`check_write_target`], so `$obj->$key++` and `$obj->$key[0] = v` answer
/// here for free. `unset($obj->$key)` cannot be diagnosed twice either — it is
/// refused where it is written and records no entry at all.
///
/// There is deliberately no constructor exemption, unlike the declared write
/// above. That exemption is stated over the one property the constructor
/// promised, and a key names no one property: a keyed write inside `T`'s own
/// constructor could write the `readonly` member a second time, which is the
/// promise itself rather than the place it is kept. The fix the diagnostic
/// prints — write the property out — is the spelling such a constructor wanted.
fn reject_readonly_write_through_key(root: &Expr, env: &mut Env<'_>) -> bool {
    let Some(ExprInfo::KeyedProperty { class, .. }) = env.exprs.lookup(root.span) else {
        return false;
    };
    let key_class = QName::parse(class);
    let refused = public_property_names(&key_class, env)
        .into_iter()
        .find_map(|name| {
            let (owner, _) = crate::signatures::resolve_property_owned(
                &key_class,
                &name,
                env.signatures,
                env.graph,
            )?;
            crate::signatures::property_is_readonly(&owner, &name, env.signatures)
                .then_some((owner, name))
        });
    let Some((owner, name)) = refused else {
        return false;
    };
    env.diags.report(
        Diagnostic::error(
            code::E_READONLY_WRITE_AFTER_CONSTRUCTION,
            format!("`{owner}::${name}` is `readonly`, so only `{owner}`'s constructor writes it"),
        )
        .with_primary(
            root.span,
            format!(
                "a `property<{key_class}>` may name any of {key_class}'s public properties, and \
                 this write cannot know which one it holds"
            ),
        )
        .with_help(
            "write the property out to name a different one, or drop the modifier if the property \
             is meant to be assignable",
        ),
    );
    true
}

/// Refuses a write to a property that declares a `get` hook and no `set`
/// hook, from outside the class that declares it
/// (`E_GET_ONLY_HOOK_WRITE`).
///
/// The hook half of the question is the [`ExprInfo::HookedProperty`] entry
/// [`super::members`] already recorded: its `set` label is `None` exactly
/// when no `set` hook has a body, and a hook block is the only thing that
/// records the entry at all. Two exemptions come free with reading that
/// rather than the signature table. A write **inside that property's own
/// hooks** is the backing slot and records a plain [`ExprInfo::Property`]
/// instead (that variant's own docs own why), so a `set` hook storing
/// through `$this->p` is untouched. And a property with only a `set` hook is
/// a write with somewhere to go, so `get.is_none()` passes it through.
///
/// **The declaring class writes it; the world outside reads it.** Novis
/// keeps a slot for every hooked property, backed or not
/// ([`crate::signatures::PropertyHooks`]' docs own why), so `$this->p = v`
/// inside the declaring class stores into that slot and a `get` hook reading
/// `$this->p` sees it — which is how a `get`-only property is armed at all,
/// and refusing it would leave the shape with no way to hold a value. From
/// outside, the accessors *are* the property: a class that declared only a
/// `get` said what it offers, and a write that reached past it into storage
/// the `get` may never read is a value lost in silence. Inheritance follows
/// the same reach `protected` does, since it is the same question about the
/// same declaration.
///
/// **This refuses one shape PHP 8.4 accepts.** PHP splits hooked properties
/// into backed and virtual and lets an outside write through to a *backed*
/// one's slot; Novis has no virtual property to tell it from, so the
/// scope-shaped rule above answers instead —
/// `docs/reference/tools/30-php-differences.md` carries the row, and the fix
/// in either language is a `set` hook naming what the write commits.
fn reject_get_only_hook_write(root: &Expr, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let Some(ExprInfo::HookedProperty {
        class,
        name,
        get,
        set,
        ..
    }) = env.exprs.lookup(root.span)
    else {
        return;
    };
    if set.is_some() || get.is_none() {
        return;
    }
    let (class, name) = (class.clone(), name.clone());
    let Some((owner, _)) =
        crate::signatures::resolve_property_owned(&class, &name, env.signatures, env.graph)
    else {
        return;
    };
    let inside = ctx.current_class.is_some_and(|current| {
        *current == owner || nvs_hir::implements_interface(current, &owner, env.graph)
    });
    if inside {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_GET_ONLY_HOOK_WRITE,
            format!(
                "`{owner}::${name}` declares a `get` hook and no `set` hook, so only `{owner}` \
                 writes it"
            ),
        )
        .with_primary(root.span, "there is no `set` hook for this write to run")
        .with_help(format!(
            "give `${name}` a `set` hook that commits the value — a write from out here has no \
             accessor behind it and would store into a slot the `get` hook may never read, \
             which is a value lost in silence rather than a value stored. Drop the hook block \
             if `${name}` is plain storage after all"
        )),
    );
}

/// Whether `kind` is a **place**: storage a separated array can be written
/// back into, rather than a value dropped at the end of the statement.
///
/// The three that are, are the three
/// `nvs_ir::lower::Lowering::write_back_array` can re-point — a local (an `inout $x`
/// parameter's slot included), a property of a receiver whose class is known
/// at compile time, and a static property, whose slot the request owns. A
/// property of a *temporary* receiver (`(new H)->rows["a"] = "y"`) is a place
/// too and deliberately so: the field belongs to a heap object with reference
/// semantics, so the write lands in real storage whatever happens to the
/// handle afterwards. That is the one place this rule is *wider* than PHP's —
/// 8.5.9 refuses a `new` in a write context outright (*"Cannot use temporary
/// expression in write context"*), while accepting the same write through a
/// call-returning receiver (`make()->rows["a"] = "y"`) — and `rule:php-migration/every-divergence-is-deliberate-and-listed` row
/// 15 records it: accepting where PHP refuses loses no program that ran, and
/// the storage the separated array goes back into is real either way.
///
/// `ExprKind::Error` is here so a parse error takes one diagnostic rather than
/// two. Parentheses never reach it, [`Expr::unparenthesized`] having peeled
/// them at every walk that calls this.
pub(crate) fn is_a_place(kind: &ExprKind) -> bool {
    matches!(
        kind,
        ExprKind::Variable(_)
            | ExprKind::PropertyAccess { .. }
            | ExprKind::StaticPropertyAccess { .. }
            | ExprKind::Error(_)
    )
}

/// `$x ⊕= e`, typed as the `$x = $x ⊕ e` it means — [`AssignOp::binary_op`]
/// is the one place that pairing is written down, and `nvs_ir::lower`
/// desugars through the same answer.
///
/// The target is read first, then the value is checked *against the target's
/// own type*, so the bare `1` in `uint $u = 0; $u += 1;` takes `uint` from
/// the position rather than defaulting to `int` and colliding with it (ADR
/// 0007 § 4's literal rule). The operator's result must then be assignable
/// back to the target: `int $i = 0; $i .= "x";` is a mismatch reported at the
/// assignment, never a silent re-typing of `$i` — `rule:types/var-inference` fixes a local's
/// type at its declaration. `.=` demands a `Stringable` operand exactly the
/// way the plain `.` does.
#[expect(
    clippy::too_many_arguments,
    reason = "the five-parameter checking context every expression walker in \n              this module carries, plus the operator, the assignment's span and \n              its two operand expressions"
)]
pub(crate) fn check_compound_assign(
    op: BinaryOp,
    span: Span,
    target: &Expr,
    value: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    note_write(target, scope, env);
    mark_write_target_levels(target, false, env);
    let target_ty = check_expr(target, None, live, scope, ctx, env);
    check_write_target(target, ctx, env);
    // [`infer`] rather than [`check_expr`]: the target's type is a *hint* for
    // an untyped literal here, not a position the value has to satisfy — the
    // operator decides that, and it is the operator's result this function
    // checks below. Handing the value to `check_expr` instead would report
    // `$i .= "x"` twice, once for a `string` where the `int` target sits and
    // once for the concatenation that is the actual mistake.
    let value_ty = infer(value, Some(target_ty), live, scope, ctx, env);
    if op == BinaryOp::Concat {
        require_stringable(target_ty, target.span, env);
        require_stringable(value_ty, value.span, env);
        // `rule:core-classes/html-escape-answers-markup`, for `.`'s compound spelling — the same row is missing
        // on both sides of it.
        reject_carrier_as_text(target_ty, target.span, env);
        reject_carrier_as_text(value_ty, value.span, env);
    }
    let result = binary_result(op, target_ty, value_ty, span, env);
    if !is_assignable(result, target_ty, env.interner, env.graph, env.signatures) {
        report_mismatch(span, target_ty, result, env);
        return target_ty;
    }
    note_float_widening_at(span, result, target_ty, env);
    // The operator's *result* is what lands in the element, and `.=` over a
    // `secret` operand produces a `secret` one (`rule:security/secret-propagation`),
    // so this spelling of the write owes the same refusal the plain one does.
    reject_secret_element_write(target, target_ty, value, result, env);
    target_ty
}

pub(crate) fn check_read(
    name: &str,
    span: Span,
    live: &FxHashSet<String>,
    scope: &LocalScope,
    env: &mut Env<'_>,
) -> TypeId {
    match scope.declared_ty(name) {
        Some(ty) if live.contains(name) => ty,
        Some(_) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNDEFINED_VARIABLE,
                    format!("`${name}` is read before any assignment reaches it"),
                )
                .with_primary(span, "not definitely assigned here"),
            );
            env.interner.mixed()
        }
        // `$this` is never declared by anything a program writes, so the
        // undeclared-name wording below would send the reader looking for a
        // declaration to add. `crate::check` seeds it for a non-`static`
        // method and for a property hook; arriving here means the body has no
        // receiver, which `rule:statements/static-is-a-member-modifier` makes an ordinary consequence of
        // `static` rather than a mistake in the name.
        None if name == "this" => {
            env.diags.report(
                Diagnostic::error(
                    code::E_THIS_WITHOUT_A_RECEIVER,
                    "`$this` names the receiver of an instance method, and this body has none",
                )
                .with_primary(span, "no `$this` in scope here")
                .with_help(
                    "a `static` method is entered through the class and not through a value: \
                     drop `static`, or take what it needs as a parameter",
                ),
            );
            env.interner.mixed()
        }
        None => {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNDEFINED_VARIABLE,
                    format!("`${name}` is not declared"),
                )
                .with_primary(span, "no declaration for this name"),
            );
            env.interner.mixed()
        }
    }
}

/// Drops whatever a dominating `!== null` test proved about `expr`, when
/// `expr` is a plain local — the call every write path in this module owes,
/// listed in `crate::locals`' narrowing docs. A write through a property or
/// an element cannot change what a *local* holds, so it has nothing to drop.
pub(crate) fn note_write(expr: &Expr, scope: &LocalScope, env: &Env<'_>) {
    if let ExprKind::Variable(span) = &expr.kind {
        scope.overwrite(strip_sigil(span_text(env.src, *span)));
    }
}

/// The callable lattice alone, asked of [`is_assignable`] directly — the rest
/// of the relation is exercised through compiled programs under
/// `crates/nvs-types/tests/`, but a signature is not yet a type any expression
/// *has* (`rule:types/callable-literal-inference` is what gives a `fn` literal
/// one), so these rows have no source spelling to reach them by.
#[cfg(test)]
mod tests {
    use super::is_assignable;
    use crate::signatures::SignatureTable;
    use crate::ty::TypeInterner;
    use nvs_hir::hierarchy::ClassGraph;

    /// `(from, to)` through the relation, with an empty hierarchy and no
    /// signatures: nothing here names a class.
    fn assignable(
        interner: &mut TypeInterner,
        from: crate::ty::TypeId,
        to: crate::ty::TypeId,
    ) -> bool {
        is_assignable(
            from,
            to,
            interner,
            &ClassGraph::default(),
            &SignatureTable::default(),
        )
    }

    /// `rule:types/callable-signature`: bare `callable` is the top, so the
    /// lattice is one-way. The refused direction is what the annotation buys —
    /// a value whose signature is unknown does not fill a position promising
    /// one.
    #[test]
    fn a_signature_satisfies_bare_callable_and_never_the_reverse() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let string = i.string();
        let sig = i.callable_sig(vec![int], string);
        let top = i.callable();
        assert!(assignable(&mut i, sig, top));
        assert!(!assignable(&mut i, top, sig));
    }

    /// `rule:types/callable-arity`: `n ≤ m`, comparing the first `n`. A closure
    /// declaring fewer parameters than the slot offers is the ordinary case,
    /// and one declaring more has no arguments to read.
    #[test]
    fn arity_is_a_prefix_match() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let string = i.string();
        let two = i.callable_sig(vec![int, string], string);
        let one = i.callable_sig(vec![int], string);
        let none = i.callable_sig(Vec::new(), string);
        assert!(assignable(&mut i, one, two));
        assert!(assignable(&mut i, none, two));
        assert!(!assignable(&mut i, two, one));
        // The prefix compared is still compared: a first parameter that does
        // not accept what the slot passes is refused at any arity.
        let wrong = i.callable_sig(vec![string], string);
        assert!(!assignable(&mut i, wrong, two));
    }

    /// `rule:types/callable-variance`: the two accepted directions and the two
    /// refused ones, asked as one, since a member that got a single direction
    /// backwards still answers plausibly on either half alone.
    #[test]
    fn parameters_are_contravariant_and_the_return_covariant() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let string = i.string();
        let mixed = i.mixed();
        let never = i.never();
        let slot = i.callable_sig(vec![int], string);
        let wider_param = i.callable_sig(vec![mixed], string);
        let narrower_return = i.callable_sig(vec![int], never);
        let wider_return = i.callable_sig(vec![int], mixed);
        assert!(assignable(&mut i, wider_param, slot));
        assert!(assignable(&mut i, narrower_return, slot));
        assert!(!assignable(&mut i, wider_return, slot));
        // And the narrower parameter, which is the other unsound direction:
        // the slot may be handed any `int`, not only the literal `1`.
        let one = i.int_literal(1);
        let narrower_param = i.callable_sig(vec![one], string);
        assert!(!assignable(&mut i, narrower_param, slot));
    }
}
