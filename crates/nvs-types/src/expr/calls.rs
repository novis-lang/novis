//! A call's target: which member `$obj->m()`, `C::m()` and `new C()` resolve
//! to, and `rule:types/callable-values`'s rule that only an anonymous function or a
//! method reference is ever callable.
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
//! **`rule:types/callable-values` (a `callable` is an anonymous function or a
//! method reference, nothing else)** lives here:
//! [`report_non_callable_value_if_applicable`] gives a bare string or
//! `[$obj, 'method']`-shaped array literal a targeted diagnostic naming the
//! method-reference replacement wherever `callable` is the expected
//! type, ahead of [`is_assignable`]'s generic mismatch (which would otherwise
//! also fire for the same expression); [`report_call_on_non_callable`] refuses
//! `$obj(...)` for any `$obj` whose static type is a resolved class — Novis has
//! no `__invoke`, so no class ever makes `()` mean anything else.
//! [`check_anon_fn`] is the other half of the same ADR pair: an anonymous
//! function's body is checked like any other body, and it owns `rule:types/anonymous-function`'s
//! capture rule and the one shape it refuses (a block body with no declared
//! return type).
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.
//!
//! ## Known gap
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-types/src/expr/calls.rs` lists them.

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
    live: &mut Live,
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
    // `rule:types/property-key-access`'s first neighbour: a computed name is admitted at a property
    // access and nowhere else, so this is `E0235` whatever the operand's type
    // is. `rule:classes/no-call-magic` refuses computed *dispatch* as a concept rather than as
    // a spelling, and a key names a property, so there is no operand that could
    // make this one resolve.
    if let MemberName::Variable(e) | MemberName::Expr(e) = method {
        report_computed_member_name(e.span, COMPUTED_METHOD_HELP, env);
    }
    if let Some(ty) = infer_callable_rebind(expr, receiver_ty, method, args, live, scope, ctx, env)
    {
        return nullsafe_result(nullsafe, object_ty, ty, env);
    }
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
    let (sig, written) =
        check_written_type_args(type_args, sig, label.as_deref(), expr.span, ctx, env);
    let (arg_types, slots, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
    // `rule:security/unclassified-parameter-refuses-tainted`'s contagion, decided here because `resolved_call` below
    // takes `slots` by value and it is the slots that say which parameter each
    // tainted argument filled. See [`carries_contagion`].
    let contagious = sig
        .as_ref()
        .is_some_and(|s| carries_contagion(s, &slots, &arg_types, env.interner));
    // `rule:expressions/intrinsic-list-is-closed`'s closed list, at the one point in an instance call where
    // the target is resolved and the arguments are typed — `$when->format("y")`
    // is the shape that reaches it here. See [`crate::intrinsics`], which
    // reports and replaces nothing.
    if let Some((owner, name, _)) = &resolved {
        crate::intrinsics::check_call(owner, name, expr.span, args, &arg_types, &slots, env);
        // `rule:core-classes/html-to-source`'s written reason, at the same point and matched the same
        // way — the one pass that refuses an argument *for* being dynamic. See
        // [`crate::reasons`], whose module doc owns why that is not the rule
        // above read backwards.
        crate::reasons::check_call(owner, name, args, &slots, ctx, env);
        // `rule:security/secret-sinks-refuse`'s graph copy at the one *instance*
        // member that makes one: `Core\Cache\Store::put` declares `mixed` for its
        // value, so the written argument is the last place the qualifier is
        // visible, exactly as it is at the static carriers. Slots for the reason
        // `Core\Topic::publish` reads them. See
        // [`super::quals::reject_secret_cached_argument`].
        reject_secret_cached_argument(owner, name, args, &arg_types, &slots, env);
    }
    // `rule:types/erased-member-access`'s deferral, and the one receiver the refusal above
    // deliberately leaves alone: `mixed` is `rule:types/conversion`'s one unchecked
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
        if matches!(args, CallArgs::MethodRef) {
            report_method_ref_on_erased_receiver(expr.span, &name, env);
        } else {
            report_args_with_no_parameter_list(args, NoParameterList::ErasedReceiver, env);
            env.exprs.record(expr.span, ExprInfo::ErasedCall { name });
        }
    }
    // `rule:types/callable-values`: `$obj->method(...)` (a method reference) names a
    // callable value, not the method's return type — the sentinel
    // `CallArgs::MethodRef` marks exactly this shape, ahead of the
    // ordinary-call typing below. The target is still recorded, as
    // `ExprInfo::CallableRef`: a callable carries its callee with it, so
    // `nvs-ir` needs the same resolved facts a call needs. The erased and
    // `mixed` receivers are already refused above and reach this with
    // `resolved` at `None`, which records nothing.
    if matches!(args, CallArgs::MethodRef) {
        if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig)
            && !reject_unforwardable_method_ref(qname, name, sig, expr.span, env)
        {
            let mut call = resolved_call(qname.clone(), name.clone(), sig, slots, env.signatures);
            // The thunk calls the native member with the same leading enum
            // name a call would — see `nvs_stdlib::registry::WRITTEN_ENUM_MEMBERS`.
            call.written_enum = written_enum_of(qname, name, &written, type_args, expr.span, env);
            env.exprs.record(expr.span, ExprInfo::CallableRef(call));
            let held = method_ref_type(sig, env);
            env.exprs.record_callable_value(expr.span, held);
            return held;
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
        let mut call = resolved_call(qname.clone(), name.clone(), sig, slots, env.signatures);
        // `Core\Db\Queryable::queryAs<T>` is the *instance* half of
        // `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS`, and it needs the class
        // its call site wrote for exactly the reason `Core\Json::decodeAs` does
        // — see the static call's own record below. The receiver settles
        // nothing here: `queryAs` produces its `T` rather than reading one off
        // a `Rows` it was called on.
        if let Some((target, list)) =
            written_class_of(qname, name, &written, type_args, expr.span, env)
        {
            target.record_on(&mut call, list);
        }
        // `Core\Router\Match::accessAs<E>` answers a case of the enum its call
        // site wrote, and a case does not say at run time which enum it is
        // from — see `nvs_stdlib::registry::WRITTEN_ENUM_MEMBERS`.
        call.written_enum = written_enum_of(qname, name, &written, type_args, expr.span, env);
        env.exprs.record(expr.span, ExprInfo::Call(call));
    }
    // `rule:statements/static-is-a-member-modifier`'s late static binding, as a type: a member declaring
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
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let class_ty = check_expr(class, None, live, scope, ctx, env);
    super::reject_class_side_outside_class(class, ctx, env);
    check_member_name(method, live, scope, ctx, env);
    // `C::$m()` is [`infer_method_call`]'s refusal written on the class side,
    // and for the same reason — the member a call names is never computed.
    if let MemberName::Variable(e) | MemberName::Expr(e) = method {
        report_computed_member_name(e.span, COMPUTED_METHOD_HELP, env);
    }
    // `rule:types/class-reference-sites`'s second site: a `class<T>` class side resolves the member
    // on `T`'s roster, the only one this site can see. The value may hold any
    // implementor, so what is checked here is `T`'s declaration and what finds
    // the override at run time is § 4's `InstKind::CallVirtual`.
    let class_ref_arg = class_ref_argument(class_ty, env.interner);
    let class_ref = class_ref_arg.and_then(|inner| class_qname_of(inner, env.interner));
    // Kept as a flag because the name itself is consumed by the resolution
    // below, and two of the rules under it are about *how* the class side was
    // written rather than about which class it named.
    let through_class_ref = class_ref.is_some();
    // Any other class side that is not a written name is the same mistake
    // `new $c()` and `$x is $c` make, and gets the same report — see
    // [`super::members::reject_dynamic_class_name`]. Everything below resolves
    // to nothing for such a side, so it would otherwise reach `nvs-ir` as a
    // static call with no target recorded, which panics.
    if !is_written_class_side(class) && class_ref.is_none() {
        reject_dynamic_class_name(
            "the class side of a `::` call must be a written class name",
            class.span,
            env,
        );
    }
    let resolved =
        match method {
            MemberName::Ident(name_span) => resolve_class_expr(class, ctx, env)
                .or(class_ref)
                .and_then(|qname| {
                    let name = span_text(env.src, *name_span).to_owned();
                    let found = resolve_method(&qname, &name, env.signatures, env.graph).map(
                        |(owner, sig)| {
                            check_method_visibility(&owner, &name, &sig, *name_span, ctx, env);
                            // The declaring class — see [`infer_method_call`] for why
                            // the receiver's own is the wrong label.
                            (owner, name.clone(), sig)
                        },
                    );
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
                    // `rule:security/response-body-is-one-typed-member`'s sixth row, noted here because this is the
                    // one site where a `Core\Response::…` call has a resolved
                    // class name to be recognized by — `crate::response` owns
                    // the roster and the refusal.
                    crate::response::note_body_member(&qname, &name, expr.span, env);
                    crate::response::reject_head_change_in_later(&qname, &name, expr.span, env);
                    // `rule:core-api/shape-rules` R20's one genuinely reachable two-spellings case — see
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
                        } else if through_class_ref
                            || (!scope.holds_receiver() && !matches!(args, CallArgs::MethodRef))
                        {
                            // A `class<T>` class side takes the refusal
                            // unconditionally, and that is the same rule rather
                            // than an extra one: what makes `self::f()` legal
                            // for a non-static `f` is that it forwards the
                            // enclosing frame's `$this`, and a class reference
                            // has no frame and no receiver to forward. `$this`
                            // where one is in scope is an instance of the
                            // enclosing class, which is not the class the
                            // reference denotes — passing it would be a field
                            // write through the wrong layout rather than a
                            // diagnostic. See `ExprInfo::ClassRefCall`.
                            report_instance_method_called_statically(expr.span, owner, &name, env);
                        } else if scope.holds_receiver() {
                            // The call, or the `(...)` reference, forwards
                            // `$this`, so it reads it: inside an anonymous
                            // function this records the capture its frame needs to
                            // have one to forward, and in a method body it
                            // records nothing.
                            let _ = scope.declared_ty("this");
                        }
                    }
                    found
                }),
            _ => None,
        };
    if let Some((owner, name, sig)) = &resolved
        && !matches!(args, CallArgs::MethodRef)
    {
        reject_abstract_static_call(expr.span, class, owner, name, sig, ctx, env);
    }
    let sig = resolved.as_ref().map(|(_, _, sig)| sig.clone());
    let label = resolved
        .as_ref()
        .map(|(owner, name, _)| format!("{owner}::{name}"));
    let (sig, written) =
        check_written_type_args(type_args, sig, label.as_deref(), expr.span, ctx, env);
    // `rule:testing/interaction-after-the-fact`'s method reference, marked
    // before the argument list is walked rather than judged after it: the
    // spelling `Mailer::send` names no constant, so what the position does is
    // give that argument — and no other — a meaning at all. See
    // [`note_method_ref_args`].
    note_method_ref_args(resolved.as_ref(), args, env);
    // `rule:core-classes/html-later`'s closed head, armed over a `later`
    // call's own arguments — see [`crate::response::is_later`].
    let outer_later = env.body_writers.in_later;
    env.body_writers.in_later |= resolved
        .as_ref()
        .is_some_and(|(owner, name, _)| crate::response::is_later(owner, name));
    let (arg_types, slots, sig) = check_args_typed(args, sig, expr.span, live, scope, ctx, env);
    env.body_writers.in_later = outer_later;
    // See [`infer_method_call`]: the same question, before the same `slots`
    // are handed to [`resolved_call`]. The two folds below return ahead of it
    // deliberately — a retrieval's and an enumeration's arguments are the
    // literals those folds require, so there is no argument left to carry a
    // qualifier by the time either answers.
    let contagious = sig
        .as_ref()
        .is_some_and(|s| carries_contagion(s, &slots, &arg_types, env.interner));
    // `rule:security/secret-sinks-refuse`'s debug-dump sink, at the one end where the qualifier is
    // still visible — both members declare `mixed`, so nothing below this
    // point can tell. See [`reject_secret_debug_argument`].
    if let Some((owner, name, _)) = &resolved {
        reject_secret_debug_argument(owner, name, args, &arg_types, env);
        // `rule:security/secret-sinks-refuse`'s cross-boundary sink, at the same end and for the same
        // reason — `Core\Serialize::encode` declares `mixed` too. See
        // [`reject_secret_boundary_argument`], which the `spawn` forms will
        // reach rather than growing a second rule.
        reject_secret_boundary_argument(owner, name, args, &arg_types, env);
        // `rule:security/secret-sinks-refuse`'s serialiser sink, the third member of the same shape:
        // `Core\Json::encode` declares `mixed` too, and what it walks is the
        // whole value. See [`reject_secret_encoded_argument`].
        reject_secret_encoded_argument(owner, name, args, &arg_types, env);
        // `rule:security/secret-sinks-refuse`'s log sink, the fourth — and the one whose open type is
        // that ADR's own decision rather than a member's convenience, which is
        // why it takes the `scope` the others do not: `fields` stays
        // `array<mixed>`, so an element that names a binding is asked about by
        // name. See [`reject_secret_logged_argument`].
        reject_secret_logged_argument(owner, name, args, &arg_types, scope, env);
        // `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`'s non-idempotent retry, and this is the only call path
        // that can reach it: every member carrying the obligation is a static
        // one (`nvs_stdlib::http`'s `CLIENT` writes an empty `instance`), so
        // [`infer_method_call`] has no arm of this hook rather than a missing
        // one. See [`reject_keyless_retry`].
        reject_keyless_retry(owner, name, args, env);
        // `rule:http-server/an-outbound-request-carries-one-body`'s two refusals, over the same members and by
        // the same reading of the same literal — a body key the verb has no use
        // for, or two of them at one call. See [`reject_ill_formed_body`].
        reject_ill_formed_body(owner, name, args, env);
        // `rule:testing/inline-snapshots`'s updater material, taken at the one site that can see
        // it: the `$expected` literal's span, which no runtime record holds.
        // Records rather than refuses, like every other hook here that reads a
        // written argument. See [`crate::testing::note_inline_snapshot`].
        crate::testing::note_inline_snapshot(owner, name, args, env);
        // `rule:security/optional-capability-degrades`'s roster, over the one member whose argument names a
        // capability — static-only, like the member. See
        // [`crate::capability`], which replaces nothing.
        crate::capability::reject_unknown_capability(owner, name, args, env);
        // `rule:expressions/intrinsic-list-is-closed`'s closed list — [`infer_method_call`]'s arm of the same
        // hook, for the `Core\Str::format(…)` / `Core\Regex::compile(…)` half
        // of the roster. See [`crate::intrinsics`].
        crate::intrinsics::check_call(owner, name, expr.span, args, &arg_types, &slots, env);
        // `rule:core-classes/html-to-source`'s written reason — `Core\Html::toSource` is a static
        // call, so this is the arm that actually reports it. See
        // [`crate::reasons`].
        crate::reasons::check_call(owner, name, args, &slots, ctx, env);
        // `rule:testing/doubles`' structural check, at the one point where the
        // interface the call site wrote and the shape it answers with are both
        // typed. Static-only, like both members: a double declares no class, so
        // this is the whole of what `crate::conformance` would otherwise have
        // asked of one. See [`crate::conformance::check_double_answers`].
        crate::conformance::check_double_answers(
            owner, name, &written, &arg_types, &slots, expr.span, env,
        );
        // A member that opens an isolate, which its row says by marking an
        // entry parameter — `rule:concurrency/an-upgrade-is-spawn-shaped`'s `Core\Socket::upgrade`. Its entry
        // takes ADR 0006 § *Decision*'s operand rule and its other arguments
        // take `rule:security/secret-sinks-refuse`'s crossing refusal, both of them the `spawn
        // script` site's own rather than a second copy. Static-only, like
        // every marked row: this is the one call path where an argument's
        // *written shape* still decides whether it is accepted. See
        // [`super::isolate::check_core_isolate_call`].
        super::isolate::check_core_isolate_call(owner, name, args, &arg_types, &slots, env);
        // `rule:core-classes/topic`'s bus, the third carrier of the same graph copy: a
        // published value is copied into every subscriber, so it takes the
        // crossing refusal `args:` takes one line above. Slots for the same
        // reason that call reads them. See
        // [`reject_secret_published_argument`].
        reject_secret_published_argument(owner, name, args, &arg_types, &slots, env);
        // The same graph copy through a session record: `Core\Session::set`
        // encodes its value into bytes a store holds until the session ends, and
        // `rule:http-server/a-session-holds-a-secret-only-sealed` gives a user's
        // own secret a sealed door of its own there, which this refusal names.
        // See [`reject_secret_session_argument`].
        reject_secret_session_argument(owner, name, args, &arg_types, &slots, env);
        // The serialiser sink one member further on: `Core\Queue::push`
        // encodes its `args:` payload with `Core\Json::encode`'s own encoder,
        // into a row a worker process decodes later. It takes the slots for
        // the call above's reason and the `scope` for the log sink's — the bag
        // arrives as the declared `CoreShape`, so the written literal is all
        // there is left to read. See [`reject_secret_enqueued_argument`].
        reject_secret_enqueued_argument(owner, name, args, &slots, scope, env);
    }
    // See [`infer_method_call`]: a method reference names a callable,
    // not the resolved method's return type, and records `CallableRef` rather
    // than `Call` for the same span. `static_class` is set here exactly as it
    // is for a call — `rule:types/callable-values` keeps `static::helper(...)` late-bound.
    if matches!(args, CallArgs::MethodRef) {
        if let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig)
            && !reject_unforwardable_method_ref(qname, name, sig, expr.span, env)
        {
            let mut call = resolved_call(qname.clone(), name.clone(), sig, slots, env.signatures);
            call.static_class = called_class_set_at(class, ctx, env);
            call.written_enum = written_enum_of(qname, name, &written, type_args, expr.span, env);
            env.exprs.record(expr.span, ExprInfo::CallableRef(call));
            let held = method_ref_type(sig, env);
            env.exprs.record_callable_value(expr.span, held);
            return held;
        }
        return env.interner.callable();
    }
    // `rule:attributes/structural-retrieval` and `rule:attributes/retrieval-folds-while-checking`'s retrieval, which is not a call at all once it has been
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
    // `rule:programs/relative-paths-resolve-from-their-file`'s `thisFile` and
    // `thisDir`, replaced by the path of the file that wrote them — the arm
    // above's reasoning, for a fold whose answer is a path. A source with no
    // file records nothing here, and the `ExprInfo::Call` below lowers the
    // call. See [`crate::paths::fold_this`].
    if let Some((qname, name, _)) = &resolved
        && crate::paths::is_this(qname, name)
    {
        let name = name.clone();
        crate::paths::fold_this(expr, &name, args, env);
    }
    // `rule:programs/implementing`'s enumeration, which is not a call at all once it has been
    // answered — the arm above's reasoning, for a fold whose answer allocates.
    // See [`crate::program`].
    if let Some((qname, name, _)) = &resolved
        && crate::program::is_enumeration(qname, name)
    {
        // The joined form's type is built from its two written arguments —
        // no registry row can spell `array<{instance: I, attribute: ?T}>`
        // as a lowered type — so it answers the type itself.
        if crate::program::is_joined(name) {
            return crate::program::expand_with(expr, &written, args, live, scope, ctx, env);
        }
        // `constructors`' type is built from its written arguments too, for
        // the same reason.
        if crate::program::is_constructors(name) {
            return crate::program::expand_constructors(expr, &written, ctx, env);
        }
        crate::program::expand(expr, &written, live, scope, ctx, env);
        return sig.map_or_else(|| env.interner.mixed(), |s| s.return_ty);
    }
    // `rule:routing/link-name-and-params-are-checked`'s link, and the one fold that is *not* made here: the route
    // a name written in the code asks for may be declared in a file § 5's scan has not
    // reached, so the site is only recorded and the lookup happens after the
    // whole walk. The `ExprInfo::Call` recorded below deliberately stands until
    // then — `crate::links` records over it, and a computed name keeps it. The
    // site is taken ahead of that record because it reads the call's own
    // argument mapping, which `resolved_call` takes by value.
    if let Some((qname, name, _)) = &resolved
        && crate::links::is_link(qname, name)
    {
        let name = name.clone();
        crate::links::record_site(expr, &name, args, &slots, ctx, env);
    }
    // See [`infer_method_call`]: persisted for `nvs-ir` to read back a resolved
    // static call's target, always as the *substituted* signature.
    //
    // `rule:types/class-reference-sites`'s class-reference side takes the other entry, for the reason
    // [`infer_new`] gives at its own record: `T` is what the member was
    // *checked* against, so a direct call to `T::f` would run the base's body
    // rather than the implementor's. `ExprInfo::ClassRefCall` carries the same
    // resolved call and says which of the two it is.
    if !is_written_class_side(class)
        && let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig)
        && through_class_ref
    {
        let call = resolved_call(qname.clone(), name.clone(), sig, slots, env.signatures);
        env.exprs.record(expr.span, ExprInfo::ClassRefCall(call));
    } else if is_written_class_side(class)
        && let (Some((qname, name, _)), Some(sig)) = (&resolved, &sig)
    {
        let mut call = resolved_call(qname.clone(), name.clone(), sig, slots, env.signatures);
        // Late static binding: which spellings set the called class and which
        // forward the caller's is [`called_class_set_at`]'s.
        call.static_class = called_class_set_at(class, ctx, env);
        if let Some((target, list)) =
            written_class_of(qname, name, &written, type_args, expr.span, env)
        {
            target.record_on(&mut call, list);
        }
        call.written_enum = written_enum_of(qname, name, &written, type_args, expr.span, env);
        env.exprs.record(expr.span, ExprInfo::Call(call));
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

/// The class a `Class::member()` site *calls on*, for `rule:statements/static-is-a-member-modifier`'s `static`
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

/// The called class a `Class::method()` site *sets*, recorded as
/// `ResolvedCall::static_class` — `None` where the site forwards the called
/// class of the frame it is written in.
///
/// A written class name sets its own. `self::` and `parent::` forward, except
/// inside an anonymous function's body: an anonymous function is a frame of its
/// own that holds no called class to forward, so both set the class it is
/// written in (`rule:statements/static-is-a-member-modifier`). That is the
/// lexical class for `parent::` as well — the parent is where the method is
/// looked up, not the class the call is made on. `static::` in an anonymous
/// function is refused
/// (`E0834`), so it forwards here, in a method frame that holds a called class.
fn called_class_set_at(class: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<QName> {
    match &class.kind {
        ExprKind::ConstFetch(_) => resolve_class_expr(class, ctx, env),
        ExprKind::SelfExpr | ExprKind::ParentExpr if ctx.in_anon_fn => ctx.current_class.cloned(),
        _ => None,
    }
}

/// `E_ABSTRACT_STATIC_CALLED`: a call to a static method with no body, made
/// on a class the site fixes while compiling ([`called_class_set_at`]).
///
/// A bodiless target dispatches on the called class when it runs, and a site
/// that sets the called class leaves no late binding that could reach a
/// subclass's override. So the call has a body to run only when the called
/// class, or a class or interface above it, declares the method with one —
/// the same chain the run-time dispatch walks. `self::`, `static::` and
/// `parent::` outside an anonymous function forward the called class and are not asked.
/// A `Class::method(...)` reference is not a call and is not asked
/// either: an attribute retrieval names an abstract method that way.
fn reject_abstract_static_call(
    span: Span,
    class: &Expr,
    owner: &QName,
    name: &str,
    sig: &MethodSig,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if !sig.is_static || sig.has_body {
        return;
    }
    let Some(called) = called_class_set_at(class, ctx, env) else {
        return;
    };
    if declares_a_body(&called, name, env) {
        return;
    }
    let in_anon_fn = !matches!(class.kind, ExprKind::ConstFetch(_));
    let (label, help) = if in_anon_fn {
        (
            format!("inside an anonymous function, this calls `{called}` itself, not a subclass"),
            "name a class that is not `abstract`, or call it with `static::` outside the \
             anonymous function and use the result inside it"
                .to_owned(),
        )
    } else {
        (
            format!("called on `{called}`, which does not give it a body"),
            format!(
                "call it on a class that is not `abstract`, or, inside a method of `{owner}`, \
                 write `static::{name}()` to call the class the method was called on"
            ),
        )
    };
    env.diags.report(
        Diagnostic::error(
            code::E_ABSTRACT_STATIC_CALLED,
            format!("`{owner}::{name}()` is `abstract`, so it has no body to call"),
        )
        .with_primary(span, label)
        .with_help(help),
    );
}

/// Whether `qname`, or any class or interface it extends or implements,
/// declares `name` with a body — what a dispatch on `qname` can find.
fn declares_a_body(qname: &QName, name: &str, env: &Env<'_>) -> bool {
    let mut seen = FxHashSet::default();
    let mut queue = vec![qname.clone()];
    while let Some(current) = queue.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        if env
            .signatures
            .get(&current)
            .and_then(|found| found.methods.get(name))
            .is_some_and(|found| found.has_body)
        {
            return true;
        }
        if let Some(links) = env.graph.get(&current) {
            queue.extend(links.extends.iter().chain(links.implements.iter()).cloned());
        }
    }
    false
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
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let target_ty = check_new_target(target, expr.span, live, scope, ctx, env);
    let target_qname = class_qname_of(target_ty, env.interner);
    let target_ty = check_new_type_args(
        type_args,
        target_qname.as_ref(),
        target_ty,
        expr.span,
        ctx,
        env,
    );
    report_method_ref_new(args, target_qname.as_ref(), expr.span, env);
    let resolved = target_qname
        .clone()
        .and_then(|qname| resolve_method(&qname, "constructor", env.signatures, env.graph));
    // `rule:core-api/written-visibility`'s levels reach `new` too, and deliberately: a `private`
    // constructor is PHP's singleton idiom, so the whole point of writing one
    // is that `new C()` is refused everywhere except `C`'s own bodies. The
    // span is the `new` expression rather than a member name, because that is
    // the only thing written here.
    if let Some((owner, sig)) = &resolved {
        check_method_visibility(owner, "constructor", sig, expr.span, ctx, env);
    }
    let ctor_owner = resolved.as_ref().map(|(owner, _)| owner.clone());
    let sig = resolved.map(|(_, sig)| sig);
    // `rule:classes/constructor-compatibility`, asked only of the dynamic form and against the signature
    // *before* [`check_args_typed`] substitutes: `T`'s constructor is what this
    // site checks against, so every implementor of `T` has to accept what it
    // accepts. Ahead of the argument check because a divergent implementor
    // makes that check's verdict meaningless either way it lands.
    if matches!(target, NewTarget::Expr(_))
        && let Some(base) = &target_qname
    {
        reject_divergent_implementor_constructor(
            base,
            ctor_owner.as_ref(),
            sig.as_ref(),
            expr.span,
            env,
        );
    }
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
        //
        // Two entries, because `rule:types/class-reference-sites`'s dynamic form cannot answer the
        // question `ExprInfo::New` is built around: that variant names the
        // class a layout comes from, and for `new $cls(...)` that is whichever
        // implementor the descriptor holds rather than `T`. So the bound goes
        // into [`ExprInfo::NewDynamic`] instead, where it reads as what it is —
        // the declaration this site checked against and the run-time lookup's
        // fallback — and the class actually allocated stays the operand, which
        // `nvs-ir` lowers to the descriptor `InstKind::NewDynamic` takes. The
        // constructor is resolved identically for both: § 5's `E0794` above is
        // what makes `T`'s signature sound to check a dynamic call against.
        let ctor = sig.as_ref().zip(ctor_owner).map(|(s, owner)| {
            resolved_call(owner, "constructor".to_owned(), s, slots, env.signatures)
        });
        let info = if matches!(target, NewTarget::Expr(_)) {
            ExprInfo::NewDynamic {
                bound: qname.clone(),
                ctor,
                ty: target_ty,
            }
        } else {
            ExprInfo::New {
                class: qname.clone(),
                ctor,
                ty: target_ty,
            }
        };
        env.exprs.record(expr.span, info);
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
///
/// **`new $cls()` over a `class<T>` is exempt for the same reason**, and the
/// exemption is the point rather than a corner: `rule:types/class-reference-sites` types that site
/// as `T`, and a class reference exists to hold a concrete implementor of an
/// `abstract` base or an interface. The descriptor is checked to be one where
/// the conversion stands (§ 2), which is the only place the question has an
/// answer.
fn reject_abstract_instantiation(target: &NewTarget, qname: &QName, span: Span, env: &mut Env<'_>) {
    if matches!(target, NewTarget::StaticTy | NewTarget::Expr(_)) {
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

/// `rule:classes/constructor-compatibility`, at `new $cls(...)` over a `class<T>`: the site checks its arguments
/// against **`T`**'s constructor, and the value may hold any implementor of
/// `T`, so an implementor whose constructor is not compatible with `T`'s makes
/// that check a promise the program cannot keep (`E0794`).
///
/// **Refused at the `new` and never at the declaration**, which is § 5's own
/// argument: a subclass nothing ever instantiates through a class reference is
/// nobody's problem, and refusing it where it is written would make an
/// unrelated file's `new` the reason a class cannot be written.
///
/// The comparison is ordinary substitutability, asked in the direction a call
/// needs — [`constructor_accepts_everything`] owns it. Only a subclass that
/// declares a constructor of *its own* is compared: one that inherits `T`'s
/// resolves to the very signature the site already checked against, which is
/// the overwhelmingly common case and costs one lookup per implementor. This
/// is stricter than PHP and never different from it: every program it accepts,
/// PHP runs the same way.
fn reject_divergent_implementor_constructor(
    base: &QName,
    base_ctor_owner: Option<&QName>,
    base_ctor: Option<&MethodSig>,
    span: Span,
    env: &mut Env<'_>,
) {
    for class in nvs_hir::implementors(base, env.graph) {
        let Some((owner, sub)) = resolve_method(&class, "constructor", env.signatures, env.graph)
        else {
            continue;
        };
        if Some(&owner) == base_ctor_owner || constructor_accepts_everything(base_ctor, &sub, env) {
            continue;
        }
        let declared = base_ctor_owner.map_or_else(
            || format!("`{base}`, which declares no constructor at all"),
            |owner| format!("`{owner}::constructor`"),
        );
        env.diags.report(
            Diagnostic::error(
                code::E_DYNAMIC_NEW_DIVERGENT_CONSTRUCTOR,
                format!("`new` over `class<{base}>` cannot check its arguments"),
            )
            .with_primary(
                span,
                format!("`{owner}::constructor` is not compatible with {declared}"),
            )
            .with_help(format!(
                "a class reference may hold any implementor, so every one of them has to \
                 accept the arguments written here — give `{class}` a constructor compatible \
                 with `{base}`'s, or narrow this to `as class<{class}>` and instantiate that",
            )),
        );
        // One report per site. `nvs_hir::implementors` is sorted by name, so
        // which subclass is named does not depend on the order files were
        // read — and a second entry is the same mistake in another file, which
        // this author fixes by fixing the first.
        return;
    }
}

/// Whether `sub` accepts every call `base` accepts — the one question
/// [`reject_divergent_implementor_constructor`] asks of a constructor pair.
///
/// Three conditions, and each is a way a call typed against `base` reaches
/// `sub` with arguments it cannot take: `sub` may not *demand* more than
/// `base` does, may not *hold* fewer than `base` does unless a variadic tail
/// absorbs the rest, and each of its parameters must admit everything `base`'s
/// admits. The last is the usual contravariance, asked with [`is_assignable`]
/// so a widened parameter passes and a narrowed one does not.
///
/// A `base` with no constructor is the empty signature rather than a special
/// case: it accepts a bare `new $cls()` and nothing else, which is exactly what
/// zero parameters say.
fn constructor_accepts_everything(
    base: Option<&MethodSig>,
    sub: &MethodSig,
    env: &mut Env<'_>,
) -> bool {
    let base_required = base.map_or(0, MethodSig::required);
    let base_params: &[TypeId] = base.map_or(&[], |s| &s.params);
    if sub.required() > base_required {
        return false;
    }
    if !sub.variadic && sub.params.len() < base_params.len() {
        return false;
    }
    base_params.iter().enumerate().all(|(index, declared)| {
        sub.param_at(index).is_some_and(|accepted| {
            is_assignable(*declared, accepted, env.interner, env.graph, env.signatures)
        })
    })
}

/// The type a `Name(...)` reference has: the member's own signature, spelled
/// as `rule:types/callable-signature`'s callable type, so a reference
/// satisfies a typed position exactly as the anonymous function wrapping the
/// same call would — `Core\Arr::map($xs, Core\Math::abs(...))` binds `U` from
/// `abs`'s declared return type and needs no anonymous function to read it out of.
///
/// **Every declared parameter, not just the required ones.** A member with an
/// optional parameter can be handed one more argument than it needs, so the
/// arity `rule:types/callable-arity` compares is the whole list; truncating to
/// the required prefix would leave that parameter's type unchecked at a
/// position that does hand it something. The refusal that costs is the safe
/// direction, and bare `callable` is still the way to write "any callable".
///
/// A bounded variable (`MethodSig::type_bounds`) is its bound here: no argument
/// binds it at a reference, so `Core\Math::abs(...)` is
/// `callable(int|float|decimal): int|float|decimal`.
fn method_ref_type(sig: &MethodSig, env: &mut Env<'_>) -> TypeId {
    if sig.type_bounds.is_empty() {
        return env.interner.callable_sig(sig.params.clone(), sig.return_ty);
    }
    let bindings: crate::generics::Bindings = sig.type_bounds.iter().cloned().collect();
    let bounded = sig.clone().substituted(&bindings, env.interner);
    env.interner
        .callable_sig(bounded.params.clone(), bounded.return_ty)
}

/// Refuses `new C(...)` — the method-reference sentinel written on `new`
/// (`E_METHOD_REF_NEW`).
///
/// `rule:types/callable-values` keeps the spelling for *members*, and a constructor is not
/// one: the callable it builds carries a callee, and `new` names a class. PHP
/// refuses the same expression, so this is the compatible answer as well as
/// the only one with a meaning. Reported ahead of everything else `new`
/// checks, and reported rather than left to `nvs-ir`, which would otherwise
/// reach `lower_call_args` with a sentinel where an argument list belongs —
/// this is the one shape that got a resolved `new` there at all.
/// `rule:types/callable-values`'s `(...)` over a member a `callable` cannot forward to —
/// `E_METHOD_REF_UNFORWARDABLE`, whose own docs own the rule.
/// Answers `true` when it refused, which is when nothing is recorded: the
/// pipeline stops at the first error, so `nvs-ir` never looks for the entry.
///
/// Both halves are the *callee's* declaration rather than anything the site
/// wrote, so the primary span is the `(...)` and the help names the member.
/// Reported here rather than left to the lowering because either one is a
/// type confusion in the callee's own frame — it reads the slot at the
/// declared representation, so an `inout` gets an `int` where an address
/// belongs and a variadic tail gets an `int` where the collected array does.
fn reject_unforwardable_method_ref(
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
            code::E_METHOD_REF_UNFORWARDABLE,
            format!("`{owner}::{name}` cannot be used as a method reference"),
        )
        .with_primary(span, format!("this method declares {offending}"))
        .with_help(
            "a `callable` has no parameter list. A call through it cannot pass a variable \
             by reference or collect variadic arguments. Write an anonymous function that \
             calls the method with the arguments you pass"
                .to_owned(),
        ),
    );
    true
}

/// Marks the argument a registry row's
/// [`CoreTy::MethodRef`](nvs_stdlib::registry::CoreTy::MethodRef) parameter
/// admits `Class::method` at, so that [`super::members::infer_class_const`]
/// reads the spelling as the method reference
/// `rule:testing/interaction-after-the-fact` gives it and every other site goes
/// on reading it as the undefined constant it is.
///
/// **Positional only.** The index a row writes is a parameter index, and the
/// two agree only while the call writes its arguments in order; a reference
/// written by name, or behind a `...`, is left to the ordinary refusal rather
/// than matched up by a second mapping that would then have to stay in step
/// with [`super::args::map_arguments`].
fn note_method_ref_args(
    resolved: Option<&(QName, String, MethodSig)>,
    args: &CallArgs,
    env: &mut Env<'_>,
) {
    let (Some((owner, member, _)), CallArgs::List(list)) = (resolved, args) else {
        return;
    };
    let Some(index) = crate::core_lib::method_ref_param(owner, member) else {
        return;
    };
    if list
        .iter()
        .take(index + 1)
        .any(|arg| arg.name.is_some() || arg.spread)
    {
        return;
    }
    // An argument the call did not write at all is the arity refusal
    // [`super::args::check_positional_arity`] already makes.
    let Some(arg) = list.get(index) else {
        return;
    };
    if matches!(arg.value.kind, ExprKind::ClassConstAccess { .. }) {
        env.method_ref_args.insert(arg.value.span);
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_METHOD_REF_REQUIRED,
            format!("`{owner}::{member}` names a method here, and this is not a method reference"),
        )
        .with_primary(arg.value.span, "write `Interface::method`")
        .with_help(
            "`rule:testing/interaction-after-the-fact`: the method is named as a reference so that \
             renaming it updates or breaks the test — a `string` holding the name is checked by \
             nothing, and a typo in one reports as a call that never happened",
        ),
    );
}

fn report_method_ref_new(args: &CallArgs, target: Option<&QName>, span: Span, env: &mut Env<'_>) {
    if !matches!(args, CallArgs::MethodRef) {
        return;
    }
    let named = target.map_or_else(|| "this class".to_owned(), |q| format!("`{q}`"));
    env.diags.report(
        Diagnostic::error(
            code::E_METHOD_REF_NEW,
            "`new` cannot be used as a method reference".to_owned(),
        )
        .with_primary(span, format!("{named} is constructed here, not called"))
        .with_help(
            "the `(...)` syntax works on a method: `Class::method(...)`, `$obj->method(...)` \
             or `self::method(...)`. A constructor is not a method. Write an anonymous \
             function instead: `fn (): T => new T(…)`"
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
/// `new Plain()` constructs. PHP refuses it, `rule:types/declaration`'s "nothing is
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
/// **The roster is [`nvs_stdlib::registry::GENERIC_CLASSES`].** `rule:types/grammar`
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
                    "user-declared generic classes are deferred (`rule:types/grammar`), so the only \
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
        param_text: sig.param_text.clone(),
        param_tys: sig.params.clone(),
        param_names: sig.param_names.clone(),
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
        written_class_is_list: false,
        // Its counterpart for a type argument that is an inline shape, set
        // beside it and by the same call — see the field's own doc comment.
        written_shape: None,
        // Set by both call arms and both callable-reference arms, for a member
        // on `registry::WRITTEN_ENUM_MEMBERS` — see the field's own doc comment.
        written_enum: None,
    }
}

pub(crate) fn report_non_callable_value_if_applicable(expr: &Expr, env: &mut Env<'_>) -> bool {
    match &expr.kind {
        ExprKind::Str(_) | ExprKind::Interpolated(_) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_CALLABLE_STRING_UNSUPPORTED,
                    "a string is not callable in Novis. Use a method reference instead",
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
                    "an array is not callable in Novis. Use a method reference instead",
                )
                .with_primary(expr.span, "this array")
                .with_help("e.g. `$obj->method(...)` instead of `[$obj, 'method']`"),
            );
            true
        }
        _ => false,
    }
}

/// `rule:types/callable-values`: `$x(...)` is refused whenever `$x`'s static type can
/// never be a callable. A class is one such type — Novis has no `__invoke`, so
/// no class ever makes `()` mean anything else, regardless of what methods it
/// declares — and so is every scalar, text, array, enum and shape type, which
/// is what refuses PHP's `$name = 'strlen'; $name($s)`. A `Ty::Mixed` callee
/// (nothing statically known), an already-`Ty::Callable` one and a union with
/// an arm that may be a callable are all left alone.
pub(crate) fn report_call_on_non_callable(callee_ty: TypeId, span: Span, env: &mut Env<'_>) {
    if let Ty::Class(qname, _) = env.interner.get(callee_ty).clone() {
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
        return;
    }
    if !never_a_callable(callee_ty, env.interner) {
        return;
    }
    let described = env.interner.describe(callee_ty);
    env.diags.report(
        Diagnostic::error(
            code::E_NOT_CALLABLE,
            format!("a value of type `{described}` is not callable"),
        )
        .with_primary(span, "called with `(...)` here")
        .with_help(
            "only an anonymous function or a method reference can be called. Write \
             `fn (...) => ...`, or `Class::method(...)` or `$obj->method(...)`. A string \
             with a function's name cannot be called",
        ),
    );
}

/// Whether no value of type `ty` is a callable: the types whose values are
/// never objects, a class (no class is callable), and a union of nothing but
/// those. Everything else — `mixed`, `object`, a type variable, `callable`
/// itself — may hold one and answers `false`.
fn never_a_callable(ty: TypeId, interner: &crate::ty::TypeInterner) -> bool {
    match interner.get(ty) {
        Ty::Null
        | Ty::Bool
        | Ty::True
        | Ty::False
        | Ty::Int
        | Ty::Uint
        | Ty::Float
        | Ty::Decimal
        | Ty::String
        | Ty::Bytes
        | Ty::TaintedString
        | Ty::TaintedBytes
        | Ty::SecretString
        | Ty::SecretBytes
        | Ty::SecretTaintedString
        | Ty::SecretTaintedBytes
        | Ty::SingleValueString(_)
        | Ty::SingleValueInt(_)
        | Ty::Array(_)
        | Ty::Enum(..)
        | Ty::EnumCase(..)
        | Ty::Shape(_)
        | Ty::Class(..) => true,
        Ty::Union(arms) => arms.iter().all(|arm| never_a_callable(*arm, interner)),
        _ => false,
    }
}

/// `$m->method(...)` — `rule:types/callable-values`'s method reference on a `mixed`
/// receiver, which is the one shape of that receiver's deferral that has no
/// run-time answer.
///
/// A *call* through a `mixed` defers to the receiver's own descriptor, which
/// is present at the call and marshals it. This spelling makes no call: it
/// names a callable **value**, and a callable carries its callee's arity and
/// parameter tags in the value itself (`nvs_runtime::callable`), so building
/// one here would mean reading a method row off a receiver for a value that
/// outlives the site and may be called anywhere. That is a mechanism rather
/// than a lowering, and no ADR asks for it — so the spelling is refused where
/// it is written, and the two fixes that exist are what the help names.
fn report_method_ref_on_erased_receiver(span: Span, name: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_METHOD_REF_ERASED_RECEIVER,
            format!("`{name}(...)` cannot be a method reference on a `mixed` value"),
        )
        .with_primary(span, "a method reference is made here")
        .with_help(format!(
            "a call on a `mixed` value finds its method when the program runs. A method \
             reference needs the class when the program compiles, and there is no class here. \
             Call the method directly (`$m->{name}(…)`), or narrow the value first with \
             `is` or `as ClassName`"
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
    /// `$fn(...)` — `rule:types/anonymous-function`'s opaque `callable`.
    Callable,
    /// `$m->method(...)` on a `mixed` receiver — `rule:types/erased-member-access`'s deferral, whose
    /// callee is whatever the receiver's runtime class answers.
    ErasedReceiver,
}

/// A call whose callee this site cannot name, refused from the call site's end
/// rather than the declaration's ([`report_by_reference_parameter`] is the
/// other end of the same rule): it may write neither a `name:` argument nor an
/// `inout` marker.
///
/// Through a `callable` the callee is one opaque type whatever function the
/// variable holds, so this site has no parameter list to resolve a name
/// against — and neither has the run time, a callable object recording its
/// arity and its parameter *tags* and never their names
/// (`nvs_runtime::callable`). Through a `mixed` receiver the callee is not
/// chosen until the call runs, and the method row that marshals it carries
/// exactly the same two facts for exactly that reason. PHP allows the spelling
/// only because a `Closure` there carries its whole declaration.
///
/// The `inout` marker is refused at both for one reason spelled two ways: no
/// anonymous function may declare such a parameter at all (`E_ANON_FN_INOUT_PARAM`), and
/// an `inout` parameter list is packed and written back at the *call site*,
/// which a call that learns its callee at run time cannot do — the same limit
/// [`E_DELEGATE_MEMBER_NOT_FORWARDABLE`] names for `rule:classes/delegation-by-field`'s synthesized
/// forward.
///
/// A `...` argument is left alone and lowers: how many arguments it hands over
/// is its own run-time length, which needs no parameter list to mean something
/// (`nvs_ir::Helper::CallCallableArray`). What still applies is rule 1 of
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
            "a `callable` has no parameter list, and an anonymous function cannot declare an \
             `inout` parameter. So nothing this call reaches can take one. Remove the `inout`"
        }
        NoParameterList::ErasedReceiver => {
            "`rule:types/erased-member-access` defers this call to the receiver's runtime class, and an `inout` \
             parameter list is packed and written back here at the call site — so a callee that \
             is not known until the call runs can never bind one; narrow the receiver with \
             `is` or `as ClassName` if the write-back is what was meant"
        }
    };
    let name_help = match callee {
        NoParameterList::Callable => {
            "a `callable` has no parameter list, so this call does not know the parameter \
             names of the function it reaches. Pass the argument by position"
        }
        NoParameterList::ErasedReceiver => {
            "`rule:types/erased-member-access`: the receiver's runtime class chooses the callee, and the method row \
             that marshals the call carries its arity and its parameter tags rather than their \
             names — pass the argument positionally, or narrow the receiver to the class that \
             declares the member"
        }
    };
    let mut positional_ends: Option<Span> = None;
    for arg in list {
        // `rule:statements/inout-is-written-at-the-call`'s marker has the same nothing to resolve against, one
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
/// `object`, an `rule:types/object-top` shape, a union naming no single class, an
/// intersection, or a type that can hold no object in the first place (a
/// scalar, an `array<T>`, a `callable`, a `void` call's result).
///
/// It is **one** code across that whole family, because it is one mistake: a
/// method is resolved against a class, and none of these names one. `rule:types/grammar` makes `object` the opaque top of every class type — a pointer with the
/// class label erased, listing no members — and `rule:types/object-top` gives a shape fields
/// and no methods at all; a union names several classes or none, and a
/// `Dog|Cat` receiver has no one signature for the argument list to be checked
/// against or for the call's position to take its type from. That is also why
/// the *property* half splits where this one does not: `rule:types/erased-member-access` answers an
/// erased property read with a name-keyed runtime fetch and refuses the rest
/// (`E_RECEIVER_HAS_NO_PROPERTIES`), while a call additionally needs a
/// signature and a return type, which no receiver here supplies. There is no
/// `__call` to fall back on either (`rule:classes/property-observer`), so every one of them is refused
/// where it is written rather than reaching `nvs-ir` with no resolved target.
///
/// `$fn->bindTo($obj)`, `$fn->bind($obj)` and `$fn->call($obj, ...)` on a
/// `callable` receiver — `rule:types/callable-is-the-only-function-type`'s three builtin
/// operations, or [`None`] for any other call.
///
/// `bind` and `bindTo` take one `?object` and give the receiver's own type
/// back. `call` takes the same first argument and passes the rest to the
/// callable, so it gives `mixed`, as a call through bare `callable` does. No
/// scope argument is accepted (`E_ARITY_MISMATCH`): a scope would open another
/// class's `private` members. Whether the object fits the body is a run-time
/// test, `nvs_runtime::callable::bind_callable`, because `callable` does not say
/// whether the function behind it uses `$this`.
#[expect(
    clippy::too_many_arguments,
    reason = "the four-part checking context every function in this module \
              threads, plus the call and the three parts of it this reads"
)]
fn infer_callable_rebind(
    expr: &Expr,
    receiver_ty: TypeId,
    method: &MemberName,
    args: &CallArgs,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let MemberName::Ident(name_span) = method else {
        return None;
    };
    if !matches!(
        env.interner.get(receiver_ty),
        Ty::Callable | Ty::CallableSig { .. }
    ) {
        return None;
    }
    let call = match span_text(env.src, *name_span) {
        "bind" | "bindTo" => false,
        "call" => true,
        _ => return None,
    };
    let CallArgs::List(list) = args else {
        return None;
    };
    report_args_with_no_parameter_list(args, NoParameterList::Callable, env);
    let object = env.interner.object();
    let null = env.interner.null();
    let this_ty = env.interner.make_union([object, null]);
    let fits = if call {
        !list.is_empty() && !list[0].spread
    } else {
        list.len() == 1 && !list[0].spread
    };
    if !fits {
        let (expected, help) = if call {
            (
                "at least 1",
                "the first argument is the new `$this` of the callable. The arguments after it \
                 are passed to the callable",
            )
        } else {
            (
                "1",
                "the one argument is the new `$this` of the callable. A second argument for the \
                 scope is not allowed",
            )
        };
        env.diags.report(
            Diagnostic::error(
                code::E_ARITY_MISMATCH,
                format!("expected {expected} argument(s), found {}", list.len()),
            )
            .with_primary(expr.span, "called here")
            .with_help(help),
        );
    }
    for (i, arg) in list.iter().enumerate() {
        let expected = (i == 0 && !arg.spread).then_some(this_ty);
        check_expr(&arg.value, expected, live, scope, ctx, env);
    }
    if fits {
        env.exprs
            .record(expr.span, ExprInfo::CallableRebind { call });
    }
    Some(if call {
        env.interner.mixed()
    } else {
        receiver_ty
    })
}

/// `mixed` is deliberately **not** here: `rule:types/conversion` makes it the one
/// unchecked position and `rule:types/erased-member-access` defers it to a run-time answer, which
/// is [`crate::expr`]'s own next slice rather than a refusal.
///
/// The help splits three ways because the fix does. A receiver that can hold
/// an object is narrowed — both spellings already lower, `is` proving
/// the class inside the guarded branch (`crate::locals::type_test_residue`)
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
             `if ($x is ClassName) {{ … }}`, or `$x as ClassName`; `rule:types/grammar` makes \
             `object` the opaque top of every class type, and `rule:types/erased-member-access` erases a property \
             access through one but not a call"
        )
    } else {
        format!(
            "only an object has methods — convert the receiver to the class that declares \
             `{name}` (`$x as Box`), or declare it `mixed`, which is the one unchecked position \
             (`rule:types/conversion`) and defers the whole question to a catchable throw at run time"
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
/// [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
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
    // no counterpart at all — `rule:errors/propagation` propagates a class, never a number.
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

/// Checks the expression a computed member name is written as, and hands its
/// type back — which is the whole of what `rule:types/property-key-access` decides `$obj->$key` on,
/// and the reason `E0235` is reported from this crate rather than from the
/// parser. `None` is a written-out name: there is no operand to have a type.
///
/// Answering the question is this function's job; asking it is the caller's,
/// because the answer differs by site. A property access admits a
/// `property<T>` its receiver satisfies
/// ([`super::members::check_property_member`]); a call refuses every operand,
/// § 4's first neighbour.
pub(crate) fn check_member_name(
    member: &MemberName,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    match member {
        MemberName::Variable(e) | MemberName::Expr(e) => {
            Some(check_expr(e, None, live, scope, ctx, env))
        }
        _ => None,
    }
}

pub(crate) fn check_args(
    args: &CallArgs,
    live: &mut Live,
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

/// `$f($a, $b)` where `$f`'s type names its parameters — the call
/// `rule:types/callable-signature` proves where it is written, answering the
/// signature's own return type instead of `mixed`.
///
/// [`None`] means "not that call", and is [`super::infer`]'s existing path: the
/// callee is bare `callable`, or is not callable at all, or the argument list
/// has a shape no signature can be matched against. Those keep `mixed` and the
/// per-argument tag check `nvs_runtime::callable::check_param_tags` performs.
/// [`Some`] is the proven site, and it records
/// [`ExprInfo::CallThroughSignature`] for `nvs-ir` to spend.
///
/// Two argument shapes are handed back rather than answered here: a `name:` and
/// an `inout` argument are refused where they are written by
/// [`report_args_with_no_parameter_list`] — a signature names no parameter for a
/// name to fill and an anonymous function declares no `inout` parameter at all — and
/// reaching that refusal is why they are left to the caller.
///
/// A `...` argument is answered but not proven. It makes the argument *count*
/// the spread subject's own run-time length, so no argument has a parameter to
/// be checked against and nothing is recorded for `nvs-ir` to spend; what the
/// callee answers with does not depend on its arguments, so the call still
/// reads the signature's own return type rather than falling back to `mixed`.
///
/// The argument *count* is exact, which the value's own arity is deliberately
/// not ([`code::E_CALLABLE_CALL_ARITY`] owns the asymmetry). An argument past
/// the signature's list is still checked, with nothing to check it against, so
/// that one wrong count does not silence every mistake inside it.
pub(crate) fn check_call_through_signature(
    expr: &Expr,
    callee_ty: TypeId,
    args: &CallArgs,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let Ty::CallableSig { params, ret } = env.interner.get(callee_ty) else {
        return None;
    };
    let (params, ret) = (params.clone(), *ret);
    if let CallArgs::List(list) = args
        && list.iter().any(|arg| arg.spread)
        && !list.iter().any(|arg| arg.name.is_some() || arg.inout)
    {
        check_args(args, live, scope, ctx, env);
        return Some(ret);
    }
    let exact = check_args_against_params(
        expr,
        &Signature::Written(&params),
        args,
        live,
        scope,
        ctx,
        env,
    )?;
    if exact {
        env.exprs
            .record(expr.span, ExprInfo::CallThroughSignature { params, ret });
    }
    Some(ret)
}

/// `fact($n - 1)` inside `fn fact(int $n): int` — the recursive call
/// `rule:types/anonymous-function-self-name` admits, checked against the parameter list of
/// the very anonymous function being written.
///
/// The self-name is not a value of the opaque `callable` type, so there is no
/// callee type to read a signature off; [`crate::FnSelf`] carries the function's
/// own parameters instead, which is the same list the body is being checked
/// under. The answer is [`crate::FnSelf::ret`] either way — this decides only
/// whether the arguments were held to anything. [`None`] is an argument list a
/// parameter list cannot be matched against at all — a `...`, whose count is the
/// spread subject's own run-time length, and the `name:` and `inout` arguments
/// an anonymous function has nothing to fill — and the caller then checks the
/// arguments with nothing to check them against.
///
/// Nothing is recorded for `nvs-ir`: a self-name call still reaches the function
/// through the ordinary dynamic path, so its per-argument tag check is what the
/// arguments are finally passed under.
pub(crate) fn check_self_name_args(
    expr: &Expr,
    args: &CallArgs,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<bool> {
    let params = env.fn_self.as_ref()?.params.clone();
    check_args_against_params(
        expr,
        &Signature::SelfName(&params),
        args,
        live,
        scope,
        ctx,
        env,
    )
}

/// The written parameter list a call's arguments are held to, and which of
/// [`code::E_CALLABLE_CALL_ARITY`]'s two subjects wrote it — the list is the
/// same shape either way, and only what a reader looks at to see the count
/// differs.
enum Signature<'a> {
    /// The callee's own `callable(T, U): R` type.
    Written(&'a [TypeId]),
    /// `rule:types/anonymous-function-self-name`'s self-name: the anonymous function being written.
    SelfName(&'a [TypeId]),
}

impl<'a> Signature<'a> {
    fn params(&self) -> &'a [TypeId] {
        match *self {
            Self::Written(params) | Self::SelfName(params) => params,
        }
    }
}

/// Each argument against the parameter it fills, and the count against the list
/// — the half [`check_call_through_signature`] and [`check_self_name_args`]
/// share, since a written signature and an anonymous function's own parameters are
/// the same list read off two different places.
///
/// [`None`] is an argument shape no parameter list can be matched against, and
/// nothing has been checked when it is returned. [`Some`] carries whether the
/// count matched exactly, which is what a proven call site needs on top of
/// having its arguments checked.
fn check_args_against_params(
    expr: &Expr,
    signature: &Signature<'_>,
    args: &CallArgs,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<bool> {
    let params = signature.params();
    let CallArgs::List(list) = args else {
        return None;
    };
    if list
        .iter()
        .any(|arg| arg.spread || arg.name.is_some() || arg.inout)
    {
        return None;
    }
    if list.len() != params.len() {
        report_callable_call_arity(expr.span, signature, params.len(), list.len(), env);
    }
    for (index, arg) in list.iter().enumerate() {
        let expected = params.get(index).copied();
        check_expr(&arg.value, expected, live, scope, ctx, env);
    }
    Some(list.len() == params.len())
}

/// `code::E_CALLABLE_CALL_ARITY` — a call through a written parameter list
/// passing a number of arguments that list does not name.
fn report_callable_call_arity(
    span: Span,
    signature: &Signature<'_>,
    declared: usize,
    given: usize,
    env: &mut Env<'_>,
) {
    let (what, help) = match signature {
        Signature::Written(_) => (
            "this `callable`",
            "a call through a written signature passes exactly the parameters the type names. \
             The value may be a callable that declares fewer, and it receives only the ones \
             it declares",
        ),
        Signature::SelfName(_) => (
            "this anonymous function",
            "an anonymous function that calls itself by its own name passes exactly the \
             parameters it declares. Its signature is the one written right here, so there is \
             no wider type to match a shorter list against",
        ),
    };
    env.diags.report(
        Diagnostic::error(
            code::E_CALLABLE_CALL_ARITY,
            format!("{what} names {declared} parameter(s), and this call passes {given}"),
        )
        .with_primary(span, format!("expected {declared}, found {given}"))
        .with_help(help),
    );
}

pub(crate) fn check_new_target(
    target: &NewTarget,
    span: Span,
    live: &mut Live,
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
                    nvs_hir::Undeclared {
                        qname: &qname,
                        text,
                        span: name.span,
                        namespace: ctx.namespace,
                        stmts: env.stmts,
                        src: env.src,
                    },
                    env.symbols,
                    &nvs_stdlib::registry::type_names(),
                ));
                env.interner.mixed()
            }
        }
        NewTarget::SelfTy | NewTarget::StaticTy => {
            let keyword = if matches!(target, NewTarget::SelfTy) {
                "self"
            } else {
                "static"
            };
            report_class_keyword_outside_class(keyword, span, ctx, env);
            class_of_ctx(ctx, env)
        }
        NewTarget::ParentTy => {
            report_class_keyword_outside_class("parent", span, ctx, env);
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
            // written name, so it *is* the dynamic form. `rule:types/class-reference-sites` makes one
            // operand type legal here: a `class<T>` answers with `T`, so
            // [`infer_new`] above resolves `T`'s constructor, types the
            // arguments against that signature and yields a `T`. That is what
            // `new static(...)` already does with the current class, and for
            // the same reason — `T` is the only signature this site can see,
            // and the value may hold any implementor of it.
            //
            // Every other operand is still the dynamic-name mistake — see
            // [`super::members::reject_dynamic_class_name`] for why the three
            // spellings share one report, and why `nvs-ir` is the wrong place
            // to find out.
            let operand = check_expr(e, None, live, scope, ctx, env);
            match class_ref_argument(operand, env.interner) {
                Some(inner) => inner,
                None => {
                    reject_dynamic_class_name(
                        "the target of `new` must be a written class name",
                        e.span,
                        env,
                    );
                    env.interner.mixed()
                }
            }
        }
        NewTarget::AnonClass(_) => env.interner.mixed(),
        _ => env.interner.mixed(),
    }
}

/// `rule:types/anonymous-function`'s `fn (...) => ...`: one anonymous function.
///
/// Three things happen here, and only the first is ordinary type-checking:
///
/// * The body is checked in a **fresh** [`LocalScope`] holding the anonymous
///   function's own parameters. `rule:types/declaration`'s declare-once rule is per body, so a
///   parameter named like an outer local shadows it rather than colliding
///   with it.
/// * Every outer binding is offered to that scope as a *capture* rather than
///   as a local ([`Captures`]), which is what makes the recorded capture set
///   "exactly the outer variables its body reads" (§ 2) rather than the whole
///   enclosing frame. `$this` is in that set like any other name, which is
///   `rule:statements/an-anonymous-function-captures-this-only-where-it-uses-it`'s bind-`$this`-only-where-used rule with no code of its own.
/// * The anonymous function's own [`ExprInfo::AnonFn`] entry is recorded — the
///   synthesized class its captures lower into, and that capture set — because
///   no type carries either of them.
///
/// # The answer is the anonymous function's own signature
///
/// The type handed back is [`Ty::CallableSig`](crate::ty::Ty::CallableSig),
/// built from the parameters' types and the return type computed below, which
/// is what `rule:types/anonymous-function-parameter-inference` asks for: an anonymous function written
/// where a signature is expected satisfies it through
/// `rule:types/callable-variance`, rather than arriving as the lattice top and
/// being refused against every signature under it.
///
/// Every parameter carries the type it was written with, because every
/// parameter is written with one: `nvs_syntax`'s `parse_param` is
/// `rule:types/declaration`'s declare-every-parameter rule and has no anonymous-function
/// exemption yet, so [`lower_optional_type`]'s `mixed` is unreachable from
/// source here. `rule:types/anonymous-function-parameter-inference`'s other half — an
/// unannotated parameter taking its type from the position the function is
/// written in — is what removes that requirement, and needs the expected type
/// threaded in from the call site rather than anything this function computes.
///
/// **A block body must declare its return type.** An expression body is its
/// own answer, so it needs no annotation; inferring one for a block would
/// mean whole-body return-type inference, which is a larger thing than ADR
/// 0037's one-initializer rule and is not something `rule:types/declaration` asks for. A
/// block body with none reports `E0450` and is checked against `void`.
///
/// **`yield` is not a generator here.** The inner [`Ctx`] clears
/// `generator_elem`, so a `yield` written inside an anonymous function sitting in a
/// generator's own body reports `E0445` — `rule:iteration/generators`'s lexical confinement.
///
/// # The self-name resolves, and is not a binding
///
/// `rule:types/anonymous-function-self-name`'s optional self-name is bound for this body alone, in
/// [`Env::fn_self`], and it is **not** a local holding the function. It is
/// legal in exactly one position — the callee of a call written inside this
/// body — where [`super::infer`]'s `ExprKind::Call` arm resolves it to *this*
/// anonymous function and records [`ExprInfo::AnonFnSelf`] on the callee's span. A bare
/// `fact` anywhere else stays `nvs_hir::members`' `E0319`, an unknown
/// constant.
///
/// That is § 3's own wording made concrete: the name is "not a capture, not a
/// second declared name reachable from anywhere else, and not a runtime slot
/// … it resolves the same way a method resolves `self::`". A local would be
/// all three of the things it says the name is not — [`LocalScope`] would
/// offer it to `isset`, to an assignment and to a nested anonymous function's capture
/// set, and the environment class would need a field pointing at itself. What
/// the call needs at run time is already to hand without any of that: the
/// invoke's own receiver, which `nvs_ir::lower::anon_fn`'s `FN_SELF` binds,
/// so an anonymous function that does not use its name costs nothing for having one.
///
/// The reach follows from that receiver. An anonymous function written *inside* this body
/// has a receiver of its own, so this name is not visible in it — hence the
/// save-and-replace below rather than a stack.
///
/// **The call is checked against this function's own signature**, which is the
/// one place this differs from `$f(...)`. § 4's opacity is a property of the
/// `callable` *type*, and the self-name is not a value of it: the function being
/// checked is right here, so both halves of its signature are facts the checker
/// holds. So [`FnSelf`] carries the parameter list
/// [`check_self_name_args`] holds the arguments to, and the call answers the
/// declared return type rather than `mixed`. A function that declares no return
/// type is checking its body to find out, so `FnSelf` takes `mixed` for that
/// half and the recursion answers rather than being circular; the parameter
/// list has no such half-measure, because it is complete before the body is
/// entered.
pub(crate) fn check_anon_fn(
    expr: &Expr,
    f: &FnExpr,
    expected: Option<TypeId>,
    live: &mut Live,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let seq = env.anon_fn_seq;
    env.anon_fn_seq += 1;
    // `$` cannot appear in an Novis identifier, so this label can never collide
    // with a declared class — the same guarantee `rule:iteration/generators`'s generator
    // state class relies on.
    let owner = ctx
        .current_class
        .map_or_else(|| "Script".to_owned(), ToString::to_string);
    let class = format!("{owner}$fn{seq}");

    let mut inner = LocalScope::new();
    // The body is checked on the enclosing `live`, and everything it assigns
    // is taken back once the body is done.
    let mark = live.mark();
    let inner_live = &mut *live;
    // The same ids the body checks against become the signature answered at the
    // end, so the type a call site reads and the type the body was checked
    // under cannot drift apart.
    // `rule:types/anonymous-function-parameter-inference`: a parameter the function left
    // unannotated takes its type from the position the function is written in,
    // and a written signature is the only position that has one to give — bare
    // `callable` is the top of the lattice and names no parameter. So this list
    // is empty everywhere else, and [`infer_param_type`] names the parameter it
    // could not answer rather than guessing.
    let from_position: Vec<TypeId> = match expected.map(|id| env.interner.get(id)) {
        Some(crate::ty::Ty::CallableSig { params, .. }) => params.clone(),
        _ => Vec::new(),
    };
    let mut params = Vec::with_capacity(f.params.len());
    for (i, param) in f.params.iter().enumerate() {
        let ty = match &param.ty {
            Some(_) => lower_optional_type(param.ty.as_ref(), ctx, env),
            None => {
                let inferred = infer_param_type(param.name, i, &from_position, env);
                // `nvs-ir` reads a parameter's type off the annotation it was
                // written with (`nvs_ir::lower::lower_decl_type`), and this one
                // has no annotation to read. The answer goes under the only
                // span the parameter does have, its own name.
                env.exprs.record_type(param.name, inferred);
                inferred
            }
        };
        let name = strip_sigil(span_text(env.src, param.name)).to_owned();
        if param.inout {
            report_by_reference_parameter(param, env);
        }
        params.push(ty);
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
        // Not inherited even inside the constructor: an anonymous function runs when it is
        // called, which this checker cannot bound, so a `readonly` write it
        // holds is not proven to happen during construction (`rule:classes/lateinit-restrictions`).
        in_constructor: false,
        in_anon_fn: true,
    };
    // An anonymous function's body is its own function: an enclosing loop's `break`
    // targets are not reachable from inside it, so the two counters
    // `nvs_types::locals` keeps start again at zero and are put back
    // afterwards. Without this, `foreach (...) { $f = fn (): void => {
    // break; }; }` would count the outer loop as a target it could leave.
    let outer_targets = std::mem::take(&mut env.exit_targets);
    let declared = f
        .return_type
        .as_ref()
        .map(|t| lower_type(t, &inner_ctx, env));
    let self_name = f.name.map(|span| FnSelf {
        name: span_text(env.src, span).to_owned(),
        params: params.clone(),
        ret: declared.unwrap_or_else(|| env.interner.mixed()),
    });
    let outer_self = std::mem::replace(&mut env.fn_self, self_name);
    let return_ty = match (&f.body, declared) {
        (FnBody::Expr(body), Some(ret)) => {
            check_expr(body, Some(ret), inner_live, &inner, &inner_ctx, env);
            ret
        }
        (FnBody::Expr(body), None) => check_expr(body, None, inner_live, &inner, &inner_ctx, env),
        (FnBody::Block(block), declared) => {
            let ret = declared.unwrap_or_else(|| {
                env.diags.report(
                    Diagnostic::error(
                        code::E_ANON_FN_RETURN_TYPE_REQUIRED,
                        "an anonymous function with a block body must declare its return type",
                    )
                    .with_primary(expr.span, "no `: T` on this `fn`")
                    .with_help(
                        "write `fn (...): T => { ... }`, or use an expression body, whose type \
                         is the expression's own",
                    ),
                );
                env.interner.void()
            });
            check_block(&block.stmts, inner_live, &mut inner, ret, &inner_ctx, env);
            // Recorded before `inner.captures` is taken below, and holding the
            // body's own bindings alone: what an outer name reaches this body
            // through is `Captures`, so a reader walking outward from here
            // finds the enclosing body's copy rather than a shared one.
            crate::check::record_locals(block.span, &mut inner, env);
            // A block body owes what a method's owes, and through the same two
            // exits — the `E0450` above having already made the declared type a
            // fact rather than a stand-in.
            if let Some(written) = f.return_type.as_ref() {
                crate::check::check_body_exits(
                    "the anonymous function",
                    block,
                    ret,
                    written.span,
                    env,
                );
            }
            ret
        }
    };
    env.exit_targets = outer_targets;
    env.fn_self = outer_self;
    live.rewind(mark);

    let captures = inner
        .captures
        .take()
        .expect("installed just above and never removed")
        .used
        .into_inner();
    // A capture the body reached through *this* function's `available` set may
    // have come from an enclosing anonymous function's own capture set rather than
    // from a real local — that function has to capture it too in order to have it
    // to hand on. Harmless when the enclosing scope is an ordinary body: it
    // has no `Captures` for this to record into.
    for (name, _) in &captures {
        scope.note_capture(name);
    }
    env.exprs.record(
        expr.span,
        ExprInfo::AnonFn {
            class,
            captures,
            return_ty,
        },
    );
    // The function's own type, and the one `crate::callables` reads back: which
    // callables satisfy a written `callable(...)` is asked of the signature the
    // checker gave each anonymous function, not of the erased object.
    let sig = env.interner.callable_sig(params, return_ty);
    env.exprs.record_callable_value(expr.span, sig);
    sig
}

/// The type an unannotated anonymous-function parameter binds — the `index`th of
/// `from_position`, which is the expected type's own parameter list and is
/// empty where the position expects no written signature
/// (`rule:types/anonymous-function-parameter-inference`).
///
/// A parameter past that list's end has nothing to take. It is answered
/// `mixed` after the refusal rather than dropped, so the body around it is
/// still checked and the function still answers a signature of the arity it was
/// written with.
fn infer_param_type(
    name: Span,
    index: usize,
    from_position: &[TypeId],
    env: &mut Env<'_>,
) -> TypeId {
    if let Some(ty) = from_position.get(index) {
        return *ty;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ANON_FN_PARAMETER_TYPE_NOT_INFERABLE,
            "this parameter has no type, and no position to take one from",
        )
        .with_primary(name, "nothing here says what this parameter holds")
        .with_help(
            "write the type — `fn (User $u) => …`. A parameter is left unannotated only \
             where the expected type is a `callable(...)` signature that names it",
        ),
    );
    env.interner.mixed()
}

/// `rule:types/callable-is-the-only-function-type`'s opaque `callable`, as a refusal: an anonymous function declares no `inout $x`
/// parameter.
///
/// A by-reference parameter is a contract between a *call site* and a
/// declaration — the site stages the cell, hands over its address and copies
/// back afterwards (`nvs_ir::lower::call`). An anonymous function's type is `callable` and
/// nothing else (§ 4), carrying no parameter list for a site to read, so there
/// is no site that could know to stage anything; and § 2's by-value capture
/// lets an anonymous function outlive every frame in scope where it was written, so even
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
            code::E_ANON_FN_INOUT_PARAM,
            format!("an anonymous function cannot take `{name}` as `inout`"),
        )
        .with_primary(param.name, "declared `inout` here")
        .with_help(
            "an anonymous function's type is `callable`, which has no parameter list. So no \
             call site can pass a variable by reference. Take the value and `return` the \
             result, or pass an object: an anonymous function that captures an object shares \
             its fields",
        ),
    );
}
