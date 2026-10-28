//! What a change to the graph brings, printed by `nvs pkg add` and `nvs pkg update` before
//! anything is written (`rule:packaging/a-vendor-resolves-from-one-registry`).
//!
//! **For each package the command named**: the packages it needs, directly and below them, their
//! count, and every capability each one requests
//! (`rule:security/package-authority-is-granted-one-line-at-a-time`). **For the whole graph,
//! against the old lock**: each package whose version, requested capabilities, owners or
//! repository changed, each new package with what it requests, and each package no longer
//! needed. The lock records those keys so there is something to compare
//! (`rule:packaging/the-lockfile-holds-every-digest`). **Every major crossed**, with the packages
//! that asked for the older version (`rule:packaging/resolution-takes-the-highest-minimum`).
//!
//! The report grants nothing: a new package's grant line is written empty, and an existing line
//! is never changed, so a capability a package newly requests stays refused until a person adds
//! it.
//!
//! Cost: the text, built once per command. Nothing here runs on the request path.

use std::collections::{BTreeMap, BTreeSet};

use super::index::Capabilities;
use super::lock::{self, Lock, Locked};
use super::manifest::Name;
use super::resolve::{Graph, Selected};

/// The report of `graph` against the `old` lock, for the packages `targets` names.
pub(crate) fn report(graph: &Graph, old: Option<&Lock>, targets: &[Name]) -> String {
    let by_slug: BTreeMap<String, &Selected> = graph
        .packages
        .iter()
        .map(|selected| (selected.name.slug(), selected))
        .collect();
    let mut out = String::new();
    for target in targets {
        let Some(&root) = by_slug.get(&target.slug()) else {
            continue;
        };
        let below = subgraph(root, &by_slug);
        out.push_str(&format!(
            "`{}` {} needs {} other package(s). Each one requests:\n",
            root.name,
            root.release.version,
            below.len()
        ));
        for selected in std::iter::once(root).chain(below) {
            out.push_str(&format!(
                "  `{}` {}: {}\n",
                selected.name,
                selected.release.version,
                requests(&selected.release.capabilities)
            ));
        }
    }
    changes(&mut out, graph, old);
    if !graph.crossings.is_empty() {
        out.push_str("Majors crossed:\n");
        for crossing in &graph.crossings {
            out.push_str(&format!("  {crossing}\n"));
        }
    }
    out
}

/// Every package `root` needs, directly and below, in slug order, without `root`.
fn subgraph<'a>(root: &Selected, by_slug: &BTreeMap<String, &'a Selected>) -> Vec<&'a Selected> {
    let mut seen = BTreeSet::new();
    let mut queue: Vec<&Name> = root.release.dependencies.iter().map(|(n, _)| n).collect();
    while let Some(name) = queue.pop() {
        let slug = name.slug();
        if slug == root.name.slug() || !seen.insert(slug.clone()) {
            continue;
        }
        if let Some(selected) = by_slug.get(&slug) {
            queue.extend(selected.release.dependencies.iter().map(|(n, _)| n));
        }
    }
    seen.iter()
        .filter_map(|slug| by_slug.get(slug).copied())
        .collect()
}

/// What `capabilities` requests, in one line.
fn requests(capabilities: &Capabilities) -> String {
    let mut parts = Vec::new();
    if !capabilities.required.is_empty() {
        parts.push(format!("required {}", capabilities.required.join(", ")));
    }
    if !capabilities.optional.is_empty() {
        parts.push(format!("optional {}", capabilities.optional.join(", ")));
    }
    if parts.is_empty() {
        "no capabilities".to_owned()
    } else {
        parts.join("; ")
    }
}

/// Each change of `graph` since `old`, added to `out`.
fn changes(out: &mut String, graph: &Graph, old: Option<&Lock>) {
    let empty = Lock::default();
    let mut lines = Vec::new();
    for selected in &graph.packages {
        let (name, release) = (&selected.name, &selected.release);
        let Some(locked) = old.unwrap_or(&empty).get(name) else {
            lines.push(format!(
                "  `{name}` {} is new. It requests {}.",
                release.version,
                requests(&release.capabilities)
            ));
            continue;
        };
        let details = differences(locked, selected);
        if locked.version == release.version && details.is_empty() {
            continue;
        }
        if locked.version == release.version {
            lines.push(format!("  `{name}` {}", release.version));
        } else {
            lines.push(format!(
                "  `{name}` {} -> {}",
                locked.version, release.version
            ));
        }
        lines.extend(details.into_iter().map(|line| format!("    {line}")));
    }
    for locked in old.map_or(&[][..], |old| &old.packages) {
        let slug = locked.name.slug();
        if !graph.packages.iter().any(|s| s.name.slug() == slug) {
            lines.push(format!(
                "  `{}` {} is no longer needed.",
                locked.name, locked.version
            ));
        }
    }
    if old.is_none() {
        out.push_str(&format!(
            "There is no {} yet, so every package is new:\n",
            lock::FILE
        ));
    } else if lines.is_empty() {
        out.push_str(&format!("No package changed since {}.\n", lock::FILE));
        return;
    } else {
        out.push_str(&format!("Changes since {}:\n", lock::FILE));
    }
    for line in lines {
        out.push_str(&line);
        out.push('\n');
    }
}

/// Each way `selected` differs from its `locked` entry, except its version: the capabilities it
/// requests, its owners and its repository.
fn differences(locked: &Locked, selected: &Selected) -> Vec<String> {
    let (old, new) = (&locked.capabilities, &selected.release.capabilities);
    let mut lines = Vec::new();
    for (kind, old, new) in [
        ("required", &old.required, &new.required),
        ("optional", &old.optional, &new.optional),
    ] {
        for capability in new.iter().filter(|c| !old.contains(c)) {
            lines.push(format!("now requests {capability} ({kind})"));
        }
        for capability in old.iter().filter(|c| !new.contains(c)) {
            lines.push(format!("no longer requests {capability} ({kind})"));
        }
    }
    if locked.owners != selected.owners {
        lines.push(format!(
            "owners were {} and are now {}",
            listed(&locked.owners),
            listed(&selected.owners)
        ));
    }
    if locked.repository != selected.release.repository {
        lines.push(format!(
            "repository was {} and is now {}",
            locked.repository, selected.release.repository
        ));
    }
    lines
}

/// `names` joined with commas, or `nobody`.
fn listed(names: &[String]) -> String {
    if names.is_empty() {
        "nobody".to_owned()
    } else {
        names.join(", ")
    }
}
