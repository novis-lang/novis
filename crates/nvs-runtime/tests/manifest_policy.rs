//! Workspace-manifest settings that are a safety policy rather than a tuning
//! choice.
//!
//! A profile flag leaves no trace a test could read back — `cargo test` builds
//! the dev profile, so a release-only setting is invisible from inside the
//! process — and the setting itself lives in a file no crate compiles. So the
//! policy is pinned at its source, exactly as `nvs-codegen`'s `backend_policy`
//! pins the Cranelift flags. Crude, and the only thing that actually fails when
//! the line is deleted.
//!
//! This lives in `nvs-runtime` because this crate is where a wrapped size
//! computation does its damage: every allocation length, refcount and index the
//! heap representation computes passes through here.
//!
//! The dependency-graph half of the file is here for the same reason and pinned
//! the same way: `rule:concurrency/one-scheduler`'s "one scheduler" is a property of the *graph*, which
//! no compiled artefact records either, so it is read back off `Cargo.toml` and
//! `Cargo.lock` or it is not checked at all.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// This repository's root — the directory holding the workspace manifest.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Read a file this test knows must exist, or fail naming it.
fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

/// The workspace manifest's `[profile.release]` block, comments stripped.
///
/// Stripping the comments is what keeps the assertion honest: the block's own
/// prose names `overflow-checks` several times while explaining why it is on,
/// so a search over the raw text would still pass with the setting deleted.
fn release_profile() -> String {
    let path = workspace_root().join("Cargo.toml");
    let text = read(&path);

    let mut lines = text
        .lines()
        .skip_while(|line| line.trim() != "[profile.release]");
    assert!(
        lines.next().is_some(),
        "the workspace manifest has no `[profile.release]` section at all"
    );

    lines
        .take_while(|line| !line.trim_start().starts_with('['))
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_release_profile_checks_integer_overflow() {
    // Cargo's own default for a release profile is `overflow-checks = false`,
    // so this is a setting that has to be present rather than one that has to
    // be absent — deleting the line reverts to silent wraparound with nothing
    // reporting it. What it buys, and the measurement saying it costs nothing,
    // are in the manifest comment sitting directly above the line.
    assert!(
        release_profile()
            .lines()
            .any(|line| line.trim() == "overflow-checks = true"),
        "`[profile.release]` no longer sets `overflow-checks = true`. Cargo's default for a \
         release profile is off, so a size or index computation that wraps becomes a wrong \
         length nothing reports rather than a panic `rule:errors/propagation` contains to one request. If this \
         is deliberate, the reasoning belongs beside the setting in the workspace manifest."
    );
}

/// Every `Cargo.toml` this repository owns, as `(path, line number, code)` with
/// the comments cut away.
///
/// The cut is the whole point here too, and more so than above: the workspace
/// manifest argues about `tokio` at length in the comment over the `hyper` line,
/// and `crates/nvs-server/Cargo.toml` names it again explaining what
/// `default-features = false` is for. A grep over the raw text would report both
/// and could never report a real dependency, so what is searched is the code.
fn manifest_code() -> Vec<(PathBuf, usize, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if path.is_dir() {
                // `target` is every dependency's vendored manifest and `.git`
                // holds none, so neither is ours to have an opinion about.
                if name != "target" && name != ".git" {
                    walk(&path, out);
                }
            } else if name == "Cargo.toml" {
                out.push(path);
            }
        }
    }

    let mut paths = Vec::new();
    walk(&workspace_root(), &mut paths);
    paths.sort();
    assert!(
        paths.len() > 10,
        "found only {} manifest(s) under {} — this walk stopped finding the crates rather than \
         the tree losing them",
        paths.len(),
        workspace_root().display()
    );

    paths
        .into_iter()
        .flat_map(|path| {
            let text = read(&path);
            text.lines()
                .enumerate()
                .map(|(index, line)| {
                    let code = line.split('#').next().unwrap_or("").trim().to_string();
                    (path.clone(), index + 1, code)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The lock file's packages, each as its name and the names it resolved to.
///
/// A lock entry records no features, so the feature set is read from its shape:
/// a dependency a feature gates is present exactly when that feature is on, and
/// absent otherwise. That indirection is what makes the assertion below possible
/// at all, and it is also why the assertion is written over `tokio`'s *own*
/// dependency list rather than over a feature string that is not in the file.
fn locked_packages() -> Vec<(String, Vec<String>)> {
    let path = workspace_root().join("Cargo.lock");
    let text = read(&path);

    text.split("[[package]]")
        .skip(1)
        .map(|block| {
            let mut name = None;
            let mut dependencies = Vec::new();
            let mut in_dependencies = false;
            for line in block.lines().map(str::trim) {
                if in_dependencies {
                    if line == "]" {
                        in_dependencies = false;
                    } else {
                        // A duplicated crate is listed as `"name version"`, so
                        // the name is the first word rather than the whole entry.
                        let entry = line.trim_end_matches(',').trim_matches('"');
                        dependencies
                            .push(entry.split_whitespace().next().unwrap_or(entry).to_string());
                    }
                } else if line == "dependencies = [" {
                    in_dependencies = true;
                } else if let Some(rest) = line.strip_prefix("name = ") {
                    name.get_or_insert_with(|| rest.trim_matches('"').to_string());
                }
            }
            (
                name.expect("a `[[package]]` block with no `name`"),
                dependencies,
            )
        })
        .collect()
}

#[test]
fn tokio_appears_in_neither_the_manifest_nor_the_lockfile() {
    // **The name reads wider than what is asserted, deliberately.** `tokio` is
    // in the lock file and has been since `hyper` 1.11, which depends on it
    // unconditionally at `features = ["sync"]` for one `oneshot` on the h1
    // server path. Renaming this check was the alternative; keeping the name and
    // making the assertion *stronger* than the name is what `rule:concurrency/one-scheduler` actually
    // wants pinned, because "the string is absent" was only ever a proxy for
    // "this binary has one scheduler and it is `nvs-host`'s". So what is checked
    // is that no crate of ours depends on `tokio`, that the graph's only route
    // to it is `hyper`'s, and that what is compiled of it is synchronisation and
    // nothing else. The workspace manifest's comment above the `hyper` line is
    // where that trade is argued, including what pinning `hyper` backwards to
    // buy the name out would have cost.
    let named: Vec<String> = manifest_code()
        .into_iter()
        .filter(|(_, _, code)| code.contains("tokio"))
        .map(|(path, line, code)| format!("{}:{line}: {code}", path.display()))
        .collect();
    assert!(
        named.is_empty(),
        "a manifest in this repository names `tokio` outside a comment:\n  {}\nNo crate of ours \
         takes an async runtime as a dependency — `rule:concurrency/one-scheduler`'s scheduler is `nvs-host`'s and there \
         is exactly one. If this is `hyper`'s transitive `tokio` being pinned or patched, the \
         reasoning belongs beside the line in the manifest.",
        named.join("\n  ")
    );

    let packages = locked_packages();

    let tokios: Vec<&str> = packages
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| *name == "tokio" || name.starts_with("tokio-"))
        .collect();
    assert_eq!(
        tokios,
        ["tokio"],
        "the lock file's `tokio` family is no longer `tokio` alone. `tokio-util`, `tokio-macros` \
         or `tokio-stream` beside it means a runtime feature was turned on somewhere, because \
         nothing `hyper` compiles under `sync` reaches any of them."
    );

    let dependents: BTreeSet<&str> = packages
        .iter()
        .filter(|(_, dependencies)| dependencies.iter().any(|dep| dep == "tokio"))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        dependents.iter().copied().collect::<Vec<_>>(),
        ["hyper"],
        "the lock file's route to `tokio` is no longer `hyper`'s alone. Every other dependency in \
         this tree was picked under ADR 0051 § 4 with an async runtime as a disqualifier, so a \
         second dependent is a dependency to reconsider rather than a line to add here."
    );

    let compiled: Vec<&str> = packages
        .iter()
        .find(|(name, _)| name == "tokio")
        .expect("checked above")
        .1
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(
        compiled,
        ["pin-project-lite"],
        "`tokio`'s own resolved dependencies changed. Under `features = [\"sync\"]` — tokio's \
         default feature set is empty and `hyper` asks for nothing else — the list is \
         `pin-project-lite` and nothing more. `mio`, `socket2`, `signal-hook-registry`, \
         `parking_lot` or `libc` appearing here is `rt`/`net`/`time` being compiled, which is a \
         second scheduler in this binary and what `rule:concurrency/one-scheduler` refuses. A `sync`-only addition in a \
         patch release is a real answer too — say which it is beside the name."
    );
}
