//! A call's target: which member `$obj->m()`, `C::m()` and `new C()` resolve
//! to, and ADR 0027's rule that only a closure is ever callable.
//!
//! Resolution is what this module owns; whether the *arguments* fit is
//! [`super::args`]. A call that does not statically resolve to a known
//! signature — an unresolved receiver, a dynamic member name, a
//! `Core`-namespaced target with no modeled stdlib signature — falls back to
//! `mixed` with no diagnostic, the same way this checker only ever reports
//! what it can be sure of. The one receiver that is *knowably* wrong rather
//! than merely unresolved is an erased one — a plain `object` or a shape,
//! neither of which lists a method at all — and it gets
//! [`report_method_on_erased_receiver`] instead. [`resolved_call`] is the record `mwl-ir` reads back
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

/// `$obj->method(...)` / `$obj?->method(...)` — [`super::infer`]'s
/// `ExprKind::MethodCall` arm.
///
/// Unlike a static call, `mwl_hir::members` never checks an instance method
/// call's existence for any receiver — including `$this` — so this is the
/// first and only place it is diagnosed.
#[expect(
    clippy::too_many_arguments,
    reason = "the four-part checking context every function in this module \
              threads — live set, scope, ctx, env — plus the call expression \
              and the four parts of the syntax it destructures"
)]
pub(super) fn infer_method_call(
    expr: &Expr,
    object: &Expr,
    method: &MemberName,
    nullsafe: bool,
    type_args: &[Type],
    args: &CallArgs,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let object_ty = check_expr(object, None, live, scope, ctx, env);
    // `?->` never reaches the method when the receiver is `null`, so the
    // method is resolved against the receiver's non-`null` half and the call's
    // own type gains the `null` that arm yields — see [`nullsafe_result`].
    let receiver_ty = strip_nullsafe_receiver(nullsafe, object_ty, object.span, env);
    check_member_name(method, live, scope, ctx, env);
    let resolved = match (class_qname_of(receiver_ty, env.interner), method) {
        (Some(qname), MemberName::Ident(name_span)) => {
            let name = span_text(env.src, *name_span).to_owned();
            let found = resolve_method(&qname, &name, env.signatures, env.graph);
            if found.is_none() && !qname.is_core() && !qname.is_reserved_global_class() {
                report_unknown_member(object.span, &qname, &name, "method", env);
            }
            if let Some((owner, sig)) = &found {
                check_method_visibility(owner, &name, sig, *name_span, ctx, env);
            }
            // The *declaring* class, not the receiver's: that is what
            // `ResolvedCall::class` promises, and `mwl-ir` renders the call's
            // target label from it — `$dog->name()` on a `Dog` that inherits
            // `name` must name `Animal::name`, the symbol that actually exists.
            found.map(|(owner, sig)| (owner, name, sig))
        }
        _ => None,
    };
    if resolved.is_none()
        && let MemberName::Ident(name_span) = method
        && matches!(env.interner.get(receiver_ty), Ty::Object | Ty::Shape(_))
    {
        let name = span_text(env.src, *name_span).to_owned();
        report_method_on_erased_receiver(object.span.to(*name_span), &name, receiver_ty, env);
    }
    let sig = resolved
        .as_ref()
        .map(|(owner, _, sig)| substitute_receiver_args(receiver_ty, owner, sig, env));
    let label = resolved
        .as_ref()
        .map(|(owner, name, _)| format!("{owner}::{name}"));
    let (sig, _written) =
        check_written_type_args(type_args, sig, label.as_deref(), expr.span, ctx, env);
    let (_, slots, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
    // ADR 0027: `$obj->method(...)` (first-class callable syntax) names a
    // `Closure` value, not the method's return type — the sentinel
    // `CallArgs::FirstClassCallable` marks exactly this shape, ahead of the
    // ordinary-call typing below.
    if matches!(args, CallArgs::FirstClassCallable) {
        return env.interner.callable();
    }
    // `mwl-ir` needs this call's resolved target (not just its return type) to
    // lower an eventual instance-call instruction — see `crate::expr_table`'s
    // own module docs. The *substituted* signature, never the one
    // `resolve_method` returned: `crate::generics` guarantees a type variable
    // never survives a call site, and this record is the one thing that carries
    // a signature past it.
    if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig) {
        let call = resolved_call(qname.clone(), name.clone(), sig, slots, env.signatures);
        env.exprs.record(expr.span, ExprInfo::Call(call));
    }
    let returned = sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty);
    nullsafe_result(nullsafe, object_ty, returned, env)
}

/// `Class::method(...)` — [`super::infer`]'s `ExprKind::StaticCall` arm.
///
/// `mwl_hir::members` already checks this reference's existence
/// (`self::`/`static::`/`parent::`/an explicit class name), so this only
/// recovers the call's *type* when a signature resolves, and adds no second
/// diagnostic when it doesn't.
#[expect(
    clippy::too_many_arguments,
    reason = "the same context [`infer_method_call`] threads, with the class \
              expression in place of the receiver and its nullsafe flag"
)]
pub(super) fn infer_static_call(
    expr: &Expr,
    class: &Expr,
    method: &MemberName,
    type_args: &[Type],
    args: &CallArgs,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    check_expr(class, None, live, scope, ctx, env);
    check_member_name(method, live, scope, ctx, env);
    let resolved = match method {
        MemberName::Ident(name_span) => resolve_class_expr(class, ctx, env).and_then(|qname| {
            let name = span_text(env.src, *name_span).to_owned();
            let found =
                resolve_method(&qname, &name, env.signatures, env.graph).map(|(owner, sig)| {
                    check_method_visibility(&owner, &name, &sig, *name_span, ctx, env);
                    // The declaring class — see [`infer_method_call`] for why
                    // the receiver's own is the wrong label.
                    (owner, name.clone(), sig)
                });
            // The same narrowing of `Core`'s blanket trust
            // [`super::members::infer_class_const`] explains: `mwl_hir` waves
            // every `Core\…::anything` through because nothing declares it, but
            // `mwl_stdlib::registry` states every member `Core` has, so a name
            // that is not one is knowably wrong *here*. Without this a typo
            // reaches `mwl-ir` as a static call with no resolved target
            // recorded, which panics.
            if found.is_none() && qname.is_core() {
                report_unknown_member(expr.span, &qname, &name, "member", env);
            }
            // ADR 0063 R20's one genuinely reachable two-spellings case — see
            // `report_core_instance_member`.
            if let Some((owner, _, sig)) = &found
                && owner.is_core()
                && !sig.is_static
            {
                report_core_instance_member(expr.span, owner, &name, env);
            }
            found
        }),
        _ => None,
    };
    let sig = resolved.as_ref().map(|(_, _, sig)| sig.clone());
    let label = resolved
        .as_ref()
        .map(|(owner, name, _)| format!("{owner}::{name}"));
    let (sig, written) =
        check_written_type_args(type_args, sig, label.as_deref(), expr.span, ctx, env);
    let (_, slots, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
    // See [`infer_method_call`]: first-class callable syntax names a `Closure`,
    // not the resolved method's return type.
    if matches!(args, CallArgs::FirstClassCallable) {
        return env.interner.callable();
    }
    // See [`infer_method_call`]: persisted for `mwl-ir` to read back a resolved
    // static call's target, always as the *substituted* signature.
    if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig) {
        let mut call = resolved_call(qname.clone(), name.clone(), sig, slots, env.signatures);
        // Late static binding: an explicitly named class *sets* the called
        // class, while `self`/`static`/`parent` forward the caller's. See
        // `ResolvedCall::static_class`.
        if matches!(class.kind, ExprKind::ConstFetch(_)) {
            call.static_class = resolve_class_expr(class, ctx, env);
        }
        call.written_class = written_class_of(qname, name, &written, type_args, expr.span, env);
        env.exprs.record(expr.span, ExprInfo::Call(call));
    }
    sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty)
}

/// `new Target(...)` — [`super::infer`]'s `ExprKind::New` arm.
///
/// A class with no explicit `constructor` accepts a bare `new Foo()` in PHP;
/// not diagnosing an arity mismatch against zero parameters here is deliberate
/// — see the crate docs' known gaps. The *declaring* class is kept, not the
/// constructed one: `new Dog(...)` on a `Dog extends Animal` that declares no
/// constructor of its own invokes `Animal::constructor`, and `mwl-ir` cannot
/// re-walk the hierarchy to find that out (see
/// `crate::expr_table::ExprInfo::New::ctor`).
#[expect(
    clippy::too_many_arguments,
    reason = "one checker context threaded positionally, as everywhere else here"
)]
pub(super) fn infer_new(
    expr: &Expr,
    target: &NewTarget,
    type_args: &[Type],
    args: &CallArgs,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let target_ty = check_new_target(target, live, scope, ctx, env);
    let target_qname = class_qname_of(target_ty, env.interner);
    let target_ty = check_new_type_args(
        type_args,
        target_qname.as_ref(),
        target_ty,
        expr.span,
        ctx,
        env,
    );
    let resolved = target_qname
        .clone()
        .and_then(|qname| resolve_method(&qname, "constructor", env.signatures, env.graph));
    // ADR 0094's levels reach `new` too, and deliberately: a `private`
    // constructor is PHP's singleton idiom, so the whole point of writing one
    // is that `new C()` is refused everywhere except `C`'s own bodies. The
    // span is the `new` expression rather than a member name, because that is
    // the only thing written here.
    if let Some((owner, sig)) = &resolved {
        check_method_visibility(owner, "constructor", sig, expr.span, ctx, env);
    }
    let ctor_owner = resolved.as_ref().map(|(owner, _)| owner.clone());
    let sig = resolved.map(|(_, sig)| sig);
    let (arg_types, slots, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
    if let Some(qname) = &target_qname {
        reject_secret_throwable_message(qname, arg_types.first().copied(), expr.span, env);
        // A `Core`-owned class has no constructor and never will: its instances
        // come from the member that produces one, and its slots are
        // `mwl-stdlib`'s layout rather than a surface a program fills in
        // (`mwl_stdlib::registry::CoreTy::Instance`). Reported here rather than
        // left to `mwl-codegen`, which would fail with "this unit declares no
        // descriptor for it" — an internal message for an ordinary mistake.
        //
        // The carve-out is `registry::CONSTRUCTORS`: spec § 9's collections are
        // written `new Core\ObjectSet<Tag>()`, and they still declare no
        // `constructor` *member* — what makes them constructible is a native
        // symbol `mwl-ir` lowers straight to, which `mwl_stdlib::instance`'s
        // module docs own. So the class is a legal `new` target while
        // `Core\ObjectSet::constructor` remains an unknown member.
        if crate::core_lib::is_registered(qname)
            && mwl_stdlib::registry::constructor_symbol(&qname.to_string()).is_none()
        {
            report_unknown_member(expr.span, qname, "constructor", "member", env);
        }
        // `mwl-ir` needs the constructed class and its resolved constructor (if
        // any) to lower `new` — see `crate::expr_table`'s own module docs.
        let ctor = sig.as_ref().zip(ctor_owner).map(|(s, owner)| {
            resolved_call(owner, "constructor".to_owned(), s, slots, env.signatures)
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

/// The `<...>` written between a `new` target and its `(` —
/// `<Tag>` in `new Core\ObjectSet<Tag>()`.
///
/// Every written type is lowered whichever way this goes, so an unknown class
/// named inside one is reported even when the list itself is refused; that is
/// [`super::args::check_written_type_args`]'s rule at a call site, for the
/// same reason.
///
/// **The roster is [`mwl_stdlib::registry::GENERIC_CLASSES`].** ADR 0007 § 3
/// makes "which target may carry a list" a resolution question, and the answer
/// is that table: a `Core`-owned generic class takes exactly the arguments it
/// declares (`E_TYPE_ARG_COUNT` on any other count, none at all included), and
/// every other target takes none (`E_TYPE_ARGS_NOT_GENERIC`), because
/// user-declared generic classes are deferred.
///
/// Returns the target's type with the written arguments bound onto it —
/// `Core\ObjectMap<Tag, int>` rather than a bare `Core\ObjectMap` — so the
/// receiver-driven substitution in [`super::args`] has something to zip the
/// parameter names against. A wrong count recovers to the bare class, exactly
/// as [`crate::lower`] does in type position: an argument the author did not
/// write has no honest value, and a name with no arguments is a shape
/// everything downstream already has a rule for.
fn check_new_type_args(
    type_args: &[Type],
    target: Option<&QName>,
    target_ty: TypeId,
    new_span: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let written: Vec<TypeId> = type_args
        .iter()
        .map(|ty| lower_type(ty, ctx, env))
        .collect();
    let span = type_args
        .first()
        .zip(type_args.last())
        .map(|(first, last)| first.span.to(last.span));
    let generic = target.and_then(|qname| {
        mwl_stdlib::registry::class_type_params(&qname.to_string()).map(|params| (qname, params))
    });
    let Some((qname, params)) = generic else {
        if let Some(span) = span {
            let named = target.map_or_else(|| "this target".to_owned(), QName::to_string);
            env.diags.report(
                Diagnostic::error(
                    code::E_TYPE_ARGS_NOT_GENERIC,
                    format!("`{named}` takes no type arguments"),
                )
                .with_primary(span, "type arguments written here")
                .with_help(
                    "user-declared generic classes are deferred (ADR 0007 § 3), so the only \
                     `new` target that may be written with one is a compiler-owned generic class",
                ),
            );
        }
        return target_ty;
    };
    if written.len() != params.len() {
        let (expected, got, names) = (params.len(), written.len(), params.join(", "));
        env.diags.report(
            Diagnostic::error(
                code::E_TYPE_ARG_COUNT,
                format!("`{qname}` takes {expected} type argument(s), not {got}"),
            )
            .with_primary(
                span.unwrap_or(new_span),
                format!("write `new {qname}<{names}>(…)`"),
            )
            .with_help(format!(
                "`docs/spec/01-core-library.md` § 9 declares `{qname}<{names}>`; the arguments \
                 are positional, and nothing about a `Core` collection infers them"
            )),
        );
        return target_ty;
    }
    env.interner.generic_class(qname.clone(), written)
}

/// Builds the [`ExprInfo::Call`] entry [`crate::expr_table::ExprTypeTable`]
/// persists for a resolved method/static call — the one place `qname`/`name`/
/// `sig` (already computed for this call's own type-checking) get bundled
/// into the shape `mwl-ir` reads back, so the `MethodCall`/`StaticCall`/`New`
/// arms below don't each repeat the field list.
pub(super) fn resolved_call(
    qname: QName,
    name: String,
    sig: &MethodSig,
    arg_slots: Vec<ArgSlot>,
    signatures: &SignatureTable,
) -> ResolvedCall {
    let overridden = signatures.is_overridden(&qname, &name);
    ResolvedCall {
        class: qname,
        method: name,
        overridden,
        arg_slots,
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

/// [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md)
/// § 1's opaque `callable`, refused from the call site's end rather than the
/// literal's ([`report_by_reference_parameter`] is the other end of the same
/// rule): a call through one may not write a `name:` argument.
///
/// `callable` is one type whatever closure the variable holds, so this site has
/// no parameter list to resolve a name against — and neither has the run time,
/// a closure object recording its arity and its parameter *tags* and never
/// their names (`mwl_runtime::closure`). PHP allows the spelling only because a
/// `Closure` there carries its whole declaration.
///
/// A `...` argument is left alone and lowers: how many arguments it hands over
/// is its own run-time length, which needs no parameter list to mean something
/// (`mwl_ir::Helper::CallClosureArray`). What still applies is rule 1 of
/// [`super::args::map_arguments`] — a positional argument cannot follow a `...`
/// — for the same reason it applies at a resolved call, so the two refusals are
/// one walk.
pub(super) fn report_named_args_through_callable(args: &CallArgs, env: &mut Env<'_>) {
    let CallArgs::List(list) = args else {
        return;
    };
    let mut positional_ends: Option<Span> = None;
    for arg in list {
        if let Some(name) = arg.name {
            positional_ends.get_or_insert(arg.span);
            let name = span_text(env.src, name).to_owned();
            env.diags.report(
                Diagnostic::error(
                    code::E_NAMED_ARG_THROUGH_CALLABLE,
                    format!("`{name}:` names no parameter of a `callable`"),
                )
                .with_primary(arg.span, "written by name here")
                .with_help(
                    "ADR 0031 § 1: `callable` is one opaque type whatever closure the variable \
                     holds, so neither this call site nor the closure it reaches carries a \
                     parameter name to fill — pass the argument positionally",
                ),
            );
        } else if arg.spread {
            positional_ends.get_or_insert(arg.span);
        } else if let Some(first) = positional_ends {
            super::args::report_positional_after_named(arg, first, env);
        }
    }
}

/// A method called on a receiver whose type names no class — a plain
/// `object`, or an ADR 0036 shape.
///
/// ADR 0007 § 3 makes `object` the opaque top of every class type: it is a
/// pointer with the class label erased, and it lists no members. A shape is
/// the structural type beside it, and ADR 0036 gives it fields and no methods
/// at all. ADR 0036 § 4 answered the *property* half of an erased receiver
/// with a name-keyed runtime fetch and deliberately stopped there — a call
/// additionally needs an argument list checked against a signature and a
/// return type for the position it sits in, and an erased receiver supplies
/// neither. There is no `__call` to fall back on either (ADR 0014), so the
/// call is refused where it is written rather than reaching `mwl-ir` with no
/// resolved target.
///
/// Both narrowing spellings that recover a class are named in the help, and
/// both already lower: `instanceof` proves it inside the guarded branch, and
/// `as ClassName` converts to it or throws.
fn report_method_on_erased_receiver(span: Span, name: &str, ty: TypeId, env: &mut Env<'_>) {
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_METHOD_ON_ERASED_RECEIVER,
            format!("`{described}` names no class, so it has no method `{name}`"),
        )
        .with_primary(span, "called on an erased receiver here")
        .with_help(format!(
            "narrow the receiver to the class that declares `{name}` first — \
             `if ($x instanceof ClassName) {{ … }}`, or `$x as ClassName`; ADR 0007 § 3 makes \
             `object` the opaque top of every class type, and ADR 0036 § 4 erases a property \
             access through one but not a call"
        )),
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
        if param.by_ref {
            report_by_reference_parameter(param, env);
        }
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
    // A closure's body is its own function: an enclosing loop's `break`
    // targets are not reachable from inside it, so the two counters
    // `mwl_types::locals` keeps start again at zero and are put back
    // afterwards. Without this, `foreach (...) { $f = fn (): void => {
    // break; }; }` would count the outer loop as a target it could leave.
    let outer_targets = std::mem::take(&mut env.exit_targets);
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
    env.exit_targets = outer_targets;

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

/// ADR 0031 § 4's opaque `callable`, as a refusal: a closure declares no `&$x`
/// parameter.
///
/// A by-reference parameter is a contract between a *call site* and a
/// declaration — the site stages the cell, hands over its address and copies
/// back afterwards (`mwl_ir::lower::call`). A closure's type is `callable` and
/// nothing else (§ 4), carrying no parameter list for a site to read, so there
/// is no site that could know to stage anything; and § 2's by-value capture
/// lets a closure outlive every frame in scope where it was written, so even
/// naming one would not make the cell outlast it. The by-reference half of § 2
/// was removed for the same reason it is refused here.
///
/// Reported once per by-reference parameter, and the parameter is then bound
/// as an ordinary one so the body checks against its declared type instead of
/// reporting an undefined name at every use.
fn report_by_reference_parameter(param: &mwl_syntax::ast::Param, env: &mut Env<'_>) {
    let name = span_text(env.src, param.name).to_owned();
    env.diags.report(
        Diagnostic::error(
            code::E_CLOSURE_BY_REF_PARAM,
            format!("a closure cannot take `{name}` by reference"),
        )
        .with_primary(param.name, "declared by reference here")
        .with_help(
            "ADR 0031 § 4: a closure's type is `callable`, which carries no parameter list, so \
             no call site knows to stage a cell — take the value and `return` the result, or \
             pass an object, whose fields a closure shares by capturing it",
        ),
    );
}
