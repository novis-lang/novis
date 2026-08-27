//! Where a value must fit a declared type: ADR 0007 § 6's assignability
//! relation, and the three positions that apply it.
//!
//! [`is_assignable`] is the relation itself, and the one place ADR 0007 § 6's
//! table is written down. It carries two amendments from ADR 0036: every class
//! or shape type is `<: object` (§ 1), and a shape target is checked
//! structurally by width subtyping plus ordinary field assignability (§ 3,
//! [`shape_satisfied`]) rather than nominally — MWL's one deliberate exception
//! to otherwise fully nominal typing. **`array<T>` is covariant in its element
//! type**, and it is the only generic name in the language that is;
//! [`is_assignable`]'s own doc comment owns that rule and why ADR 0007 § 5's
//! copy-on-write value semantics make it sound where an aliasing language
//! could not. A qualifier widens but never narrows across it — see
//! [`super::quals`]. ADR 0047 § 4 adds the last amendment: a literal type, an
//! enum-case type, and any union of them widen to their base for free, which
//! [`is_assignable`] answers by one recursion through
//! [`TypeInterner::literal_base`] rather than by four table rows of its own.
//!
//! The positions are [`check_assign`] (`$x = e`, and the reads a target is
//! made of), [`check_compound_assign`] (`$x ⊕= e`, typed as the `$x = $x ⊕ e`
//! it means) and [`check_return`].
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(super)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// Whether a value of type `from` may be used where `to` is declared —
/// `to == mixed` always accepts; `from == mixed` never implicitly satisfies
/// a non-`mixed` target (ADR 0007 § 6: "`mixed` never absorbs implicitly in
/// the other direction"); otherwise `from` must equal `to`, or `to` must be
/// a union `from` is (or, if `from` is itself a union, every member is) a
/// member of. ADR 0036 § 1 amends this with real `object` subtyping (every
/// class or shape is `<: object`), and § 3 with a shape target's structural
/// check (see [`shape_satisfied`]) — the two amendments this ADR makes to
/// ADR 0007 § 6's table, needing `graph`/`signatures` only to resolve a
/// class receiver's own property types against a shape target. ADR 0024 § 2
/// and ADR 0033 § 2 add one more: a same-base `string`/`bytes` value widens
/// freely on its `tainted`/`secret` axes (see the qualifier check just above
/// [`shape_satisfied`]'s call), never narrows. ADR 0047 § 4 adds the last:
/// `"a" → string`, `Mode::Read → Mode`, and each of those over a union, are
/// free — see the widening step below for why one recursion states all four
/// rows and why the reverse direction needs no rule to refuse it. ADR 0007
/// § 2's own amendment is the last: `int`/`uint` widen into a `float`
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
    if from == to {
        return true;
    }
    if matches!(interner.get(to), Ty::Mixed) {
        return true;
    }
    if matches!(interner.get(from), Ty::Mixed) {
        return false;
    }
    // ADR 0047 § 4's first four rows, all at once. `literal_base` widens a
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
    if widened != from && is_assignable(widened, to, interner, graph, signatures) {
        return true;
    }
    // A union target is satisfied member-wise, and **membership is checked
    // first because it is the common case** — a union built out of the same
    // interned ids answers with one `contains` and no recursion at all.
    //
    // The recursion behind it is what makes every *other* rule in this
    // function apply inside a union, and it is not a special case for one
    // caller: `?Animal` refusing a `Dog` was the same hole as
    // `array<T>|Iterable<T>|Iterator<T>` (`mwl_stdlib::registry::CoreTy::Iterated`)
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
                    .any(|member| is_assignable(from, *member, interner, graph, signatures))
        };
        return match interner.get(from).clone() {
            Ty::Union(from_members) => from_members
                .into_iter()
                .all(|member| satisfies(member, interner)),
            _ => satisfies(from, interner),
        };
    }
    // ADR 0007 § 2's implicit conversion, and the whole of it: "Implicit
    // conversion happens in exactly one place: **`int` or `uint` widening into
    // a `float` position**, which is the one coercion PHP's own
    // `strict_types` permits, and it throws above 2^53 rather than rounding."
    // The throw is `mwl_ir::lower::Lowering::coerce`'s half; here it is only
    // the accepting.
    //
    // A *union* source is checked member-wise against a non-union target for
    // this row's sake rather than as a rule of its own — ADR 0007 § 4's
    // `int|float` quotient reaching a declared `float` is the one shape that
    // needs it, and it is the ADR's own worked example (`float $avg = $sum /
    // $n;`). Written as a general member-wise check because that is what the
    // relation means, and because narrowing the rule to the quotient's exact
    // two unions would make `is_assignable` name an operator.
    if let Ty::Union(from_members) = interner.get(from) {
        let from_members = from_members.clone();
        return from_members
            .into_iter()
            .all(|member| is_assignable(member, to, interner, graph, signatures));
    }
    if matches!(interner.get(to), Ty::Float) && matches!(interner.get(from), Ty::Int | Ty::Uint) {
        return true;
    }
    if matches!(interner.get(to), Ty::Object)
        && matches!(interner.get(from), Ty::Class(..) | Ty::Shape(_))
    {
        return true;
    }
    if let (Ty::Class(from_q, _), Ty::Class(to_q, to_args)) = (interner.get(from), interner.get(to))
    {
        return class_satisfied(from_q, to_q, to_args, graph, signatures);
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
    // the widened view lands in storage the narrow view still reads. ADR 0007
    // § 5 makes an MWL array a copy-on-write **value** instead, so the widened
    // binding is a separate array the moment anything writes to it, and the
    // narrow one can never observe the write. What covariance buys is every
    // signature the spec writes over a union — `Core\Arr::sum`'s
    // `array<int|float|decimal>` takes an `array<int>`, which is what a caller
    // means by it — and `Core\Arr::flip`'s `array<T>` binding a `T` it could
    // not otherwise reach.
    if let (Ty::Array(from_elem), Ty::Array(to_elem)) = (interner.get(from), interner.get(to)) {
        let (from_elem, to_elem) = (*from_elem, *to_elem);
        return is_assignable(from_elem, to_elem, interner, graph, signatures);
    }
    // ADR 0024 § 2 / ADR 0033 § 2: `tainted` and `secret` are two independent
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
/// interface `to_q` (at `to_args`) is declared — MWL's one nominal subtyping
/// rule, and deliberately the whole of it.
///
/// `from_q` satisfies `to_q` when it reaches it through `extends`/
/// `implements`; the two [`QName`]s being equal is already handled by
/// [`is_assignable`]'s interning check, since a class type is interned
/// structurally. There is **no variance**: a generic target (ADR 0053 § 2's
/// `Iterable<T>`/`Iterator<T>`, which are the only generic names user code
/// can write) additionally requires the arguments `from_q` fixed for it to
/// equal `to_args` exactly, so `Iterator<int>` never satisfies
/// `Iterator<mixed>`. Widening a cursor's element type is not obviously
/// sound in either direction — `current()` returns `T` while a future
/// `Sink<T>` would consume one — and nothing on ADR 0053's path needs it, so
/// the invariant rule is what is committed to here rather than a covariant
/// one that would be expensive to take back.
pub(super) fn class_satisfied(
    from_q: &QName,
    to_q: &QName,
    to_args: &[TypeId],
    graph: &ClassGraph,
    signatures: &SignatureTable,
) -> bool {
    if !mwl_hir::hierarchy::implements_interface(from_q, to_q, graph) {
        return false;
    }
    if to_args.is_empty() {
        return true;
    }
    crate::signatures::resolve_interface_args(from_q, to_q, signatures, graph)
        .is_some_and(|args| args == to_args)
}

/// ADR 0036 § 3's structural check for a shape target: `from` must have at
/// least every field `to_fields` names, each satisfying the field's declared
/// type by this same [`is_assignable`] rule (width subtyping — an extra
/// field on `from` is never a problem). A class receiver's field types come
/// from [`resolve_property`], the same ancestor walk an ordinary `$obj->prop`
/// access already uses; any other `from` (a scalar, `object`, a mismatched
/// shape) never satisfies a shape target.
pub(super) fn shape_satisfied(
    from: TypeId,
    to_fields: &[(String, TypeId)],
    interner: &mut TypeInterner,
    graph: &ClassGraph,
    signatures: &SignatureTable,
) -> bool {
    // Cloned rather than borrowed: [`is_assignable`] needs the interner
    // mutably for ADR 0047 § 4's widening step, and a shape's field list is
    // read while it recurses. A shape type is a handful of fields and this
    // path runs once per shape-typed assignment.
    match interner.get(from).clone() {
        Ty::Shape(from_fields) => to_fields.iter().all(|(name, field_ty)| {
            from_fields
                .iter()
                .find(|(n, _)| n == name)
                .is_some_and(|(_, from_field_ty)| {
                    is_assignable(*from_field_ty, *field_ty, interner, graph, signatures)
                })
        }),
        Ty::Class(qname, _) => to_fields.iter().all(|(name, field_ty)| {
            resolve_property(&qname, name, signatures, graph).is_some_and(|from_field_ty| {
                is_assignable(from_field_ty, *field_ty, interner, graph, signatures)
            })
        }),
        _ => false,
    }
}

pub(super) fn report_mismatch(span: Span, expected: TypeId, actual: TypeId, env: &mut Env<'_>) {
    let expected_desc = env.interner.describe(expected);
    let actual_desc = env.interner.describe(actual);
    env.diags.report(
        Diagnostic::error(
            code::E_TYPE_MISMATCH,
            format!("expected `{expected_desc}`, found `{actual_desc}`"),
        )
        .with_primary(span, format!("this is `{actual_desc}`")),
    );
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
    // ADR 0053 § 5: a generator is a lazy sequence and nothing more, so a
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
                "ADR 0053 § 5: there is no generator return value to retrieve — write \
                 `return;` to stop the sequence, or `yield` this value",
            ),
        );
        return;
    }
    let actual = infer(expr, Some(return_ty), live, scope, ctx, env);
    if !is_assignable(actual, return_ty, env.interner, env.graph, env.signatures) {
        let expected_desc = env.interner.describe(return_ty);
        let actual_desc = env.interner.describe(actual);
        env.diags.report(
            Diagnostic::error(
                code::E_BAD_RETURN_TYPE,
                format!("this method declares `{expected_desc}` but returns `{actual_desc}`"),
            )
            .with_primary(expr.span, format!("this is `{actual_desc}`")),
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the same context [`check_compound_assign`] states, with the \n              assignment operator in place of the binary one it maps to"
)]
pub(super) fn check_assign(
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
        let target_ty = check_expr(target, None, live, scope, ctx, env);
        check_write_target(target, env);
        check_expr(value, Some(target_ty), live, scope, ctx, env);
        target_ty
    }
}

/// The three assignment targets that have nowhere to write to, refused where
/// they are written rather than lowered into something that quietly drops the
/// write. The first two match PHP, which refuses the same two spellings, and
/// are standing decisions in `docs/agent/loop-goal.md`.
///
/// A **nullsafe** target (`$a?->b = v`) is refused for the operator's own
/// reason: `?->` yields `null` where the receiver is `null`, and `null` is not
/// a place. PHP says "can't use nullsafe operator in write context"; the
/// alternative is an assignment that silently does nothing on one path.
///
/// An **element write through an ADR 0014 § 1 hooked property**
/// (`$obj->hooked[0] = v`) is refused because a hooked property is a pair of
/// accessors and not a slot. ADR 0007 § 5 separates the array the `get` hook
/// answered with, and no rule pushes the separated copy back through `set` —
/// PHP raises "indirect modification of overloaded property" and discards the
/// write, so refusing *is* the PHP-compatible answer rather than a divergence.
/// Read the array into a local, write the element, assign it back.
///
/// An **element write through an erased property** — an ADR 0036 shape's
/// field, or any property of ADR 0007 § 3's plain `object` — is refused
/// because ADR 0036 § 4 resolves one by *name* at run time and stopped there:
/// a read needs only the name, a write needs a slot for the separated array to
/// land in. This is the write half of the question `E0477` answers for a
/// method call, and the answer is the same one — narrow the receiver. It is
/// the one of the three PHP would have allowed (on a `stdClass`), and it is
/// refused for the language's own reason rather than PHP's: the property reads
/// as `mixed`, and `mixed` is not indexable anywhere else either.
///
/// Called *after* the target is checked, because the hooked half reads the
/// [`ExprInfo::HookedProperty`] entry [`super::members`] records while
/// checking the access. Only the root of a subscript chain is examined:
/// `mwl_ir::lower::Lowering::lower_reassignment` flattens a nested element
/// write down to its root holder and writes every level back through that, so
/// the root is the only level with a holder at all — which is also why
/// `$obj->hooked[0][1] = v` is this same refusal and not a deeper one.
fn check_write_target(target: &Expr, env: &mut Env<'_>) {
    let mut root = target;
    let mut through_subscript = false;
    while let ExprKind::Index { base, .. } = &root.kind {
        root = base;
        through_subscript = true;
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
                    "ADR 0014 § 1 makes a hooked property a pair of accessors, not a slot, so the \
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
                    "ADR 0036 § 4 resolves such a property by *name* at run time, which gives the \
                     separated array no slot to be written back into — convert the receiver to \
                     the class that declares it first (`var $c = $x as ClassName;`), or read the \
                     property into a typed local, write the element there, and assign it back",
                ),
            );
        }
        _ => {}
    }
}

/// `$x ⊕= e`, typed as the `$x = $x ⊕ e` it means — [`AssignOp::binary_op`]
/// is the one place that pairing is written down, and `mwl_ir::lower`
/// desugars through the same answer.
///
/// The target is read first, then the value is checked *against the target's
/// own type*, so the bare `1` in `uint $u = 0; $u += 1;` takes `uint` from
/// the position rather than defaulting to `int` and colliding with it (ADR
/// 0007 § 4's literal rule). The operator's result must then be assignable
/// back to the target: `int $i = 0; $i .= "x";` is a mismatch reported at the
/// assignment, never a silent re-typing of `$i` — ADR 0037 fixes a local's
/// type at its declaration. `.=` demands a `Stringable` operand exactly the
/// way the plain `.` does.
#[expect(
    clippy::too_many_arguments,
    reason = "the five-parameter checking context every expression walker in \n              this module carries, plus the operator, the assignment's span and \n              its two operand expressions"
)]
pub(super) fn check_compound_assign(
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
    let target_ty = check_expr(target, None, live, scope, ctx, env);
    check_write_target(target, env);
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
    }
    let result = binary_result(op, target_ty, value_ty, span, env);
    if !is_assignable(result, target_ty, env.interner, env.graph, env.signatures) {
        report_mismatch(span, target_ty, result, env);
    }
    target_ty
}

pub(super) fn check_read(
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
pub(super) fn note_write(expr: &Expr, scope: &LocalScope, env: &Env<'_>) {
    if let ExprKind::Variable(span) = &expr.kind {
        scope.overwrite(strip_sigil(span_text(env.src, *span)));
    }
}
