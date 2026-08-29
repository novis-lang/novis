//! `nvs meta --json` — the `Core` registry as JSON, for a consumer outside
//! the build.
//!
//! [ADR 0117](../../../docs/adr/0117-an-implemented-core-member-documents-itself-in-the-registry.md)
//! § 2 is the contract, and this command owns it: every class in
//! [`nvs_stdlib::registry::CLASSES`], every member — static ones first, then
//! instance ones — with the `$name` each of its positional parameters is
//! callable by under `names` ([ADR 0063](../../../docs/adr/0063-core-api-conventions.md)
//! R2) and, for a member whose row carries a
//! [`MethodDoc`], its reference card under a `doc` key; every constant of the
//! class beside its members, with its one-sentence card as its `doc`; and,
//! top-level beside `classes`, every enum in
//! [`nvs_stdlib::registry::ENUMS`] with its [`EnumDoc`] — top-level because
//! that roster is, an enum having no owner class in the registry. A consumer
//! ignores fields it does not know, so a field may be added here without
//! breaking one; a field may not be renamed or moved.
//!
//! ## What is omitted, and why
//!
//! A row with no documentation has **no `doc` key at all**, and a documented
//! row's `short`/`return` strings and `params`/`shape`/`errors`/`cases` arrays
//! appear **only when non-empty** — a class with no constants has no
//! `constants` key either. That is § 3's field-wise precedence made
//! mechanical: the registry spells "not written yet" as the empty value, and a
//! consumer must be able to tell that apart from "written, and empty" without
//! learning the convention — an absent key is the one spelling that needs no
//! explanation. The website's `scripts/lib/meta.mjs` is the consumer this was
//! built against.
//!
//! Built and written as JSON rather than as text through the `serde_json`
//! this crate already carries for ADR 0085's document; the workspace manifest
//! owns why this crate and not another.

use std::process::ExitCode;

use nvs_stdlib::registry::{
    CLASSES, CoreClass, CoreConst, CoreEnum, CoreMethod, ENUMS, EnumDoc, ErrorDoc, MethodDoc,
    ParamDoc,
};
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
    json!({
        "classes": CLASSES.iter().map(class_json).collect::<Vec<_>>(),
        "enums": ENUMS.iter().map(enum_json).collect::<Vec<_>>(),
    })
}

/// One class: its name, its members — methods before instance members, each
/// roster in the spec's own order — and its constants, when it has any.
fn class_json(class: &CoreClass) -> Value {
    let members: Vec<Value> = class
        .methods
        .iter()
        .chain(class.instance)
        .map(member_json)
        .collect();
    let mut out = Map::new();
    out.insert("name".into(), Value::from(class.name));
    out.insert("members".into(), Value::Array(members));
    put_list(&mut out, "constants", class.constants, constant_json);
    Value::Object(out)
}

/// One member: its name, the `$name` each positional parameter is callable by
/// ([ADR 0063](../../../docs/adr/0063-core-api-conventions.md) R2), and its
/// `doc` only when the row carries one.
///
/// `names` is the row's own `CoreMethod::names`, so a member with no
/// positional parameter has no `names` key — and a trailing options bag has no
/// entry there either, its one name being
/// `nvs_stdlib::registry::OPTIONS_NAME` for every member that has one. It is
/// emitted for **every** row, documented or not, because it is signature and
/// not documentation: a consumer building a call needs it where a reference
/// card is optional.
fn member_json(member: &CoreMethod) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(member.name));
    put_list(&mut out, "names", member.names, |name| Value::from(*name));
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

/// One constant: its name, and its one-sentence card as `doc` only when
/// written — a constant's whole card is one string, so there is no object to
/// leave empty.
fn constant_json(constant: &CoreConst) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(constant.name));
    put_str(&mut out, "doc", constant.desc);
    Value::Object(out)
}

/// One enum: its name, and its `doc` only when the row carries one.
fn enum_json(declared: &CoreEnum) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(declared.name));
    if let Some(doc) = declared.doc {
        out.insert("doc".into(), enum_doc_json(doc));
    }
    Value::Object(out)
}

/// One enum's card, with every empty field left out.
fn enum_doc_json(doc: &EnumDoc) -> Value {
    let mut out = Map::new();
    put_str(&mut out, "short", doc.short);
    put_list(
        &mut out,
        "cases",
        doc.cases,
        |case| json!({ "name": case.name, "desc": case.desc }),
    );
    Value::Object(out)
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
    use nvs_stdlib::registry::{CaseDoc, ShapeKeyDoc};

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

    /// An enum's card with both fields written.
    const FULL_ENUM: EnumDoc = EnumDoc {
        short: "Chooses a thing.",
        cases: &[CaseDoc {
            name: "One",
            desc: "The first thing.",
        }],
    };

    /// An enum's card with nothing written yet.
    const EMPTY_ENUM: EnumDoc = EnumDoc {
        short: "",
        cases: &[],
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

    /// § 2's enum shape, key for key.
    #[test]
    fn a_full_enum_card_emits_every_field_of_the_contract() {
        assert_eq!(
            enum_doc_json(&FULL_ENUM),
            json!({
                "short": "Chooses a thing.",
                "cases": [ { "name": "One", "desc": "The first thing." } ]
            })
        );
    }

    /// The same omission rule, on an enum's card.
    #[test]
    fn an_empty_enum_card_emits_an_empty_object() {
        assert_eq!(enum_doc_json(&EMPTY_ENUM), json!({}));
    }

    /// A constant with nothing written is its name alone; one with a card
    /// carries it as `doc`.
    #[test]
    fn a_constant_carries_its_doc_only_when_written() {
        let unwritten = CoreConst {
            name: "X",
            ty: nvs_stdlib::registry::CoreTy::Int,
            value: nvs_stdlib::registry::Const::Int(1),
            desc: "",
        };
        assert_eq!(constant_json(&unwritten), json!({ "name": "X" }));
        let written = CoreConst {
            desc: "One.",
            ..unwritten
        };
        assert_eq!(
            constant_json(&written),
            json!({ "name": "X", "doc": "One." })
        );
    }

    /// Every class in the registry appears, and every one of its members —
    /// methods and then instance members, in the rosters' own order — and
    /// every one of its constants, in roster order, under a key that is absent
    /// for a class with none.
    #[test]
    fn the_document_lists_every_class_member_and_constant_in_registry_order() {
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

            let constants: Vec<&str> = emitted["constants"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|c| c["name"].as_str().expect("a name"))
                        .collect()
                })
                .unwrap_or_default();
            let expected: Vec<&str> = class.constants.iter().map(|c| c.name).collect();
            assert_eq!(constants, expected, "{}", class.name);
        }
    }

    /// Every enum in the registry appears, top-level, in the roster's order.
    #[test]
    fn the_document_lists_every_enum_in_roster_order() {
        let document = document();
        let names: Vec<&str> = document["enums"]
            .as_array()
            .expect("an array")
            .iter()
            .map(|e| e["name"].as_str().expect("a name"))
            .collect();
        let expected: Vec<&str> = ENUMS.iter().map(|e| e.name).collect();
        assert_eq!(names, expected);
    }
}
