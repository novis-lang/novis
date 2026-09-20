//! `rule:core-api/core-means-always-present` as a gate:
//! nothing outside Tier 0 registers a class under the `Core` namespace.
//!
//! § 1's table is what makes this checkable. Tier 0 is "compiled into every
//! `nvs` binary, reachable under the `Core` namespace, **no build flag**";
//! Tier 2 (Native) is "removable at build time by a Cargo feature". So the
//! violation § 5 names — a `use Core\Intl;` that compiles in development and
//! fails in production — is not a *naming* mistake that a grep for the prefix
//! would find. It is a `Core\` name whose presence depends on how the binary
//! was built, and the two shapes it can take are the two this file walks: a
//! class registered from somewhere other than Tier 0's own crate, and a
//! registration a `#[cfg]` can compile out.
//!
//! **A `Core\`-prefixed name is not by itself a tier violation**, which is why
//! this gate does not look for one. `Core\Db`, `Core\Crypto` and
//! `Core\Http\Client` are all Native subsystems and all keep the prefix — the
//! split `rule:core-api/tier-roster` states for `Core\Metrics` is the general rule: the
//! *class* is Tier 0 and unconditional, and it is the backend behind it that a
//! Cargo feature may remove. What may never be conditional is whether the name
//! resolves.
//!
//! # The two rosters
//!
//! [`registry::CLASSES`] is one of them and not both. The other is
//! `nvs_hir::errors::TREE`, which carries `Core\Test\Failure`,
//! `Core\Cli\NotInteractive`, `Core\Db\DbError` and `Core\Db\RolledBack` —
//! `Core` classes a program names in a `catch` and that have no registry row,
//! for the reason that file's own module doc gives. A gate that read "in the
//! `Core` namespace" as "in the registry"
//! would leave them unguarded, so this reads both, and it reads the second
//! from source because `nvs-stdlib` does not depend on `nvs-hir` and must not
//! grow the edge to satisfy a test.
//!
//! Reading a sibling crate's source is what `loop-goal.toml` asks for by
//! putting this check in `cargo test -p nvs-stdlib`: the claim is about the
//! repository, not about this crate, and the other repository-wide gates here
//! (`conformance_coverage.rs`, `allocation_policy.rs`) are already scans of
//! the same shape.

use std::fs;
use std::path::{Path, PathBuf};

use nvs_stdlib::registry;

/// Every `.rs` file under `dir`, recursively, in no particular order.
fn rust_sources(dir: &Path, into: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            rust_sources(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
}

/// Every `crates/<name>/src/**/*.rs` in the repository, each with the crate it
/// belongs to.
fn crate_sources() -> Vec<(String, PathBuf)> {
    let crates = nvs_repo::path("crates");
    let entries = fs::read_dir(&crates).unwrap_or_else(|err| panic!("{}: {err}", crates.display()));
    let mut found = Vec::new();
    for entry in entries {
        let dir = entry.expect("a readable directory entry").path();
        let src = dir.join("src");
        if !src.is_dir() {
            continue;
        }
        let name = dir
            .file_name()
            .expect("a crate directory")
            .to_string_lossy()
            .into_owned();
        let mut files = Vec::new();
        rust_sources(&src, &mut files);
        found.extend(files.into_iter().map(|path| (name.clone(), path)));
    }
    assert!(
        found.len() > 100,
        "{} yielded only {} source file(s), which is too few to be the workspace — \
         a scan over it would pass vacuously",
        crates.display(),
        found.len()
    );
    found
}

/// The body of the `const` whose declaration `opening` opens, from that line to
/// the `];` that closes it in column 0.
fn const_body<'a>(text: &'a str, opening: &str, file: &Path) -> &'a str {
    let at = text
        .find(opening)
        .unwrap_or_else(|| panic!("{} no longer declares `{opening}`", file.display()));
    let rest = &text[at..];
    let end = rest
        .find("\n];")
        .unwrap_or_else(|| panic!("{}: `{opening}` is not closed by a `];`", file.display()));
    &rest[..end]
}

/// The attributes written directly above line `at` — the contiguous run of
/// `#[…]`, `///` and `//` lines above it, which is where a `#[cfg]` on the item
/// would sit.
fn attributes_above<'a>(lines: &[&'a str], at: usize) -> Vec<&'a str> {
    let mut found = Vec::new();
    for line in lines[..at].iter().rev() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[") || trimmed.starts_with("//") {
            found.push(*line);
        } else {
            break;
        }
    }
    found
}

/// `rule:core-api/core-means-always-present`: nothing outside Tier 0 registers a class under the `Core`
/// namespace, and nothing a build flag can remove is registered there at all.
///
/// Three claims, because "outside Tier 0" has three edges on a tree that has
/// no Tier 1 or Tier 2 component in it yet — and the third is the one that
/// stays true after the first such component lands, which is when this gate
/// starts earning its keep.
///
/// 1. **Tier 0's roster *is* the `Core` namespace.** Every class
///    [`registry::CLASSES`] declares names itself `Core\…`, so the two
///    following claims are total rather than a sample: there is no class in
///    the roster that the prefix rule does not reach.
/// 2. **Only Tier 0's own crate builds one.** A class reaches the compiler's
///    `Core` namespace by being a `CoreClass` in `nvs-stdlib`, and a
///    `CoreClass` literal anywhere else is a second door onto the namespace
///    regardless of what it names.
/// 3. **No registration is conditional.** Neither roster's entries, nor the
///    per-module `CLASS` const each entry points at, may carry a `#[cfg]`. A
///    `#[cfg(test)]` module *inside* a domain module is untouched by this —
///    the scan reads the attributes on the registration itself, not the file
///    it sits in, because a platform `#[cfg]` in a member's body is an
///    implementation detail and not a tier question.
#[test]
fn no_class_outside_tier_zero_registers_a_core_name() {
    let unprefixed: Vec<&str> = registry::CLASSES
        .iter()
        .map(|class| class.name)
        .filter(|name| !name.starts_with(r"Core\"))
        .collect();
    assert!(
        unprefixed.is_empty(),
        "{} class(es) in `registry::CLASSES` do not name themselves under `Core`: {}\n\
         Tier 0 is the `Core` namespace (`rule:core-api/five-placements` and `rule:core-api/core-means-always-present`), so a row here that is \
         named anything else is a class claiming Tier 0's unconditional presence \
         without its namespace.",
        unprefixed.len(),
        unprefixed.join(", ")
    );

    let sources = crate_sources();

    let mut doors: Vec<String> = Vec::new();
    let mut conditional: Vec<String> = Vec::new();
    for (krate, path) in &sources {
        let text =
            fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        if !text.contains("CoreClass {") {
            continue;
        }
        if krate != "nvs-stdlib" {
            doors.push(path.display().to_string());
            continue;
        }
        let lines: Vec<&str> = text.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            if !line.contains("const CLASS: CoreClass") {
                continue;
            }
            if attributes_above(&lines, at)
                .iter()
                .any(|above| above.trim_start().starts_with("#[cfg"))
            {
                conditional.push(format!("{}:{}", path.display(), at + 1));
            }
        }
    }
    assert!(
        doors.is_empty(),
        "{} file(s) outside `nvs-stdlib` build a `CoreClass`: {}\n\
         Tier 0's crate is the only one that may — `rule:core-api/core-means-always-present`. A subsystem at any \
         other tier is named under its own namespace, so that its tier is visible at \
         the use site.",
        doors.len(),
        doors.join(", ")
    );

    let registry_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/registry.rs");
    let text = fs::read_to_string(&registry_src)
        .unwrap_or_else(|err| panic!("{}: {err}", registry_src.display()));
    if const_body(&text, "pub const CLASSES: &[CoreClass] = &[", &registry_src).contains("#[cfg") {
        conditional.push(format!("{}'s `CLASSES`", registry_src.display()));
    }

    let tree_src = nvs_repo::path("crates/nvs-hir/src/errors.rs");
    let text =
        fs::read_to_string(&tree_src).unwrap_or_else(|err| panic!("{}: {err}", tree_src.display()));
    let tree = const_body(
        &text,
        "pub const TREE: &[(&str, Option<&str>)] = &[",
        &tree_src,
    );
    assert!(
        tree.contains(r#"("Core\\"#),
        "{} no longer carries a `Core\\`-named row, so the second roster this gate \
         reads has moved and the gate is reading nothing",
        tree_src.display()
    );
    if tree.contains("#[cfg") {
        conditional.push(format!("{}'s `TREE`", tree_src.display()));
    }

    assert!(
        conditional.is_empty(),
        "{} `Core` class registration(s) are conditional: {}\n\
         `rule:core-api/core-means-always-present`: a `Core\\` name whose presence depends on a build flag makes \
         `rule:classes/no-free-functions-or-constants`'s reserved namespace conditional, which is the failure that section \
         exists to refuse. The feature gate belongs on the backend behind the class, \
         the way `rule:core-api/tier-roster` splits `Core\\Metrics`.",
        conditional.len(),
        conditional.join(", ")
    );
}
