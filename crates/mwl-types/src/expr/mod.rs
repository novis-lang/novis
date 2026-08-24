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
    MemberName, NewTarget, StringPart, Type, UnaryOp,
};
use rustc_hash::FxHashSet;

use crate::expr_table::{ExprInfo, ForeachDrive, ResolvedCall};
use crate::locals::{Captures, LocalScope, check_block};
use crate::lower::{lower_optional_type, lower_type};
use crate::signatures::{MethodSig, SignatureTable, resolve_method, resolve_property};
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
    operators::require_stringable,
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
        // ADR 0007 § 4: "An integer literal that does not fit `int` is legal
        // only where a `uint` is expected, and is otherwise a diagnostic
        // saying exactly that." The literal's own digits are never negative —
        // a leading `-` is a separate, wrapping `ExprKind::Unary` node (see
        // that arm below), which already produces an ordinary `int`/`uint`
        // type mismatch on its own when negated and assigned into a `uint`
        // target, with no magnitude check needed for that half. What *does*
        // need one: whether the bare digit run fits `int`'s `0..=i64::MAX`
        // half, `uint`'s full `0..=u64::MAX` range, or neither at all.
        ExprKind::Int(span) if wants_decimal(expected, env) => {
            // ADR 0054 §§ 3-4: an `int`/`uint` is exact in a 96-bit mantissa,
            // so an integer literal placed at `decimal` needs only that wider
            // bound checked — not `int`'s 64-bit one below.
            check_decimal_int_literal(*span, expr.span, env);
            record_decimal_placement(expr.span, env)
        }
        ExprKind::Int(span) => {
            let wants_uint = expected.is_some_and(|id| matches!(env.interner.get(id), Ty::Uint));
            let (radix, digits) = int_literal_digits(env.src, *span);
            match u64::from_str_radix(&digits, radix) {
                Ok(n) if i64::try_from(n).is_ok() => {
                    if wants_uint {
                        env.interner.uint()
                    } else {
                        env.interner.int()
                    }
                }
                Ok(_) if wants_uint => env.interner.uint(),
                Ok(_) => {
                    env.diags.report(
                        Diagnostic::error(
                            code::E_INT_LITERAL_OUT_OF_RANGE,
                            "this integer literal is too large for `int`; it is only legal \
                             where a `uint` is expected",
                        )
                        .with_primary(expr.span, "does not fit `int`"),
                    );
                    env.interner.int()
                }
                Err(_) => {
                    env.diags.report(
                        Diagnostic::error(
                            code::E_INT_LITERAL_OUT_OF_RANGE,
                            "this integer literal is too large to represent in either `int` or \
                             `uint`",
                        )
                        .with_primary(expr.span, "too large for a 64-bit integer"),
                    );
                    if wants_uint {
                        env.interner.uint()
                    } else {
                        env.interner.int()
                    }
                }
            }
        }
        // ADR 0054 § 2: a literal carrying a fractional part or an exponent is
        // untyped until placed, and takes `decimal` or `float` from the type
        // of the position it appears in. `float` is the answer everywhere
        // else, including `var $x = 19.99;`, which has no target at all.
        ExprKind::Float(span) if wants_decimal(expected, env) => {
            check_decimal_float_literal(*span, expr.span, env);
            record_decimal_placement(expr.span, env)
        }
        ExprKind::Float(_) => env.interner.float(),
        // ADR 0070 § 2: a duration literal is `Core\Time\Duration` and nothing
        // places it — the suffix *is* the type, unlike ADR 0054's fractional
        // literal just above. The lexer has already run the grammar and
        // reported anything wrong, so there is nothing left to check here.
        ExprKind::Duration(_) => env
            .interner
            .class(QName::parse(mwl_stdlib::time::DURATION_NAME)),
        ExprKind::Str(span) => {
            // A single-quoted literal's own two escapes (`\\`/`\'`) can
            // never produce invalid UTF-8, so it gets no cooking-diagnostic
            // pass at all. A double-quoted literal runs the richer escape
            // grammar `check_double_quoted_text_issues` cooks. A
            // heredoc/nowdoc-sourced `Str` (whose span opens with `<`, not a
            // quote) runs `crate::string_lit`'s flexible-indentation check
            // first, then the same escape grammar too — unless it's a
            // nowdoc, which (like PHP's) applies no escapes at all.
            let raw = span_text(env.src, *span);
            if raw.starts_with('"') {
                check_double_quoted_text_issues(inner_quoted_span(*span), env);
            } else if raw.starts_with("<<<") {
                let (shape, indent_issues) = crate::string_lit::heredoc_shape(env.src, *span);
                report_heredoc_indent_issues(indent_issues, env);
                let run_escapes = !crate::string_lit::heredoc_is_nowdoc(raw);
                check_heredoc_run_issues(&shape.indent, shape.body, true, true, run_escapes, env);
            }
            env.interner.string()
        }
        ExprKind::Interpolated(parts) => {
            // Only a heredoc/nowdoc can ever reach this arm with the
            // opening `<<<`-only span it needs its own flexible-indentation
            // strip (`mwl_syntax::parser::collapse_string_parts` never
            // produces a nowdoc `Interpolated` at all: a nowdoc has no
            // interpolation syntax by construction, so it always collapses
            // to `ExprKind::Str`, whose arm above already handles it).
            let raw = span_text(env.src, expr.span);
            let is_heredoc = raw.starts_with("<<<");
            let indent = if is_heredoc {
                let (shape, indent_issues) = crate::string_lit::heredoc_shape(env.src, expr.span);
                report_heredoc_indent_issues(indent_issues, env);
                shape.indent
            } else {
                String::new()
            };
            let last_text_idx = is_heredoc
                .then(|| parts.iter().rposition(|p| matches!(p, StringPart::Text(_))))
                .flatten();
            let mut tainted = false;
            let mut secret = false;
            for (i, part) in parts.iter().enumerate() {
                match part {
                    StringPart::Expr(e) => {
                        let ty = check_expr(e, None, live, scope, ctx, env);
                        require_stringable(ty, e.span, env);
                        tainted |= is_tainted(ty, env.interner);
                        secret |= is_secret(ty, env.interner);
                    }
                    // A `Text` run's escapes follow exactly the same grammar
                    // regardless of whether the overall literal is
                    // double-quoted or an interpolated heredoc — see
                    // `crate::string_lit`'s own module docs for why one
                    // routine cooks both. The span never includes a quote
                    // character (`mwl_syntax::parser::parse_string_body`
                    // never emits one as part of a `Text` token), so no
                    // quote-kind check is needed here the way `Str` above
                    // needs one — except a heredoc's own flexible
                    // indentation, which has to be stripped from each run
                    // first (`is_heredoc`'s own doc comment above: this
                    // literal is never a nowdoc, so escapes always run).
                    StringPart::Text(span) => {
                        if is_heredoc {
                            check_heredoc_run_issues(
                                &indent,
                                *span,
                                i == 0,
                                Some(i) == last_text_idx,
                                true,
                                env,
                            );
                        } else {
                            check_double_quoted_text_issues(*span, env);
                        }
                    }
                }
            }
            qualified_scalar(false, tainted, secret, env.interner)
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
            let inner_ty = check_expr(inner, None, live, scope, ctx, env);
            match op {
                UnaryOp::Not => env.interner.bool_ty(),
                UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitNot => {
                    reject_arithmetic_on_object(*op, inner_ty, expr.span, env);
                    inner_ty
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
            let result = lower_type(ty, ctx, env);
            // ADR 0054 § 2: `expr as T` is itself a placing position, so a
            // numeric *literal* written directly under one takes `T` as its
            // target rather than being typed first and converted afterwards.
            // Without this, `19.99 as decimal` would round-trip through an
            // `f64` and lose everything past ~17 digits — § 4's `float →
            // decimal` row — making a wider literal unwritable anywhere that
            // lacks an annotation. Restricted to a literal operand on purpose:
            // any other operand already has a type of its own, and handing it
            // an expectation would silently change what `as` converts *from*.
            // `infer` rather than `check_expr`, because a placement is not an
            // assignment: `1 as string` still places the literal at `string`
            // and still converts, so the conformance check `check_expr` would
            // run here would reject every conversion that does any work.
            let inner_ty = if matches!(inner.kind, ExprKind::Int(_) | ExprKind::Float(_)) {
                infer(inner, Some(result), live, scope, ctx, env)
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
        ExprKind::InstanceOf { expr: inner, class } => {
            check_expr(inner, None, live, scope, ctx, env);
            // A bare `Foo` on the right of `instanceof` is a class name, not a
            // constant read — recorded here so `mwl-ir` never has to resolve
            // one (see `crate::expr_table::ExprInfo::InstanceOf`). Anything
            // else is the dynamic form, which still checks as an ordinary
            // expression and records nothing.
            if let ExprKind::ConstFetch(name) = &class.kind {
                let text = span_text(env.src, name.span);
                let qname = mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports);
                if env.symbols.get(&qname).is_some() || qname.is_reserved_global_class() {
                    env.exprs
                        .record(expr.span, ExprInfo::InstanceOf { class: qname });
                }
            } else {
                check_expr(class, None, live, scope, ctx, env);
            }
            env.interner.bool_ty()
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
        } => {
            let object_ty = check_expr(object, None, live, scope, ctx, env);
            // `?->` never reaches the method when the receiver is `null`, so
            // the method is resolved against the receiver's non-`null` half
            // and the call's own type gains the `null` that arm yields — see
            // [`nullsafe_result`].
            let receiver_ty = strip_nullsafe_receiver(*nullsafe, object_ty, object.span, env);
            check_member_name(method, live, scope, ctx, env);
            // Unlike a static call, `mwl_hir::members` never checks an
            // instance method call's existence for any receiver — including
            // `$this` — so this is the first and only place it's diagnosed.
            let resolved = match (class_qname_of(receiver_ty, env.interner), method) {
                (Some(qname), MemberName::Ident(name_span)) => {
                    let name = span_text(env.src, *name_span).to_owned();
                    let found = resolve_method(&qname, &name, env.signatures, env.graph);
                    if found.is_none() && !qname.is_core() && !qname.is_reserved_global_class() {
                        report_unknown_member(object.span, &qname, &name, "method", env);
                    }
                    if let Some((owner, sig)) = &found {
                        check_interface_private_visibility(owner, &name, sig, *name_span, ctx, env);
                    }
                    // The *declaring* class, not the receiver's: that is what
                    // `ResolvedCall::class` promises, and `mwl-ir` renders the
                    // call's target label from it — `$dog->name()` on a `Dog`
                    // that inherits `name` must name `Animal::name`, the
                    // symbol that actually exists.
                    found.map(|(owner, sig)| (owner, name, sig))
                }
                _ => None,
            };
            let sig = resolved
                .as_ref()
                .map(|(owner, _, sig)| substitute_receiver_args(receiver_ty, owner, sig, env));
            let label = resolved
                .as_ref()
                .map(|(owner, name, _)| format!("{owner}::{name}"));
            let (sig, _written) =
                check_written_type_args(type_args, sig, label.as_deref(), expr.span, ctx, env);
            let (_, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
            // ADR 0027: `$obj->method(...)` (first-class callable syntax)
            // names a `Closure` value, not the method's return type — the
            // sentinel `CallArgs::FirstClassCallable` marks exactly this
            // shape, ahead of the ordinary-call typing below.
            if matches!(args, CallArgs::FirstClassCallable) {
                return env.interner.callable();
            }
            // `mwl-ir` needs this call's resolved target (not just its return
            // type) to lower an eventual instance-call instruction — see
            // `crate::expr_table`'s own module docs.
            // The *substituted* signature, never the one `resolve_method`
            // returned: `crate::generics` guarantees a type variable never
            // survives a call site, and this record is the one thing that
            // carries a signature past it.
            if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig) {
                let call = resolved_call(qname.clone(), name.clone(), sig, env.signatures);
                env.exprs.record(expr.span, ExprInfo::Call(call));
            }
            let returned = sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty);
            nullsafe_result(*nullsafe, object_ty, returned, env)
        }
        ExprKind::StaticCall {
            class,
            method,
            type_args,
            args,
        } => {
            check_expr(class, None, live, scope, ctx, env);
            check_member_name(method, live, scope, ctx, env);
            // `mwl_hir::members` already checks this reference's existence
            // (`self::`/`static::`/`parent::`/an explicit class name) — this
            // only recovers the call's *type* when a signature resolves, and
            // adds no second diagnostic when it doesn't.
            let resolved = match method {
                MemberName::Ident(name_span) => {
                    resolve_class_expr(class, ctx, env).and_then(|qname| {
                        let name = span_text(env.src, *name_span).to_owned();
                        let found = resolve_method(&qname, &name, env.signatures, env.graph).map(
                            |(owner, sig)| {
                                check_interface_private_visibility(
                                    &owner, &name, &sig, *name_span, ctx, env,
                                );
                                // The declaring class — see the `MethodCall`
                                // arm above for why the receiver's own is the
                                // wrong label.
                                (owner, name.clone(), sig)
                            },
                        );
                        // The same narrowing of `Core`'s blanket trust the
                        // `ClassConstAccess` arm below explains: `mwl_hir`
                        // waves every `Core\…::anything` through because
                        // nothing declares it, but `mwl_stdlib::registry`
                        // states every member `Core` has, so a name that is
                        // not one is knowably wrong *here*. Without this a
                        // typo reaches `mwl-ir` as a static call with no
                        // resolved target recorded, which panics.
                        if found.is_none() && qname.is_core() {
                            report_unknown_member(expr.span, &qname, &name, "member", env);
                        }
                        // ADR 0063 R20's one genuinely reachable two-spellings
                        // case — see `report_core_instance_member`.
                        if let Some((owner, _, sig)) = &found
                            && owner.is_core()
                            && !sig.is_static
                        {
                            report_core_instance_member(expr.span, owner, &name, env);
                        }
                        found
                    })
                }
                _ => None,
            };
            let sig = resolved.as_ref().map(|(_, _, sig)| sig.clone());
            let label = resolved
                .as_ref()
                .map(|(owner, name, _)| format!("{owner}::{name}"));
            let (sig, written) =
                check_written_type_args(type_args, sig, label.as_deref(), expr.span, ctx, env);
            let (_, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
            // See the `MethodCall` arm above: first-class callable syntax
            // names a `Closure`, not the resolved method's return type.
            if matches!(args, CallArgs::FirstClassCallable) {
                return env.interner.callable();
            }
            // See the `MethodCall` arm above: persisted for `mwl-ir` to read
            // back a resolved static call's target.
            // The *substituted* signature, never the one `resolve_method`
            // returned: `crate::generics` guarantees a type variable never
            // survives a call site, and this record is the one thing that
            // carries a signature past it.
            if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig) {
                let mut call = resolved_call(qname.clone(), name.clone(), sig, env.signatures);
                // Late static binding: an explicitly named class *sets* the
                // called class, while `self`/`static`/`parent` forward the
                // caller's. See `ResolvedCall::static_class`.
                if matches!(class.kind, ExprKind::ConstFetch(_)) {
                    call.static_class = resolve_class_expr(class, ctx, env);
                }
                call.written_class =
                    written_class_of(qname, name, &written, type_args, expr.span, env);
                env.exprs.record(expr.span, ExprInfo::Call(call));
            }
            sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty)
        }
        ExprKind::PropertyAccess {
            object,
            property,
            nullsafe,
        } => check_property_access(object, property, *nullsafe, false, live, scope, ctx, env),
        ExprKind::StaticPropertyAccess { class, name } => {
            check_expr(class, None, live, scope, ctx, env);
            let text = span_text(env.src, *name);
            let prop_name = strip_sigil(text).to_owned();
            resolve_class_expr(class, ctx, env)
                .and_then(|qname| resolve_property(&qname, &prop_name, env.signatures, env.graph))
                .unwrap_or_else(|| env.interner.mixed())
        }
        // Two shapes of `Class::CONST` are typed precisely, and they split by
        // what the left-hand side names. `EnumName::CaseName` is ADR 0010 § 4's
        // case, recovered as `Ty::Enum`; `Core\Math::PI` is ADR 0011's class
        // constant, recovered as the declared type of the
        // `mwl_stdlib::registry::CoreConst` row. A **user-declared** class's
        // constant is still unmodeled (`mixed`) — see the crate docs' known
        // gaps — because nothing collects one into a signature table to look
        // it up in. `mwl_hir::members` has already checked that every one of
        // the three exists, so this only recovers the type.
        ExprKind::ClassConstAccess { class, name } => {
            check_expr(class, None, live, scope, ctx, env);
            let qname = resolve_class_expr(class, ctx, env);
            // A `Core`-owned enum has no `SymbolKind::Enum` entry — nothing
            // declared it — but it is in the same enum table, seeded from
            // `mwl_stdlib::registry::ENUMS`, so asking that table is the one
            // question that answers both. `crate::enums::seed_core` owns why
            // there is one table rather than two.
            let is_enum = qname.as_ref().is_some_and(|qname| {
                matches!(env.symbols.get(qname), Some(sym) if sym.kind == SymbolKind::Enum)
                    || (qname.is_core() && env.enums.get(qname).is_some())
            });
            match qname {
                Some(qname) if is_enum => {
                    // ADR 0010 § 3: the case *is* its integer constant, so
                    // `mwl-ir` needs the value, not just the type — see
                    // `ExprInfo::EnumCase`. A name `mwl_hir::members` already
                    // reported as undeclared records nothing.
                    let case = span_text(env.src, *name).to_owned();
                    if let Some(value) = env.enums.case(&qname, &case) {
                        env.exprs.record(expr.span, ExprInfo::EnumCase { value });
                    } else if qname.is_core() {
                        // One of the two places `Core`'s blanket trust is
                        // *narrowed* rather than relied on — the `StaticCall`
                        // arm above does the same for a member name:
                        // `mwl_hir::members` waves a
                        // `Core\…::Anything` through because nothing declares
                        // it, but `mwl_stdlib::registry::ENUMS` states every
                        // case a `Core` enum has, so a name that is not one is
                        // knowably wrong here. Without this the mistake
                        // reaches `mwl-ir` as a `Class::CONST` with no value
                        // recorded, which panics.
                        report_unknown_member(class.span, &qname, &case, "case", env);
                    }
                    let backing = env.enums.backing_of(&qname);
                    env.interner.enum_(qname, backing)
                }
                // ADR 0011's class constant, on a `Core` class the registry
                // states. The *value* is recorded, not just the type, for
                // exactly ADR 0010 § 3's reason one line above: a constant is
                // inlined at every use site, so `mwl-ir` needs the constant
                // itself and there is no storage to read it from at run time.
                Some(qname) if qname.is_core() => {
                    let constant = span_text(env.src, *name).to_owned();
                    match crate::core_lib::constant(&qname, &constant, env.interner) {
                        Some((ty, value)) => {
                            env.exprs.record(expr.span, ExprInfo::CoreConst { value });
                            ty
                        }
                        None => {
                            // The third narrowing of `Core`'s blanket trust,
                            // on the same terms as the two above: a class the
                            // registry *states* is checked like any other,
                            // while one it does not yet know stays trusted so
                            // the rest of the spec can be written in a fixture
                            // before it is implemented (`crate::core_lib`'s
                            // own docs own that rule).
                            if crate::core_lib::is_registered(&qname) {
                                report_unknown_member(
                                    class.span, &qname, &constant, "constant", env,
                                );
                            }
                            env.interner.mixed()
                        }
                    }
                }
                _ => env.interner.mixed(),
            }
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
        ExprKind::New { target, args } => {
            let target_ty = check_new_target(target, live, scope, ctx, env);
            let target_qname = class_qname_of(target_ty, env.interner);
            // A class with no explicit `constructor` accepts a bare `new
            // Foo()` in PHP; not diagnosing an arity mismatch against zero
            // parameters here is deliberate — see the crate docs' known gaps.
            // The *declaring* class is kept, not the constructed one: `new
            // Dog(...)` on a `Dog extends Animal` that declares no constructor
            // of its own invokes `Animal::constructor`, and `mwl-ir` cannot
            // re-walk the hierarchy to find that out (see
            // `crate::expr_table::ExprInfo::New::ctor`).
            let resolved = target_qname
                .clone()
                .and_then(|qname| resolve_method(&qname, "constructor", env.signatures, env.graph));
            let ctor_owner = resolved.as_ref().map(|(owner, _)| owner.clone());
            let sig = resolved.map(|(_, sig)| sig);
            let (arg_types, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
            if let Some(qname) = &target_qname {
                reject_secret_throwable_message(qname, arg_types.first().copied(), expr.span, env);
                // A `Core`-owned class has no constructor and never will: its
                // instances come from the member that produces one, and its
                // slots are `mwl-stdlib`'s layout rather than a surface a
                // program fills in (`mwl_stdlib::registry::CoreTy::Instance`).
                // Reported here rather than left to `mwl-codegen`, which would
                // fail with "this unit declares no descriptor for it" — an
                // internal message for an ordinary mistake.
                if crate::core_lib::is_registered(qname) {
                    report_unknown_member(expr.span, qname, "constructor", "member", env);
                }
                // `mwl-ir` needs the constructed class and its resolved
                // constructor (if any) to lower `new` — see
                // `crate::expr_table`'s own module docs.
                let ctor = sig.as_ref().zip(ctor_owner).map(|(s, owner)| {
                    resolved_call(owner, "constructor".to_owned(), s, env.signatures)
                });
                env.exprs.record(
                    expr.span,
                    ExprInfo::New {
                        class: qname.clone(),
                        ctor,
                        ty: target_ty,
                    },
                );
            }
            target_ty
        }
        ExprKind::Clone(inner) => check_expr(inner, None, live, scope, ctx, env),
        ExprKind::Fn(fn_expr) => check_fn_literal(expr, fn_expr, live, scope, ctx, env),
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
        // ADR 0053 § 4. Whether this is legal here at all, and what the
        // operand has to satisfy, are the same question — see
        // `Ctx::generator_elem`, which `crate::check::check_method` set from
        // the enclosing body's own shape.
        ExprKind::Yield { key, value } => {
            if let Some(k) = key {
                check_expr(k, None, live, scope, ctx, env);
                env.diags.report(
                    Diagnostic::error(
                        code::E_YIELD_FORM_UNSUPPORTED,
                        "a `yield` has no key half in MWL",
                    )
                    .with_primary(k.span, "no key exists here")
                    .with_help(
                        "ADR 0053 § 1 gives `Iterator<T>` exactly `advance()` and \
                         `current()`; drop the `key =>`",
                    ),
                );
            }
            match (ctx.generator_elem, value) {
                (Some(elem), Some(v)) => {
                    check_expr(v, Some(elem), live, scope, ctx, env);
                }
                (Some(_), None) => {
                    // ADR 0007 leaves no position untyped, and a bare `yield`
                    // would have to produce a `T` out of nothing.
                    env.diags.report(
                        Diagnostic::error(
                            code::E_YIELD_FORM_UNSUPPORTED,
                            "a `yield` needs a value",
                        )
                        .with_primary(expr.span, "nothing is yielded here")
                        .with_help("ADR 0053 § 1: `current()` returns a `T`, never nothing"),
                    );
                }
                (None, _) => {
                    if let Some(v) = value {
                        check_expr(v, None, live, scope, ctx, env);
                    }
                    report_yield_outside_generator(expr.span, env);
                }
            }
            env.interner.void()
        }
        ExprKind::YieldFrom(inner) => {
            check_expr(inner, None, live, scope, ctx, env);
            env.diags.report(
                Diagnostic::error(
                    code::E_YIELD_FORM_UNSUPPORTED,
                    "`yield from` does not exist in MWL",
                )
                .with_primary(expr.span, "this delegation form")
                .with_help(
                    "ADR 0053 § 5: write `foreach ($inner as T $v) { yield $v; }`, which is \
                     what it is a second spelling of",
                ),
            );
            env.interner.void()
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
