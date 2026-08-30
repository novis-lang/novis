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
//! The sinks reachable here are seven, and the three below are the ones a
//! conversion reaches. [`reject_non_literal_markup_conversion`]
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
//! The other three are read off a written argument rather than off a
//! conversion, because the member they reach declares `mixed` and the call
//! site is the last place the qualifier is visible:
//! [`reject_secret_debug_argument`], [`reject_secret_attribute_constant`], and
//! [`reject_secret_boundary_argument`] — ADR 0033 § 4's `serialize()`-and-
//! `spawn` bullet, which is one check for both of ADR 0023 § 2's carriers.
//!
//! The seventh has no member behind it at all: [`reject_secret_output`] is
//! § 4's terminal-output bullet, asked at `echo` and `print`, where the
//! operand is a statement's rather than a call's — and where the qualifier
//! has usually arrived by the spreading described above rather than being
//! written on the operand itself.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;
use nvs_stdlib::registry::Qual;

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

/// The same type with the `tainted` bit cleared, and everything else — the
/// `secret` axis, the base — left exactly as it was.
///
/// ADR 0088 § 2's admission is a **narrowing** of one qualifier at one kind of
/// position, and it is spelled as clearing the bit on the *argument* before
/// [`is_assignable`] sees it rather than as widening the parameter's declared
/// type. That is what keeps the diagnostic honest: the parameter is what a
/// mismatch names, and `expected tainted string` is a type no member declares
/// and no reader would recognise.
pub(crate) fn untainted(ty: TypeId, interner: &mut TypeInterner) -> TypeId {
    match qualifiable_base(ty, interner) {
        Some(is_bytes) if is_tainted(ty, interner) => {
            let secret = is_secret(ty, interner);
            qualified_scalar(is_bytes, false, secret, interner)
        }
        _ => ty,
    }
}

/// The same type with `tainted` set wherever the type can carry it — the atom
/// itself, an array's element, or every member of a union — and unchanged
/// where it cannot.
///
/// It reaches through `array<…>` and a union because that is where a
/// contagious member's answer actually lands: `Core\Str::split` hands back an
/// `array<string>` and `Core\Str::after` a `?string`, and a result that
/// dropped the qualifier on the way through either would be laundering by
/// return type. Nothing else is reached into — a shape's or an object's own
/// fields are qualified where they are written, not here — which is exactly
/// the case [`admits_tainted_argument`] refuses the argument for rather than
/// laundering it.
pub(crate) fn tainted_result(ty: TypeId, interner: &mut TypeInterner) -> TypeId {
    if let Some(is_bytes) = qualifiable_base(ty, interner) {
        let secret = is_secret(ty, interner);
        return qualified_scalar(is_bytes, true, secret, interner);
    }
    match interner.get(ty).clone() {
        Ty::Array(elem) => {
            let elem = tainted_result(elem, interner);
            interner.array(elem)
        }
        Ty::Union(members) => {
            let members: Vec<TypeId> = members
                .iter()
                .map(|&member| tainted_result(member, interner))
                .collect();
            interner.make_union(members)
        }
        _ => ty,
    }
}

/// ADR 0088 § 2's admission as one question: does a parameter the registry
/// classified `qual` accept an argument carrying `tainted`?
///
/// * A [`Qual::Sink`] never does — that is the whole of ADR 0088 § 1 — and
///   neither does an unclassified parameter, which is § 2's flipped default
///   and every parameter of a signature that is not a `Core` row
///   ([`MethodSig::param_quals`]).
/// * [`Qual::Neutral`] and [`Qual::Launder`] always do, for one reason:
///   neither puts a byte of the argument into the answer, so there is nothing
///   for the qualifier to be carried into. `Launder` is not named in § 2's own
///   sentence and follows from its definition — a member that removes the
///   qualifier and refused to be handed one would launder nothing.
/// * [`Qual::Contagious`] does **only where the result can carry the qualifier
///   back out**, which is what `tainted_result` answering something different
///   means. That conservative half is deliberate: admitting a tainted argument
///   into a member whose return type has nowhere to put the bit is laundering,
///   and AGENTS.md's ordering does not trade priority 1 for a call that
///   compiles. A member in that position is either mis-classified — an answer
///   carrying no byte of any argument is `Neutral` — or answers a shape or an
///   object, whose fields are a slice of their own.
///
/// * [`Qual::Reveal`] does, for [`Qual::Contagious`]'s reason and not for
///   [`Qual::Launder`]'s: the mark removes `secret`, which says nothing about
///   `tainted`, so a `tainted secret string` handed to `Core\Secret::reveal`
///   answers a `tainted string` and the bit has somewhere to go.
///
/// Only the `tainted` axis is admitted. `secret` is refused here exactly as it
/// is refused today: whether a `Neutral` parameter launders `secret` is a
/// laundering decision ADR 0088 owes an answer to, and being over-strict costs
/// a refusal rather than a leak. [`Qual::Reveal`] is the one mark that will
/// answer differently — it is ADR 0033 § 3's named escape hatch and the
/// spelling decision recorded in `nvs_stdlib::registry`'s [`Qual`] doc comment
/// — but the admission itself lands with `Core\Secret`'s own rows, so today the
/// mark is written by no row and this function still refuses every `secret`
/// argument in the tree.
pub(crate) fn admits_tainted_argument(
    qual: Option<Qual>,
    return_ty: TypeId,
    interner: &mut TypeInterner,
) -> bool {
    match qual {
        None | Some(Qual::Sink) => false,
        Some(Qual::Neutral | Qual::Launder | Qual::Reveal) => true,
        Some(Qual::Contagious) => tainted_result(return_ty, interner) != return_ty,
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

/// Whether `ty` carries ADR 0033 § 1's `secret` anywhere a serialiser would
/// walk to: on the type itself, or on an element, a field or a member of the
/// composites a written value takes. [`is_secret`] answers the atom; this
/// answers the whole value, which is what ADR 0033 § 4's serialiser bullet
/// asks for — "a `secret` anywhere in the value it walks".
///
/// It stops at a class, deliberately: a `secret`-typed *property* of an
/// object being encoded is the run-time walk's question, the same split
/// [`reject_secret_boundary_argument`] documents, and a static rule reaching
/// into it would answer half of it twice.
pub(crate) fn contains_secret(ty: TypeId, interner: &TypeInterner) -> bool {
    if is_secret(ty, interner) {
        return true;
    }
    match interner.get(ty) {
        Ty::Array(elem) => contains_secret(*elem, interner),
        Ty::Shape(fields) | Ty::Options(fields) => {
            fields.iter().any(|(_, f)| contains_secret(*f, interner))
        }
        Ty::Union(members) | Ty::Intersection(members) => {
            members.iter().any(|&m| contains_secret(m, interner))
        }
        _ => false,
    }
}

/// ADR 0033 § 4's serialiser sink: `Core\Json::encode` refuses a `secret`
/// anywhere in the value it is handed.
///
/// A call-site rule rather than a parameter type, exactly as
/// [`reject_secret_debug_argument`] is — `encode` declares `mixed`, which a
/// `secret string` satisfies. What it adds over that one is [`contains_secret`]
/// rather than [`is_secret`]: an encoded document is written *through* its
/// composites, so the array literal holding one credential beside four public
/// fields is the shape this exists for, and refusing only a bare `secret`
/// argument would leave the common spelling open.
///
/// **What it does not reach is the written array literal**, and that is the
/// container axis rather than this rule: an `["token" => $s]` with no
/// expectation on it infers `array<mixed>`
/// ([`check_array_literal`](super::literals::check_array_literal) joins
/// nothing), so the qualifier is gone before the call is looked at, and it is
/// equally gone one statement later through a variable — which no call-site
/// rule could recover. ADR 0033 names that gap as its own; the fix is an
/// element type for a literal, not a second walk here.
///
/// The way out is written at the field rather than at the call —
/// `Core\Secret::reveal(..., "reason")` on the one value that must travel — so
/// the rest of the document stays covered by the rule. That is § 4's own
/// sentence, and it is why this help does not read like
/// [`reject_secret_debug_argument`]'s.
pub(crate) fn reject_secret_encoded_argument(
    qname: &QName,
    member: &str,
    args: &CallArgs,
    arg_types: &[TypeId],
    env: &mut Env<'_>,
) {
    if qname.to_string() != r"Core\Json" || member != "encode" {
        return;
    }
    let CallArgs::List(list) = args else {
        return;
    };
    for (arg, &ty) in list.iter().zip(arg_types) {
        if !contains_secret(ty, env.interner) {
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_SECRET_ENCODED,
                "a `secret`-qualified value cannot be passed to `Core\\Json::encode`; an \
                 encoded document is on its way to a response, a log or a queue, and none \
                 of those is the credential being used",
            )
            .with_primary(arg.value.span, "secret value encoded here")
            .with_help(
                "reveal the one field that must travel with \
                 `Core\\Secret::reveal(..., \"reason\")`, written at that field rather than \
                 at the call, so the rest of the value stays covered",
            ),
        );
    }
}

/// ADR 0033 § 4's terminal-output sink: `echo` and `print` refuse a
/// `secret`-qualified operand, with **no `Core\Cli\Text` bypass**. Returns
/// whether it refused, so a caller can leave [`require_stringable`] unasked —
/// `secret bytes` is the one operand both would answer, and confidentiality
/// is the half to act on.
///
/// This is the one sink of § 4's list that is reached by a *statement* rather
/// than by a member, which is why it is checked here instead of falling out
/// of a parameter type: `Core\Cli::write` declares plain `string` and is
/// already refused by the assignability rule, and there is no signature
/// anywhere behind an `echo`.
///
/// **The operand is usually not the secret binding**, and the help is written
/// for that: interpolation and `.` spread the qualifier to their result (see
/// this module's own head), so `echo "Bearer $token"` arrives here as a
/// `secret string` whose span is the whole literal. That spreading is what
/// makes one check at the sink cover ADR 0033 § 4's *"`echo` and
/// interpolation"* both, rather than needing a rule per composition form.
///
/// The reach is wider than the word "terminal": ADR 0088 § 3 sends a
/// scheduled script's, a job worker's, a `#[Test]` method's and a
/// `spawn script` isolate's output through this same sink, so what this
/// refuses is as often a credential landing in a CI log as one printed to a
/// tty. A tty-dependent version of the rule is not available — ADR 0086 § 1
/// rejects tty-dependent behaviour outright.
pub(crate) fn reject_secret_output(ty: TypeId, span: Span, form: &str, env: &mut Env<'_>) -> bool {
    if !is_secret(ty, env.interner) {
        return false;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_OUTPUT,
            format!(
                "a `secret`-qualified value cannot be written by `{form}`; output is read \
                 by a person or captured into a log, so the value would be disclosed \
                 rather than used"
            ),
        )
        .with_primary(span, "secret value written here")
        .with_help(
            "reveal it explicitly first with `Core\\Secret::reveal(..., \"reason\")`; \
             interpolating or concatenating it into a larger string does not help — the \
             qualifier spreads to the result",
        ),
    );
    true
}

/// ADR 0033 § 4's cross-boundary sink: a `secret`-qualified value handed to
/// [ADR 0023](../../../../docs/adr/0023-clone-serialize-and-cross-boundary-copy.md)
/// § 2's graph copy.
///
/// **One check for both carriers**, which is how § 4 states the rule: the
/// bullet refuses the value "at the one recursive graph-copy operation ADR
/// 0023 already defines, for both its callers alike, rather than drawing a new
/// distinction between crossing to a live isolate and externalizing to bytes."
/// `Core\Serialize::encode` is this function's caller; `spawn script`'s `args:`
/// is the other carrier and it reaches the same refusal through
/// [`reject_secret_crossing`], from `super::isolate::check_spawn_script`, rather
/// than growing a rule of its own. That shared tail is the whole reason this
/// reads the arguments and their types instead of the member's signature —
/// `args:` has no signature to read.
///
/// A call-site rule rather than a parameter type, exactly as
/// [`reject_secret_debug_argument`] is: `encode` declares `mixed`, which a
/// `secret string` satisfies, so the written argument is the last place the
/// qualifier is still visible.
///
/// **What this does not reach, and why that is not a gap.** A `secret`-typed
/// *property* of an object being copied is invisible from a call site — the
/// argument's static type is the class, not its storage — so the walk itself
/// refuses that one at run time (`nvs_runtime::graph`'s `field_is_secret`).
/// The two halves are the same rule read off the two things that can carry the
/// qualifier, which is why they share [`code::E_SECRET_CROSSES_A_BOUNDARY`]'s
/// reasoning without sharing a mechanism.
pub(crate) fn reject_secret_boundary_argument(
    qname: &QName,
    member: &str,
    args: &CallArgs,
    arg_types: &[TypeId],
    env: &mut Env<'_>,
) {
    if qname.to_string() != r"Core\Serialize" || member != "encode" {
        return;
    }
    let CallArgs::List(list) = args else {
        return;
    };
    for (arg, &ty) in list.iter().zip(arg_types) {
        reject_secret_crossing(
            &arg.value,
            ty,
            "`Core\\Serialize::encode` writes it into bytes that outlive the request",
            env,
        );
    }
}

/// § 4's refusal itself, with the carrier named by the caller.
///
/// The sentence around `carrier` is the same for every caller on purpose: § 4
/// refuses the value **at the graph copy**, not at either carrier's own
/// surface, so a reader who has seen the message once at
/// `Core\Serialize::encode` recognizes it unchanged at `spawn script`. Only the
/// middle clause — where the copy goes and why the qualifier cannot follow it —
/// differs, and it is a clause rather than a whole message so the two cannot
/// drift into two different explanations of one rule.
///
/// The type is the *argument's* inferred type, never the parameter's: `encode`
/// declares `mixed` and `args:` declares nothing at all, so the written
/// expression is the last place the qualifier is still visible.
pub(crate) fn reject_secret_crossing(at: &Expr, ty: TypeId, carrier: &str, env: &mut Env<'_>) {
    if !is_secret(ty, env.interner) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_CROSSES_A_BOUNDARY,
            format!(
                "a `secret`-qualified value cannot cross a copy boundary; {carrier}, \
                 where nothing carries the qualifier that was protecting it"
            ),
        )
        .with_primary(at.span, "secret value copied out here")
        .with_help(
            "reveal it explicitly first with `Core\\Secret::reveal(..., \"reason\")`, at \
             the one call site where handing the secret over is the point; a \
             `secret`-typed *property* of a copied object needs nothing here — the walk \
             refuses that one itself",
        ),
    );
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
/// It is the one sink whose qualifier is read off a table rather than off the
/// inferred type, and that is now a question of *when* rather than of whether:
/// a payload is checked over the written expression, where the class name has
/// no resolved [`nvs_hir::QName`] for [`crate::signatures::resolve_const`] to
/// be asked with. [`crate::consts::ConstTable`] is keyed by the same name this
/// walk resolves for itself, so the bit is read from there.
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
