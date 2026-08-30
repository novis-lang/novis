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
//! [`report_method_on_erased_receiver`] instead; `mixed` is the one that
//! neither resolves nor is wrong, and records
//! [`crate::expr_table::ExprInfo::ErasedCall`] for the run-time dispatch ADR
//! 0036 § 4 defers it to. [`resolved_call`] is the record `nvs-ir` reads back
//! (see [`crate::expr_table`]), and it always carries the *declaring* class
//! rather than the receiver's.
//!
//! **ADR 0027 (`callable` is closures only)** lives here:
//! [`report_non_callable_value_if_applicable`] gives a bare string or
//! `[$obj, 'method']`-shaped array literal a targeted diagnostic naming the
//! first-class-callable-syntax replacement wherever `callable` is the expected
//! type, ahead of [`is_assignable`]'s generic mismatch (which would otherwise
//! also fire for the same expression); [`report_call_on_non_callable`] refuses
//! `$obj(...)` for any `$obj` whose static type is a resolved class — Novis has
//! no `__invoke`, so no class ever makes `()` mean anything else.
//! [`check_fn_literal`] is the other half of the same ADR pair: a closure
//! literal's body is checked like any other body, and it owns ADR 0031's
//! capture rule and the one shape it refuses (a block body with no declared
//! return type).
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// `$obj->method(...)` / `$obj?->method(...)` — [`super::infer`]'s
/// `ExprKind::MethodCall` arm.
///
/// Unlike a static call, `nvs_hir::members` never checks an instance method
/// call's existence for any receiver — including `$this` — so this is the
/// first and only place it is diagnosed.
#[expect(
    clippy::too_many_arguments,
    reason = "the four-part checking context every function in this module \
              threads — live set, scope, ctx, env — plus the call expression \
              and the four parts of the syntax it destructures"
)]
pub(crate) fn infer_method_call(
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
            // A `Core` receiver is held to the registry here for the same reason
            // the static path holds one at [`infer_static_call`]: the registry
            // states every member `Core` has, so a name it does not hold is
            // knowably wrong *here*, and left unreported it reaches `nvs-ir` as
            // an instance call with no resolved target recorded — which panics
            // (`nvs_ir::lower::expr`'s own message says so). A receiver's class
            // is always one the registry named, since the only way to hold a
            // `Core` instance is to have been given one by a row that returns it.
            if found.is_none() {
                if qname.is_reserved_global_class() {
                    report_exception_accessor(object.span, &qname, &name, env);
                } else {
                    report_unknown_member(object.span, &qname, &name, "method", env);
                }
            }
            if let Some((owner, sig)) = &found {
                check_method_visibility(owner, &name, sig, *name_span, ctx, env);
            }
            // The *declaring* class, not the receiver's: that is what
            // `ResolvedCall::class` promises, and `nvs-ir` renders the call's
            // target label from it — `$dog->name()` on a `Dog` that inherits
            // `name` must name `Animal::name`, the symbol that actually exists.
            found.map(|(owner, sig)| (owner, name, sig))
        }
        _ => None,
    };
    if resolved.is_none()
        && let MemberName::Ident(name_span) = method
        && !matches!(env.interner.get(receiver_ty), Ty::Mixed)
        && class_qname_of(receiver_ty, env.interner).is_none()
        // A nullable receiver whose non-`null` half *is* a class already took
        // `E_NULLABLE_RECEIVER` on the way in ([`strip_nullsafe_receiver`]),
        // and is one mistake rather than two.
        && class_qname_of(env.interner.without_null(receiver_ty), env.interner).is_none()
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
    let (arg_types, slots, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
    // ADR 0088 § 2's contagion, decided here because `resolved_call` below
    // takes `slots` by value and it is the slots that say which parameter each
    // tainted argument filled. See [`carries_contagion`].
    let contagious = sig
        .as_ref()
        .is_some_and(|s| carries_contagion(s, &slots, &arg_types, env.interner));
    // ADR 0057 § 1's closed list, at the one point in an instance call where
    // the target is resolved and the arguments are typed — `$when->format("y")`
    // is the shape that reaches it here. See [`crate::intrinsics`], which
    // reports and replaces nothing.
    if let Some((owner, name, _)) = &resolved {
        crate::intrinsics::check_call(owner, name, args, &arg_types, env);
    }
    // ADR 0036 § 4's deferral, and the one receiver the refusal above
    // deliberately leaves alone: `mixed` is ADR 0007 § 2's one unchecked
    // position, so which class is behind the handle — and whether there is one
    // at all — is answered by the receiver's own descriptor when the call runs
    // (`docs/adr/README.md` § *Decisions taken at project start*). What the
    // site can still settle it settles here; the rest is recorded for `nvs-ir`
    // to dispatch on.
    if resolved.is_none()
        && matches!(env.interner.get(receiver_ty), Ty::Mixed)
        && let MemberName::Ident(name_span) = method
    {
        let name = span_text(env.src, *name_span).to_owned();
        if matches!(args, CallArgs::FirstClassCallable) {
            report_first_class_callable_on_erased_receiver(expr.span, &name, env);
        } else {
            report_args_with_no_parameter_list(args, NoParameterList::ErasedReceiver, env);
            env.exprs.record(expr.span, ExprInfo::ErasedCall { name });
        }
    }
    // ADR 0027: `$obj->method(...)` (first-class callable syntax) names a
    // `Closure` value, not the method's return type — the sentinel
    // `CallArgs::FirstClassCallable` marks exactly this shape, ahead of the
    // ordinary-call typing below. The target is still recorded, as
    // `ExprInfo::CallableRef`: a closure carries its callee with it, so
    // `nvs-ir` needs the same resolved facts a call needs. The erased and
    // `mixed` receivers are already refused above and reach this with
    // `resolved` at `None`, which records nothing.
    if matches!(args, CallArgs::FirstClassCallable) {
        if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig)
            && !reject_unforwardable_first_class_callable(qname, name, sig, expr.span, env)
        {
            let call = resolved_call(qname.clone(), name.clone(), sig, slots, env.signatures);
            env.exprs.record(expr.span, ExprInfo::CallableRef(call));
        }
        return env.interner.callable();
    }
    // `nvs-ir` needs this call's resolved target (not just its return type) to
    // lower an eventual instance-call instruction — see `crate::expr_table`'s
    // own module docs. The *substituted* signature, never the one
    // `resolve_method` returned: `crate::generics` guarantees a type variable
    // never survives a call site, and this record is the one thing that carries
    // a signature past it.
    if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig) {
        let call = resolved_call(qname.clone(), name.clone(), sig, slots, env.signatures);
        env.exprs.record(expr.span, ExprInfo::Call(call));
    }
    // ADR 0008 § 1's late static binding, as a type: a member declaring
    // `static` answers the *called* class, which at an instance call is the
    // receiver's own. `MethodSig::returns_static` owns why substituting here
    // is sound, and why an unresolved (or erased) receiver falls back to the
    // declared type rather than guessing.
    let returned = match &sig {
        Some(s) if s.returns_static && matches!(env.interner.get(receiver_ty), Ty::Class(..)) => {
            receiver_ty
        }
        Some(s) => s.return_ty,
        None => env.interner.mixed(),
    };
    // A contagious member's answer carries its arguments' `tainted` —
    // [`tainted_result`] owns where in a result the qualifier can land.
    let returned = if contagious {
        tainted_result(returned, env.interner)
    } else {
        returned
    };
    nullsafe_result(nullsafe, object_ty, returned, env)
}

/// `Class::method(...)` — [`super::infer`]'s `ExprKind::StaticCall` arm.
///
/// `nvs_hir::members` already checks this reference's existence
/// (`self::`/`static::`/`parent::`/an explicit class name), so this only
/// recovers the call's *type* when a signature resolves, and adds no second
/// diagnostic when it doesn't.
#[expect(
    clippy::too_many_arguments,
    reason = "the same context [`infer_method_call`] threads, with the class \
              expression in place of the receiver and its nullsafe flag"
)]
pub(crate) fn infer_static_call(
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
    // A class side that is not a written name is the same mistake `new $c()`
    // and `$x instanceof $c` make, and gets the same report — see
    // [`super::members::reject_dynamic_class_name`]. Everything below resolves
    // to nothing for such a side, so it would otherwise reach `nvs-ir` as a
    // static call with no target recorded, which panics.
    if !is_written_class_side(class) {
        reject_dynamic_class_name(
            "the class side of a `::` call must be a written class name",
            class.span,
            env,
        );
    }
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
            // [`super::members::infer_class_const`] explains: `nvs_hir` waves
            // every `Core\…::anything` through because nothing declares it, but
            // `nvs_stdlib::registry` states every member `Core` has, so a name
            // that is not one is knowably wrong *here*. Without this a typo
            // reaches `nvs-ir` as a static call with no resolved target
            // recorded, which panics.
            if found.is_none() && qname.is_core() {
                report_unknown_member(expr.span, &qname, &name, "member", env);
            }
            // ADR 0063 R20's one genuinely reachable two-spellings case — see
            // `report_core_instance_member`. Its user-class sibling asks the
            // narrower question `report_instance_method_called_statically`
            // owns: `self::f()`/`parent::f()` from an instance method forward
            // that frame's `$this` and are the ordinary spelling, so what is
            // refused is a non-static target reached where no receiver is in
            // scope — the frame `nvs_ir::lower::expr` would panic on. ADR
            // 0027's `Class::method(...)` is not that frame and not a call:
            // it names the method, and `Core\Attributes::get<T>(C::m(...))`
            // folds it at check time without ever needing a receiver
            // (`docs/reference/lang/90-attributes.md`).
            if let Some((owner, _, sig)) = &found
                && !sig.is_static
            {
                if owner.is_core() {
                    report_core_instance_member(expr.span, owner, &name, env);
                } else if !scope.holds_receiver() && !matches!(args, CallArgs::FirstClassCallable) {
                    report_instance_method_called_statically(expr.span, owner, &name, env);
                }
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
    let (arg_types, slots, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
    // See [`infer_method_call`]: the same question, before the same `slots`
    // are handed to [`resolved_call`]. The two folds below return ahead of it
    // deliberately — a retrieval's and an enumeration's arguments are the
    // literals those folds require, so there is no argument left to carry a
    // qualifier by the time either answers.
    let contagious = sig
        .as_ref()
        .is_some_and(|s| carries_contagion(s, &slots, &arg_types, env.interner));
    // ADR 0033 § 4's debug-dump sink, at the one end where the qualifier is
    // still visible — both members declare `mixed`, so nothing below this
    // point can tell. See [`reject_secret_debug_argument`].
    if let Some((owner, name, _)) = &resolved {
        reject_secret_debug_argument(owner, name, args, &arg_types, env);
        // ADR 0033 § 4's cross-boundary sink, at the same end and for the same
        // reason — `Core\Serialize::encode` declares `mixed` too. See
        // [`reject_secret_boundary_argument`], which the `spawn` forms will
        // reach rather than growing a second rule.
        reject_secret_boundary_argument(owner, name, args, &arg_types, env);
        // ADR 0033 § 4's serialiser sink, the third member of the same shape:
        // `Core\Json::encode` declares `mixed` too, and what it walks is the
        // whole value. See [`reject_secret_encoded_argument`].
        reject_secret_encoded_argument(owner, name, args, &arg_types, env);
        // ADR 0057 § 1's closed list — [`infer_method_call`]'s arm of the same
        // hook, for the `Core\Str::format(…)` / `Core\Regex::compile(…)` half
        // of the roster. See [`crate::intrinsics`].
        crate::intrinsics::check_call(owner, name, args, &arg_types, env);
    }
    // See [`infer_method_call`]: first-class callable syntax names a `Closure`,
    // not the resolved method's return type, and records `CallableRef` rather
    // than `Call` for the same span. `static_class` is set here exactly as it
    // is for a call — ADR 0027 § 1 keeps `static::helper(...)` late-bound.
    if matches!(args, CallArgs::FirstClassCallable) {
        if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig)
            && !reject_unforwardable_first_class_callable(qname, name, sig, expr.span, env)
        {
            let mut call = resolved_call(qname.clone(), name.clone(), sig, slots, env.signatures);
            if matches!(class.kind, ExprKind::ConstFetch(_)) {
                call.static_class = resolve_class_expr(class, ctx, env);
            }
            env.exprs.record(expr.span, ExprInfo::CallableRef(call));
        }
        return env.interner.callable();
    }
    // ADR 0046 §§ 4-5's retrieval, which is not a call at all once it has been
    // resolved: the answer is recorded against this span as an ordinary
    // compile-time constant, so the `ExprInfo::Call` below must *not* also be
    // recorded — one span carries one entry, and `nvs-ir` would then lower the
    // call it was told to replace. See [`crate::retrieval`].
    if let Some((qname, name, _)) = &resolved
        && crate::retrieval::is_retrieval(qname, name)
    {
        let name = name.clone();
        crate::retrieval::fold_retrieval(expr, &name, &written, args, ctx, env);
        return sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty);
    }
    // ADR 0061 § 3's enumeration, which is not a call at all once it has been
    // answered — the arm above's reasoning, for a fold whose answer allocates.
    // See [`crate::program`].
    if let Some((qname, name, _)) = &resolved
        && crate::program::is_enumeration(qname, name)
    {
        crate::program::expand(expr, &written, env);
        return sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty);
    }
    // See [`infer_method_call`]: persisted for `nvs-ir` to read back a resolved
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
    // ADR 0077 § 4's link, and the one fold that is *not* made here: the route
    // a literal name asks for may be declared in a file § 5's scan has not
    // reached, so the site is only recorded and the lookup happens after the
    // whole walk. The `ExprInfo::Call` just above deliberately stands until
    // then — `crate::links` records over it, and a computed name keeps it.
    if let Some((qname, name, _)) = &resolved
        && crate::links::is_link(qname, name)
    {
        let name = name.clone();
        crate::links::record_site(expr, &name, args, env);
    }
    // The static-call half of the same substitution the instance-call arm
    // above documents — see `MethodSig::returns_static`.
    let returned = match &sig {
        Some(s) if s.returns_static => match called_class_of(class, ctx, env) {
            Some(called) => env.interner.class(called),
            None => s.return_ty,
        },
        Some(s) => s.return_ty,
        None => env.interner.mixed(),
    };
    // See [`infer_method_call`]: the static half of the same contagion.
    if contagious {
        tainted_result(returned, env.interner)
    } else {
        returned
    }
}

/// The class a `Class::member()` site *calls on*, for ADR 0008 § 1's `static`
/// return type.
///
/// Not [`resolve_class_expr`], and the difference is `parent::`: that resolver
/// answers the class the member is looked up on, while late static binding
/// forwards the caller's called class through all three of
/// `self`/`static`/`parent`. The enclosing class is the tightest sound answer
/// for those, and an explicitly named class is its own.
fn called_class_of(class: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<QName> {
    match &class.kind {
        ExprKind::SelfExpr | ExprKind::StaticExpr | ExprKind::ParentExpr => {
            ctx.current_class.cloned()
        }
        ExprKind::ConstFetch(_) => resolve_class_expr(class, ctx, env),
        _ => None,
    }
}

/// `new Target(...)` — [`super::infer`]'s `ExprKind::New` arm.
///
/// A class with no explicit `constructor` accepts a bare `new Foo()` in PHP
/// and nothing else, which is what [`reject_arguments_to_implicit_constructor`]
/// holds it to. The *declaring* class is kept, not the
/// constructed one: `new Dog(...)` on a `Dog extends Animal` that declares no
/// constructor of its own invokes `Animal::constructor`, and `nvs-ir` cannot
/// re-walk the hierarchy to find that out (see
/// `crate::expr_table::ExprInfo::New::ctor`).
#[expect(
    clippy::too_many_arguments,
    reason = "one checker context threaded positionally, as everywhere else here"
)]
pub(crate) fn infer_new(
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
    report_first_class_callable_new(args, target_qname.as_ref(), expr.span, env);
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
        reject_abstract_instantiation(target, qname, expr.span, env);
        reject_secret_throwable_message(qname, arg_types.first().copied(), expr.span, env);
        // A `Core`-owned class has no constructor and never will: its instances
        // come from the member that produces one, and its slots are
        // `nvs-stdlib`'s layout rather than a surface a program fills in
        // (`nvs_stdlib::registry::CoreTy::Instance`). Reported here rather than
        // left to `nvs-codegen`, which would fail with "this unit declares no
        // descriptor for it" — an internal message for an ordinary mistake.
        //
        // The carve-out is `registry::CONSTRUCTORS`: spec § 9's collections are
        // written `new Core\ObjectSet<Tag>()`, and they still declare no
        // `constructor` *member* — what makes them constructible is a native
        // symbol `nvs-ir` lowers straight to, which `nvs_stdlib::instance`'s
        // module docs own. So the class is a legal `new` target while
        // `Core\ObjectSet::constructor` remains an unknown member.
        if crate::core_lib::is_registered(qname)
            && nvs_stdlib::registry::constructor_symbol(&qname.to_string()).is_none()
        {
            report_unknown_member(expr.span, qname, "constructor", "member", env);
        }
        reject_arguments_to_implicit_constructor(args, qname, sig.as_ref(), expr.span, env);
        // `nvs-ir` needs the constructed class and its resolved constructor (if
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

/// Refuses `new` on a declaration that has no instances — an `abstract`
/// class or an interface (`E_ABSTRACT_INSTANTIATED`).
///
/// The question is [`nvs_hir::ClassLinks::concrete`], asked of the class
/// graph rather than answered from a table of this crate's own: that field
/// exists because `implementors` already needed it, and its own doc comment
/// says why a second table consulted per candidate would only be a second
/// place for the two to disagree. One lookup covers both shapes, which are
/// one mistake — a target whose members are not all filled in — and the
/// message names which of the two was written.
///
/// **`new static()` is exempt, and must stay so.** Late static binding
/// resolves it to the concrete subclass the call arrived through, so an
/// `abstract` class calling `new static()` in a factory method is the idiom
/// rather than the error; `new self()` and `new parent()` do name their
/// class and are refused like any other written name. Reported here rather
/// than left to `nvs-ir`, which lowers an allocation of a class whose
/// methods have no compiled function behind them and fails at run time with
/// `FATAL: internal error: a method with no body was called`.
fn reject_abstract_instantiation(target: &NewTarget, qname: &QName, span: Span, env: &mut Env<'_>) {
    if matches!(target, NewTarget::StaticTy) {
        return;
    }
    if env.graph.get(qname).map(|links| links.concrete) != Some(false) {
        return;
    }
    let is_interface =
        env.symbols.get(qname).map(|symbol| symbol.kind) == Some(nvs_hir::SymbolKind::Interface);
    let (what, fix) = if is_interface {
        (
            format!("`{qname}` is an interface, so it has no instances"),
            format!("`new` a class that implements `{qname}`"),
        )
    } else {
        (
            format!("`{qname}` is `abstract`, so it has no instances"),
            format!(
                "`new` a subclass that gives every open member a body, or drop `abstract` from \
                 `{qname}`'s own declaration"
            ),
        )
    };
    env.diags.report(
        Diagnostic::error(code::E_ABSTRACT_INSTANTIATED, what)
            .with_primary(span, "allocated here")
            .with_help(fix),
    );
}

/// Refuses `new C(...)` — the first-class callable sentinel written on `new`
/// (`E_FIRST_CLASS_CALLABLE_NEW`).
///
/// ADR 0027 § 1 keeps the spelling for *members*, and a constructor is not
/// one: the closure it builds carries a callee, and `new` names a class. PHP
/// refuses the same expression, so this is the compatible answer as well as
/// the only one with a meaning. Reported ahead of everything else `new`
/// checks, and reported rather than left to `nvs-ir`, which would otherwise
/// reach `lower_call_args` with a sentinel where an argument list belongs —
/// this is the one shape that got a resolved `new` there at all.
/// ADR 0027 § 1's `(...)` over a member a `callable` cannot forward to —
/// `E_FIRST_CLASS_CALLABLE_UNFORWARDABLE`, whose own docs own the rule.
/// Answers `true` when it refused, which is when nothing is recorded: the
/// pipeline stops at the first error, so `nvs-ir` never looks for the entry.
///
/// Both halves are the *callee's* declaration rather than anything the site
/// wrote, so the primary span is the `(...)` and the help names the member.
/// Reported here rather than left to the lowering because either one is a
/// type confusion in the callee's own frame — it reads the slot at the
/// declared representation, so an `inout` gets an `int` where an address
/// belongs and a variadic tail gets an `int` where the collected array does.
fn reject_unforwardable_first_class_callable(
    owner: &QName,
    name: &str,
    sig: &MethodSig,
    span: Span,
    env: &mut Env<'_>,
) -> bool {
    let offending = if sig.inout.iter().any(|&by_ref| by_ref) {
        "an `inout` parameter"
    } else if sig.variadic {
        "a variadic parameter"
    } else {
        return false;
    };
    env.diags.report(
        Diagnostic::error(
            code::E_FIRST_CLASS_CALLABLE_UNFORWARDABLE,
            format!("`{owner}::{name}` has no first-class callable form"),
        )
        .with_primary(span, format!("this names a member declaring {offending}"))
        .with_help(
            "ADR 0031 § 4 gives `callable` no parameter list, so a call through one cannot \
             stage a by-reference cell or collect a variadic tail — the callee would read \
             the slot at the wrong representation. Write the closure out over the \
             arguments the caller does pass"
                .to_owned(),
        ),
    );
    true
}

fn report_first_class_callable_new(
    args: &CallArgs,
    target: Option<&QName>,
    span: Span,
    env: &mut Env<'_>,
) {
    if !matches!(args, CallArgs::FirstClassCallable) {
        return;
    }
    let named = target.map_or_else(|| "this class".to_owned(), |q| format!("`{q}`"));
    env.diags.report(
        Diagnostic::error(
            code::E_FIRST_CLASS_CALLABLE_NEW,
            "`new` has no first-class callable form".to_owned(),
        )
        .with_primary(span, format!("{named} is constructed here, not called"))
        .with_help(
            "ADR 0027 § 1 gives the `(...)` spelling to a member — `Class::method(...)`, \
             `$obj->method(...)`, `self::method(...)` — and a constructor is not one. Write the \
             closure out: `fn (): T => new T(…)`"
                .to_owned(),
        ),
    );
}

/// Holds `new C(...)` on a class that declares no `constructor` to the zero
/// arguments such a class can accept (`E_ARITY_MISMATCH`).
///
/// [`check_positional_arity`](super::args) is a count against a *signature*,
/// so a class with no constructor had no signature to be counted against and
/// every argument written there was inferred, checked against nothing and
/// dropped: `new Plain(1, 2)` compiled and ran, constructing exactly what
/// `new Plain()` constructs. PHP refuses it, ADR 0007 § 1's "nothing is
/// untyped" leaves an unchecked argument no home, and `nvs-ir` lowers `new`
/// with `ctor: None` — so the arguments were not even evaluated for their
/// effects.
///
/// Two targets are exempt and neither is a class the author declared. A
/// **`Core`-owned** class is constructed by a native symbol rather than by a
/// `constructor` member ([`nvs_stdlib::registry::constructor_symbol`], and
/// the neighbouring check refuses the ones that have none), so its argument
/// list is that symbol's rather than a signature's. A class this unit has
/// **no signature for at all** has already been reported as unknown, and a
/// second diagnostic about how many arguments it does not take would name the
/// mistake twice.
fn reject_arguments_to_implicit_constructor(
    args: &CallArgs,
    qname: &QName,
    sig: Option<&MethodSig>,
    call_span: Span,
    env: &mut Env<'_>,
) {
    let CallArgs::List(list) = args else {
        return;
    };
    if sig.is_some() || list.is_empty() {
        return;
    }
    if crate::core_lib::is_registered(qname) || env.signatures.get(qname).is_none() {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ARITY_MISMATCH,
            format!(
                "expected 0 argument(s), found {} — `{qname}` declares no `constructor`",
                list.len()
            ),
        )
        .with_primary(call_span, "called here")
        .with_help("drop the arguments, or declare a `constructor` on the class that takes them"),
    );
}

/// The `<...>` written between a `new` target and its `(` —
/// `<Tag>` in `new Core\ObjectSet<Tag>()`.
///
/// Every written type is lowered whichever way this goes, so an unknown class
/// named inside one is reported even when the list itself is refused; that is
/// [`super::args::check_written_type_args`]'s rule at a call site, for the
/// same reason.
///
/// **The roster is [`nvs_stdlib::registry::GENERIC_CLASSES`].** ADR 0007 § 3
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
        nvs_stdlib::registry::class_type_params(&qname.to_string()).map(|params| (qname, params))
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
/// into the shape `nvs-ir` reads back, so the `MethodCall`/`StaticCall`/`New`
/// arms below don't each repeat the field list.
pub(crate) fn resolved_call(
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
        inout: sig.inout.clone(),
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

pub(crate) fn report_non_callable_value_if_applicable(expr: &Expr, env: &mut Env<'_>) -> bool {
    match &expr.kind {
        ExprKind::Str(_) | ExprKind::Interpolated(_) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_CALLABLE_STRING_UNSUPPORTED,
                    "a string is not callable in Novis; take a reference with first-class \
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
                    "an array is not callable in Novis; take a reference with first-class \
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
/// resolves to a class — Novis has no `__invoke`, so no class ever makes `()`
/// mean anything else, regardless of what methods it declares. A `Ty::Mixed`
/// callee (nothing statically known) and an already-`Ty::Callable` one are
/// both left alone.
pub(crate) fn report_call_on_non_callable(callee_ty: TypeId, span: Span, env: &mut Env<'_>) {
    let Ty::Class(qname, _) = env.interner.get(callee_ty).clone() else {
        return;
    };
    env.diags.report(
        Diagnostic::error(
            code::E_NOT_CALLABLE,
            format!(
                "`{qname}` is not callable; Novis has no `__invoke` — call a named method \
                 instead, e.g. `$obj->methodName(...)`"
            ),
        )
        .with_primary(span, "called with `(...)` here"),
    );
}

/// `$m->method(...)` — ADR 0027's first-class callable spelling on a `mixed`
/// receiver, which is the one shape of that receiver's deferral that has no
/// run-time answer.
///
/// A *call* through a `mixed` defers to the receiver's own descriptor, which
/// is present at the call and marshals it. This spelling makes no call: it
/// names a closure **value**, and a closure carries its callee's arity and
/// parameter tags in the value itself (`nvs_runtime::closure`), so building
/// one here would mean reading a method row off a receiver for a value that
/// outlives the site and may be called anywhere. That is a mechanism rather
/// than a lowering, and no ADR asks for it — so the spelling is refused where
/// it is written, and the two fixes that exist are what the help names.
fn report_first_class_callable_on_erased_receiver(span: Span, name: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_FIRST_CLASS_CALLABLE_ERASED_RECEIVER,
            format!("`{name}(...)` names no closure through a `mixed` receiver"),
        )
        .with_primary(span, "a closure value is named here")
        .with_help(format!(
            "ADR 0036 § 4 defers a *call* through a `mixed` to the receiver's runtime class, but \
             a closure value carries its callee with it and there is no class here to read one \
             off — call the member directly (`$m->{name}(…)`), or narrow the receiver first with \
             `instanceof` or `as ClassName`"
        )),
    );
}

/// Which call site is asking [`report_args_with_no_parameter_list`] — the two
/// places a call reaches a callee no signature at this site describes.
///
/// They differ in *why* there is no parameter list and therefore in what the
/// help says, and in nothing else: neither the site nor the run time can
/// resolve a name against a callee it does not name, so the rule and its codes
/// are one.
#[derive(Clone, Copy)]
pub(crate) enum NoParameterList {
    /// `$fn(...)` — [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md)
    /// § 1's opaque `callable`.
    Callable,
    /// `$m->method(...)` on a `mixed` receiver — ADR 0036 § 4's deferral, whose
    /// callee is whatever the receiver's runtime class answers.
    ErasedReceiver,
}

/// A call whose callee this site cannot name, refused from the call site's end
/// rather than the declaration's ([`report_by_reference_parameter`] is the
/// other end of the same rule): it may write neither a `name:` argument nor an
/// `inout` marker.
///
/// Through a `callable` the callee is one opaque type whatever closure the
/// variable holds, so this site has no parameter list to resolve a name
/// against — and neither has the run time, a closure object recording its
/// arity and its parameter *tags* and never their names
/// (`nvs_runtime::closure`). Through a `mixed` receiver the callee is not
/// chosen until the call runs, and the method row that marshals it carries
/// exactly the same two facts for exactly that reason. PHP allows the spelling
/// only because a `Closure` there carries its whole declaration.
///
/// The `inout` marker is refused at both for one reason spelled two ways: no
/// closure may declare such a parameter at all (`E_CLOSURE_INOUT_PARAM`), and
/// an `inout` parameter list is packed and written back at the *call site*,
/// which a call that learns its callee at run time cannot do — the same limit
/// [`E_DELEGATE_MEMBER_NOT_FORWARDABLE`] names for ADR 0043 § 4's synthesized
/// forward.
///
/// A `...` argument is left alone and lowers: how many arguments it hands over
/// is its own run-time length, which needs no parameter list to mean something
/// (`nvs_ir::Helper::CallClosureArray`). What still applies is rule 1 of
/// [`super::args::map_arguments`] — a positional argument cannot follow a `...`
/// — for the same reason it applies at a resolved call, so the two refusals are
/// one walk.
pub(crate) fn report_args_with_no_parameter_list(
    args: &CallArgs,
    callee: NoParameterList,
    env: &mut Env<'_>,
) {
    let CallArgs::List(list) = args else {
        return;
    };
    let described = match callee {
        NoParameterList::Callable => "a `callable`",
        NoParameterList::ErasedReceiver => "a call through a `mixed` receiver",
    };
    let inout_help = match callee {
        NoParameterList::Callable => {
            "ADR 0031 § 1 keeps `callable` opaque and § 4 refuses an `inout` closure parameter \
             outright, so nothing this call reaches can bind one — drop the `inout`"
        }
        NoParameterList::ErasedReceiver => {
            "ADR 0036 § 4 defers this call to the receiver's runtime class, and an `inout` \
             parameter list is packed and written back here at the call site — so a callee that \
             is not known until the call runs can never bind one; narrow the receiver with \
             `instanceof` or `as ClassName` if the write-back is what was meant"
        }
    };
    let name_help = match callee {
        NoParameterList::Callable => {
            "ADR 0031 § 1: `callable` is one opaque type whatever closure the variable holds, so \
             neither this call site nor the closure it reaches carries a parameter name to fill \
             — pass the argument positionally"
        }
        NoParameterList::ErasedReceiver => {
            "ADR 0036 § 4: the receiver's runtime class chooses the callee, and the method row \
             that marshals the call carries its arity and its parameter tags rather than their \
             names — pass the argument positionally, or narrow the receiver to the class that \
             declares the member"
        }
    };
    let mut positional_ends: Option<Span> = None;
    for arg in list {
        // ADR 0107 § 2's marker has the same nothing to resolve against, one
        // step worse: neither callee can bind such a parameter at all.
        if arg.inout {
            env.diags.report(
                Diagnostic::error(
                    code::E_INOUT_ARG_UNEXPECTED,
                    format!("`inout` names no parameter of {described}"),
                )
                .with_primary(arg.span, "marked `inout` here")
                .with_help(inout_help),
            );
        }
        if let Some(name) = arg.name {
            positional_ends.get_or_insert(arg.span);
            let name = span_text(env.src, name).to_owned();
            env.diags.report(
                Diagnostic::error(
                    code::E_NAMED_ARG_THROUGH_CALLABLE,
                    format!("`{name}:` names no parameter of {described}"),
                )
                .with_primary(arg.span, "written by name here")
                .with_help(name_help),
            );
        } else if arg.spread {
            positional_ends.get_or_insert(arg.span);
        } else if let Some(first) = positional_ends {
            super::args::report_positional_after_named(arg, first, env);
        }
    }
}

/// A method called on a receiver whose type names no class at all — a plain
/// `object`, an ADR 0036 shape, a union naming no single class, an
/// intersection, or a type that can hold no object in the first place (a
/// scalar, an `array<T>`, a `callable`, a `void` call's result).
///
/// It is **one** code across that whole family, because it is one mistake: a
/// method is resolved against a class, and none of these names one. ADR 0007
/// § 3 makes `object` the opaque top of every class type — a pointer with the
/// class label erased, listing no members — and ADR 0036 gives a shape fields
/// and no methods at all; a union names several classes or none, and a
/// `Dog|Cat` receiver has no one signature for the argument list to be checked
/// against or for the call's position to take its type from. That is also why
/// the *property* half splits where this one does not: ADR 0036 § 4 answers an
/// erased property read with a name-keyed runtime fetch and refuses the rest
/// (`E_RECEIVER_HAS_NO_PROPERTIES`), while a call additionally needs a
/// signature and a return type, which no receiver here supplies. There is no
/// `__call` to fall back on either (ADR 0014), so every one of them is refused
/// where it is written rather than reaching `nvs-ir` with no resolved target.
///
/// `mixed` is deliberately **not** here: ADR 0007 § 2 makes it the one
/// unchecked position and ADR 0036 § 4 defers it to a run-time answer, which
/// is [`crate::expr`]'s own next slice rather than a refusal.
///
/// The help splits three ways because the fix does. A receiver that can hold
/// an object is narrowed — both spellings already lower, `instanceof` proving
/// the class inside the guarded branch (`crate::locals::instanceof_residue`)
/// and `as ClassName` converting to it or throwing. One that cannot
/// ([`can_hold_an_object`]) has nothing to narrow, so the help is the
/// property half's: convert, or declare the receiver `mixed` and take the
/// deferral. A `void` call has no value at all, so neither applies.
fn report_method_on_erased_receiver(span: Span, name: &str, ty: TypeId, env: &mut Env<'_>) {
    let described = env.interner.describe(ty);
    let help = if matches!(env.interner.get(ty), Ty::Void) {
        format!(
            "a call that returns `void` yields no value, so there is nothing for `{name}` to be \
             called on — call `{name}` on the receiver you meant"
        )
    } else if can_hold_an_object(ty, env.interner) {
        format!(
            "narrow the receiver to the class that declares `{name}` first — \
             `if ($x instanceof ClassName) {{ … }}`, or `$x as ClassName`; ADR 0007 § 3 makes \
             `object` the opaque top of every class type, and ADR 0036 § 4 erases a property \
             access through one but not a call"
        )
    } else {
        format!(
            "only an object has methods — convert the receiver to the class that declares \
             `{name}` (`$x as Box`), or declare it `mixed`, which is the one unchecked position \
             (ADR 0007 § 2) and defers the whole question to a catchable throw at run time"
        )
    };
    env.diags.report(
        Diagnostic::error(
            code::E_METHOD_ON_ERASED_RECEIVER,
            format!("`{described}` names no class, so it has no method `{name}`"),
        )
        .with_primary(span, "called on a receiver that names no class here")
        .with_help(help),
    );
}

/// `$e->getMessage()` — a PHP accessor on the exception tree, which has none.
///
/// [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
/// § 10 gives that tree **properties**, and [`crate::error_lib`] seeds exactly
/// those plus the synthesized constructor, so every PHP accessor resolves to
/// nothing here. Until this existed the tree was exempt from the unknown-member
/// refusal above — an exemption that outlived the seeding it was written for —
/// and `$e->getMessage()` reached `nvs-ir` with no resolved target and panicked
/// there.
///
/// It is the ordinary [`code::E_UNKNOWN_MEMBER`] the property half already
/// reports for `$e->nope`: one mistake, one code, whichever spelling reached
/// it. What is worth a help of its own is that this is the one unknown method a
/// *ported* program writes on purpose, so the help names the property that
/// answers the same question, and the roster it names is
/// [`nvs_hir::errors::PROPERTIES`] read rather than copied.
fn report_exception_accessor(span: Span, qname: &QName, name: &str, env: &mut Env<'_>) {
    // PHP's accessors, mapped to the property that answers the same question.
    // `getFile`/`getLine` are one property here because a throw site is one
    // string (`crate::error_lib`'s own docs own that shape), and `getCode` has
    // no counterpart at all — ADR 0002 propagates a class, never a number.
    let property = match name {
        "getMessage" => Some("message"),
        "getPrevious" => Some("previous"),
        "getTrace" | "getTraceAsString" => Some("backtrace"),
        "getFile" | "getLine" => Some("location"),
        _ => None,
    };
    let roster = nvs_hir::errors::PROPERTIES
        .iter()
        .map(|p| format!("`{p}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let help = property.map_or_else(
        || format!("the exception tree declares {roster} and no method but its constructor"),
        |property| {
            format!(
                "the exception tree declares properties rather than accessors — write \
                 `->{property}`; spec § 10 lists {roster}"
            )
        },
    );
    env.diags.report(
        Diagnostic::error(
            code::E_UNKNOWN_MEMBER,
            format!("`{qname}` has no method named `{name}`"),
        )
        .with_primary(span, "referenced here")
        .with_help(help),
    );
}

pub(crate) fn check_member_name(
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

pub(crate) fn check_args(
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

pub(crate) fn check_new_target(
    target: &NewTarget,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    match target {
        NewTarget::Name(name) => {
            let text = span_text(env.src, name.span);
            let qname = nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports);
            // A `Core` name is held to the registry rather than to its
            // namespace. `is_core` is a *spelling* test — anything under
            // `Core\` answers it — so accepting one on that alone let
            // `new Core\Bogus()` past every check the checker has and fail
            // in `nvs-codegen` with "this unit declares no descriptor for
            // it", an internal message for an ordinary typo. The registry is
            // the whole roster of `Core` classes
            // ([`nvs_stdlib::registry::CLASSES`]), so a name it does not hold
            // is undeclared in exactly the sense the arm below reports.
            //
            // The exception tree is the second roster, and `Core\Test\Failure`
            // is the only row of it that needs naming here: every other
            // `nvs_hir::errors::TREE` entry is a bare global answered by
            // `is_reserved_global_class`, and that predicate's own docs say
            // the namespaced one is trusted through `is_core` instead —
            // which is exactly the trust this arm just withdrew.
            //
            // A registered `Core` class that has no constructor is a
            // *different* mistake and keeps its own diagnostic: `infer_new`
            // reports the missing `constructor` member, naming the one thing
            // the class is missing rather than the class itself.
            if env.symbols.get(&qname).is_some()
                || (qname.is_core()
                    && (crate::core_lib::is_registered(&qname)
                        || nvs_hir::errors::is_exception_class(&qname.to_string())))
                || qname.is_reserved_global_class()
            {
                env.interner.class(qname)
            } else {
                // Diagnosed rather than erased to `mixed`: `nvs-ir` has no
                // class to allocate and panics naming the missing table entry,
                // which is a worse report of the same fact. The spelling this
                // most often catches is PHP's `new Exception(…)` — spec § 10
                // has no such class, so the ordinary undeclared-class
                // diagnostic is exactly the right answer.
                env.diags.report(nvs_hir::undeclared_name(
                    &qname,
                    text,
                    name.span,
                    ctx.namespace,
                    env.symbols,
                ));
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
            // The parser produces this arm only for a target that is not a
            // written name, so it *is* the dynamic form — see
            // [`super::members::reject_dynamic_class_name`] for why the three
            // spellings of that mistake share one report, and why `nvs-ir` is
            // the wrong place to find out.
            check_expr(e, None, live, scope, ctx, env);
            reject_dynamic_class_name(
                "the target of `new` must be a written class name",
                e.span,
                env,
            );
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
/// call shape that does not exist yet — see `nvs_ir::lower`'s own docs for
/// which closure call sites lower at all.
pub(crate) fn check_fn_literal(
    expr: &Expr,
    f: &FnExpr,
    live: &FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let seq = env.closure_seq;
    env.closure_seq += 1;
    // `$` cannot appear in an Novis identifier, so this label can never collide
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
        if param.inout {
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
        // Not inherited even inside the constructor: a closure runs when it is
        // called, which this checker cannot bound, so a `readonly` write it
        // holds is not proven to happen during construction (ADR 0038 § 1).
        in_constructor: false,
    };
    // A closure's body is its own function: an enclosing loop's `break`
    // targets are not reachable from inside it, so the two counters
    // `nvs_types::locals` keeps start again at zero and are put back
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

/// ADR 0031 § 4's opaque `callable`, as a refusal: a closure declares no `inout $x`
/// parameter.
///
/// A by-reference parameter is a contract between a *call site* and a
/// declaration — the site stages the cell, hands over its address and copies
/// back afterwards (`nvs_ir::lower::call`). A closure's type is `callable` and
/// nothing else (§ 4), carrying no parameter list for a site to read, so there
/// is no site that could know to stage anything; and § 2's by-value capture
/// lets a closure outlive every frame in scope where it was written, so even
/// naming one would not make the cell outlast it. The by-reference half of § 2
/// was removed for the same reason it is refused here.
///
/// Reported once per by-reference parameter, and the parameter is then bound
/// as an ordinary one so the body checks against its declared type instead of
/// reporting an undefined name at every use.
fn report_by_reference_parameter(param: &nvs_syntax::ast::Param, env: &mut Env<'_>) {
    let name = span_text(env.src, param.name).to_owned();
    env.diags.report(
        Diagnostic::error(
            code::E_CLOSURE_INOUT_PARAM,
            format!("a closure cannot take `{name}` as `inout`"),
        )
        .with_primary(param.name, "declared `inout` here")
        .with_help(
            "ADR 0031 § 4: a closure's type is `callable`, which carries no parameter list, so \
             no call site knows to stage a cell — take the value and `return` the result, or \
             pass an object, whose fields a closure shares by capturing it",
        ),
    );
}
