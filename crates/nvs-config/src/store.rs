//! `rule:config/cache-shared-is-the-grant-over-the-configured-store`'s boot half: what a
//! configuration already says about the coherent tier before any request asks for it.
//!
//! One question today, and it is a *pair* rather than a value: `[cache.shared] url` names a store
//! and `[capabilities] cache.shared` says who may reach one, and a tree holding the first without
//! the second describes a store nothing can open. Neither key is wrong on its own, so this is
//! [`W1008`](nvs_diagnostics::code::W_STORE_CONFIGURED_UNGRANTED) and never a refusal — the code's
//! own doc is the home of that reasoning.
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

/// The advisory a tree earns by configuring a store no capability may reach, or `None` when it
/// configured none or granted one.
///
/// Asked of the merged tree for [`crate::session::validate`]'s reason: which `url` and which grant
/// are in force is a question only the whole stream has answered, and a per-file check would warn
/// about a base file that an include was about to grant.
#[must_use]
pub fn advise(config: &Config, origins: &BTreeMap<String, Origin>) -> Option<Diagnostic> {
    let url = config
        .cache
        .as_ref()?
        .shared
        .as_ref()?
        .url
        .as_deref()
        .filter(|url| !url.is_empty())?;

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

    use super::advise;
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
}
