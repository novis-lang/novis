//! A reload: the tree the files hold now, published over the one a process is serving, and
//! `rule:config/a-reload-names-what-it-could-not-apply`'s report of what that changed.
//!
//! **There is one reload function, and it is [`reload`].** A running server calls it when its own
//! check sees a saved configuration file, and nothing else starts a reload. Resolving the tree
//! again is the caller's, because the roots it booted on and the unit
//! cache it holds are the caller's; what is here is the publish and the comparison that names what
//! the publish did.
//!
//! Cost, as `rule:programs/memory-priority` requires: two flattened key maps for the comparison,
//! built and dropped inside the call. Nothing per request.

use std::collections::BTreeMap;
use std::path::PathBuf;

use nvs_diagnostics::Diagnostic;

use crate::resolve::Files;
use crate::snapshot::{AppBlocks, Current, Snapshot};

/// `rule:config/a-reload-names-what-it-could-not-apply`'s answer to a reload: what it did, and what it could not do.
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The dotted keys the swap changed and which are now in force, in dotted-key order.
    pub applied: Vec<String>,
    /// The `Boot` keys whose written value changed and which therefore did **not** take effect,
    /// each named individually. § 5's own paragraph is why they are named rather than counted: a
    /// deployment that silently ignores a changed listen address believes it applied a change it
    /// did not. A reloadable key whose new resource could not be built is named here too
    /// ([`reload`]'s `keep`).
    pub ignored: Vec<&'static str>,
    /// How many compiled units the swap invalidated, so an operator knows a recompile wave is
    /// coming. § 4's rule is what decides it: a changed `env_hash` rekeys **every** unit and an
    /// unchanged one rekeys none, because a unit key carries the hash whole.
    pub invalidated: usize,
}

/// Publishes the set built from `next` and `blocks` over what `current` is serving, and reports
/// § 5's three answers.
///
/// `next` is the host's snapshot and `blocks` the `[[app]]` roster each entry's own snapshot is
/// folded from. Every file in `entries` is folded before anything is swapped, so a block that does
/// not fold for one of them refuses the whole reload ([`Current::publish_set`]).
///
/// `held` is how many compiled units the caller has, which is the only half of § 5's third answer
/// this module cannot know: the unit cache is `nvs-cli`'s.
///
/// `keep` names the reloadable keys whose new resource the caller could not build. Each keeps
/// its running value and is reported in [`Report::ignored`], as [`Current::publish_keeping`]
/// says.
///
/// # Errors
///
/// Whatever [`Current::publish_set`] refuses — `E0601` for a tree that does not deserialize once
/// the running `Boot` values are carried into it, or a block that does not fold for one of
/// `entries`. The previous set is still serving in that case, because publishing is the last step
/// and it never ran.
pub fn reload(
    current: &Current,
    next: Snapshot,
    blocks: AppBlocks,
    entries: &[PathBuf],
    files: &dyn Files,
    held: usize,
    keep: &[&str],
) -> Result<Report, Diagnostic> {
    let before = current.load();
    let published = current.publish_set(next, blocks, entries, files, keep)?;
    // The comparison is against the *published* table and not the submitted one, which is what
    // makes a changed `Boot` key absent from `applied` rather than present in both lists: publishing
    // carries the running value back over it, so by this line the two tables agree about it again.
    let (was, now) = (leaves(&before), leaves(&published.snapshot));
    let mut applied: Vec<String> = now
        .iter()
        .filter(|(key, value)| was.get(*key) != Some(*value))
        .map(|(key, _)| key.clone())
        .chain(
            was.keys()
                .filter(|key| !now.contains_key(*key))
                .map(Clone::clone),
        )
        .collect();
    applied.sort();
    applied.dedup();
    let (was_env, now_env) = (
        crate::cache::env_hash(&before.config),
        crate::cache::env_hash(&published.snapshot.config),
    );
    let invalidated =
        usize::from(was_env.digest().as_bytes() != now_env.digest().as_bytes()) * held;
    Ok(Report {
        applied,
        ignored: published.boot.iter().map(|row| row.key).collect(),
        invalidated,
    })
}

/// Every leaf in `snapshot`'s table by its dotted key — the shape a key-by-key diff needs and the
/// typed tree cannot be turned back into, which is the same reason [`Snapshot::table`] is kept at
/// all. The `[[app]]` roster is one more leaf, `app`, because the table does not carry it: an array
/// is a leaf wherever it is written, so this is the key the roster would have had there.
fn leaves(snapshot: &Snapshot) -> BTreeMap<String, toml::Value> {
    fn walk(table: &toml::Table, prefix: &str, into: &mut BTreeMap<String, toml::Value>) {
        for (key, value) in table {
            let dotted = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            match value.as_table() {
                Some(inner) => walk(inner, &dotted, into),
                None => drop(into.insert(dotted, value.clone())),
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(&snapshot.table, "", &mut out);
    if let Some(roster) = &snapshot.roster {
        nvs_footprint::every_app();
        out.insert("app".to_string(), roster.clone());
    }
    out
}
