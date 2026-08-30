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
            "kind": "static",
            "names": ["s"],
            "signature": "length(string $s): uint",
            "params": [ { "name": "s", "type": "string", "qualifier": "neutral" } ],
            "returns": "uint",
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

/// Every member the document lists carries a `doc` with a `short` — the
/// registry's own guard `every_registry_row_carries_a_reference_card` seen
/// from the consumer's side, so a card the emitter dropped on the way out
/// would show here. `names` is beside it on every row, because ADR 0063 R2's
/// by-name surface is signature and not documentation. The omission rule
/// itself — no key for a field with nothing written — is proven over
/// synthetic cards in `meta.rs`'s own tests, since no shipped row is
/// undocumented any more.
#[test]
fn every_member_carries_a_doc_key() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let mut undocumented = Vec::new();
    for class in document["classes"]
        .as_array()
        .expect("`classes` is an array")
    {
        for m in class["members"].as_array().expect("`members` is an array") {
            // `names` is absent, not empty, on a member with no parameter —
            // the omission rule again — so it is not a thing to require here.
            let written = m["doc"]["short"].as_str().is_some_and(|s| !s.is_empty());
            if !written {
                undocumented.push(format!("{}::{}", class["name"], m["name"]));
            }
        }
    }
    assert!(
        undocumented.is_empty(),
        "members without a card: {undocumented:?}"
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
    assert_eq!(
        errors.len(),
        1,
        "one entry per class thrown, its conditions in the desc"
    );
    assert_eq!(errors[0]["error"], "RuntimeError");
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
            "type": "float",
            "value": "3.141592653589793",
            "doc": "The ratio of a circle's circumference to its diameter, `3.14159…` as the \
                    nearest `float` — PHP's `M_PI`."
        })
    );
}

/// Every enum and every constant carries its card too — the same guard, on
/// the two rosters a member's card points at. `Core\SetOn` and `Core\Math::TAU`
/// were the last two undocumented entries when this was a test that they
/// carried *no* key, which is why they are the ones named.
#[test]
fn every_enum_and_constant_carries_a_doc_key() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert!(core_enum(&document, r"Core\SetOn")["doc"]["short"].is_string());
    assert!(constant(&document, r"Core\Math", "TAU")["doc"].is_string());
    let mut undocumented = Vec::new();
    for declared in document["enums"].as_array().expect("`enums` is an array") {
        if !declared["doc"]["short"].is_string() {
            undocumented.push(declared["name"].to_string());
        }
    }
    for class in document["classes"]
        .as_array()
        .expect("`classes` is an array")
    {
        // A class with no constants has no `constants` key at all.
        let Some(constants) = class["constants"].as_array() else {
            continue;
        };
        for c in constants {
            if !c["doc"].is_string() {
                undocumented.push(format!("{}::{}", class["name"], c["name"]));
            }
        }
    }
    assert!(
        undocumented.is_empty(),
        "enums and constants without a card: {undocumented:?}"
    );
}

/// `--json` is required: a `meta` that prints nothing would be a subcommand
/// that succeeds having done nothing, exactly as `build` without `--openapi`.
#[test]
fn meta_without_json_is_refused() {
    let (_, ok) = meta(&[]);
    assert!(!ok);
}
