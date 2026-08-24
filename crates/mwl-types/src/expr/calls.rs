//! A call's target: which member `$obj->m()`, `C::m()` and `new C()` resolve
//! to, and ADR 0027's rule that only a closure is ever callable.
//!
//! Resolution is what this module owns; whether the *arguments* fit is
//! [`super::args`]. A call that does not statically resolve to a known
//! signature — an unresolved receiver, a dynamic member name, a
//! `Core`-namespaced target with no modeled stdlib signature — falls back to
//! `mixed` with no diagnostic, the same way this checker only ever reports
//! what it can be sure of. [`resolved_call`] is the record `mwl-ir` reads back
//! (see [`crate::expr_table`]), and it always carries the *declaring* class
//! rather than the receiver's.
//!
//! **ADR 0027 (`callable` is closures only)** lives here:
//! [`report_non_callable_value_if_applicable`] gives a bare string or
//! `[$obj, 'method']`-shaped array literal a targeted diagnostic naming the
//! first-class-callable-syntax replacement wherever `callable` is the expected
//! type, ahead of [`is_assignable`]'s generic mismatch (which would otherwise
//! also fire for the same expression); [`report_call_on_non_callable`] refuses
//! `$obj(...)` for any `$obj` whose static type is a resolved class — MWL has
//! no `__invoke`, so no class ever makes `()` mean anything else.
//! [`check_fn_literal`] is the other half of the same ADR pair: a closure
//! literal's body is checked like any other body, and it owns ADR 0031's
//! capture rule and the one shape it refuses (a block body with no declared
//! return type).
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(super)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// Builds the [`ExprInfo::Call`] entry [`crate::expr_table::ExprTypeTable`]
/// persists for a resolved method/static call — the one place `qname`/`name`/
/// `sig` (already computed for this call's own type-checking) get bundled
/// into the shape `mwl-ir` reads back, so the `MethodCall`/`StaticCall`/`New`
/// arms below don't each repeat the field list.
pub(super) fn resolved_call(
    qname: QName,
    name: String,
    sig: &MethodSig,
    signatures: &SignatureTable,
) -> ResolvedCall {
    let overridden = signatures.is_overridden(&qname, &name);
    ResolvedCall {
        class: qname,
        method: name,
        overridden,
        param_tys: sig.params.clone(),
        by_ref: sig.by_ref.clone(),
        variadic: sig.variadic,
        defaults: sig.defaults.clone(),
        is_static: sig.is_static,
        return_ty: sig.return_ty,
        has_body: sig.has_body,
        // Set only by the `StaticCall` arm, and only for an explicitly named
        // class — see the field's own doc comment.
        static_class: None,
        // Set only by the `StaticCall` arm, and only for a member on
        // `registry::WRITTEN_CLASS_MEMBERS` — see the field's own doc comment.
        written_class: None,
    }
}

pub(super) fn report_non_callable_value_if_applicable(expr: &Expr, env: &mut Env<'_>) -> bool {
    match &expr.kind {
        ExprKind::Str(_) | ExprKind::Interpolated(_) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_CALLABLE_STRING_UNSUPPORTED,
                    "a string is not callable in MWL; take a reference with first-class \
                     callable syntax instead",
                )
                .with_primary(expr.span, "this string")
                .with_help("e.g. `Class::method(...)` or `$obj->method(...)`"),
            );
            true
        }
        ExprKind::ArrayLiteral(_) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_CALLABLE_ARRAY_UNSUPPORTED,
                    "an array is not callable in MWL; take a reference with first-class \
                     callable syntax instead",
                )
                .with_primary(expr.span, "this array")
                .with_help("e.g. `$obj->method(...)` instead of `[$obj, 'method']`"),
            );
            true
        }
        _ => false,
    }
}

/// ADR 0027 § 1: `$obj(...)` is refused whenever `$obj`'s static type
/// resolves to a class — MWL has no `__invoke`, so no class ever makes `()`
/// mean anything else, regardless of what methods it declares. A `Ty::Mixed`
/// callee (nothing statically known) and an already-`Ty::Callable` one are
/// both left alone.
pub(super) fn report_call_on_non_callable(callee_ty: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname, _) = env.interner.get(callee_ty).clone() else {
        return;
    };
    env.diags.report(
        Diagnostic::error(
            code::E_NOT_CALLABLE,
            format!(
                "`{qname}` is not callable; MWL has no `__invoke` — call a named method \
                 instead, e.g. `$obj->methodName(...)`"
            ),
        )
        .with_primary(span, "called with `(...)` here"),
    );
}

pub(super) fn check_member_name(
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

pub(super) fn check_args(
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

pub(super) fn check_new_target(
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
            if env.symbols.get(&qname).is_some()
                || qname.is_core()
                || qname.is_reserved_global_class()
            {
                env.interner.class(qname)
            } else {
                // Diagnosed rather than erased to `mixed`: `mwl-ir` has no
                // class to allocate and panics naming the missing table entry,
                // which is a worse report of the same fact. The spelling this
                // most often catches is PHP's `new Exception(…)` — spec § 10
                // has no such class, so the ordinary undeclared-class
                // diagnostic is exactly the right answer.
                env.diags.report(
                    Diagnostic::error(
                        code::E_UNDEFINED_CLASS,
                        format!("`{qname}` is not declared"),
                    )
                    .with_primary(name.span, "no matching declaration"),
                );
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

/// [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md)'s
/// `fn` closure literal.
///
/// Three things happen here, and only the first is ordinary type-checking:
///
/// * The body is checked in a **fresh** [`LocalScope`] holding the closure's
///   own parameters. ADR 0007 § 1's declare-once rule is per body, so a
///   parameter named like an outer local shadows it rather than colliding
///   with it.
/// * Every outer binding is offered to that scope as a *capture* rather than
///   as a local ([`Captures`]), which is what makes the recorded capture set
///   "exactly the outer variables its body reads" (§ 2) rather than the whole
///   enclosing frame. `$this` is in that set like any other name, which is
///   ADR 0008 § 4's bind-`$this`-only-where-used rule with no code of its own.
/// * The literal's own [`ExprInfo::Closure`] entry is recorded, because a
///   `callable` type carries none of it (§ 4 keeps that type opaque).
///
/// **A block body must declare its return type.** An expression body is its
/// own answer, so it needs no annotation; inferring one for a block would
/// mean whole-body return-type inference, which is a larger thing than ADR
/// 0037's one-initializer rule and is not something ADR 0007 asks for. A
/// block body with none reports `E0450` and is checked against `void`.
///
/// **`yield` is not a generator here.** The inner [`Ctx`] clears
/// `generator_elem`, so a `yield` written inside a closure sitting in a
/// generator's own body reports `E0445` — ADR 0053 § 4's lexical confinement.
///
/// # Known gap
///
/// ADR 0031 § 3's optional self-name is parsed and ignored: nothing binds it,
/// so calling it inside the body reports an undefined name. Recursion through
/// a closure is the one § 3 capability with no other route, but it needs a
/// call shape that does not exist yet — see `mwl_ir::lower`'s own docs for
/// which closure call sites lower at all.
pub(super) fn check_fn_literal(
    expr: &Expr,
    f: &FnExpr,
    live: &FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let seq = env.closure_seq;
    env.closure_seq += 1;
    // `$` cannot appear in an MWL identifier, so this label can never collide
    // with a declared class — the same guarantee ADR 0053 § 4's generator
    // state class relies on.
    let owner = ctx
        .current_class
        .map_or_else(|| "Script".to_owned(), ToString::to_string);
    let class = format!("{owner}$fn{seq}");

    let mut inner = LocalScope::new();
    let mut inner_live = live.clone();
    for param in &f.params {
        let ty = lower_optional_type(param.ty.as_ref(), ctx, env);
        let name = strip_sigil(span_text(env.src, param.name)).to_owned();
        inner.declare_param(name.clone(), ty, param.name);
        inner_live.insert(name);
    }
    inner.captures = Some(Captures {
        available: scope.visible(),
        used: std::cell::RefCell::new(Vec::new()),
    });

    let inner_ctx = Ctx {
        namespace: ctx.namespace,
        imports: ctx.imports,
        current_class: ctx.current_class,
        current_hook: ctx.current_hook,
        generator_elem: None,
    };
    let declared = f
        .return_type
        .as_ref()
        .map(|t| lower_type(t, &inner_ctx, env));
    let return_ty = match (&f.body, declared) {
        (FnBody::Expr(body), Some(ret)) => {
            check_expr(body, Some(ret), &mut inner_live, &inner, &inner_ctx, env);
            ret
        }
        (FnBody::Expr(body), None) => {
            check_expr(body, None, &mut inner_live, &inner, &inner_ctx, env)
        }
        (FnBody::Block(block), declared) => {
            let ret = declared.unwrap_or_else(|| {
                env.diags.report(
                    Diagnostic::error(
                        code::E_CLOSURE_RETURN_TYPE_REQUIRED,
                        "a block-bodied closure must declare its return type",
                    )
                    .with_primary(expr.span, "no `: T` on this `fn`")
                    .with_help(
                        "write `fn (...): T => { ... }`, or use an expression body, whose type \
                         is the expression's own",
                    ),
                );
                env.interner.void()
            });
            check_block(
                &block.stmts,
                &mut inner_live,
                &mut inner,
                ret,
                &inner_ctx,
                env,
            );
            ret
        }
    };

    let captures = inner
        .captures
        .take()
        .expect("installed just above and never removed")
        .used
        .into_inner();
    // A capture the body reached through *this* closure's `available` set may
    // have come from an enclosing closure's own capture set rather than from
    // a real local — that closure has to capture it too in order to have it
    // to hand on. Harmless when the enclosing scope is an ordinary body: it
    // has no `Captures` for this to record into.
    for (name, _) in &captures {
        scope.note_capture(name);
    }
    env.exprs.record(
        expr.span,
        ExprInfo::Closure {
            class,
            captures,
            return_ty,
        },
    );
    env.interner.callable()
}
