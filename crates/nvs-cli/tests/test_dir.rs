//! `nvs test <directory>`, driven through the built binary: a directory holding
//! `.nvs` files is one program that requires every one of them, so a test suite
//! runs without an entry that lists each file
//! (`rule:testing/a-directory-of-programs-is-one-test-program`).
//!
//! The fixture is an application laid out the way the testing reference
//! describes one: a bootstrap under `public/` that declares the `autoload`,
//! classes under `src/`, and a `tests/` directory whose `bootstrap.nvs`
//! requires the bootstrap and whose test files require nothing.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// `nvs test <args...> <path>`, as `(stdout, stderr, success)`.
fn run_test(args: &[&str], path: &Path) -> (String, String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("test")
        .args(args)
        .arg(path)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the report is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.success(),
    )
}

/// `tests/fixtures/test-dir/<name>`.
fn fixture(name: &str) -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "test-dir",
        name,
    ]
    .iter()
    .collect()
}

/// The `--format json` document `nvs test <args...> <fixture>` printed.
fn document(args: &[&str], name: &str) -> Value {
    let (out, err, ok) = run_test(args, &fixture(name));
    assert!(ok, "the run failed:\n{err}\n{out}");
    serde_json::from_str(&out)
        .unwrap_or_else(|error| panic!("`--format json` printed a document: {error}\n{out}"))
}

/// The `Class::method` of every record under `key`, sorted.
fn names(document: &Value, key: &str) -> Vec<String> {
    let mut names: Vec<String> = document[key]
        .as_array()
        .unwrap_or_else(|| panic!("`{key}` is an array: {document}"))
        .iter()
        .map(|test| {
            format!(
                "{}::{}",
                test["class"].as_str().expect("a class"),
                test["method"].as_str().expect("a method")
            )
        })
        .collect();
    names.sort();
    names
}

/// The whole directory is one program: the class under `src/` resolves through
/// the bootstrap one file in the directory requires, and a test in a
/// subdirectory runs beside one at the top.
#[test]
fn a_directory_of_test_files_runs_as_one_program() {
    let document = document(&["--format", "json"], "shop/tests");
    assert_eq!(
        names(&document, "tests"),
        ["CartTest::totalIsThree", "HelperTest::addition"]
    );
}

/// `--list` over a directory names each test's own file, not the entry the
/// runner wrote for the directory.
#[test]
fn listing_a_directory_names_each_tests_own_file() {
    let document = document(&["--list", "--format", "json"], "shop/tests");
    assert_eq!(
        names(&document, "listed"),
        ["CartTest::totalIsThree", "HelperTest::addition"]
    );
    let cart = document["listed"]
        .as_array()
        .expect("an array")
        .iter()
        .find(|test| test["class"] == "CartTest")
        .expect("the cart test is listed");
    let file = cart["file"].as_str().expect("a file");
    assert!(file.ends_with("CartTest.nvs"), "{file}");
}

/// A subdirectory is a program of its own: what needs no bootstrap runs, and
/// the tests above it are not swept in.
#[test]
fn a_subdirectory_is_run_on_its_own() {
    let document = document(&["--format", "json"], "shop/tests/unit");
    assert_eq!(names(&document, "tests"), ["HelperTest::addition"]);
}

/// The two suites share no report, so a directory holding both kinds of file
/// is refused rather than run as whichever kind was found first.
#[test]
fn a_directory_mixing_programs_and_cases_is_refused() {
    let (_, err, ok) = run_test(&[], &fixture("mixed"));
    assert!(!ok);
    assert!(err.contains("run separately"), "{err}");
}
