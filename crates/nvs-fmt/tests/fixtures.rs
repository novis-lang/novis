//! The fixture pairs under `tests/fmt/`: a file as somebody wrote it beside the
//! one layout `nvs fmt` writes for it.
//!
//! The pairs are where a rule is exercised over a whole file rather than over
//! the one construct its own case is about, and they are the half of this
//! suite a contributor can extend without writing Rust: drop
//! `tests/fmt/input/<name>.nvs` beside `tests/fmt/formatted/<name>.nvs` and the
//! walk below picks it up. The goal's § *Standing decisions* is where that
//! layout is decided.
//!
//! One pair is also read on its own: `comments.nvs` writes a comment in every
//! position the grammar allows one, and the last test here is the claim that
//! none of them moves — which the pair's own byte comparison makes too, and
//! says nothing readable about when it fails.
//!
//! **The frozen half is frozen.** It is the expected output, so a rule that
//! changes it is a rule whose diff shows every file it moves — which is
//! `rule:tooling/fmt-is-one-canonical-style`'s point that the style is a
//! stable set rather than a setting. It is also a fixed point
//! (`rule:tooling/fmt-is-idempotent`), which is what `nvs fmt --check` over the
//! directory asserts from the command's side.

use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_fmt::format;
use nvs_syntax::TriviaKind;

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

/// Every comment in `source`, in source order, as what it is rather than as
/// where it is.
///
/// Its kind, its own bytes, whether it opens its line or follows code on one,
/// and the path of productions it sits inside — and no offset, because every
/// offset in a file moves the moment its layout does. That path is what
/// "attached where it started" means for a layer the grammar holds no node for:
/// a comment written above a method is inside the class, and one written above
/// the method's first statement is inside the method.
fn comments(source: &str) -> Vec<String> {
    let mut map = SourceMap::new();
    let id = map.add("comments.nvs".to_owned(), source);
    let mut diagnostics = Diagnostics::new();
    let parsed = nvs_syntax::parse(map.file(id), &mut diagnostics);
    parsed
        .trivia
        .iter()
        .filter(|trivium| trivium.kind != TriviaKind::Whitespace)
        .map(|trivium| {
            let (start, end) = (trivium.span.start as usize, trivium.span.end as usize);
            let line = source[..start].rfind('\n').map_or(0, |brk| brk + 1);
            let opens = if source[line..start].trim().is_empty() {
                "opens its line"
            } else {
                "follows code"
            };
            let path = parsed.index.at(trivium.span.start);
            let inside: Vec<&str> = path.nodes().iter().rev().map(|node| node.kind).collect();
            format!(
                "{:?} {opens} inside [{}]: {}",
                trivium.kind,
                inside.join(" > "),
                &source[start..end]
            )
        })
        .collect()
}

/// `rule:ide/tokens-plus-trivia-reproduce-the-file` read as the promise a
/// formatter makes: every comment comes back, and comes back where it was.
#[test]
fn a_comment_in_every_position_the_grammar_allows_survives_where_it_started() {
    let input = half("input").join("comments.nvs");
    let before = comments(&on_disk(&input));
    let after = comments(&formatted(&input));

    assert!(
        before.len() > 15,
        "tests/fmt/input/comments.nvs holds {} comment(s), which is too few to be about every \
         position the grammar allows one",
        before.len()
    );
    assert_eq!(
        before, after,
        "every comment survives formatting, in order, with its own bytes and inside the same \
         productions"
    );
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
