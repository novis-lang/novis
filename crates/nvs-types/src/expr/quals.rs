//! `rule:security/tainted-qualifier`'s `tainted` and `rule:security/secret-qualifier`'s `secret`: one representation, two
//! independent axes, and the sinks that refuse them.
//!
//! The two are implemented together because they share one representation
//! ([`Ty::TaintedString`]/[`Ty::SecretString`]/[`Ty::SecretTaintedString`]/
//! etc. — one atom per combination) and one set of helpers
//! ([`is_tainted`]/[`is_secret`]/[`qualifiable_base`]/[`qualified_scalar`]).
//! Concatenation and interpolation poison their result on each axis
//! independently, exactly like `rule:types/declaration`'s `mixed`-arithmetic precedent.
//! [`apply_qualifier_conversion_rule`] is the conversion half: a checked
//! conversion to `uint`/`int`/`float`/`bool`/an enum's backing type launders
//! both qualifiers for free, since none of those targets carry either to begin
//! with (a known, ADR-accepted gap for `secret`: unlike `tainted`,
//! "shape-proof implies safe" doesn't actually transfer — see `rule:security/secret-propagation`'s
//! own *Alternatives rejected*), while `bytes`/`string` (including the
//! identity-shaped `tainted string as string`/`secret string as string`, which
//! would otherwise be a silent bypass) keep both qualifiers across either
//! direction, per `rule:types/conversion`. In [`super::assign`]'s relation a same-base
//! value widens freely on either bit — a trusted, non-secret value is always a
//! safe over-approximation of "may be tainted"/"may be secret," the same
//! direction `mixed` never gets — but never narrows through assignment.
//!
//! The sinks reachable here are eleven, and the three below are the ones a
//! conversion reaches. [`reject_computed_markup_conversion`]
//! is `rule:core-classes/html-auto-escape`'s one M2-scoped rule: `as Core\Html\Markup` accepts only a
//! literal string token, `tainted` or not — the rest of § 5 (auto-escaping,
//! `Markup + Markup`) waits on `Core\Html` actually existing.
//! [`reject_secret_markup_conversion`] is `rule:security/secret-sinks-refuse`'s sibling, giving a
//! `secret` operand there its own specific diagnostic ahead of the generic one
//! (escaping doesn't restore confidentiality, so `secret` gets no auto-escape
//! carve-out even once one exists for `tainted`). And
//! [`reject_secret_throwable_message`] is a `Throwable`-shaped class's
//! constructor message argument — see [`is_throwable_shaped`] for how that is
//! decided without a declared `Throwable`/`Exception`/`Error` stdlib to check
//! against.
//!
//! The next seven are read off a written argument rather than off a
//! conversion, because the member they reach declares an open type and the
//! call site is the last place the qualifier is visible:
//! [`reject_secret_debug_argument`], [`reject_secret_attribute_constant`],
//! [`reject_secret_boundary_argument`] — `rule:security/secret-sinks-refuse`'s `serialize()`-and-
//! `spawn` bullet, which is one check for both of `rule:classes/graph-copy`'s carriers —
//! [`reject_secret_published_argument`], which is `rule:core-classes/topic`'s bus reaching
//! that same graph copy through a third carrier,
//! [`reject_secret_cached_argument`], which is `Core\Cache\Store::put` reaching
//! it through a fourth, [`reject_secret_session_argument`], which is
//! `Core\Session::set` reaching it through the record a request writes back —
//! those two being the refusals that name a member taking the secret rather
//! than a reveal — [`reject_secret_encoded_argument`],
//! [`reject_secret_enqueued_argument`], which is that same encoder reached
//! through `Core\Queue::push` rather than written at the call, and
//! [`reject_secret_logged_argument`], whose open type is `array<mixed>` **by
//! design** rather than pending. The last two are the ones that also read the
//! elements of a written literal, because a payload with one credential among
//! public fields is where the qualifier is in practice.
//!
//! The last has no member behind it at all: [`reject_secret_output`] is
//! § 4's terminal-output bullet, asked at `echo` and `print`, where the
//! operand is a statement's rather than a call's — and where the qualifier
//! has usually arrived by the spreading described above rather than being
//! written on the operand itself.
//!
//! [`reject_secret_into_container`] is deliberately not counted among those
//! sinks: a sink refuses a value about to be *disclosed*, and this refuses one
//! about to be *stored* where the qualifier stops being visible —
//! `rule:security/secret-qualifier`'s container axis, asked at an array
//! element and at a shape field, with [`reject_secret_element_write`] the same
//! question at `$a["k"] = $secret`. It is the one refusal here that an
//! argument list steps past, because the three positions
//! `rule:security/secret-sinks-refuse` leaves open are all written as one.
//!
//! Part of [`super`]'s one expression checker, split across this directory so
//! a session editing one rule does not carry the rest in context. Every item
//! moved here unchanged; an item is `pub(crate)` where it reaches across these
//! modules, which is the reach it had when `expr` was a single file.

use super::*;
use nvs_stdlib::registry::Qual;

/// Whether `ty` carries `rule:security/tainted-qualifier`'s `tainted` qualifier — on its own
/// (`tainted string`/`tainted bytes`) or composed with `secret`
/// (`secret tainted string`/`secret tainted bytes`, `rule:security/secret-qualifier`). The one
/// question every taint propagation/laundering rule in this module reduces
/// to.
///
/// `pub` because `nvs_lsp::semantic` colours a use site with the qualifier its
/// value carries, and "what does this type carry" must have one answer for the
/// checker and the editor both.
#[must_use]
pub fn is_tainted(ty: TypeId, interner: &TypeInterner) -> bool {
    matches!(
        interner.get(ty),
        Ty::TaintedString | Ty::TaintedBytes | Ty::SecretTaintedString | Ty::SecretTaintedBytes
    )
}

/// Whether `ty` carries `tainted` **anywhere it could be carried** — the atom
/// itself, an array's element, or any member of a union.
///
/// [`is_tainted`] asks about one atom, which is the right question for a
/// conversion, an operator and an assignment, because each of those already
/// walks a composite structurally. An *argument* is the one position where it
/// is the wrong question: `rule:security/unclassified-parameter-refuses-tainted`'s admission hands a whole argument to
/// [`untainted`], so the contagion that admission implies has to be read with
/// the same reach [`untainted`] and [`tainted_result`] have. A
/// [`Qual::Contagious`] parameter handed a `string|tainted string` — which is
/// what `$name ?? "default"` over a `?tainted string` is, and so what every
/// environment read arrives as — would otherwise be admitted on one rule and
/// found untainted on the other, and the call would launder for free.
///
/// It reaches one shape further than [`untainted`] does, through `array<…>` as
/// well, and the asymmetry is the point: this answer decides whether to *set*
/// the bit on a result, where reaching too far only over-taints, while
/// [`untainted`]'s decides whether to *admit*, where reaching too far is a
/// leak. So the safe direction is the wide one here and the narrow one there.
pub(crate) fn carries_tainted(ty: TypeId, interner: &TypeInterner) -> bool {
    if is_tainted(ty, interner) {
        return true;
    }
    match interner.get(ty) {
        Ty::Array(elem) => carries_tainted(*elem, interner),
        Ty::Union(members) => members
            .iter()
            .any(|&member| carries_tainted(member, interner)),
        _ => false,
    }
}

/// Whether `ty` carries `rule:security/secret-qualifier`'s `secret` qualifier — on its own or
/// composed with `tainted`. The `secret`-axis counterpart of [`is_tainted`];
/// the two are independent bits, so a caller checking one never implies
/// anything about the other. `pub` for [`is_tainted`]'s reason.
#[must_use]
pub fn is_secret(ty: TypeId, interner: &TypeInterner) -> bool {
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
/// `rule:security/unclassified-parameter-refuses-tainted`'s admission is a **narrowing** of one qualifier at one kind of
/// position, and it is spelled as clearing the bit on the *argument* before
/// [`is_assignable`] sees it rather than as widening the parameter's declared
/// type. That is what keeps the diagnostic honest: the parameter is what a
/// mismatch names, and `expected tainted string` is a type no member declares
/// and no reader would recognise.
///
/// It reaches through a **union**, arm by arm, because that is the shape a
/// laundered value actually arrives in: `Core\Env::get` answers
/// `?tainted string`, so `$name ?? "default"` is a `string|tainted string`, and
/// a launderer that refused it would be refusing the only spelling an
/// environment read has. The admission does not weaken by reaching — every arm
/// is narrowed and then compared, so a union is admitted only where each of its
/// arms is — and [`carries_tainted`] reads the contagion back out.
///
/// It deliberately does **not** reach through `array<…>`, where
/// [`tainted_result`] does. The reach is not symmetric because the two
/// directions are not: setting the bit further than necessary refuses, and
/// clearing it further than necessary admits. No array can reach here anyway —
/// [`crate::core_lib`]'s `qual_of` gives an `array<text>` parameter no
/// classification at all, so the array's entries are the over-strictness that
/// function's own comment records rather than something this one may spend.
/// If a row ever classifies an array parameter, this is the second place to
/// change and the first is that limit.
///
/// [`unsecret`] does not follow even into a union: `rule:core-classes/secret-reveal`'s escape hatch
/// is four parameters of two classes with no contagion to carry, so the same
/// reach would buy a `secret` admission nothing and cost the axis its posture —
/// being over-strict there is a refusal, not a leak.
pub(crate) fn untainted(ty: TypeId, interner: &mut TypeInterner) -> TypeId {
    if let Some(is_bytes) = qualifiable_base(ty, interner) {
        let secret = is_secret(ty, interner);
        return qualified_scalar(is_bytes, false, secret, interner);
    }
    match interner.get(ty).clone() {
        Ty::Union(members) => {
            let members: Vec<TypeId> = members
                .iter()
                .map(|&member| untainted(member, interner))
                .collect();
            interner.make_union(members)
        }
        _ => ty,
    }
}

/// The same type with the `secret` bit cleared, and everything else — the
/// `tainted` axis, the base — left exactly as it was. [`untainted`]'s twin one
/// axis over, spelled the same way and for the same reason.
///
/// The one caller is [`Qual::Reveal`]'s argument admission: `rule:core-classes/secret-reveal`'s
/// escape hatch is a *narrowing* at four parameters of two classes, so it
/// clears the bit on the argument before [`is_assignable`] sees it rather than
/// widening what `Core\Secret::reveal` declares. Nothing else in the checker
/// removes `secret` — a checked conversion goes through
/// [`apply_qualifier_conversion_rule`], which decides both axes at once.
pub(crate) fn unsecret(ty: TypeId, interner: &mut TypeInterner) -> TypeId {
    match qualifiable_base(ty, interner) {
        Some(is_bytes) if is_secret(ty, interner) => {
            let tainted = is_tainted(ty, interner);
            qualified_scalar(is_bytes, tainted, false, interner)
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

/// `rule:security/unclassified-parameter-refuses-tainted`'s admission as one question: does a parameter the registry
/// classified `qual` accept an argument carrying `tainted`?
///
/// * A [`Qual::Sink`] never does — that is the whole of `rule:security/sink-predicate` — and
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
///   and AGENTS.md's ordering does not trade security for a call that
///   compiles. A member in that position is either mis-classified — an answer
///   carrying no byte of any argument is `Neutral` — or answers a shape or an
///   object, whose fields are a slice of their own.
///
/// * [`Qual::Reveal`] does, for [`Qual::Contagious`]'s reason and not for
///   [`Qual::Launder`]'s: the mark removes `secret`, which says nothing about
///   `tainted`, so a `tainted secret string` handed to `Core\Secret::reveal`
///   answers a `tainted string` and the bit has somewhere to go.
///
/// Only the `tainted` axis is decided here. `secret` is refused at every mark
/// but one: whether a `Neutral` parameter launders `secret` is a laundering
/// decision `rule:security/sink-predicate` owes an answer to, and being over-strict costs a refusal
/// rather than a leak. [`Qual::Reveal`] is the one mark that answers
/// differently — `rule:core-classes/secret-reveal`'s named escape hatch, written by
/// `nvs_stdlib::secret`'s rows and `nvs_stdlib::password`'s two and by no
/// others — and its `secret` admission is
/// [`admits_secret_argument`]'s, so that one question is asked in one place
/// rather than folded in here.
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

/// Whether a parameter classified `qual` accepts a `secret` argument — ADR
/// 0033 § 3, which is one mark and, in the registry that writes it, two
/// classes wide: `Core\Secret`, which is the escape hatch, and `Core\Password`,
/// whose `hash` and `verify` are the one *operation* on a password that
/// legitimately answers something that is not one. That roster is closed and
/// checked, in `crate::core_lib`'s
/// `reveal_and_the_password_helpers_are_the_only_launderers_of_secret`.
///
/// [`admits_tainted_argument`]'s counterpart, and deliberately not a clause
/// inside it: the two axes are independent bits and a caller asking about one
/// must not be answered about the other. Every other mark refuses, which is
/// what makes a reveal greppable — a `secret` value reaches an ordinary `Core`
/// member only through a call that says so by name.
///
/// The answer's own qualifier is not this function's question. The parameter
/// is declared unqualified, so `secret` drops by not being in the return type;
/// `tainted` survives because [`Qual::Reveal`] carries contagion exactly as
/// [`Qual::Contagious`] does (`crate::expr::args`'s `carries_contagion`).
pub(crate) fn admits_secret_argument(qual: Option<Qual>) -> bool {
    matches!(qual, Some(Qual::Reveal))
}

/// `rule:security/taint-propagation` / `rule:security/secret-propagation`: what an `as` conversion's result carries on
/// the `tainted`/`secret` axes. A checked conversion that already throws on
/// a malformed shape — `uint`, `int`, `float`, `bool`, an enum's backing
/// type — removes both qualifiers on success for free, since none of those
/// targets carry either to begin with (`rule:security/secret-propagation`'s known, accepted gap:
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

/// `rule:security/secret-sinks-refuse`: a `secret`-qualified value converted `as Core\Html\Markup`
/// is refused with a diagnostic naming the qualifier specifically, ahead of
/// [`reject_computed_markup_conversion`]'s generic "must be written in the
/// code" one — escaping (this ADR's whole reason for diverging from `tainted`'s
/// auto-escape default) neutralizes injection risk, not exposure, so it is
/// the wrong tool here regardless of how the value was produced. In
/// practice a `secret` value is never a literal token to begin with (nothing
/// grants `secret` ambiently — `rule:security/secret-qualifier` — so it only ever reaches this
/// point through a declared binding), meaning the generic literal check
/// alone would already refuse it; this exists to give that refusal its own,
/// more specific reason. Scoped to a conversion whose target actually
/// resolves to `Core\Html\Markup`, same as its sibling; the inline
/// `<?= expr ?>`/templating-helper interpolation position `rule:security/secret-sinks-refuse` also
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
    if qname.to_string() != crate::CORE_HTML_MARKUP_CLASS {
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

/// `rule:core-classes/html-auto-escape`: `as Core\Html\Markup` trusts only a string literal —
/// a `tainted` value, or any other runtime-computed one, can never become
/// trusted markup this way, closing "compute the escape-defeating payload at
/// runtime, then cast it." Scoped to a conversion whose target actually
/// resolves to `Core\Html\Markup`; every other target is untouched. The rest
/// of § 5 (auto-escaping a non-`Markup` interpolation, `Markup + Markup`)
/// waits on `Core\Html` actually existing as a stdlib class.
pub(crate) fn reject_computed_markup_conversion(
    inner: &Expr,
    to: TypeId,
    span: Span,
    env: &mut Env<'_>,
) {
    let Ty::Class(qname, _) = env.interner.get(to) else {
        return;
    };
    if qname.to_string() != crate::CORE_HTML_MARKUP_CLASS {
        return;
    }
    if is_literal_string(inner) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_MARKUP_NEEDS_WRITTEN_STRING,
            "only a string written directly in the code can be converted `as Markup`. A \
             computed or `tainted` value cannot become trusted markup this way",
        )
        .with_primary(span, "converted here")
        .with_help("build the markup from strings written in the code, or escape the value with a `Core\\Html` method"),
    );
}

/// Whether `expr` is a literal string token, unwrapping any enclosing
/// parentheses — `("text")` is exactly as trusted as `"text"` for
/// [`reject_computed_markup_conversion`]'s purposes.
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

/// `rule:security/secret-sinks-refuse`: a `Throwable`-shaped class's constructor message argument
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

/// `rule:security/secret-sinks-refuse`'s debug-dump sink at its *call-site* half, which is how
/// `rule:errors/record-transformations`'s redaction row states it: a property whose declared type carries
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
/// extension of the item that added this.** `rule:errors/record-transformations`'s closing paragraph
/// makes the renderings non-bypassable — there is no `dumpRaw` and no
/// rendering selected by an argument — so a member that answers the record's
/// text as a `Core\Cli\Text` carrier is the same disclosure one `echo` later,
/// and § 4's terminal bullet refuses that value with **no `Core\Cli\Text`
/// bypass** in any case. Refusing only `dump` would leave
/// `echo Core\Debug::render($secret)` as the way round both bullets.
///
/// Scoped to the arguments this call actually writes, in written order, so a
/// `dump($a, $secret, $b)` names the one it is about, and asked with
/// [`contains_secret`]: a dump is written *through* the composites it is
/// handed, so an `array<secret string>` is as disclosing as the element, and
/// the argument that carries one is the same shape the serialiser sink
/// refuses. What it does not reach is a `secret` value held in a *property*,
/// which reaches the walk rather than the site and is the Redacted node this
/// row's other half owns — and a `...$xs` spread, which hands over a subject
/// whose *element* type carries the qualifier.
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
        if !contains_secret(ty, env.interner) {
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

/// Whether `ty` carries `rule:security/secret-qualifier`'s `secret` anywhere a serialiser would
/// walk to: on the type itself, or on an element, a field or a member of the
/// composites a written value takes. [`is_secret`] answers the atom; this
/// answers the whole value, which is what `rule:security/secret-sinks-refuse`'s serialiser bullet
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
        Ty::Shape(fields) => fields.iter().any(|f| contains_secret(f.ty, interner)),
        Ty::CoreShape(shape) => shape
            .fields
            .iter()
            .any(|field| contains_secret(field.ty, interner)),
        Ty::Union(members) | Ty::Intersection(members) => {
            members.iter().any(|&m| contains_secret(m, interner))
        }
        _ => false,
    }
}

/// `rule:security/secret-qualifier`'s container axis: a `secret` value may be
/// written into an array element or a shape field only where the position's own
/// type carries the qualifier.
///
/// Not one of the sinks above, and the difference is the whole of why it exists
/// — a sink refuses a value about to be *disclosed*, and this refuses one about
/// to be *stored somewhere the qualifier stops being visible*. The two
/// containers lose it at different moments and the same test answers both.
/// [`check_array_literal`](super::literals::check_array_literal) joins no
/// element types, so `[$secret]` placed at an `array<mixed>` drops the bit at
/// the bracket; [`check_anon_object`](super::literals::check_anon_object)
/// infers a field type and *keeps* it, so `{token: $secret}` drops it one step
/// later, where the literal meets a field declared wider. Asking the position
/// what it carries covers both, and accepts the spellings that keep it:
/// `array<secret string>` and `{token: secret string}` are ordinary types the
/// grammar already writes, and a value read back out of either is still
/// `secret`.
///
/// **An argument list steps aside**, through [`Env::in_call_argument`].
/// `rule:security/secret-sinks-refuse` names three positions a credential
/// legitimately reaches — a bound database parameter, a process argv, an
/// outbound request — and every one of them is written as an `array<mixed>`
/// argument, so refusing there would make the qualifier unusable for its own
/// purpose. What an argument owes is that rule's sinks, above.
///
/// A `placed` of `None` is a position with no declared type at all, and only
/// an array element reaches it: a field with no declared shape above it is not
/// asked, its literal's own inferred type being where the qualifier goes next.
///
/// The `placed` type is asked with [`contains_secret`] rather than
/// [`is_secret`], so an `array<array<secret string>>` keeps its inner element
/// and a union spelling one arm `secret` is a position that carries it. A
/// position that already reported its own mismatch is not asked at all — the
/// caller guards on the diagnostic count, the way
/// [`check_spread_element`](super::literals::check_spread_element) does, so
/// `array<string> $a = [$secret];` stays the one `E0401` it has always been.
pub(crate) fn reject_secret_into_container(
    value: &Expr,
    value_ty: TypeId,
    placed: Option<TypeId>,
    position: &str,
    env: &mut Env<'_>,
) {
    if env.in_call_argument || !contains_secret(value_ty, env.interner) {
        return;
    }
    if placed.is_some_and(|ty| contains_secret(ty, env.interner)) {
        return;
    }
    let holds = match placed {
        Some(ty) => format!("`{}` keeps no qualifier", env.interner.describe(ty)),
        None => "no element type is declared here, so this holds `mixed`".to_owned(),
    };
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_INTO_CONTAINER,
            format!("a `secret` value cannot be stored in {position}: {holds}"),
        )
        .with_primary(value.span, "this value is `secret`")
        .with_help(
            "the container's own type is what carries the qualifier onward — declare it \
             `array<secret string>` for an element, or `{name: secret string}` for a field — or \
             hand the value over deliberately with `Core\\Secret::reveal(..., \"reason\")`. A \
             bound database parameter, a process argv and an outbound request each take a \
             `secret` as a written argument and need neither (`rule:security/secret-sinks-refuse`)",
        ),
    );
}

/// [`reject_secret_into_container`] at the other spelling of the same write:
/// `$a["k"] = $secret`, where the element type is the assignment's own target
/// type and there is no literal to read a position off.
///
/// Scoped to a subscript target, which is `rule:security/secret-qualifier`'s
/// container axis; a property is the *declared* end of
/// `rule:errors/record-transformations`'s redaction row and answers for itself.
/// Reached from the plain `=` and from a compound one alike, because `.=` over
/// a `secret` operand produces a `secret` result
/// (`rule:security/secret-propagation`) and a refusal one character wide is not
/// a refusal.
pub(crate) fn reject_secret_element_write(
    target: &Expr,
    target_ty: TypeId,
    value: &Expr,
    value_ty: TypeId,
    env: &mut Env<'_>,
) {
    if !matches!(target.unparenthesized().kind, ExprKind::Index { .. }) {
        return;
    }
    reject_secret_into_container(value, value_ty, Some(target_ty), "an array element", env);
}

/// `rule:security/secret-sinks-refuse`'s serialiser sink: `Core\Json::encode` refuses a `secret`
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
/// nothing), so the qualifier is gone before the call is looked at. The axis
/// answers it where the value is written instead
/// ([`reject_secret_into_container`]), which is why the literal cannot be
/// built one statement earlier and handed over through a variable — and why
/// this rule needs no second walk over an expression it reads a type off.
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

/// The same serialiser sink one member further on: `Core\Queue::push`'s
/// `args:` payload is encoded into a durable row that a *worker process*
/// decodes later, by the encoder [`reject_secret_encoded_argument`] refuses at
/// — `nvs_stdlib::queue`'s `payload_of` hands `crate::json::Encodable` to
/// `serde_json` — so it reports that bullet's code rather than one of its own.
/// `rule:security/secret-sinks-refuse` names a queue as one of the three places
/// an encoded document is on its way to, and this is the spelling where the
/// encode is inside the member rather than at a call the program wrote.
///
/// A call-site rule for every sibling's reason: `args` is declared `mixed`,
/// which a `secret string` satisfies, so the written option is the last place
/// the qualifier is visible at all. It is also the one option asked about, on
/// [`is_fields_argument`]'s split — `queue` and `key` are `string`s, which a
/// `secret string` is not assignable to, and the other three are an instant, a
/// duration and a `uint`, so every one of them is already refused by its own
/// declared type and a second report here would answer one mistake twice.
///
/// **The qualifier is read off the written literal and the scope**, never off
/// `arg_types`, and that is forced rather than chosen: the bag arrives as the
/// [`Ty::CoreShape`](crate::ty::Ty::CoreShape) the *registry* declared, whose
/// `args` field is `mixed`, so nothing of the argument's own type survives on
/// the slot. The reach is [`reject_secret_logged_argument`]'s and so is the
/// gap it leaves — a payload that arrives *composed* is past this rule, which
/// is the container axis `rule:security/secret-qualifier` already owns rather
/// than a hole in this one.
pub(crate) fn reject_secret_enqueued_argument(
    qname: &QName,
    member: &str,
    args: &CallArgs,
    slots: &[ArgSlot],
    scope: &LocalScope,
    env: &mut Env<'_>,
) {
    if qname.to_string() != r"Core\Queue" || member != "push" {
        return;
    }
    let CallArgs::List(list) = args else {
        return;
    };
    for (arg, &slot) in list.iter().zip(slots) {
        // Slot 1 is the bag in `nvs_stdlib::queue`'s row, and a bag is written
        // as an anonymous object or not written at all — a [`Ty::Shape`] is
        // never assignable to a [`Ty::CoreShape`], which is
        // [`super::args`]'s own fork — so there is no second spelling of the
        // payload for this walk to be missing.
        if slot != ArgSlot::Param(1) {
            continue;
        }
        let ExprKind::AnonObject(fields) = &arg.value.kind else {
            continue;
        };
        for field in fields {
            if span_text(env.src, field.name)
                .trim()
                .trim_end_matches(':')
                .trim()
                != "args"
            {
                continue;
            }
            reject_secret_enqueued_value(&field.value, scope, env);
        }
    }
}

/// [`reject_secret_enqueued_argument`]'s walk over one written payload: a
/// binding that carries the qualifier, and the same question asked of every
/// element of an array literal or anonymous object written inline, which is where the
/// shape the rule exists for — one credential among public fields — is
/// actually written.
///
/// A binding is asked about by name for [`reject_secret_logged_argument`]'s
/// reason: an inline literal checked against a `mixed` expectation is that
/// expectation, so nothing of an element's own type reaches here.
fn reject_secret_enqueued_value(at: &Expr, scope: &LocalScope, env: &mut Env<'_>) {
    match &at.kind {
        ExprKind::Variable(span) => {
            let name = strip_sigil(span_text(env.src, *span));
            let Some(bound) = scope.narrowed_ty(name).or_else(|| scope.declared_ty(name)) else {
                return;
            };
            if contains_secret(bound, env.interner) {
                report_secret_enqueued(at.span, env);
            }
        }
        ExprKind::ArrayLiteral(items) => {
            for item in items {
                reject_secret_enqueued_value(&item.value, scope, env);
            }
        }
        ExprKind::AnonObject(fields) => {
            for field in fields {
                reject_secret_enqueued_value(&field.value, scope, env);
            }
        }
        _ => {}
    }
}

/// [`reject_secret_enqueued_argument`]'s one diagnostic, written once because
/// the rule reaches it from two directions — a payload that carries the
/// qualifier, and one element of a payload object that does — and the author
/// is owed the same sentence either way.
fn report_secret_enqueued(span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_ENCODED,
            "a `secret`-qualified value cannot be passed to `Core\\Queue::push`; the `args` \
             payload is encoded into a durable row that a worker process decodes later, and a \
             stored job is not the credential being used",
        )
        .with_primary(span, "secret value enqueued here")
        .with_help(
            "reveal the one field that must travel with \
             `Core\\Secret::reveal(..., \"reason\")`, written at that field rather than at the \
             call, so the rest of the payload stays covered",
        ),
    );
}

/// `rule:security/secret-sinks-refuse`'s log sink: `Core\Log::write` refuses a `secret` in its
/// `fields` bag — the opposite default from `tainted`, which `rule:security/sink-predicate`
/// explicitly wants recorded.
///
/// A call-site rule for [`reject_secret_encoded_argument`]'s reason and one
/// more that only this sink has: `fields` is declared `array<mixed>` **by
/// design**, and § 4 says so in the bullet itself, so the parameter type is
/// not a thing to be fixed later — the written argument is where the qualifier
/// is, and nowhere below is. [`is_fields_argument`] owns why the other two
/// parameters are left to their own declared types.
///
/// **The element half is read off the written literal**, which is what § 4's
/// "including inside a `fields` array literal" asks for and what no type could
/// answer: a literal checked against an `array<mixed>` expectation *is* that
/// expectation ([`check_array_literal`](super::literals::check_array_literal)
/// hands the expected id straight back), so `["token" => $t]` arrives here as
/// an `array<mixed>` with nothing left of `$t` on it. So each element that
/// names a binding is asked about by name instead, against the same scope the
/// read itself used. An element that *composes* one — `"Bearer " . $t` — is
/// past this rule, and that is the container axis `rule:security/secret-qualifier` already owns rather
/// than a hole in this one: the fix there is an element type for a literal,
/// and it would make this half fall out of [`contains_secret`] like the rest.
pub(crate) fn reject_secret_logged_argument(
    qname: &QName,
    member: &str,
    args: &CallArgs,
    arg_types: &[TypeId],
    scope: &LocalScope,
    env: &mut Env<'_>,
) {
    if qname.to_string() != r"Core\Log" || member != "write" {
        return;
    }
    let CallArgs::List(list) = args else {
        return;
    };
    for (index, (arg, &ty)) in list.iter().zip(arg_types).enumerate() {
        if !is_fields_argument(index, arg, env) {
            continue;
        }
        if contains_secret(ty, env.interner) {
            report_secret_logged(arg.value.span, env);
            continue;
        }
        let ExprKind::ArrayLiteral(items) = &arg.value.kind else {
            continue;
        };
        for item in items {
            let ExprKind::Variable(span) = &item.value.kind else {
                continue;
            };
            let name = strip_sigil(span_text(env.src, *span));
            let Some(bound) = scope.narrowed_ty(name).or_else(|| scope.declared_ty(name)) else {
                continue;
            };
            if contains_secret(bound, env.interner) {
                report_secret_logged(item.value.span, env);
            }
        }
    }
}

/// Whether this written argument is `Core\Log::write`'s `fields` bag — its
/// third parameter, or the one an `rule:core-api/shape-rules` R2 named argument spells `fields:`.
///
/// The rule is scoped to it rather than to the whole call because the *other*
/// two positions are already refused by their declared types: `message` is a
/// plain `string`, which a `secret string` is not assignable to, and `level`
/// is an enum. Reporting there as well would be two diagnostics for one
/// mistake, and the second would not say anything the first did not. That
/// split — an open type checked at the site, a closed one checked by the
/// signature — is `rule:security/secret-sinks-refuse`'s own, stated in the bullet this rule is.
fn is_fields_argument(index: usize, arg: &Arg, env: &Env<'_>) -> bool {
    match arg.name {
        Some(span) => span_text(env.src, span).trim_end_matches(':').trim() == "fields",
        None => index == 2,
    }
}

/// [`reject_secret_logged_argument`]'s one diagnostic, written once because
/// the rule reaches it from two directions — a whole argument that carries the
/// qualifier, and one element of a `fields` literal that does — and the author
/// is owed the same sentence either way.
fn report_secret_logged(span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_LOGGED,
            "a `secret`-qualified value cannot be written to a log record; a log is read by a \
             person and kept by a pipeline, so the value would be disclosed rather than used",
        )
        .with_primary(span, "secret value logged here")
        .with_help(
            "record something that identifies the credential instead — its key name, its \
             fingerprint — or, where the value itself is genuinely the record, reveal it with \
             `Core\\Secret::reveal(..., \"reason\")` at that field",
        ),
    );
}

/// `rule:security/secret-sinks-refuse`'s terminal-output sink: `echo` and `print` refuse a
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
/// makes one check at the sink cover `rule:security/secret-sinks-refuse`'s *"`echo` and
/// interpolation"* both, rather than needing a rule per composition form.
///
/// The reach is wider than the word "terminal": `rule:tooling/echo-always-has-a-sink` sends a
/// scheduled script's, a job worker's, a `#[Test]` method's and a
/// `spawn script` isolate's output through this same sink, so what this
/// refuses is as often a credential landing in a CI log as one printed to a
/// tty. A tty-dependent version of the rule is not available — `rule:tooling/terminal-output-is-a-sink`
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

/// `rule:security/secret-sinks-refuse`'s cross-boundary sink: a `secret`-qualified value handed to
/// `rule:classes/graph-copy`'s graph copy.
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
            None,
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
///
/// `instead` is the one carrier-specific clause the help gets, and it is
/// `Some` only where the language has a member that takes the secret and
/// carries it safely — `Core\Cache\Store::putSecret` is the first. A reveal is
/// always available and is what the rest of the help offers; where there is a
/// sealed door, naming it first is the difference between a rule and an
/// instruction.
pub(crate) fn reject_secret_crossing(
    at: &Expr,
    ty: TypeId,
    carrier: &str,
    instead: Option<&str>,
    env: &mut Env<'_>,
) {
    if !is_secret(ty, env.interner) {
        return;
    }
    let reveal = "reveal it explicitly first with `Core\\Secret::reveal(..., \"reason\")`, at \
                  the one call site where handing the secret over is the point; a \
                  `secret`-typed *property* of a copied object needs nothing here — the walk \
                  refuses that one itself";
    env.diags.report(
        Diagnostic::error(
            code::E_SECRET_CROSSES_A_BOUNDARY,
            format!(
                "a `secret`-qualified value cannot cross a copy boundary; {carrier}, \
                 where nothing carries the qualifier that was protecting it"
            ),
        )
        .with_primary(at.span, "secret value copied out here")
        .with_help(instead.map_or_else(
            || reveal.to_owned(),
            |instead| format!("{instead}; or {reveal}"),
        )),
    );
}

/// `rule:concurrency/cross-request-state-is-explicit`'s store, which is
/// [`reject_secret_crossing`]'s fourth carrier: `Core\Cache\Store::put` copies
/// its value out of the request heap through `rule:classes/graph-copy`'s graph
/// copy, into a tier that outlives the request that wrote it.
///
/// **The one carrier with a door of its own.** `putSecret` takes the same
/// secret, seals it under a key ring and puts ciphertext in the tier, so what
/// this refusal points at is that member rather than a reveal — the sealed door
/// is the only door, and a program that reveals here to get past the check has
/// put a plaintext secret in a store instead of being told where the sealed one
/// goes.
///
/// A call-site rule rather than a parameter type, exactly as every sibling here
/// is one: `put` declares `mixed` for its value, which a `secret string`
/// satisfies, so the written argument is the last place the qualifier is still
/// visible. The value is found through its [`ArgSlot`] rather than by position,
/// because `value:` fills the parameter as surely as the second positional
/// argument does — and `$key` is not asked about, being text that reaches no
/// answer at all.
pub(crate) fn reject_secret_cached_argument(
    qname: &QName,
    member: &str,
    args: &CallArgs,
    arg_types: &[TypeId],
    slots: &[ArgSlot],
    env: &mut Env<'_>,
) {
    if qname.to_string() != r"Core\Cache\Store" || member != "put" {
        return;
    }
    let CallArgs::List(list) = args else {
        return;
    };
    // Slot 1 is `$value` in `nvs_stdlib::cache`'s row, whose parameters are
    // `[CoreTy::Text(Qual::Neutral), CoreTy::Mixed, CoreTy::Options(…)]`.
    for ((arg, &ty), &slot) in list.iter().zip(arg_types).zip(slots) {
        if slot != ArgSlot::Param(1) {
            continue;
        }
        reject_secret_crossing(
            &arg.value,
            ty,
            "`Core\\Cache\\Store::put` copies it into a tier that outlives the request",
            Some(
                "write `putSecret($key, $value, $ttl, $keys)` instead, which seals the value \
                 under a key ring so that what reaches the tier is ciphertext",
            ),
            env,
        );
    }
}

/// `rule:http-server/a-session-holds-a-secret-only-sealed`'s record, which is
/// [`reject_secret_crossing`]'s fifth carrier: `Core\Session::set` encodes its
/// value into the record this request writes back to a store that outlives it.
///
/// **The second carrier with a door of its own**, and it names it exactly as
/// [`reject_secret_cached_argument`] names `putSecret`: `setSecret` takes the
/// same secret, seals it under a key ring and writes ciphertext into the
/// record, so a program that revealed here to get past the check would have put
/// a plaintext credential in a session store rather than being told where the
/// sealed one goes.
///
/// A call-site rule rather than a parameter type, exactly as every sibling here
/// is one: `set` declares `mixed` for its value, so the written argument is the
/// last place the qualifier is visible. The value is found through its
/// [`ArgSlot`] for [`reject_secret_published_argument`]'s reason, and `$key` is
/// not asked about, being text that reaches no answer at all.
pub(crate) fn reject_secret_session_argument(
    qname: &QName,
    member: &str,
    args: &CallArgs,
    arg_types: &[TypeId],
    slots: &[ArgSlot],
    env: &mut Env<'_>,
) {
    if qname.to_string() != r"Core\Session" || member != "set" {
        return;
    }
    let CallArgs::List(list) = args else {
        return;
    };
    // Slot 1 is `$value` in `nvs_stdlib::session`'s row, whose parameters are
    // `[CoreTy::Text(Qual::Neutral), CoreTy::Mixed]`.
    for ((arg, &ty), &slot) in list.iter().zip(arg_types).zip(slots) {
        if slot != ArgSlot::Param(1) {
            continue;
        }
        reject_secret_crossing(
            &arg.value,
            ty,
            "`Core\\Session::set` encodes it into the record this request writes back to a \
             store that outlives it",
            Some(
                "write `setSecret($key, $value, $keys)` instead, which seals the value under a \
                 key ring so that what reaches the record is ciphertext",
            ),
            env,
        );
    }
}

/// `rule:core-classes/topic`'s bus, which is [`reject_secret_crossing`]'s third carrier:
/// `Core\Topic::publish` copies its value into every subscriber's own arena
/// through `rule:classes/graph-copy`'s graph copy, so "a `secret` may never be published" is
/// the disclosure `Core\Serialize::encode` and `spawn script`'s `args:` are
/// already refused for, and it reports the same code rather than one of its
/// own.
///
/// A call-site rule rather than a parameter type, exactly as every sibling
/// here is one: `publish` declares `mixed` for its value, which a
/// `secret string` satisfies, so the written argument is the last place the
/// qualifier is still visible.
///
/// **The topic is not asked about**, and that is not an omission: it is a
/// [`Qual::Sink`] parameter, so a qualified name is already an ordinary
/// mismatch there — § 4's own `tainted` refusal — and asking a second time
/// would report one error as two.
///
/// The value is found through its [`ArgSlot`] rather than by position, for
/// [`super::isolate::check_core_isolate_call`]'s reason: `value:` fills the
/// parameter as surely as the second positional argument does.
pub(crate) fn reject_secret_published_argument(
    qname: &QName,
    member: &str,
    args: &CallArgs,
    arg_types: &[TypeId],
    slots: &[ArgSlot],
    env: &mut Env<'_>,
) {
    if qname.to_string() != r"Core\Topic" || member != "publish" {
        return;
    }
    let CallArgs::List(list) = args else {
        return;
    };
    // Slot 1 is `$value` in `nvs_stdlib::topic`'s row, whose parameters are
    // `[CoreTy::Text(Qual::Sink), CoreTy::Mixed]`.
    for ((arg, &ty), &slot) in list.iter().zip(arg_types).zip(slots) {
        if slot != ArgSlot::Param(1) {
            continue;
        }
        reject_secret_crossing(
            &arg.value,
            ty,
            "`Core\\Topic::publish` copies it into every subscriber's own arena, on this core \
             and on every other",
            None,
            env,
        );
    }
}

/// `rule:security/secret-sinks-refuse`'s fifth sink: a `secret` class constant reaching an attribute
/// payload, reported at the value where it is written.
///
/// The sink exists because of `rule:attributes/payload-is-a-compile-time-constant` rather than in spite of it. A payload admits only compile-time
/// constants — no variable, no call, no `new` — and a class constant is one of
/// the shapes it admits, so the storage class `rule:security/secret-qualifier`'s own values live in is
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
pub(crate) fn reject_secret_attribute_constant(
    expr: &Expr,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> bool {
    let ExprKind::ClassConstAccess { class, name } = &expr.kind else {
        return false;
    };
    let Some(qname) = super::members::resolve_class_expr(class, ctx, env) else {
        return false;
    };
    let constant = span_text(env.src, *name).to_owned();
    if !env.consts.is_secret(&qname, &constant, env.graph) {
        return false;
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
            "`rule:security/secret-sinks-refuse`: there is no `Core\\Secret::reveal(..., \"reason\")` way out of this \
             one, a payload admitting no call at all — a credential belongs somewhere read at \
             run time, and the attribute carries the name of where to read it from",
        ),
    );
    true
}
