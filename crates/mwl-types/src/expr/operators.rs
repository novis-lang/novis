//! What an operator's operands have to be, and what it produces: ADR 0007
//! § 4's result-type table and the three refusals layered onto it.
//!
//! [`binary_result`] is the table itself, including refusing `int ⊕ uint`.
//! ADR 0013's `Comparable` requirement is the amendment for the five ordering
//! operators when both operands are objects ([`object_comparison_result`]),
//! with no property-walk fallback. ADR 0028 § 1's sibling is
//! [`require_stringable`], which refuses an object at every *implicit*
//! string-conversion site — interpolation, concatenation, `echo`/`print`,
//! `as string` — unless it provably implements the reserved global
//! `Stringable` interface. ADR 0054 § 3 refuses `decimal ⊕ float` outright
//! ([`reject_decimal_float_operands`]), and ADR 0010 refuses arithmetic on an
//! enum and a conversion between two of them.
//!
//! `as` is here too, as the conversion's *operand* rule
//! ([`reject_enum_to_enum_conversion`]); what a conversion does to a qualifier
//! is [`super::quals`], and what its target type may be spelled as is
//! [`crate::lower`].
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(super)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// `expr as T` — [`super::infer`]'s `ExprKind::Conversion` arm, and the only
/// conversion spelling there is (ADR 0034 rejects PHP's legacy `(T)expr`).
///
/// ADR 0054 § 2: `expr as T` is itself a placing position, so a numeric
/// *literal* written directly under one takes `T` as its target rather than
/// being typed first and converted afterwards. Without this, `19.99 as decimal`
/// would round-trip through an `f64` and lose everything past ~17 digits — § 4's
/// `float → decimal` row — making a wider literal unwritable anywhere that lacks
/// an annotation. Restricted to a literal operand on purpose: any other operand
/// already has a type of its own, and handing it an expectation would silently
/// change what `as` converts *from*. [`super::infer`] rather than
/// [`check_expr`], because a placement is not an assignment: `1 as string` still
/// places the literal at `string` and still converts, so the conformance check
/// [`check_expr`] would run here would reject every conversion that does any
/// work.
pub(super) fn infer_conversion(
    expr: &Expr,
    inner: &Expr,
    ty: &Type,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let result = lower_type(ty, ctx, env);
    let inner_ty = if matches!(inner.kind, ExprKind::Int(_) | ExprKind::Float(_)) {
        super::infer(inner, Some(result), live, scope, ctx, env)
    } else {
        check_expr(inner, None, live, scope, ctx, env)
    };
    if matches!(env.interner.get(result), Ty::String) {
        require_stringable(inner_ty, inner.span, env);
    }
    reject_enum_to_enum_conversion(inner_ty, result, expr.span, env);
    reject_secret_markup_conversion(inner_ty, result, expr.span, env);
    reject_non_literal_markup_conversion(inner, result, expr.span, env);
    apply_qualifier_conversion_rule(inner_ty, result, env.interner)
}

/// The binary-operator result-type table, ADR 0007 § 4, amended by ADR 0013
/// § 6 for `< <= > >= <=>` when both operands are objects. Beyond that one
/// amendment, only `int`/`uint`/`float` operands are modeled this slice —
/// anything else (`mixed`, an unresolved call result) falls back to `mixed`
/// rather than diagnosing, since no general operator-overload rule is
/// implemented yet.
pub(super) fn binary_result(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> TypeId {
    match op {
        // ADR 0024 § 2 / ADR 0033 § 2: concatenating a qualified operand with
        // an unqualified one poisons the result on that axis, the same
        // "poisoned" shape ADR 0007 already uses for mixed-type arithmetic —
        // `tainted` and `secret` poison independently of each other.
        BinaryOp::Concat => {
            let tainted = is_tainted(lhs, env.interner) || is_tainted(rhs, env.interner);
            let secret = is_secret(lhs, env.interner) || is_secret(rhs, env.interner);
            qualified_scalar(false, tainted, secret, env.interner)
        }
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Mod => {
            arithmetic_result(lhs, rhs, span, env)
        }
        BinaryOp::Pow => power_result(lhs, rhs, span, env),
        BinaryOp::Div => division_result(lhs, rhs, span, env),
        BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
            bitwise_result(lhs, rhs, span, env)
        }
        BinaryOp::Cmp => {
            object_comparison_result(op, lhs, rhs, span, env).unwrap_or_else(|| env.interner.int())
        }
        BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
            object_comparison_result(op, lhs, rhs, span, env)
                .unwrap_or_else(|| env.interner.bool_ty())
        }
        BinaryOp::Eq
        | BinaryOp::NotEq
        | BinaryOp::Identical
        | BinaryOp::NotIdentical
        | BinaryOp::And
        | BinaryOp::Or => env.interner.bool_ty(),
        // `$a ?? $b` yields `$b` exactly when `$a` is `null`, so `null` is
        // gone from the result unless `$b` can be one — which is what makes
        // `string $s = $maybe ?? "d";` type-check at all. Recorded for
        // `mwl-ir` at the same time: see `ExprInfo::Coalesce`.
        BinaryOp::Coalesce => {
            let non_null = env.interner.without_null(lhs);
            let result = env.interner.make_union([non_null, rhs]);
            env.exprs.record(
                span,
                crate::expr_table::ExprInfo::Coalesce { non_null, result },
            );
            result
        }
        _ => env.interner.mixed(),
    }
}

/// ADR 0013 §§ 2-4: `< <= > >= <=>` lower to a `compareTo` call when both
/// operands are objects, so ordering them requires both sides to be the same
/// class and that class to (transitively) implement the reserved global
/// `Comparable` interface — returns `None` when either operand isn't a class
/// at all, leaving [`binary_result`]'s ordinary scalar/`mixed` fallback in
/// place untouched, since this ADR only amends ADR 0007 § 4's table with a
/// new object-operand row rather than replacing it. An enum operand
/// (`Ty::Enum`) is deliberately not treated as an object here either — ADR
/// 0010's own item (still unimplemented) is what would say whether an enum
/// can ever be `Comparable`.
pub(super) fn object_comparison_result(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let (Ty::Class(lhs_q, _), Ty::Class(rhs_q, _)) =
        (env.interner.get(lhs).clone(), env.interner.get(rhs).clone())
    else {
        return None;
    };
    if lhs_q != rhs_q {
        report_comparable_diagnostic(
            span,
            format!(
                "`{lhs_q}` and `{rhs_q}` are different classes; `<`/`<=`/`>`/`>=`/`<=>` never \
                 compare across classes, even when both implement `Comparable`"
            ),
            env,
        );
        return Some(env.interner.mixed());
    }
    let comparable = QName::parse("Comparable");
    if !mwl_hir::implements_interface(&lhs_q, &comparable, env.graph) {
        report_comparable_diagnostic(
            span,
            format!(
                "`{lhs_q}` does not implement `Comparable`; ordering two objects with \
                 `<`/`<=`/`>`/`>=`/`<=>` requires it"
            ),
            env,
        );
        return Some(env.interner.mixed());
    }
    // ADR 0013 § 2: the comparison *is* a `compareTo` call, so `mwl-ir` needs
    // its resolved target the same way an ordinary `$a->compareTo($b)` does —
    // recorded under the *binary expression's* own span, since there is no
    // call node in the AST to key it by. `Comparable::compareTo` is bodiless,
    // so `has_body` is `false` and the call dispatches on the receiver's
    // runtime class, exactly like any other call to an interface method.
    if let Some((owner, sig)) = resolve_method(&lhs_q, "compareTo", env.signatures, env.graph) {
        let call = resolved_call(owner, "compareTo".to_owned(), &sig, env.signatures);
        env.exprs.record(span, ExprInfo::Call(call));
    }
    Some(match op {
        BinaryOp::Cmp => env.interner.int(),
        _ => env.interner.bool_ty(),
    })
}

pub(super) fn report_comparable_diagnostic(span: Span, message: String, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(code::E_COMPARISON_REQUIRES_COMPARABLE, message)
            .with_primary(span, "compared here")
            .with_help("implement `Comparable`'s `compareTo(self $other): int` on the class"),
    );
}

pub(super) fn arithmetic_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    if let Some(mixed) = reject_enum_operand(lhs, rhs, span, env) {
        return mixed;
    }
    if let Some(mixed) = reject_decimal_float_operands(lhs, rhs, span, env) {
        return mixed;
    }
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
        // ADR 0054 § 3: a `decimal` combined with an integer stays `decimal` —
        // an `int`/`uint` is exact in 96 bits, so nothing is lost. The
        // `decimal`/`float` pair never reaches here; it was rejected above.
        (Ty::Decimal, Ty::Decimal | Ty::Int | Ty::Uint) | (Ty::Int | Ty::Uint, Ty::Decimal) => {
            env.interner.decimal()
        }
        (Ty::Float, _) | (_, Ty::Float) => env.interner.float(),
        (Ty::Int, Ty::Int) => env.interner.int(),
        (Ty::Uint, Ty::Uint) => env.interner.uint(),
        (Ty::Int, Ty::Uint) | (Ty::Uint, Ty::Int) => {
            report_int_uint(span, env);
            env.interner.mixed()
        }
        _ => env.interner.mixed(),
    }
}

/// ADR 0010 § 5: "No arithmetic or bitwise operator is defined on an enum
/// type directly" — `Permission::Read | Permission::Write` must be diagnosed
/// naming `as uint`/`as int` as the fix rather than silently falling through
/// to [`arithmetic_result`]/[`bitwise_result`]'s existing `_ => mixed` arm,
/// which would otherwise swallow the mistake with no diagnostic at all.
/// Returns `Some(mixed)` when either operand is `Ty::Enum` (already
/// diagnosed), `None` for every other operand pair so the caller's own table
/// runs unchanged.
pub(super) fn reject_enum_operand(
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let lhs_enum = matches!(env.interner.get(lhs), Ty::Enum(..));
    let rhs_enum = matches!(env.interner.get(rhs), Ty::Enum(..));
    if !lhs_enum && !rhs_enum {
        return None;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ENUM_ARITHMETIC_UNSUPPORTED,
            "no arithmetic or bitwise operator is defined on an enum type directly",
        )
        .with_primary(span, "enum operand used here")
        .with_help("convert to the underlying type first: `... as int`/`... as uint`"),
    );
    Some(env.interner.mixed())
}

pub(super) fn division_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    if let Some(mixed) = reject_enum_operand(lhs, rhs, span, env) {
        return mixed;
    }
    if let Some(mixed) = reject_decimal_float_operands(lhs, rhs, span, env) {
        return mixed;
    }
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
        // ADR 0054 § 3's deliberate divergence from `int / int`: a decimal
        // quotient is always `decimal`, never a union with `float`. Division is
        // the one place the result may be inexact, and the ADR fixes its
        // rounding in the language rather than in a union the caller unpacks.
        (Ty::Decimal, Ty::Decimal | Ty::Int | Ty::Uint) | (Ty::Int | Ty::Uint, Ty::Decimal) => {
            env.interner.decimal()
        }
        (Ty::Float, _) | (_, Ty::Float) => env.interner.float(),
        (Ty::Int, Ty::Int) => {
            let int = env.interner.int();
            let float = env.interner.float();
            env.interner.make_union([int, float])
        }
        (Ty::Uint, Ty::Uint) => {
            let uint = env.interner.uint();
            let float = env.interner.float();
            env.interner.make_union([uint, float])
        }
        (Ty::Int, Ty::Uint) | (Ty::Uint, Ty::Int) => {
            report_int_uint(span, env);
            env.interner.mixed()
        }
        _ => env.interner.mixed(),
    }
}

pub(super) fn bitwise_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    if let Some(mixed) = reject_enum_operand(lhs, rhs, span, env) {
        return mixed;
    }
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
        (Ty::Int, Ty::Int) => env.interner.int(),
        (Ty::Uint, Ty::Uint) => env.interner.uint(),
        (Ty::Int, Ty::Uint) | (Ty::Uint, Ty::Int) => {
            report_int_uint(span, env);
            env.interner.mixed()
        }
        _ => env.interner.mixed(),
    }
}

/// ADR 0054 § 3: `decimal ⊕ float` is a compile error, on the same grounds
/// `int ⊕ uint` already is — there is no type that represents both operands'
/// values, so the fix is to convert one side and say which. Returns
/// `Some(mixed)` once diagnosed, `None` for every other pair so the caller's
/// own table runs unchanged. Comparison is deliberately *not* routed through
/// here: the same § 3 permits `decimal < 1.5`, because an exact comparison is
/// computable even where a common arithmetic type is not.
pub(super) fn reject_decimal_float_operands(
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let pair = (env.interner.get(lhs), env.interner.get(rhs));
    if !matches!(pair, (Ty::Decimal, Ty::Float) | (Ty::Float, Ty::Decimal)) {
        return None;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_DECIMAL_FLOAT_ARITHMETIC,
            "`decimal` and `float` have no representable common type in arithmetic",
        )
        .with_primary(span, "mixed `decimal`/`float` operand")
        .with_help("convert one side explicitly with `as decimal`/`as float`"),
    );
    Some(env.interner.mixed())
}

/// ADR 0054 § 3's last row: `**` with a `decimal` base is a compile error,
/// because a general decimal power has no exact result at a bounded scale —
/// `Core\Decimal::pow` names the rounding instead. A decimal *exponent* is
/// refused by the same diagnostic: the row does not define `int ** decimal`
/// either, and letting it fall through to [`arithmetic_result`]'s decimal row
/// would invent a fractional exponentiation the ADR never granted.
pub(super) fn power_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    if matches!(env.interner.get(lhs), Ty::Decimal) || matches!(env.interner.get(rhs), Ty::Decimal)
    {
        env.diags.report(
            Diagnostic::error(
                code::E_DECIMAL_FLOAT_ARITHMETIC,
                "`**` is not defined on `decimal`",
            )
            .with_primary(span, "`decimal` operand of `**`")
            .with_help("use `Core\\Decimal::pow`, which names the rounding it does"),
        );
        return env.interner.mixed();
    }
    arithmetic_result(lhs, rhs, span, env)
}

pub(super) fn report_int_uint(span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_INT_UINT_ARITHMETIC,
            "`int` and `uint` have no representable common type in arithmetic",
        )
        .with_primary(span, "mixed-signedness operand")
        .with_help("convert one side explicitly with `as int`/`as uint`"),
    );
}

/// ADR 0028 § 1: every implicit string-conversion site — interpolation,
/// concatenation, `echo`/`print`, `as string`/`(string)` — accepts an object
/// only when its static type provably implements the reserved global
/// `Stringable` interface. Returns without diagnosing for any non-`Ty::Class`
/// operand (including `Ty::Enum`, `mixed`, and a scalar) and for an
/// unmodeled `Core` class, the same scoping [`object_comparison_result`] and
/// [`check_property_access`] already use.
/// Reports `E_TYPE_MISMATCH` for `-`, `+` or `~` applied to an object.
///
/// MWL has no operator overloading, so there is no arithmetic an object can
/// take part in — and the first place a program reaches for one is
/// [ADR 0070](../../../docs/adr/0070-duration-literals.md) § 4's `-7d`, which
/// that ADR refuses outright in favour of `->minus(7d)`. Left unchecked it
/// reaches `mwl-codegen`, which panics naming the representation; a
/// diagnostic naming the operator is what the author needs.
///
/// Only the three arithmetic prefixes: `!` is ADR 0035's truthy test, which an
/// object is perfectly legal in, and `@` is a suppression marker that says
/// nothing about its operand's type.
pub(super) fn reject_arithmetic_on_object(op: UnaryOp, ty: TypeId, span: Span, env: &mut Env<'_>) {
    if !matches!(env.interner.get(ty), Ty::Class(..) | Ty::Object) {
        return;
    }
    let spelling = match op {
        UnaryOp::Neg => "-",
        UnaryOp::Plus => "+",
        _ => "~",
    };
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_TYPE_MISMATCH,
            format!("`{spelling}` has no meaning for `{described}`"),
        )
        .with_primary(span, "an object takes part in no arithmetic")
        .with_help(
            "MWL has no operator overloading; call the member that does this — a \
             `Core\\Time\\Duration` negates with `->negated()` and subtracts with `->minus(…)`",
        ),
    );
}

/// ADR 0010 § 5: "`EnumName` → a different `EnumName`, even with the same
/// underlying type — **rejected**, even via `as`." Two enums sharing an
/// underlying type are not the same closed set, so this refuses the
/// conversion outright rather than letting [`ExprKind::Conversion`]'s
/// ordinary `lower_type` result stand unchecked; converting the same enum to
/// itself, or to/from anything that isn't `Ty::Enum` (its underlying type,
/// `mixed`, a checked-throw source) is untouched.
pub(super) fn reject_enum_to_enum_conversion(
    from: TypeId,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) {
    let (Ty::Enum(from_q, _), Ty::Enum(to_q, _)) =
        (env.interner.get(from).clone(), env.interner.get(to).clone())
    else {
        return;
    };
    if from_q == to_q {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ENUM_CONVERSION_UNSUPPORTED,
            format!(
                "`{from_q}` cannot be converted to `{to_q}`; two different enums are never \
                 interconvertible, even via `as`"
            ),
        )
        .with_primary(span, "converted here")
        .with_help("write an explicit `match` naming every case instead"),
    );
}

pub(crate) fn require_stringable(ty: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname, _) = env.interner.get(ty).clone() else {
        return;
    };
    if qname.is_core() {
        return;
    }
    let stringable = QName::parse("Stringable");
    if !mwl_hir::implements_interface(&qname, &stringable, env.graph) {
        env.diags.report(
            Diagnostic::error(
                code::E_STRINGABLE_REQUIRED,
                format!(
                    "`{qname}` cannot be converted to `string` here; it does not implement \
                     `Stringable`"
                ),
            )
            .with_primary(span, "converted to `string` here")
            .with_help("implement `Stringable`'s `toString(): string` on the class"),
        );
    }
}
