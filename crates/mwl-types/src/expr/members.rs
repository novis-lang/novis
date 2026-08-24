//! A member reference that is not a call: a property access, a class
//! constant, an enum case — and which receiver shapes this checker diagnoses
//! rather than `mwl_hir`.
//!
//! **Diagnosing a missing member is split by receiver, not duplicated.** A
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
//! [`check_property_access`]'s shape/`object` arms are the M2 half of ADR 0036
//! § 4: a field a shape names types cleanly with no diagnostic either way; a
//! name it doesn't list, or a plain `object` receiver, is silently `mixed`
//! rather than `E_UNKNOWN_MEMBER` — deferred to ADR 0014 § 5's runtime-checked
//! fallback, which needs M4's IR/codegen to actually throw from and so has no
//! code yet. `unset()` on any *declared* object property is refused outright
//! regardless of nullability (ADR 0028 § 3, [`check_unset_target`]).
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(super)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// The class or enum a resolved type names, if it names one at all — the
/// receiver-type question every member-access/call arm below needs answered
/// before it can look anything up in a [`crate::signatures::SignatureTable`].
pub(super) fn class_qname_of(ty: TypeId, interner: &TypeInterner) -> Option<QName> {
    match interner.get(ty) {
        Ty::Class(q, _) | Ty::Enum(q, _) => Some(q.clone()),
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
pub(super) fn resolve_class_expr(class_expr: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<QName> {
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
#[expect(
    clippy::too_many_arguments,
    reason = "the same context [`check_property_member`] states, with the \
              nullsafe flag in place of the receiver type it computes"
)]
pub(super) fn check_property_access(
    object: &Expr,
    property: &MemberName,
    nullsafe: bool,
    is_unset: bool,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let object_ty = check_expr(object, None, live, scope, ctx, env);
    // `?->` resolves the property against the receiver's non-`null` half and
    // adds `null` back to the whole access's type — see [`nullsafe_result`],
    // which the method-call arm of [`infer`] shares.
    let receiver_ty = strip_nullsafe_receiver(nullsafe, object_ty, object.span, env);
    let member_ty = check_property_member(
        object,
        receiver_ty,
        property,
        is_unset,
        live,
        scope,
        ctx,
        env,
    );
    nullsafe_result(nullsafe, object_ty, member_ty, env)
}

/// The type a `?->` yields once the member itself has one: the member's own
/// type, plus the `null` the short-circuiting arm answers with.
///
/// A receiver that is not nullable in the first place gains nothing — `?->`
/// on it is exactly `->`, which is also what `mwl-ir` lowers it to. Neither
/// does a `void` member: there is no `?void`, the value is unusable either
/// way, and unioning one would make every `$obj?->doThing();` statement carry
/// a type nothing can consume.
pub(super) fn nullsafe_result(
    nullsafe: bool,
    receiver_ty: TypeId,
    member_ty: TypeId,
    env: &mut Env<'_>,
) -> TypeId {
    if !nullsafe
        || !env.interner.is_nullable(receiver_ty)
        || matches!(env.interner.get(member_ty), Ty::Void)
    {
        return member_ty;
    }
    let null = env.interner.null();
    env.interner.make_union([member_ty, null])
}

/// The half of a `?->` receiver's type that actually reaches the member —
/// everything but `null`. Left alone for `->`, whose receiver reaches the
/// member whole.
pub(super) fn strip_nullsafe_receiver(
    nullsafe: bool,
    object_ty: TypeId,
    span: Span,
    env: &mut Env<'_>,
) -> TypeId {
    if nullsafe {
        return env.interner.without_null(object_ty);
    }
    // A plain `->` on a receiver that may be `null` is refused rather than
    // resolved against its non-`null` half. Two reasons, and the second is the
    // load-bearing one: PHP throws at run time for exactly this, and
    // `mwl-ir` has no lowering for it at all — `class_qname_of` answers
    // nothing for a union, so no target is recorded and lowering panics naming
    // the span.
    //
    // `if ($m !== null) { $m->text(); }` — which every PHP program writes —
    // does not land here: `crate::locals`' narrowing gives the receiver the
    // class type inside that block, so this sees a resolved class rather than
    // a union. What still lands here is a receiver nothing tested, and one a
    // write inside the block widened again.
    if env.interner.is_nullable(object_ty) && !matches!(env.interner.get(object_ty), Ty::Null) {
        let described = env.interner.describe(object_ty);
        env.diags.report(
            Diagnostic::error(
                code::E_NULLABLE_RECEIVER,
                format!("`{described}` may be `null`, so `->` cannot reach a member of it"),
            )
            .with_primary(span, "this receiver is nullable")
            .with_help(
                "test it first — inside `if ($x !== null) { … }` the receiver is no longer \
                 nullable — or use `?->`, which answers `null` instead of reaching the member",
            ),
        );
    }
    object_ty
}

/// [`check_property_access`]'s member half: everything after the receiver's
/// own type is known, so that `?->` and `->` reach it identically.
#[expect(
    clippy::too_many_arguments,
    reason = "the four-part checking context every function in this module \
              threads — live set, scope, ctx, env — plus the receiver, its \
              already-computed type, the member and `unset()`'s flag"
)]
pub(super) fn check_property_member(
    object: &Expr,
    object_ty: TypeId,
    property: &MemberName,
    is_unset: bool,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    check_member_name(property, live, scope, ctx, env);
    let MemberName::Ident(name_span) = property else {
        return env.interner.mixed();
    };
    let name = span_text(env.src, *name_span).to_owned();

    // ADR 0036 § 4, extending ADR 0014 § 5's "a dynamically computed property
    // name is a checked runtime throw, never a fallback" rule to a second
    // trigger: an *erased receiver type*. A field a shape type names is
    // proven present at compile time — reading it never throws, so this just
    // recovers its type, same as any other statically-known access. A name
    // the shape doesn't list, or a plain `object` receiver, is fully erased;
    // whether it exists at runtime isn't a question this compile-time
    // checker can answer either way, so — unlike an ordinary class receiver's
    // `E_UNKNOWN_MEMBER` below — nothing is diagnosed here. The actual
    // checked-throw fallback this defers to is M4 work (no IR/codegen exists
    // yet to throw from); see the crate docs' known gaps.
    match env.interner.get(object_ty).clone() {
        Ty::Shape(fields) => {
            return fields
                .iter()
                .find(|(n, _)| *n == name)
                .map_or_else(|| env.interner.mixed(), |(_, ty)| *ty);
        }
        Ty::Object => return env.interner.mixed(),
        _ => {}
    }

    match class_qname_of(object_ty, env.interner) {
        Some(qname) => match crate::signatures::resolve_property_owned(
            &qname,
            &name,
            env.signatures,
            env.graph,
        ) {
            Some((owner, ty)) => {
                if is_unset {
                    report_unset_on_property(object.span.to(*name_span), &qname, &name, env);
                }
                // ADR 0014 § 1: a hooked property's access is a call to its
                // accessor, not a field touch — except inside that property's
                // own hooks, where `$this->p` is the backing slot (see
                // `Ctx::current_hook`). `is_unset` never reaches here with a
                // hook in play without also having been refused above, so
                // there is no third case.
                let hooks = crate::signatures::hooks_of(&owner, &name, env.signatures);
                let inside_own_hook =
                    ctx.current_hook == Some(name.as_str()) && is_this_receiver(object, env.src);
                if hooks != crate::signatures::PropertyHooks::default() && !inside_own_hook {
                    env.exprs.record(
                        object.span.to(*name_span),
                        ExprInfo::HookedProperty {
                            class: qname.clone(),
                            name: name.clone(),
                            ty,
                            get: hooks.get.then(|| {
                                crate::signatures::hook_label(
                                    &owner,
                                    &name,
                                    mwl_syntax::ast::PropertyHookKind::Get,
                                )
                            }),
                            set: hooks.set.then(|| {
                                crate::signatures::hook_label(
                                    &owner,
                                    &name,
                                    mwl_syntax::ast::PropertyHookKind::Set,
                                )
                            }),
                        },
                    );
                    return ty;
                }
                // `mwl-ir` needs this access's resolved declaring class to
                // lower an eventual field-read instruction — see
                // `crate::expr_table`'s own module docs. The key must match
                // `mwl-ir`'s lookup exactly: `object.span.to(*name_span)` is
                // precisely how the parser built the enclosing
                // `PropertyAccess` expression's own span (see
                // `Parser::parse_new_target_expr`'s `?->`/`->` arm), so
                // there's no need to thread that span through as a separate
                // parameter.
                env.exprs.record(
                    object.span.to(*name_span),
                    ExprInfo::Property {
                        class: qname.clone(),
                        name: name.clone(),
                        ty,
                    },
                );
                ty
            }
            None => {
                // `$this->missing` is already `E_UNDEFINED_PROPERTY`
                // from `mwl_hir::members` — every other receiver
                // shape has never been checked before this.
                if !qname.is_core()
                    && !qname.is_reserved_global_class()
                    && !is_this_receiver(object, env.src)
                {
                    report_unknown_member(object.span, &qname, &name, "property", env);
                }
                env.interner.mixed()
            }
        },
        None => env.interner.mixed(),
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
        object,
        property,
        nullsafe,
    } = &expr.kind
    {
        // The nullsafe spelling is passed through so ADR 0028 § 3's refusal
        // fires on `unset($a?->b)` too, rather than silently resolving to
        // nothing because the receiver's type still carried `null`.
        check_property_access(object, property, *nullsafe, true, live, scope, ctx, env);
    } else {
        note_write(expr, scope, env);
        check_expr(expr, None, live, scope, ctx, env);
    }
}

pub(super) fn report_unset_on_property(span: Span, qname: &QName, name: &str, env: &mut Env<'_>) {
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

/// Reports `E_UNKNOWN_MEMBER` for a property/method access this module
/// resolved a receiver class for, but found nothing declared under `name` on
/// it or any ancestor.
pub(super) fn report_unknown_member(
    span: Span,
    qname: &QName,
    name: &str,
    kind: &str,
    env: &mut Env<'_>,
) {
    env.diags.report(
        Diagnostic::error(
            code::E_UNKNOWN_MEMBER,
            format!("`{qname}` has no {kind} named `{name}`"),
        )
        .with_primary(span, "referenced here"),
    );
}

/// ADR 0063 R20, at the one place two spellings can reach one `Core` member:
/// an instance member's receiver travels in argument slot 0, so
/// `Core\Regex\Match::text($m)` passes the arity check that `$m->text()`
/// passes and lowers to the identical helper call. Worse, the *zero*-argument
/// spelling passes it too, since a `Core` instance member declares no
/// parameter for its receiver — and that one reaches the helper with an empty
/// argument slice.
///
/// Reported for a `Core` class only. A user-declared class's non-static method
/// called statically is PHP's own error, and belongs with the visibility rules
/// this crate still owes rather than here.
pub(super) fn report_core_instance_member(
    span: Span,
    qname: &QName,
    name: &str,
    env: &mut Env<'_>,
) {
    env.diags.report(
        Diagnostic::error(
            code::E_CORE_INSTANCE_MEMBER_CALLED_STATICALLY,
            format!("`{qname}::{name}` is an instance member, so it is called on a value"),
        )
        .with_primary(span, "called through the class name here")
        .with_help(format!(
            "write `$value->{name}(…)`; ADR 0063 R20 gives every `Core` operation exactly one \
             spelling"
        )),
    );
}

/// ADR 0043 § 3: a `private` interface method is an internal helper, never
/// part of that interface's contract — visible only from inside its own
/// declaring interface's method bodies (a default or another private
/// method), never through an implementing class, a subinterface, or any
/// other interface. `owner` is the [`QName`] [`resolve_method`] found `sig`
/// declared on, which may differ from the receiver's own static type when
/// the method was inherited — exactly the case this check cares about.
pub(super) fn check_interface_private_visibility(
    owner: &QName,
    name: &str,
    sig: &MethodSig,
    span: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if !sig.interface_private || ctx.current_class == Some(owner) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE,
            format!("`{owner}`'s private method `{name}` is not visible here"),
        )
        .with_primary(span, "not part of the interface's contract")
        .with_help(format!(
            "`{name}` is an internal helper of `{owner}` — call it only from `{owner}`'s own \
             method bodies"
        )),
    );
}
