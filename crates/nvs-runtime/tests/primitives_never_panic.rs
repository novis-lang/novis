//! `rule:errors/helper-abi`: a primitive has no `catch_unwind` around it, so a
//! panic in one aborts the process. No program may reach one.
//!
//! A primitive is a bare `extern "C" fn` that compiled code calls directly. A
//! helper written through `nvs_helper!` is not one: its wrapper already
//! contains a panic to its request. The build holds the rule through clippy,
//! and these tests hold that every primitive asks clippy to: each carries the
//! `deny` set below, and a site that stays carries an `#[expect]` whose reason
//! says why no program reaches it.
//!
//! The tests read the source, so they cover a primitive the day it is added.
//! A primitive is a bare `extern "C" fn` declared at the top level of its file,
//! or one anywhere that is exported with `#[unsafe(no_mangle)]`. An indented
//! one with no export is a test's own callback.

use std::path::{Path, PathBuf};

/// The lints every primitive denies in its own body.
const DENIED: [&str; 6] = [
    "clippy::expect_used",
    "clippy::unwrap_used",
    "clippy::panic",
    "clippy::unreachable",
    "clippy::indexing_slicing",
    "clippy::arithmetic_side_effects",
];

fn source_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The name of the bare `extern "C" fn` this line declares, if it declares one.
fn declared_primitive(line: &str) -> Option<&str> {
    let (_, rest) = line.split_once("extern \"C\" fn ")?;
    let name_end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
    let name = &rest[..name_end];
    let starts_like_a_name = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    (starts_like_a_name && rest[name_end..].starts_with('(')).then_some(name)
}

/// The doc comment and attributes above line `at`: every line back to the
/// blank line, the closing brace or the statement end that ends the item
/// before it.
fn preamble(lines: &[&str], at: usize) -> String {
    let start = lines[..at]
        .iter()
        .rposition(|line| {
            let line = line.trim_end();
            line.trim().is_empty() || line.ends_with('}') || line.ends_with(';')
        })
        .map_or(0, |end| end + 1);
    lines[start..at].join("\n")
}

/// Whether `preamble` holds one `#[deny(...)]` naming every lint in [`DENIED`].
fn denies_every_panic(preamble: &str) -> bool {
    preamble.match_indices("#[deny(").any(|(open, _)| {
        let body = &preamble[open..];
        let body = &body[..body.find(")]").unwrap_or(body.len())];
        DENIED.iter().all(|lint| body.contains(lint))
    })
}

/// Every primitive in `file` that does not deny a panic in its own body, as
/// `file:line name`.
fn primitives_missing_the_deny(file: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(file)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()));
    let lines: Vec<&str> = text.lines().collect();
    let mut missing = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let Some(name) = declared_primitive(line) else {
            continue;
        };
        let attributes = preamble(&lines, at);
        let top_level = !line.starts_with(char::is_whitespace);
        if !top_level && !attributes.contains("no_mangle") {
            continue;
        }
        if !denies_every_panic(&attributes) {
            missing.push(format!("{}:{} {name}", file.display(), at + 1));
        }
    }
    missing
}

fn every_source_file(dir: &Path, into: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot list {}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            every_source_file(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
}

/// Fails naming each primitive in `files` that does not deny a panic.
fn assert_every_primitive_denies_a_panic(group: &str, files: &[PathBuf]) {
    let missing: Vec<String> = files
        .iter()
        .flat_map(|file| primitives_missing_the_deny(file))
        .collect();
    assert!(
        missing.is_empty(),
        "these {group} primitives do not `#[deny(...)]` every lint in {DENIED:?}, so a \
         panic in their body would abort the process:\n{}",
        missing.join("\n"),
    );
}

fn files(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(|name| source_dir().join(name)).collect()
}

#[test]
fn the_array_primitives_deny_a_panic_in_their_own_body() {
    assert_every_primitive_denies_a_panic("array", &files(&["array.rs", "identity.rs"]));
}

#[test]
fn the_string_and_value_primitives_deny_a_panic_in_their_own_body() {
    assert_every_primitive_denies_a_panic(
        "string, value and arithmetic",
        &files(&["string.rs", "value.rs", "arith.rs"]),
    );
}

#[test]
fn the_object_primitives_deny_a_panic_in_their_own_body() {
    assert_every_primitive_denies_a_panic("object", &files(&["object.rs"]));
}

#[test]
fn every_primitive_denies_a_panic_in_its_own_body() {
    let mut all = Vec::new();
    every_source_file(&source_dir(), &mut all);
    all.sort();
    assert_every_primitive_denies_a_panic("runtime", &all);
}

#[test]
fn the_scan_finds_a_primitive_and_reads_its_deny() {
    let source = [
        "/// A primitive.",
        "#[deny(",
        "    clippy::expect_used,",
        "    clippy::unwrap_used,",
        "    clippy::panic,",
        "    clippy::unreachable,",
        "    clippy::indexing_slicing,",
        "    clippy::arithmetic_side_effects",
        ")]",
        "#[unsafe(no_mangle)]",
        "pub unsafe extern \"C\" fn nvs_example(ptr: *mut u8) {",
    ];
    assert_eq!(declared_primitive(source[10]), Some("nvs_example"));
    assert!(denies_every_panic(&preamble(&source, 10)));
    assert!(!denies_every_panic(&preamble(&source[2..], 8)));
    assert_eq!(
        declared_primitive("type F = extern \"C\" fn(*mut u8);"),
        None
    );
    assert_eq!(
        declared_primitive("    unsafe extern \"C\" fn $name("),
        None
    );
}
