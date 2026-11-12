//! `rule:security/optional-capability-degrades`
//! 's `Core\Cap::has`, and the one thing about it that is decided while
//! compiling: the name it asks about has to be a capability.
//!
//! The member itself runs, because the operator's grant is the request's and is
//! not known here. What is known here is the namespace's own grant line, and
//! [`prepare_cap_has`] hands the member that as a prepared word.
//!
//! # Why a misspelling cannot be left to run time
//!
//! `has` answers a `bool`, and a name outside the roster
//! (`rule:security/capability-roster-is-closed`) answers `false` —
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
//!
//! # An extension is asked like a `Core` class
//!
//! A call to a loaded extension's class needs the capabilities its manifest's
//! `requests` imply, for every method alike
//! ([`crate::ext_lib::implied_capabilities`],
//! `rule:security/extension-grants-are-an-intersection`). A manifest that
//! requests nothing needs no line.
//!
//! The Novis source a third-party `.nvsx` carries is a package. Where no line
//! matches its namespace, [`authority_of`] returns [`Authority::Implied`], an
//! empty line, also where the application has no grant table at all. A
//! built-in component's source keeps the application's authority, because only
//! Novis declares under `Novis\`
//! (`rule:packaging/an-extension-package-carries-two-payloads`). Cost: one
//! compare per loaded third-party extension per lookup while compiling.
//!
//! # An optional capability compiles to a guard
//!
//! A capability the call's package lists under `[capabilities] optional`
//! compiles where its grant line does not list it
//! (`rule:security/optional-capability-degrades`). The checker records a
//! [`CapabilityGuard`] at the call instead of reporting it, and `nvs-ir`
//! throws a `RuntimeError` there before the call runs. The grant line is
//! compile-time input and is in the unit's key, so the guard is decided here
//! and the program carries no branch: a granted call has no guard at all, and
//! an ungranted one always throws when it is reached. The package is the one
//! [`crate::package::check`] placed the call's file in, so a file of the
//! application never has a guard.
//!
//! # A package reads its own files with no grant
//!
//! Code in a package's namespace may read the files its package carries and
//! downloads with no `fs.read` on its line
//! (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`).
//! [`own_file_word`] allows exactly one shape: a member on
//! [`nvs_stdlib::registry::OWN_FILE_READS`], whose first argument is a string
//! literal, whose path canonicalizes to a file inside the package's folder
//! that `vendor/package-files.toml` lists for it
//! ([`nvs_config::package::Package::files`]). A path built at run time, a
//! write, a listing, a file the set does not list and another package's file
//! all still need the grant. `..` and a symlink are resolved before the
//! comparison, so neither leaves the set.
//!
//! The approved call also carries [`PreparedFact::OwnFile`], the
//! [`nvs_config::capability::own_file_word`] of the file, because the
//! operator's `fs.read` scope may leave `vendor/` out. The door resolves the
//! path again when it runs and passes `fs.read` only when the words match, so
//! a run-time path to the same file still meets the scope. The fact is filed
//! whether or not the line lists `fs.read`, since the scope is asked either
//! way. Cost: one `canonicalize` and one hash per such call site in a package
//! while compiling; at run time, one resolution and one hash for a read the
//! scope refuses.

use std::path::{Path, PathBuf};

use nvs_config::Cap;
use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{CallArgs, ExprKind};

use crate::defaults::ConstArg;
use crate::expr_table::{ArgSlot, CapabilityGuard, Prepared, PreparedFact};
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
    if matches!(authority_of(ctx, env), Authority::Application) {
        env.exprs.record_prepared(
            span,
            Prepared {
                literal: span,
                fact: PreparedFact::ApplicationSet,
            },
        );
    }
}

/// Files [`PreparedFact::CapGrants`] for a `Core\Cap::has`: the capabilities
/// its namespace's `[grants]` line lists, or every capability where no line
/// matches. The member answers `true` only for a capability in the word that
/// the operator also granted, so it answers what its own call site holds.
/// A method reference to `has` carries no word, and answers `false` wherever
/// it was written: it fails closed, as `Core\Config::set`'s does. Cost: one
/// table lookup per `Core\Cap::has` call site while compiling.
pub(crate) fn prepare_cap_has(
    owner: &QName,
    member: &str,
    span: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if !nvs_stdlib::cap::is_query(&owner.to_string(), member) {
        return;
    }
    let authority = authority_of(ctx, env);
    let mask = nvs_stdlib::cap::grants_mask(authority.caps().unwrap_or(Cap::ALL).iter().copied());
    env.exprs.record_prepared(
        span,
        Prepared {
            literal: span,
            fact: PreparedFact::CapGrants(mask),
        },
    );
}

/// Refuses a call to `owner::member` where the enclosing namespace's
/// `[grants]` line does not list a capability the member needs, as
/// [`nvs_stdlib::cap::needs`] gives them. A namespace with no line, and the
/// global namespace, hold the application's authority and pass.
///
/// A missing capability that the call's package lists as `optional` is not
/// refused: it records a [`CapabilityGuard`], and the call throws when it runs.
/// A missing `fs.read` is not refused for a read of the package's own file
/// ([`own_file_word`]).
///
/// The hook `crate::expr::calls` reaches once a call or a method reference has
/// resolved, on the instance and the static path alike. `args` and `slots` are
/// the call's written arguments and the parameter each fills.
pub(crate) fn reject_ungranted_call(
    owner: &QName,
    member: &str,
    span: Span,
    (args, slots): (&CallArgs, &[ArgSlot]),
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if matches!(authority_of(ctx, env), Authority::Application) {
        return;
    }
    let class = owner.to_string();
    let own = own_file_word(&class, member, span, args, slots, env);
    if let Some(word) = own {
        env.exprs.record_prepared(
            span,
            Prepared {
                literal: span,
                fact: PreparedFact::OwnFile(word),
            },
        );
    }
    // A loaded extension's class has no registry row: its needs are what its
    // manifest requests, for every method alike.
    let needs = nvs_stdlib::cap::needs(&class, member)
        .chain(env.signatures.extension_requests(owner).iter().copied())
        .filter(|cap| own.is_none() || *cap != Cap::FsRead)
        .collect::<Vec<_>>();
    reject_ungranted(&needs, &format!("`{class}::{member}`"), span, ctx, env);
}

/// Whose authority the code at a call site holds
/// (`rule:security/authority-is-the-enclosing-namespace`).
pub(crate) enum Authority<'g> {
    /// The application's own: the global namespace, a namespace no `[grants]`
    /// line matches, or any namespace where the table is empty
    /// (`rule:security/an-unmatched-namespace-holds-the-application`).
    Application,
    /// The `[grants]` line that matches the enclosing namespace.
    Line(&'g nvs_config::grants::Grant),
    /// An empty line no file writes: the namespace is at or under a loaded
    /// third-party extension's own, and no `[grants]` line matches it
    /// (`rule:packaging/an-extension-package-carries-two-payloads`).
    Implied,
}

impl Authority<'_> {
    /// The capabilities this authority holds by its line: none for an implied
    /// line, and `None` for the application, which holds every one.
    fn caps(&self) -> Option<&[Cap]> {
        match self {
            Self::Application => None,
            Self::Line(line) => Some(line.caps()),
            Self::Implied => Some(&[]),
        }
    }
}

/// The authority of the code `ctx` is checking: the one test every grant check
/// in this module makes. A written line wins. Where none matches, the source
/// of a third-party extension holds an implied empty line, also when the table
/// is empty, and every other namespace holds the application's authority.
/// Cost: one table lookup and one compare per loaded third-party extension,
/// and nothing when the namespace is global.
pub(crate) fn authority_of<'g>(ctx: &Ctx<'_>, env: &Env<'g>) -> Authority<'g> {
    if ctx.namespace.is_empty() {
        return Authority::Application;
    }
    let namespace = ctx.namespace.join("\\");
    let grants: &'g nvs_config::grants::Grants = env.namespace_grants;
    if let Some(line) = grants.lookup(&namespace) {
        return Authority::Line(line);
    }
    if env.signatures.is_third_party_source(&namespace) {
        return Authority::Implied;
    }
    Authority::Application
}

/// Refuses the construct at `span`, named by `what`, where the enclosing
/// namespace's `[grants]` line does not list each of `needs`. The application
/// passes. A missing capability the call's package lists as `optional`
/// records a [`CapabilityGuard`] instead, and the construct throws when it
/// runs.
pub(crate) fn reject_ungranted(
    needs: &[Cap],
    what: &str,
    span: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let authority = authority_of(ctx, env);
    let Some(held) = authority.caps() else {
        return;
    };
    let namespace = ctx.namespace.join("\\");
    let mut missing: Vec<Cap> = needs
        .iter()
        .copied()
        .filter(|cap| !held.contains(cap))
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
    // A written line is named by its key. An implied one has no key yet, so
    // the text names the line to write.
    let key = match &authority {
        Authority::Line(line) => line.key(),
        Authority::Implied | Authority::Application => namespace.clone(),
    };
    let optional = env
        .packages
        .and_then(|packages| packages.of(span.file))
        .map_or(&[][..], nvs_config::package::Package::optional);
    if missing.iter().all(|cap| optional.contains(cap)) {
        let fix = match &authority {
            Authority::Line(_) => format!(
                "Add {} to the line '{key}' in `{file}` to allow it.",
                names(&missing, "\""),
            ),
            Authority::Implied | Authority::Application => format!(
                "Add the line '{key}' = [{}] to `[grants]` in `{file}` to allow it.",
                names(&missing, "\""),
            ),
        };
        env.exprs.record_capability_guard(
            span,
            CapabilityGuard {
                message: format!(
                    "{what} needs {}. The namespace `{namespace}` does not have it. {fix}",
                    names(&missing, "`"),
                ),
            },
        );
        return;
    }
    missing.retain(|cap| !optional.contains(cap));
    let (note, help) = match &authority {
        Authority::Line(_) => (
            format!(
                "this line in `{file}` gives the capabilities of `{namespace}`: '{key}' = [{}]",
                names(held, "\"")
            ),
            format!(
                "add {} to that line if this code should have it",
                names(&missing, "\"")
            ),
        ),
        Authority::Implied | Authority::Application => (
            format!(
                "`{namespace}` is the namespace of an extension's own code. It has no capabilities until a line in `[grants]` names it"
            ),
            format!(
                "add this line to `[grants]` in `{file}` if this code should have it: '{key}' = [{}]",
                names(&missing, "\"")
            ),
        ),
    };
    env.diags.report(
        Diagnostic::error(
            code::E_CAPABILITY_NOT_GRANTED,
            format!(
                "the namespace `{namespace}` does not have the capability {}",
                names(&missing, "`")
            ),
        )
        .with_primary(span, format!("{what} needs {}", names(&missing, "`")))
        .with_note(note)
        .with_help(help),
    );
}

/// The own-file word of the call `class::member` at `span` when it reads a
/// file of its own package's set, so it needs no `fs.read` grant: the member
/// is on [`nvs_stdlib::registry::OWN_FILE_READS`], the call is in a package,
/// and the argument that fills the first parameter is a string literal whose
/// canonical path is inside the package's folder and listed in its set.
/// `None` for every other call.
fn own_file_word(
    class: &str,
    member: &str,
    span: Span,
    args: &CallArgs,
    slots: &[ArgSlot],
    env: &Env<'_>,
) -> Option<i64> {
    if !nvs_stdlib::registry::OWN_FILE_READS
        .iter()
        .any(|&(owner, name)| owner == class && name == member)
    {
        return None;
    }
    let package = env.packages.and_then(|packages| packages.of(span.file))?;
    let CallArgs::List(list) = args else {
        return None;
    };
    let arg = list
        .iter()
        .zip(slots)
        .find_map(|(arg, slot)| matches!(slot, ArgSlot::Param(0)).then_some(arg))?;
    let ExprKind::Str(literal) = arg.value.unparenthesized().kind else {
        return None;
    };
    // A relative literal was joined to its file's folder by `crate::paths`; an
    // absolute one is its own text.
    let path = env.exprs.written_path(literal).map_or_else(
        || {
            PathBuf::from(nvs_syntax::string_lit::cook_string_literal(
                env.src, literal,
            ))
        },
        PathBuf::from,
    );
    let listed = path.is_absolute()
        && nvs_hir::requires::canonicalize(&path)
            .and_then(|real| relative_in(&real, package.dir()))
            .is_some_and(|relative| package.lists(&relative));
    // The word is of the path as the door resolves it, so both sides hash
    // one spelling.
    listed
        .then(|| nvs_config::capability::resolved(&path, &nvs_config::resolve::Disk))
        .flatten()
        .map(|at| nvs_config::capability::own_file_word(&at))
}

/// `path` relative to `dir`, its parts joined with `/`, or `None` when it is
/// not inside `dir` or a part is not UTF-8. Both paths are canonical.
fn relative_in(path: &Path, dir: &Path) -> Option<String> {
    let parts = path
        .strip_prefix(dir)
        .ok()?
        .components()
        .map(|part| part.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()?;
    (!parts.is_empty()).then(|| parts.join("/"))
}
