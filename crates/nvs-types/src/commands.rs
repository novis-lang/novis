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
//! This module is the pass behind those two roster entries: a name on that
//! closed list with nothing checking its payload would admit
//! `#[Command(nmae: "deploy")]` silently, which is the whole argument
//! [`crate::derive`]'s own gap 3 makes for keeping the list short.
//!
//! # What is checked here, and what the table still owes
//!
//! Checked: the payload's field names against [`COMMAND_OPTIONS`] /
//! [`OPTION_OPTIONS`], each value's type, and a field given twice. That is one
//! declaration read on its own, which is all this pass can see — it runs from
//! [`crate::attributes`]'s per-attribute walk.
//!
//! # Known gaps
//!
//! 1. **The table itself is not built**, so § 6's three compile errors are not
//!    reported: a duplicate command name (a question about the whole program's
//!    enumeration), two options sharing a short or long spelling (a question
//!    about one method's parameter list), and an `#[Option]` on a parameter
//!    whose declared type has no conversion from `string` (a question about the
//!    parameter the attribute is attached to, which this walk does not carry).
//!    All three want the walk that assembles rows, not the one that reads a
//!    payload.
//! 2. **Whether `name` is required on `#[Command]` is not decided here.** § 6
//!    writes every example with one and says nothing about leaving it out, and
//!    ADR 0077 § 1's "optional and never derived" is a rule about *routes*. The
//!    table pass is what needs a name to build a row, so it is what should
//!    refuse a row that has none.

use nvs_diagnostics::{Diagnostic, code};
use nvs_syntax::ast::ObjectLiteralField;
use rustc_hash::FxHashSet;

use crate::expr::check_expr;
use crate::locals::LocalScope;
use crate::ty::Ty;
use crate::{Ctx, Env, span_text};

/// `#[Command(name: string, about: string)]` — ADR 0086 § 6's own spelling, in
/// the order that section writes it.
pub(crate) const COMMAND_OPTIONS: &[&str] = &["name", "about"];

/// `#[Option(short: string, long: string, about: string)]` — § 6 writes
/// `short` and `about`; `long` is the spelling an option is matched by when the
/// parameter's own name is not it, and it is named here for the same reason
/// `short` is: nothing about a command line is inferred.
pub(crate) const OPTION_OPTIONS: &[&str] = &["short", "long", "about"];

/// One `#[Command]` or `#[Option]` payload, checked against its roster.
///
/// Called only for a payload [`crate::attributes`] has already proved constant,
/// for that module's own reason: the author is told about a value they wrote
/// before they are told what it failed to satisfy.
///
/// `attribute` is the name as it is written in a diagnostic — `#[Command]` or
/// `#[Option]` — and `options` its roster. Every option of both is a `string`,
/// which is why there is no per-option type here; the first one that is not
/// brings [`crate::testing`]'s `OptionTy` with it.
pub(crate) fn check_payload(
    attribute: &str,
    options: &[&str],
    fields: &[ObjectLiteralField],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    // The scope is empty and stays empty: ADR 0046 § 2 has just proved this
    // payload reads no variable, so there is no binding to mark live and none
    // to capture.
    let mut live = FxHashSet::default();
    let scope = LocalScope::new();
    let expected = env.interner.intern(Ty::String);
    let mut seen: Vec<String> = Vec::with_capacity(fields.len());
    for field in fields {
        let name = span_text(env.src, field.name).to_owned();
        let declared = options.contains(&name.as_str());
        check_expr(
            &field.value,
            declared.then_some(expected),
            &mut live,
            &scope,
            ctx,
            env,
        );
        if !declared {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNKNOWN_OPTION,
                    format!("`{name}` is not an option of `#[{attribute}]`"),
                )
                .with_primary(field.span, "no such option")
                .with_help(format!(
                    "the options are: {}",
                    options
                        .iter()
                        .map(|option| format!("{option}: string"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            );
        } else if seen.iter().any(|already| already == &name) {
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
}
