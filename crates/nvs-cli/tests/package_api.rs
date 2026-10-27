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

/// `nvs pkg diff <old> <new> [extra...]`: whether it succeeded, and its stdout.
fn diff(old: &Path, new: &Path, extra: &[&str]) -> (bool, String) {
    let mut args = vec![
        "pkg".as_ref(),
        "diff".as_ref(),
        old.as_os_str(),
        new.as_os_str(),
    ];
    args.extend(extra.iter().map(std::ffi::OsStr::new));
    let out = nvs(&args);
    (
        out.status.success(),
        String::from_utf8(out.stdout).expect("the report is UTF-8"),
    )
}

/// Each pair under `package-api/<class>/` makes one change, and the report is that change alone:
/// its class, where it is and what it is. A pair with a break exits non-zero, and one without
/// exits zero.
#[test]
fn each_change_between_two_versions_gets_its_class() {
    let cases: &[(&str, &[&str])] = &[
        (
            "removed-method",
            &["breaking: Api\\Client::send() — method removed"],
        ),
        (
            "renamed-class",
            &[
                "breaking: Api\\Client — class removed",
                "additive: Api\\Connection — new class",
            ],
        ),
        (
            "lowered-visibility",
            &["breaking: Api\\Client::ping() — `visibility` changed from `public` to `protected`"],
        ),
        (
            "interface-method",
            &[
                "breaking: Api\\Sender::close() — new method on an interface, which every implementing class must write",
            ],
        ),
        (
            "abstract-method",
            &["breaking: Api\\Handler::handle() — new abstract method"],
        ),
        (
            "enum-case",
            &[
                "breaking: Api\\Mode — new case `Paused`, which a `match` over every case does not cover",
            ],
        ),
        (
            "required-parameter",
            &["breaking: Api\\Client::send() — new required parameter `$timeout`"],
        ),
        (
            "narrowed-parameter",
            &[
                "breaking: Api\\Client::send($id) — `type` changed from `int|string` to `int`, which no longer accepts every value it did",
            ],
        ),
        (
            "widened-return",
            &[
                "breaking: Api\\Client::find() — `returns` changed from `string` to `null|string`, which may return a value it did not before",
            ],
        ),
        (
            "undecidable-parameter",
            &[
                "breaking: Api\\Client::send($ids) — `type` changed from `array<int>` to `array<int|string>`, and the comparison cannot tell which way it moved",
            ],
        ),
        (
            "widened-parameter",
            &[
                "additive: Api\\Client::send($id) — `type` changed from `int` to `int|string`, which is wider",
            ],
        ),
        (
            "narrowed-return",
            &[
                "additive: Api\\Client::find() — `returns` changed from `null|string` to `string`, which is narrower",
            ],
        ),
        (
            "parent-parameter",
            &[
                "additive: Api\\Client::send($to) — `type` changed from `Api\\Admin` to `Api\\User`, which is wider",
            ],
        ),
        ("new-class", &["additive: Api\\Server — new class"]),
        (
            "concrete-method",
            &["additive: Api\\Client::close() — new method"],
        ),
        (
            "optional-parameter",
            &["additive: Api\\Client::send() — new optional parameter `$timeout`"],
        ),
    ];
    let mut wrong = Vec::new();
    for (name, want) in cases {
        let pair = fixture(name);
        let (ok, printed) = diff(&pair.join("old"), &pair.join("new"), &[]);
        let lines: Vec<&str> = printed.lines().collect();
        let breaks = want.iter().any(|line| line.starts_with("breaking"));
        let fits = lines
            .split_last()
            .is_some_and(|(summary, changes)| changes == *want && summary.ends_with(" additive"));
        if !fits || ok == breaks {
            wrong.push(format!("{name}: exit ok {ok}\n{printed}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A side may be the document `nvs pkg api` printed: the pinned document against its own package
/// is no change, `--json` prints the changes as a document, and a file of another schema is
/// refused rather than compared.
#[test]
fn a_side_may_be_an_api_document() {
    let pinned = fixture("shop.json");
    let (ok, printed) = diff(&pinned, &fixture("shop"), &[]);
    assert!(ok, "{printed}");
    assert_eq!(printed, "no change\n");

    let pair = fixture("removed-method");
    let (ok, printed) = diff(&pair.join("old"), &pair.join("new"), &["--json"]);
    assert!(!ok, "a break exits non-zero");
    let report: serde_json::Value = serde_json::from_str(&printed).expect("stdout is JSON");
    assert_eq!(
        report,
        serde_json::json!({
            "schema": 1,
            "changes": [
                {"class": "breaking", "at": "Api\\Client::send()", "what": "method removed"}
            ]
        })
    );

    let other = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("package-api-schema-2.json");
    std::fs::write(&other, r#"{"schema": 2, "declarations": []}"#).expect("write the document");
    let out = nvs(&[
        "pkg".as_ref(),
        "diff".as_ref(),
        other.as_os_str(),
        pinned.as_os_str(),
    ]);
    std::fs::remove_file(&other).expect("remove the document");
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("has API schema 2"), "{stderr}");
}
