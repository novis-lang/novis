//! `nvs build --openapi` and `nvs api diff`, driven as a user drives them —
//! [ADR 0085](../../../docs/adr/0085-openapi-is-generated-from-the-route-table.md)'s
//! *Verification* section, for the rows the route table supplies today.
//!
//! Through the built binary rather than by calling the emitter, because
//! `nvs-cli` is a binary crate with no library target and because the document
//! is a *command's* output: the thing ADR 0085 § 3 promises is what
//! `nvs build --openapi` writes, and a unit test over a function reachable only
//! from `main.rs` would be asserting one layer below the promise. `nvs test`'s
//! own `.nvst` cases cannot cover this either — a case is a program's stdout,
//! and this is a build artifact produced without running anything.

use std::process::Command;

/// The fixture ADR 0077's route table already uses: three routes over two
/// paths, one of them capturing a `uint`.
const ROUTES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/routes.nvs");

/// A program with no `#[Route]` anywhere in it.
const HELLO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/hello.nvs");

/// `nvs build --openapi <file>`, as `(stdout, stderr, success)`.
fn build(file: &str) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["build", "--openapi", file])
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the document is UTF-8"),
        String::from_utf8(out.stderr).expect("diagnostics are UTF-8"),
        out.status.success(),
    )
}

/// § 1 and the *Fidelity* bullet: every route in the table appears exactly once
/// in the document, at its own path, under its own verb, carrying the
/// `operationId` its `name` gave it.
#[test]
fn an_openapi_document_names_every_route_of_the_fixture_once() {
    let (doc, _, ok) = build(ROUTES);
    assert!(ok, "the fixture compiles, so the emitter runs");

    assert!(
        doc.contains("\"openapi\": \"3.1"),
        "3.1 and no other: {doc}"
    );
    for fragment in [
        "\"/users\"",
        "\"/users/{id}\"",
        "\"operationId\": \"Users::index\"",
        "\"operationId\": \"Users::create\"",
        "\"operationId\": \"Users::show\"",
    ] {
        assert_eq!(
            doc.matches(fragment).count(),
            1,
            "`{fragment}` appears exactly once in\n{doc}"
        );
    }

    // § 1's "path parameters by declared type": `show(uint $id)`, so the
    // capture is a non-negative integer and the document says both halves.
    assert!(
        doc.contains("\"name\": \"id\"")
            && doc.contains("\"in\": \"path\"")
            && doc.contains("\"required\": true")
            && doc.contains("\"minimum\": 0"),
        "the `{{id}}` capture is a required `uint` path parameter:\n{doc}"
    );
}

/// § 3: two builds of the same source produce byte-identical documents, which
/// is what makes § 4's diff mean anything. The cheapest way for it not to be is
/// a hash map in the emitter, and this is what would catch one.
#[test]
fn an_openapi_document_is_byte_identical_across_two_emissions() {
    let (first, _, ok) = build(ROUTES);
    let (second, _, _) = build(ROUTES);
    assert!(ok, "the fixture compiles");
    assert_eq!(first, second, "the emitter is not deterministic");
}

/// § 1's "a program with no `#[Route]` generates nothing and runs no pass": no
/// document, a sentence saying why, and a success exit — having nothing to emit
/// is not a failure.
#[test]
fn a_program_declaring_no_route_emits_no_document() {
    let (doc, err, ok) = build(HELLO);
    assert!(ok, "nothing to emit is not a failure: {err}");
    assert!(
        doc.is_empty(),
        "no document at all, not an empty one: {doc}"
    );
    assert!(err.contains("nothing to emit"), "and it says so: {err}");
}

/// A program the front end refuses emits no document — the ADR's whole premise
/// is that the document cannot say what the compiler did not agree to, so a
/// partial one is worse than none.
#[test]
fn a_program_with_a_diagnostic_emits_no_document() {
    let missing = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/nope.nvs");
    let (doc, _, ok) = build(missing);
    assert!(!ok, "a file that cannot be read is a failure");
    assert!(doc.is_empty(), "and writes no document: {doc}");
}

/// § 1's last row: the summary is the doc comment's first sentence, the
/// description is the rest, and a method that wrote no doc comment carries
/// neither member.
///
/// The bound is asserted on both sides on purpose — an emitter that wrote a
/// `summary` for every operation would pass the first half alone, and one that
/// wrote none would pass the second. The `//`-commented and `/* */`-commented
/// handlers are the third side: a doc comment is `/** … */` and prose above a
/// declaration in any other spelling is a note to the next reader of the source,
/// not text this document may publish.
#[test]
fn an_operations_summary_and_description_come_from_the_handlers_doc_comment() {
    let (doc, err, ok) = build(&fixture("documented"));
    assert!(ok, "the fixture compiles: {err}");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let operation = |path: &str, verb: &str| document["paths"][path][verb].clone();

    // Wrapped across two source lines, and one line in the document.
    let index = operation("/notes", "get");
    assert_eq!(
        index["summary"], "Lists every note the caller may read.",
        "the first sentence, collapsed onto one line:\n{doc}"
    );
    let description = index["description"]
        .as_str()
        .unwrap_or_else(|| panic!("the rest of the comment is the description:\n{doc}"));
    assert!(
        description.starts_with("The first sentence above")
            && description.contains("\n\nA second paragraph"),
        "trimmed, with its paragraph break kept:\n{description}"
    );

    // One sentence is a summary and nothing else -- an empty `description` is
    // a member a hand-written document would not carry.
    let show = operation("/notes/{id}", "get");
    assert_eq!(show["summary"], "One sentence and no more.");
    assert!(
        show["description"].is_null(),
        "a one-sentence comment writes no description:\n{show}"
    );

    // Neither spelling above `create` and `destroy` is a doc comment.
    for (path, verb) in [("/notes", "post"), ("/notes/{id}", "delete")] {
        let bare = operation(path, verb);
        assert!(
            bare["summary"].is_null() && bare["description"].is_null(),
            "`{verb} {path}` documented nothing, so the operation says nothing:\n{bare}"
        );
    }
    assert!(
        !doc.contains("not a doc comment") && !doc.contains("note to the next reader"),
        "and no line-comment text reached the document at all:\n{doc}"
    );
}

/// § 1's response body row: the `200` response's schema is the handler's
/// declared return type, through the same `schema` mapping a parameter's is.
///
/// The three answers the emitter has to tell apart, in one fixture: a type it
/// maps (`uint`, floor and all), a type it does not (a class, whose fields are
/// ADR 0071's codec rather than this document's guess), and `void`, which
/// carries no `content` at all. The third is the one that would go wrong
/// silently — an empty schema means *any body*, and *no body* is a different
/// promise to a generated client.
#[test]
fn an_operations_response_schema_is_the_handlers_declared_return_type() {
    let (doc, err, ok) = build(&fixture("returns"));
    assert!(ok, "the fixture compiles: {err}");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let ok_response = |path: &str| document["paths"][path]["get"]["responses"]["200"].clone();

    let counted = ok_response("/notes/count");
    assert_eq!(
        counted["content"]["application/json"]["schema"],
        serde_json::json!({"type": "integer", "minimum": 0}),
        "a `uint` return is a non-negative integer, exactly as a `uint` capture is:\n{doc}"
    );
    assert!(
        counted["description"].is_string(),
        "3.1 requires a description on every response object:\n{counted}"
    );

    assert_eq!(
        ok_response("/notes/first")["content"]["application/json"]["schema"],
        serde_json::json!({}),
        "a class has no schema here until its codec's roster reaches the row:\n{doc}"
    );

    let empty = ok_response("/notes/ping");
    assert!(
        empty["content"].is_null(),
        "a `void` handler answers with no body, which is not a body admitting anything:\n{empty}"
    );
}

// -------------------------------------------------------------------------
// § 4 — `nvs api diff`, the gate.
//
// Every case below builds both sides with the emitter rather than pasting
// JSON, which is what makes them a test of the *gate* rather than of a fixture
// someone hand-edited: the documents are what this binary writes today, so a
// change to the emitter that alters the shape shows up here instead of leaving
// two frozen files agreeing with each other.
// -------------------------------------------------------------------------

/// The three-fixture family under `tests/fixtures/api`, each one file: `base`,
/// `base` with an operation deleted, and `base` with one optional `#[Query]`
/// parameter added.
fn fixture(stem: &str) -> String {
    format!(
        "{}/tests/fixtures/api/{stem}.nvs",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// `nvs build --openapi` for a fixture, written to a file under the target
/// directory cargo gives an integration test for exactly this.
///
/// **A fresh file per call, not one per stem.** Three tests here ask for
/// `base`, `cargo test` runs them on their own threads, and a reader that
/// catches another thread's `fs::write` half-done gets a truncated document and
/// a diff that reports nothing — which is a flake in exactly the four cases
/// `loop-goal.toml` names as acceptance checks. The counter is what stops two
/// calls sharing a path at all; the documents are byte-identical either way, so
/// nothing about what is compared changes.
fn document(stem: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let (doc, err, ok) = build(&fixture(stem));
    assert!(ok, "the `{stem}` fixture compiles: {err}");
    let nth = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{stem}.{nth}.json"));
    std::fs::write(&path, doc).expect("the target directory is writable");
    path
}

/// `nvs api diff <old> <new>`, as `(stdout, stderr, success)`.
fn api_diff(old: &std::path::Path, new: &std::path::Path) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["api", "diff"])
        .args([old, new])
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the report is UTF-8"),
        String::from_utf8(out.stderr).expect("diagnostics are UTF-8"),
        out.status.success(),
    )
}

/// § 4's first breaking change, and the *Verification* section's "removing an
/// operation … exits non-zero from `nvs api diff`".
#[test]
fn an_api_diff_classifies_a_removed_route_as_breaking() {
    let (before, after) = (document("base"), document("removed"));
    let (report, _, ok) = api_diff(&before, &after);
    assert!(!ok, "a breaking change exits non-zero:\n{report}");
    assert!(
        report.contains("breaking: paths./items/{id}"),
        "and names the operation that went:\n{report}"
    );
    assert!(
        report.contains("1 breaking"),
        "one breaking change, not a class applied to the whole document:\n{report}"
    );
}

/// The other half of the same *Verification* line: "adding an optional field
/// and a new operation each exit zero". The parameter is `#[Query] $sort` with
/// a default, which ADR 0102 § 3 makes optional.
#[test]
fn an_api_diff_classifies_an_added_optional_parameter_as_compatible() {
    let (before, after) = (document("base"), document("optional"));
    let (report, _, ok) = api_diff(&before, &after);
    assert!(ok, "an additive change exits zero:\n{report}");
    assert!(
        report.contains("additive: paths./items.get.query.sort"),
        "and the parameter is named where it was added:\n{report}"
    );
    assert!(
        report.contains("0 breaking"),
        "nothing about it is breaking:\n{report}"
    );
}

/// The same *Verification* line's new operation, which is the removal read
/// backwards — the one direction a fixture pair gives for free.
#[test]
fn an_api_diff_classifies_an_added_route_as_compatible() {
    let (before, after) = (document("removed"), document("base"));
    let (report, _, ok) = api_diff(&before, &after);
    assert!(ok, "a new operation exits zero:\n{report}");
    assert!(
        report.contains("additive: paths./items/{id}"),
        "and the new path is named:\n{report}"
    );
}

/// A document against itself is the gate's resting state, and it has to be
/// silent: a gate that reports a change on every build is one a team turns off.
/// This rests on § 3's determinism, which
/// `an_openapi_document_is_byte_identical_across_two_emissions` pins directly.
#[test]
fn an_api_diff_of_a_document_against_itself_reports_no_change() {
    let same = document("base");
    let (report, _, ok) = api_diff(&same, &same);
    assert!(ok, "no change exits zero:\n{report}");
    assert_eq!(report.trim(), "no change", "and says exactly that");
}

/// "I could not read it" and "nothing changed" must not share an exit code:
/// in CI the second is a pass, so a mistyped path that reported one would turn
/// the gate off silently.
#[test]
fn an_api_diff_of_an_unreadable_document_is_a_failure() {
    let missing = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-such-document.json");
    let (report, err, ok) = api_diff(&document("base"), &missing);
    assert!(!ok, "an unreadable document is a failure:\n{report}{err}");
    assert!(
        err.contains("cannot read") && err.contains("no-such-document.json"),
        "and says which file:\n{err}"
    );
    assert!(
        !report.contains("no change"),
        "and never reports the resting state:\n{report}"
    );
}

// -------------------------------------------------------------------------
// § 2 -- `#[Api]` may add and may not contradict.
//
// One fixture per contradiction plus one that agrees, and the bound is
// asserted on both sides on purpose: four refusing fixtures alone would pass
// just as well against an `#[Api]` that refused everything it was handed, and
// the accepting one alone would pass against an `#[Api]` nothing checked at
// all. `crates/nvs-types/src/routes.rs`'s `check_api` owns which half of § 2's
// `security` rule is askable today and why the other half is not.
// -------------------------------------------------------------------------

/// § 2's four contradictions, each named by the message that tells it from the
/// other three -- they share `E0771`, because § 2 states one rule that four
/// different writings can break.
#[test]
fn an_api_attribute_contradicting_its_own_signature_is_a_diagnostic() {
    let contradictions = [
        (
            "errors-names-no-class",
            "is not a class a handler could produce",
        ),
        ("example-names-no-field", "declares no `quantity`"),
        ("security-is-not-a-name", "entry is not a string"),
        ("api-without-route", "annotates no operation"),
    ];
    for (stem, message) in contradictions {
        let (doc, err, ok) = build(&fixture(stem));
        assert!(!ok, "`{stem}` contradicts the code, so it does not build");
        assert!(
            err.contains("E0771") && err.contains(message),
            "`{stem}` is refused as `E0771: … {message} …`:\n{err}"
        );
        assert!(
            doc.trim().is_empty(),
            "and no document is written for a program that does not compile:\n{doc}"
        );
    }

    // The other side of the bound: the same four fields, agreeing.
    let (doc, err, ok) = build(&fixture("api-that-agrees"));
    assert!(ok, "an `#[Api]` that agrees with its code builds: {err}");
    assert!(
        doc.contains("\"operationId\": \"Items::show\""),
        "and the operation it annotates is in the document:\n{doc}"
    );
}
