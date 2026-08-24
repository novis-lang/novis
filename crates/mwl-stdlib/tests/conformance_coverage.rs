//! The coverage gate `docs/agent/loop-goal.md`'s Stage 4 names: every `Core`
//! member this crate registers is called by at least one `.mwlt` case under
//! `tests/conformance/`.
//!
//! A registry row and an implementation together still prove nothing about
//! *behaviour* — `crates/mwl-stdlib/src/lib.rs` says a half-added member
//! cannot link, and this says a member cannot be added without a case that
//! runs it. It is the one check that reads across the crate boundary, which is
//! why it is an integration test rather than a `#[cfg(test)]` module: it needs
//! the repository, not the crate.
//!
//! **The enumerable set is what is registered, not what the spec owes.**
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) §§ 1-12
//! is the work list and `mwl-stdlib`'s known gap 1 tracks how much of it is on
//! disk; a member the registry does not hold is not yet a member, and the
//! compiler refuses to resolve a call to one, so there is nothing here to
//! check. What this forbids is the other order: registering a member, then
//! never writing the case.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use mwl_stdlib::registry;

/// Every `.mwlt` file under `dir`, recursively, in no particular order.
fn cases(dir: &Path, into: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            cases(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "mwlt") {
            into.push(path);
        }
    }
}

#[test]
fn every_part_one_member_has_a_conformance_case() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance");
    let mut paths = Vec::new();
    cases(&root, &mut paths);
    assert!(
        paths.len() > 100,
        "{} holds {} cases, which is too few to be the conformance suite — \
         this check would pass vacuously",
        root.display(),
        paths.len()
    );

    // The `--FILE--` section alone, so a member named in a title or in an
    // expected diagnostic is not mistaken for one a case calls.
    let mut source = String::new();
    for path in &paths {
        let text =
            fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        let case = mwl_test::case::parse(path, &text)
            .unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        source.push_str(&case.file);
        source.push('\n');
    }

    let mut uncovered = BTreeSet::new();
    for class in registry::CLASSES {
        for method in class.methods {
            // The call spelling, with the open parenthesis, so `Str::trim`
            // does not answer for `Str::trimStart`.
            let call = format!("{}::{}(", class.name, method.name);
            if !source.contains(&call) {
                uncovered.insert(call);
            }
        }
    }

    assert!(
        uncovered.is_empty(),
        "{} registered `Core` member(s) are never called by a conformance case: {}\n\
         Write one under tests/conformance/ — never with an `--ORACLE--` section, which \
         makes the Linux leg skip the case entirely.",
        uncovered.len(),
        uncovered
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
}
