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
    let path = nvs_repo::path("Cargo.toml");
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

    // The whole tree, because a manifest may sit in any directory of it.
    let root = nvs_repo::root();
    let mut paths = Vec::new();
    walk(&root, &mut paths);
    paths.sort();
    assert!(
        paths.len() > 10,
        "found only {} manifest(s) under {} — this walk stopped finding the crates rather than \
         the tree losing them",
        paths.len(),
        root.display()
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
    let path = nvs_repo::path("Cargo.lock");
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
    // in the lock file: `hyper` depends on it unconditionally at
    // `features = ["sync"]` for one `oneshot` on the h1 server path. The name
    // stays and the assertion is *stronger* than it, because `rule:concurrency/one-scheduler` wants
    // "this binary has one scheduler and it is `nvs-host`'s" pinned, and "the
    // string is absent" is only a proxy for that. So what is checked
    // is that no crate of ours depends on `tokio`, that the graph's only route
    // to it is `hyper`'s, and that what is compiled of it is synchronisation and
    // nothing else. The workspace manifest's comment above the `hyper` line is
    // where that trade is argued, including what pinning `hyper` backwards to
    // buy the name out would cost.
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
         this tree was picked under `rule:packaging/a-c-dependency-answers-two-questions` with an async runtime as a disqualifier, so a \
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

/// Every crate that brings a task scheduler with it, by its crates.io name.
///
/// This is the *scheduler* half of the async ecosystem and deliberately not the
/// `Future` half: `futures-core`, `futures-util` and `async-trait` are traits
/// and combinators, they run nothing, and `wasmtime` already resolves two of
/// them. `futures-executor` is on the list because it is the one crate in that
/// family that does run tasks.
const ASYNC_RUNTIMES: &[&str] = &[
    "actix-rt",
    "async-executor",
    "async-global-executor",
    "async-io",
    "async-std",
    "compio",
    "embassy-executor",
    "futures-executor",
    "glommio",
    "may",
    "monoio",
    "smol",
    "tokio",
];

/// Whether a package name is one of those or a crate in one's family, since
/// `tokio-util` and `async-std-*` are the same dependency under a longer name.
fn is_async_runtime(name: &str) -> bool {
    ASYNC_RUNTIMES.iter().any(|runtime| {
        name == *runtime
            || name
                .strip_prefix(runtime)
                .is_some_and(|rest| rest.starts_with('-'))
    })
}

/// Whether a manifest section's entries are dependencies.
///
/// `[dev-dependencies]` and `[target.'cfg(unix)'.dependencies]` are as much a
/// route into the binary as `[dependencies]` is, and `[patch.crates-io]` is one
/// that names a crate without depending on it.
fn declares_dependencies(section: &str) -> bool {
    section
        .rsplit('.')
        .next()
        .unwrap_or(section)
        .ends_with("dependencies")
        || section.starts_with("patch.")
        || section == "replace"
}

#[test]
fn the_workspace_has_no_async_runtime() {
    // The test above is `tokio` in depth, because `tokio` is the name actually
    // in this graph. This one is the family, and it is what
    // `rule:concurrency/one-scheduler` asks for: what that rule refuses is a
    // *second scheduler*, not one crate's name, so a move to `smol` or
    // `glommio` would satisfy every assertion above while taking away the whole
    // thing they protect. Both halves are needed — a manifest of ours asking
    // for one is the line to catch first, and the lock file is where a rename
    // or a transitive edge reaches the binary with no line to grep.
    let manifests = manifest_code();
    let mut ours: BTreeSet<String> = BTreeSet::new();
    let mut asked: Vec<String> = Vec::new();

    let mut file = PathBuf::new();
    let mut section = String::new();
    for (path, line, code) in &manifests {
        if *path != file {
            file.clone_from(path);
            section.clear();
        }
        let at = format!("{}:{line}: {code}", path.display());

        if code.starts_with('[') && code.ends_with(']') {
            section = code
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string();
            // `[dependencies.tokio]` names its dependency in the header itself.
            if let Some((outer, name)) = section.rsplit_once('.')
                && declares_dependencies(outer)
                && is_async_runtime(name)
            {
                asked.push(at);
            }
            continue;
        }

        let Some((key, value)) = code.split_once('=') else {
            continue;
        };
        if section == "package" && key.trim() == "name" {
            ours.insert(value.trim().trim_matches('"').to_string());
        }
        // `tokio = "1"` and `tokio.workspace = true` are the same request.
        let named = key.trim().split('.').next().unwrap_or_default();
        if declares_dependencies(&section) && is_async_runtime(named) {
            asked.push(at.clone());
        }
        // A rename keeps the crate's real name in a `package = "…"` value, so
        // every quoted word on a line carrying one is read as a name too.
        if code.contains("package") && code.split('"').skip(1).step_by(2).any(is_async_runtime) {
            asked.push(at);
        }
    }

    assert!(
        asked.is_empty(),
        "a manifest in this repository asks for an async runtime:\n  {}\nThis binary has one \
         scheduler and it is `nvs-host`'s coroutines — `rule:concurrency/one-scheduler`. A crate \
         that brings an executor, a reactor or a `spawn` with it brings a second concurrency model \
         beside them, so a capability that genuinely needs one is a `BLOCKED` naming the \
         capability rather than a name added to `ASYNC_RUNTIMES`' exceptions.",
        asked.join("\n  ")
    );
    assert!(
        ours.len() > 10,
        "found only {} `[package] name` line(s) across this repository's manifests — this walk \
         stopped reading them rather than the tree losing its crates",
        ours.len()
    );

    let packages = locked_packages();

    let linked: Vec<&str> = packages
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| is_async_runtime(name))
        .collect();
    assert_eq!(
        linked,
        ["tokio"],
        "the lock file resolves an async runtime beside `hyper`'s `tokio`, which the test above \
         argues line by line: under `sync` alone it is a channel library rather than a runtime. A \
         second name here is a second scheduler in the binary whichever crate asked for it, so \
         what changes is that dependency."
    );

    let members = packages
        .iter()
        .filter(|(name, _)| ours.contains(name))
        .count();
    assert!(
        members > 10,
        "only {members} of this repository's crates are in the lock file. `benches/abi-probe` and \
         `fuzz/` are their own workspaces and are expected to be missing; the rest are not, and \
         without them the next assertion checks nothing."
    );

    let reached: Vec<String> = packages
        .iter()
        .filter(|(name, _)| ours.contains(name))
        .flat_map(|(name, dependencies)| {
            dependencies
                .iter()
                .filter(|dependency| is_async_runtime(dependency))
                .map(move |dependency| format!("{name} -> {dependency}"))
        })
        .collect();
    assert!(
        reached.is_empty(),
        "a crate of ours resolved to an async runtime in the lock file:\n  {}\nThe manifest walk \
         above missed it, which means it arrived under a spelling that walk does not read — a \
         rename, or a table it does not treat as a dependency table. Fix the dependency first and \
         then teach `declares_dependencies` the spelling, so the cheap half keeps catching this.",
        reached.join("\n  ")
    );
}

/// A manifest's `[patch.crates-io]` rows, each as its crate name and the
/// directory its `path` resolves to.
fn patched(path: &Path, code: &[(PathBuf, usize, String)]) -> BTreeSet<(String, PathBuf)> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut section = "";
    let mut rows = BTreeSet::new();
    for (file, _, line) in code.iter().filter(|(file, _, _)| file == path) {
        if line.starts_with('[') {
            section = line.as_str();
            continue;
        }
        if section != "[patch.crates-io]" {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let target = value
            .split("path")
            .nth(1)
            .and_then(|rest| rest.split('"').nth(1))
            .unwrap_or_else(|| {
                panic!("{}: `{line}` is a patch row with no `path`", file.display())
            });
        let resolved = fs::canonicalize(dir.join(target))
            .unwrap_or_else(|err| panic!("{}: `{target}`: {err}", file.display()));
        rows.insert((name.trim().to_string(), resolved));
    }
    rows
}

#[test]
fn every_workspace_that_builds_our_crates_carries_the_root_patch_table() {
    // Cargo reads the `[patch]` table of the workspace being built and no
    // other, so `fuzz/` building `nvs-stdlib` against the published
    // `html5ever` compiles nothing that `crates/nvs-stdlib/src/html.rs` sets on
    // the patched one. The build fails, or worse, a patch that only changes
    // behaviour is silently absent from what the fuzzer runs.
    let code = manifest_code();
    let root = nvs_repo::path("Cargo.toml");
    let wanted = patched(&root, &code);
    assert!(
        !wanted.is_empty(),
        "the workspace manifest has no `[patch.crates-io]` rows, so this test checks nothing"
    );

    let manifests: BTreeSet<&PathBuf> = code.iter().map(|(path, _, _)| path).collect();
    let mut checked = 0;
    for path in manifests {
        let lines = code.iter().filter(|(file, _, _)| file == path);
        let own_workspace = lines.clone().any(|(_, _, line)| line == "[workspace]");
        let builds_ours = lines
            .clone()
            .any(|(_, _, line)| line.contains("path") && line.contains("crates/nvs-"));
        if *path == root || !own_workspace || !builds_ours {
            continue;
        }
        checked += 1;
        let missing: Vec<String> = wanted
            .difference(&patched(path, &code))
            .map(|(name, dir)| format!("{name} = {}", dir.display()))
            .collect();
        assert!(
            missing.is_empty(),
            "{} is its own workspace and builds this repository's crates, but its \
             `[patch.crates-io]` table lacks the root one's rows:\n  {}\nCopy each row, with its \
             `path` written relative to that manifest.",
            path.display(),
            missing.join("\n  ")
        );
    }
    assert!(
        checked > 0,
        "no separate workspace builds this repository's crates, so this test checks nothing. \
         `fuzz/` is one; if it moved, the walk above stopped finding it."
    );
}
