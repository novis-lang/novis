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

/// The parsed document's top-level enum `name` names.
fn core_enum(document: &serde_json::Value, name: &str) -> serde_json::Value {
    document["enums"]
        .as_array()
        .expect("`enums` is an array")
        .iter()
        .find(|e| e["name"] == name)
        .unwrap_or_else(|| panic!("{name} is in the document"))
        .clone()
}

/// The parsed document's constant `class::name` names.
fn constant(document: &serde_json::Value, class: &str, name: &str) -> serde_json::Value {
    document["classes"]
        .as_array()
        .expect("`classes` is an array")
        .iter()
        .find(|c| c["name"] == class)
        .unwrap_or_else(|| panic!("{class} is in the document"))["constants"]
        .as_array()
        .expect("`constants` is an array")
        .iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("{class}::{name} is in the document"))
        .clone()
}

/// § 2's golden: `Core\Str::length`, the simple case — the `$name` its one
/// parameter is callable by, a `short`, one documented parameter, a `return`,
/// and no `errors` or `shape` key because neither is written.
#[test]
fn the_golden_for_str_length_matches_the_contract() {
    let (doc, ok) = meta(&["--json"]);
    assert!(ok, "`nvs meta --json` succeeds");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert_eq!(
        member(&document, r"Core\Str", "length"),
        serde_json::json!({
            "name": "length",
            "names": ["s"],
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
/// field it lacks falls back to the spec" starts with the whole card. It still
/// carries `names`, because ADR 0063 R2's by-name surface is signature and not
/// documentation: a consumer building a call needs it where a card is optional.
#[test]
fn an_undocumented_member_carries_no_doc_key() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let at = member(&document, r"Core\Str", "at");
    assert_eq!(
        at,
        serde_json::json!({ "name": "at", "names": ["s", "index"] })
    );
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

/// § 2's enum golden: `Core\Order`, top-level beside `classes`, a `short` and
/// one `cases` entry per case in declaration order.
#[test]
fn the_golden_for_order_matches_the_contract() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert_eq!(
        core_enum(&document, r"Core\Order"),
        serde_json::json!({
            "name": r"Core\Order",
            "doc": {
                "short": "The direction `Core\\Arr::sort` and `sortByKey` put elements in — spec \
                          § 2's `{order: …}` option, which is `Asc` when omitted.",
                "cases": [
                    { "name": "Asc", "desc": "Smallest first — what an omitted `{order: …}` means." },
                    { "name": "Desc", "desc": "Largest first — `rsort`, `arsort` and `krsort` as \
                                               one option rather than three names." }
                ]
            }
        })
    );
}

/// § 2's constant golden: `Core\Math::PI`, under its class's `constants`, its
/// one-sentence card as `doc`.
#[test]
fn the_golden_for_math_pi_matches_the_contract() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert_eq!(
        constant(&document, r"Core\Math", "PI"),
        serde_json::json!({
            "name": "PI",
            "doc": "The ratio of a circle's circumference to its diameter, `3.14159…` as the \
                    nearest `float` — PHP's `M_PI`."
        })
    );
}

/// An enum whose row is not documented has no `doc` key at all, exactly as
/// an undocumented member has none — and a constant with nothing written is
/// its name alone.
#[test]
fn an_undocumented_enum_or_constant_carries_no_doc_key() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert_eq!(
        core_enum(&document, r"Core\SetOn"),
        serde_json::json!({ "name": r"Core\SetOn" })
    );
    assert_eq!(
        constant(&document, r"Core\Math", "TAU"),
        serde_json::json!({ "name": "TAU" })
    );
}

/// `--json` is required: a `meta` that prints nothing would be a subcommand
/// that succeeds having done nothing, exactly as `build` without `--openapi`.
#[test]
fn meta_without_json_is_refused() {
    let (_, ok) = meta(&[]);
    assert!(!ok);
}
