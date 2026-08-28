//! ADR 0024's `tainted` and ADR 0033's `secret`: one representation, two
//! independent axes, and the sinks that refuse them.
//!
//! The two are implemented together because they share one representation
//! ([`Ty::TaintedString`]/[`Ty::SecretString`]/[`Ty::SecretTaintedString`]/
//! etc. — one atom per combination) and one set of helpers
//! ([`is_tainted`]/[`is_secret`]/[`qualifiable_base`]/[`qualified_scalar`]).
//! Concatenation and interpolation poison their result on each axis
//! independently, exactly like ADR 0007's `mixed`-arithmetic precedent.
//! [`apply_qualifier_conversion_rule`] is the conversion half: a checked
//! conversion to `uint`/`int`/`float`/`bool`/an enum's backing type launders
//! both qualifiers for free, since none of those targets carry either to begin
//! with (a known, ADR-accepted gap for `secret`: unlike `tainted`,
//! "shape-proof implies safe" doesn't actually transfer — see ADR 0033 § 2's
//! own *Alternatives rejected*), while `bytes`/`string` (including the
//! identity-shaped `tainted string as string`/`secret string as string`, which
//! would otherwise be a silent bypass) keep both qualifiers across either
//! direction, per ADR 0009 § 3. In [`super::assign`]'s relation a same-base
//! value widens freely on either bit — a trusted, non-secret value is always a
//! safe over-approximation of "may be tainted"/"may be secret," the same
//! direction `mixed` never gets — but never narrows through assignment.
//!
//! The M2-reachable sinks are three. [`reject_non_literal_markup_conversion`]
//! is ADR 0024 § 5's one M2-scoped rule: `as Core\Html\Markup` accepts only a
//! literal string token, `tainted` or not — the rest of § 5 (auto-escaping,
//! `Markup + Markup`) waits on `Core\Html` actually existing.
//! [`reject_secret_markup_conversion`] is ADR 0033 § 4's sibling, giving a
//! `secret` operand there its own specific diagnostic ahead of the generic one
//! (escaping doesn't restore confidentiality, so `secret` gets no auto-escape
//! carve-out even once one exists for `tainted`). And
//! [`reject_secret_throwable_message`] is a `Throwable`-shaped class's
//! constructor message argument — see [`is_throwable_shaped`] for how that is
//! decided without a declared `Throwable`/`Exception`/`Error` stdlib to check
//! against.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;

/// Whether `ty` carries ADR 0024 § 1's `tainted` qualifier — on its own
/// (`tainted string`/`tainted bytes`) or composed with `secret`
/// (`secret tainted string`/`secret tainted bytes`, ADR 0033 § 1). The one
/// question every taint propagation/laundering rule in this module reduces
/// to.
pub(crate) fn is_tainted(ty: TypeId, interner: &TypeInterner) -> bool {
    matches!(
        interner.get(ty),
        Ty::TaintedString | Ty::TaintedBytes | Ty::SecretTaintedString | Ty::SecretTaintedBytes
    )
}

/// Whether `ty` carries ADR 0033 § 1's `secret` qualifier — on its own or
/// composed with `tainted`. The `secret`-axis counterpart of [`is_tainted`];
/// the two are independent bits, so a caller checking one never implies
/// anything about the other.
pub(crate) fn is_secret(ty: TypeId, interner: &TypeInterner) -> bool {
    matches!(
        interner.get(ty),
        Ty::SecretString | Ty::SecretBytes | Ty::SecretTaintedString | Ty::SecretTaintedBytes
    )
}

/// Whether `ty` is one of the eight `string`/`bytes`-shaped atoms `tainted`/
/// `secret` apply to — every combination of the two qualifier bits over the
/// same base. Returns `Some(true)` for a `bytes`-based atom, `Some(false)`
/// for a `string`-based one, `None` for anything else (a scalar this axis
/// pair never touches, a class, `mixed`, ...).
pub(crate) fn qualifiable_base(ty: TypeId, interner: &TypeInterner) -> Option<bool> {
    match interner.get(ty) {
        Ty::String | Ty::TaintedString | Ty::SecretString | Ty::SecretTaintedString => Some(false),
        Ty::Bytes | Ty::TaintedBytes | Ty::SecretBytes | Ty::SecretTaintedBytes => Some(true),
        _ => None,
    }
}

/// Interns whichever of the eight `string`/`bytes`-shaped atoms `is_bytes`/
/// `tainted`/`secret` name — the one place that maps the two independent
/// qualifier bits back onto [`Ty`]'s one-atom-per-combination representation.
pub(crate) fn qualified_scalar(
    is_bytes: bool,
    tainted: bool,
    secret: bool,
    interner: &mut TypeInterner,
) -> TypeId {
    match (is_bytes, tainted, secret) {
        (false, false, false) => interner.string(),
        (false, true, false) => interner.tainted_string(),
        (false, false, true) => interner.secret_string(),
        (false, true, true) => interner.secret_tainted_string(),
        (true, false, false) => interner.bytes(),
        (true, true, false) => interner.tainted_bytes(),
        (true, false, true) => interner.secret_bytes(),
        (true, true, true) => interner.secret_tainted_bytes(),
    }
}

/// ADR 0024 § 2 / ADR 0033 § 2: what an `as` conversion's result carries on
/// the `tainted`/`secret` axes. A checked conversion that already throws on
/// a malformed shape — `uint`, `int`, `float`, `bool`, an enum's backing
/// type — removes both qualifiers on success for free, since none of those
/// targets carry either to begin with (ADR 0033 § 2's known, accepted gap:
/// this strips `secret` too, even though "shape-proof implies safety" only
/// ever justified it for `tainted`). Otherwise `to` is itself one of the
/// eight `string`/`bytes` atoms, and `from`'s qualifiers cross into it
/// unconditionally — including the identity-shaped `tainted string as
/// string`/`secret string as string`, which are not themselves
/// shape-proving conversions and must not silently launder; that would be
/// exactly the bypass this whole mechanism exists to close.
pub(crate) fn apply_qualifier_conversion_rule(
    from: TypeId,
    to: TypeId,
    interner: &mut TypeInterner,
) -> TypeId {
    let from_tainted = is_tainted(from, interner);
    let from_secret = is_secret(from, interner);
    if !from_tainted && !from_secret {
        return to;
    }
    let Some(to_is_bytes) = qualifiable_base(to, interner) else {
        return to;
    };
    let to_tainted = from_tainted || is_tainted(to, interner);
    let to_secret = from_secret || is_secret(to, interner);
    qualified_scalar(to_is_bytes, to_tainted, to_secret, interner)
}

/// ADR 0033 § 4: a `secret`-qualified value converted `as Core\Html\Markup`
/// is refused with a diagnostic naming the qualifier specifically, ahead of
/// [`reject_non_literal_markup_conversion`]'s generic "must be a literal"
/// one — escaping (this ADR's whole reason for diverging from `tainted`'s
/// auto-escape default) neutralizes injection risk, not exposure, so it is
/// the wrong tool here regardless of how the value was produced. In
/// practice a `secret` value is never a literal token to begin with (nothing
/// grants `secret` ambiently — ADR 0033 § 1 — so it only ever reaches this
/// point through a declared binding), meaning the generic literal check
/// alone would already refuse it; this exists to give that refusal its own,
/// more specific reason. Scoped to a conversion whose target actually
/// resolves to `Core\Html\Markup`, same as its sibling; the inline
/// `<?= expr ?>`/templating-helper interpolation position ADR 0033 § 4 also
/// names waits on `Core\Html` actually existing (see the crate docs' known
/// gaps).
pub(crate) fn reject_secret_markup_conversion(
    inner_ty: TypeId,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) {
    let Ty::Class(qname, _) = env.interner.get(to) else {
        return;
    };
    if qname.to_string() != "Core\\Html\\Markup" {
        return;
    }
    if !is_secret(inner_ty, env.interner) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_MARKUP_UNSUPPORTED,
            "a `secret`-qualified value cannot be converted `as Markup`; escaping neutralizes \
             injection risk, not confidentiality, so it is refused outright rather than \
             auto-escaped",
        )
        .with_primary(span, "converted here")
        .with_help("reveal it explicitly first with `Core\\Secret::reveal(..., \"reason\")`"),
    );
}

/// ADR 0024 § 5: `as Core\Html\Markup` trusts only a source-literal string —
/// a `tainted` value, or any other runtime-computed one, can never become
/// trusted markup this way, closing "compute the escape-defeating payload at
/// runtime, then cast it." Scoped to a conversion whose target actually
/// resolves to `Core\Html\Markup`; every other target is untouched. The rest
/// of § 5 (auto-escaping a non-`Markup` interpolation, `Markup + Markup`)
/// waits on `Core\Html` actually existing as a stdlib class.
pub(crate) fn reject_non_literal_markup_conversion(
    inner: &Expr,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) {
    let Ty::Class(qname, _) = env.interner.get(to) else {
        return;
    };
    if qname.to_string() != "Core\\Html\\Markup" {
        return;
    }
    if is_literal_string(inner) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_MARKUP_REQUIRES_LITERAL,
            "only a source-literal string may be converted `as Markup`; a runtime-computed or \
             `tainted` value can never become trusted markup this way",
        )
        .with_primary(span, "converted here")
        .with_help("build markup from literal fragments, or escape via a `Core\\Html` helper"),
    );
}

/// Whether `expr` is a literal string token, unwrapping any enclosing
/// parentheses — `("text")` is exactly as trusted as `"text"` for
/// [`reject_non_literal_markup_conversion`]'s purposes.
pub(crate) fn is_literal_string(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Str(_) => true,
        ExprKind::Paren(inner) => is_literal_string(inner),
        _ => false,
    }
}

/// Whether `qname` is `Throwable` or reaches it by walking its `extends`
/// chain — the same reachability question [`nvs_hir::implements_interface`]
/// already answers for `Comparable`/`Stringable`.
///
/// One root is enough because spec § 10 makes `Throwable` the only one:
/// every other exception class, seeded or user-declared, descends from it
/// through links `nvs_hir::seed_exception_tree` put in the graph.
pub(crate) fn is_throwable_shaped(qname: &QName, graph: &ClassGraph) -> bool {
    let root = QName::parse(nvs_hir::errors::ROOT);
    *qname == root || nvs_hir::implements_interface(qname, &root, graph)
}

/// ADR 0033 § 4: a `Throwable`-shaped class's constructor message argument
/// (its first positional argument) refuses a `secret`-qualified value —
/// closing the common real-world leak of a credential ending up in a stack
/// trace or an error page, the same "sink requires the plain type" shape ADR
/// 0024 § 4 already gives `Core\Db`'s query text. `first_arg_ty` is the
/// already-checked type [`check_args_typed`] computed for that argument, so
/// this never re-walks (and re-diagnoses) the expression itself. `Throwable`/
/// `Exception`/`Error` have no declared stdlib member table yet, so this is
/// scoped to the constructor call shape alone — "anywhere a message is later
/// composed" (e.g. through a setter) is not modeled this slice.
pub(crate) fn reject_secret_throwable_message(
    qname: &QName,
    first_arg_ty: Option<TypeId>,
    span: Span,
    env: &mut Env<'_>,
) {
    if !is_throwable_shaped(qname, env.graph) {
        return;
    }
    let Some(ty) = first_arg_ty else {
        return;
    };
    if !is_secret(ty, env.interner) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_THROWABLE_MESSAGE,
            "a `secret`-qualified value cannot be passed as a `Throwable` message; it would \
             surface in a stack trace or an error page",
        )
        .with_primary(span, "secret value used as the message here")
        .with_help("reveal it explicitly first with `Core\\Secret::reveal(..., \"reason\")`"),
    );
}

/// ADR 0033 § 4's debug-dump sink at its *call-site* half, which is how
/// [ADR 0092](../../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
/// § 5's redaction row states it: a property whose declared type carries
/// `secret` becomes a Redacted node, and a `secret` value handed straight to
/// the dump is refused by `nvs check`. The two halves are one rule about one
/// record seen from its two ends — what the walk finds behind an object, and
/// what the author wrote at the site — and only the second is still visible
/// as a *type*, which is why it is checked here at all: both members declare
/// `mixed ...$values`/`mixed $value`, a parameter a `secret string`
/// satisfies, so the qualifier survives nowhere below this point. That is the
/// same mechanism split § 4 already makes for `Core\Log::write`, and its
/// *Context* is that argument's one home.
///
/// **`render` is refused on the same terms as `dump`, and not as an
/// extension of the item that added this.** ADR 0092 § 5's closing paragraph
/// makes the renderings non-bypassable — there is no `dumpRaw` and no
/// rendering selected by an argument — so a member that answers the record's
/// text as a `Core\Cli\Text` carrier is the same disclosure one `echo` later,
/// and § 4's terminal bullet refuses that value with **no `Core\Cli\Text`
/// bypass** in any case. Refusing only `dump` would leave
/// `echo Core\Debug::render($secret)` as the way round both bullets.
///
/// Scoped to the arguments this call actually writes, in written order, so a
/// `dump($a, $secret, $b)` names the one it is about. Two shapes it does not
/// reach and neither is a gap in this rule: a `...$xs` spread hands over a
/// subject whose *element* type carries the qualifier, and a `secret` value
/// stored in a property or an array element reaches the walk rather than the
/// site — the first is ADR 0033's unmodelled container axis, the second is
/// the Redacted node this row's other half owns.
pub(crate) fn reject_secret_debug_argument(
    qname: &QName,
    member: &str,
    args: &CallArgs,
    arg_types: &[TypeId],
    env: &mut Env<'_>,
) {
    if qname.to_string() != r"Core\Debug" || !matches!(member, "dump" | "render") {
        return;
    }
    let CallArgs::List(list) = args else {
        return;
    };
    for (arg, &ty) in list.iter().zip(arg_types) {
        if !is_secret(ty, env.interner) {
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_SECRET_DEBUG_ARGUMENT,
                format!(
                    "a `secret`-qualified value cannot be passed to \
                     `Core\\Debug::{member}`; a dump is written to be read by a person, so \
                     the value would be disclosed rather than used"
                ),
            )
            .with_primary(arg.value.span, "secret value dumped here")
            .with_help(
                "reveal it explicitly first with `Core\\Secret::reveal(..., \"reason\")`; a \
                 `secret`-typed *property* of a dumped object needs nothing — the record \
                 redacts it",
            ),
        );
    }
}

/// ADR 0033 § 4's fifth sink: a `secret` class constant reaching an attribute
/// payload, reported at the value where it is written.
///
/// The sink exists because of [ADR 0046](../../../../docs/adr/0046-attributes-shape-literal-metadata.md)
/// § 2 rather than in spite of it. A payload admits only compile-time
/// constants — no variable, no call, no `new` — and a class constant is one of
/// the shapes it admits, so the storage class ADR 0033's own values live in is
/// the *only* way a `secret` value could reach a payload at all. Every other
/// spelling is already refused for being computed, which is why this is one
/// check over one expression kind rather than a walk of its own.
///
/// It is the one sink whose qualifier cannot be read off an inferred type, and
/// that is a gap one crate over rather than a choice: `crate::signatures`
/// leaves a class constant's declared type unmodeled, so `Class::TOKEN` infers
/// `mixed` at every expression site and [`is_secret`] over that answers
/// `false` for a value that plainly is one. [`crate::consts::ConstTable`] is
/// where the annotation was last visible, so the bit is read from there.
///
/// Two shapes it does not reach and neither is a gap in this rule: a `Core`
/// class's constant, which is the registry's own declaration and carries no
/// user qualifier, and a constant reached through a class expression that is
/// not statically known, which `resolve_class_expr` has already refused for
/// its own reasons.
pub(crate) fn reject_secret_attribute_constant(expr: &Expr, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let ExprKind::ClassConstAccess { class, name } = &expr.kind else {
        return;
    };
    let Some(qname) = super::members::resolve_class_expr(class, ctx, env) else {
        return;
    };
    let constant = span_text(env.src, *name).to_owned();
    if !env.consts.is_secret(&qname, &constant, env.graph) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_ATTRIBUTE_PAYLOAD,
            format!(
                "a `secret`-qualified class constant cannot appear in an attribute payload; \
                 `{qname}::{constant}` would be folded into the compiled unit's constant pool \
                 and handed back to anything that reads the attribute"
            ),
        )
        .with_primary(expr.span, "secret value written into metadata here")
        .with_help(
            "ADR 0033 § 4: there is no `Core\\Secret::reveal(..., \"reason\")` way out of this \
             one, a payload admitting no call at all — a credential belongs somewhere read at \
             run time, and the attribute carries the name of where to read it from",
        ),
    );
}
