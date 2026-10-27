//! `rule:security/optional-capability-degrades`
//! 's `Core\Cap::has`, and the one thing about it that is decided while
//! compiling: the name it asks about has to be a capability.
//!
//! The member itself runs — it reads the request's own grant table, which is
//! not known here — so unlike [`crate::retrieval`] and [`crate::program`] this
//! pass replaces nothing and records nothing. It is a refusal and no more.
//!
//! # Why a misspelling cannot be left to run time
//!
//! `has` answers a `bool`, and a name outside § 8's roster answers `false` —
//! the only safe answer, since nothing grants a capability that does not exist.
//! But *not granted* is what a correct spelling answers too, on every
//! deployment that did not grant it. So `Core\Cap::has("fs.delete")` is a
//! branch that never runs, on every machine, and there is no state anywhere at
//! run time from which the two could be told apart. That is exactly `E0798`'s
//! argument for checking a written `$member` at an attribute retrieval, and it
//! applies here for the same reason: the wrong answer and the right one are the
//! same value.
//!
//! Only a **written** name is refused. `rule:expressions/intrinsic-constant-arguments`'s rule — nothing is refused
//! for being dynamic — leaves a computed argument to run time, where
//! `nvs_stdlib::cap`'s helper answers `false` for it.
//!
//! # Both spellings come from `nvs-stdlib`
//!
//! Which member this is, and which names are capabilities, are
//! [`nvs_stdlib::cap::is_query`] and [`nvs_stdlib::cap::is_capability`]. The
//! latter reads `nvs_config::capability::Cap` — the same roster a `[grants]`
//! line is read against — so a capability added there is one this refusal
//! accepts in the same edit, and the checker holds no copy of a name. That
//! matters more than usual here: a stale copy would refuse a capability the
//! configuration grants, which is a compile error nobody can work around.
//!
//! # A call holds its namespace's grant line
//!
//! [`reject_ungranted_call`] is `rule:security/authority-is-the-enclosing-namespace`
//! at a call site: a `Core` member that needs a capability compiles only where
//! the namespace enclosing the call has it on its `[grants]` line, or has no
//! line at all (`rule:security/an-unmatched-namespace-holds-the-application`).
//! The namespace is [`Ctx::namespace`], which is lexical, so file-scope code is
//! held like a method body, an anonymous function keeps the namespace it was
//! written in, and a `use` import changes nothing. A method reference is held
//! where it is written, as a call is, so a reference cannot carry a member out
//! of a namespace that may not call it.
//!
//! This is the namespace's half of the question. The operator's half, the
//! `nvs.toml` grant and its scope, stays at the door when the call runs
//! (`rule:security/no-runtime-grant`), so a call holds the intersection of the
//! two. Cost: one table lookup per `Core` call site while compiling, and none
//! when the table is empty. Nothing runs on the request path.

use nvs_config::Cap;
use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::CallArgs;

use crate::defaults::ConstArg;
use crate::expr_table::{Prepared, PreparedFact};
use crate::ty::Ty;
use crate::{Ctx, Env};

/// Refuses a `Core\Cap::has` whose written capability is not one, and passes
/// everything else through untouched.
///
/// The hook `crate::expr::calls` reaches after the target has resolved, beside
/// the other refusals over a `Core` call's own arguments. Static-only, because
/// the member is: there is no receiver a shape of this could arrive through.
pub(crate) fn reject_unknown_capability(
    owner: &QName,
    member: &str,
    args: &CallArgs,
    env: &mut Env<'_>,
) {
    if !nvs_stdlib::cap::is_query(&owner.to_string(), member) {
        return;
    }
    let CallArgs::List(list) = args else {
        return;
    };
    // A `name:` argument still fills the one parameter this member has, so it
    // is read like a positional one; a `...` spread is not a written name at
    // all and falls to the dynamic case below with everything else.
    let Some(arg) = list.iter().find(|arg| !arg.spread) else {
        return;
    };
    let declared = env.interner.intern(Ty::String);
    let Some(ConstArg::Str(name)) = crate::defaults::literal_default(&arg.value, declared, env)
    else {
        return;
    };
    if nvs_stdlib::cap::is_capability(&name) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_NOT_A_CAPABILITY,
            format!("`{name}` is not a capability"),
        )
        .with_primary(arg.value.span, "no `nvs.toml` grant can name this")
        .with_help(format!(
            "this call would return `false` on every server. A capability that is not granted \
             also returns `false`, so a misspelled name is found only here. The capabilities are: {}",
            nvs_stdlib::cap::roster()
        )),
    );
}

/// Files [`PreparedFact::ApplicationSet`] for a `Core\Config::set` written in
/// the application's own code: a namespace no `[grants]` line matches, or the
/// global namespace.
///
/// A package may lower a limit and never raise one
/// (`rule:security/package-authority-is-granted-one-line-at-a-time`), and the
/// member cannot see its caller's namespace. So the fact is the word that lets
/// a `set` raise, and a call without it may only lower. A method reference
/// lowers to a thunk that carries no word, so it can only lower wherever it
/// was written. Cost: two segment compares per static call named `set`, and
/// one table lookup for a `Core\Config::set`, while compiling.
pub(crate) fn prepare_config_set(
    owner: &QName,
    member: &str,
    span: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if member != "set" || owner.segments() != ["Core", "Config"] {
        return;
    }
    let application = ctx.namespace.is_empty()
        || env.namespace_grants.is_empty()
        || env
            .namespace_grants
            .lookup(&ctx.namespace.join("\\"))
            .is_none();
    if application {
        env.exprs.record_prepared(
            span,
            Prepared {
                literal: span,
                fact: PreparedFact::ApplicationSet,
            },
        );
    }
}

/// Refuses a call to `owner::member` where the enclosing namespace's
/// `[grants]` line does not list a capability the member needs, as
/// [`nvs_stdlib::cap::needs`] gives them. A namespace with no line, and the
/// global namespace, hold the application's authority and pass.
///
/// The hook `crate::expr::calls` reaches once a call or a method reference has
/// resolved, on the instance and the static path alike.
pub(crate) fn reject_ungranted_call(
    owner: &QName,
    member: &str,
    span: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if env.namespace_grants.is_empty() || ctx.namespace.is_empty() {
        return;
    }
    let namespace = ctx.namespace.join("\\");
    let Some(line) = env.namespace_grants.lookup(&namespace) else {
        return;
    };
    let class = owner.to_string();
    let mut missing: Vec<Cap> = nvs_stdlib::cap::needs(&class, member)
        .filter(|cap| !line.grants(*cap))
        .collect();
    if missing.is_empty() {
        return;
    }
    missing.dedup();
    let names = |caps: &[Cap], quote: &str| {
        caps.iter()
            .map(|cap| format!("{quote}{}{quote}", cap.name()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let file = env.namespace_grants.file().map_or_else(
        || "package.toml".to_owned(),
        |path| path.display().to_string(),
    );
    env.diags.report(
        Diagnostic::error(
            code::E_CAPABILITY_NOT_GRANTED,
            format!(
                "the namespace `{namespace}` does not have the capability {}",
                names(&missing, "`")
            ),
        )
        .with_primary(
            span,
            format!("`{class}::{member}` needs {}", names(&missing, "`")),
        )
        .with_note(format!(
            "this line in `{file}` gives the capabilities of `{namespace}`: '{}' = [{}]",
            line.key(),
            names(line.caps(), "\"")
        ))
        .with_help(format!(
            "add {} to that line if this code should have it",
            names(&missing, "\"")
        )),
    );
}
