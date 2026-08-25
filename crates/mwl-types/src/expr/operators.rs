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
//! Two more refusals are about which operands may *meet* rather than what they
//! produce. ADR 0090 § 2 refuses `==`/`!=` between two statically **disjoint**
//! types ([`reject_disjoint_equality`]), and its § 6 points a `switch` label
//! and a `match` arm at the same check; ADR 0069 § 2 refuses `+`/`+=` with an
//! array operand ([`reject_array_combination`]), naming `Core\Arr::underlay`.
//!
//! `as` is here too, as the conversion's *operand* rule
//! ([`reject_enum_to_enum_conversion`], and ADR 0047 § 6's
//! [`reject_impossible_literal_conversion`] for a conversion whose operand
//! already names a value the target's closed set does not contain), plus
//! ADR 0066 § 3's target rule for `as ?T`
//! ([`check_class_target_conversion`]: a class target is refused, and its
//! parse roster is the exception it records for `mwl_ir` to lower); what a
//! conversion does to a qualifier is [`super::quals`], and what its target type
//! may be spelled as is [`crate::lower`].
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
///
/// ADR 0047 § 4 adds the string literal to that same branch, for the same
/// reason one step further on: `"a" as "a"|"b"` is a conversion the target
/// *statically satisfies*, and without the placement the operand would be a
/// plain `string` converting into the set at run time. What the placement
/// cannot satisfy, [`reject_impossible_literal_conversion`] refuses outright —
/// § 6's two diagnostics, and the only pair of conversions the operand's own
/// type can prove nothing will ever come of.
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
    let inner_ty = if matches!(
        inner.kind,
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Str(_)
    ) {
        super::infer(inner, Some(result), live, scope, ctx, env)
    } else {
        check_expr(inner, None, live, scope, ctx, env)
    };
    if matches!(env.interner.get(result), Ty::String) {
        require_stringable(inner_ty, inner.span, env);
    }
    check_class_target_conversion(ty, inner_ty, result, expr.span, env);
    reject_enum_to_enum_conversion(inner_ty, result, expr.span, env);
    reject_secret_markup_conversion(inner_ty, result, expr.span, env);
    reject_non_literal_markup_conversion(inner, result, expr.span, env);
    reject_impossible_literal_conversion(inner, inner_ty, result, expr.span, env);
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
        BinaryOp::Add => reject_array_combination(lhs, rhs, span, env)
            .unwrap_or_else(|| arithmetic_result(lhs, rhs, span, env)),
        BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Mod => arithmetic_result(lhs, rhs, span, env),
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
        BinaryOp::Eq | BinaryOp::NotEq => {
            reject_disjoint_equality(lhs, rhs, span, env);
            // ADR 0033 § 5: two `secret` operands compare in constant time.
            // Nothing about the *result* changes — it is a `bool` either way —
            // so this records the fact for `mwl-ir` rather than returning a
            // different type. It has to be recorded here because the qualifier
            // does not survive `mwl_ir::ty::Ty`, which is § 1's promise that a
            // `secret string` costs no representation; see
            // `ExprInfo::SecretEquality`.
            if is_secret(lhs, env.interner) || is_secret(rhs, env.interner) {
                env.exprs
                    .record(span, crate::expr_table::ExprInfo::SecretEquality);
            }
            env.interner.bool_ty()
        }
        BinaryOp::And | BinaryOp::Or => env.interner.bool_ty(),
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

/// [ADR 0090](../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
/// § 2: a comparison whose two static types are **disjoint** — no single value
/// inhabits both — is a compile error, because the compiler already knows the
/// answer and the author did not mean to write it. Reported for `==`/`!=` from
/// [`binary_result`] and, per that ADR's § 6, for a `switch` label and a
/// `match` arm against their subject ([`crate::locals`]'s `Switch` arm and
/// [`super::infer`]'s `Match`), which are the same comparison written without
/// the operator.
///
/// Only the refusal lives here. What equality *means* at a type that survives
/// it is § 3, which is `mwl-runtime`'s; the narrowing a null test performs is
/// [`crate::locals`]'s `narrow`.
pub(crate) fn reject_disjoint_equality(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) {
    if !types_are_disjoint(lhs, rhs, env) {
        return;
    }
    let lhs_is_null = matches!(env.interner.get(lhs), Ty::Null);
    let rhs_is_null = matches!(env.interner.get(rhs), Ty::Null);
    // The table gives the `null` row its own wording: the author wrote a test
    // that reads as a question, and what they need told is that the binding
    // they tested was never declared able to answer it.
    let (message, help) = if lhs_is_null || rhs_is_null {
        let held = env.interner.describe(if lhs_is_null { rhs } else { lhs });
        (
            format!("`{held}` cannot hold `null`, so this test is always false"),
            "declare the binding nullable — `?T` — if it is meant to be optional, or drop the test",
        )
    } else {
        let lhs_described = env.interner.describe(lhs);
        let rhs_described = env.interner.describe(rhs);
        (
            format!(
                "`{lhs_described}` and `{rhs_described}` are disjoint; no value is both, so this \
                 comparison is always false"
            ),
            "convert one side deliberately and then compare — `$s == ($n as string)`",
        )
    };
    env.diags.report(
        Diagnostic::error(code::E_DISJOINT_EQUALITY, message)
            .with_primary(span, "compared here")
            .with_help(help),
    );
}

/// Whether no single value inhabits both `lhs` and `rhs` — ADR 0090 § 2's
/// table, read as a *disjointness* question rather than an equality-of-types
/// one, which is what keeps every shape ordinary code writes compiling.
///
/// Deliberately one-sided: it answers `true` only where disjointness is
/// provable from the two types alone, and `false` — "these may overlap" — for
/// everything it does not model. A missed diagnostic costs an author nothing;
/// a wrong one costs them a program that used to build. `mixed` (§ 5 resolves
/// it at runtime instead), `iterable`, an intersection, an options bag and a
/// type variable all take that branch, as does any class name this
/// compilation did not declare.
fn types_are_disjoint(lhs: TypeId, rhs: TypeId, env: &Env<'_>) -> bool {
    if lhs == rhs {
        return false;
    }
    // A union is disjoint from the other side only when *every* member is,
    // which is the whole of the table's `?T`/union rows: `?T == null` and
    // `?T == T` both find an overlapping member, while `T == null` has none.
    if let Ty::Union(members) = env.interner.get(lhs) {
        return members.iter().all(|m| types_are_disjoint(*m, rhs, env));
    }
    if let Ty::Union(members) = env.interner.get(rhs) {
        return members.iter().all(|m| types_are_disjoint(lhs, *m, env));
    }
    let (lhs_ty, rhs_ty) = (env.interner.get(lhs), env.interner.get(rhs));
    let (Some(lhs_domain), Some(rhs_domain)) = (equality_domain(lhs_ty), equality_domain(rhs_ty))
    else {
        return false;
    };
    if lhs_domain != rhs_domain {
        return true;
    }
    // One row refines further: two class names share a domain, but no
    // instance is both unless one of them reaches the other.
    match (lhs_ty, rhs_ty) {
        (Ty::Class(lhs_q, _), Ty::Class(rhs_q, _)) => classes_are_unrelated(lhs_q, rhs_q, env),
        _ => false,
    }
}

/// The domains ADR 0090 § 2's table partitions comparable types into: two
/// values can only ever be equal when their types land in the same one.
/// `None` means "not modeled", and [`types_are_disjoint`] owns what that
/// buys.
#[derive(PartialEq, Eq)]
enum EqDomain<'a> {
    Null,
    Bool,
    /// `int`, `uint`, `float` and `decimal` are **one** domain — the table's
    /// numeric row makes them mathematically exact over the full range of
    /// both, so no pairing among them is ever disjoint.
    Numeric,
    Str,
    Bytes,
    Array,
    /// A class, a shape, or `object`.
    Object,
    Callable,
    /// An enum is its own domain, per name — which is what makes both
    /// `$e == 1` and `$e == $otherEnum` refusals, the first answered by
    /// ADR 0010 § 3's `as int` and the second by an explicit `match`.
    Enum(&'a QName),
}

fn equality_domain(ty: &Ty) -> Option<EqDomain<'_>> {
    Some(match ty {
        Ty::Null => EqDomain::Null,
        Ty::Bool | Ty::True | Ty::False => EqDomain::Bool,
        Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal => EqDomain::Numeric,
        // A qualifier is a fact about where a value has been, never about
        // which values it can hold (ADR 0024 § 2 / ADR 0033 § 2), so all four
        // spellings of each base are one domain.
        Ty::String | Ty::TaintedString | Ty::SecretString | Ty::SecretTaintedString => {
            EqDomain::Str
        }
        Ty::Bytes | Ty::TaintedBytes | Ty::SecretBytes | Ty::SecretTaintedBytes => EqDomain::Bytes,
        // ADR 0047 § 5 gives a literal type its base's representation exactly,
        // so it lands in its base's domain and nothing more: `$mode == "z"`
        // where `$mode` is `"a"|"b"` compares two strings and is answered at
        // run time. Refusing it because the two literal *sets* do not overlap
        // would be a new row in ADR 0090 § 2's table, not a consequence of
        // this one.
        Ty::StringLiteral(_) => EqDomain::Str,
        Ty::IntLiteral(_) => EqDomain::Numeric,
        Ty::Array(_) => EqDomain::Array,
        Ty::Object | Ty::Class(..) | Ty::Shape(_) => EqDomain::Object,
        Ty::Callable | Ty::CallableTo(_) => EqDomain::Callable,
        // Both spellings of "a value of this enum" — ADR 0047 § 3 keeps a case
        // type a *subtype* of its enum, so it shares its enum's domain and
        // stays disjoint from every other one, `int` included.
        Ty::Enum(qname, _) | Ty::EnumCase(qname, _, _) => EqDomain::Enum(qname),
        Ty::Mixed
        | Ty::Iterable
        | Ty::Void
        | Ty::Never
        | Ty::Union(_)
        | Ty::Intersection(_)
        | Ty::Options(_)
        | Ty::TypeVar(_) => return None,
    })
}

/// Whether no instance can be both a `lhs_q` and a `rhs_q` — the table's "two
/// unrelated classes" row, and the one place this check consults the
/// hierarchy rather than the type alone.
///
/// An **interface** on either side is never unrelated: this sees two names,
/// and some third class the comparison never mentions may implement both. A
/// name with no `class` declaration in this compilation — every `Core` class
/// among them — is treated the same way, which is the same scoping
/// [`object_comparison_result`] and [`require_stringable`] already use.
fn classes_are_unrelated(lhs_q: &QName, rhs_q: &QName, env: &Env<'_>) -> bool {
    if lhs_q == rhs_q {
        return false;
    }
    if !is_declared_class(lhs_q, env) || !is_declared_class(rhs_q, env) {
        return false;
    }
    !mwl_hir::implements_interface(lhs_q, rhs_q, env.graph)
        && !mwl_hir::implements_interface(rhs_q, lhs_q, env.graph)
}

fn is_declared_class(qname: &QName, env: &Env<'_>) -> bool {
    env.symbols
        .get(qname)
        .is_some_and(|symbol| symbol.kind == mwl_hir::SymbolKind::Class)
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

/// [ADR 0069](../../../docs/adr/0069-array-combination-is-key-type-independent.md)
/// § 2: binary `+` and `+=` with an array operand are a compile error naming
/// `Core\Arr::underlay`. PHP's array union operator is *removed*, not migrated,
/// so there is no silent behaviour change to fall into — the operator simply
/// stops compiling, and `mwl convert` rewrites `$a + $b` to the member.
///
/// Returns `Some` once diagnosed, `None` for every other operand pair so
/// [`arithmetic_result`]'s own table runs unchanged. The recovery type is the
/// array operand rather than `mixed`, so `$a += $b` reports this once instead
/// of also failing [`check_compound_assign`]'s write-back check with a second,
/// less useful diagnostic.
fn reject_array_combination(
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let lhs_is_array = matches!(env.interner.get(lhs), Ty::Array(_));
    let rhs_is_array = matches!(env.interner.get(rhs), Ty::Array(_));
    if !lhs_is_array && !rhs_is_array {
        return None;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ARRAY_PLUS_UNSUPPORTED,
            "`+` is not defined with an array operand",
        )
        .with_primary(span, "array operand of `+`")
        .with_help(concat!(
            r"use `Core\Arr::underlay($a, $b)`, which keeps the left array's value at every ",
            "key both of them have — the one operation `$a + $b` ever meant",
        )),
    );
    Some(if lhs_is_array { lhs } else { rhs })
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

/// [ADR 0066](../../../docs/adr/0066-nullable-conversion-operator.md) § 3's
/// last table row and the **parse-roster** row above it, which are one
/// decision read twice: `$obj as ?SomeClass` is a compile error because
/// `instanceof` plus ADR 0007 § 6's narrowing already answers class
/// membership, while `$s as ?Core\Uri` and `$s as ?Core\Uuid` are the closed
/// roster that ADR defines directly as "that type's `parse`, and `null` where
/// it throws". Turning text into a value is a different question from class
/// membership, which is why the roster does not reopen the row.
///
/// Keyed on the **written `?T` sugar**, not on the interned target: § 1
/// deliberately leaves the `Core\Uri|null` union spelling out of the form, so
/// a target reached any other way is not this ADR's and is left alone. A
/// plain `as SomeClass` is left alone too — that is ADR 0007 § 2's table
/// having no row for a class type, which is a separate refusal this slice
/// does not add.
///
/// Recording [`ExprInfo::ParseRosterConversion`] is the other half of the
/// job, and it is what `mwl_ir` lowers from: nothing survives into
/// `mwl_ir::ty::Ty` that says which class was written, and the roster is
/// `mwl_stdlib`'s to state, so the symbol travels through the table the way
/// [`ExprInfo::SecretEquality`] travels.
fn check_class_target_conversion(
    ty: &Type,
    from: TypeId,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) {
    if !is_written_nullable(ty) {
        return;
    }
    let Some(class) = nullable_class_target(to, env) else {
        return;
    };
    let roster = mwl_stdlib::registry::parse_roster_symbol(&class);
    if let Some(symbol) = roster
        && operand_is_text(from, env.interner)
    {
        env.exprs
            .record(span, ExprInfo::ParseRosterConversion { symbol });
        return;
    }
    let (message, help) = if roster.is_some() {
        let described = env.interner.describe(from);
        (
            format!("`{described}` is not text, so there is no `{class}` to parse out of it"),
            "`as ?T` over a parse-roster type reads a `string` — convert the operand to one first",
        )
    } else {
        (
            format!("`{class}` is a class, so `as ?{class}` is not a conversion"),
            "ask `$x instanceof Name` and use the value the test narrows; text becomes a value \
             through that class's own named constructor",
        )
    };
    env.diags.report(
        Diagnostic::error(code::E_CLASS_CONVERSION_TARGET, message)
            .with_primary(span, "converted here")
            .with_help(help),
    );
}

/// Whether the target was written as the `?T` sugar, `(...)` transparent —
/// `mwl_ir::lower`'s own `nullable_target` reads it the same way, and the two
/// have to agree or a target this leaves alone reaches a lowering that
/// expects it to have been decided here.
fn is_written_nullable(ty: &Type) -> bool {
    match &ty.kind {
        TypeKind::Nullable(_) => true,
        TypeKind::Paren(inner) => is_written_nullable(inner),
        _ => false,
    }
}

/// The class an `as ?T` target names, or `None` for every other target.
///
/// `?T` interns as exactly `Union([Null, T])` — the checker has no separate
/// nullable type ([`crate::lower::lower_type`]) — so this reads the one
/// non-`null` member back out. A union with more than one is not the `?T`
/// sugar's shape and is left to the caller's `None` path.
fn nullable_class_target(to: TypeId, env: &Env<'_>) -> Option<String> {
    let Ty::Union(members) = env.interner.get(to) else {
        return None;
    };
    let mut named = members
        .iter()
        .filter(|member| !matches!(env.interner.get(**member), Ty::Null));
    let only = *named.next()?;
    if named.next().is_some() {
        return None;
    }
    match env.interner.get(only) {
        Ty::Class(name, _) => Some(name.to_string()),
        _ => None,
    }
}

/// Whether an operand is text a roster type's `parse` can read — ADR 0066
/// § 3's row says "a `string` operand", and its `mixed` row says every target
/// has a checked path from there.
///
/// All four qualified spellings are text, and the qualifier is not this
/// conversion's question: `as ?Core\Uri` *is* `Core\Uri::parse`, so whatever
/// that member does with a `tainted` argument it does here too. Classifying
/// a `Core` parameter's qualifier at all is
/// [ADR 0088](../../../docs/adr/0088-taint-carriers-and-sinks.md)'s
/// registry-wide item, still open, and closing it closes both spellings at
/// once rather than one of them here.
fn operand_is_text(from: TypeId, interner: &TypeInterner) -> bool {
    matches!(
        interner.get(from),
        Ty::String
            | Ty::TaintedString
            | Ty::SecretString
            | Ty::SecretTaintedString
            | Ty::StringLiteral(_)
            | Ty::Mixed
    )
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

/// [ADR 0047](../../../docs/adr/0047-literal-and-enum-case-types.md) § 6: a
/// checked `as` into a closed set of literals or enum cases, whose operand
/// names one value and that value is not in the set — `"z" as "a"|"b"`,
/// `Mode::Admin as Mode::Read|Mode::Write`. Nothing about it is conditional at
/// run time: it would compile and then throw on every execution, so the
/// author is told now.
///
/// Both halves have to be closed for that to be provable. The **target** is
/// closed only when every atom is a literal or a case ([`closed_set_atoms`]) —
/// one wider atom (`string`, or the `null` an `as ?T` adds) is a member the
/// operand may well reach. The **operand** is closed only when the expression
/// itself names a single value ([`conversion_operand_singleton`]), since
/// `$s as "a"|"b"` over a plain `string` is exactly § 4's checked row and has
/// to compile.
///
/// This is deliberately not [`types_are_disjoint`]'s business.
/// [ADR 0090](../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
/// § 2 puts a literal type in its base type's domain, so `$mode == "z"` stays
/// an ordinary run-time string comparison; what is refused here is a
/// *conversion* that can only throw, which is a different question reaching a
/// different answer.
fn reject_impossible_literal_conversion(
    inner: &Expr,
    inner_ty: TypeId,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) {
    let Some(accepted) = closed_set_atoms(to, env.interner) else {
        return;
    };
    let Some(operand) = conversion_operand_singleton(inner, inner_ty, env) else {
        return;
    };
    if accepted.contains(&operand) {
        return;
    }
    // § 6: the accepted set is generated from the type, never written per
    // site, so every atom is rendered by the interner that holds it.
    let cases_only = accepted
        .iter()
        .all(|id| matches!(env.interner.get(*id), Ty::EnumCase(..)));
    let listed: Vec<String> = accepted
        .iter()
        .map(|id| format!("`{}`", env.interner.describe(*id)))
        .collect();
    let code = if cases_only {
        code::E_ENUM_CASE_SUBSET_MISMATCH
    } else {
        code::E_LITERAL_TYPE_MISMATCH
    };
    let named = env.interner.describe(operand);
    env.diags.report(
        Diagnostic::error(
            code,
            format!("`{named}` is not one of {}", listed.join(", ")),
        )
        .with_primary(span, "converted here")
        .with_help(
            "write one of the accepted values, or widen the target type to include this one",
        ),
    );
}

/// The atoms of a target type that is **entirely** literals and enum cases,
/// in the order the type itself states them, or `None` for every other
/// target — which is what keeps
/// [`reject_impossible_literal_conversion`] to the one row it can prove.
fn closed_set_atoms(to: TypeId, interner: &TypeInterner) -> Option<Vec<TypeId>> {
    let atoms: Vec<TypeId> = match interner.get(to) {
        Ty::Union(members) => members.clone(),
        _ => vec![to],
    };
    atoms
        .iter()
        .all(|id| {
            matches!(
                interner.get(*id),
                Ty::StringLiteral(_) | Ty::IntLiteral(_) | Ty::EnumCase(..)
            )
        })
        .then_some(atoms)
}

/// The one value the conversion's *operand* names, or `None` where it names
/// more than one.
///
/// The first arm is the placement in [`infer_conversion`] answering for
/// itself: an operand the target accepts has already **taken** one of the
/// target's own atoms, so it is in hand with nothing to re-derive. Everything
/// after it is the failing side of that same placement, where the literal has
/// widened back to its base and the value it named has to be recovered from
/// the expression — [`literal_self_type`] for § 1's two literals, and the
/// recorded [`crate::expr_table::ExprInfo::EnumCase`] for § 3's case. That
/// record rather than the expression's shape, because `Core\X::SOME_CONST` is
/// the same `ClassConstAccess` node and may be enum-typed without naming a
/// case at all.
fn conversion_operand_singleton(
    inner: &Expr,
    inner_ty: TypeId,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    if matches!(
        env.interner.get(inner_ty),
        Ty::StringLiteral(_) | Ty::IntLiteral(_) | Ty::EnumCase(..)
    ) {
        return Some(inner_ty);
    }
    if let Some(literal) = literal_self_type(inner, env) {
        return Some(literal);
    }
    let ExprKind::ClassConstAccess { name, .. } = &inner.kind else {
        return None;
    };
    if !matches!(
        env.exprs.lookup(inner.span),
        Some(crate::expr_table::ExprInfo::EnumCase { .. })
    ) {
        return None;
    }
    let Ty::Enum(qname, backing) = env.interner.get(inner_ty).clone() else {
        return None;
    };
    let case = span_text(env.src, *name).to_owned();
    Some(env.interner.enum_case(qname, backing, case))
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
        return;
    }
    // ADR 0028 § 1: the conversion *is* a `toString()` call, so `mwl-ir` needs
    // its resolved target the same way an ordinary `$obj->toString()` does —
    // and, exactly like `object_comparison_result`'s `compareTo`, there is no
    // call node in the AST to key it by. The operand's own span is the key,
    // and it goes in its own map rather than an `ExprInfo`: the operand is an
    // ordinary expression that has usually already recorded an entry there
    // (`echo $b->build()` records the call `it` is), and one span holding two
    // independent facts is what `ExprTypeTable`'s side maps exist for.
    if let Some((owner, sig)) = resolve_method(&qname, "toString", env.signatures, env.graph) {
        let call = resolved_call(owner, "toString".to_owned(), &sig, env.signatures);
        env.exprs.record_to_string(span, call);
    }
}
