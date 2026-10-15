//! `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`'s
//! `[session] backend`: the stores a session record may live in, and the boot-time refusal of the
//! memory tier `rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent` removes.
//!
//! **This module is what "enforced rather than documented" means.**
//! `rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent` says
//! `Core\Session`'s configurable backends do not include the memory tier, which is a claim about a
//! roster — and a roster that exists only in prose excludes nothing. [`Backend`] is the roster;
//! [`Backend::of`] is the one place a written word becomes one; [`validate`] is the refusal. A
//! spelling admitted anywhere else would be a second roster, and the shape of that failure is a
//! deployment whose sessions live in one process while its configuration says they do not.
//!
//! **Refused where it is written, never where it is used**, which is [`crate::log`]'s rule applied
//! to a worse failure. A session in one process's memory does not error: it forgets people, at a
//! rate set by which process happened to accept the request and when it last restarted, and every
//! symptom of it looks like a user signing themselves out. There is no moment at run time when that
//! is reportable, so boot — where an operator is reading output and holding the file — is the only
//! place the sentence lands.
//!
//! Cost: one match over a short string per `[session]` block in the merged tree, at boot, and
//! nothing at all per request.

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::Config;

/// One of `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`'s stores, as written.
///
/// The type carries no variant for the memory tier and must not gain one: what makes
/// `rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent` enforced is that there is
/// no value of this type meaning the process's map, so no later reader can select it however
/// carelessly it matches. A backend added here is another store that answers
/// `rule:http-server/a-session-store-answers-four-operations`'s
/// operations, and the memory tier cannot answer them — `load` where nothing wrote is the question
/// it gets wrong, in the next process a deployment scales to and in this one after a restart.
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

/// Every backend `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`
/// admits, in the order it lists them.
///
/// Public because the refusal's own help text is built from it and because the roster is the thing
/// asserted — a test that reads this constant fails when a `Memory` is added, where a test that
/// spelled the words itself would keep passing beside it.
pub const BACKENDS: &[(&str, Backend)] = &[("shared", Backend::Shared), ("db", Backend::Db)];

impl Backend {
    /// What `written` names, or `None` for a word the roster does not spell — `memory` included,
    /// which is the whole point and is why this answers `None` rather than an error of its own.
    #[must_use]
    pub fn of(written: &str) -> Option<Self> {
        BACKENDS
            .iter()
            .find(|(word, _)| *word == written)
            .map(|(_, backend)| *backend)
    }
}

/// The word the refusal below exists for: `Core\Cache`'s own name for the memory tier,
/// deliberately. An operator writes it here because that is what the tier is called, so the refusal
/// has to recognise it in order to say anything better than "unknown value".
const REFUSED: &str = "memory";

/// What the refusal of [`REFUSED`] says, in the words an operator reads at boot.
const REFUSED_NOTE: &str = "The `memory` cache is kept inside one server process. A request that \
                            another server process handles does not find the session, and a \
                            restart deletes every session.";

/// `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot` over the merged
/// tree: every `[session] backend` names a store that rule admits.
///
/// The merged tree rather than each file, for [`crate::http::validate`]'s reason — a base file
/// naming `memory` that an include replaces is not a deployment running on `memory`, and refusing
/// it per file would refuse a correction that had already been made.
///
/// # Errors
///
/// `E0626` for the memory tier, with a note saying how far that tier reaches and the file the key
/// was written in; the same code with a plainer note for a word that names nothing at all.
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
        REFUSED_NOTE.to_owned()
    } else {
        format!("A session store is {spellings}.")
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

    use super::{BACKENDS, Backend, REFUSED, REFUSED_NOTE, validate};
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

    /// `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`, both sides
    /// of the roster: the words it admits resolve, and the memory tier
    /// `rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent` removes is refused with
    /// its own reason rather than with a generic unknown-value message an operator learns nothing
    /// from.
    ///
    /// Driven off the roster rather than off spellings written here, so a store added to it is
    /// asserted about by existing in it — a case naming its own words passes beside a roster that
    /// grew a third.
    // covers: directive:session
    #[test]
    fn the_memory_tier_is_not_a_backend_and_the_two_stores_resolve() {
        for (word, backend) in BACKENDS {
            assert_eq!(Backend::of(word), Some(*backend));
            validate(&wrote(word), &BTreeMap::new()).expect("a store the roster admits");
        }

        assert_eq!(Backend::of(REFUSED), None, "`{REFUSED}` names no store");
        let refused = validate(&wrote(REFUSED), &BTreeMap::new()).expect_err("the memory tier");
        assert_eq!(refused.code, Some(code::E_SESSION_BACKEND));
        // The *reason*: a note that only said the word is not allowed would leave the operator
        // guessing why the cache they already use for everything else is wrong here.
        assert!(
            refused
                .notes
                .iter()
                .any(|note| note.contains(REFUSED_NOTE) && note.contains("restart deletes")),
            "`{REFUSED}` is only refused as enforcement if the note says what it loses: {:?}",
            refused.notes
        );

        // A word that names nothing gets the same code and a different sentence: there is no
        // coherence reasoning to give about a typo, and giving it anyway would teach that the
        // message is boilerplate.
        let unknown = validate(&wrote("redis"), &BTreeMap::new()).expect_err("no such store");
        assert_eq!(unknown.code, Some(code::E_SESSION_BACKEND));
        assert!(
            unknown
                .notes
                .iter()
                .all(|note| !note.contains("restart deletes"))
        );
    }

    /// `rule:http-server/no-session-block-means-no-store`: a tree with no `[session]` block, and one that wrote the
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
