//! `nvs meta --json`, driven as a consumer drives it —
//! [ADR 0117](../../../docs/adr/0117-an-implemented-core-member-documents-itself-in-the-registry.md)'s
//! *Verification* section: a golden of § 2's shape over the first documented
//! member, through the built binary for the same reason `openapi.rs` goes
//! through it — the contract is what the *command* prints.

use std::process::Command;

/// `nvs meta <args...>`, as `(stdout, success)`.
fn meta(args: &[&str]) -> (String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("meta")
        .args(args)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the document is UTF-8"),
        out.status.success(),
    )
}

/// The parsed document, and the one member of it `class::member` names.
fn member(document: &serde_json::Value, class: &str, member: &str) -> serde_json::Value {
    document["classes"]
        .as_array()
        .expect("`classes` is an array")
        .iter()
        .find(|c| c["name"] == class)
        .unwrap_or_else(|| panic!("{class} is in the document"))["members"]
        .as_array()
        .expect("`members` is an array")
        .iter()
        .find(|m| m["name"] == member)
        .unwrap_or_else(|| panic!("{class}::{member} is in the document"))
        .clone()
}

/// § 2's golden: `Core\Str::length`, the simple case — a `short`, one
/// parameter, a `return`, and no `errors` or `shape` key because neither is
/// written.
#[test]
fn the_golden_for_str_length_matches_the_contract() {
    let (doc, ok) = meta(&["--json"]);
    assert!(ok, "`nvs meta --json` succeeds");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert_eq!(
        member(&document, r"Core\Str", "length"),
        serde_json::json!({
            "name": "length",
            "doc": {
                "short": "Counts the graphemes in `$s` — user-perceived characters, ADR 0009's \
                          default unit — so a combining sequence counts once and this is never \
                          a byte count.",
                "params": [ { "name": "s", "desc": "The string to measure." } ],
                "return": "The grapheme count; `0` for the empty string."
            }
        })
    );
}

/// A member whose row is not documented has no `doc` key at all — § 3's "a
/// field it lacks falls back to the spec" starts with the whole card.
#[test]
fn an_undocumented_member_carries_no_doc_key() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let at = member(&document, r"Core\Str", "at");
    assert_eq!(at, serde_json::json!({ "name": "at" }));
}

/// The two other members documented as proof: an options bag is one `params`
/// entry per option under the option's own name, and a thrown error is an
/// `errors` entry naming the class a `catch` writes.
#[test]
fn the_options_and_errors_of_the_proof_members_are_emitted() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");

    let encode = member(&document, r"Core\Json", "encode");
    let names: Vec<&str> = encode["doc"]["params"]
        .as_array()
        .expect("params")
        .iter()
        .map(|p| p["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, ["value", "pretty", "escapeUnicode"]);
    assert_eq!(encode["doc"]["errors"][0]["error"], "LogicError");

    let matched = member(&document, r"Core\Regex", "match");
    let errors = matched["doc"]["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|e| e["error"] == "RuntimeError"));
}

/// `--json` is required: a `meta` that prints nothing would be a subcommand
/// that succeeds having done nothing, exactly as `build` without `--openapi`.
#[test]
fn meta_without_json_is_refused() {
    let (_, ok) = meta(&[]);
    assert!(!ok);
}
