//! `nvs build --openapi`, driven as a user drives it —
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
