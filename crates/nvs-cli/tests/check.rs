//! `nvs check --json`, driven the way CI and an agent drive it — through the
//! built binary, because what
//! `rule:ide/check-json-is-the-diagnostic-record-as-a-document` promises is
//! what the *command* prints, the same reason `ast.rs` goes through it.
//!
//! Every assertion here is a comparison against the text rendering rather than
//! against a frozen literal, because the rule's promise is that the two say the
//! same thing: one record per diagnostic the text renderer prints, with the
//! same codes, the same positions and the same help. A snapshot could freeze
//! the document and still let the two surfaces drift apart.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// `nvs check <args...> <path>`, as `(stdout, stderr, success)`.
fn run_check(args: &[&str], path: &Path) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("check")
        .args(args)
        .arg(path)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the document is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.success(),
    )
}

/// `tests/fixtures/check/<name>`.
fn fixture(name: &str) -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "check",
        name,
    ]
    .iter()
    .collect()
}

/// The document `nvs check --json <fixture>` printed, parsed.
fn document(name: &str) -> Value {
    let (out, _, _) = run_check(&["--json"], &fixture(name));
    serde_json::from_str(&out)
        .unwrap_or_else(|err| panic!("`--json` printed a document: {err}\n{out}"))
}

/// The `severity`, `code` and `message` of every record in a document.
fn json_heads(document: &Value) -> Vec<(String, String, String)> {
    document["diagnostics"]
        .as_array()
        .expect("`diagnostics` is an array")
        .iter()
        .map(|d| {
            (
                d["severity"].as_str().expect("a severity").to_string(),
                d["code"].as_str().unwrap_or("").to_string(),
                d["message"].as_str().expect("a message").to_string(),
            )
        })
        .collect()
}

/// The same three, read off the heads of a terminal rendering.
///
/// A head is the one line of a rendered diagnostic that starts at column zero
/// with a severity label; every other line of the snippet is indented or
/// begins with a line number. The summary the renderer closes with —
/// `error: aborting due to N errors` — is a total rather than a record, and is
/// dropped here for the same reason the document does not carry it.
fn text_heads(rendered: &str) -> Vec<(String, String, String)> {
    const LABELS: [&str; 5] = [
        "internal compiler error",
        "warning",
        "error",
        "help",
        "note",
    ];
    rendered
        .lines()
        .filter_map(|line| {
            let label = LABELS.iter().find(|label| line.starts_with(*label))?;
            let rest = &line[label.len()..];
            let (code, rest) = match rest.strip_prefix('[') {
                Some(rest) => {
                    let (code, rest) = rest.split_once(']')?;
                    (code, rest)
                }
                None => ("", rest),
            };
            let message = rest.strip_prefix(": ")?;
            if message.starts_with("aborting due to ") {
                return None;
            }
            Some(((*label).to_string(), code.to_string(), message.to_string()))
        })
        .collect()
}

/// The rule's central claim, asserted as the equality it is.
#[test]
fn check_json_emits_one_record_per_diagnostic_the_text_renderer_prints() {
    let (out, rendered, ok) = run_check(&[], &fixture("diagnostics.nvs"));
    assert!(!ok, "the fixture does not compile");
    assert_eq!(
        out, "",
        "the text rendering writes nothing to standard output"
    );

    let text = text_heads(&rendered);
    assert!(
        text.len() > 1,
        "the fixture is worth comparing over: {rendered}"
    );
    assert_eq!(
        json_heads(&document("diagnostics.nvs")),
        text,
        "the document is the records the renderer printed, in the same order:\n{rendered}"
    );

    // The other half of "one record per diagnostic": a file with none gets a
    // document with an empty list rather than no document at all.
    let clean = document("clean.nvs");
    assert_eq!(clean["diagnostics"], serde_json::json!([]));
    assert_eq!(clean["schemaVersion"], 1);
}

/// Every field `rule:ide/check-json-is-the-diagnostic-record-as-a-document`
/// names, checked against the source the span points into and against the
/// header the text rendering prints for it.
#[test]
fn check_json_carries_the_code_span_severity_help_and_suggestions() {
    let document = document("diagnostics.nvs");
    let records = document["diagnostics"]
        .as_array()
        .expect("`diagnostics` is an array");
    let record = |code: &str| {
        records
            .iter()
            .find(|d| d["code"] == code)
            .unwrap_or_else(|| panic!("the fixture reports {code}: {document:#}"))
            .clone()
    };

    let source = std::fs::read_to_string(fixture("diagnostics.nvs")).expect("the fixture is read");
    let casing = record("E0110");
    assert_eq!(casing["severity"], "error");
    let label = &casing["labels"][0];
    assert_eq!(label["style"], "primary");
    assert_eq!(label["message"], "class name must be PascalCase");
    let span = |value: &Value| {
        let offset = |at: usize| {
            let offset = value["span"][at].as_u64().expect("a byte offset");
            usize::try_from(offset).expect("a source file is shorter than a pointer")
        };
        source[offset(0)..offset(1)].to_string()
    };
    assert_eq!(
        span(label),
        "my_class",
        "the span is a byte range into the file the label names"
    );

    // The positions are the ones the `-->` header shows, which is the whole
    // reason they are in the document beside the offsets.
    let (_, rendered, _) = run_check(&[], &fixture("diagnostics.nvs"));
    let header = rendered
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("--> "))
        .expect("the rendering anchors the first diagnostic");
    let (file, position) = header
        .rsplit_once(':')
        .and_then(|(head, col)| {
            let (file, line) = head.rsplit_once(':')?;
            Some((file.to_string(), (line.to_string(), col.to_string())))
        })
        .expect("the header is path:line:col");
    assert_eq!(
        label["file"].as_str().expect("a file"),
        file,
        "the document names the file the header does"
    );
    assert_eq!(
        (
            label["start"]["line"].to_string(),
            label["start"]["column"].to_string()
        ),
        position,
        "the document's one-based position is the header's"
    );

    let suggestion = &casing["suggestions"][0];
    assert_eq!(suggestion["replacement"], "MyClass");
    assert_eq!(suggestion["message"], "rename to `MyClass`");
    assert_eq!(suggestion["safe"], true);
    assert_eq!(suggestion["alternative"], false);
    assert_eq!(span(suggestion), "my_class", "the edit replaces the name");

    // `help` is its own array, and carries the text without the prefix the
    // plaintext rendering puts in front of it.
    let eval = record("E0201");
    let help = eval["help"][0].as_str().expect("a help line");
    assert!(
        eval["notes"]
            .as_array()
            .expect("`notes` is an array")
            .is_empty(),
        "a help line is not also a note: {eval:#}"
    );
    assert!(
        rendered.contains(&format!("= help: {help}")),
        "the help line is the one the renderer prints: {rendered}"
    );
    assert_eq!(
        eval["suggestions"],
        serde_json::json!([]),
        "a diagnostic with no edit carries an empty list rather than no field"
    );
}

/// The flag adds a rendering and changes nothing about the one that was there.
#[test]
fn the_default_rendering_is_still_text() {
    let (out, rendered, ok) = run_check(&[], &fixture("clean.nvs"));
    assert!(ok, "the fixture compiles: {rendered}");
    assert_eq!(out, "no errors\n", "the success line is unchanged");

    let (failing, rendered, ok) = run_check(&[], &fixture("diagnostics.nvs"));
    assert!(!ok);
    assert_eq!(failing, "", "diagnostics go to standard error, as they did");
    assert!(
        rendered.contains("error[E0110]: ") && rendered.contains("  |"),
        "the terminal rendering is the snippet it always was: {rendered}"
    );

    // Under `--json` the document is standard output's only occupant: the
    // success line is dropped, since an empty `diagnostics` array says it.
    let (out, err, ok) = run_check(&["--json"], &fixture("clean.nvs"));
    assert!(ok);
    assert_eq!(err, "", "the document is not printed beside a rendering");
    serde_json::from_str::<Value>(&out).expect("standard output is the document alone");

    // And it cannot share that output with the autoload map, which is the
    // other thing this command prints there.
    let refused = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["check", "--json", "--autoload-map"])
        .arg(fixture("clean.nvs"))
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(
        !refused.status.success(),
        "the two flags are refused together"
    );
}
