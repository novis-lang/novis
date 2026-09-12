//! The fixture pairs under `tests/fmt/`: a file as somebody wrote it beside the
//! one layout `nvs fmt` writes for it.
//!
//! The pairs are where a rule is exercised over a whole file rather than over
//! the one construct its own case is about, and they are the half of this
//! suite a contributor can extend without writing Rust: drop
//! `tests/fmt/input/<name>.nvs` beside `tests/fmt/formatted/<name>.nvs` and
//! both tests below pick it up. The goal's § *Standing decisions* is where that
//! layout is decided.
//!
//! **The frozen half is frozen.** It is the expected output, so a rule that
//! changes it is a rule whose diff shows every file it moves — which is
//! `rule:tooling/fmt-is-one-canonical-style`'s point that the style is a
//! stable set rather than a setting. It is also a fixed point
//! (`rule:tooling/fmt-is-idempotent`), which is what `nvs fmt --check` over the
//! directory asserts from the command's side.

use std::path::{Path, PathBuf};

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `<repo>/tests/fmt/<half>`, whichever directory in the pair `half` names.
fn half(half: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "..", "..", "tests", "fmt", half]
        .iter()
        .collect()
}

/// Every `.nvs` file directly under `directory`, by name, sorted.
fn names(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(directory)
        .unwrap_or_else(|err| panic!("{} is readable: {err}", directory.display()))
        .map(|entry| entry.expect("the directory entry is readable").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "nvs"))
        .map(|path| {
            path.file_name()
                .expect("a file has a name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// `path`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(path: &Path) -> String {
    let mut map = SourceMap::new();
    let id = map
        .load(path)
        .unwrap_or_else(|err| panic!("{} is readable: {err}", path.display()));
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

/// `path`'s bytes as they sit on disk.
fn on_disk(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("{} is readable: {err}", path.display()))
}

/// The pair is a pair: neither half has a file the other does not, so a
/// fixture added to one directory and forgotten in the other fails loudly
/// instead of being silently skipped by a walk over the smaller side.
#[test]
fn every_input_fixture_has_a_frozen_pair() {
    assert_eq!(
        names(&half("input")),
        names(&half("formatted")),
        "every `tests/fmt/input/<name>.nvs` has its `tests/fmt/formatted/<name>.nvs`"
    );
    assert!(
        !names(&half("input")).is_empty(),
        "the fixture tree is not empty"
    );
}

/// Each input, formatted, is its pair byte for byte.
#[test]
fn every_input_fixture_formats_to_its_frozen_pair() {
    for name in names(&half("input")) {
        let frozen = half("formatted").join(&name);
        assert_eq!(
            formatted(&half("input").join(&name)),
            on_disk(&frozen),
            "tests/fmt/input/{name} formats to its frozen pair"
        );
    }
}

/// And each frozen file formats to itself: the directory `nvs fmt --check`
/// runs over holds nothing that would change.
#[test]
fn every_frozen_fixture_is_a_fixed_point() {
    for name in names(&half("formatted")) {
        let frozen = half("formatted").join(&name);
        assert_eq!(
            formatted(&frozen),
            on_disk(&frozen),
            "tests/fmt/formatted/{name} is already formatted"
        );
    }
}
