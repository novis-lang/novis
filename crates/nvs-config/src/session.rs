//! [ADR 0139](/docs/adr/0139-a-session-is-a-record-its-store-issued.md) § 3's
//! `[session] backend`: the two stores a session record may live in, and the boot-time refusal of
//! the one [ADR 0059](/docs/adr/0059-cross-request-state-is-explicit.md) § 4 removed.
//!
//! **This module is what § 4's "enforced rather than documented" means.** That section says
//! `Core\Session`'s configurable backends do not include the local tier, which is a claim about a
//! roster — and a roster that exists only in prose excludes nothing. [`Backend`] is the roster;
//! [`Backend::of`] is the one place a written word becomes one; [`validate`] is the refusal. A
//! third spelling anywhere would be a second roster, and the shape of that failure is a deployment
//! whose sessions are per-core while its configuration says they are not.
//!
//! **Refused where it is written, never where it is used**, which is [`crate::log`]'s rule applied
//! to a worse failure. A session on a per-core store does not error: it forgets people, at a rate
//! set by which core happened to accept the request, and every symptom of it looks like a user
//! signing themselves out. There is no moment at run time when that is reportable, so boot — where
//! an operator is reading output and holding the file — is the only place the sentence lands.
//!
//! Cost: one match over a short string per `[session]` block in the merged tree, at boot, and
//! nothing at all per request.

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::Config;

/// One of ADR 0139 § 3's two stores, as written.
///
/// The type carries no `Local` variant and must not gain one: what makes ADR 0059 § 4 enforced is
/// that there is no value of this type meaning the per-core tier, so no later reader can select it
/// however carelessly it matches. A backend added here is a third store that answers § 2's four
/// operations, and the local tier cannot answer them — `load` on the core that never wrote is the
/// one question it gets wrong.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Backend {
    /// `shared` — `rule:core-api/two-cache-tiers`'s coherent tier, reached at `[cache.shared] url`. One store per
    /// deployment, so a session store that named a second address would be a second thing to
    /// configure for no coherence the first does not already have.
    Shared,
    /// `db` — a table in a `[db.<name>]`, for a deployment that has a database and would rather not
    /// have a second stateful dependency. `nvs_stdlib::session`'s module doc owns which half is on
    /// disk.
    Db,
}

/// Every backend § 3 admits, in the order the ADR lists them.
///
/// Public because the refusal's own help text is built from it and because the roster is the thing
/// asserted — a test that reads this constant fails when a `Local` is added, where a test that
/// spelled the two words itself would keep passing beside it.
pub const BACKENDS: &[(&str, Backend)] = &[("shared", Backend::Shared), ("db", Backend::Db)];

impl Backend {
    /// What `written` names, or `None` for a word § 3 does not spell — `local` included, which is
    /// the whole point and is why this answers `None` rather than an error of its own.
    #[must_use]
    pub fn of(written: &str) -> Option<Self> {
        BACKENDS
            .iter()
            .find(|(word, _)| *word == written)
            .map(|(_, backend)| *backend)
    }
}

/// The word the refusal below exists for, spelled once.
///
/// `Core\Cache`'s own tier name, deliberately: an operator writes it here because that is what the
/// tier is called, so the refusal has to recognise it in order to say anything better than "unknown
/// value".
const REFUSED: &str = "local";

/// ADR 0139 § 3 over the merged tree: every `[session] backend` names a store § 3 admits.
///
/// The merged tree rather than each file, for [`crate::http::validate`]'s reason — a base file
/// naming `local` that an include replaces is not a deployment running on `local`, and refusing it
/// per file would refuse a correction that had already been made.
///
/// # Errors
///
/// `E0626` for the local tier, naming ADR 0059 § 4 and the file the key was written in; the same
/// code with a plainer note for a word that names nothing at all.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(session) = config.session.as_ref() else {
        return Ok(());
    };
    let Some(written) = session.backend.as_deref() else {
        return Ok(());
    };
    if Backend::of(written).is_some() {
        return Ok(());
    }

    let spellings = BACKENDS
        .iter()
        .map(|(word, _)| format!("`\"{word}\"`"))
        .collect::<Vec<_>>()
        .join(" or ");
    let note = if written == REFUSED {
        "ADR 0059 § 4: a session read on one core and written on another must see one value, and \
         the local tier is per-core — a session kept there is forgotten at a rate set by which \
         core accepted the request, which is an authentication bug wearing a cache's clothes"
            .to_owned()
    } else {
        format!("ADR 0139 § 3 admits {spellings}, and nothing else")
    };
    Err(Diagnostic::error(
        code::E_SESSION_BACKEND,
        format!("`[session] backend = \"{written}\"` names no store a session may live in"),
    )
    .with_note(format!(
        "{note}{}",
        origin_note(origins.get("session.backend"))
    ))
    .with_help(format!("write `backend = {spellings}`")))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nvs_diagnostics::code;

    use super::{BACKENDS, Backend, validate};
    use crate::tree::{Config, Session};

    /// A tree whose `[session]` block writes `backend`, and nothing else.
    fn wrote(backend: &str) -> Config {
        Config {
            session: Some(Session {
                backend: Some(backend.to_owned()),
                ..Session::default()
            }),
            ..Config::default()
        }
    }

    /// ADR 0139 § 3, both sides of the roster: the two words it admits resolve, and the one ADR
    /// 0059 § 4 removed is refused with that section named rather than with a generic
    /// unknown-value message an operator learns nothing from.
    #[test]
    fn the_local_tier_is_refused_by_name_and_the_two_stores_resolve() {
        for (word, backend) in BACKENDS {
            assert_eq!(Backend::of(word), Some(*backend));
            validate(&wrote(word), &BTreeMap::new()).expect("a store § 3 admits");
        }
        assert_eq!(Backend::of("local"), None);

        let refused = validate(&wrote("local"), &BTreeMap::new()).expect_err("the per-core tier");
        assert_eq!(refused.code, Some(code::E_SESSION_BACKEND));
        // The *reason*, not the citation that introduces it. Asserting on the reference alone
        // passes for a note that cites § 4 and then says nothing, which is the failure this test
        // exists to catch; it also pins a spelling that belongs to the docs rather than to this
        // module, so the citation cannot be re-pointed without a red test in an unrelated crate.
        assert!(
            refused
                .notes
                .iter()
                .any(|note| note.contains("an authentication bug wearing a cache's clothes")),
            "the refusal is only enforcement if it carries § 4's reason: {:?}",
            refused.notes
        );

        // A word that names nothing gets the same code and a different sentence: there is no § 4
        // reasoning to give about a typo, and giving it anyway would teach that the message is
        // boilerplate.
        let unknown = validate(&wrote("redis"), &BTreeMap::new()).expect_err("no such store");
        assert!(
            unknown
                .notes
                .iter()
                .all(|note| !note.contains("an authentication bug"))
        );
    }

    /// § 3's "absent is not a default": a tree with no `[session]` block, and one that wrote the
    /// block without choosing, both boot — the refusal belongs to `Core\Session::start()`, which is
    /// where a program that actually wants a session says so.
    #[test]
    fn a_tree_that_configures_no_session_still_boots() {
        validate(&Config::default(), &BTreeMap::new()).expect("no block at all");
        validate(
            &Config {
                session: Some(Session::default()),
                ..Config::default()
            },
            &BTreeMap::new(),
        )
        .expect("a block that chose nothing");
    }
}
