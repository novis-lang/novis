//! A minimal bidirectional expression checker: the dispatch, and the modules
//! each rule lives in.
//!
//! [`check_expr`] takes an optional expected type: an
//! [`nvs_syntax::ast::ExprKind::ArrayLiteral`] checked against an `array<T>`
//! target checks every element directly against `T` (`rule:types/arrays` — "never
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
//! | [`assign`] | `rule:types/unions-and-mixed`'s assignability, and the positions applying it |
//! | [`calls`] | which member a call resolves to; `rule:types/callable-is-a-closure`'s `callable` |
//! | [`isolate`] | `rule:security/isolate-shares-nothing`'s `spawn script` and `await`, and what each types as |
//! | [`iteration`] | `rule:iteration/two-interfaces`'s `foreach` sources and `yield` forms |
//! | [`literals`] | how a literal takes its type from its position |
//! | [`members`] | a property, class constant or enum case, and who diagnoses it |
//! | [`operators`] | `rule:types/arithmetic`'s result table and the refusals layered on it |
//! | [`presence`] | `rule:classes/unset-is-refused-on-a-property`'s `isset(...)`, and what its operands may be |
//! | [`quals`] | `rule:security/tainted-qualifier`'s `tainted`, `rule:security/secret-qualifier`'s `secret`, and their sinks |
//! | [`type_test`] | `rule:types/type-test`'s `is`: what it answers, what it folds, and the two right-hand sides it refuses |
//!
//! Two fallbacks are deliberate and belong to no module. A method/static call
//! not statically resolvable to a known signature — an unresolved receiver, a
//! dynamic member name, a `Core`-namespaced target with no modeled stdlib
//! signature — types as `mixed` with no diagnostic, the same way this checker
//! only ever reports what it can be sure of. `empty(...)` is the other: its
//! operand is still left entirely unchecked, which is [`presence`]'s next
//! slice rather than a decision — `isset(...)`'s operands are checked there
//! now, and PHP's "an unset operand is tolerated" is answered by `rule:types/declaration` instead, every local being declared before it can be named at all.

use nvs_diagnostics::{Diagnostic, SourceFile, Span, code};
use nvs_hir::{ClassGraph, QName, SymbolKind};
use nvs_syntax::ast::{
    Arg, ArrayItem, AssignOp, BinaryOp, CallArgs, CatchArm, Expr, ExprKind, FnBody, FnExpr,
    ForeachBinding, MemberName, NewTarget, ObjectLiteralField, StringPart, Type, TypeKind, UnaryOp,
};
use rustc_hash::FxHashSet;

use crate::expr_table::{ArgSlot, ExprInfo, ForeachDrive, ResolvedCall};
use crate::locals::{Captures, LocalScope, check_block};
use crate::lower::{lower_optional_type, lower_type};
use crate::signatures::{
    MethodSig, SignatureTable, resolve_method, resolve_property, resolve_property_owned,
};
use crate::ty::{ShapeField, Ty, TypeId, TypeInterner};
use crate::{Ctx, Env, FnSelf, span_text, strip_sigil};

// One expression checker split across this directory — see each module's own
// header for what it owns. A rule reaches its neighbours as `pub(crate)`,
// which is the reach it had when `expr` was a single file, and no further.
pub(crate) mod args;
pub(crate) mod assign;
pub(crate) mod calls;
pub(crate) mod isolate;
pub(crate) mod iteration;
pub(crate) mod literals;
pub(crate) mod members;
pub(crate) mod operators;
pub(crate) mod presence;
pub mod quals;
pub(crate) mod type_test;

use self::{
    args::*, assign::*, calls::*, iteration::*, literals::*, members::*, operators::*, quals::*,
    type_test::*,
};

// What the rest of the crate reaches through `crate::expr::…`, unchanged by
// the split: the same names, at the same path, whichever module now holds
// them.
pub(crate) use self::{
    assign::{check_return, is_assignable, report_mismatch},
    iteration::{check_foreach_inout, check_foreach_key, check_foreach_value, foreach_source},
    literals::{check_array_key_type, check_object_literal, int_literal_digits},
    members::{
        can_hold_an_object, check_unset_target, is_this_receiver, reject_finish_marker_arm,
        resolve_class_expr,
    },
    operators::{reject_carrier_as_text, reject_disjoint_equality, require_stringable},
    quals::{reject_secret_attribute_constant, reject_secret_output},
};

/// Public because `nvs-ir` asks the same roster this crate does: a `Core` class
/// name a downcast and a closure parameter's entry check can test a value
/// against is exactly the one `instanceof` accepts, and one predicate answering
/// both is what keeps the two passes from disagreeing about which names have a
/// descriptor.
pub use self::members::testable_core_class;

/// Whether `ty` carries `rule:security/secret-qualifier`
/// 's `secret` qualifier — the one thing outside this crate a *declared*
/// type is asked, and asked at the one end that knows.
///
/// `rule:errors/record-transformations`'s redaction row is two halves of one rule about one record,
/// and neither half can be decided from a value: `secret` is a qualifier on a
/// declared type, erased everywhere below the checker. The call-site half is
/// [`quals::reject_secret_debug_argument`], made here; the property half is a
/// bit per field slot, carried down through `nvs_ir::ir::Class::secret_fields`
/// to `nvs_runtime::ClassDesc` so `nvs_stdlib::debug`'s walk can answer it
/// from an instance. This is what that lowering reads.
pub fn type_is_secret(ty: TypeId, interner: &TypeInterner) -> bool {
    quals::is_secret(ty, interner)
}

/// Checks `expr`, optionally against `expected`, returning the type it was
/// found (or, for an array literal checked against a target, declared) to
/// have. Reports `E_TYPE_MISMATCH` when `expected` is given and not
/// satisfied.
///
/// A parenthesised group reports nothing of its own: [`infer`]'s `Paren` arm
/// re-enters here with the same `expected`, so by the time this returns the
/// group's inner expression has already had the whole of this function applied
/// to it — the refusal below, and the callable one above it — at its own,
/// narrower span. Repeating either here is one mistake reported twice, a column
/// apart, which is what `Door::a(({name: "x"}))` used to print.
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
        && !matches!(expr.kind, ExprKind::Paren(_))
    {
        // A written signature is a callable position like the bare type is
        // (`rule:types/callable-signature`), so a value that is not a closure
        // at all gets `rule:types/callable-is-a-closure`'s own refusal at
        // either spelling rather than a bare mismatch at one of them.
        let wants_callable = matches!(
            env.interner.get(expected_id),
            Ty::Callable | Ty::CallableSig { .. }
        );
        if wants_callable && report_non_callable_value_if_applicable(expr, env) {
            return actual;
        }
        if !is_assignable(actual, expected_id, env.interner, env.graph, env.signatures) {
            report_mismatch(expr.span, expected_id, actual, env);
        }
    }
    actual
}

/// [`check_expr`] for an expression used as a **condition**, and the one site
/// in this crate that asks what a condition's type is.
///
/// `rule:expressions/truthy-positions` makes a condition the one place a value is tested without `as`, so
/// there is nothing to check here in the ordinary sense — its truthy table has
/// a row for every type. What it does not have is a row for a value that is
/// not one, which is [`reject_void_condition`]'s whole subject. Every position
/// that tests a value without an operator goes through here — `if`, `while`,
/// `do`/`while`, a `for` header's middle clause and a ternary's condition —
/// while `!` and `empty()` report the same thing at their own arms, having
/// already inferred their operand for a reason of their own.
///
/// `&&`, `||` and `??` are deliberately *not* here: they are `rule:types/arithmetic`'s
/// operands and keep `E0718` through
/// [`operators::reject_void_operand`](reject_void_operand). See
/// [`code::E_VOID_IS_NOT_A_CONDITION`] for where that line is drawn and why.
pub(crate) fn check_condition(
    cond: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let ty = check_expr(cond, None, live, scope, ctx, env);
    reject_void_condition(ty, cond.span, env);
    ty
}

/// [`check_expr`] for an expression used as its own **statement**.
///
/// Two shapes mean something in that position and nothing anywhere else, so
/// this is where they are told apart rather than by threading a flag through
/// the whole dispatch. Both mirror `nvs_ir::lower::Lowering::lower_expr_stmt`
/// exactly, and both look through parentheses for its reason: `(require 'a');`
/// is the same statement, `nvs_syntax::ast::Expr::unparenthesized` being what
/// finds the root there too.
///
/// * `yield $v;` — `rule:iteration/generators`'s suspension point, whose value nothing
///   consumes. [`infer`]'s own arm refuses every *other* position (`E0448`),
///   so this call is the one path that reaches [`infer_yield`].
/// * `require '…';` — `rule:statements/require-is-the-only-inclusion-construct`'s statement form, lowered to nothing because
///   the graph is resolved at compile time. Only the path expression is
///   checked; [`infer`]'s arm refuses the value form (`E0704`).
///
/// Everything else is an ordinary expression and goes straight to
/// [`check_expr`] with no expectation, exactly as before this split.
pub(crate) fn check_expr_stmt(
    expr: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let inner = expr.unparenthesized();
    match &inner.kind {
        ExprKind::Yield { key, value } => {
            infer_yield(
                inner,
                key.as_deref(),
                value.as_deref(),
                live,
                scope,
                ctx,
                env,
            );
        }
        ExprKind::Require { path } => {
            check_expr(path, None, live, scope, ctx, env);
        }
        _ => {
            check_expr(expr, None, live, scope, ctx, env);
        }
    }
}

/// The dispatch: one arm per AST expression variant, each either a couple of
/// lines or a single call into the module that owns its rule.
///
/// `pub(crate)` for one caller: [`super::operators::infer_conversion`] needs
/// the *placing* walk rather than [`check_expr`]'s conforming one, so a numeric
/// literal written directly under an `as` takes the target as its expectation
/// (`rule:types/numeric-literal-placement`).
#[expect(
    clippy::too_many_lines,
    reason = "one match arm per AST expression variant, each a couple of lines"
)]
pub(crate) fn infer(
    expr: &Expr,
    expected: Option<TypeId>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    match &expr.kind {
        ExprKind::Null => env.interner.null(),
        ExprKind::Bool(value) => infer_bool_literal(*value, expected, env),
        ExprKind::Int(span) => infer_int_literal(*span, expr.span, expected, env),
        ExprKind::Float(span) => infer_float_literal(*span, expr.span, expected, env),
        // `rule:types/duration-literal`: a duration literal is `Core\Time\Duration` and nothing
        // places it — the suffix *is* the type, unlike `rule:types/decimal`'s fractional
        // literal just above. The lexer has already run the grammar and
        // reported anything wrong, so there is nothing left to check here.
        ExprKind::Duration(_) => env
            .interner
            .class(QName::parse(nvs_stdlib::time::DURATION_NAME)),
        ExprKind::Str(span) => infer_str_literal(*span, expected, env),
        ExprKind::Interpolated(parts) => infer_interpolated(expr, parts, live, scope, ctx, env),
        ExprKind::Markup(parts) => infer_markup_literal(parts, live, scope, ctx, env),
        ExprKind::Variable(span) => {
            let name = strip_sigil(span_text(env.src, *span)).to_owned();
            let ty = check_read(&name, expr.span, live, scope, env);
            // A narrowing is only usable below the checker if the read that
            // benefits from it says so — `ExprInfo::NarrowedRead` owns why the
            // fact belongs here rather than at each consumer. The equality is
            // what keeps an *unnarrowed* read (and a `check_read` that
            // recovered with `mixed` after reporting) from recording one.
            if scope.narrowed_ty(&name) == Some(ty) {
                env.exprs
                    .record(expr.span, ExprInfo::NarrowedRead { to: ty });
            }
            ty
        }
        ExprKind::ConstFetch(_) => env.interner.mixed(),
        ExprKind::SelfExpr | ExprKind::StaticExpr => class_of_ctx(ctx, env),
        ExprKind::ParentExpr => env.interner.mixed(),
        ExprKind::ArrayLiteral(items) => {
            check_array_literal(items, expected, live, scope, ctx, env)
        }
        ExprKind::ObjectLiteral(fields) => {
            check_object_literal(fields, expected, live, scope, ctx, env)
        }
        ExprKind::Unary { op, expr: inner } => {
            // `infer`, not `check_expr`: the operand inherits an *expectation*
            // rather than a position it has to satisfy, so a `-$n` under a
            // `-1` target is still an ordinary mismatch reported once, at the
            // negation, rather than twice.
            let hint = negated_literal_expectation(*op, expected, env.interner);
            let inner_ty = infer(inner, hint, live, scope, ctx, env);
            match op {
                // `!` is `rule:expressions/truthy-positions`'s truthy test written out rather than one of
                // `rule:types/arithmetic`'s rows, so its `void` operand is the condition
                // refusal and not the arithmetic one below it.
                UnaryOp::Not => {
                    reject_void_condition(inner_ty, expr.span, env);
                    env.interner.bool_ty()
                }
                UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitNot => {
                    reject_unary_arith_operand(*op, inner_ty, expr.span, env);
                    negated_literal_result(*op, inner_ty, env.interner)
                }
                // `UnaryOp::Suppress` is not a row here and never will be: the
                // parser refuses `@` where it is written (`E0236`), so nothing
                // this function can be handed carries it. The wildcard is the
                // `#[non_exhaustive]` enum's, not that operator's.
                _ => inner_ty,
            }
        }
        ExprKind::PreIncDec { expr: inner, .. } | ExprKind::PostIncDec { expr: inner, .. } => {
            note_write(inner, scope, env);
            // An increment is `rule:types/arithmetic`'s `± 1` and produces the target's
            // own type — the write does not widen it, exactly as `$x += 1`
            // does not. `reject_increment_on_non_numeric` owns which targets
            // that table leaves nothing to lower for.
            //
            // It is also a *write*, which is the half this arm used to leave
            // out: `nvs_ir::lower` desugars `$x++` into the same `$x = $x + 1`
            // a compound assignment becomes, so the three targets
            // [`check_write_target`] refuses have no more of a place to put
            // `± 1` than they had to put an assigned value. Marking the
            // subscript levels first is what lets `refused_as_a_write_target`
            // hold `E0482` back for `$maybe?->rows["0"]++`, whose nullable
            // base is a symptom of the `?->` the next call names properly;
            // `false` because `$a[]++` names no element and stays
            // `E_APPEND_IN_READ_POSITION`.
            mark_write_target_levels(inner, false, env);
            let inner_ty = check_expr(inner, None, live, scope, ctx, env);
            let before = env.diags.len();
            check_write_target(inner, ctx, env);
            // A target with nowhere to write to is one mistake, not two: the
            // `?int` a refused `$maybe?->n` reads as would otherwise take a
            // second, narrower diagnostic for a nullability the first one is
            // already about.
            if env.diags.len() == before {
                reject_increment_on_non_numeric(inner_ty, expr.span, env);
            }
            inner_ty
        }
        ExprKind::Binary { op, lhs, rhs } => {
            // PHP's `??` is "absent or `null`, without the warning", so no read
            // under one takes `rule:php-migration/every-divergence-is-deliberate-and-listed` row 11's throw — and that
            // is the whole chain, not only the outermost level: PHP reads
            // `$a["k"]["j"] ?? "d"` as "`"d"` unless every level is there".
            // `presence::mark_guarded_places` is the walk, shared with `isset`
            // and `empty` so the three cannot drift apart, and it runs before
            // the operand is checked because the arms that read the mark are
            // inside that check — see `Env::coalesce_guarded`.
            if *op == BinaryOp::Coalesce {
                presence::mark_guarded_places(lhs, env);
            }
            // The literal side is checked *second*, so the type it takes its
            // placement from is already in hand — see
            // [`uint_operand_expectation`] for why `uint` is the only type
            // that placement changes anything for. When both sides are digit
            // runs neither places the other and the source order stands, which
            // is also the order every diagnostic below is reported in.
            let (lhs_ty, rhs_ty) =
                if matches!(lhs.kind, ExprKind::Int(_)) && !matches!(rhs.kind, ExprKind::Int(_)) {
                    let rhs_ty = check_expr(rhs, None, live, scope, ctx, env);
                    let placed = uint_operand_expectation(lhs, rhs_ty, env.interner);
                    (check_expr(lhs, placed, live, scope, ctx, env), rhs_ty)
                } else {
                    let lhs_ty = check_expr(lhs, None, live, scope, ctx, env);
                    let placed = uint_operand_expectation(rhs, lhs_ty, env.interner);
                    (lhs_ty, check_expr(rhs, placed, live, scope, ctx, env))
                };
            if *op == BinaryOp::Concat {
                require_stringable(lhs_ty, lhs.span, env);
                require_stringable(rhs_ty, rhs.span, env);
                // `rule:core-classes/html-escape-answers-markup`: `.` has no row for the HTML carrier, which is
                // the refusal that makes `+` the composition operator rather
                // than one of two.
                reject_carrier_as_text(lhs_ty, lhs.span, env);
                reject_carrier_as_text(rhs_ty, rhs.span, env);
            }
            binary_result(*op, lhs_ty, rhs_ty, expr.span, env)
        }
        ExprKind::Assign {
            op,
            target,
            value,
            by_ref,
        } => {
            // `$a = &$b;` — refused here rather than left to `check_assign`,
            // whose whole job is the type on either side of a `=` and which
            // has nothing to say about the `&`. Reported and then checked
            // anyway, so a reference assignment that is *also* a type error
            // says both things in one run.
            if *by_ref {
                report_by_reference_assignment(expr.span, value.span, env);
            }
            check_assign(*op, expr.span, target, value, live, scope, ctx, env)
        }
        ExprKind::Ternary { cond, then, else_ } => {
            let cond_ty = check_condition(cond, live, scope, ctx, env);
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
        ExprKind::TypeTest {
            expr: inner,
            against,
        } => infer_type_test(expr, inner, against, live, scope, ctx, env),
        ExprKind::Call { callee, args } => {
            // `rule:types/closure-self-name`'s self-name, resolved before the callee is checked
            // as an expression: it is a name this closure's body binds and not
            // a value, so `check_expr` has nothing to say about it and would
            // answer `mixed` for an unknown constant instead.
            // `calls::check_fn_literal` owns the rule.
            if let ExprKind::ConstFetch(name) = &callee.kind
                && let Some(fn_self) = &env.fn_self
                && fn_self.name == span_text(env.src, name.span)
            {
                let ret = fn_self.ret;
                env.exprs.record(callee.span, ExprInfo::ClosureSelf);
                // The closure being written is its own signature, so the
                // arguments are held to its parameter list here rather than to
                // `nvs_runtime::closure`'s tag at a time
                // (`calls::check_self_name_args`). The three argument shapes it
                // hands back are the ones with no parameter to be checked
                // against, and they take the ordinary walk below.
                if check_self_name_args(expr, args, live, scope, ctx, env).is_none() {
                    check_args(args, live, scope, ctx, env);
                }
                report_args_with_no_parameter_list(args, NoParameterList::Callable, env);
                if matches!(args, CallArgs::FirstClassCallable) {
                    return env.interner.callable();
                }
                return ret;
            }
            let callee_ty = check_expr(callee, None, live, scope, ctx, env);
            // `rule:types/callable-signature`: where the callee's type names
            // its parameters, the arguments are proven here rather than a tag
            // at a time in `nvs_runtime::closure`. Ahead of `check_args`
            // because each argument is checked against the parameter it fills,
            // which is also where an `fn` literal argument takes its own
            // parameter types from (`calls::check_call_through_signature`).
            if let Some(ret) =
                check_call_through_signature(expr, callee_ty, args, live, scope, ctx, env)
            {
                return ret;
            }
            check_args(args, live, scope, ctx, env);
            report_args_with_no_parameter_list(args, NoParameterList::Callable, env);
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
        } => check_property_access(
            expr.span, object, property, *nullsafe, false, live, scope, ctx, env,
        ),
        ExprKind::StaticPropertyAccess { class, name } => {
            check_expr(class, None, live, scope, ctx, env);
            // A static property's storage is resolved where the access is
            // written, so `static::` — which PHP re-resolves against the
            // *called* class — has no honest answer here. See
            // `code::E_STATIC_PROPERTY_LATE_BOUND`, which owns the rule.
            if matches!(class.kind, ExprKind::StaticExpr) {
                env.diags.report(
                    Diagnostic::error(
                        code::E_STATIC_PROPERTY_LATE_BOUND,
                        "`static::` does not resolve a static property",
                    )
                    .with_primary(class.span, "the called class is only known at run time")
                    .with_help(
                        "write `self::` for the class this is declared in, or name the class \
                         whose storage is meant",
                    ),
                );
            }
            let text = span_text(env.src, *name);
            let prop_name = strip_sigil(text).to_owned();
            // Resolved through the *owning* class so `rule:core-api/written-visibility`'s level test
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
                    // Keyed by the *declaring* class, which is the storage's
                    // identity — see `ExprInfo::StaticProperty`. Recorded for
                    // a write exactly as for a read: `check_assign`'s general
                    // arm routes an assignment target back through this same
                    // path, so both are keyed by this expression's own span.
                    env.exprs.record(
                        expr.span,
                        ExprInfo::StaticProperty {
                            class: owner,
                            name: prop_name,
                            ty,
                        },
                    );
                    ty
                }
                None => env.interner.mixed(),
            }
        }
        ExprKind::ClassConstAccess { class, name } => {
            infer_class_const(expr, class, *name, expected, live, scope, ctx, env)
        }
        // `Foo::class` — the class's own fully qualified name, as a `string`
        // constant folded here. PHP resolves it against the file's imports and
        // namespace with no runtime step at all, and so does this: the class
        // side is never *checked* as a value (that would be `E0319`/`E0321` on
        // every `Foo::` in the program), it is resolved the same way a static
        // call's is. A class side that is not statically known is `E0702`.
        ExprKind::ClassNameConst { class } => {
            members::check_class_name_const(expr, class, live, scope, ctx, env)
        }
        // `nvs-ir` needs the element's declared type to lower an eventual
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
            // A base that reported its own error already answered `mixed` as
            // *recovery*, and `report_unsubscriptable` below would then blame
            // the subscript for it — `$nope["0"]` is one mistake, not two.
            let before = env.diags.len();
            let base_ty = check_expr(base, None, live, scope, ctx, env);
            let base_reported = env.diags.len() != before;
            match index {
                Some(index) => {
                    let index_ty = check_expr(index, None, live, scope, ctx, env);
                    check_array_key_type(index_ty, index.span, env);
                }
                // `$a[]` names the key one past the highest integer key,
                // which is an answer only where a value is being put there.
                // `assign::mark_append_targets` recorded the spans where it
                // is — every level of a plain `=`'s target chain — so any
                // other one reaching here is a read, exactly as PHP's own
                // "Cannot use [] for reading" decides it.
                None if env.write_target_levels.get(&expr.span) != Some(&true) => {
                    env.diags.report(
                        Diagnostic::error(
                            code::E_APPEND_IN_READ_POSITION,
                            "`[]` can only be assigned to",
                        )
                        .with_primary(expr.span, "append syntax names no existing element")
                        .with_help(
                            "`$a[] = …;` adds an element, and that is the one position `[]` \
                             has a meaning in — everywhere else, name the element you mean \
                             (`$a[\"0\"]`, or `Core\\Arr::last($a)`)",
                        ),
                    );
                }
                None => {}
            }
            let guarded = env.coalesce_guarded.contains(&expr.span);
            // A guarded level's *base* may itself be a guarded read, and one of
            // those answers `?array<T>` — so under a `??`, and only there, the
            // `null` is dropped before the element type is read off the base.
            // That is not the narrowing `E0482` asks an untested nullable for:
            // a `null` base under a `??` has an answer (`null`, and then the
            // right operand), which is exactly what PHP does with the whole
            // chain. `nvs_array_optional_get` is where the runtime half of it
            // lives.
            let base_ty = if guarded {
                env.interner.without_null(base_ty)
            } else {
                base_ty
            };
            // A `mixed` base is `rule:types/conversion`'s one unchecked position, so
            // `rule:types/erased-member-access`'s deferral applies to a subscript exactly as it
            // does to a member access: the base defers not only which array
            // is behind the handle but whether there is one at all, and the
            // *tag* answers both below (`nvs_ir::Helper::ValueIndexGet`).
            // The element is then `mixed` too — an array behind a `mixed`
            // carries no declared element type either.
            //
            // A **write** target is deliberately not part of this: an element
            // write through an erased base has `rule:types/arrays`'s copy-on-write
            // separation to write back through a holder that is only a tag,
            // and until that exists `E0482` refuses it where it is written
            // rather than leaving `nvs-ir` to panic.
            let deferred_to_the_tag = matches!(env.interner.get(base_ty), Ty::Mixed)
                && !env.write_target_levels.contains_key(&expr.span);
            let mixed = env.interner.mixed();
            let elem_ty = match env.interner.get(base_ty) {
                Ty::Array(elem) => Some(*elem),
                Ty::Mixed if deferred_to_the_tag => Some(mixed),
                _ => None,
            };
            match elem_ty {
                Some(elem_ty) => {
                    // A guarded read answers `?elem_ty`, and that is the whole
                    // mechanism: `binary_result`'s `Coalesce` arm strips the
                    // `null` back off for the non-null branch, and the `null`
                    // it leaves in the operand's type is what keeps `nvs-ir`'s
                    // `lower_coalesce` from short-circuiting a `??` whose left
                    // operand looked statically non-nullable.
                    env.exprs
                        .record(expr.span, ExprInfo::Index { elem_ty, guarded });
                    if guarded {
                        let null = env.interner.null();
                        env.interner.make_union([elem_ty, null])
                    } else {
                        elem_ty
                    }
                }
                None => {
                    if !base_reported && !refused_as_a_write_target(expr, base, env) {
                        report_unsubscriptable(base, base_ty, env);
                    }
                    env.interner.mixed()
                }
            }
        }
        ExprKind::New {
            target,
            type_args,
            args,
        } => infer_new(expr, target, type_args, args, live, scope, ctx, env),
        // `rule:classes/clone-is-shallow`'s operand rule, then the operand's own type unchanged:
        // a clone is a new instance of the same class. See
        // [`members::reject_non_object_clone`] for why the refusal is asked
        // of what provably cannot be an object rather than of what is.
        ExprKind::Clone(inner) => {
            let ty = check_expr(inner, None, live, scope, ctx, env);
            reject_non_object_clone(ty, inner.span, env);
            ty
        }
        ExprKind::Fn(fn_expr) => check_fn_literal(expr, fn_expr, expected, live, scope, ctx, env),
        ExprKind::Match { subject, arms } => {
            let subject_ty = check_expr(subject, None, live, scope, ctx, env);
            // `rule:types/unions-and-mixed`'s fourth narrowing spelling: under `match (true)` a
            // label is a condition rather than a value, so the arm body is
            // checked under exactly what `crate::locals::narrow` installs for
            // it — see [`crate::locals::is_true_literal`] for why only the
            // written literal counts and why a `default` or comma-separated
            // arm gets nothing.
            let labels_are_conditions = crate::locals::is_true_literal(subject);
            let mut arm_types = Vec::with_capacity(arms.len());
            for arm in arms {
                let mut narrowed = crate::locals::Narrowing::default();
                if let Some(conds) = &arm.conditions {
                    for c in conds {
                        // `rule:expressions/switch-match-equality`: an arm is compared against the subject
                        // by the one equality rule, so a disjoint arm is § 2's
                        // refusal written without the operator. `match (true)`
                        // is unaffected — every arm there is a `bool` too.
                        let cond_ty = check_expr(c, None, live, scope, ctx, env);
                        reject_disjoint_equality(subject_ty, cond_ty, c.span, env);
                    }
                    if labels_are_conditions && let [only] = &conds[..] {
                        narrowed = crate::locals::narrow(only, true, scope, env);
                    }
                }
                arm_types.push(check_expr(&arm.body, None, live, scope, ctx, env));
                narrowed.restore(scope);
            }
            if arm_types.is_empty() {
                // A `match` is an expression, so an arm-less one has no value
                // for the position it sits in — every path through it throws.
                // PHP agrees on the behaviour (`UnhandledMatchError`, on every
                // evaluation) and only disagrees on when it is said; refusing
                // it here loses no program that ran, and it is what keeps
                // `nvs_ir::lower::Lowering::lower_match` from having to invent
                // a value for a phi with no incoming edge.
                env.diags.report(
                    Diagnostic::error(
                        code::E_MATCH_NO_ARMS,
                        "a `match` with no arms has no value".to_owned(),
                    )
                    .with_primary(expr.span, "every path through this `match` throws")
                    .with_help(
                        "write the arm this was meant to have, or `throw` outright if the \
                         intent was to refuse every subject",
                    ),
                );
                env.interner.mixed()
            } else {
                env.interner.make_union(arm_types)
            }
        }
        // `rule:iteration/generators` first: a `yield` written where there is no generator
        // body to suspend is `E0445` wherever it stands, and that rule is
        // reported by [`infer_yield`] — a closure inside a generator is the
        // case that makes the order matter, since its body is an expression
        // *and* is not the generator's own. The position question below is
        // only asked once there is a generator to ask it about.
        ExprKind::Yield { key, value } if ctx.generator_elem.is_none() => infer_yield(
            expr,
            key.as_deref(),
            value.as_deref(),
            live,
            scope,
            ctx,
            env,
        ),
        // `rule:iteration/generators`'s suspension point, reached *inside* another
        // expression. `check_expr_stmt` is what a `yield;` of its own goes
        // through, so arriving here is the proof this one was written where a
        // value is consumed — and § 5 gives a generator no `send()`, so there
        // is nothing for `yield` to produce. Same code as the two spellings
        // beside it: this is one more shape of `yield` the language does not
        // have.
        ExprKind::Yield { .. } => {
            env.diags.report(
                Diagnostic::error(
                    code::E_YIELD_FORM_UNSUPPORTED,
                    "a `yield` is a statement, not a value",
                )
                .with_primary(expr.span, "nothing is produced here")
                .with_help(
                    "`rule:iteration/one-way-only` gives a generator no `send()`, so a resumed `yield` has \
                     nothing to hand back — write `yield $v;` on its own",
                ),
            );
            env.interner.mixed()
        }
        ExprKind::YieldFrom(inner) => infer_yield_from(expr, inner, live, scope, ctx, env),
        ExprKind::Print(inner) => {
            let ty = check_expr(inner, None, live, scope, ctx, env);
            // The same sink `echo` reaches, and refused on the same terms —
            // see [`quals::reject_secret_output`]. `print` differs from `echo`
            // only in being an expression with a value, which is nothing the
            // qualifier cares about.
            if !reject_secret_output(ty, inner.span, "print", env) {
                require_stringable(ty, inner.span, env);
            }
            env.interner.int()
        }
        // `never` whatever the operand is — the expression does not complete,
        // which is what `rule:expressions/catch-arm-is-an-expression`'s arm relies on. The operand is still
        // held to spec § 10's tree: `nvs_ir::lower::exception` builds a
        // landing pad against it and lowers no other shape.
        ExprKind::Throw(inner) => {
            let ty = check_expr(inner, None, live, scope, ctx, env);
            reject_unthrowable(ty, inner.span, env);
            env.interner.never()
        }
        // `rule:classes/unset-is-refused-on-a-property`: `isset($x)` is `$x != null`, and a list of operands
        // is the conjunction — so every one of them is checked, and every
        // subscript and shape-property level under one is a guarded read.
        // `presence` owns both rules.
        ExprKind::Isset(operands) => {
            for operand in operands {
                presence::check_isset_operand(operand, live, scope, ctx, env);
            }
            env.interner.bool_ty()
        }
        // `empty($x)` is `!$x` — `rule:expressions/truthy-table`'s table negated — over any
        // expression at all, so it shares `isset`'s guarded-read rule and
        // none of its shape check. `presence` owns both.
        ExprKind::Empty(operand) => {
            let operand_ty = presence::check_empty_operand(operand, live, scope, ctx, env);
            // `empty($x)` is `!$x`, so it takes `!`'s refusal too.
            reject_void_condition(operand_ty, operand.span, env);
            env.interner.bool_ty()
        }
        ExprKind::Exit(opt) => {
            if let Some(e) = opt {
                // PHP's `exit` takes either spelling: an `int` is the process
                // status, a `string` is a message written before the program
                // stops. `rule:types/conversion` has no implicit conversion to offer for
                // anything else, so anything else is a mismatch here rather
                // than a silent `as`.
                let actual = check_expr(e, None, live, scope, ctx, env);
                let int_ty = env.interner.int();
                let string_ty = env.interner.string();
                let never_ty = env.interner.never();
                if actual != never_ty
                    && !is_assignable(actual, int_ty, env.interner, env.graph, env.signatures)
                    && !is_assignable(actual, string_ty, env.interner, env.graph, env.signatures)
                {
                    let actual_desc = env.interner.describe(actual);
                    env.diags.report(
                        Diagnostic::error(
                            code::E_TYPE_MISMATCH,
                            format!("expected `int` or `string`, found `{actual_desc}`"),
                        )
                        .with_primary(
                            e.span,
                            "`exit` takes an `int` process status or a `string` message".to_owned(),
                        ),
                    );
                }
            }
            env.interner.never()
        }
        // `rule:security/isolate-shares-nothing`'s two constructs, both of which carry a rule of their own:
        // what each is typed as, and why the handle and the result are
        // answered by opposite mechanisms. [`isolate`] owns it.
        ExprKind::SpawnScript { path, options } => {
            isolate::check_spawn_script(path, options, live, scope, ctx, env)
        }
        ExprKind::Await(inner) => isolate::check_await(inner, live, scope, ctx, env),
        // `rule:statements/a-require-expression-is-mixed`'s **value** form. The statement form never reaches here
        // — `crate::locals::check_stmt` checks only the path for one, matching
        // `nvs_ir::lower::Lowering::lower_expr_stmt` — so arriving at all is
        // the proof this `require` was used for its value.
        //
        // § 3 types that boundary `mixed` and gives exactly one reason: what a
        // `return`-ing target hands back cannot be known statically, and
        // `mixed` is `rule:types/conversion`'s one unchecked position. There is nothing
        // to check here beyond the path, and nothing narrower to answer — a
        // typed binding takes the same `as` any other `mixed` boundary needs.
        ExprKind::Require { path } => {
            check_expr(path, None, live, scope, ctx, env);
            env.interner.mixed()
        }
        // `rule:expressions/catch-result-type` and `rule:expressions/bare-throwable-arm-warns`. The result is the union of the guarded expression's
        // type and every arm's, through the same `make_union` the
        // `ExprKind::Match` arm above reaches for — it is the same rule, so
        // neither side is checked against the other and both are checked
        // against nothing but the position the whole expression sits in.
        ExprKind::Catch { guarded, arms } => {
            // § 4: an arm is checked from the **pre-guard** definite-assignment
            // state, as a block clause is, because an arm runs precisely when
            // the guard did not complete — so a local the guard assigned cannot
            // be assumed assigned inside one. What is live afterwards is the
            // join across the ways the expression can produce a value, the same
            // shape `crate::locals`' `StmtKind::Try` arm builds.
            let before = live.clone();
            let never = env.interner.never();
            let mut types = vec![check_expr(guarded, None, live, scope, ctx, env)];
            let mut joins: Vec<FxHashSet<String>> = vec![live.clone()];
            for arm in arms {
                let ty = lower_type(&arm.ty, ctx, env);
                reject_finish_marker_arm(ty, arm.ty.span, env);
                let mut arm_live = before.clone();
                let mut binding = None;
                let mut fresh = None;
                match arm.var {
                    Some(var) => {
                        let name = strip_sigil(span_text(env.src, var)).to_owned();
                        binding = Some(crate::locals::bind_catch_arm(scope, &name, ty, var, env));
                        // A name the guard could not have assigned is the
                        // arm's alone, so it leaves with the arm; one that was
                        // already live reused an existing binding and stays.
                        if !before.contains(&name) {
                            fresh = Some(name.clone());
                        }
                        arm_live.insert(name);
                    }
                    None => warn_discarding_throwable_arm(arm, ty, env),
                }
                let arm_ty = check_expr(&arm.body, None, &mut arm_live, scope, ctx, env);
                if let Some(binding) = binding {
                    binding.release(scope);
                }
                if let Some(name) = fresh {
                    arm_live.remove(&name);
                }
                // § 3: a `throw` arm is typed `never`, so it produces no value
                // — it contributes nothing to the union and is not a way the
                // expression finishes, exactly as a terminating clause is left
                // out of the block form's own join.
                if arm_ty != never {
                    types.push(arm_ty);
                    joins.push(arm_live);
                }
            }
            *live = joins
                .into_iter()
                .reduce(|a, b| a.intersection(&b).cloned().collect())
                .unwrap_or_default();
            env.interner.make_union(types)
        }
        ExprKind::Paren(inner) => check_expr(inner, expected, live, scope, ctx, env),
        ExprKind::Error(_) => env.interner.mixed(),
        _ => env.interner.mixed(),
    }
}

/// `rule:expressions/bare-throwable-arm-warns`'s warning: an arm naming `Throwable` itself, binding nothing,
/// over a body that is not a `throw`.
///
/// All three conditions carry weight. Bound, the value is carried and the site
/// can do something with it; over a `throw`, the failure leaves; and naming a
/// class below the root is a site saying which failure it anticipated. Only
/// the three together spell *discard every failure, including the ones this
/// site never anticipated* — which is PHP's `@`, removed by
/// `rule:php-migration/every-divergence-is-deliberate-and-listed` and regrown
/// as a one-liner. `nvs_diagnostics::code::W_CATCH_ARM_DISCARDS_EVERY_FAILURE`
/// says why it is a warning and not a refusal.
fn warn_discarding_throwable_arm(arm: &CatchArm, ty: TypeId, env: &mut Env<'_>) {
    if matches!(arm.body.kind, ExprKind::Throw(_)) {
        return;
    }
    // The root by name, not by reachability: every class in the tree "is a"
    // `Throwable`, and an arm naming `IOError` is the honest spelling this
    // warning asks for rather than another instance of the shape it refuses.
    if class_qname_of(ty, env.interner) != Some(QName::parse(nvs_hir::errors::ROOT)) {
        return;
    }
    env.diags.report(
        Diagnostic::warning(
            code::W_CATCH_ARM_DISCARDS_EVERY_FAILURE,
            "this `catch` arm discards every failure",
        )
        .with_primary(
            arm.span,
            "`Throwable` matches every class, and nothing here carries the value",
        )
        .with_help(
            "name the class this site expects, or bind the value and carry it — \
             `catch (Throwable $e) => report($e)`",
        ),
    );
}

/// `self`/`static`/`$this`'s type, resolved against the enclosing
/// declaration. ADR 0010: an enum has no methods to reach this from in a
/// well-formed program, but the parser still recovers a member it rejected
/// with `E_ENUM_MEMBER_UNSUPPORTED` (see `nvs-syntax::parser::parse_enum_body`)
/// and hands it to this checker anyway — so this must resolve the same way
/// [`crate::lower::lower_type`]'s `self`/`static` atom already does, an
/// enum-declared `qname` interning to `Ty::Enum` rather than `Ty::Class`.
/// Whether this subscript is a level of an assignment target whose chain
/// root `assign::check_write_target` is about to refuse by name — a nullsafe
/// receiver (`E0479`), a hooked property (`E0478`) or an erased one
/// (`E0480`).
///
/// All three make the property read as `mixed` or as `?array<T>`, so
/// [`report_unsubscriptable`] would otherwise fire first and blame the
/// subscript for a receiver problem the next call states properly. It is
/// gated on the target chain because the *read* `echo $erased->rows["0"];`
/// has no second diagnostic coming and `E0482` is the only thing standing
/// between it and a panic in `nvs-ir`. All four write spellings mark that
/// chain — `unset($erased->rows["0"])` is a write for this purpose as much
/// as for [`assign::check_write_target`]'s, and used to take both
/// diagnostics because it was the one that did not.
///
/// Walks to the root for the same reason `check_write_target` does:
/// `$erased->rows["0"]["1"] = v` is one holder and two levels.
fn refused_as_a_write_target(expr: &Expr, base: &Expr, env: &Env<'_>) -> bool {
    if !env.write_target_levels.contains_key(&expr.span) {
        return false;
    }
    let mut root = base.unparenthesized();
    while let ExprKind::Index { base, .. } = &root.kind {
        root = base.unparenthesized();
    }
    if matches!(root.kind, ExprKind::PropertyAccess { nullsafe: true, .. }) {
        return true;
    }
    // A root that is no place at all takes `E0700` at the write, and that is
    // one mistake however the level below it reads — `$h->maybeRows()["a"] = v`
    // is not additionally a nullable subscript.
    if !is_a_place(&root.kind) {
        return true;
    }
    matches!(
        env.exprs.lookup(root.span),
        Some(ExprInfo::HookedProperty { .. } | ExprInfo::ShapeProperty { .. })
    )
}

/// `$x[…]` where `$x` is not an `array<T>`, refused where it is written.
///
/// `rule:types/arrays` keys an element read on the array's *declared* element type,
/// which is the entry [`ExprInfo::Index`] carries and the only thing
/// `nvs_ir::lower::Lowering::lower_index` has to lower against. A base with
/// no element type therefore has nothing to read, and every one of these
/// used to reach `nvs-ir` and panic there instead — a subscripted `mixed`,
/// a subscripted scalar, and an *untested* `?array<T>`.
///
/// The help splits three ways because the three have different answers, and
/// naming the wrong one costs a session: `mixed` needs the binding declared
/// as what it holds, a `string` needs `rule:types/string-is-utf8`'s grapheme indexing said
/// out loud as `Core\Str::slice`, and a nullable array needs the `!= null`
/// test `rule:expressions/nullable-conversion` already gives it — [`narrow`](crate::locals) drops the
/// `null` and the subscript is then an ordinary one.
fn report_unsubscriptable(base: &Expr, base_ty: TypeId, env: &mut Env<'_>) {
    let residue = env.interner.without_null(base_ty);
    let nullable_array = residue != base_ty && matches!(env.interner.get(residue), Ty::Array(_));
    let help = if nullable_array {
        "a nullable array has no elements until it is known not to be `null` — put the \
         subscript inside an `if ($x != null) { … }`, which narrows the binding to its \
         `array<T>` for the whole branch. Only a *binding* narrows, so a nullable array \
         that came straight out of a call has to be bound to one first"
    } else if matches!(
        env.interner.get(base_ty),
        Ty::String
            | Ty::Bytes
            | Ty::TaintedString
            | Ty::TaintedBytes
            | Ty::SecretString
            | Ty::SecretBytes
            | Ty::SecretTaintedString
            | Ty::SecretTaintedBytes
    ) {
        "`rule:types/string-is-utf8` indexes a `string` by extended grapheme cluster, which is said out \
         loud rather than spelled with a subscript — `Core\\Str::slice($s, $i, 1)`, or \
         `Core\\Bytes::slice` for a byte offset"
    } else {
        "only an `array<T>` has elements to subscript — declare the binding as the \
         `array<T>` it holds, so `rule:types/arrays` has an element type to check the read \
         against"
    };
    let desc = env.interner.describe(base_ty);
    env.diags.report(
        Diagnostic::error(
            code::E_SUBSCRIPT_ON_NON_ARRAY,
            format!("`{desc}` cannot be subscripted"),
        )
        .with_primary(base.span, format!("this is `{desc}`"))
        .with_help(help),
    );
}

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
