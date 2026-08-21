//! A minimal bidirectional expression checker.
//!
//! [`check_expr`] takes an optional expected type: an [`mwl_syntax::ast::ExprKind::ArrayLiteral`]
//! checked against an `array<T>` target checks every element directly
//! against `T` (ADR 0007 § 5 — "never inferred and then compared"); anything
//! else infers its type bottom-up and, when an expected type was given,
//! reports `E_TYPE_MISMATCH` on a mismatch via [`is_assignable`].
//!
//! Beyond literals, variable reads, the binary-operator result-type table
//! (ADR 0007 § 4, including refusing `int ⊕ uint`), `as`/cast conversions
//! and array literals, a property access, method call, static call/property,
//! `new` and `match`/ternary are now typed too — see [`class_qname_of`] and
//! its callers below. A method/static call not statically resolvable to a
//! known signature (an unresolved receiver, a dynamic member name, a
//! `Core`-namespaced target with no modeled stdlib signature) still falls
//! back to `mixed` with no diagnostic, same as everywhere else this checker
//! only reports what it can be sure of. A closure's body is the one
//! remaining form walked only for nested variable reads and reported as
//! `mixed`; see the crate docs' known gaps for why.
//!
//! ADR 0013's `Comparable` check for the five ordering operators has a
//! sibling now: [`require_stringable`] refuses an object at every implicit
//! string-conversion site (interpolation, concatenation, `echo`/`print`,
//! `as string`/`(string)`) unless it provably implements the reserved
//! global `Stringable` interface (ADR 0028 § 1), and `unset()` on any
//! *declared* object property is refused outright regardless of nullability
//! (ADR 0028 § 3, via [`check_unset_target`]/[`check_property_access`]).
//!
//! **Diagnosing a missing member is split by receiver, not duplicated:** a
//! `self::`/`static::`/`parent::`/explicit-class-name static call, static
//! property, or class constant is already checked for existence by
//! `mwl_hir::members`, so this module only recovers its *type* there and adds
//! no second diagnostic. A `$this->prop` property access is the same story
//! (`mwl_hir::members` already reports `E_UNDEFINED_PROPERTY` for it). Every
//! other receiver shape — an instance method call regardless of receiver, and
//! a property access on anything but `$this` — has never been checked by
//! `mwl_hir` at all (it has no static type to check against), so this module
//! reports `E_UNKNOWN_MEMBER` for those directly.
//!
//! `isset(...)`/`empty(...)` are a deliberate exception: PHP tolerates an
//! unset operand there by design, and whether that still holds once every
//! local is declared and flow-checked is an open language question beyond
//! this slice, so their operands are left entirely unchecked rather than
//! guessed at.

use mwl_diagnostics::{Diagnostic, Span, code};
use mwl_hir::QName;
use mwl_syntax::ast::{
    Arg, ArrayItem, AssignOp, BinaryOp, CallArgs, CastType, Expr, ExprKind, MemberName, NewTarget,
    StringPart, UnaryOp,
};
use rustc_hash::FxHashSet;

use crate::locals::LocalScope;
use crate::lower::lower_type;
use crate::signatures::{MethodSig, resolve_method, resolve_property};
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
                    let ty = check_expr(e, None, live, scope, ctx, env);
                    require_stringable(ty, e.span, env);
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
            let inner_ty = check_expr(inner, None, live, scope, ctx, env);
            if *ty == CastType::String {
                require_stringable(inner_ty, inner.span, env);
            }
            cast_result_type(*ty, env)
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let lhs_ty = check_expr(lhs, None, live, scope, ctx, env);
            let rhs_ty = check_expr(rhs, None, live, scope, ctx, env);
            if *op == BinaryOp::Concat {
                require_stringable(lhs_ty, lhs.span, env);
                require_stringable(rhs_ty, rhs.span, env);
            }
            binary_result(*op, lhs_ty, rhs_ty, expr.span, env)
        }
        ExprKind::Assign {
            op, target, value, ..
        } => check_assign(*op, target, value, live, scope, ctx, env),
        ExprKind::Ternary { cond, then, else_ } => {
            let cond_ty = check_expr(cond, None, live, scope, ctx, env);
            // `$a ?: $b` (`then` omitted) evaluates to `$a` itself on the
            // truthy path — its type joins the union the same way an
            // explicit `then` branch would.
            let then_ty = match then {
                Some(then) => check_expr(then, None, live, scope, ctx, env),
                None => cond_ty,
            };
            let else_ty = check_expr(else_, None, live, scope, ctx, env);
            env.interner.make_union([then_ty, else_ty])
        }
        ExprKind::Conversion { expr: inner, ty } => {
            let inner_ty = check_expr(inner, None, live, scope, ctx, env);
            let result = lower_type(ty, ctx, env);
            if matches!(env.interner.get(result), Ty::String) {
                require_stringable(inner_ty, inner.span, env);
            }
            result
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
            let object_ty = check_expr(object, None, live, scope, ctx, env);
            check_member_name(method, live, scope, ctx, env);
            // Unlike a static call, `mwl_hir::members` never checks an
            // instance method call's existence for any receiver — including
            // `$this` — so this is the first and only place it's diagnosed.
            let sig = match (class_qname_of(object_ty, env.interner), method) {
                (Some(qname), MemberName::Ident(name_span)) => {
                    let name = span_text(env.src, *name_span).to_owned();
                    let found = resolve_method(&qname, &name, env.signatures, env.graph);
                    if found.is_none() && !qname.is_core() {
                        report_unknown_member(object.span, &qname, &name, "method", env);
                    }
                    found
                }
                _ => None,
            };
            check_args_typed(args, sig.as_ref(), expr.span, live, scope, ctx, env);
            sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty)
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
        } => {
            check_expr(class, None, live, scope, ctx, env);
            check_member_name(method, live, scope, ctx, env);
            // `mwl_hir::members` already checks this reference's existence
            // (`self::`/`static::`/`parent::`/an explicit class name) — this
            // only recovers the call's *type* when a signature resolves, and
            // adds no second diagnostic when it doesn't.
            let sig = match method {
                MemberName::Ident(name_span) => {
                    resolve_class_expr(class, ctx, env).and_then(|qname| {
                        let name = span_text(env.src, *name_span).to_owned();
                        resolve_method(&qname, &name, env.signatures, env.graph)
                    })
                }
                _ => None,
            };
            check_args_typed(args, sig.as_ref(), expr.span, live, scope, ctx, env);
            sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty)
        }
        ExprKind::PropertyAccess {
            object, property, ..
        } => check_property_access(object, property, false, live, scope, ctx, env),
        ExprKind::StaticPropertyAccess { class, name } => {
            check_expr(class, None, live, scope, ctx, env);
            let text = span_text(env.src, *name);
            let prop_name = strip_sigil(text).to_owned();
            resolve_class_expr(class, ctx, env)
                .and_then(|qname| resolve_property(&qname, &prop_name, env.signatures, env.graph))
                .unwrap_or_else(|| env.interner.mixed())
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
            let target_ty = check_new_target(target, live, scope, ctx, env);
            // A class with no explicit `constructor` accepts a bare `new
            // Foo()` in PHP; not diagnosing an arity mismatch against zero
            // parameters here is deliberate — see the crate docs' known gaps.
            let sig = class_qname_of(target_ty, env.interner)
                .and_then(|qname| resolve_method(&qname, "constructor", env.signatures, env.graph));
            check_args_typed(args, sig.as_ref(), expr.span, live, scope, ctx, env);
            target_ty
        }
        ExprKind::Clone(inner) => check_expr(inner, None, live, scope, ctx, env),
        ExprKind::Fn(_) => env.interner.callable(),
        ExprKind::Match { subject, arms } => {
            check_expr(subject, None, live, scope, ctx, env);
            let mut arm_types = Vec::with_capacity(arms.len());
            for arm in arms {
                if let Some(conds) = &arm.conditions {
                    for c in conds {
                        check_expr(c, None, live, scope, ctx, env);
                    }
                }
                arm_types.push(check_expr(&arm.body, None, live, scope, ctx, env));
            }
            if arm_types.is_empty() {
                env.interner.mixed()
            } else {
                env.interner.make_union(arm_types)
            }
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
            let ty = check_expr(inner, None, live, scope, ctx, env);
            require_stringable(ty, inner.span, env);
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

pub(crate) fn class_of_ctx(ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
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
        NewTarget::ParentTy => {
            let parent = ctx
                .current_class
                .and_then(|c| env.graph.get(c))
                .and_then(|links| links.extends.first())
                .cloned();
            match parent {
                Some(parent) => env.interner.class(parent),
                None => env.interner.mixed(),
            }
        }
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

/// The class or enum a resolved type names, if it names one at all — the
/// receiver-type question every member-access/call arm below needs answered
/// before it can look anything up in a [`crate::signatures::SignatureTable`].
fn class_qname_of(ty: TypeId, interner: &TypeInterner) -> Option<QName> {
    match interner.get(ty) {
        Ty::Class(q) | Ty::Enum(q) => Some(q.clone()),
        _ => None,
    }
}

/// Whether `object` is exactly the `$this` variable — the one receiver shape
/// `mwl_hir::members` already diagnoses a missing property on, so
/// [`infer`]'s `PropertyAccess` arm must not diagnose it a second time.
pub(crate) fn is_this_receiver(object: &Expr, src: &mwl_diagnostics::SourceFile) -> bool {
    matches!(&object.kind, ExprKind::Variable(span) if span_text(src, *span) == "$this")
}

/// Resolves a `Class::…`-side expression to the class it names, the same way
/// `mwl_hir::members::check_member_ref` does for existence checking:
/// `self`/`static` against the enclosing class, `parent` against its first
/// `extends` link, an explicit name via the same unqualified/qualified/
/// fully-qualified lookup every resolver in this codebase shares. A dynamic
/// class side (a variable, a parenthesized expression, ...) has no statically
/// knowable class and resolves to `None` — callers fall back to `mixed` with
/// no diagnostic, matching `mwl_hir::members`'s own silent skip for the same
/// shape.
fn resolve_class_expr(class_expr: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<QName> {
    match &class_expr.kind {
        ExprKind::SelfExpr | ExprKind::StaticExpr => ctx.current_class.cloned(),
        ExprKind::ParentExpr => {
            let current = ctx.current_class?;
            env.graph.get(current)?.extends.first().cloned()
        }
        ExprKind::ConstFetch(name) => {
            let text = span_text(env.src, name.span);
            Some(mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports))
        }
        _ => None,
    }
}

/// Shared body for a property access, whether it appears as an ordinary
/// expression (`$obj->prop`, `is_unset` false) or as `unset()`'s operand
/// (`is_unset` true) — the receiver/member resolution is identical either
/// way; only what happens once a *declared* property is found differs (ADR
/// 0028 § 3: `unset()` on one is refused outright, per ADR 0022's guarantee
/// that a declared property can never become uninitialized again).
fn check_property_access(
    object: &Expr,
    property: &MemberName,
    is_unset: bool,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let object_ty = check_expr(object, None, live, scope, ctx, env);
    check_member_name(property, live, scope, ctx, env);
    match (class_qname_of(object_ty, env.interner), property) {
        (Some(qname), MemberName::Ident(name_span)) => {
            let name = span_text(env.src, *name_span).to_owned();
            match resolve_property(&qname, &name, env.signatures, env.graph) {
                Some(ty) => {
                    if is_unset {
                        report_unset_on_property(object.span.to(*name_span), &qname, &name, env);
                    }
                    ty
                }
                None => {
                    // `$this->missing` is already `E_UNDEFINED_PROPERTY`
                    // from `mwl_hir::members` — every other receiver
                    // shape has never been checked before this.
                    if !qname.is_core() && !is_this_receiver(object, env.src) {
                        report_unknown_member(object.span, &qname, &name, "property", env);
                    }
                    env.interner.mixed()
                }
            }
        }
        _ => env.interner.mixed(),
    }
}

/// `unset()`'s operand: refuses a declared object property (ADR 0028 § 3)
/// via [`check_property_access`], and otherwise checks the operand exactly
/// like any other expression — an array element or a local variable is
/// untouched, since that section is scoped to object properties only.
pub(crate) fn check_unset_target(
    expr: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if let ExprKind::PropertyAccess {
        object, property, ..
    } = &expr.kind
    {
        check_property_access(object, property, true, live, scope, ctx, env);
    } else {
        check_expr(expr, None, live, scope, ctx, env);
    }
}

fn report_unset_on_property(span: Span, qname: &QName, name: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_UNSET_ON_PROPERTY,
            format!(
                "`unset()` on `{qname}::${name}` is refused; a declared property can never \
                 become uninitialized again"
            ),
        )
        .with_primary(span, "unset here")
        .with_help(
            "ADR 0022 already guarantees this property is always definitely initialized; \
             assign `null` instead if it is nullable",
        ),
    );
}

/// ADR 0028 § 1: every implicit string-conversion site — interpolation,
/// concatenation, `echo`/`print`, `as string`/`(string)` — accepts an object
/// only when its static type provably implements the reserved global
/// `Stringable` interface. Returns without diagnosing for any non-`Ty::Class`
/// operand (including `Ty::Enum`, `mixed`, and a scalar) and for an
/// unmodeled `Core` class, the same scoping [`object_comparison_result`] and
/// [`check_property_access`] already use.
pub(crate) fn require_stringable(ty: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname) = env.interner.get(ty).clone() else {
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

/// Reports `E_UNKNOWN_MEMBER` for a property/method access this module
/// resolved a receiver class for, but found nothing declared under `name` on
/// it or any ancestor.
fn report_unknown_member(span: Span, qname: &QName, name: &str, kind: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_UNKNOWN_MEMBER,
            format!("`{qname}` has no {kind} named `{name}`"),
        )
        .with_primary(span, "referenced here"),
    );
}

/// Checks a call's arguments against a resolved [`MethodSig`], when one was
/// found: reports `E_ARITY_MISMATCH` for a wrong non-variadic argument count,
/// then checks each positional argument against its parameter's type the
/// same way an ordinary assignment is checked. Falls back to the old
/// "just walk nested expressions, `mixed` throughout" behaviour when no
/// signature resolved, and also when any argument is named or spread — PHP's
/// named/variadic call resolution isn't a straight positional mapping, and
/// modeling that is out of scope for this slice.
fn check_args_typed(
    args: &CallArgs,
    sig: Option<&MethodSig>,
    call_span: Span,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let CallArgs::List(list) = args else {
        return;
    };
    let Some(sig) = sig else {
        for Arg { value, .. } in list {
            check_expr(value, None, live, scope, ctx, env);
        }
        return;
    };
    if list.iter().any(|a| a.name.is_some() || a.spread) {
        for Arg { value, .. } in list {
            check_expr(value, None, live, scope, ctx, env);
        }
        return;
    }
    if !sig.variadic && list.len() != sig.params.len() {
        env.diags.report(
            Diagnostic::error(
                code::E_ARITY_MISMATCH,
                format!(
                    "expected {} argument(s), found {}",
                    sig.params.len(),
                    list.len()
                ),
            )
            .with_primary(call_span, "called here"),
        );
    }
    let last_param_index = sig.params.len().saturating_sub(1);
    for (i, arg) in list.iter().enumerate() {
        let expected = if sig.variadic && i >= last_param_index {
            sig.params.last().copied()
        } else {
            sig.params.get(i).copied()
        };
        check_expr(&arg.value, expected, live, scope, ctx, env);
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

/// The binary-operator result-type table, ADR 0007 § 4, amended by ADR 0013
/// § 6 for `< <= > >= <=>` when both operands are objects. Beyond that one
/// amendment, only `int`/`uint`/`float` operands are modeled this slice —
/// anything else (`mixed`, an unresolved call result) falls back to `mixed`
/// rather than diagnosing, since no general operator-overload rule is
/// implemented yet.
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
        | BinaryOp::Or
        | BinaryOp::LowAnd
        | BinaryOp::LowOr
        | BinaryOp::LowXor => env.interner.bool_ty(),
        BinaryOp::Coalesce => env.interner.make_union([lhs, rhs]),
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
fn object_comparison_result(
    op: BinaryOp,
    lhs: TypeId,
    rhs: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let (Ty::Class(lhs_q), Ty::Class(rhs_q)) =
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
    Some(match op {
        BinaryOp::Cmp => env.interner.int(),
        _ => env.interner.bool_ty(),
    })
}

fn report_comparable_diagnostic(span: Span, message: String, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(code::E_COMPARISON_REQUIRES_COMPARABLE, message)
            .with_primary(span, "compared here")
            .with_help("implement `Comparable`'s `compareTo(self $other): int` on the class"),
    );
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
