//! Every diagnostic code is asserted by its code in some test
//! (`rule:tooling/a-diagnostic-code-carries-its-card`, `rule:testing/feature-proofs`).
//!
//! A code is a promise the language makes, and `has_errors` alone proves only that something was
//! refused. So each code in [`code::ALL`] must appear in a test the way a test asserts one: as a
//! rendered `[E0401]`, a quoted `"E0401"`, or its constant `code::E_TYPE_MISMATCH`. A test is a
//! file under a `tests` directory of the repository, a `.nvst` case wherever it sits, or the
//! `#[cfg(test)]` tail of a crate's source file. This crate's own sources do not count: they
//! declare the codes, and their renderer tests use a code as an arbitrary example.
//!
//! The walk reads the repository's `crates/` and `tests/` trees once per run of this test, through
//! `nvs_repo`, so the test runs again when a test anywhere in them changes.

use std::fs;
use std::path::Path;

use nvs_diagnostics::code;

/// Every code's number and the name of its constant, read out of the area files under
/// `src/code/`.
fn declarations() -> Vec<(String, String)> {
    let dir = nvs_repo::path("crates/nvs-diagnostics/src/code");
    let mut source = String::new();
    for entry in fs::read_dir(&dir)
        .expect("the area files are readable")
        .flatten()
    {
        source.push_str(&fs::read_to_string(entry.path()).expect("an area file is text"));
        source.push('\n');
    }
    let mut out = Vec::new();
    for line in source.lines() {
        let Some(rest) = line.trim().strip_prefix("pub const ") else {
            continue;
        };
        let Some((name, rest)) = rest.split_once(": Code = Code::new(\"") else {
            continue;
        };
        let Some((number, _)) = rest.split_once('"') else {
            continue;
        };
        out.push((number.to_owned(), name.to_owned()));
    }
    out
}

/// Collects the test text under `dir`: every file below a `tests` directory, every `.nvst`
/// case, and the `#[cfg(test)]` tail of every other `.rs` file.
fn collect(dir: &Path, in_tests: bool, skipped: &Path, out: &mut String) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if path == skipped || matches!(name.as_ref(), "target" | "node_modules" | ".git") {
                continue;
            }
            collect(&path, in_tests || name == "tests", skipped, out);
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if in_tests || name.ends_with(".nvst") {
            out.push_str(&text);
        } else if name.ends_with(".rs")
            && let Some(at) = text.find("#[cfg(test)]")
        {
            out.push_str(&text[at..]);
        }
        out.push('\n');
    }
}

/// Whether `text` asserts the code: its number rendered or quoted, or its constant with no
/// longer name continuing it.
fn asserted(text: &str, number: &str, name: &str) -> bool {
    if text.contains(&format!("[{number}]")) || text.contains(&format!("\"{number}\"")) {
        return true;
    }
    let constant = format!("code::{name}");
    text.match_indices(&constant).any(|(at, _)| {
        text[at + constant.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_ascii_alphanumeric() || next == '_'))
    })
}

#[test]
#[cfg_attr(
    miri,
    ignore = "reads every file under `crates/` and `tests/`, which Miri interprets for longer than CI's miri job may run, and a text scan has no undefined behaviour of its own to find"
)]
fn every_code_is_asserted_by_its_code_in_a_test() {
    let crates = nvs_repo::path("crates");
    let own = crates.join("nvs-diagnostics").join("src");
    let mut text = String::new();
    collect(&crates, false, &own, &mut text);
    collect(&nvs_repo::path("tests"), true, &own, &mut text);

    let declared = declarations();
    assert_eq!(
        declared.len(),
        code::ALL.len(),
        "the registry's source and `code::ALL` disagree on how many codes there are"
    );

    let untested: Vec<String> = declared
        .iter()
        .filter(|(number, name)| !asserted(&text, number, name))
        .map(|(number, name)| format!("{number} ({name})"))
        .collect();
    assert!(
        untested.is_empty(),
        "no test asserts these codes by their code: {}",
        untested.join(", ")
    );
}
