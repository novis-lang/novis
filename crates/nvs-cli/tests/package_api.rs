//! `nvs pkg api`, driven through the built binary
//! (`rule:packaging/only-a-major-may-break-the-api`): the document is what the command prints, so
//! the pinned bytes are the command's stdout.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A fixture package's directory under `tests/fixtures/package-api`.
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/package-api")
        .join(name)
}

/// `nvs <args...>`, its whole output.
fn nvs(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(args)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// `nvs pkg api <dir>`'s stdout, after asserting it succeeded.
fn api(dir: &Path) -> String {
    let out = nvs(&["pkg".as_ref(), "api".as_ref(), dir.as_os_str()]);
    assert!(
        out.status.success(),
        "`nvs pkg api` fails: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("the document is UTF-8")
}

/// The fixture's API is pinned byte for byte, and a second run prints the same bytes. The fixture
/// covers an import, an alias, a nullable type, a promoted property, `self`, an owned enum, and
/// private members, which are left out.
#[test]
fn the_api_of_a_package_is_pinned_byte_for_byte() {
    let dir = fixture("shop");
    let printed = api(&dir);
    let pinned = std::fs::read_to_string(fixture("shop.json")).expect("the pinned document");
    assert_eq!(printed, pinned);
    assert_eq!(api(&dir), printed, "two runs print the same bytes");
}

/// A package file's top-level `echo` prints when the file is run, and prints nothing when its API
/// is extracted: stdout is the document and nothing else
/// (`rule:packaging/nothing-runs-before-the-program`).
#[test]
fn a_top_level_statement_has_no_effect_on_the_extraction() {
    let dir = fixture("effect");
    let file = dir.join("src/Greeter.nvs");
    let ran = nvs(&["run".as_ref(), file.as_os_str()]);
    assert!(
        String::from_utf8_lossy(&ran.stdout).contains("this line ran"),
        "the statement prints when the file runs"
    );

    let printed = api(&dir);
    assert!(!printed.contains("this line ran"), "{printed}");
    let document: serde_json::Value =
        serde_json::from_str(&printed).expect("stdout is the document alone");
    assert_eq!(document["declarations"][0]["name"], "Effect\\Greeter");
}
