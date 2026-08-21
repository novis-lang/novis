//! A minimal bidirectional expression checker.
//!
//! [`check_expr`] takes an optional expected type: an [`mwl_syntax::ast::ExprKind::ArrayLiteral`]
//! checked against an `array<T>` target checks every element directly
//! against `T` (ADR 0007 § 5 — "never inferred and then compared"); anything
//! else infers its type bottom-up and, when an expected type was given,
//! reports `E_TYPE_MISMATCH` on a mismatch via [`is_assignable`].
//!
//! Only a handful of expression forms are actually modeled: literals,
//! variable reads, the binary-operator result-type table (ADR 0007 § 4,
//! including refusing `int ⊕ uint`), `as`/cast conversions, array literals,
//! `new` with a bare class-name/`self`/`static` target, and `$arr[$i]`
//! indexing when `$arr`'s own type is known. Every other form — a method/
//! function call's return, property access, `match`, ternary, a closure's
//! body — is walked only for nested variable reads and reported as `mixed`;
//! see the crate docs' known gaps for why each is deferred.
//!
//! `isset(...)`/`empty(...)` are a deliberate exception: PHP tolerates an
//! unset operand there by design, and whether that still holds once every
//! local is declared and flow-checked is an open language question beyond
//! this slice, so their operands are left entirely unchecked rather than
//! guessed at.

use mwl_diagnostics::{Diagnostic, Span, code};
use mwl_syntax::ast::{
    Arg, ArrayItem, AssignOp, BinaryOp, CallArgs, CastType, Expr, ExprKind, MemberName, NewTarget,
    StringPart, UnaryOp,
};
use rustc_hash::FxHashSet;

use crate::locals::LocalScope;
use crate::lower::lower_type;
use crate::ty::{Ty, TypeId, TypeInterner};
use crate::{Ctx, Env, span_text, strip_sigil};

/// Checks `expr`, optionally against `expected`, returning the type it was
/// found (or, for an array literal checked against a target, declared) to
/// have. Reports `E_TYPE_MISMATCH` when `expected` is given and not
/// satisfied.
pub(crate) fn check_expr(
    expr: &Expr,
    expected: Option<TypeId>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let actual = infer(expr, expected, live, scope, ctx, env);
    if let Some(expected_id) = expected
        && !is_assignable(actual, expected_id, env.interner)
    {
        report_mismatch(expr.span, expected_id, actual, env);
    }
    actual
}

/// Whether a value of type `from` may be used where `to` is declared —
/// `to == mixed` always accepts; `from == mixed` never implicitly satisfies
/// a non-`mixed` target (ADR 0007 § 6: "`mixed` never absorbs implicitly in
/// the other direction"); otherwise `from` must equal `to`, or `to` must be
/// a union `from` is (or, if `from` is itself a union, every member is) a
/// member of.
#[must_use]
pub(crate) fn is_assignable(from: TypeId, to: TypeId, interner: &TypeInterner) -> bool {
    if from == to {
        return true;
    }
    if matches!(interner.get(to), Ty::Mixed) {
        return true;
    }
    if matches!(interner.get(from), Ty::Mixed) {
        return false;
    }
    if let Ty::Union(members) = interner.get(to) {
        return match interner.get(from) {
            Ty::Union(from_members) => from_members.iter().all(|m| members.contains(m)),
            _ => members.contains(&from),
        };
    }
    false
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
    let actual = infer(expr, Some(return_ty), live, scope, ctx, env);
    if !is_assignable(actual, return_ty, env.interner) {
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

fn report_mismatch(span: Span, expected: TypeId, actual: TypeId, env: &mut Env<'_>) {
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

#[expect(
    clippy::too_many_lines,
    reason = "one match arm per AST expression variant, each a couple of lines"
)]
fn infer(
    expr: &Expr,
    expected: Option<TypeId>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    match &expr.kind {
        ExprKind::Null => env.interner.null(),
        ExprKind::Bool(_) => env.interner.bool_ty(),
        // ADR 0007 § 4: "An integer literal ... is legal only where a `uint`
        // is expected" — the same digits mean `int` ordinarily and `uint`
        // exactly where that's the target, so a literal is one of the few
        // expressions checked against `expected` rather than inferred blind.
        // Magnitude range-checking (negative-into-`uint`, too-large-for-
        // either) is not modeled this slice.
        ExprKind::Int(_) => {
            let wants_uint = expected.is_some_and(|id| matches!(env.interner.get(id), Ty::Uint));
            if wants_uint {
                env.interner.uint()
            } else {
                env.interner.int()
            }
        }
        ExprKind::Float(_) => env.interner.float(),
        ExprKind::Str(_) => env.interner.string(),
        ExprKind::Interpolated(parts) => {
            for part in parts {
                if let StringPart::Expr(e) = part {
                    check_expr(e, None, live, scope, ctx, env);
                }
            }
            env.interner.string()
        }
        ExprKind::Variable(span) => {
            let name = strip_sigil(span_text(env.src, *span)).to_owned();
            check_read(&name, expr.span, live, scope, env)
        }
        ExprKind::ConstFetch(_) => env.interner.mixed(),
        ExprKind::SelfExpr | ExprKind::StaticExpr => class_of_ctx(ctx, env),
        ExprKind::ParentExpr => env.interner.mixed(),
        ExprKind::ArrayLiteral(items) => {
            check_array_literal(items, expected, live, scope, ctx, env)
        }
        ExprKind::Unary { op, expr: inner } => {
            let inner_ty = check_expr(inner, None, live, scope, ctx, env);
            match op {
                UnaryOp::Not => env.interner.bool_ty(),
                UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitNot | UnaryOp::Suppress => inner_ty,
                _ => inner_ty,
            }
        }
        ExprKind::PreIncDec { expr: inner, .. } | ExprKind::PostIncDec { expr: inner, .. } => {
            check_expr(inner, None, live, scope, ctx, env)
        }
        ExprKind::Cast { ty, expr: inner } => {
            check_expr(inner, None, live, scope, ctx, env);
            cast_result_type(*ty, env)
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let lhs_ty = check_expr(lhs, None, live, scope, ctx, env);
            let rhs_ty = check_expr(rhs, None, live, scope, ctx, env);
            binary_result(*op, lhs_ty, rhs_ty, expr.span, env)
        }
        ExprKind::Assign {
            op, target, value, ..
        } => check_assign(*op, target, value, live, scope, ctx, env),
        ExprKind::Ternary { cond, then, else_ } => {
            check_expr(cond, None, live, scope, ctx, env);
            if let Some(then) = then {
                check_expr(then, None, live, scope, ctx, env);
            }
            check_expr(else_, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::Conversion { expr: inner, ty } => {
            check_expr(inner, None, live, scope, ctx, env);
            lower_type(ty, ctx, env)
        }
        ExprKind::InstanceOf { expr: inner, class } => {
            check_expr(inner, None, live, scope, ctx, env);
            check_expr(class, None, live, scope, ctx, env);
            env.interner.bool_ty()
        }
        ExprKind::Call { callee, args } => {
            check_expr(callee, None, live, scope, ctx, env);
            check_args(args, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            check_expr(object, None, live, scope, ctx, env);
            check_member_name(method, live, scope, ctx, env);
            check_args(args, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
        } => {
            check_expr(class, None, live, scope, ctx, env);
            check_member_name(method, live, scope, ctx, env);
            check_args(args, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::PropertyAccess {
            object, property, ..
        } => {
            check_expr(object, None, live, scope, ctx, env);
            check_member_name(property, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::StaticPropertyAccess { class, .. } => {
            check_expr(class, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::ClassConstAccess { class, .. } | ExprKind::ClassNameConst { class } => {
            check_expr(class, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::Index { base, index } => {
            let base_ty = check_expr(base, None, live, scope, ctx, env);
            if let Some(index) = index {
                check_expr(index, None, live, scope, ctx, env);
            }
            match env.interner.get(base_ty) {
                Ty::Array(elem) => *elem,
                _ => env.interner.mixed(),
            }
        }
        ExprKind::New { target, args } => {
            check_args(args, live, scope, ctx, env);
            check_new_target(target, live, scope, ctx, env)
        }
        ExprKind::Clone(inner) => check_expr(inner, None, live, scope, ctx, env),
        ExprKind::Fn(_) => env.interner.callable(),
        ExprKind::Match { subject, arms } => {
            check_expr(subject, None, live, scope, ctx, env);
            for arm in arms {
                if let Some(conds) = &arm.conditions {
                    for c in conds {
                        check_expr(c, None, live, scope, ctx, env);
                    }
                }
                check_expr(&arm.body, None, live, scope, ctx, env);
            }
            env.interner.mixed()
        }
        ExprKind::Yield { key, value } => {
            if let Some(k) = key {
                check_expr(k, None, live, scope, ctx, env);
            }
            if let Some(v) = value {
                check_expr(v, None, live, scope, ctx, env);
            }
            env.interner.mixed()
        }
        ExprKind::YieldFrom(inner) => {
            check_expr(inner, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::Print(inner) => {
            check_expr(inner, None, live, scope, ctx, env);
            env.interner.int()
        }
        ExprKind::Throw(inner) => {
            check_expr(inner, None, live, scope, ctx, env);
            env.interner.never()
        }
        ExprKind::Isset(_) | ExprKind::Empty(_) => env.interner.bool_ty(),
        ExprKind::ExitOrDie(opt) => {
            if let Some(e) = opt {
                check_expr(e, None, live, scope, ctx, env);
            }
            env.interner.never()
        }
        ExprKind::SpawnScript { path, options } => {
            check_expr(path, None, live, scope, ctx, env);
            for opt in options {
                check_expr(&opt.value, None, live, scope, ctx, env);
            }
            env.interner.mixed()
        }
        ExprKind::Require { path } => {
            check_expr(path, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        ExprKind::Paren(inner) => check_expr(inner, expected, live, scope, ctx, env),
        ExprKind::Error => env.interner.mixed(),
        _ => env.interner.mixed(),
    }
}

fn class_of_ctx(ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    match ctx.current_class {
        Some(qname) => env.interner.class(qname.clone()),
        None => env.interner.mixed(),
    }
}

fn check_read(
    name: &str,
    span: Span,
    live: &FxHashSet<String>,
    scope: &LocalScope,
    env: &mut Env<'_>,
) -> TypeId {
    match scope.by_name.get(name) {
        Some(info) if live.contains(name) => info.ty,
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

fn check_assign(
    op: AssignOp,
    target: &Expr,
    value: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    if let (AssignOp::Assign, ExprKind::Variable(span)) = (op, &target.kind) {
        let name = strip_sigil(span_text(env.src, *span)).to_owned();
        let declared = scope.by_name.get(&name).map(|info| info.ty);
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
        check_expr(value, Some(target_ty), live, scope, ctx, env);
        target_ty
    }
}

fn check_array_literal(
    items: &[ArrayItem],
    expected: Option<TypeId>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let elem_expected = expected.and_then(|id| match env.interner.get(id) {
        Ty::Array(elem) => Some(*elem),
        _ => None,
    });
    for item in items {
        if let Some(key) = &item.key {
            check_expr(key, None, live, scope, ctx, env);
        }
        check_expr(&item.value, elem_expected, live, scope, ctx, env);
    }
    match (expected, elem_expected) {
        (Some(id), Some(_)) => id,
        _ => {
            let mixed = env.interner.mixed();
            env.interner.array(mixed)
        }
    }
}

fn check_new_target(
    target: &NewTarget,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    match target {
        NewTarget::Name(name) => {
            let text = span_text(env.src, name.span);
            let qname = mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports);
            if env.symbols.get(&qname).is_some() || qname.is_core() {
                env.interner.class(qname)
            } else {
                env.interner.mixed()
            }
        }
        NewTarget::SelfTy | NewTarget::StaticTy => class_of_ctx(ctx, env),
        NewTarget::ParentTy => env.interner.mixed(),
        NewTarget::Expr(e) => {
            check_expr(e, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        NewTarget::AnonClass(_) => env.interner.mixed(),
        _ => env.interner.mixed(),
    }
}

fn check_member_name(
    member: &MemberName,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if let MemberName::Variable(e) | MemberName::Expr(e) = member {
        check_expr(e, None, live, scope, ctx, env);
    }
}

fn check_args(
    args: &CallArgs,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let CallArgs::List(list) = args else {
        return;
    };
    for Arg { value, .. } in list {
        check_expr(value, None, live, scope, ctx, env);
    }
}

fn cast_result_type(ty: CastType, env: &mut Env<'_>) -> TypeId {
    match ty {
        CastType::Int => env.interner.int(),
        CastType::Uint => env.interner.uint(),
        CastType::Float => env.interner.float(),
        CastType::String => env.interner.string(),
        CastType::Bool => env.interner.bool_ty(),
        CastType::Array => {
            let mixed = env.interner.mixed();
            env.interner.array(mixed)
        }
        CastType::Object => env.interner.object(),
    }
}

/// The binary-operator result-type table, ADR 0007 § 4. Only `int`/`uint`/
/// `float` operands are modeled this slice — anything else (a class,
/// `mixed`, an unresolved call result) falls back to `mixed` rather than
/// diagnosing, since neither `Comparable` (ADR 0013) nor a general operator-
/// overload rule is implemented yet.
fn binary_result(op: BinaryOp, lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    match op {
        BinaryOp::Concat => env.interner.string(),
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Pow | BinaryOp::Mod => {
            arithmetic_result(lhs, rhs, span, env)
        }
        BinaryOp::Div => division_result(lhs, rhs, span, env),
        BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
            bitwise_result(lhs, rhs, span, env)
        }
        BinaryOp::Cmp => env.interner.int(),
        BinaryOp::Eq
        | BinaryOp::NotEq
        | BinaryOp::Identical
        | BinaryOp::NotIdentical
        | BinaryOp::Lt
        | BinaryOp::LtEq
        | BinaryOp::Gt
        | BinaryOp::GtEq
        | BinaryOp::And
        | BinaryOp::Or
        | BinaryOp::LowAnd
        | BinaryOp::LowOr
        | BinaryOp::LowXor => env.interner.bool_ty(),
        BinaryOp::Coalesce => env.interner.make_union([lhs, rhs]),
        _ => env.interner.mixed(),
    }
}

fn arithmetic_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
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

fn division_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
    match (env.interner.get(lhs).clone(), env.interner.get(rhs).clone()) {
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

fn bitwise_result(lhs: TypeId, rhs: TypeId, span: Span, env: &mut Env<'_>) -> TypeId {
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

fn report_int_uint(span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_INT_UINT_ARITHMETIC,
            "`int` and `uint` have no representable common type in arithmetic",
        )
        .with_primary(span, "mixed-signedness operand")
        .with_help("convert one side explicitly with `as int`/`as uint`"),
    );
}
