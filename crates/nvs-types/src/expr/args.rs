//! A call's arguments: arity, each argument against its parameter, ADR 0063
//! R2's trailing options bag, an `inout $x` argument, and the type arguments a call
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
//! [`Written`](nvs_stdlib::registry::CoreTy::Written); [`check_generic_args`]
//! infers every other one from the arguments themselves, and a call site
//! restating one of those would be a second, unchecked spelling of a fact the
//! arguments already settle.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;
use nvs_stdlib::registry::Qual;

/// Checks a call's arguments against a resolved [`MethodSig`], when one was
/// found: works out which parameter each written argument fills, reports
/// `E_ARITY_MISMATCH` where that leaves a required one unfilled, then checks
/// each argument against its parameter's type the same way an ordinary
/// assignment is checked. Falls back to the old "just walk nested expressions,
/// `mixed` throughout" behaviour when no signature resolved.
///
/// Returns each argument's own checked type, in call order —
/// [`ExprKind::New`]'s arm reads the first one back to feed
/// [`reject_secret_throwable_message`] without a second,
/// diagnostic-duplicating pass over the same expression — and beside it the
/// [`ArgSlot`] mapping, which is the fact `nvs-ir` cannot re-derive and so the
/// one thing this pass has to hand down: a name resolves against
/// [`MethodSig::param_names`], which no later pass holds.
///
/// **The all-positional list is still its own path**, and deliberately: its
/// mapping is the identity and its arity check is one count against another,
/// which is the message every existing call site already gets. [`map_arguments`]
/// is reached only by a call that writes a `name:` or a `...`, where a count is
/// no longer the question — *which* parameter is unfilled is.
pub(crate) fn check_args_typed(
    args: &CallArgs,
    sig: Option<MethodSig>,
    call_span: Span,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> (Vec<TypeId>, Vec<ArgSlot>, Option<MethodSig>) {
    let CallArgs::List(list) = args else {
        return (Vec::new(), Vec::new(), sig);
    };
    let Some(sig) = sig else {
        let types = list
            .iter()
            .map(|Arg { value, .. }| check_expr(value, None, live, scope, ctx, env))
            .collect();
        return (types, Vec::new(), None);
    };
    let slots = if list.iter().any(|a| a.name.is_some() || a.spread) {
        map_arguments(list, &sig, call_span, env)
    } else {
        check_positional_arity(list.len(), &sig, call_span, env);
        (0..list.len()).map(ArgSlot::Param).collect()
    };
    check_inout_markers(list, &slots, &sig, env);
    if sig.is_generic(env.interner) {
        let (types, sig) = check_generic_args(list, &slots, sig, live, scope, ctx, env);
        return (types, slots, sig);
    }
    let mut arg_types = Vec::with_capacity(list.len());
    for (arg, &slot) in list.iter().zip(&slots) {
        let expected = declared_for(slot, &sig, env.interner);
        // ADR 0088 § 2's admission and ADR 0033 § 3's, asked at the one position
        // that can answer either — a parameter a registry row classified. The
        // two axes are independent, so the marks are asked separately and a
        // `Qual::Reveal` parameter answers yes to both. Every other argument in
        // the language takes `check_arg`'s path unchanged, including a `...`
        // spread: the qualifier there is on the array rather than on the
        // entries, which is a question this rule does not ask.
        let admitted = match slot {
            ArgSlot::Param(index) => Admitted {
                tainted: admits_tainted_argument(sig.qual_at(index), sig.return_ty, env.interner),
                secret: admits_secret_argument(sig.qual_at(index)),
            },
            // A `...` hands over a subject's entries rather than the subject,
            // so the qualifier is on the array and not on what fills the
            // parameter — the question this rule does not ask.
            ArgSlot::Spread(_) | ArgSlot::Unresolved => Admitted::NONE,
        };
        let actual = match expected {
            Some(want) if admitted.any() => {
                check_arg_admitting_quals(&arg.value, want, admitted, live, scope, ctx, env)
            }
            _ => check_arg(&arg.value, expected, live, scope, ctx, env),
        };
        // Only a whole argument can be written back: a `...` hands over a
        // subject's entries rather than the subject, so there is no one
        // storage location for a by-reference parameter to alias.
        if let ArgSlot::Param(index) = slot
            && sig.is_inout(index)
        {
            note_write(&arg.value, scope, env);
            check_inout_arg(arg, actual, expected, env);
        }
        arg_types.push(actual);
    }
    (arg_types, slots, Some(sig))
}

/// ADR 0107 § 2's call-site marker, both directions, over whichever mapping
/// [`check_args_typed`] arrived at.
///
/// This is the half a rename could not have bought: `Adder::bump($n)` is
/// otherwise indistinguishable at the point of call from
/// `Adder::sum($a, $b)`, and only one of them writes to the caller's storage.
/// It is checked here rather than beside [`check_inout_arg`] because it is a
/// question about the *marker* and not about the argument's type, so a
/// generic call — which returns before that loop to re-check its arguments
/// against inferred type arguments — owes the same answer.
///
/// A `...` never carries one: its entries fill the variadic parameter rather
/// than the argument itself, so there is no one storage location to write
/// back to, which is [`check_inout_arg`]'s obligation 1 arrived at from the
/// call site's end.
fn check_inout_markers(list: &[Arg], slots: &[ArgSlot], sig: &MethodSig, env: &mut Env<'_>) {
    for (arg, &slot) in list.iter().zip(slots) {
        let declared = matches!(slot, ArgSlot::Param(index) if sig.is_inout(index));
        match (declared, arg.inout) {
            (true, false) => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_INOUT_ARG_MISSING,
                        "this argument binds an `inout` parameter and must say so",
                    )
                    .with_primary(arg.span, "write `inout` before it")
                    .with_help(
                        "ADR 0107 § 2 writes the marker at both ends: the callee writes back \
                         through this argument, and a call that does not say so hides it",
                    ),
                );
            }
            (false, true) => {
                let help = if matches!(slot, ArgSlot::Spread(_)) {
                    "a `...` hands over its subject's entries rather than the subject, so \
                     there is no one storage location to write back to — drop the `inout`"
                } else {
                    "drop the `inout`, or declare the parameter `inout` (ADR 0107 § 1)"
                };
                env.diags.report(
                    Diagnostic::error(
                        code::E_INOUT_ARG_UNEXPECTED,
                        "`inout` here names a by-value parameter",
                    )
                    .with_primary(arg.span, "marked `inout` here")
                    .with_help(help),
                );
            }
            _ => {}
        }
    }
}

/// The arity check for an all-positional argument list: one count against
/// another, reported at the call.
fn check_positional_arity(written: usize, sig: &MethodSig, call_span: Span, env: &mut Env<'_>) {
    let required = sig.required();
    // A variadic tail removes the *upper* bound and nothing else: a call still
    // has to supply every fixed parameter before it, which `required` already
    // stops one short of counting.
    let too_many = !sig.variadic && written > sig.params.len();
    if written >= required && !too_many {
        return;
    }
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
            format!("expected {expected} argument(s), found {written}"),
        )
        .with_primary(call_span, "called here"),
    );
}

/// What one argument's own value has to be, given the parameter it fills.
///
/// The one thing that is not the parameter's declared type is a spread: it
/// hands over a whole array whose *entries* become arguments, so what it owes
/// is `array<` the parameter's element type `>`. That indirection is the whole
/// of the spread rule — expressing it as an expectation rather than as a
/// comparison afterwards is what makes an `array<int>` spread into a
/// `string ...$rest` an ordinary `E_TYPE_MISMATCH` naming both array types,
/// exactly as an array literal's own `[...$a]` element already is.
///
/// **`E_SPREAD_SUBJECT_NOT_AN_ARRAY` therefore has no call-site twin**, which
/// the array-literal side's own docs predict rather than contradict: that code
/// is what a position with *no* `array<T>` expectation is left with, and a
/// spread only ever reaches a variadic parameter, which always has one. A
/// `string` spread at a call is `expected array<T>, found string` — the same
/// mistake, named better.
fn declared_for(slot: ArgSlot, sig: &MethodSig, interner: &mut TypeInterner) -> Option<TypeId> {
    match slot {
        ArgSlot::Param(index) => sig.param_at(index),
        ArgSlot::Spread(index) => sig.param_at(index).map(|elem| interner.array(elem)),
        ArgSlot::Unresolved => None,
    }
}

/// Which parameter each argument of a call that writes a `name:` or a `...`
/// fills — and every refusal working that out can produce.
///
/// Four rules, and PHP refuses on three of the four for its own reasons:
///
/// 1. A positional argument may not follow a `name:` or `...` one, because
///    which parameter it fills *is* its position and neither of those leaves
///    one defined (`E_POSITIONAL_AFTER_NAMED`).
/// 2. The name must reach a parameter a call can fill by name, which excludes
///    a `...$rest` tail: Novis builds that array at the call site out of the
///    arguments written into it, so a name has nowhere to be recorded
///    (`E_UNKNOWN_ARG_NAME`). This is the one rule PHP does not share — it
///    collects an unmatched name into the variadic as a string key. Every
///    signature has names ([`MethodSig::param_names`]), so this is the only
///    thing a `name:` at a resolved target can be refused for.
/// 3. No parameter may be filled twice (`E_DUPLICATE_ARG`).
/// 4. A `...` must land wholly in a variadic tail, every fixed parameter
///    already filled (`E_SPREAD_ARG_NOT_VARIADIC`). How many entries an array
///    holds is a run-time fact, so a spread that could fill fixed parameters
///    would leave the call's arity uncheckable — which is the one thing this
///    whole mapping exists to keep.
///
/// Then the arity check, which here is not a count: with names in play, *which*
/// required parameter went unfilled is both knowable and the useful message.
fn map_arguments(
    list: &[Arg],
    sig: &MethodSig,
    call_span: Span,
    env: &mut Env<'_>,
) -> Vec<ArgSlot> {
    // Indices below this are the fixed parameters — the ones a call fills with
    // one argument each, at most once. A variadic tail is neither.
    let fixed = match sig.variadic {
        true => sig.params.len().saturating_sub(1),
        false => sig.params.len(),
    };
    let mut filled = vec![false; fixed];
    let mut slots = Vec::with_capacity(list.len());
    let mut next = 0_usize;
    let mut positional_ends: Option<Span> = None;
    for arg in list {
        let slot = if let Some(name) = arg.name {
            positional_ends.get_or_insert(arg.span);
            named_slot(name, sig, env)
        } else if arg.spread {
            positional_ends.get_or_insert(arg.span);
            spread_slot(arg, sig, fixed, &filled, env)
        } else if let Some(first) = positional_ends {
            report_positional_after_named(arg, first, env);
            ArgSlot::Unresolved
        } else {
            let index = next;
            next += 1;
            ArgSlot::Param(index)
        };
        slots.push(match slot {
            ArgSlot::Param(index) if index < fixed => {
                match std::mem::replace(&mut filled[index], true) {
                    true => {
                        report_duplicate_arg(arg, index, sig, env);
                        ArgSlot::Unresolved
                    }
                    false => ArgSlot::Param(index),
                }
            }
            // Past the last parameter of a signature with no variadic tail:
            // one report per extra argument, at the argument rather than at
            // the call, since the call itself is no longer the whole story.
            ArgSlot::Param(_) if !sig.variadic => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_ARITY_MISMATCH,
                        format!(
                            "this call's target declares {} parameter(s)",
                            sig.params.len()
                        ),
                    )
                    .with_primary(arg.span, "no parameter left for this argument"),
                );
                ArgSlot::Unresolved
            }
            other => other,
        });
    }
    // One mistake is one diagnostic: an argument that reached no parameter is
    // exactly why some parameter has none, so reporting the arity behind it
    // would name the same mistake a second time and from further away.
    if slots.iter().all(|slot| *slot != ArgSlot::Unresolved) {
        report_unfilled(&filled, sig, call_span, env);
    }
    slots
}

/// Rule 1 of [`map_arguments`], and the one rule of the five that survives
/// having no signature at all: which parameter a positional argument fills is
/// its own place in the list either way, so [`super::calls`] reports it for a
/// call through a `callable` too.
pub(crate) fn report_positional_after_named(arg: &Arg, first: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_POSITIONAL_AFTER_NAMED,
            "a positional argument cannot follow a `name:` or `...` argument",
        )
        .with_primary(arg.span, "no position left for this argument")
        .with_secondary(first, "the list stops being positional here")
        .with_help(
            "which parameter a positional argument fills is its own place in the list, so write \
             every positional argument before the first `name:` or `...` one",
        ),
    );
}

/// Rule 2 of [`map_arguments`]: the parameter a `name:` argument fills.
fn named_slot(name_span: Span, sig: &MethodSig, env: &mut Env<'_>) -> ArgSlot {
    let name = span_text(env.src, name_span).to_owned();
    if let Some(index) = sig.param_index(&name) {
        return ArgSlot::Param(index);
    }
    let is_variadic_tail = sig.variadic && sig.param_names.last().is_some_and(|last| *last == name);
    let help = if is_variadic_tail {
        format!(
            "`...${name}` is one array built out of the arguments written into it, so there is \
             nothing for a name to key — write them out positionally"
        )
    } else {
        format!("the parameters are: {}", parameter_names(sig))
    };
    env.diags.report(
        Diagnostic::error(
            code::E_UNKNOWN_ARG_NAME,
            format!("no parameter of this call can be filled by the name `{name}`"),
        )
        .with_primary(name_span, "no such parameter")
        .with_help(help),
    );
    ArgSlot::Unresolved
}

/// Rule 4 of [`map_arguments`]: the variadic tail a `...` argument lands in.
fn spread_slot(
    arg: &Arg,
    sig: &MethodSig,
    fixed: usize,
    filled: &[bool],
    env: &mut Env<'_>,
) -> ArgSlot {
    if sig.variadic && filled.iter().all(|f| *f) {
        return ArgSlot::Spread(fixed);
    }
    let (label, help) = match sig.variadic {
        true => (
            "this call's fixed parameters still need arguments",
            "how many entries an array holds is a run-time fact, so a spread that could fill a \
             fixed parameter would leave the call's arity uncheckable — write those arguments \
             out and let the spread supply the tail",
        ),
        false => (
            "this call's target declares no `...` parameter",
            "a `...` only ever supplies a `...$rest` tail, because how many entries an array \
             holds is a run-time fact and a fixed parameter list has to be counted — write the \
             arguments out",
        ),
    };
    env.diags.report(
        Diagnostic::error(
            code::E_SPREAD_ARG_NOT_VARIADIC,
            "a `...` argument has no variadic parameter to spread into",
        )
        .with_primary(arg.span, label)
        .with_help(help),
    );
    ArgSlot::Unresolved
}

/// Rule 3 of [`map_arguments`].
fn report_duplicate_arg(arg: &Arg, index: usize, sig: &MethodSig, env: &mut Env<'_>) {
    let parameter = match sig.param_names.get(index) {
        Some(name) => format!("`${name}`"),
        None => format!("at position {}", index + 1),
    };
    env.diags.report(
        Diagnostic::error(
            code::E_DUPLICATE_ARG,
            format!("the parameter {parameter} is given an argument twice"),
        )
        .with_primary(arg.span, "already filled by an earlier argument"),
    );
}

/// The arity check for a call that writes a `name:` or a `...`: every required
/// parameter the mapping left unfilled, named.
fn report_unfilled(filled: &[bool], sig: &MethodSig, call_span: Span, env: &mut Env<'_>) {
    let missing: Vec<String> = (0..sig.required())
        .filter(|index| !filled.get(*index).copied().unwrap_or(true))
        .map(|index| match sig.param_names.get(index) {
            Some(name) => format!("`${name}`"),
            None => format!("position {}", index + 1),
        })
        .collect();
    if missing.is_empty() {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ARITY_MISMATCH,
            format!("no argument for the parameter(s) {}", missing.join(", ")),
        )
        .with_primary(call_span, "called here"),
    );
}

/// A signature's parameter names as a call site would write them, for a
/// diagnostic's help line.
fn parameter_names(sig: &MethodSig) -> String {
    let fixed = match sig.variadic {
        true => sig.params.len().saturating_sub(1),
        false => sig.params.len(),
    };
    match sig.param_names.get(..fixed).unwrap_or(&sig.param_names) {
        [] => "none — this member takes no named argument".to_owned(),
        names => names
            .iter()
            .map(|name| format!("`{name}:`"))
            .collect::<Vec<_>>()
            .join(", "),
    }
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
pub(crate) fn check_arg(
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

/// Which qualifier a parameter's classification admits that its declared type
/// does not spell — the two independent axes, answered separately.
///
/// A struct rather than the [`Qual`] itself so that the two questions stay
/// asked where they are answered: `admits_tainted_argument` needs the
/// signature's return type as well as the mark, and folding either answer into
/// the other is how one axis's rule ends up deciding the other's. Both false
/// is the ordinary case and takes [`check_arg`]'s unchanged path.
#[derive(Clone, Copy)]
struct Admitted {
    tainted: bool,
    secret: bool,
}

impl Admitted {
    /// Neither axis — every slot that is not a parameter of a classified row.
    const NONE: Self = Self {
        tainted: false,
        secret: false,
    };

    /// Whether the narrowed comparison is worth making at all.
    const fn any(self) -> bool {
        self.tainted || self.secret
    }
}

/// [`check_arg`] for an argument at a parameter whose classification admits a
/// qualifier the declared type does not spell — ADR 0088 § 2's `tainted`
/// admission ([`admits_tainted_argument`]) or ADR 0033 § 3's `secret` one
/// ([`admits_secret_argument`]), each cleared only where its own mark says so.
///
/// The argument is inferred against the parameter's declared type exactly as
/// anywhere else, because the expectation is what shapes an array literal and
/// a closure literal; only the *comparison* is made against the narrowed
/// type. So a genuine mismatch here still reads `expected string, found int`
/// rather than naming a qualified type the member never declared.
///
/// The inferred type is what comes back either way, qualifiers intact: what a
/// call *answers* is decided from the return type plus [`carries_contagion`],
/// and a caller that read a laundered type here would launder by argument.
///
/// [`check_arg`]'s options-bag fork is deliberately not repeated: a bag is a
/// [`Ty::Options`], never one of the eight qualifiable atoms, so no slot that
/// reaches here is one.
fn check_arg_admitting_quals(
    value: &Expr,
    expected: TypeId,
    admitted: Admitted,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let actual = infer(value, Some(expected), live, scope, ctx, env);
    let mut compared = actual;
    if admitted.tainted {
        compared = untainted(compared, env.interner);
    }
    if admitted.secret {
        compared = unsecret(compared, env.interner);
    }
    if !is_assignable(compared, expected, env.interner, env.graph, env.signatures) {
        report_mismatch(value.span, expected, actual, env);
    }
    actual
}

/// Whether this call's result carries `tainted` — ADR 0088 § 2's contagion,
/// asked of the arguments that actually filled a [`Qual::Contagious`]
/// parameter, or a [`Qual::Reveal`] one.
///
/// `Reveal` is here for `Contagious`'s reason and not for [`Qual::Launder`]'s:
/// ADR 0033 § 3's mark removes `secret`, which says nothing about where the
/// value came from, so `Core\Secret::reveal` over a `tainted secret string`
/// answers a `tainted string`. A `Reveal` that did not carry contagion would
/// launder the other axis for free, which is the one thing an escape hatch on
/// this axis must not do.
///
/// Answered from the slots rather than from the argument list, because which
/// parameter an argument filled is the whole question and a `name:` argument
/// does not fill the one it was written at. A caller reads it before
/// [`resolved_call`] takes the slots by value.
pub(crate) fn carries_contagion(
    sig: &MethodSig,
    slots: &[ArgSlot],
    arg_types: &[TypeId],
    interner: &TypeInterner,
) -> bool {
    slots.iter().zip(arg_types).any(|(slot, &ty)| {
        matches!(
            slot,
            ArgSlot::Param(index)
                if matches!(sig.qual_at(*index), Some(Qual::Contagious | Qual::Reveal))
        ) && is_tainted(ty, interner)
    })
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
/// representation at all: `nvs_ir::lower::lower_call_args` flattens it into
/// one ordinary argument per declared option, taking the written value where
/// there is one and the option's default where there is not. A variable
/// holding a shape could not be flattened without a per-call runtime lookup
/// per option, which is the allocation-on-the-common-path that
/// `nvs_stdlib::registry`'s own docs record rejecting.
pub(crate) fn check_options_arg(
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
pub(crate) fn option_names(options: &[(String, TypeId)]) -> String {
    options
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The two extra obligations an argument at an `inout $x` parameter position
/// carries, beyond the assignability [`check_args_typed`] already checked for
/// every argument.
///
/// 1. **It must be a writable place.** The callee writes back through the
///    reference, so the argument has to name storage that survives the call.
///    Two shapes do: a bare local, and a compile-time-known property. Those
///    are exactly the two `nvs_ir::lower::Lowering::write_back_ref` can
///    re-point, and the same two `nvs_ir::lower::is_aliasing_read` already
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
pub(crate) fn check_inout_arg(
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
                code::E_INOUT_ARG_NOT_A_PLACE,
                "a property with hooks cannot be passed to an `inout` parameter",
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
                    code::E_INOUT_ARG_NOT_A_PLACE,
                    "an array element cannot be passed to an `inout` parameter yet",
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
                    code::E_INOUT_ARG_NOT_A_PLACE,
                    "only a variable or a property can be passed to an `inout` parameter",
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
                code::E_INOUT_ARG_TYPE_NOT_EXACT,
                format!("an `inout` parameter declared `{want}` needs an argument of exactly that type, not `{got}`"),
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
pub(crate) fn check_generic_args(
    list: &[Arg],
    slots: &[ArgSlot],
    sig: MethodSig,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> (Vec<TypeId>, Option<MethodSig>) {
    // The bag is identified by the *parameter* it fills, not by where it was
    // written: with a `name:` or a `...` in the list those two differ.
    let deferred = options_param(&sig, env.interner).map(ArgSlot::Param);
    let mut arg_types: Vec<TypeId> = Vec::with_capacity(list.len());
    for (index, Arg { value, .. }) in list.iter().enumerate() {
        // A placeholder for the bag: overwritten in the second pass below,
        // and never read in between — `crate::generics::bind` is skipped for
        // this index too.
        if deferred == Some(slots[index]) {
            arg_types.push(env.interner.mixed());
            continue;
        }
        // Only a position still open is checked with no expectation; see this
        // function's own docs for what an expected type carries that
        // assignability alone does not.
        let expected = declared_for(slots[index], &sig, env.interner)
            .filter(|id| !crate::generics::mentions_type_var(*id, env.interner));
        arg_types.push(check_expr(value, expected, live, scope, ctx, env));
    }

    let mut bindings = crate::generics::Bindings::default();
    for (index, actual) in arg_types.iter().enumerate() {
        if deferred == Some(slots[index]) {
            continue;
        }
        // A spread binds through `array<T>` against the subject's own array
        // type, which is the same rule one entry at a time — so `T` comes out
        // of `Core\Arr::of(...$xs)` exactly as it does out of a written-out
        // element.
        let Some(declared) = declared_for(slots[index], &sig, env.interner) else {
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
        // The same answer one level up, and the other binding not read out of
        // a type: ADR 0072 § 1's `Core\Task::all` answers a shape of what each
        // field's closure returns, and the argument's own type is a shape of
        // opaque `callable`s. `bind_callable_shape` reads the literals.
        if let Some(name) = crate::generics::callable_shape_var(declared, env.interner) {
            let shape = bind_callable_shape(&list[index].value, env);
            bindings.entry(name).or_insert(shape);
            continue;
        }
        crate::generics::bind(
            declared,
            *actual,
            env.interner,
            env.graph,
            env.signatures,
            &mut bindings,
        );
    }
    let sig = sig.substituted(&bindings, env.interner);

    for (index, arg) in list.iter().enumerate() {
        let Some(declared) = declared_for(slots[index], &sig, env.interner) else {
            continue;
        };
        if deferred == Some(slots[index]) {
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

/// The shape a [`Ty::CallableShapeTo`] parameter binds: the argument's own
/// field names, each carrying the return type of the `fn` literal written
/// there.
///
/// This is the one place ADR 0072 § 1's two restrictions are enforced, and
/// they are enforced *here* rather than by assignability because both are
/// facts about the written expression rather than about its type — a variable
/// holding `{user: $loader}` has exactly the type a literal would have, and
/// says nothing about what `$loader` returns.
///
/// A field this cannot read still appears in the shape, typed `mixed`. That is
/// the honest answer for "this field's type is unconstrained", and it keeps
/// the *rest* of the call checking: `$page->orders` past a bad `user` field is
/// still an `array<Order>`, so one mistake yields one diagnostic rather than a
/// cascade at every use site.
fn bind_callable_shape(value: &Expr, env: &mut Env<'_>) -> TypeId {
    let ExprKind::ObjectLiteral(fields) = &value.kind else {
        env.diags.report(
            Diagnostic::error(
                code::E_CALLABLE_SHAPE_NOT_A_LITERAL,
                "this argument must be written out as `{...}` at the call site",
            )
            .with_primary(value.span, "not a shape literal")
            .with_help(
                "the result's type is read off the `fn` literal written in each field, so a \
                 variable holding the shape has nothing to bind from — write the fields inline",
            ),
        );
        return env.interner.shape(Vec::new());
    };
    let mut bound: Vec<(String, TypeId)> = Vec::with_capacity(fields.len());
    for field in fields {
        let name = span_text(env.src, field.name).to_owned();
        // The literal was checked in the first pass over the argument list, so
        // its return type is already recorded at its own span; nothing here
        // walks the body a second time.
        let ty = match env.exprs.lookup(field.value.span) {
            Some(ExprInfo::Closure { return_ty, .. }) => *return_ty,
            _ => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_CALLABLE_SHAPE_FIELD_NOT_A_LITERAL,
                        format!("the field `{name}` must be written as an `fn` literal"),
                    )
                    .with_primary(field.value.span, "not an `fn` literal")
                    .with_help(
                        "`callable` carries no signature (ADR 0031), so this field's own \
                         result type exists only at the literal — a variable, a parameter or \
                         a first-class callable has none to read",
                    ),
                );
                env.interner.mixed()
            }
        };
        bound.push((name, ty));
    }
    env.interner.shape(bound)
}

/// The index of `sig`'s trailing options-bag parameter, if it has one — ADR
/// 0063 R2 puts at most one, and always last, which `nvs_stdlib::registry`'s
/// own `an_options_bag_is_last_and_never_empty` holds mechanically.
pub(crate) fn options_param(sig: &MethodSig, interner: &TypeInterner) -> Option<usize> {
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
/// [`Written`](nvs_stdlib::registry::CoreTy::Written) is writable. An
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
pub(crate) fn check_written_type_args(
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

/// The class a member on `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` was
/// asked to build, reporting `E_TYPE_ARG_NOT_A_CLASS` when what was written is
/// not a class at all.
///
/// `None` for every member not on that roster, which is all but one of them —
/// so this is a table lookup on the ordinary path and nothing more.
pub(crate) fn written_class_of(
    owner: &QName,
    method: &str,
    written: &[TypeId],
    type_args: &[Type],
    call_span: Span,
    env: &mut Env<'_>,
) -> Option<QName> {
    if !nvs_stdlib::registry::takes_written_class(&owner.to_string(), method) {
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
pub(crate) fn report_type_arg_count(sig: &MethodSig, member: &str, span: Span, env: &mut Env<'_>) {
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
/// (from [`nvs_hir::interfaces`], the one roster) are zipped against the
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
pub(crate) fn substitute_receiver_args(
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
    let Some(params) = nvs_hir::interfaces::type_params(qname.short_name())
        .or_else(|| nvs_stdlib::registry::class_type_params(&qname.to_string()))
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
