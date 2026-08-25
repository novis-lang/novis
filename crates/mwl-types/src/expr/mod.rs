//! A minimal bidirectional expression checker: the dispatch, and the modules
//! each rule lives in.
//!
//! [`check_expr`] takes an optional expected type: an
//! [`mwl_syntax::ast::ExprKind::ArrayLiteral`] checked against an `array<T>`
//! target checks every element directly against `T` (ADR 0007 § 5 — "never
//! inferred and then compared"); anything else infers its type bottom-up and,
//! when an expected type was given, reports `E_TYPE_MISMATCH` on a mismatch
//! via [`is_assignable`].
//!
//! # This file is the dispatch, and the rules live elsewhere
//!
//! [`infer`] is one arm per AST expression variant. An arm that is a couple of
//! lines stays inline; an arm carrying a rule of its own is one call into the
//! module that owns that rule. **That is this file's whole charter**, and it
//! is what the directory exists to hold: `expr.rs` reached 3.5k lines by
//! growing every new rule into the same file, and a `mod.rs` that accepts
//! "just this one helper" is that file again within a week.
//!
//! | module | owns |
//! |---|---|
//! | [`args`] | a call's arguments, its options bag, its type arguments |
//! | [`assign`] | ADR 0007 § 6's assignability, and the positions applying it |
//! | [`calls`] | which member a call resolves to; ADR 0027's `callable` |
//! | [`iteration`] | ADR 0053's `foreach` sources and `yield` forms |
//! | [`literals`] | how a literal takes its type from its position |
//! | [`members`] | a property, class constant or enum case, and who diagnoses it |
//! | [`operators`] | ADR 0007 § 4's result table and the refusals layered on it |
//! | [`quals`] | ADR 0024's `tainted`, ADR 0033's `secret`, and their sinks |
//!
//! Two fallbacks are deliberate and belong to no module. A method/static call
//! not statically resolvable to a known signature — an unresolved receiver, a
//! dynamic member name, a `Core`-namespaced target with no modeled stdlib
//! signature — types as `mixed` with no diagnostic, the same way this checker
//! only ever reports what it can be sure of. `isset(...)`/`empty(...)` go
//! further: PHP tolerates an unset operand there by design, and whether that
//! still holds once every local is declared and flow-checked is an open
//! language question beyond this slice, so their operands are left entirely
//! unchecked rather than guessed at.

use mwl_diagnostics::{Diagnostic, SourceFile, Span, code};
use mwl_hir::{ClassGraph, QName, SymbolKind};
use mwl_syntax::ast::{
    Arg, ArrayItem, AssignOp, BinaryOp, CallArgs, Expr, ExprKind, FnBody, FnExpr, ForeachBinding,
    MemberName, NewTarget, StringPart, Type, TypeKind, UnaryOp,
};
use rustc_hash::FxHashSet;

use crate::expr_table::{ExprInfo, ForeachDrive, ResolvedCall};
use crate::locals::{Captures, LocalScope, check_block};
use crate::lower::{lower_optional_type, lower_type};
use crate::signatures::{
    MethodSig, SignatureTable, resolve_method, resolve_property, resolve_property_owned,
};
use crate::ty::{Ty, TypeId, TypeInterner};
use crate::{Ctx, Env, span_text, strip_sigil};

// One expression checker split across this directory — see each module's own
// header for what it owns. A rule reaches its neighbours as `pub(super)`,
// which is the reach it had when `expr` was a single file, and no further.
mod args;
mod assign;
mod calls;
mod iteration;
mod literals;
mod members;
mod operators;
mod quals;

use self::{
    args::*, assign::*, calls::*, iteration::*, literals::*, members::*, operators::*, quals::*,
};

// What the rest of the crate reaches through `crate::expr::…`, unchanged by
// the split: the same names, at the same path, whichever module now holds
// them.
pub(crate) use self::{
    assign::{check_return, is_assignable},
    iteration::{check_foreach_key, check_foreach_value, foreach_source},
    literals::int_literal_digits,
    members::{check_unset_target, is_this_receiver},
    operators::{reject_disjoint_equality, require_stringable},
};

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
    if let Some(expected_id) = expected {
        let wants_callable = matches!(env.interner.get(expected_id), Ty::Callable);
        if wants_callable && report_non_callable_value_if_applicable(expr, env) {
            return actual;
        }
        if !is_assignable(actual, expected_id, env.interner, env.graph, env.signatures) {
            report_mismatch(expr.span, expected_id, actual, env);
        }
    }
    actual
}

/// The dispatch: one arm per AST expression variant, each either a couple of
/// lines or a single call into the module that owns its rule.
///
/// `pub(super)` for one caller: [`super::operators::infer_conversion`] needs
/// the *placing* walk rather than [`check_expr`]'s conforming one, so a numeric
/// literal written directly under an `as` takes the target as its expectation
/// (ADR 0054 § 2).
#[expect(
    clippy::too_many_lines,
    reason = "one match arm per AST expression variant, each a couple of lines"
)]
pub(super) fn infer(
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
        ExprKind::Int(span) => infer_int_literal(*span, expr.span, expected, env),
        ExprKind::Float(span) => infer_float_literal(*span, expr.span, expected, env),
        // ADR 0070 § 2: a duration literal is `Core\Time\Duration` and nothing
        // places it — the suffix *is* the type, unlike ADR 0054's fractional
        // literal just above. The lexer has already run the grammar and
        // reported anything wrong, so there is nothing left to check here.
        ExprKind::Duration(_) => env
            .interner
            .class(QName::parse(mwl_stdlib::time::DURATION_NAME)),
        ExprKind::Str(span) => infer_str_literal(*span, expected, env),
        ExprKind::Interpolated(parts) => infer_interpolated(expr, parts, live, scope, ctx, env),
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
        // ADR 0036 § 2: each field's type is inferred from its own
        // initializer (same idea as an `array<T>` literal's element type),
        // and the literal's precise type is the exact-fields shape those
        // infer to — `is_assignable`'s width subtyping is what lets it flow
        // into a narrower shape or plain `object` target on its own.
        ExprKind::ObjectLiteral(fields) => {
            let mut out = Vec::with_capacity(fields.len());
            for field in fields {
                let name = span_text(env.src, field.name).to_owned();
                let field_ty = check_expr(&field.value, None, live, scope, ctx, env);
                out.push((name, field_ty));
            }
            env.interner.shape(out)
        }
        ExprKind::Unary { op, expr: inner } => {
            // `infer`, not `check_expr`: the operand inherits an *expectation*
            // rather than a position it has to satisfy, so a `-$n` under a
            // `-1` target is still an ordinary mismatch reported once, at the
            // negation, rather than twice.
            let hint = negated_literal_expectation(*op, expected, env.interner);
            let inner_ty = infer(inner, hint, live, scope, ctx, env);
            match op {
                UnaryOp::Not => env.interner.bool_ty(),
                UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitNot => {
                    reject_arithmetic_on_object(*op, inner_ty, expr.span, env);
                    negated_literal_result(*op, inner_ty, env.interner)
                }
                UnaryOp::Suppress => inner_ty,
                _ => inner_ty,
            }
        }
        ExprKind::PreIncDec { expr: inner, .. } | ExprKind::PostIncDec { expr: inner, .. } => {
            note_write(inner, scope, env);
            check_expr(inner, None, live, scope, ctx, env)
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
        } => check_assign(*op, expr.span, target, value, live, scope, ctx, env),
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
            infer_conversion(expr, inner, ty, live, scope, ctx, env)
        }
        ExprKind::InstanceOf { expr: inner, class } => {
            infer_instanceof(expr, inner, class, live, scope, ctx, env)
        }
        ExprKind::Call { callee, args } => {
            let callee_ty = check_expr(callee, None, live, scope, ctx, env);
            check_args(args, live, scope, ctx, env);
            if matches!(args, CallArgs::FirstClassCallable) {
                return env.interner.callable();
            }
            report_call_on_non_callable(callee_ty, expr.span, env);
            env.interner.mixed()
        }
        ExprKind::MethodCall {
            object,
            method,
            nullsafe,
            type_args,
            args,
        } => infer_method_call(
            expr, object, method, *nullsafe, type_args, args, live, scope, ctx, env,
        ),
        ExprKind::StaticCall {
            class,
            method,
            type_args,
            args,
        } => infer_static_call(expr, class, method, type_args, args, live, scope, ctx, env),
        ExprKind::PropertyAccess {
            object,
            property,
            nullsafe,
        } => check_property_access(object, property, *nullsafe, false, live, scope, ctx, env),
        ExprKind::StaticPropertyAccess { class, name } => {
            check_expr(class, None, live, scope, ctx, env);
            let text = span_text(env.src, *name);
            let prop_name = strip_sigil(text).to_owned();
            // Resolved through the *owning* class so ADR 0094's level test
            // reaches the static spelling too — `Foo::$secret` is the same
            // access as `$foo->secret` with the receiver written as a name.
            let resolved = resolve_class_expr(class, ctx, env).and_then(|qname| {
                resolve_property_owned(&qname, &prop_name, env.signatures, env.graph)
            });
            match resolved {
                Some((owner, ty)) => {
                    check_member_visibility(
                        expr.span,
                        &owner,
                        &format!("${prop_name}"),
                        crate::signatures::property_visibility(&owner, &prop_name, env.signatures),
                        ctx,
                        env,
                    );
                    ty
                }
                None => env.interner.mixed(),
            }
        }
        ExprKind::ClassConstAccess { class, name } => {
            infer_class_const(expr, class, *name, expected, live, scope, ctx, env)
        }
        ExprKind::ClassNameConst { class } => {
            check_expr(class, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        // `mwl-ir` needs the element's declared type to lower an eventual
        // indexed read/write instruction — see `crate::expr_table`'s own
        // module docs. Recorded only when `base_ty` statically resolved to a
        // known `Ty::Array` element type, never when it erased to `mixed`
        // (an untyped/unresolved array) — the same "nothing compile-time-
        // known to read" split `check_property_access` already draws for a
        // shape/plain-`object` receiver. `check_assign`'s general (non-plain-
        // local) arm routes an assignment target back through this same
        // function, so a write records exactly the entry a read would, keyed
        // by this `Index` expression's own span either way.
        ExprKind::Index { base, index } => {
            let base_ty = check_expr(base, None, live, scope, ctx, env);
            if let Some(index) = index {
                let index_ty = check_expr(index, None, live, scope, ctx, env);
                check_array_key_type(index_ty, index.span, env);
            }
            let elem_ty = match env.interner.get(base_ty) {
                Ty::Array(elem) => Some(*elem),
                _ => None,
            };
            match elem_ty {
                Some(elem_ty) => {
                    env.exprs.record(expr.span, ExprInfo::Index { elem_ty });
                    elem_ty
                }
                None => env.interner.mixed(),
            }
        }
        ExprKind::New {
            target,
            type_args,
            args,
        } => infer_new(expr, target, type_args, args, live, scope, ctx, env),
        ExprKind::Clone(inner) => check_expr(inner, None, live, scope, ctx, env),
        ExprKind::Fn(fn_expr) => check_fn_literal(expr, fn_expr, live, scope, ctx, env),
        ExprKind::Match { subject, arms } => {
            let subject_ty = check_expr(subject, None, live, scope, ctx, env);
            let mut arm_types = Vec::with_capacity(arms.len());
            for arm in arms {
                if let Some(conds) = &arm.conditions {
                    for c in conds {
                        // ADR 0090 § 6: an arm is compared against the subject
                        // by the one equality rule, so a disjoint arm is § 2's
                        // refusal written without the operator. `match (true)`
                        // is unaffected — every arm there is a `bool` too.
                        let cond_ty = check_expr(c, None, live, scope, ctx, env);
                        reject_disjoint_equality(subject_ty, cond_ty, c.span, env);
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
        ExprKind::Yield { key, value } => infer_yield(
            expr,
            key.as_deref(),
            value.as_deref(),
            live,
            scope,
            ctx,
            env,
        ),
        ExprKind::YieldFrom(inner) => infer_yield_from(expr, inner, live, scope, ctx, env),
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
        ExprKind::Exit(opt) => {
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

/// `self`/`static`/`$this`'s type, resolved against the enclosing
/// declaration. ADR 0010: an enum has no methods to reach this from in a
/// well-formed program, but the parser still recovers a member it rejected
/// with `E_ENUM_MEMBER_UNSUPPORTED` (see `mwl-syntax::parser::parse_enum_body`)
/// and hands it to this checker anyway — so this must resolve the same way
/// [`crate::lower::lower_type`]'s `self`/`static` atom already does, an
/// enum-declared `qname` interning to `Ty::Enum` rather than `Ty::Class`.
pub(crate) fn class_of_ctx(ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    match ctx.current_class {
        Some(qname) => match env.symbols.get(qname) {
            Some(sym) if sym.kind == SymbolKind::Enum => {
                let backing = env.enums.backing_of(qname);
                env.interner.enum_(qname.clone(), backing)
            }
            _ => env.interner.class(qname.clone()),
        },
        None => env.interner.mixed(),
    }
}
