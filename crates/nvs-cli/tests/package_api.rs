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
// covers: tools:packages/nvs-pkg-api-and-nvs-pkg-diff
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

/// The attack under `tests/hostile/tools/packages/nvs-pkg-api-and-nvs-pkg-diff/` writes one type
/// many ways: an alias of an alias, `?T`, `self` and a union out of order. As a package's only source
/// file, each type is written in one resolved, sorted form.
// covers: tools:packages/nvs-pkg-api-and-nvs-pkg-diff
#[test]
fn a_type_written_many_ways_has_one_form() {
    let attack = nvs_repo::path(
        "tests/hostile/tools/packages/nvs-pkg-api-and-nvs-pkg-diff/01-one-type-written-many-ways.nvs",
    );
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("package-api-one-type-many-ways");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).expect("the package directory");
    std::fs::copy(&attack, dir.join("src/Leaf.nvs")).expect("the attack is the package's source");

    let document: serde_json::Value =
        serde_json::from_str(&api(&dir)).expect("stdout is the document");
    let _ = std::fs::remove_dir_all(&dir);
    let method = |class: &str, name: &str| {
        let declaration = document["declarations"]
            .as_array()
            .expect("a list of declarations")
            .iter()
            .find(|d| d["name"] == class)
            .unwrap_or_else(|| panic!("{class} is declared"));
        declaration["methods"]
            .as_array()
            .expect("a list of methods")
            .iter()
            .find(|m| m["name"] == name)
            .unwrap_or_else(|| panic!("{class}::{name} is declared"))
            .clone()
    };
    let copy = method("Shop\\Leaf", "copy");
    assert_eq!(copy["params"][0]["type"], "Shop\\Leaf|null");
    assert_eq!(copy["params"][1]["type"], "int|string");
    assert_eq!(copy["returns"], "Shop\\Leaf");
    let same = method("Shop\\Leaf", "same");
    assert_eq!(same["params"][0]["type"], "int|null|string");
    assert_eq!(same["returns"], "int|null|string");
    assert_eq!(method("Shop\\Base", "id")["returns"], "int|null|string");
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

/// The report's last line when neither side has a version.
const NO_VERSION: &str = "Neither version is known, so no breaking change is allowed.";

/// Each pair under `package-api/<class>/` makes one change, and the report is that change alone:
/// its class, where it is and what it is. No side has a version, so a pair with a break exits
/// non-zero, and one without exits zero.
// covers: tools:packages/nvs-pkg-api-and-nvs-pkg-diff
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
        let fits = lines.len() >= 2 && {
            let (changes, tail) = lines.split_at(lines.len() - 2);
            changes == *want && tail[0].ends_with(" additive") && tail[1] == NO_VERSION
        };
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
            "from": null,
            "to": null,
            "step": "unknown",
            "allowed": false,
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

/// The version step decides whether a break fails. `removed-method` breaks and `new-class` only
/// adds, and `--from` and `--to` give each side's version, since neither has a `package.toml`.
// covers: tools:packages/nvs-pkg-api-and-nvs-pkg-diff
#[test]
fn the_version_step_decides_whether_a_break_fails() {
    let cases: &[(&str, &[&str], bool, &str)] = &[
        (
            "removed-method",
            &["--from", "1.2.0", "--to", "1.3.0"],
            false,
            "1.2.0 to 1.3.0 is a minor release, so no breaking change is allowed.",
        ),
        (
            "removed-method",
            &["--from", "1.2.0", "--to", "1.2.1"],
            false,
            "1.2.0 to 1.2.1 is a patch release, so no breaking change is allowed.",
        ),
        (
            "removed-method",
            &["--from", "0.1.0", "--to", "0.2.0"],
            false,
            "0.1.0 to 0.2.0 is a minor release, so no breaking change is allowed.",
        ),
        (
            "removed-method",
            &["--from", "1.2.0", "--to", "2.0.0"],
            true,
            "1.2.0 to 2.0.0 is a major release, so every change is allowed.",
        ),
        (
            "removed-method",
            &["--from", "0.0.3", "--to", "0.0.4"],
            true,
            "0.0.3 to 0.0.4: a 0.0.x version is experimental, so every change is allowed.",
        ),
        (
            "removed-method",
            &["--from", "0.0.9", "--to", "0.1.0"],
            true,
            "0.0.9 to 0.1.0: a 0.0.x version is experimental, so every change is allowed.",
        ),
        (
            "removed-method",
            &["--from", "1.3.0", "--to", "1.2.0"],
            false,
            "1.2.0 is not higher than 1.3.0, so no breaking change is allowed.",
        ),
        (
            "removed-method",
            &["--to", "2.0.0"],
            false,
            "The old version is not known, so no breaking change is allowed.",
        ),
        (
            "new-class",
            &["--from", "1.2.0", "--to", "1.2.1"],
            true,
            "1.2.0 to 1.2.1 is a patch release, so no breaking change is allowed.",
        ),
    ];
    let mut wrong = Vec::new();
    for (name, extra, passes, step) in cases {
        let pair = fixture(name);
        let (ok, printed) = diff(&pair.join("old"), &pair.join("new"), extra);
        if ok != *passes || printed.lines().last() != Some(*step) {
            wrong.push(format!("{name} {extra:?}: exit ok {ok}\n{printed}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Each side's version is the `version` in its `package.toml`, and `--to` replaces the new one.
/// `versioned` removes a method from `1.4.0` to `1.5.0`, which a minor does not allow, and the
/// JSON form names the step. A version that is not three numbers is an error.
#[test]
fn each_side_reads_its_version_from_package_toml() {
    let pair = fixture("versioned");
    let (old, new) = (pair.join("old"), pair.join("new"));
    let (ok, printed) = diff(&old, &new, &[]);
    assert!(!ok, "{printed}");
    assert_eq!(
        printed.lines().last(),
        Some("1.4.0 to 1.5.0 is a minor release, so no breaking change is allowed.")
    );

    let (ok, printed) = diff(&old, &new, &["--to", "2.0.0", "--json"]);
    assert!(ok, "{printed}");
    let report: serde_json::Value = serde_json::from_str(&printed).expect("stdout is JSON");
    assert_eq!(report["from"], "1.4.0");
    assert_eq!(report["to"], "2.0.0");
    assert_eq!(report["step"], "major");
    assert_eq!(report["allowed"], true);

    let out = nvs(&[
        "pkg".as_ref(),
        "diff".as_ref(),
        old.as_os_str(),
        new.as_os_str(),
        "--from".as_ref(),
        "1.4".as_ref(),
    ]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("`1.4` is not a version"), "{stderr}");
}
