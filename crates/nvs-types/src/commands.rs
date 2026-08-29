//! [ADR 0086](../../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 6's
//! two command-table attributes: what `#[Command]` and `#[Option]` may carry.
//!
//! # Why these are recognized names rather than shape aliases
//!
//! § 6 builds a *table* from them while compiling — the same
//! [ADR 0061](../../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
//! § 3 scan ADR 0077's route table is built by — so a userland
//! `type Command = {name: string};` must not contribute a command however it is
//! spelled. That is exactly
//! [ADR 0071](../../../../docs/adr/0071-derived-codecs.md) § 1's rule, so both
//! names sit on [`crate::derive::ATTRIBUTES`] and are matched *nominally* after
//! [`nvs_hir::resolve_ref`]. [`crate::attributes`]'s ADR 0046 § 1 rule — the
//! name is a shape-typed `type` alias — is the rule for the userland names,
//! which are the only ones that could ever be aliases.
//!
//! This module declares what is checked behind those two roster entries: a
//! name on that closed list with nothing checking its payload would admit
//! `#[Command(nmae: "deploy")]` silently, which is the whole argument
//! [`crate::derive`]'s own gap 3 makes for keeping the list short.
//!
//! # What is checked here, and what the table still owes
//!
//! Two passes, and they are two because they read different things.
//!
//! **One payload at a time**, from [`crate::attributes`]'s per-attribute walk:
//! the field names against [`COMMAND_OPTIONS`] / [`OPTION_OPTIONS`], each
//! value's type, and a field given twice. The walk itself is
//! [`crate::attributes::check_roster`], shared with every other recognized name
//! that carries a payload; what this module owns is the two rosters.
//!
//! **One method at a time**, from [`crate::check`]'s per-class walk:
//! [`check_class_commands`] holds each `#[Command]` to the two of § 6's three
//! compile errors that are decidable from one parameter list — two options
//! sharing a spelling, and an `#[Option]` on a parameter no argument text could
//! be converted into. Neither needs the table, and waiting for it would leave
//! an author's typo unreported until the pass that assembles rows exists.
//!
//! **Every method, from [`crate::attributes`]' own walk:**
//! [`check_stray_options`], which is the one question the per-class walk cannot
//! ask, because that walk sees only the methods a `#[Command]` selects.
//!
//! # Known gaps
//!
//! 1. **The table itself is not built**, so § 6's remaining compile error — a
//!    duplicate command name — is not reported: it is a question about the
//!    whole program's enumeration rather than about one declaration, and it
//!    wants the walk that assembles rows.
//! 2. **Whether `name` is required on `#[Command]` is not decided here.** § 6
//!    writes every example with one and says nothing about leaving it out, and
//!    ADR 0077 § 1's "optional and never derived" is a rule about *routes*. The
//!    table pass is what needs a name to build a row, so it is what should
//!    refuse a row that has none.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{Attribute, ClassDecl, ClassMemberKind, MethodMember, Param};

use crate::defaults::ConstArg;
use crate::testing::OptionTy;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// `#[Command(name: string, about: string)]` — ADR 0086 § 6's own spelling, in
/// the order that section writes it.
pub(crate) const COMMAND_OPTIONS: &[(&str, OptionTy)] =
    &[("name", OptionTy::Str), ("about", OptionTy::Str)];

/// `#[Option(short: string, long: string, about: string)]` — § 6 writes
/// `short` and `about`; `long` is the spelling an option is matched by when the
/// parameter's own name is not it, and it is named here for the same reason
/// `short` is: nothing about a command line is inferred.
pub(crate) const OPTION_OPTIONS: &[(&str, OptionTy)] = &[
    ("short", OptionTy::Str),
    ("long", OptionTy::Str),
    ("about", OptionTy::Str),
];

/// Every `#[Command]` method `decl` declares, held to the two of § 6's three
/// compile errors that one parameter list answers.
///
/// A no-op — not even a lookup — for a class carrying no `#[Command]`, which is
/// what keeps § 6's "a program with no `#[Command]` pays nothing" true of this
/// pass as well as of the table.
///
/// Run from [`crate::check`]'s per-class walk rather than from
/// [`crate::attributes`]', because both questions are about the *method*: the
/// per-attribute walk holds one payload and can see neither the sibling option
/// it collides with nor the parameter it is attached to.
pub(crate) fn check_class_commands(
    decl: &ClassDecl,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        if crate::testing::attribute_named(&m.attributes, crate::derive::COMMAND, ctx, env)
            .is_none()
        {
            continue;
        }
        check_options(m, class, ctx, env);
    }
}

/// ADR 0086 § 6's marker held to the declaration that reads it: an `#[Option]`
/// on a parameter of a method carrying no `#[Command]`.
///
/// Asked from [`crate::attributes`]' per-method walk rather than from
/// [`check_class_commands`], because that walk cannot see this mistake at all:
/// it selects the methods a `#[Command]` marks, and a stray `#[Option]` is by
/// definition on one of the others. This does not wait for the table gap 1 still
/// owes — what a stray marker is stray of is the sibling attribute on its own
/// method, which one declaration answers — and it is
/// [`crate::routes::check_stray_query`]'s question asked of the other pass's
/// marker, the two written apart so each sits beside the attribute that gives
/// its marker a meaning.
///
/// Refused rather than ignored for the reason [`crate::derive::ATTRIBUTES`] is a
/// closed roster: a name the compiler knows, written where the compiler never
/// looks, reads to its author as a declaration that binds an argument.
pub(crate) fn check_stray_options(m: &MethodMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    if crate::testing::attribute_named(&m.attributes, crate::derive::COMMAND, ctx, env).is_some() {
        return;
    }
    for param in &m.params {
        let Some(attr) =
            crate::testing::attribute_named(&param.attributes, crate::derive::OPTION, ctx, env)
        else {
            continue;
        };
        let name = strip_sigil(span_text(env.src, param.name)).to_owned();
        env.diags.report(
            Diagnostic::error(
                code::E_OPTION_WITHOUT_COMMAND,
                format!("`#[Option] ${name}` is on a method that declares no command"),
            )
            .with_primary(attr.span, "nothing reads this marker")
            .with_help(
                "ADR 0086 § 6 gives `#[Option]` its meaning on a `#[Command]` method's parameter, \
                 where it is the spelling an argument arrives by — anywhere else nothing supplies \
                 it: write the `#[Command]` this parameter serves, or delete the marker",
            ),
        );
    }
}

/// One `#[Command]` method's parameter list: each `#[Option]`'s declared type,
/// and the spellings they claim between them.
fn check_options(m: &MethodMember, class: &QName, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    // Copied out of `env` rather than read through it, so the table stays
    // readable while a diagnostic is being reported into the same `env`.
    let signatures = env.signatures;
    let method = span_text(env.src, m.name).to_owned();
    let sig = signatures
        .get(class)
        .and_then(|class_sig| class_sig.methods.get(&method));
    // The spellings claimed so far, each with the span that claimed it. One
    // list rather than two, because `-n` and `--n` are rendered with the
    // dashes a command line writes and so cannot collide across forms.
    let mut taken: Vec<(String, Span)> = Vec::new();
    for (index, param) in m.params.iter().enumerate() {
        let Some(attr) =
            crate::testing::attribute_named(&param.attributes, crate::derive::OPTION, ctx, env)
        else {
            continue;
        };
        let attr = attr.clone();
        if let Some(ty) = sig.and_then(|sig| sig.params.get(index).copied()) {
            check_convertible(param, &method, ty, env);
        }
        for (spelling, span) in spellings(&attr, param, env) {
            if let Some((_, prior)) = taken.iter().find(|(already, _)| *already == spelling) {
                let prior = *prior;
                env.diags.report(
                    Diagnostic::error(
                        code::E_OPTION_SPELLING_TAKEN,
                        format!("two options of `{method}` are spelled `{spelling}`"),
                    )
                    .with_primary(span, "this spelling is already claimed")
                    .with_secondary(prior, "claimed here")
                    .with_help(
                        "a command line matches an option by its spelling, so two parameters \
                         answering to one of them have no answer: give this one its own `short:` \
                         or `long:`",
                    ),
                );
            } else {
                taken.push((spelling, span));
            }
        }
    }
}

/// The spellings one `#[Option]` claims: its `short:` if it wrote one, and its
/// long form, which is the parameter's own name unless `long:` gives another.
///
/// Rendered with the `-`/`--` a command line writes them with, which is what
/// makes one list of both forms correct: `-n` and `--n` are two spellings, and
/// a bare `n` compared against a bare `n` would call them one.
fn spellings(attr: &Attribute, param: &Param, env: &mut Env<'_>) -> Vec<(String, Span)> {
    let mut claimed = Vec::with_capacity(2);
    if let Some((short, span)) = folded_option(attr, "short", env) {
        claimed.push((format!("-{short}"), span));
    }
    match folded_option(attr, "long", env) {
        Some((long, span)) => claimed.push((format!("--{long}"), span)),
        // The parameter's own name is the default long spelling, so an option
        // that writes nothing still claims one — the collision this catches is
        // an explicit `long:` written over a sibling's parameter name, which
        // reading either declaration alone would never show.
        None => claimed.push((
            format!("--{}", strip_sigil(span_text(env.src, param.name))),
            param.name,
        )),
    }
    claimed
}

/// One option's written value, folded — `None` where the field was not written
/// or where its value was not the `string` the roster declares, both of which
/// [`crate::attributes::check_roster`] has already reported.
fn folded_option(attr: &Attribute, option: &str, env: &mut Env<'_>) -> Option<(String, Span)> {
    let field = attr
        .fields
        .iter()
        .find(|field| span_text(env.src, field.name) == option)?;
    let declared = env.interner.intern(Ty::String);
    let span = field.span;
    match crate::defaults::literal_default(&field.value, declared, env) {
        Some(ConstArg::Str(value)) => Some((value, span)),
        _ => None,
    }
}

/// § 6's third compile error: the parameter an `#[Option]` is attached to must
/// have a type an argument's text can be converted to.
fn check_convertible(param: &Param, method: &str, ty: TypeId, env: &mut Env<'_>) {
    if converts_from_string(ty, env) {
        return;
    }
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_OPTION_TYPE_HAS_NO_CONVERSION,
            format!("`{described}` is not a type an option of `{method}` can be given at"),
        )
        .with_primary(param.span, "no conversion from an argument's text")
        .with_help(
            "an argument arrives as text and its type comes from the parameter, so an option \
             declares `string`, `int`, `uint`, `decimal`, `bool`, an enum, a union of literal \
             types, or `Core\\Uuid`",
        ),
    );
}

/// ADR 0077 § 3's conversion roster, which § 6 takes unchanged, plus the one
/// row § 6 adds: a `bool` `#[Option]` is a flag, so it is given by being
/// written rather than by carrying text.
///
/// The one home for "what an argument's text may become", so the route table's
/// own pass reads this rather than growing a second list that agrees with it
/// today.
///
/// A `float` is deliberately absent where `decimal` is not: § 3 names one and
/// not the other, and an argument that quietly rounds is the class of bug
/// ADR 0054 exists to make unwritable.
pub(crate) fn converts_from_string(ty: TypeId, env: &Env<'_>) -> bool {
    match env.interner.get(ty) {
        Ty::String
        | Ty::TaintedString
        | Ty::Int
        | Ty::Uint
        | Ty::Decimal
        | Ty::Bool
        | Ty::True
        | Ty::False
        | Ty::StringLiteral(_)
        | Ty::IntLiteral(_)
        | Ty::Enum(..)
        | Ty::EnumCase(..) => true,
        // A parameter written with no type at all interns as `mixed` and has
        // already been reported for the omission; naming it again here would
        // charge one mistake twice.
        Ty::Mixed => true,
        Ty::Class(name, _) => *name == QName::parse(r"Core\Uuid"),
        // § 3 admits a union of `string` or `int` literal types and a subset of
        // an enum's cases, and nothing wider: a `string|int` would make the
        // conversion itself ambiguous, which is the question ADR 0095 refuses
        // to answer by guessing.
        Ty::Union(members) => members.iter().all(|member| {
            matches!(
                env.interner.get(*member),
                Ty::StringLiteral(_) | Ty::IntLiteral(_) | Ty::EnumCase(..)
            )
        }),
        _ => false,
    }
}
