//! `rule:config/cache-shared-is-the-grant-over-the-configured-store`'s boot half: what a
//! configuration already says about the coherent tier before any request asks for it.
//!
//! The first question is a *pair* rather than a value: `[cache.shared] url` names a store
//! and `[capabilities] cache.shared` says who may reach one, and a tree holding the first without
//! the second describes a store nothing can open. Neither key is wrong on its own, so this is
//! [`W1008`](nvs_diagnostics::code::W_STORE_CONFIGURED_UNGRANTED) and never a refusal — the code's
//! own doc is the home of that reasoning.
//!
//! The second is about the same key alone, and it is a refusal: a `unix:` store on a build with no
//! `AF_UNIX` transport names something this binary cannot open at all, which is
//! [`E0635`](nvs_diagnostics::code::E_NO_UNIX_TRANSPORT). Two functions rather than one pass,
//! because [`validate`] returns and [`advise`] does not — the difference `resolve`'s two call sites
//! are written around.
//!
//! **Here rather than in [`mod@crate::capability`]**, which is the pure decision procedure: it
//! answers a grant question and reports nothing, and a pass that builds diagnostics would be a
//! second thing that module does. **Here rather than in [`mod@crate::cache`]**, which holds the
//! unit-key formula and reads no `Config` at all. What this module is for is the `[cache.shared]`
//! block as a *boot* reads it, which is where the next question about that block also lands.
//!
//! Cost: two `Option` reads over the merged tree, at boot, and nothing per request.

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, code};

use crate::capability::Cap;
use crate::resolve::{Origin, origin_note};
use crate::tree::Config;

/// The scheme a `[cache.shared] url` spells a Unix-domain socket with —
/// `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host`, whose other half is the bare
/// absolute path `[db.<name>] host` takes.
///
/// Here rather than beside the client that dials one, because the spelling belongs to the key and
/// the key belongs to this crate: `crate::store` decides what may be written and
/// `nvs_stdlib::cache` reads what was.
pub const UNIX_SCHEME: &str = "unix:";

/// The store `[cache.shared] url` names, or `None` for a tree that configured none.
///
/// An empty value is `None` on purpose: `url = ""` is how a later file turns an inherited store
/// back off, and a tree with no coherent tier has neither question below to answer.
fn configured(config: &Config) -> Option<&str> {
    config
        .cache
        .as_ref()?
        .shared
        .as_ref()?
        .url
        .as_deref()
        .filter(|url| !url.is_empty())
}

/// The refusal a tree earns by naming a socket this build has no transport for.
///
/// Asked of the merged tree for [`advise`]'s reason, and a refusal rather than a warning because
/// `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot` is that a deployment
/// must not run believing it has a store it will never reach.
///
/// # Errors
///
/// `E0635` for a `[cache.shared] url` carrying [`UNIX_SCHEME`] on a build with no `AF_UNIX`
/// transport. Never anything on a build that has one: there the spelling is ordinary, and what the
/// path itself must look like is the door's question (`nvs_stdlib::cache`).
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    if cfg!(unix) {
        return Ok(());
    }
    let Some(url) = configured(config).filter(|url| url.starts_with(UNIX_SCHEME)) else {
        return Ok(());
    };

    Err(Diagnostic::error(
        code::E_NO_UNIX_TRANSPORT,
        format!(
            "`[cache.shared] url = \"{url}\"` names a Unix-domain socket, and this build carries \
             no transport for one"
        ),
    )
    .with_note(format!(
        "`rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot`: the transport is \
         `#[cfg(unix)]` and this is a {} build, so the store named here would never be reached{}",
        std::env::consts::OS,
        origin_note(origins.get("cache.shared.url"))
    ))
    .with_help(
        "write the store as `redis://host[:port]`; a `unix:` url is not quietly read as loopback \
         TCP, because a configuration that reads as one transport and runs as another is invisible \
         in exactly the review that would have caught it"
            .to_string(),
    ))
}

/// The advisory a tree earns by configuring a store no capability may reach, or `None` when it
/// configured none or granted one.
///
/// Asked of the merged tree for [`crate::session::validate`]'s reason: which `url` and which grant
/// are in force is a question only the whole stream has answered, and a per-file check would warn
/// about a base file that an include was about to grant.
#[must_use]
pub fn advise(config: &Config, origins: &BTreeMap<String, Origin>) -> Option<Diagnostic> {
    let url = configured(config)?;

    let granted = config
        .capabilities
        .as_ref()
        .is_some_and(|caps| caps.allows_unscoped(Cap::CacheShared));
    if granted {
        return None;
    }

    Some(
        Diagnostic::warning(
            code::W_STORE_CONFIGURED_UNGRANTED,
            format!(
                "`[cache.shared] url = \"{url}\"` names a store, and `cache.shared` is not granted \
                 to anything that could reach it"
            ),
        )
        .with_note(format!(
            "`rule:config/cache-shared-is-the-grant-over-the-configured-store`: the grant over a \
             store an operator configured names the *store* and not its address, so \
             `Core\\Cache::shared()` and `Core\\RateLimit::consume` ask `cache.shared` and no \
             longer ask `net.connect` at this URL's host{}",
            origin_note(origins.get("cache.shared.url"))
        ))
        .with_help(
            "write `cache.shared = true` under `[capabilities]`; a deployment that had granted \
             `net.connect` for this store's host can drop that entry, and the `net.internal` \
             exception it needed for a loopback store with it"
                .to_string(),
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nvs_diagnostics::code;

    use super::{advise, validate};
    use crate::tree::{CacheShared, CapCache, Capabilities, Config, Setting};

    /// A tree whose `[cache.shared]` block names `url`, granted or not.
    fn wrote(url: Option<&str>, grant: Option<Setting>) -> Config {
        Config {
            cache: Some(crate::tree::Cache {
                shared: Some(CacheShared {
                    url: url.map(str::to_owned),
                    ..CacheShared::default()
                }),
                ..crate::tree::Cache::default()
            }),
            capabilities: grant.map(|shared| Capabilities {
                cache: Some(CapCache {
                    shared: Some(shared),
                }),
                ..Capabilities::default()
            }),
            ..Config::default()
        }
    }

    /// `rule:config/cache-shared-is-the-grant-over-the-configured-store`'s boot report, on both
    /// sides: a configured store with no grant is `W1008`, and the same store granted is silent.
    ///
    /// The grant is the migration this warning exists for — a deployment carrying `net.connect`
    /// for its store's host boots with the tier unreachable and nothing else says so — so a test
    /// asserting only the silent side would pass against a function that never warns at all.
    #[test]
    fn a_configured_url_without_its_grant_is_a_boot_warning() {
        let origins = BTreeMap::new();

        let warned = advise(&wrote(Some("redis://cache.internal"), None), &origins)
            .expect("a configured store with no grant warns");
        assert_eq!(warned.code, Some(code::W_STORE_CONFIGURED_UNGRANTED));
        assert!(
            warned.message.contains("redis://cache.internal"),
            "the message names the URL that was written: {}",
            warned.message
        );

        for granted in [Setting::Bool(true), Setting::List(vec!["yes".to_owned()])] {
            assert!(
                advise(
                    &wrote(Some("redis://cache.internal"), Some(granted.clone())),
                    &origins
                )
                .is_none(),
                "a granted store is not reported: {granted:?}"
            );
        }

        assert!(
            advise(
                &wrote(Some("redis://cache.internal"), Some(Setting::Bool(false))),
                &origins
            )
            .is_some(),
            "`cache.shared = false` grants nothing, so the pair is still worth naming"
        );
        assert!(
            advise(&wrote(None, None), &origins).is_none(),
            "a deployment that configured no store has no store to reach"
        );
    }

    /// `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot`, from whichever side
    /// of it this build is on: a `unix:` store is an ordinary one where there is a transport for it
    /// and `E0635` where there is none.
    ///
    /// Both halves in one case because what is being asserted is that they **disagree by
    /// platform**. A case written for one of them alone passes against a function that ignores the
    /// platform and answers that way everywhere, which is the mistake with a cost: reading the
    /// spelling as loopback TCP is the thing the rule refuses.
    #[test]
    fn a_unix_url_is_refused_at_boot_only_where_there_is_no_transport() {
        let origins = BTreeMap::new();
        let socket = wrote(Some("unix:/run/redis.sock"), Some(Setting::Bool(true)));

        let verdict = validate(&socket, &origins);
        if cfg!(unix) {
            assert!(
                verdict.is_ok(),
                "a socket is an ordinary store where the reactor carries one"
            );
        } else {
            let refused = verdict.expect_err("a socket with no transport is refused at the key");
            assert_eq!(refused.code, Some(code::E_NO_UNIX_TRANSPORT));
            assert!(
                refused.message.contains("unix:/run/redis.sock"),
                "the message names the url that was written: {}",
                refused.message
            );
            assert!(
                refused
                    .notes
                    .iter()
                    .any(|note| note.contains(std::env::consts::OS)),
                "the note names the platform, since that is what has to change: {:?}",
                refused.notes
            );
        }

        // Silent on every platform for the other spelling and for a tree that configured no store,
        // so the refusal is about the scheme rather than about the key being written at all.
        assert!(validate(&wrote(Some("redis://cache.internal"), None), &origins).is_ok());
        assert!(validate(&wrote(None, None), &origins).is_ok());
    }
}
