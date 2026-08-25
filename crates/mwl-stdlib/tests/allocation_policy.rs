//! Every count-shaped argument goes through one check, and only one.
//!
//! The guard this pins was four hand-written copies of the same three lines,
//! and the members with the largest appetite had none at all — which is the
//! failure mode the test exists for. A copy is easy to add and impossible to
//! notice, so the invariant is checked at the source rather than left to
//! review: `mwl_runtime::affordable` is the only place a size becomes a
//! refusal, and it is where `[limits.hard]` attaches when the M6 arena carries
//! it (ADR 0004).

use std::fs;
use std::path::Path;

/// Every `.rs` file under this crate's `src/`, as `(name, contents)`.
fn sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display())) {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let name = path
                .file_name()
                .expect("a file with an extension has a name")
                .to_string_lossy()
                .into_owned();
            let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{name}: {err}"));
            out.push((name, text));
        }
    }
    assert!(!out.is_empty(), "the stdlib source directory is not empty");
    out
}

#[test]
fn no_member_writes_its_own_allocation_guard() {
    // `isize::try_from(...)` on a size is the shape every one of the four
    // copies had. The point is not the spelling — it is that a member which
    // needs this check reaches for the shared one, so that the day a budget
    // replaces the overflow test, every member gets it at once.
    let offenders: Vec<String> = sources()
        .into_iter()
        .filter(|(_, text)| text.contains("isize::try_from"))
        .map(|(name, _)| name)
        .collect();

    assert!(
        offenders.is_empty(),
        "these files hand-write an allocation guard instead of calling \
         `mwl_runtime::affordable`: {offenders:?}. That function is the one seam the per-request \
         ceiling attaches to; a private copy silently opts its member out of it."
    );
}
