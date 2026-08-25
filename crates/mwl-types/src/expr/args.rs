//! A call's arguments: arity, each argument against its parameter, ADR 0063
//! R2's trailing options bag, a `&$x` argument, and the type arguments a call
//! site binds.
//!
//! Which member a call resolves to is [`super::calls`]; this module starts
//! from the [`MethodSig`] it found. Everything here degrades the same way:
//! with no resolved signature, every argument is still *walked* — nested
//! expressions are checked, `mixed` throughout — so a call the checker cannot
//! model never hides a mistake inside its arguments.
//!
//! Two kinds of type argument meet here, and the wall between them is
//! `docs/agent/loop-goal.md`'s standing decision that type variables stay
//! compiler-owned. [`check_written_type_args`] takes the `<...>` a call site
//! wrote and binds it, but only where the registry marks the variable
//! [`Written`](mwl_stdlib::registry::CoreTy::Written); [`check_generic_args`]
//! infers every other one from the arguments themselves, and a call site
//! restating one of those would be a second, unchecked spelling of a fact the
//! arguments already settle.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(super)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// Checks a call's arguments against a resolved [`MethodSig`], when one was
/// found: reports `E_ARITY_MISMATCH` for a wrong non-variadic argument count,
/// then checks each positional argument against its parameter's type the
/// same way an ordinary assignment is checked. Falls back to the old
/// "just walk nested expressions, `mixed` throughout" behaviour when no
/// signature resolved, and also when any argument is named or spread — PHP's
/// named/variadic call resolution isn't a straight positional mapping, and
/// modeling that is out of scope for this slice. Returns each positional
/// argument's own checked type, in call order — [`ExprKind::New`]'s arm reads
/// the first one back to feed [`reject_secret_throwable_message`] without a
/// second, diagnostic-duplicating pass over the same expression.
pub(super) fn check_args_typed(
    args: &CallArgs,
    sig: Option<MethodSig>,
    call_span: Span,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> (Vec<TypeId>, Option<MethodSig>) {
    let CallArgs::List(list) = args else {
        return (Vec::new(), sig);
    };
    let Some(sig) = sig else {
        let types = list
            .iter()
            .map(|Arg { value, .. }| check_expr(value, None, live, scope, ctx, env))
            .collect();
        return (types, None);
    };
    if list.iter().any(|a| a.name.is_some() || a.spread) {
        let types = list
            .iter()
            .map(|Arg { value, .. }| check_expr(value, None, live, scope, ctx, env))
            .collect();
        return (types, Some(sig));
    }
    let required = sig.required();
    // A variadic tail removes the *upper* bound and nothing else: a call still
    // has to supply every fixed parameter before it, which `required` already
    // stops one short of counting.
    let too_many = !sig.variadic && list.len() > sig.params.len();
    if list.len() < required || too_many {
        let expected = if sig.variadic {
            format!("at least {required}")
        } else if required == sig.params.len() {
            format!("{required}")
        } else {
            format!("{required} to {}", sig.params.len())
        };
        env.diags.report(
            Diagnostic::error(
                code::E_ARITY_MISMATCH,
                format!("expected {expected} argument(s), found {}", list.len()),
            )
            .with_primary(call_span, "called here"),
        );
    }
    if sig.is_generic(env.interner) {
        return check_generic_args(list, sig, live, scope, ctx, env);
    }
    let last_param_index = sig.params.len().saturating_sub(1);
    let mut arg_types = Vec::with_capacity(list.len());
    for (i, arg) in list.iter().enumerate() {
        let expected = if sig.variadic && i >= last_param_index {
            sig.params.last().copied()
        } else {
            sig.params.get(i).copied()
        };
        let actual = check_arg(&arg.value, expected, live, scope, ctx, env);
        if sig.is_by_ref(i) {
            note_write(&arg.value, scope, env);
            check_by_ref_arg(arg, actual, expected, env);
        }
        arg_types.push(actual);
    }
    (arg_types, Some(sig))
}

/// One argument against its parameter's declared type — [`check_expr`] for
/// every position but ADR 0063 R2's trailing options bag, which is checked by
/// [`check_options_arg`] instead.
///
/// The fork exists because a bag is a *type* with no assignability rule: an
/// ADR 0036 object literal infers to a [`Ty::Shape`], and a shape is never
/// assignable to a [`Ty::Options`] — deliberately, since the two are checked
/// by opposite rules (width subtyping accepts an unnamed extra field, an
/// options bag refuses one). Routing the argument here rather than teaching
/// [`is_assignable`] about bags keeps that asymmetry in one place, and keeps
/// `{...}` in every *other* position meaning exactly what ADR 0036 says.
pub(super) fn check_arg(
    value: &Expr,
    expected: Option<TypeId>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    if let Some(id) = expected
        && let Ty::Options(options) = env.interner.get(id)
    {
        let options = options.clone();
        return check_options_arg(value, id, &options, live, scope, ctx, env);
    }
    check_expr(value, expected, live, scope, ctx, env)
}

/// ADR 0063 R2's options bag at a call site: it must be written out as an
/// object literal (or omitted, which never reaches here), every field must be
/// an option the member declares, and each field's value must be assignable to
/// that option's own declared type.
///
/// Returns the bag's own type either way, so one malformed bag never also
/// produces an `E_TYPE_MISMATCH` for the same span.
///
/// **Why the literal must be written here.** A bag has no runtime
/// representation at all: `mwl_ir::lower::lower_call_args` flattens it into
/// one ordinary argument per declared option, taking the written value where
/// there is one and the option's default where there is not. A variable
/// holding a shape could not be flattened without a per-call runtime lookup
/// per option, which is the allocation-on-the-common-path that
/// `mwl_stdlib::registry`'s own docs record rejecting.
pub(super) fn check_options_arg(
    value: &Expr,
    options_ty: TypeId,
    options: &[(String, TypeId)],
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let ExprKind::ObjectLiteral(fields) = &value.kind else {
        // Walked anyway, so a local it reads is still marked live and its own
        // errors are still reported — the argument is wrong, not unwritten.
        infer(value, None, live, scope, ctx, env);
        let names = option_names(options);
        env.diags.report(
            Diagnostic::error(
                code::E_OPTIONS_NOT_A_LITERAL,
                "an options argument must be written out as `{...}` at the call site",
            )
            .with_primary(value.span, "not an option shape literal")
            .with_help(format!(
                "the options are flattened into one argument each at the call, so they cannot \
                 come from a variable — write the ones you want inline: {names}"
            )),
        );
        return options_ty;
    };
    let mut seen: Vec<&str> = Vec::with_capacity(fields.len());
    for field in fields {
        let name = span_text(env.src, field.name);
        let declared = options
            .iter()
            .find(|(option, _)| option == name)
            .map(|(_, ty)| *ty);
        check_arg(&field.value, declared, live, scope, ctx, env);
        if declared.is_none() {
            let names = option_names(options);
            env.diags.report(
                Diagnostic::error(
                    code::E_UNKNOWN_OPTION,
                    format!("`{name}` is not an option of this member"),
                )
                .with_primary(field.span, "no such option")
                .with_help(format!("the options are: {names}")),
            );
        } else if seen.contains(&name) {
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("the option `{name}` is given twice"),
                )
                .with_primary(field.span, "already set above"),
            );
        }
        seen.push(name);
    }
    options_ty
}

/// The declared option names, comma-separated — the help text every
/// [`check_options_arg`] diagnostic ends with, so a typo is answered with the
/// list rather than with a type spelling nobody wrote.
pub(super) fn option_names(options: &[(String, TypeId)]) -> String {
    options
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The two extra obligations an argument at a `&$x` parameter position
/// carries, beyond the assignability [`check_args_typed`] already checked for
/// every argument.
///
/// 1. **It must be a writable place.** The callee writes back through the
///    reference, so the argument has to name storage that survives the call.
///    Two shapes do: a bare local, and a compile-time-known property. Those
///    are exactly the two `mwl_ir::lower::Lowering::write_back_ref` can
///    re-point, and the same two `mwl_ir::lower::is_aliasing_read` already
///    recognises as durable storage.
/// 2. **Its type must be exactly the parameter's.** An ordinary argument may
///    widen on the way in (`int` into a `float` parameter); a by-reference one
///    may not, because the callee writes back at the *declared* type and the
///    caller's storage would then have to narrow on the way out — silently,
///    and lossily. ADR 0007 § 1's "no type ever changes by itself" leaves no
///    room for that, so the two sides must agree exactly.
///
/// Obligation 2 is only reported when the argument would otherwise have been
/// accepted: a type that is not assignable at all already produced
/// `E_TYPE_MISMATCH` at the same span, and saying it twice helps nobody.
pub(super) fn check_by_ref_arg(
    arg: &Arg,
    actual: TypeId,
    expected: Option<TypeId>,
    env: &mut Env<'_>,
) {
    // ADR 0014 § 1 makes a hooked property's read a call and its write a
    // second one, so it has no address to hand out — and no rule for what a
    // callee writing through one would even mean. Checked before the shape
    // match so `$obj->hooked` is refused for the right reason.
    let hooked = matches!(
        env.exprs.lookup(arg.value.span),
        Some(ExprInfo::HookedProperty { .. })
    );
    if hooked {
        env.diags.report(
            Diagnostic::error(
                code::E_BY_REF_ARG_NOT_A_PLACE,
                "a property with hooks cannot be passed to a `&` parameter",
            )
            .with_primary(arg.value.span, "passed by reference here")
            .with_help(
                "reading it runs its `get` hook and writing it runs its `set` hook (ADR 0014 \
                 § 1) — read it into a local, pass that, and assign the result back",
            ),
        );
        return;
    }
    match &arg.value.kind {
        ExprKind::Variable(_) => {}
        ExprKind::PropertyAccess { nullsafe, .. } if !nullsafe => {}
        ExprKind::Index { .. } => {
            env.diags.report(
                Diagnostic::error(
                    code::E_BY_REF_ARG_NOT_A_PLACE,
                    "an array element cannot be passed to a `&` parameter yet",
                )
                .with_primary(arg.value.span, "passed by reference here")
                .with_help(
                    "ADR 0007 § 5's copy-on-write separation gives an element no stable \
                     address — read it into a local, pass that, and write it back",
                ),
            );
        }
        _ => {
            env.diags.report(
                Diagnostic::error(
                    code::E_BY_REF_ARG_NOT_A_PLACE,
                    "only a variable or a property can be passed to a `&` parameter",
                )
                .with_primary(arg.value.span, "passed by reference here")
                .with_help(
                    "the callee writes back through the reference, so this argument has to \
                     name storage that outlives the call",
                ),
            );
        }
    }
    if let Some(declared) = expected
        && declared != actual
        && is_assignable(actual, declared, env.interner, env.graph, env.signatures)
    {
        let (want, got) = (
            env.interner.describe(declared),
            env.interner.describe(actual),
        );
        env.diags.report(
            Diagnostic::error(
                code::E_BY_REF_ARG_TYPE_NOT_EXACT,
                format!("a `&` parameter declared `{want}` needs an argument of exactly that type, not `{got}`"),
            )
            .with_primary(arg.value.span, format!("this is `{got}`"))
            .with_help(
                "a by-reference argument is written back at the parameter's declared type, so \
                 widening on the way in would mean narrowing on the way out",
            ),
        );
    }
}

/// [`check_args_typed`] for a signature that mentions a type variable, which
/// today means a `Core` member and nothing else ([`crate::generics`] owns
/// why).
///
/// The ordering is the whole content: a variable's value *is* an argument's
/// type, so there is nothing to check an argument against until every
/// argument has been inferred. An argument at a position whose declared type
/// is still open is therefore checked with no expectation first -- which is
/// also the honest expectation for such a position -- then the bindings are
/// read off, the signature is rewritten concrete, and only then is each
/// argument checked for assignability against its now-known parameter type.
/// One pass over the arguments, so nothing is diagnosed twice.
///
/// **A position whose declared type mentions no variable is already known**,
/// so it is checked against it in that first pass, exactly as
/// [`check_args_typed`] would. That is not an optimization: an expected type
/// is what tells an integer literal it is a `uint` (ADR 0007 § 4's rule, in
/// the [`ExprKind::Int`] arm of [`check_expr`]), so without it
/// `Core\Arr::padStart($a, 4, "-")` would report `expected uint, found int`
/// for a literal that is plainly in range -- while `Core\Str::padStart`, whose
/// signature happens to mention no variable and so never reaches this
/// function, accepted the same spelling. Substitution cannot change such a
/// position's type, and [`crate::generics::bind`] reads nothing out of it, so
/// knowing it early is free.
///
/// One binding does not come from an argument's *type* at all: a
/// [`Ty::CallableTo`] parameter takes its variable from the closure literal's
/// recorded return type, which the first pass has just produced by checking
/// that literal. [`crate::generics`] owns why, and owns the case that binds
/// nothing.
///
/// An ADR 0063 R2 options bag is the one argument left out of the first pass
/// and checked entirely in the second. It is always the last parameter, so
/// nothing it could bind is ever needed by an earlier one; and its own option
/// types may mention a variable the earlier arguments bind, so checking it
/// first would check a field against an unsubstituted `T`.
pub(super) fn check_generic_args(
    list: &[Arg],
    sig: MethodSig,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> (Vec<TypeId>, Option<MethodSig>) {
    let deferred = options_param(&sig, env.interner);
    let mut arg_types: Vec<TypeId> = Vec::with_capacity(list.len());
    for (index, Arg { value, .. }) in list.iter().enumerate() {
        // A placeholder for the bag: overwritten in the second pass below,
        // and never read in between — `crate::generics::bind` is skipped for
        // this index too.
        if deferred == Some(index) {
            arg_types.push(env.interner.mixed());
            continue;
        }
        // Only a position still open is checked with no expectation; see this
        // function's own docs for what an expected type carries that
        // assignability alone does not.
        let expected = sig
            .param_at(index)
            .filter(|id| !crate::generics::mentions_type_var(*id, env.interner));
        arg_types.push(check_expr(value, expected, live, scope, ctx, env));
    }

    let mut bindings = crate::generics::Bindings::default();
    for (index, actual) in arg_types.iter().enumerate() {
        if deferred == Some(index) {
            continue;
        }
        let Some(declared) = sig.param_at(index) else {
            continue;
        };
        // The one binding that is not read out of a type. A `callable`
        // parameter's argument type says nothing about the value the callback
        // produces (ADR 0027 § 2), so `Core\Arr::map`'s `U` comes from the
        // closure literal's own recorded return type instead — and from
        // nowhere else, which is `crate::generics`' own known gap.
        if let Some(name) = crate::generics::callback_result_var(declared, env.interner) {
            if let Some(ExprInfo::Closure { return_ty, .. }) =
                env.exprs.lookup(list[index].value.span)
            {
                bindings.entry(name).or_insert(*return_ty);
            }
            continue;
        }
        crate::generics::bind(declared, *actual, env.interner, &mut bindings);
    }
    let sig = sig.substituted(&bindings, env.interner);

    for (index, arg) in list.iter().enumerate() {
        let Some(declared) = sig.param_at(index) else {
            continue;
        };
        if deferred == Some(index) {
            arg_types[index] = check_arg(&arg.value, Some(declared), live, scope, ctx, env);
            continue;
        }
        let actual = arg_types[index];
        if !is_assignable(actual, declared, env.interner, env.graph, env.signatures) {
            report_mismatch(arg.value.span, declared, actual, env);
        }
    }
    (arg_types, Some(sig))
}

/// The index of `sig`'s trailing options-bag parameter, if it has one — ADR
/// 0063 R2 puts at most one, and always last, which `mwl_stdlib::registry`'s
/// own `an_options_bag_is_last_and_never_empty` holds mechanically.
pub(super) fn options_param(sig: &MethodSig, interner: &TypeInterner) -> Option<usize> {
    let last = sig.params.len().checked_sub(1)?;
    matches!(interner.get(sig.params[last]), Ty::Options(_)).then_some(last)
}

/// The `<...>` list written between a member name and its `(`, checked
/// against what the resolved member actually declares and bound into `sig`.
///
/// Two spellings of the same wall, both already coded for their type-position
/// twins in [`crate::lower`]: a member declaring no type parameter refuses a
/// written list (`E_TYPE_ARGS_NOT_GENERIC`), and one that declares some
/// requires exactly that many (`E_TYPE_ARG_COUNT`, "including none at all").
/// `docs/agent/loop-goal.md`'s standing decision is what draws the line —
/// user-declared generics stay parked, and a call site may write the argument
/// only where the compiler owns the declaration.
///
/// Only a variable the registry marks
/// [`Written`](mwl_stdlib::registry::CoreTy::Written) is writable. An
/// *inferred* one is bound from an argument's type by [`check_generic_args`]
/// below, and letting a call site restate it would be a second, unchecked
/// spelling of a fact the arguments already settle — `Core\Arr::first<string>`
/// over an `array<int>` has no honest answer.
///
/// Every written argument is lowered whichever way this goes, so an unknown
/// class named inside one is reported even when the list itself is refused.
/// A wrong count is recovered from by binding what *was* written, positionally;
/// [`crate::generics`] substitutes any variable left over to `mixed`.
/// Returns the checked signature alongside **what was written**, positional:
/// the bindings are erased into the signature, and one consumer needs the
/// written types themselves — see [`ResolvedCall::written_class`].
pub(super) fn check_written_type_args(
    type_args: &[Type],
    sig: Option<MethodSig>,
    label: Option<&str>,
    call_span: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> (Option<MethodSig>, Vec<TypeId>) {
    let written: Vec<TypeId> = type_args
        .iter()
        .map(|ty| lower_type(ty, ctx, env))
        .collect();
    let Some(sig) = sig else {
        return (None, written);
    };
    let member = label.unwrap_or("this member");
    let Some(span) = type_args
        .first()
        .map(|first| first.span.to(type_args[type_args.len() - 1].span))
    else {
        // Nothing written. Only a member that *requires* one has anything to
        // say about that; every other call takes this path and is unchanged.
        if !sig.type_params.is_empty() {
            report_type_arg_count(&sig, member, call_span, env);
        }
        return (Some(sig), written);
    };
    if sig.type_params.is_empty() {
        env.diags.report(
            Diagnostic::error(
                code::E_TYPE_ARGS_NOT_GENERIC,
                format!("`{member}` takes no type arguments"),
            )
            .with_primary(span, "type arguments written here")
            .with_help(
                "user-declared type parameters are deferred (ADR 0007 § 1), and a `Core` member                  whose spec signature writes none infers every type it needs from its arguments",
            ),
        );
        return (Some(sig), written);
    }
    if written.len() != sig.type_params.len() {
        report_type_arg_count(&sig, member, span, env);
    }
    let bindings: crate::generics::Bindings = sig
        .type_params
        .iter()
        .cloned()
        .zip(written.iter().copied())
        .collect();
    (Some(sig.substituted(&bindings, env.interner)), written)
}

/// The class a member on `mwl_stdlib::registry::WRITTEN_CLASS_MEMBERS` was
/// asked to build, reporting `E_TYPE_ARG_NOT_A_CLASS` when what was written is
/// not a class at all.
///
/// `None` for every member not on that roster, which is all but one of them —
/// so this is a table lookup on the ordinary path and nothing more.
pub(super) fn written_class_of(
    owner: &QName,
    method: &str,
    written: &[TypeId],
    type_args: &[Type],
    call_span: Span,
    env: &mut Env<'_>,
) -> Option<QName> {
    if !mwl_stdlib::registry::takes_written_class(&owner.to_string(), method) {
        return None;
    }
    let first = *written.first()?;
    if let Ty::Class(qname, _) = env.interner.get(first) {
        return Some(qname.clone());
    }
    let found = env.interner.describe(first);
    let span = type_args.first().map_or(call_span, |ty| ty.span);
    env.diags.report(
        Diagnostic::error(
            code::E_TYPE_ARG_NOT_A_CLASS,
            format!("`{owner}::{method}` builds a class, and `{found}` is not one"),
        )
        .with_primary(span, format!("`{found}` written here"))
        .with_help(
            "ADR 0071 § 2: a decode is an ordinary `new`, so the type argument names the \
             class to construct — write a class carrying `#[Json\\Derive]`",
        ),
    );
    None
}

/// `E_TYPE_ARG_COUNT` for a call site, from both places [`check_written_type_args`]
/// reports it: a list of the wrong length, and no list at all.
pub(super) fn report_type_arg_count(sig: &MethodSig, member: &str, span: Span, env: &mut Env<'_>) {
    let names = sig.type_params.join(", ");
    let expected = sig.type_params.len();
    env.diags.report(
        Diagnostic::error(
            code::E_TYPE_ARG_COUNT,
            format!("`{member}` takes {expected} type argument(s)"),
        )
        .with_primary(span, format!("write `{member}<{names}>(…)`")),
    );
}

/// `sig` with the receiver's own type arguments substituted in — the
/// receiver-driven half of [`crate::generics`]' two binding sites.
///
/// `$cursor->current()` on a receiver typed `Iterator<int>` resolves to
/// `current(): T`, and there is no argument list to read `T` out of: the
/// binding is the receiver's. So the declaring interface's parameter names
/// (from [`mwl_hir::interfaces`], the one roster) are zipped against the
/// receiver's written arguments and the signature is rewritten concrete
/// before a single argument is checked — the same guarantee the argument-side
/// path already gives, that a type variable never survives a call site.
///
/// Everything else is returned untouched, which is every call in a program
/// that does not name one of ADR 0053 § 2's two interfaces: `owner` must be
/// exactly the class the receiver is typed as, so an inherited member reached
/// through an implementing class is deliberately *not* substituted here.
/// Fixing `Counter`'s `T` from its `implements Iterable<int>` clause needs
/// [`crate::signatures::ClassSignature::implements`], and is the iteration
/// lowering's own work rather than this call site's.
pub(super) fn substitute_receiver_args(
    receiver: TypeId,
    owner: &QName,
    sig: &MethodSig,
    env: &mut Env<'_>,
) -> MethodSig {
    let Ty::Class(qname, args) = env.interner.get(receiver).clone() else {
        return sig.clone();
    };
    if args.is_empty() || &qname != owner {
        return sig.clone();
    }
    // Two rosters, one question. ADR 0053 § 2's interfaces are named by their
    // short name because a program writes `Iterator<int>` unqualified; a
    // `Core`-owned generic class is named in full, because `Core\ObjectSet` is
    // the only spelling there is. Neither can answer for the other's names, so
    // the fallback is a fallback rather than a merged table.
    let Some(params) = mwl_hir::interfaces::type_params(qname.short_name())
        .or_else(|| mwl_stdlib::registry::class_type_params(&qname.to_string()))
    else {
        return sig.clone();
    };
    let bindings: crate::generics::Bindings = params
        .iter()
        .map(|name| (*name).to_owned())
        .zip(args)
        .collect();
    sig.clone().substituted(&bindings, env.interner)
}
