//! A call's arguments: arity, each argument against its parameter, `rule:core-api/shape-rules`
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
        // `rule:security/unclassified-parameter-refuses-tainted`'s admission and `rule:core-classes/secret-reveal`'s, asked at the one position
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

/// `rule:statements/inout-is-written-at-the-call`'s call-site marker, both directions, over whichever mapping
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
                        "`rule:statements/inout-is-written-at-the-call` writes the marker at both ends: the callee writes back \
                         through this argument, and a call that does not say so hides it",
                    ),
                );
            }
            (false, true) => {
                let help = if matches!(slot, ArgSlot::Spread(_)) {
                    "a `...` hands over its subject's entries rather than the subject, so \
                     there is no one storage location to write back to — drop the `inout`"
                } else {
                    "drop the `inout`, or declare the parameter `inout` (`rule:statements/inout-is-the-by-reference-spelling`)"
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
/// every position but `rule:core-api/shape-rules` R2's trailing options bag, which is checked by
/// [`check_options_arg`] instead.
///
/// The fork exists because a bag is a *type* with no assignability rule: an
/// `rule:types/object-top` object literal infers to a [`Ty::Shape`], and a shape is never
/// assignable to a [`Ty::CoreShape`] — deliberately, since the two are checked
/// by opposite rules (width subtyping accepts an unnamed extra field, an
/// options bag refuses one). Routing the argument here rather than teaching
/// [`is_assignable`] about bags keeps that asymmetry in one place, and keeps
/// `{...}` in every *other* position meaning exactly what `rule:types/object-top` says.
pub(crate) fn check_arg(
    value: &Expr,
    expected: Option<TypeId>,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    if let Some(id) = expected
        && let Ty::CoreShape(shape) = env.interner.get(id)
    {
        let shape = shape.clone();
        return check_options_arg(value, id, &shape, live, scope, ctx, env);
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
/// qualifier the declared type does not spell — `rule:security/unclassified-parameter-refuses-tainted`'s `tainted`
/// admission ([`admits_tainted_argument`]) or `rule:core-classes/secret-reveal`'s `secret` one
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
/// [`Ty::CoreShape`], never one of the eight qualifiable atoms, so no slot that
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

/// Whether this call's result carries `tainted` — `rule:security/unclassified-parameter-refuses-tainted`'s contagion,
/// asked of the arguments that actually filled a [`Qual::Contagious`]
/// parameter, or a [`Qual::Reveal`] one.
///
/// `Reveal` is here for `Contagious`'s reason and not for [`Qual::Launder`]'s:
/// `rule:core-classes/secret-reveal`'s mark removes `secret`, which says nothing about where the
/// value came from, so `Core\Secret::reveal` over a `tainted secret string`
/// answers a `tainted string`. A `Reveal` that did not carry contagion would
/// launder the other axis for free, which is the one thing an escape hatch on
/// this axis must not do.
///
/// Answered from the slots rather than from the argument list, because which
/// parameter an argument filled is the whole question and a `name:` argument
/// does not fill the one it was written at. A caller reads it before
/// [`resolved_call`] takes the slots by value.
///
/// The argument is asked with [`carries_tainted`] and not [`is_tainted`],
/// because [`check_arg_admitting_quals`] admitted it with [`untainted`]'s
/// reach: an argument admitted as a union or an array whose taint the atom
/// question cannot see would be laundered by the call that took it.
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
        ) && carries_tainted(ty, interner)
    })
}

/// `rule:core-api/shape-rules` R2's options bag at a call site: it must be written out as an
/// object literal (or omitted, which never reaches here), every field must be
/// an option the member declares, and each field's value must be assignable to
/// that option's own declared type.
///
/// **And `rule:core-api/shape-parameter`'s shape parameter, on the same terms.** Both intern to
/// [`Ty::CoreShape`](crate::ty::Ty::CoreShape), so both are checked here; the
/// one rule a bag never reaches is the missing-required-key refusal below,
/// because a bag's every field is optional.
///
/// **Two passes, because § 2's arm cannot be chosen before the values are
/// typed.** The first checks each written value against its *merged* slot —
/// the union of what the arms declare for that key — so a value any arm would
/// accept is not refused before its arm is known, and reports the two mistakes
/// the merge does settle: a key no arm declares at all, and a key written
/// twice. [`select_arm`] then picks the arm, and [`report_against_arm`] holds
/// the literal to that one arm. For a bag and for a one-arm shape the arm is
/// the merged list itself, so the second pass is exactly the refusal the first
/// one used to make and nothing about either changed.
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
    shape: &crate::ty::CoreShape,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let options = &shape.fields;
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
    let mut written: Vec<WrittenKey<'_>> = Vec::with_capacity(fields.len());
    for field in fields {
        let name = span_text(env.src, field.name);
        let slot = options.iter().find(|option| option.name == name);
        let declared = slot.map(|option| option.ty);
        let ty = match slot {
            // `rule:core-api/shape-flattens-at-the-abi`'s classification, which is on the field: a sink key
            // is checked here so that its refusal can say what to do about it.
            Some(option) if option.qual == Some(Qual::Sink) => {
                check_shape_field(&field.value, option, live, scope, ctx, env)
            }
            _ => check_arg(&field.value, declared, live, scope, ctx, env),
        };
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
        } else if written.iter().any(|key| key.name == name) {
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("the option `{name}` is given twice"),
                )
                .with_primary(field.span, "already set above"),
            );
        }
        written.push(WrittenKey {
            name,
            span: field.span,
            ty,
        });
    }
    // `rule:core-api/shape-arms-are-disjoint`'s *exactly one arm accepts it*, which is where the rest of
    // the exact-key check lives. One arm is the ordinary case and every bag,
    // and there the arm is the merged list itself — so what follows is the
    // missing-required-key refusal this function has always made, asked of a
    // list that a two-armed shape narrows first.
    let arm = select_arm(shape, &written, env);
    report_against_arm(value, shape, arm, &written, env);
    options_ty
}

/// [`check_arg`] for a shape key `rule:core-api/shape-flattens-at-the-abi` classifies [`Qual::Sink`] —
/// `Db\Settings`'s `host` and its `path`, which are the two the registry
/// declares.
///
/// **The check is the ordinary one and stays so.** The declared type is the
/// plain atom, so [`is_assignable`] refuses a `tainted` value with no help
/// from the mark; what the mark buys is the *diagnostic*. `rule:core-classes/db-capabilities` gives a
/// host no launderer — no string check can establish that an address is safe
/// to send a credential to — so a reader told only "expected `string`, found
/// `tainted string`" has nowhere to go, and `Core\Taint::assertTrusted` is the
/// one way through that exists.
///
/// **The help is attached here and not at an ordinary sink parameter**, where
/// the same refusal has a different answer: a `tainted` value at
/// `Core\Db::query`'s statement text is a bound parameter's job (`rule:security/sink-predicate`
/// ) and at the HTML sink it is `Core\Html::escape`'s, so naming the escape
/// hatch there would push the wrong fix at every one of them. A shape key that
/// *does* gain a launderer is where this grows its second case.
fn check_shape_field(
    value: &Expr,
    field: &crate::ty::CoreShapeField,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    // `infer` rather than `check_expr`, for [`check_arg_admitting_quals`]'
    // reason: the expectation still shapes an array or a closure literal, and
    // the comparison is made here so that exactly one of the two branches
    // below reports the refusal.
    let actual = infer(value, Some(field.ty), live, scope, ctx, env);
    if is_assignable(actual, field.ty, env.interner, env.graph, env.signatures) {
        return actual;
    }
    // Only where the qualifier is the whole objection. A `tainted int` at a
    // `string` key is two mistakes, and `Core\Taint::assertTrusted` fixes
    // neither of them — that one reads as the plain mismatch it is.
    let laundered = untainted(actual, env.interner);
    let qualifier_alone = carries_tainted(actual, env.interner)
        && is_assignable(laundered, field.ty, env.interner, env.graph, env.signatures);
    let mut diag = mismatch(value.span, field.ty, actual, env);
    if qualifier_alone {
        diag = diag.with_help(format!(
            "`{}` is a sink and no `Core` member launders one for it: the only way through is \
             `Core\\Taint::assertTrusted($value, $reason)`, which is forbidden by default, \
             greppable, and carries in the source the reason the value can be trusted",
            field.name
        ));
    }
    env.diags.report(diag);
    actual
}

/// Whether the merged slot for this key accepts the value written there — i.e.
/// whether the first pass stayed quiet about it, which is the only thing
/// [`report_against_arm`] asks this.
fn merged_accepts(shape: &crate::ty::CoreShape, key: &WrittenKey<'_>, env: &mut Env<'_>) -> bool {
    shape
        .fields
        .iter()
        .find(|field| field.name == key.name)
        .is_some_and(|field| {
            is_assignable(key.ty, field.ty, env.interner, env.graph, env.signatures)
        })
}

/// One key a call site wrote in an options or shape literal: its name, where it
/// was written, and the type its value checked to.
///
/// The type is what `rule:core-api/shape-arms-are-disjoint`'s arm selection needs and the reason the
/// selection cannot happen first — an arm accepts on its keys *and* on its
/// values, so every value is typed against the merged slot before any arm is
/// chosen.
struct WrittenKey<'src> {
    name: &'src str,
    span: Span,
    ty: TypeId,
}

/// `rule:core-api/shape-arms-are-disjoint`'s arm selection: the arm that accepts the written literal, or —
/// where none does, which is the call site's error — the arm it is closest to.
///
/// An arm **accepts** when every written key is one it declares, every key it
/// requires is written, and every written value is assignable to the type it
/// declares for that key. At most one can, since the arms are pairwise disjoint
/// and `nvs_stdlib::registry`'s `a_shapes_arms_are_pairwise_disjoint` refuses a
/// roster where they are not — so this returns the first accepting arm without
/// looking for a second, and two accepting would be a registry bug that test
/// has already failed on.
///
/// Where none accepts, the answer is the arm with the **fewest** mistakes of
/// those three kinds, ties going to declaration order. That is what makes the
/// diagnostic name the form the caller was evidently writing:
/// `{driver: Driver::Sqlite, path: …, host: …}` misses the SQLite arm by one
/// key and the server arm by five, so it is answered with "`host` is not a key
/// of this form" rather than with the four keys the server arm would want. A
/// wrong *value* counts the same as a wrong key deliberately: weighting the
/// discriminant would be a second, weaker spelling of the disjointness the
/// types already carry, which is the same argument § 2 makes against naming a
/// discriminant field at all.
fn select_arm(
    shape: &crate::ty::CoreShape,
    written: &[WrittenKey<'_>],
    env: &mut Env<'_>,
) -> usize {
    let mut best = (0, usize::MAX);
    for (index, arm) in shape.arms.iter().enumerate() {
        let mut mistakes = 0;
        for key in written {
            match arm.iter().find(|field| field.name == key.name) {
                None => mistakes += 1,
                Some(field)
                    if !is_assignable(
                        key.ty,
                        field.ty,
                        env.interner,
                        env.graph,
                        env.signatures,
                    ) =>
                {
                    mistakes += 1;
                }
                Some(_) => {}
            }
        }
        mistakes += arm
            .iter()
            .filter(|field| field.required && !written.iter().any(|key| key.name == field.name))
            .count();
        if mistakes == 0 {
            return index;
        }
        if mistakes < best.1 {
            best = (index, mistakes);
        }
    }
    best.0
}

/// The literal held to the one arm [`select_arm`] chose: a key that arm does
/// not declare, a key it requires that the literal did not carry, and a value
/// its own declaration refuses.
///
/// Reports nothing when the arm accepts, which is every call this function has
/// ever made for a bag or a one-arm shape.
///
/// **Three refusals, and no code of its own for any of them.** A key belonging
/// to another arm is still not a key of *this* call, so it is the
/// `E_UNKNOWN_OPTION` the merged pass reports for a key no arm declares, with
/// the selected arm's keys named instead of the merged list's. A missing
/// required key is an `E_ARITY_MISMATCH` because § 3 flattens each key into one
/// ABI argument, so a literal short of one is a call one argument short. A
/// value the arm refuses is the ordinary [`report_mismatch`], naming the type
/// that arm declares — which is how `driver: Driver::Sqlite` written against
/// the server arm reads as the four cases it is not.
fn report_against_arm(
    value: &Expr,
    shape: &crate::ty::CoreShape,
    arm: usize,
    written: &[WrittenKey<'_>],
    env: &mut Env<'_>,
) {
    let Some(fields) = shape.arms.get(arm) else {
        return;
    };
    for key in written {
        match fields.iter().find(|field| field.name == key.name) {
            // Reported already, against the merged list, and with the same
            // code: a key no arm declares is a typo and not a wrong form.
            None if !shape.fields.iter().any(|field| field.name == key.name) => {}
            None => {
                let names = option_names(fields);
                env.diags.report(
                    Diagnostic::error(
                        code::E_UNKNOWN_OPTION,
                        format!(
                            "`{}` is not a key of the form this literal writes",
                            key.name
                        ),
                    )
                    .with_primary(key.span, "not a key of this form")
                    .with_help(format!(
                        "the keys of the form the other values select are: {names}"
                    )),
                );
            }
            // Reported already where the *merged* slot refuses it too, since a
            // value no arm accepts is one mistake and reads as one diagnostic.
            // This arm is the value that some other arm would have taken.
            Some(field)
                if !is_assignable(key.ty, field.ty, env.interner, env.graph, env.signatures)
                    && merged_accepts(shape, key, env) =>
            {
                report_mismatch(key.span, field.ty, key.ty, env);
            }
            Some(_) => {}
        }
    }
    let names: Vec<&str> = written.iter().map(|key| key.name).collect();
    for name in missing_required_keys(fields, &names) {
        let required = required_key_names(fields);
        env.diags.report(
            Diagnostic::error(
                code::E_ARITY_MISMATCH,
                format!("this member requires the shape key `{name}`"),
            )
            .with_primary(value.span, format!("`{name}` is not given"))
            .with_help(format!("the keys this member requires are: {required}")),
        );
    }
}

/// The keys the merged field list requires that a written literal does not
/// carry, in the list's own declaration order.
///
/// Pure, and separated from [`check_options_arg`] for that: the rule is
/// testable without a registry row declaring a shape parameter, which no row
/// does yet.
fn missing_required_keys<'a>(
    options: &'a [crate::ty::CoreShapeField],
    written: &[&str],
) -> Vec<&'a str> {
    options
        .iter()
        .filter(|option| option.required && !written.contains(&option.name.as_str()))
        .map(|option| option.name.as_str())
        .collect()
}

/// [`missing_required_keys`]' answer as the list a diagnostic names — the
/// required keys only, where [`option_names`] names every key the member
/// declares.
fn required_key_names(options: &[crate::ty::CoreShapeField]) -> String {
    options
        .iter()
        .filter(|option| option.required)
        .map(|option| option.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key` at its compile-time half: a request member whose verb repeats
/// an *effect* — `Core\Http\Client::post` — may not ask for retries without an
/// idempotency key.
///
/// **This is reportable at all only because of `rule:core-api/shape-rules` R2.** The verb is the
/// member's own name and the bag has to be written out as a literal at the call
/// site ([`check_options_arg`] is the refusal that makes it so), so both halves
/// of § 7's question are in hand while compiling — which is the payoff R2 was
/// designed for, arriving in a place nobody planned it for. Where the verb is
/// genuinely dynamic, § 7 moves the check to the send itself, which is the
/// spec's `send(Core\Http\Request)` row and throws before the *first* attempt.
///
/// Which members carry the obligation, and the two option names it is written
/// over, are `nvs_stdlib::registry::idempotent_retry_rule`'s — this pass holds
/// no copy of either spelling, so a renamed option cannot leave the rule
/// looking for a name no row writes.
///
/// **Asked of every object literal in the call, not of the trailing argument.**
/// A bag written by name (`options: {...}`) is not last, and the alternative —
/// re-deriving which argument filled the [`Ty::CoreShape`] parameter — is the
/// slot mapping this function is deliberately not handed. The overreach that
/// buys is an object literal at the *URL* position naming `retryAttempts`,
/// which is an `E_TYPE_MISMATCH` in the same breath.
///
/// An omitted `retryAttempts` obliges nothing, and that is the registry's
/// judgement rather than this pass's leniency: `IdempotentRetry::asks` owns
/// why.
pub(crate) fn reject_keyless_retry(
    qname: &QName,
    member: &str,
    args: &CallArgs,
    env: &mut Env<'_>,
) {
    let Some(rule) = nvs_stdlib::registry::idempotent_retry_rule(&qname.to_string(), member) else {
        return;
    };
    let CallArgs::List(list) = args else {
        return;
    };
    for arg in list {
        let ExprKind::ObjectLiteral(fields) = &arg.value.kind else {
            continue;
        };
        let Some(asked) = fields
            .iter()
            .find(|field| span_text(env.src, field.name) == rule.asks)
        else {
            continue;
        };
        if fields
            .iter()
            .any(|field| span_text(env.src, field.name) == rule.key)
        {
            return;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_RETRY_WITHOUT_IDEMPOTENCY_KEY,
                format!(
                    "`{qname}::{member}` asks for retries without `{}`",
                    rule.key
                ),
            )
            .with_primary(asked.span, "retries asked for here")
            .with_help(format!(
                "a repeated `{}` is a second effect rather than a second question, so an attempt \
                 needs an identity the server can recognise: add `{}: \"...\"` to the same bag, \
                 sent as `Idempotency-Key` and identical across attempts",
                member.to_uppercase(),
                rule.key
            )),
        );
        return;
    }
}

/// The declared option names, comma-separated — the help text every
/// [`check_options_arg`] diagnostic ends with, so a typo is answered with the
/// list rather than with a type spelling nobody wrote.
pub(crate) fn option_names(options: &[crate::ty::CoreShapeField]) -> String {
    options
        .iter()
        .map(|option| option.name.as_str())
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
///
///    **An array element is not one of them, and that is permanent** —
///    findings.md's D22, which was filed against the word *yet* this refusal
///    used to carry. `rule:types/arrays`'s copy-on-write leaves an element no
///    address that survives a call, so the only way to accept
///    `M::bump(inout $a["k"])` is to copy the element into a temporary at the
///    call site and copy it back afterwards. That is a reference only for as
///    long as the callee does not reach the same array: where it does, the
///    write-back lands after the callee's own writes and silently discards
///    them, and where it does not, the two are identical. Novis does not
///    offer a reference that is sometimes not one (priority 2, and `rule:types/arrays`'s separation is what buys priority 1), so the refusal names the
///    rewrite instead — the same copy, written where the reader can see it.
///    `an_array_element_passed_by_reference_is_diagnosed` in
///    `crates/nvs-types/tests/by_reference.rs` holds it.
/// 2. **Its type must be exactly the parameter's.** An ordinary argument may
///    widen on the way in (`int` into a `float` parameter); a by-reference one
///    may not, because the callee writes back at the *declared* type and the
///    caller's storage would then have to narrow on the way out — silently,
///    and lossily. `rule:types/declaration`'s "no type ever changes by itself" leaves no
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
    // `rule:classes/property-hooks` makes a hooked property's read a call and its write a
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
                "reading it runs its `get` hook and writing it runs its `set` hook (`rule:classes/property-observer` \
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
                    "an array element cannot be passed to an `inout` parameter",
                )
                .with_primary(arg.value.span, "passed by reference here")
                .with_help(
                    "`rule:types/arrays`'s copy-on-write separation gives an element no stable \
                     address — read it into a local, pass that, and write it back, where \
                     the copy is visible",
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
/// is what tells an integer literal it is a `uint` (`rule:types/arithmetic`'s rule, in
/// the [`ExprKind::Int`] arm of [`check_expr`]), so without it
/// `Core\Arr::padStart($a, 4, "-")` would report `expected uint, found int`
/// for a literal that is plainly in range -- while `Core\Str::padStart`, whose
/// signature happens to mention no variable and so never reaches this
/// function, accepted the same spelling. Substitution cannot change such a
/// position's type, and [`crate::generics::bind`] reads nothing out of it, so
/// knowing it early is free.
///
/// **A literal at a position only the bindings close is *placed* against the
/// substituted type, not compared to it** — findings.md's D33, decided here.
/// The alternative, admitting the literal to the binding pass on the strength
/// of the type it reports with nothing to be placed against, is the wrong one
/// in both directions: `Core\Test::assertSame($u, 2)` binds `T` to `uint` and
/// then compares an `int` against it, and `assertSame(2, $u)` binds `T` to
/// `int` and fails the `uint` instead. Placement is what an expected type
/// *does* for a literal (`rule:types/conversion`, "untyped until placed"), so the third
/// pass checks such an argument a second time against the now-known parameter
/// type rather than testing it for assignability, and
/// [`super::literals::is_unplaced_literal`] bounds which arguments are worth
/// that. It runs only where the first pass reported nothing about the
/// literal's own text, which is what keeps the promise above that nothing is
/// diagnosed twice.
///
/// The binding pass runs in two rounds for the same reason: a literal's
/// reported type is evidence about a variable only where nothing written
/// binds it. `Core\Arr::of(1, 2, 3)` still gets `T = int` out of the second
/// round, because there the literals are all there is.
///
/// Two arguments are left out of that first pass. An
/// `rule:core-api/shape-rules` R2 options bag is checked entirely in the last
/// one: it is always the last parameter, so nothing it could bind is ever
/// needed by an earlier one; and its own option types may mention a variable
/// the earlier arguments bind, so checking it first would check a field
/// against an unsubstituted `T`.
///
/// A `fn` literal at a parameter written in variables is the other, and it is
/// checked inside the binding pass, in a round of its own, because it is the
/// one argument that both *takes* a binding and *gives* one.
/// `rule:types/callable-literal-inference` types `$u` in
/// `Core\Arr::map($users, fn($u) => $u->name)` from the substituted parameter,
/// so that check cannot run until `T` is bound; the signature the literal then
/// reports is what binds `U`, so it has to run before
/// [`MethodSig::substituted`], which is where a variable nothing bound becomes
/// `mixed` and stops being bindable at all. Its round sits between the two:
/// after everything written, whose types are evidence it needs, and before the
/// untyped literals, because a parameter the author *annotated* is better
/// evidence about a variable than the type an unplaced literal takes with
/// nothing to be placed against.
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
    // The literals still waiting for a position, by argument index. Their
    // entry in `arg_types` is the type they take with nothing to be placed
    // against, which is what the second round of binding may fall back on;
    // the third pass places them properly.
    let mut unplaced = vec![false; list.len()];
    // The closure literals whose parameter types only the bindings can give,
    // by argument index. Their entry in `arg_types` is a placeholder too, and
    // the pass between the binding and the substitution below is where they
    // are checked.
    let mut deferred_fn = vec![false; list.len()];
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
        let declared = declared_for(slots[index], &sig, env.interner);
        // A `fn` literal at a parameter still written in variables is left for
        // the pass below: `rule:types/callable-literal-inference` gives its
        // unannotated parameters the types of the position it stands in, and
        // that position says `callable(T, string): U` until the other
        // arguments have bound `T`. Checked here it would take those types
        // from an unsubstituted variable — or, with none to take, report
        // `E0808` for a parameter the call site does describe.
        if defers_to_bindings(value, declared, env.interner) {
            deferred_fn[index] = true;
            arg_types.push(env.interner.mixed());
            continue;
        }
        let open = declared.is_some_and(|id| crate::generics::mentions_type_var(id, env.interner));
        let expected = if open {
            super::literals::unplaced_expectation(value, env)
        } else {
            declared
        };
        // Every diagnostic and not just the errors: a warning repeated is as
        // much a second report as an error repeated.
        let reported = env.diags.iter().len();
        arg_types.push(check_expr(value, expected, live, scope, ctx, env));
        // A literal that reported anything is left where it is: it has said
        // its piece already, and checking it again would repeat that.
        // `unplaced_expectation` is what keeps the one run of digits that
        // would otherwise land here out of it.
        unplaced[index] = open
            && super::literals::is_unplaced_literal(value)
            && env.diags.iter().len() == reported;
    }

    let mut bindings = crate::generics::Bindings::default();
    // Two rounds, the literals second — this function's docs own why. One
    // iterator over indexes rather than a nested loop, so the body below is
    // the single pass it reads as, and `or_insert` is what makes the second
    // round a fallback rather than a second opinion.
    let order = (0..arg_types.len())
        .filter(|index| !unplaced[*index] && !deferred_fn[*index])
        .chain((0..arg_types.len()).filter(|index| deferred_fn[*index]))
        .chain((0..arg_types.len()).filter(|index| unplaced[*index]));
    for index in order {
        // The bag alone: `arg_types` holds a placeholder for it until the last
        // pass, and binding a variable against that would bind it to `mixed`.
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
        // The deferred literal, checked in its own round: the expected type is
        // its parameter with the bindings so far put in, which is what types an
        // unannotated parameter, and the signature it answers with is then
        // bound below like any other argument's type. Before the substitution,
        // because [`crate::generics::substitute`] collapses a variable nothing
        // bound to `mixed` on the way past — including the one in the return
        // position, which is the one this check was run to learn.
        //
        // [`infer`] rather than [`check_expr`], so the expectation places the
        // literal's parameters without also *reporting* against it: a variable
        // still open here is one a later round may yet bind, and the pass below
        // asks the same question of the substituted type. That is where a
        // mismatched callback is reported, once.
        if deferred_fn[index] {
            let expected = crate::generics::substitute(declared, &bindings, env.interner);
            let value = &list[index].value;
            arg_types[index] = infer(value, Some(expected), live, scope, ctx, env);
        }
        let actual = arg_types[index];
        crate::generics::bind(
            declared,
            actual,
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
        // `rule:types/conversion`'s untyped literal, finally placeable: the bindings
        // have produced the type the first pass had none of. Checked rather
        // than compared, and returning rather than falling into the
        // assignability test below, for the bag's reason one arm up —
        // `check_arg` reports its own mismatch. A variable the call left
        // unbound is the one case where there is still nothing to place
        // against, and the ordinary path handles it.
        if unplaced[index] && !crate::generics::mentions_type_var(declared, env.interner) {
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

/// Whether this argument is a closure literal the bindings have to reach
/// before it can be checked: a `fn` written at a `rule:types/callable-signature`
/// parameter that still mentions a variable.
///
/// Narrow on purpose, in both halves. A written signature is the only expected
/// type that names a parameter position at all, so a literal anywhere else has
/// nothing to gain by waiting and would lose the binding it already makes. And
/// a parameter mentioning no variable
/// is its own final type in the first pass, so deferring it would only move
/// the same check later.
fn defers_to_bindings(value: &Expr, declared: Option<TypeId>, interner: &TypeInterner) -> bool {
    let Some(declared) = declared else {
        return false;
    };
    matches!(value.kind, ExprKind::Fn(_))
        && matches!(interner.get(declared), Ty::CallableSig { .. })
        && crate::generics::mentions_type_var(declared, interner)
}

/// The index of `sig`'s trailing options-bag parameter, if it has one — ADR
/// 0063 R2 puts at most one, and always last, which `nvs_stdlib::registry`'s
/// own `an_options_bag_is_last_and_never_empty` holds mechanically.
pub(crate) fn options_param(sig: &MethodSig, interner: &TypeInterner) -> Option<usize> {
    let last = sig.params.len().checked_sub(1)?;
    matches!(interner.get(sig.params[last]), Ty::CoreShape(_)).then_some(last)
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
                "user-declared type parameters are deferred (`rule:types/declaration`), and a `Core` member                  whose spec signature writes none infers every type it needs from its arguments",
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

/// What a call site wrote in that position: a declared class, or an inline
/// shape that names its own fields.
///
/// Two variants rather than one label, because the two are recorded on
/// different fields of [`crate::expr_table::ResolvedCall`] and reach `nvs-ir`
/// carrying different amounts: a class is a [`QName`] whose descriptor and
/// codec are both already filed under it, while a shape's descriptor is
/// labelled `$shape{…}` — which no name can be — and its wire contract is
/// filed under the call site instead, since the label cannot tell two shapes
/// with one field set apart.
pub(crate) enum WrittenTarget {
    /// A declared class, `Core\Json::decodeAs<User>`.
    Class(QName),
    /// An inline shape, `Core\Arr::shapeAs<{n: int}>`.
    Shape(crate::expr_table::WrittenShape),
}

impl WrittenTarget {
    /// Writes this target onto the call record, with the list flag both halves
    /// share.
    ///
    /// One method rather than a `match` at each of the two sites that build a
    /// [`crate::expr_table::ResolvedCall`], so the instance and static halves
    /// of the same roster cannot drift apart.
    pub(crate) fn record_on(self, call: &mut crate::expr_table::ResolvedCall, list: bool) {
        match self {
            Self::Class(class) => call.written_class = Some(class),
            Self::Shape(shape) => call.written_shape = Some(shape),
        }
        call.written_class_is_list = list;
    }
}

/// The class or inline shape a member on
/// `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` was asked to build and whether
/// it was written as a **list** of one, reporting `E_TYPE_ARG_NOT_A_CLASS` when
/// what was written is neither.
///
/// `array<C>` records `C` with `true`: a list decode is the same decode run
/// once per element, so what the native member needs is the element's and the
/// flag is the whole of what distinguishes the two shapes. Nesting stops
/// there — `array<array<C>>` is not a document shape `rule:core-api/required-optional-and-nullable` gives a
/// field, so it is refused here rather than recorded as a class it is not.
///
/// A shape leaves **two** records where a class leaves one: the label goes back
/// to the caller on [`WrittenTarget::Shape`], and the per-field wire contract
/// [`crate::derive::shape_codec`] reads off the type goes onto the expression
/// table under the type argument's own span. It cannot ride the label —
/// `{n: int}` and `{n: string}` are one `$shape{n}` — and it cannot ride
/// [`crate::expr_table::ResolvedCall`], which `nvs-ir` reads by span anyway.
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
) -> Option<(WrittenTarget, bool)> {
    if !nvs_stdlib::registry::takes_written_class(&owner.to_string(), method) {
        return None;
    }
    let first = *written.first()?;
    let (element, list) = match env.interner.get(first) {
        Ty::Array(element) => (*element, true),
        _ => (first, false),
    };
    if let Ty::Class(qname, _) = env.interner.get(element) {
        let qname = qname.clone();
        // `rule:core-classes/db-column-types`'s own question about the class, which is a question
        // about the whole program and so is only *recorded* here — see
        // [`crate::derive::check_row_sites`]. A decode site is the other
        // recorded question, for `rule:security/derived-codec-qualifiers`'s
        // qualifier rather than for a type map — see
        // [`crate::derive::check_decode_sites`] — and it is recorded for a
        // member whose document is a peer's whatever the argument carrying it
        // was typed as: `Core\Request::jsonAs` reads the request body, and
        // `Core\Jwt::verifyIssued` reads a payload another party wrote, which
        // `rule:security/verification-does-not-launder` keeps `tainted` because
        // a signature proves origin and not safety.
        // `Core\Json::decodeAs` gets neither: it takes its own document through
        // a plain `string` parameter, so a tainted argument is already refused
        // where it is passed rather than at the class it writes.
        let span = type_args.first().map_or(call_span, |ty| ty.span);
        if method == "jsonAs" || method == "verifyIssued" {
            env.decode_sites.push(crate::derive::DecodeSite::new(
                format!("{owner}::{method}"),
                qname.clone(),
                span,
            ));
        }
        if method == "queryAs" {
            env.row_sites.push(crate::derive::RowSite::new(
                format!("{owner}::{method}"),
                qname.clone(),
                list,
                span,
            ));
        }
        return Some((WrittenTarget::Class(qname), list));
    }
    // An inline shape declares its own fields, so the two whole-program
    // questions a class raises are already answered where it is written: there
    // is no declaration further down the file to find a deriving attribute on,
    // and no property list to look a column type up in later. It therefore
    // records no row site, and no decode site either — the qualifier question a
    // decode site exists to defer is answered here instead, against the fields
    // in hand. What it does record is the contract itself, read straight off
    // the type.
    if let Ty::Shape(fields) = env.interner.get(element) {
        let names: Vec<String> = fields.iter().map(|field| field.name.clone()).collect();
        let label = crate::derive::shape_class_label(&names);
        let span = type_args.first().map_or(call_span, |ty| ty.span);
        // Every `Core\Request` member on this roster reads the request —
        // `rule:security/tainted-sources` makes the body and the query alike a
        // peer's octets — and `Core\Jwt::verifyIssued` reads a payload another
        // party wrote, which `rule:security/verification-does-not-launder`
        // keeps `tainted`. The other owners do not: `Core\Json::decodeAs` takes
        // its document through a plain `string` parameter, so a tainted one is
        // refused where it is passed; `Core\Arr::shapeAs` converts an array the
        // program already holds, whose taint it carries in already; and a
        // `Core\Db` row is not a taint source at all.
        let owner_name = owner.to_string();
        if owner_name == r"Core\Request" || owner_name == r"Core\Jwt" {
            crate::derive::check_shape_decode_site(
                element,
                &format!("{owner}::{method}"),
                span,
                env.interner,
                env.diags,
            );
        }
        if let Some(codec) = crate::derive::shape_codec(element, env.interner, env.enums) {
            env.exprs.record_shape_codec(span, codec);
        }
        let shape = crate::expr_table::WrittenShape { label, codec: span };
        return Some((WrittenTarget::Shape(shape), list));
    }
    let found = env.interner.describe(first);
    let span = type_args.first().map_or(call_span, |ty| ty.span);
    env.diags.report(
        Diagnostic::error(
            code::E_TYPE_ARG_NOT_A_CLASS,
            format!(
                "`{owner}::{method}` builds a class or an inline shape, or a list of one, and \
                 `{found}` is none of those"
            ),
        )
        .with_primary(span, format!("`{found}` written here"))
        .with_help(
            "`rule:types/arrays`: a written type argument names the thing to build — a class \
             carrying the deriving attribute its format asks for, so hydrating is an ordinary \
             `new` (`rule:core-classes/derive-field-list`), or an inline shape such as \
             `{id: uint, name: string}`, which declares its own fields where it is written \
             (`rule:types/shape-type`). `array<…>` of either is the form for a source that is a \
             list",
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
/// that does not name one of `rule:iteration/concrete-generic-implements`'s two interfaces: `owner` must be
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
    // Two rosters, one question. `rule:iteration/concrete-generic-implements`'s interfaces are named by their
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::CoreShapeField;

    /// `rule:core-api/shape-flattens-at-the-abi`'s required half, asked of the rule itself: no registry row
    /// declares a shape parameter yet, so the call-site case that reports this
    /// arrives with `Core\Db::open` and this is what holds the rule until then.
    /// Both directions in one test on purpose — a bag is the all-optional
    /// special case, so a rule that reported a missing key correctly and also
    /// reported one for every bag would look right on either half alone.
    #[test]
    fn a_required_key_is_missing_only_when_the_literal_omits_it() {
        let mut interner = crate::ty::TypeInterner::new();
        let ty = interner.mixed();
        let key = |name: &str, required: bool| CoreShapeField {
            name: name.to_owned(),
            ty,
            required,
            qual: None,
        };
        let shape = [key("driver", true), key("host", true), key("port", false)];

        assert_eq!(
            missing_required_keys(&shape, &["driver"]),
            ["host"],
            "an optional key is never missing, and a written one is not either",
        );
        assert_eq!(
            missing_required_keys(&shape, &["port"]),
            ["driver", "host"],
            "reported in the merged list's own order, not the literal's",
        );
        assert!(missing_required_keys(&shape, &["host", "driver"]).is_empty());
        assert_eq!(required_key_names(&shape), "driver, host");

        // `rule:core-api/shape-rules` R2's bag: every field optional, so this rule has nothing to
        // say about one however little the call site wrote.
        let bag = [key("by", false), key("comparator", false)];
        assert!(missing_required_keys(&bag, &[]).is_empty());
        assert_eq!(required_key_names(&bag), "");
    }
}
