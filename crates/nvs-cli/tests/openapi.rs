//! `nvs build --openapi` and `nvs api diff`, driven as a user drives them —
//! `rule:routing/api-document-is-generated-from-the-route-table`'s
//! *Verification* section, for the rows the route table supplies today.
//!
//! Through the built binary rather than by calling the emitter, because
//! `nvs-cli` is a binary crate with no library target and because the document
//! is a *command's* output: the thing `rule:routing/api-document-is-a-deterministic-build-artifact` promises is what
//! `nvs build --openapi` writes, and a unit test over a function reachable only
//! from `main.rs` would be asserting one layer below the promise. `nvs test`'s
//! own `.nvst` cases cannot cover this either — a case is a program's stdout,
//! and this is a build artifact produced without running anything.

use std::process::Command;

/// The fixture `rule:routing/routes-are-compiled-not-registered`'s route table already uses: three routes over two
/// paths, one of them capturing a `uint`.
const ROUTES: &str = "examples/routes.nvs";

/// A program with no `#[Route]` anywhere in it.
const HELLO: &str = "examples/hello.nvs";

/// The path of `relative`, which is written from the repository root, as the argument `nvs` takes.
fn in_repo(relative: &str) -> String {
    nvs_repo::path(relative).display().to_string()
}

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
    let (doc, _, ok) = build(&in_repo(ROUTES));
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
    let (first, _, ok) = build(&in_repo(ROUTES));
    let (second, _, _) = build(&in_repo(ROUTES));
    assert!(ok, "the fixture compiles");
    assert_eq!(first, second, "the emitter is not deterministic");
}

/// § 1's "a program with no `#[Route]` generates nothing and runs no pass": no
/// document, a sentence saying why, and a success exit — having nothing to emit
/// is not a failure.
#[test]
fn a_program_declaring_no_route_emits_no_document() {
    let (doc, err, ok) = build(&in_repo(HELLO));
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
    let missing = in_repo("examples/nope.nvs");
    let (doc, _, ok) = build(&missing);
    assert!(!ok, "a file that cannot be read is a failure");
    assert!(doc.is_empty(), "and writes no document: {doc}");
}

/// § 1's last row: the summary is the doc comment's first sentence, the
/// description is the rest, and a method that wrote no doc comment carries
/// neither member.
///
/// The bound is asserted on both sides on purpose — an emitter that wrote a
/// `summary` for every operation would pass the first half alone, and one that
/// wrote none would pass the second. The `//`-commented, `/* */`-commented and
/// `/** */`-commented handlers are the third side: a doc comment is `///`
/// (`rule:tooling/doc-comment-is-three-slashes`) and prose above a declaration
/// in any other spelling is a note to the next reader of the source, not text
/// this document may publish.
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

    // None of the three spellings above `create`, `destroy` and `restore` is
    // a doc comment.
    for (path, verb) in [
        ("/notes", "post"),
        ("/notes/{id}", "delete"),
        ("/notes/{id}", "put"),
    ] {
        let bare = operation(path, verb);
        assert!(
            bare["summary"].is_null() && bare["description"].is_null(),
            "`{verb} {path}` documented nothing, so the operation says nothing:\n{bare}"
        );
    }
    assert!(
        !doc.contains("not a doc comment")
            && !doc.contains("note to the next reader")
            && !doc.contains("PHPDoc"),
        "and no ordinary-comment text reached the document at all:\n{doc}"
    );
}

/// § 1's response body row: the `200` response's schema is the handler's
/// declared return type, through the same `schema` mapping a parameter's is.
///
/// The answers the emitter has to tell apart, in one fixture: a type it
/// maps (`uint`, floor and all), a type it does not (a class, whose fields are
/// `rule:core-classes/derive-attribute`'s codec rather than this document's guess), and `void`, which
/// carries no `content` at all. That last one is what would go wrong
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

/// § 1's *Enumerations* row, and
/// `rule:routing/a-capture-narrows-to-a-closed-set`
/// 's own promise about it — "the generated document emits
/// `enum: [en, de, fr]` with no further work".
///
/// Asserted as whole schema objects rather than as `doc.contains("enum")`,
/// because what would go wrong here is invisible to a substring: a set emitted
/// in some order other than the union's, which § 3's determinism forbids, and a
/// set emitted *beside* a `type` the row never carried. The `int` union is where
/// a stray `type` would show — its members are the segment text they are
/// written with, so they are strings here and `1` in the URL either way, which
/// [`nvs_types::RouteParam::allowed`] owns.
#[test]
fn a_closed_set_parameter_carries_the_unions_members_as_an_enum() {
    let (doc, err, ok) = build(&fixture("narrowed"));
    assert!(ok, "the fixture compiles: {err}");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let params = &document["paths"]["/{lang}/docs/{page}"]["get"]["parameters"];

    assert_eq!(
        params[0],
        serde_json::json!({
            "name": "lang",
            "in": "path",
            "required": true,
            "schema": {"enum": ["en", "de", "fr"]},
        }),
        "a `\"en\"|\"de\"|\"fr\"` capture is the union's members, in its order:\n{doc}"
    );
    assert_eq!(
        params[1]["schema"],
        serde_json::json!({"type": "string"}),
        "the capture beside it is unnarrowed and gains no `enum`:\n{doc}"
    );
    assert_eq!(
        params[2],
        serde_json::json!({
            "name": "depth",
            "in": "query",
            "required": true,
            "schema": {"enum": ["1", "2", "3"]},
        }),
        "a `#[Query]` key narrows by the same walk, `rule:routing/a-query-parameter-is-declared-like-a-capture` giving it § 5's type list:\n{doc}"
    );
}

/// § 1's *Enumerations* row for the other kind of closed set, and
/// `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`
/// 's promise that one spelling serves the route, its links and this document
/// alike.
///
/// Both halves of that rule at once, which is what makes it an assertion rather
/// than a row: a subset whose every admitted case wrote its value lists those
/// integers, and one where any case counted lists the case names. A document
/// that read the backing value under either spelling passes against the first
/// of the two alone.
#[test]
fn an_enum_captures_cases_are_the_enum_row_under_their_own_spelling() {
    let (doc, err, ok) = build(&fixture("narrowed"));
    assert!(ok, "the fixture compiles: {err}");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let params = &document["paths"]["/p/{lang}/{shelf}"]["get"]["parameters"];

    assert_eq!(
        params[0],
        serde_json::json!({
            "name": "lang",
            "in": "path",
            "required": true,
            "schema": {"enum": ["7", "9"]},
        }),
        "a whole enum whose cases wrote their values is spelled by those values:\n{doc}"
    );
    assert_eq!(
        params[1],
        serde_json::json!({
            "name": "shelf",
            "in": "path",
            "required": true,
            "schema": {"enum": ["New", "Sale"]},
        }),
        "a subset of a counted enum is the admitted case names, and never the case it leaves out:\n{doc}"
    );
}

/// § 1 by way of `rule:routing/a-capture-narrows-to-a-closed-set`'s other named capture type: `Core\Uuid` is the one
/// class a segment converts to, and `format: uuid` is what the JSON Schema
/// dialect 3.1 uses already registers for it. An empty schema in its place would
/// say *any string, or any number, or any object*.
#[test]
fn a_uuid_capture_is_a_string_with_the_registered_format() {
    let (doc, err, ok) = build(&fixture("narrowed"));
    assert!(ok, "the fixture compiles: {err}");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");

    assert_eq!(
        document["paths"]["/docs/{id}"]["get"]["parameters"][0]["schema"],
        serde_json::json!({"type": "string", "format": "uuid"}),
        "a `Core\\Uuid` capture is a formatted string:\n{doc}"
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

/// One fixture of `tests/fixtures/api`, each one file. The diff family below
/// takes `base`, `base` with an operation deleted, and `base` with one optional
/// `#[Query]` parameter added.
fn fixture(stem: &str) -> String {
    format!(
        "{}/tests/fixtures/api/{stem}.nvs",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// `nvs build --openapi` for a fixture, written to a file under the target
/// directory cargo gives an integration test for exactly this.
///
/// **A fresh file per call, not one per stem.** Several tests here ask for
/// `base`, `cargo test` runs them on their own threads, and a reader that
/// catches another thread's `fs::write` half-done gets a truncated document and
/// a diff that reports nothing — which is a flake in exactly the cases the
/// goal records under `data/goals/` name as acceptance checks. The counter is what stops two
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
// covers: tools:cli/nvs-api-diff
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
/// a default, which `rule:routing/a-query-parameter-is-declared-like-a-capture` makes optional.
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

/// A document nested far past what the JSON reader accepts is refused as a
/// document, with the file named, and never reaches the schema walk, which
/// recurses once per level. A pipeline can hand the gate any file, so this is
/// the depth an attacker picks.
// covers: tools:cli/nvs-api-diff
#[test]
fn an_api_diff_of_a_document_nested_past_the_reader_limit_is_a_failure() {
    let mut schema = String::from(r#"{"type":"string"}"#);
    for _ in 0..5000 {
        schema = format!(r#"{{"type":"object","properties":{{"x":{schema}}}}}"#);
    }
    let deep = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("nested-5000-levels.json");
    std::fs::write(
        &deep,
        format!(r#"{{"openapi":"3.1.0","paths":{{"/p":{{"post":{{"requestBody":{{"content":{{"application/json":{{"schema":{schema}}}}}}},"responses":{{}}}}}}}}}}"#),
    )
    .expect("the target directory is writable");
    let (report, err, ok) = api_diff(&document("base"), &deep);
    assert!(
        !ok,
        "a document too deep to read is a failure:\n{report}{err}"
    );
    assert!(
        err.contains("nested-5000-levels.json") && err.contains("not an OpenAPI document"),
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
// asserted on both sides on purpose: the refusing fixtures alone would pass
// just as well against an `#[Api]` that refused everything it was handed, and
// the accepting one alone would pass against an `#[Api]` nothing checked at
// all. `crates/nvs-types/src/routes.rs`'s `check_api` owns which half of § 2's
// `security` rule is askable today and why the other half is not.
// -------------------------------------------------------------------------

/// § 2's contradictions, each named by the message that tells it from the
/// others -- they share `E0771`, because § 2 states one rule that several
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

    // The other side of the bound: the same fields, agreeing.
    let (doc, err, ok) = build(&fixture("api-that-agrees"));
    assert!(ok, "an `#[Api]` that agrees with its code builds: {err}");
    assert!(
        doc.contains("\"operationId\": \"Items::show\""),
        "and the operation it annotates is in the document:\n{doc}"
    );
}

/// § 2's values in the document, read back off the operation they annotate
/// — and absent from the operation beside it that declares no `#[Api]`.
///
/// One test over one fixture holding both rows, rather than a test per
/// value: § 2's promise is that the annotation reaches the document *as
/// written*, and a per-value test passes just as well against an emitter that
/// writes each value into a member of its own choosing. The `#[Api]`-less row
/// is the half that catches the opposite mistake — a `tags: []` or a `security:
/// []` written for every operation reads as an answer where the code gave none.
// covers: lang:attributes/core-api-and-the-openapi-document
#[test]
fn an_apis_four_values_reach_the_operation_and_nothing_else() {
    let (doc, err, ok) = build(&fixture("api-that-agrees"));
    assert!(ok, "the fixture compiles: {err}");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let annotated = document["paths"]["/items/{id}"]["get"].clone();
    let bare = document["paths"]["/items"]["get"].clone();

    assert_eq!(
        annotated["tags"],
        serde_json::json!(["Items"]),
        "`tags` as written:\n{annotated}"
    );
    assert_eq!(
        annotated["security"],
        serde_json::json!([{"bearer": []}]),
        "one 3.1 security requirement per scheme name, each with no scopes:\n{annotated}"
    );
    assert_eq!(
        annotated["responses"]["500"]["description"], "RuntimeError",
        "the `errors` entry is a response of its own, described by its class:\n{annotated}"
    );
    assert!(
        annotated["responses"]["500"]["content"].is_null(),
        "and with no schema, which is the class-body gap this module records:\n{annotated}"
    );
    assert_eq!(
        annotated["responses"]["200"]["content"]["application/json"]["example"],
        serde_json::json!({"name": "a thing", "count": 3}),
        "the example sits beside the schema it is an example of:\n{annotated}"
    );

    assert!(
        bare["tags"].is_null() && bare["security"].is_null(),
        "a route with no `#[Api]` carries neither member:\n{bare}"
    );
    let responses = bare["responses"]
        .as_object()
        .unwrap_or_else(|| panic!("every operation answers something:\n{bare}"));
    assert_eq!(
        responses.keys().collect::<Vec<_>>(),
        ["200"],
        "and answers § 1's declared return type alone:\n{bare}"
    );
    assert!(
        bare["responses"]["200"]["content"]["application/json"]["example"].is_null(),
        "with no example, since nothing wrote one:\n{bare}"
    );
}

// -------------------------------------------------------------------------
// § 1's type list at its last entry: a class a segment reaches through its own
// `parse`. The two tests are one bound asserted on both sides — what a class
// answers, and what the one class the engine ships answers over it.

/// A capture declared at a `Parses` class is the string its `parse` is given
/// and nothing narrower: the contract says the text either parses or does not,
/// never which texts do, so there is no set to publish. The `#[Query]` key
/// beside it is the same answer by the same walk
/// (`rule:routing/a-query-parameter-is-declared-like-a-capture`), which is what
/// holds the *second* row builder to the first — a class and an enum render
/// alike, so a row that did not say which would leave both of these the empty
/// schema, meaning *any string, or any number, or any object*.
#[test]
fn a_parses_capture_is_a_string_schema() {
    let (doc, err, ok) = build(&fixture("parses"));
    assert!(ok, "the fixture compiles: {err}");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let params = &document["paths"]["/posts/{slug}"]["get"]["parameters"];

    assert_eq!(
        params[0],
        serde_json::json!({
            "name": "slug",
            "in": "path",
            "required": true,
            "schema": {"type": "string"},
        }),
        "a capture at a class built from text is a bare string:\n{doc}"
    );
    assert_eq!(
        params[1]["schema"],
        serde_json::json!({"type": "string"}),
        "and a `#[Query]` key at that class is the same schema:\n{doc}"
    );
}

/// `Core\Uuid` implements that same contract, so a document written off the
/// contract alone would drop `format: uuid` from the type the engine ships. The
/// format is a documentation hint over the string schema every implementor
/// answers, registered by the JSON Schema dialect for a type this binary owns —
/// not a conversion rule, and not something a class may declare for itself.
#[test]
fn core_uuid_keeps_its_named_format_over_that_schema() {
    let (doc, err, ok) = build(&fixture("parses"));
    assert!(ok, "the fixture compiles: {err}");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");

    assert_eq!(
        document["paths"]["/posts/by-id/{id}"]["get"]["parameters"][0]["schema"],
        serde_json::json!({"type": "string", "format": "uuid"}),
        "the engine's own class keeps its format over the bare string:\n{doc}"
    );
}
