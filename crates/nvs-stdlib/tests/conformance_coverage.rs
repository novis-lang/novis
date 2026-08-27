//! The coverage gate `docs/agent/loop-goal.md`'s Stage 4 names: every `Core`
//! member this crate registers is called by at least one `.nvst` case under
//! `tests/conformance/`.
//!
//! A registry row and an implementation together still prove nothing about
//! *behaviour* — `crates/nvs-stdlib/src/lib.rs` says a half-added member
//! cannot link, and this says a member cannot be added without a case that
//! runs it. It is the one check that reads across the crate boundary, which is
//! why it is an integration test rather than a `#[cfg(test)]` module: it needs
//! the repository, not the crate.
//!
//! **The enumerable set is what is registered, not what the spec owes.**
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) §§ 1-12
//! is the work list and `nvs-stdlib`'s known gap 1 tracks how much of it is on
//! disk; a member the registry does not hold is not yet a member, and the
//! compiler refuses to resolve a call to one, so there is nothing here to
//! check. What this forbids is the other order: registering a member, then
//! never writing the case.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use nvs_stdlib::registry;

/// Every `.nvst` file under `dir`, recursively, in no particular order.
fn cases(dir: &Path, into: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            cases(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "nvst") {
            into.push(path);
        }
    }
}

/// Whether `haystack` writes `needle` as a whole name — the same match
/// `haystack.contains(needle)` performs, plus the one boundary an identifier
/// needs on its right.
fn mentions(haystack: &str, needle: &str) -> bool {
    haystack.match_indices(needle).any(|(at, _)| {
        haystack[at + needle.len()..]
            .chars()
            .next()
            .is_none_or(|next| !next.is_alphanumeric() && next != '_')
    })
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
        let case = nvs_test::case::parse(path, &text)
            .unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        source.push_str(&case.file);
        source.push('\n');
    }

    let mut uncovered = BTreeSet::new();
    for class in registry::CLASSES {
        for method in class.methods {
            // The call spelling, with the open parenthesis, so `Str::trim`
            // does not answer for `Str::trimStart`. A member declaring a
            // written type parameter is spelled `decodeAs<User>(` at every
            // call site, so its own boundary is the `<`.
            let opener = if method.written().is_empty() {
                '('
            } else {
                '<'
            };
            let call = format!("{}::{}{opener}", class.name, method.name);
            if !source.contains(&call) {
                uncovered.insert(call);
            }
        }
        for method in class.instance {
            // An instance member is reached through a value, so what a case
            // writes is `->name(` and the class name appears nowhere. That is
            // weaker than the static spelling above — any receiver's `->text(`
            // answers for `Core\Regex\Match::text` — and deliberately so: the
            // alternative is inferring a receiver's type here, which would
            // mean a second checker rather than a coverage gate.
            let call = format!("->{}(", method.name);
            if !source.contains(&call) {
                uncovered.insert(format!("{}::{}", class.name, method.name));
            }
        }
        for declared in class.constants {
            // A constant has no parenthesis to bound it, so the boundary is
            // checked instead — otherwise `Core\Math::E` would be covered by
            // any case that writes `Core\Math::EPSILON`.
            let write = format!("{}::{}", class.name, declared.name);
            if !mentions(&source, &write) {
                uncovered.insert(write);
            }
        }
    }

    assert!(
        uncovered.is_empty(),
        "{} registered `Core` member(s) are never used by a conformance case: {}\n\
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
