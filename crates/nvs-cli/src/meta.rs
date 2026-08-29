//! `nvs meta --json` — the `Core` registry as JSON, for a consumer outside
//! the build.
//!
//! [ADR 0117](../../../docs/adr/0117-an-implemented-core-member-documents-itself-in-the-registry.md)
//! § 2 is the contract, and this command owns it: every class in
//! [`nvs_stdlib::registry::CLASSES`], every member — static ones first, then
//! instance ones — and, for a member whose row carries a
//! [`MethodDoc`], its reference card under a `doc` key. A consumer ignores
//! fields it does not know, so a field may be added here without breaking one;
//! a field may not be renamed or moved.
//!
//! ## What is omitted, and why
//!
//! A row with no documentation has **no `doc` key at all**, and a documented
//! row's `short`/`return` strings and `params`/`shape`/`errors` arrays appear
//! **only when non-empty**. That is § 3's field-wise precedence made
//! mechanical: `MethodDoc` spells "not written yet" as the empty value, and a
//! consumer must be able to tell that apart from "written, and empty" without
//! learning the convention — an absent key is the one spelling that needs no
//! explanation. The website's `scripts/lib/meta.mjs` is the consumer this was
//! built against.
//!
//! Built and written as JSON rather than as text through the `serde_json`
//! this crate already carries for ADR 0085's document; the workspace manifest
//! owns why this crate and not another.

use std::process::ExitCode;

use nvs_stdlib::registry::{CLASSES, CoreClass, CoreMethod, ErrorDoc, MethodDoc, ParamDoc};
use serde_json::{Map, Value, json};

/// Prints the registry and returns success.
///
/// Nothing here can fail: every value is a compile-time constant of
/// `nvs-stdlib`, and serializing a tree of strings does not error.
pub(crate) fn run() -> ExitCode {
    println!("{}", document());
    ExitCode::SUCCESS
}

/// The whole document — § 2's outermost object.
///
/// Separate from [`run`] so the tests below can assert on it without
/// capturing standard output.
fn document() -> Value {
    json!({ "classes": CLASSES.iter().map(class_json).collect::<Vec<_>>() })
}

/// One class: its name and its members, methods before instance members, each
/// roster in the spec's own order.
fn class_json(class: &CoreClass) -> Value {
    let members: Vec<Value> = class
        .methods
        .iter()
        .chain(class.instance)
        .map(member_json)
        .collect();
    json!({ "name": class.name, "members": members })
}

/// One member: its name, and its `doc` only when the row carries one.
fn member_json(member: &CoreMethod) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(member.name));
    if let Some(doc) = member.doc {
        out.insert("doc".into(), doc_json(doc));
    }
    Value::Object(out)
}

/// One reference card, with every empty field left out — see the module doc.
fn doc_json(doc: &MethodDoc) -> Value {
    let mut out = Map::new();
    put_str(&mut out, "short", doc.short);
    put_list(&mut out, "params", doc.params, param_json);
    put_str(&mut out, "return", doc.ret);
    put_list(&mut out, "errors", doc.errors, error_json);
    Value::Object(out)
}

/// One parameter, with its `shape` only for a shape-typed one.
fn param_json(param: &ParamDoc) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(param.name));
    put_str(&mut out, "desc", param.desc);
    put_list(
        &mut out,
        "shape",
        param.shape,
        |key| json!({ "key": key.key, "type": key.ty, "desc": key.desc }),
    );
    Value::Object(out)
}

/// One thrown error.
fn error_json(error: &ErrorDoc) -> Value {
    json!({ "error": error.error, "desc": error.desc })
}

/// Inserts `key` only when `value` is written.
fn put_str(out: &mut Map<String, Value>, key: &str, value: &'static str) {
    if !value.is_empty() {
        out.insert(key.into(), Value::from(value));
    }
}

/// Inserts `key` as an array only when `items` holds at least one.
fn put_list<T>(out: &mut Map<String, Value>, key: &str, items: &[T], each: impl Fn(&T) -> Value) {
    if !items.is_empty() {
        out.insert(key.into(), Value::Array(items.iter().map(each).collect()));
    }
}

#[cfg(test)]
mod tests {
    use nvs_stdlib::registry::ShapeKeyDoc;

    use super::*;

    /// A card with every field written, including a shape-typed parameter —
    /// which no implemented member declares yet, so the emitter's half of
    /// [`ShapeKeyDoc`] is held here rather than through the binary.
    const FULL: MethodDoc = MethodDoc {
        short: "Does a thing.",
        params: &[ParamDoc {
            name: "parts",
            desc: "The parts.",
            shape: &[ShapeKeyDoc {
                key: "hour",
                ty: "int",
                desc: "The hour.",
            }],
        }],
        ret: "The thing.",
        errors: &[ErrorDoc {
            error: "LogicError",
            desc: "When it cannot.",
        }],
    };

    /// A card with nothing written yet.
    const EMPTY: MethodDoc = MethodDoc {
        short: "",
        params: &[],
        ret: "",
        errors: &[],
    };

    /// § 2's shape, key for key, from a card that fills every field.
    #[test]
    fn a_full_card_emits_every_field_of_the_contract() {
        assert_eq!(
            doc_json(&FULL),
            json!({
                "short": "Does a thing.",
                "params": [ { "name": "parts", "desc": "The parts.",
                              "shape": [ { "key": "hour", "type": "int", "desc": "The hour." } ] } ],
                "return": "The thing.",
                "errors": [ { "error": "LogicError", "desc": "When it cannot." } ]
            })
        );
    }

    /// An unwritten field is an absent key, never an empty one — the module
    /// doc's *What is omitted* rule.
    #[test]
    fn an_empty_card_emits_an_empty_object() {
        assert_eq!(doc_json(&EMPTY), json!({}));
    }

    /// Every class in the registry appears, and every one of its members —
    /// methods and then instance members, in the rosters' own order.
    #[test]
    fn the_document_lists_every_class_and_member_in_registry_order() {
        let document = document();
        let classes = document["classes"].as_array().expect("an array");
        assert_eq!(classes.len(), CLASSES.len());
        for (class, emitted) in CLASSES.iter().zip(classes) {
            assert_eq!(emitted["name"], class.name);
            let names: Vec<&str> = emitted["members"]
                .as_array()
                .expect("an array")
                .iter()
                .map(|m| m["name"].as_str().expect("a name"))
                .collect();
            let expected: Vec<&str> = class
                .methods
                .iter()
                .chain(class.instance)
                .map(|m| m.name)
                .collect();
            assert_eq!(names, expected, "{}", class.name);
        }
    }
}
