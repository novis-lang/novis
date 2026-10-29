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
    build_in(file, None)
}

/// [`build`], run from `cwd` where one is given, so the `package.toml` the
/// command finds is that directory's.
fn build_in(file: &str, cwd: Option<&std::path::Path>) -> (String, String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let out = command
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .args(["build", "--openapi", file])
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the document is UTF-8"),
        String::from_utf8(out.stderr).expect("diagnostics are UTF-8"),
        out.status.success(),
    )
}

/// `rule:routing/api-document-is-generated-from-the-route-table`: every route in
/// the table appears exactly once
/// in the document, at its own path, under its own verb, carrying the
/// `operationId` its method gave it.
// covers: tools:cli/nvs-build-openapi
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
        "\"operationId\": \"App.Users.index\"",
        "\"operationId\": \"App.Users.create\"",
        "\"operationId\": \"App.Users.show\"",
    ] {
        assert_eq!(
            doc.matches(fragment).count(),
            1,
            "`{fragment}` appears exactly once in\n{doc}"
        );
    }

    // Path parameters by declared type: `show(uint $id)`, so the
    // capture is a non-negative integer and the document says both halves.
    assert!(
        doc.contains("\"name\": \"id\"")
            && doc.contains("\"in\": \"path\"")
            && doc.contains("\"required\": true")
            && doc.contains("\"minimum\": 0"),
        "the `{{id}}` capture is a required `uint` path parameter:\n{doc}"
    );
}

/// `rule:routing/api-document-is-a-deterministic-build-artifact`: two builds of
/// the same source produce byte-identical documents, which is what makes
/// `nvs api diff` mean anything. The cheapest way for it not to be is
/// a hash map in the emitter, and this is what would catch one.
// covers: tools:cli/nvs-build-openapi
#[test]
fn an_openapi_document_is_byte_identical_across_two_emissions() {
    let (first, _, ok) = build(&in_repo(ROUTES));
    let (second, _, _) = build(&in_repo(ROUTES));
    assert!(ok, "the fixture compiles");
    assert_eq!(first, second, "the emitter is not deterministic");
}

/// "A program with no `#[Route]` generates nothing and runs no pass": no
/// document, a sentence saying why, and a success exit — having nothing to emit
/// is not a failure.
// covers: tools:cli/nvs-build-openapi
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

/// A fresh folder under the target directory, named `name`, holding a
/// `package.toml` of `manifest` when one is given.
fn folder(name: &str, manifest: Option<&str>) -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("openapi-version")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the target directory is writable");
    if let Some(manifest) = manifest {
        std::fs::write(dir.join("package.toml"), manifest).expect("the folder is writable");
    }
    dir
}

/// `info.version` of the document `nvs build --openapi` writes from `cwd`.
fn info_version(cwd: &std::path::Path) -> String {
    let (doc, err, ok) = build_in(&in_repo(ROUTES), Some(cwd));
    assert!(ok, "the fixture compiles: {err}");
    let doc: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    doc["info"]["version"]
        .as_str()
        .expect("`info.version` is a string")
        .to_owned()
}

/// `rule:packaging/package-toml-is-a-tool-input`: the nearest `package.toml`'s
/// `version` is the document's `info.version`, found from a folder below it.
// covers: tools:cli/nvs-build-openapi
#[test]
fn info_version_is_the_package_tomls_version() {
    let dir = folder(
        "with",
        Some("[package]\nname = 'Acme\\Shop'\nversion = \"2.3.4\"\n"),
    );
    let below = dir.join("src");
    std::fs::create_dir_all(&below).expect("the folder is writable");
    assert_eq!(info_version(&below), "2.3.4");
}

/// With no `package.toml`, or one without a `version`, `info.version` is
/// `0.0.0`, which 3.1 needs a document to carry.
#[test]
fn info_version_is_zero_without_a_version_to_read() {
    assert_eq!(info_version(&folder("without", None)), "0.0.0");
    let application = folder("application", Some("[dependencies]\n"));
    assert_eq!(info_version(&application), "0.0.0");
}

/// A `package.toml` the reader refuses stops the build, and no document is
/// written with a version it could not read.
#[test]
fn a_package_toml_that_is_refused_writes_no_document() {
    let dir = folder(
        "refused",
        Some("[package]\nname = 'Acme\\Shop'\nversion = \"2.3\"\n"),
    );
    let (doc, err, ok) = build_in(&in_repo(ROUTES), Some(&dir));
    assert!(!ok, "a version that is not three numbers fails: {doc}");
    assert!(doc.is_empty(), "and writes no document: {doc}");
    assert!(
        err.contains("`2.3` is not a version"),
        "and says why: {err}"
    );
}

/// Two routes the router serves apart, `/files/{path}` and `/files/{path...}`,
/// write one 3.1 template. The command names both and writes nothing, so no
/// operation is dropped from the document unseen.
#[test]
fn two_routes_that_write_one_template_write_no_document() {
    let (doc, err, ok) = build(&fixture("one-template-twice"));
    assert!(!ok, "the pair is an error: {doc}");
    assert!(doc.is_empty(), "and writes no document: {doc}");
    assert!(
        err.contains("Files::one") && err.contains("Files::all") && err.contains("`/files/{path}`"),
        "and names both routes and the template: {err}"
    );
}

/// A program the front end refuses emits no document — the rule's whole premise
/// is that the document cannot say what the compiler did not agree to, so a
/// partial one is worse than none.
#[test]
fn a_program_with_a_diagnostic_emits_no_document() {
    let missing = in_repo("examples/nope.nvs");
    let (doc, _, ok) = build(&missing);
    assert!(!ok, "a file that cannot be read is a failure");
    assert!(doc.is_empty(), "and writes no document: {doc}");
}

/// The attack written against the document: doc comments holding quotes, a
/// backslash, a tab, a JSON fragment that tries to open a `paths` entry of its
/// own, a closing `</script>` and text outside ASCII. The output parses as
/// JSON, it has the two paths the program declares and no third, and each
/// text reads back exactly as written.
// covers: tools:cli/nvs-build-openapi
#[test]
fn a_doc_comment_full_of_json_syntax_reaches_the_document_as_text() {
    let (doc, err, ok) = build(&in_repo(
        "tests/hostile/tools/cli/nvs-build-openapi/01-doc-comments-that-break-the-document.nvs",
    ));
    assert!(ok, "the attack compiles, so the emitter runs: {err}");
    let parsed: serde_json::Value = serde_json::from_str(&doc).expect("the document is valid JSON");

    let paths = parsed["paths"]
        .as_object()
        .expect("the document has a `paths` object");
    let mut keys: Vec<&str> = paths.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["/notes", "/notes/{id}/{version}"],
        "the paths the program declares, and not the one a comment wrote"
    );

    let index = &paths["/notes"]["get"];
    assert_eq!(
        index["summary"], "Lists \"all\" notes \\ every one of them.",
        "the first sentence, quotes and backslash intact"
    );
    assert_eq!(
        index["description"],
        "A tab is here:\tand \"}, \"paths\": {\"/admin\": {}}, \"x\": \" tries to add a path.",
        "the rest, as one string"
    );
    assert_eq!(
        paths["/notes/{id}/{version}"]["get"]["summary"],
        "Shows one note </script><!-- and ends a script block ✓ ünïcödé 🗒.",
        "text outside ASCII is kept byte for byte"
    );
}

/// The doc-comment row: the summary is the doc comment's first sentence, the
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

/// The response body row: the `200` response's schema is the handler's
/// declared return type, through the same `schema` mapping a parameter's is.
///
/// The answers the emitter has to tell apart, in one fixture: a type it
/// maps (`uint`, floor and all), a class with no JSON codec, which has no
/// fields this document could name, and `void`, which
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
        "a class with no `#[Json\\Derive]` has no JSON form, so its schema is the empty one:\n{doc}"
    );

    let empty = ok_response("/notes/ping");
    assert!(
        empty["content"].is_null(),
        "a `void` handler answers with no body, which is not a body admitting anything:\n{empty}"
    );
}

/// The document for `class-response.nvs`, parsed.
fn class_response() -> serde_json::Value {
    let (doc, err, ok) = build(&fixture("class-response"));
    assert!(ok, "the fixture compiles: {err}");
    serde_json::from_str(&doc).expect("the document is JSON")
}

/// The `200` schema of `GET <path>` in `document`.
fn ok_schema(document: &serde_json::Value, path: &str) -> serde_json::Value {
    document["paths"][path]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
        .clone()
}

/// A class response is a `$ref` to a `components.schemas` entry, and the entry
/// is the class's `#[Json\Derive]` codec: each field under its wire key, with
/// its type, and `required` where a document must carry the key.
///
/// The fixture's class has both kinds of property the codec decides about. A
/// skipped field, public or private, is absent. A private property the codec
/// writes is present, because the document describes the wire and not the
/// class. A nested class is an entry of its own.
#[test]
fn a_class_response_is_its_codecs_schema_under_a_ref() {
    let document = class_response();
    assert_eq!(
        ok_schema(&document, "/articles/first"),
        serde_json::json!({"$ref": "#/components/schemas/Article"}),
        "a class response is a reference to its entry:\n{document}"
    );
    let article = &document["components"]["schemas"]["Article"];
    assert_eq!(
        article,
        &serde_json::json!({
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "word_count": {"type": "integer", "minimum": 0},
                "subtitle": {"type": ["string", "null"]},
                "labels": {"type": "array", "items": {"type": "string"}},
                "status": {"type": "integer", "enum": [1, 2]},
                "author": {"$ref": "#/components/schemas/Author"},
                "owner": {"type": "string"},
                "rank": {"type": "integer"},
            },
            "required": ["title", "word_count", "subtitle", "labels", "status", "author", "owner"],
        }),
        "the codec's fields: `cache` and `secret` are skipped, and `rank` has a default:\n{document}"
    );
    assert_eq!(
        document["components"]["schemas"]["Author"],
        serde_json::json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"],
        }),
        "a nested class is its own entry:\n{document}"
    );
    assert_eq!(
        ok_schema(&document, "/articles/all"),
        serde_json::json!({"type": "array", "items": {"$ref": "#/components/schemas/Article"}}),
        "a list of a class refers to the same entry:\n{document}"
    );
}

/// An `errors` entry whose class has a JSON codec carries that codec's schema
/// under its status, the same way a `200` does.
#[test]
fn an_errors_entrys_class_has_its_codecs_schema() {
    let document = class_response();
    let missing = &document["paths"]["/articles/first"]["get"]["responses"]["404"];
    assert_eq!(missing["description"], "Missing", "{document}");
    assert_eq!(
        missing["content"]["application/json"]["schema"],
        serde_json::json!({"$ref": "#/components/schemas/Missing"}),
        "the error response refers to its class's entry:\n{document}"
    );
    assert_eq!(
        document["components"]["schemas"]["Missing"]["properties"],
        serde_json::json!({"reason": {"type": "string"}}),
        "{document}"
    );
}

/// A `?T` return is `T`'s schema with `null` allowed.
#[test]
fn a_nullable_return_allows_null_beside_its_type() {
    let document = class_response();
    assert_eq!(
        ok_schema(&document, "/articles/count"),
        serde_json::json!({"type": ["integer", "null"]}),
        "{document}"
    );
}

/// An `array<T>` return is an array whose `items` are `T`'s schema.
#[test]
fn an_array_return_has_its_element_as_items() {
    let document = class_response();
    assert_eq!(
        ok_schema(&document, "/articles/ranks"),
        serde_json::json!({"type": "array", "items": {"type": "integer"}}),
        "{document}"
    );
}

/// An enum return lists its cases' backing values, which is what `Core\Json`
/// writes a case as.
#[test]
fn an_enum_return_lists_its_backing_values() {
    let document = class_response();
    assert_eq!(
        ok_schema(&document, "/articles/status"),
        serde_json::json!({"type": "integer", "enum": [1, 2]}),
        "{document}"
    );
}

/// The document for `request-body.nvs`, parsed.
fn request_body() -> serde_json::Value {
    let (doc, err, ok) = build(&fixture("request-body"));
    assert!(ok, "the fixture compiles: {err}");
    serde_json::from_str(&doc).expect("the document is JSON")
}

/// The `requestBody` of `POST <path>` in `document`.
fn body_of(document: &serde_json::Value, path: &str) -> serde_json::Value {
    document["paths"][path]["post"]["requestBody"].clone()
}

/// `rule:routing/a-request-body-is-read-from-the-handlers-own-call`: a
/// `jsonAs<T>()` in the handler is a required `application/json` body with
/// `T`'s schema, because `jsonAs` throws on an empty body.
#[test]
fn a_json_read_in_the_handler_is_its_required_request_body() {
    let document = request_body();
    assert_eq!(
        body_of(&document, "/notes/json"),
        serde_json::json!({
            "required": true,
            "content": {"application/json": {"schema": {"$ref": "#/components/schemas/NewNote"}}},
        }),
        "{document}"
    );
    assert_eq!(
        document["components"]["schemas"]["NewNote"],
        serde_json::json!({
            "type": "object",
            "properties": {"text": {"type": "string"}},
            "required": ["text"],
        }),
        "the body's class is an entry of its own:\n{document}"
    );
}

/// A `postAs<T>()` is a form body. It is required only when `T` has a
/// required field, because `postAs` throws only when one is missing.
#[test]
fn a_form_read_is_a_form_body_required_when_its_class_needs_a_field() {
    let document = request_body();
    assert_eq!(
        body_of(&document, "/notes/form"),
        serde_json::json!({
            "required": true,
            "content": {
                "application/x-www-form-urlencoded": {
                    "schema": {"$ref": "#/components/schemas/NewNote"},
                },
            },
        }),
        "{document}"
    );
    assert_eq!(
        body_of(&document, "/notes/search")["required"],
        false,
        "every field of `Search` has a default:\n{document}"
    );
}

/// A handler with a JSON read and a form read has both media types, and two
/// JSON reads of two types are a `oneOf` that names each type once.
#[test]
fn both_kinds_are_two_media_types_and_two_types_of_one_kind_are_a_one_of() {
    let document = request_body();
    assert_eq!(
        body_of(&document, "/notes/both")["content"],
        serde_json::json!({
            "application/json": {"schema": {"$ref": "#/components/schemas/NewNote"}},
            "application/x-www-form-urlencoded": {
                "schema": {"$ref": "#/components/schemas/Search"},
            },
        }),
        "{document}"
    );
    assert_eq!(
        body_of(&document, "/notes/either")["content"]["application/json"]["schema"],
        serde_json::json!({"oneOf": [
            {"$ref": "#/components/schemas/NewNote"},
            {"$ref": "#/components/schemas/Draft"},
        ]}),
        "{document}"
    );
}

/// A read inside a method the handler calls is not the handler's body, and a
/// `postAs` that writes `name` reads one field. Neither route has a
/// `requestBody`.
#[test]
fn a_read_in_a_helper_or_of_one_field_gives_no_request_body() {
    let document = request_body();
    for path in ["/notes/helper", "/notes/field"] {
        assert!(
            body_of(&document, path).is_null(),
            "{path} has no `requestBody`:\n{document}"
        );
    }
}

/// The document's *Enumerations* row, and
/// `rule:routing/a-capture-narrows-to-a-closed-set`
/// 's own promise about it — "the generated document emits
/// `enum: [en, de, fr]` with no further work".
///
/// Asserted as whole schema objects rather than as `doc.contains("enum")`,
/// because what would go wrong here is invisible to a substring: a set emitted
/// in some order other than the union's, which determinism forbids, and a
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
        "a `#[Query]` key narrows by the same walk, `rule:routing/a-query-parameter-is-declared-like-a-capture` giving it a capture's type list:\n{doc}"
    );
}

/// The document's *Enumerations* row for the other kind of closed set, and
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

/// The document by way of `rule:routing/a-capture-narrows-to-a-closed-set`'s other named
/// capture type: `Core\Uuid` is the one
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
// `nvs api diff`, the gate (`rule:routing/api-diff-fails-a-breaking-change`).
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
/// goal files under `docs/goals/` name as acceptance checks. The counter is what stops two
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

/// The gate's first breaking change: removing an operation exits non-zero from
/// `nvs api diff`.
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
/// This rests on the document's determinism, which
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
// `rule:attributes/api-adds-and-cannot-contradict` -- `#[Api]` may add and may not contradict.
//
// One fixture per contradiction plus one that agrees, and the bound is
// asserted on both sides on purpose: the refusing fixtures alone would pass
// just as well against an `#[Api]` that refused everything it was handed, and
// the accepting one alone would pass against an `#[Api]` nothing checked at
// all. `crates/nvs-types/src/routes.rs`'s `check_api` owns which half of the
// `security` rule is askable today and why the other half is not.
// -------------------------------------------------------------------------

/// `#[Api]`'s contradictions, each named by the message that tells it from the
/// others -- they share `E0771`, because it is one rule that several
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
        doc.contains("\"operationId\": \"Items.show\""),
        "and the operation it annotates is in the document:\n{doc}"
    );
}

/// `rule:routing/a-method-is-one-endpoint-everywhere`'s `operationId`: the
/// method as `Class.method`, the lowercased verb appended where one method
/// serves two verbs, and `#[Api]`'s `operationId` in place of both.
#[test]
fn an_operation_id_is_the_method_unless_its_api_writes_one() {
    let (doc, err, ok) = build(&fixture("operation-id"));
    assert!(ok, "the fixture compiles: {err}");
    let doc: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let id = |path: &str, verb: &str| doc["paths"][path][verb]["operationId"].clone();
    assert_eq!(id("/products/{id}", "get"), "Shop.show");
    assert_eq!(id("/cart", "post"), "Shop.cart.post");
    assert_eq!(id("/cart", "put"), "Shop.cart.put");
    assert_eq!(id("/products", "get"), "listProducts");
}

/// `rule:attributes/api-adds-and-cannot-contradict`: two methods whose
/// `#[Api]` write one `operationId` are `E0771`, naming both.
#[test]
fn one_operation_id_on_two_methods_does_not_compile() {
    let (doc, err, ok) = build(&fixture("operation-id-twice"));
    assert!(!ok, "two methods with one `operationId` do not build");
    assert!(
        err.contains("E0771") && err.contains("the same `operationId` `products`"),
        "the error names the id:\n{err}"
    );
    assert!(
        err.contains("the first method with this `operationId`"),
        "and points at the first method:\n{err}"
    );
    assert!(doc.trim().is_empty(), "and no document is written:\n{doc}");
}

/// `#[Api]`'s values in the document, read back off the operation they annotate
/// — and absent from the operation beside it that declares no `#[Api]`.
///
/// One test over one fixture holding both rows, rather than a test per
/// value: the promise is that the annotation reaches the document *as
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
        "and with no schema, because `RuntimeError` has no JSON codec:\n{annotated}"
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
        "and answers the declared return type alone:\n{bare}"
    );
    assert!(
        bare["responses"]["200"]["content"]["application/json"]["example"].is_null(),
        "with no example, since nothing wrote one:\n{bare}"
    );
}

// -------------------------------------------------------------------------
// `rule:routing/api-document-is-generated-from-the-route-table`'s type list at
// its last entry: a class a segment reaches through its own
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

// -------------------------------------------------------------------------
// `rule:routing/a-security-scheme-is-declared-in-code` -- `#[Api\Scheme]`.
// -------------------------------------------------------------------------

/// Every declared scheme reaches `components.securitySchemes` under its name,
/// sorted, in the specification's spelling, whichever class declared it.
#[test]
fn every_declared_scheme_is_a_security_scheme_component_sorted_by_name() {
    let (doc, err, ok) = build(&fixture("schemes-of-every-type"));
    assert!(ok, "the fixture compiles: {err}");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let schemes = &document["components"]["securitySchemes"];
    assert_eq!(
        *schemes,
        serde_json::json!({
            "bearer": {
                "type": "http",
                "scheme": "bearer",
                "bearerFormat": "JWT",
                "description": "A token from the login route.",
            },
            "key": {"type": "apiKey", "in": "header", "name": "X-Api-Key"},
            "mtls": {"type": "mutualTLS"},
            "oauth": {
                "type": "oauth2",
                "flows": {
                    "authorizationCode": {
                        "authorizationUrl": "https://example.com/authorize",
                        "tokenUrl": "https://example.com/token",
                        "scopes": {"orders:read": "Read orders", "orders:write": "Change orders"},
                    },
                    "clientCredentials": {"tokenUrl": "https://example.com/token", "scopes": {}},
                },
            },
            "openid": {
                "type": "openIdConnect",
                "openIdConnectUrl": "https://example.com/.well-known/openid-configuration",
            },
        }),
        "{doc}"
    );
    let names: Vec<&String> = schemes.as_object().expect("an object").keys().collect();
    let at = |name: &str| doc.find(&format!("\"{name}\": {{")).expect(name);
    assert!(
        at("bearer") < at("key") && at("key") < at("mtls") && at("oauth") < at("openid"),
        "the written document lists {names:?} by name:\n{doc}"
    );
    assert_eq!(
        document["paths"]["/items"]["get"]["security"],
        serde_json::json!([{"bearer": []}, {"oauth": []}]),
        "and the operation names two of them"
    );

    let (doc, err, ok) = build(&fixture("api-that-agrees"));
    assert!(ok, "{err}");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert_eq!(
        document["components"]["securitySchemes"]["bearer"]["type"], "http",
        "{doc}"
    );
    let (doc, err, ok) = build(&in_repo(ROUTES));
    assert!(ok, "{err}");
    assert!(
        !doc.contains("\"components\""),
        "a program that declares no scheme writes no components:\n{doc}"
    );
}

/// The three ways a scheme declaration and its uses disagree, each its own
/// code, and no document for any of them.
#[test]
fn a_scheme_that_is_undeclared_repeated_or_misfit_does_not_compile() {
    let cases: [(&str, &[&str]); 3] = [
        (
            "scheme-security-undeclared",
            &[
                "E0886",
                "`beraer` is not a declared security scheme",
                "the declared schemes are: `bearer`, `key`",
            ],
        ),
        (
            "scheme-declared-twice",
            &["E0887", "the security scheme `bearer` is declared twice"],
        ),
        (
            "scheme-fields-that-do-not-fit",
            &[
                "E0888",
                "this `Http` scheme does not use `in`",
                "this `ApiKey` scheme needs `parameter`",
                "the `password` flow needs `tokenUrl`",
            ],
        ),
    ];
    for (stem, messages) in cases {
        let (doc, err, ok) = build(&fixture(stem));
        assert!(!ok, "`{stem}` does not build");
        for message in messages {
            assert!(
                err.contains(message),
                "`{stem}` reports `{message}`:\n{err}"
            );
        }
        assert!(doc.trim().is_empty(), "and writes no document:\n{doc}");
    }
    let (_, err, _) = build(&fixture("scheme-fields-that-do-not-fit"));
    assert!(
        !err.contains("E0886"),
        "a refused scheme is still a declared name:\n{err}"
    );
}

/// The official OpenAPI 3.1 schema, with the dialect that checks every Schema
/// Object in a document, built offline from the copies under
/// `tests/fixtures/openapi`. The 2020-12 meta-schemas the dialect names are
/// the validator's own. The copies are the OpenAPI Initiative's, under
/// Apache-2.0, as `spec.openapis.org/oas/3.1/` publishes them: `schema` and
/// `schema-base` of 2025-09-15, and `dialect` and `meta` of 2024-11-10.
fn strict_validator() -> jsonschema::Validator {
    let vendored = |name: &str| -> serde_json::Value {
        let path = format!(
            "{}/tests/fixtures/openapi/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(&path).expect("the vendored schema is readable");
        serde_json::from_str(&text).expect("the vendored schema is JSON")
    };
    let [schema, base, dialect, meta] =
        ["schema-3.1", "schema-base-3.1", "dialect-3.1", "meta-3.1"].map(vendored);
    let id = |value: &serde_json::Value| {
        value["$id"]
            .as_str()
            .expect("every schema has an `$id`")
            .to_owned()
    };
    let registry = jsonschema::Registry::new()
        .add(id(&schema), schema)
        .and_then(|registry| registry.add(id(&dialect), dialect))
        .and_then(|registry| registry.add(id(&meta), meta))
        .and_then(|registry| registry.prepare())
        .expect("the vendored schemas register");
    jsonschema::options()
        .offline()
        .with_registry(&registry)
        .build(&base)
        .expect("the OpenAPI 3.1 schema builds")
}

/// What the schema cannot check, because it is a question about two places in
/// one document: every `$ref` points at a member that exists, every `security`
/// name is a declared scheme, every `operationId` is unique, and every path
/// template's `{name}` is exactly one `path` parameter of each operation under
/// it. The problems found, one line each.
fn cross_references(document: &serde_json::Value) -> Vec<String> {
    fn refs<'a>(value: &'a serde_json::Value, found: &mut Vec<&'a str>) {
        match value {
            serde_json::Value::Object(object) => {
                if let Some(serde_json::Value::String(target)) = object.get("$ref") {
                    found.push(target);
                }
                object.values().for_each(|member| refs(member, found));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|item| refs(item, found)),
            _ => {}
        }
    }
    let mut problems = Vec::new();
    let mut found = Vec::new();
    refs(document, &mut found);
    for target in found {
        let resolves = target
            .strip_prefix('#')
            .is_some_and(|pointer| document.pointer(pointer).is_some());
        if !resolves {
            problems.push(format!("`$ref` {target} points at nothing"));
        }
    }
    let schemes = &document["components"]["securitySchemes"];
    let mut ids = std::collections::BTreeSet::new();
    let paths = document["paths"].as_object().expect("`paths` is an object");
    for (path, operations) in paths {
        let templated: std::collections::BTreeSet<&str> = path
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect();
        for (verb, operation) in operations.as_object().expect("a path item is an object") {
            let id = operation["operationId"].as_str().unwrap_or_default();
            if !ids.insert(id.to_owned()) {
                problems.push(format!("`operationId` {id} is used twice"));
            }
            for requirement in operation["security"].as_array().into_iter().flatten() {
                for name in requirement
                    .as_object()
                    .into_iter()
                    .flat_map(|names| names.keys())
                {
                    if schemes.get(name).is_none() {
                        problems.push(format!(
                            "{verb} {path} names the undeclared scheme `{name}`"
                        ));
                    }
                }
            }
            let params: Vec<&str> = operation["parameters"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|param| param["in"] == "path")
                .filter_map(|param| param["name"].as_str())
                .collect();
            let declared: std::collections::BTreeSet<&str> = params.iter().copied().collect();
            if declared != templated || declared.len() != params.len() {
                problems.push(format!(
                    "{verb} {path} has the path parameters {params:?}, and its template names {templated:?}"
                ));
            }
        }
    }
    problems
}

/// A document is valid OpenAPI 3.1 when the official schema accepts it, with
/// every Schema Object held to the OpenAPI dialect, and when every reference
/// inside it resolves. Checked over the fixture that uses every feature the
/// document renders, and over every other fixture that compiles.
// covers: tools:cli/nvs-build-openapi
#[test]
fn a_strict_openapi_31_validator_accepts_every_document_the_fixtures_build() {
    let validator = strict_validator();
    let wrong = |parameter: serde_json::Value| {
        serde_json::json!({
            "openapi": "3.1.0",
            "info": {"title": "Shop", "version": "0.0.0"},
            "paths": {"/a/{id}": {"get": {"parameters": [parameter], "responses": {}}}},
        })
    };
    let optional_path = wrong(serde_json::json!(
        {"name": "id", "in": "path", "required": false, "schema": {"type": "string"}}
    ));
    let misspelled_type = wrong(serde_json::json!(
        {"name": "id", "in": "path", "required": true, "schema": {"type": "strng"}}
    ));
    assert!(
        !validator.is_valid(&optional_path) && !validator.is_valid(&misspelled_type),
        "the validator rejects a path parameter that is not required, and a schema the dialect rejects"
    );
    let every_feature = format!(
        "{}/tests/fixtures/openapi/every-feature.nvs",
        env!("CARGO_MANIFEST_DIR")
    );
    let (doc, err, ok) = build(&every_feature);
    assert!(
        ok && err.is_empty(),
        "the every-feature fixture builds cleanly:\n{err}"
    );
    let mut programs = vec![(every_feature, doc)];
    let dir = format!("{}/tests/fixtures/api", env!("CARGO_MANIFEST_DIR"));
    let mut others: Vec<_> = std::fs::read_dir(&dir)
        .expect("the fixtures folder is readable")
        .map(|entry| entry.expect("a folder entry").path())
        .collect();
    others.sort();
    for path in others {
        let file = path.display().to_string();
        let (doc, _, ok) = build(&file);
        if ok && !doc.trim().is_empty() {
            programs.push((file, doc));
        }
    }
    assert!(programs.len() > 5, "most fixtures build a document");

    let mut problems = Vec::new();
    for (file, doc) in &programs {
        let document: serde_json::Value = serde_json::from_str(doc).expect("the document is JSON");
        for error in validator.iter_errors(&document) {
            problems.push(format!("{file}: {} at {}", error, error.instance_path()));
        }
        for problem in cross_references(&document) {
            problems.push(format!("{file}: {problem}"));
        }
    }
    assert!(
        problems.is_empty(),
        "every document is valid:\n{}",
        problems.join("\n")
    );

    let every: serde_json::Value =
        serde_json::from_str(&programs[0].1).expect("the document is JSON");
    assert_eq!(
        every["components"]["securitySchemes"]
            .as_object()
            .map(serde_json::Map::len),
        Some(5),
        "the fixture declares a scheme of each type"
    );
    assert!(
        every["paths"]["/products/page"]["get"]["operationId"] == "Catalog.page.without-page"
            && every["paths"]["/products/page/{page}"]["get"]["parameters"][0]["required"] == true,
        "an optional capture is two paths, and the one that has it requires it:\n{every:#}"
    );
    assert!(
        every["paths"]["/files/{path}"]["get"].is_object(),
        "a rest capture is a plain template:\n{every:#}"
    );
}
