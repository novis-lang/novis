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
//! A row with no documentation has **no `doc` key at all** — a shape the
//! registry's own `every_registry_row_carries_a_reference_card` no longer lets
//! a shipped row take, but that a consumer reading an older toolchain still
//! meets — and a documented row's `short`/`return` strings and
//! `params`/`shape`/`errors`/`cases` arrays appear **only when non-empty** — a
//! class with no constants has no `constants` key either, and a member that
//! throws nothing has no `errors`. That is § 3's field-wise precedence made
//! mechanical: the registry spells "not written yet" as the empty value, and a
//! consumer must be able to tell that apart from "written, and empty" without
//! learning the convention — an absent key is the one spelling that needs no
//! explanation. The website's `scripts/lib/meta.mjs` is the consumer this was
//! built against.
//!
//! ## The signature half, and the four tables beside the registry
//!
//! A row is **signature** as well as documentation, and `docs/novis.md` —
//! the one-file reference `tools/reference.py` generates — is built from this
//! command alone, so that a member's types never have to be scraped out of
//! the spec a second time. Every member therefore also carries `kind`
//! (`static` or `instance`), `signature` (the spec's own spelling,
//! `length(string $s): uint`), `params` with each positional parameter's
//! type, qualifier and default, `options` for a trailing bag, and `returns`;
//! a class carries `typeParams` and `constructor` when it has them, a
//! constant its `type` and `value`. Beside `classes` and `enums` sit the four
//! rosters the compiler declares outside the registry and a program can
//! reach: `exceptions` ([`nvs_hir::errors::TREE`]), `interfaces`
//! ([`nvs_hir::interfaces::RESERVED`]), `attributes`
//! ([`nvs_types::derive::ATTRIBUTES`]) and `directives`
//! ([`nvs_config::DIRECTIVES`]). Each is a table already, so this is one
//! `map` per roster and no second home for any of them.
//!
//! Built and written as JSON rather than as text through the `serde_json`
//! this crate already carries for ADR 0085's document; the workspace manifest
//! owns why this crate and not another.

use std::process::ExitCode;

use nvs_stdlib::registry::{
    CLASSES, Const, CoreClass, CoreConst, CoreEnum, CoreMethod, CoreOption, CoreTy, ENUMS, EnumDoc,
    ErrorDoc, MethodDoc, ParamDoc, Qual, class_type_params, constructor_of,
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
        "exceptions": nvs_hir::errors::TREE.iter().map(exception_json).collect::<Vec<_>>(),
        "interfaces": nvs_hir::interfaces::RESERVED.iter().map(interface_json).collect::<Vec<_>>(),
        "attributes": nvs_types::derive::ATTRIBUTES.iter().map(|name| Value::from(*name)).collect::<Vec<_>>(),
        "directives": nvs_config::DIRECTIVES.iter().map(directive_json).collect::<Vec<_>>(),
    })
}

/// One class: its name, its type parameters and constructor when it has
/// them, its members — methods before instance members, each roster in the
/// spec's own order — and its constants, when it has any.
fn class_json(class: &CoreClass) -> Value {
    let members: Vec<Value> = class
        .methods
        .iter()
        .map(|member| member_json(member, "static"))
        .chain(
            class
                .instance
                .iter()
                .map(|member| member_json(member, "instance")),
        )
        .collect();
    let mut out = Map::new();
    out.insert("name".into(), Value::from(class.name));
    if let Some(params) = class_type_params(class.name) {
        put_list(&mut out, "typeParams", params, |name| Value::from(*name));
    }
    if let Some(constructor) = constructor_of(class.name) {
        out.insert(
            "constructor".into(),
            member_json(constructor, "constructor"),
        );
    }
    out.insert("members".into(), Value::Array(members));
    put_list(&mut out, "constants", class.constants, constant_json);
    Value::Object(out)
}

/// One exception class of the compiler-declared tree: its name, its parent
/// when it has one, and the properties it declares of its own.
fn exception_json(row: &(&str, Option<&str>)) -> Value {
    let (name, parent) = *row;
    let own: &[&str] = nvs_hir::errors::OWN_PROPERTIES
        .iter()
        .find(|(owner, _)| *owner == name)
        .map_or(&[], |(_, properties)| *properties);
    let mut out = Map::new();
    out.insert("name".into(), Value::from(name));
    if let Some(parent) = parent {
        out.insert("parent".into(), Value::from(parent));
    }
    put_list(&mut out, "properties", own, |property| {
        Value::from(*property)
    });
    Value::Object(out)
}

/// One compiler-declared global interface: its name and its type parameters.
fn interface_json(row: &(&str, &[&str])) -> Value {
    let (name, params) = *row;
    let mut out = Map::new();
    out.insert("name".into(), Value::from(name));
    put_list(&mut out, "typeParams", params, |param| Value::from(*param));
    Value::Object(out)
}

/// One `nvs.toml` directive: its key, its class and when a change applies.
fn directive_json(directive: &nvs_config::Directive) -> Value {
    json!({
        "key": directive.key,
        "class": format!("{:?}", directive.class),
        "apply": format!("{:?}", directive.apply),
    })
}

/// One member: its name, its `kind`, the `$name` each positional parameter is
/// callable by ([ADR 0063](../../../docs/adr/0063-core-api-conventions.md)
/// R2), its signature half — `signature`, `params`, `options`, `returns` —
/// and its `doc` only when the row carries one.
///
/// `names` is the row's own `CoreMethod::names`, so a member with no
/// positional parameter has no `names` key — and a trailing options bag has no
/// entry there either, its one name being
/// `nvs_stdlib::registry::OPTIONS_NAME` for every member that has one. It is
/// emitted for **every** row, documented or not, because it is signature and
/// not documentation: a consumer building a call needs it where a reference
/// card is optional. The same holds of every field the module doc's
/// *signature half* names.
fn member_json(member: &CoreMethod, kind: &str) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(member.name));
    out.insert("kind".into(), Value::from(kind));
    put_list(&mut out, "names", member.names, |name| Value::from(*name));
    out.insert("signature".into(), Value::from(signature(member)));
    let params: Vec<Value> = positional_json(member);
    if !params.is_empty() {
        out.insert("params".into(), Value::Array(params));
    }
    if let Some(options) = member.options() {
        put_list(&mut out, "options", options, option_json);
    }
    out.insert("returns".into(), Value::from(ty_string(&member.return_ty)));
    if let Some(doc) = member.doc {
        out.insert("doc".into(), doc_json(doc));
    }
    Value::Object(out)
}

/// Every positional parameter of `member` — the row's `params` without a
/// trailing options bag — as `{name, type, qualifier?, default?, variadic?}`.
///
/// `defaults` aligns to the *end* of the positional list, exactly as a
/// user-declared method's trailing defaults do, so the first `n - d`
/// parameters are required. A variadic tail carries no default and says
/// `variadic` instead. A parameter whose type carries an ADR 0088 § 2
/// classification names it as `qualifier`; one that carries none — every
/// non-text type, and the unclassified `string`/`bytes` spellings, which
/// refuse a tainted argument — has no key.
fn positional_json(member: &CoreMethod) -> Vec<Value> {
    let positional = member.positional();
    let required = positional.len().saturating_sub(member.defaults.len());
    positional
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let mut out = Map::new();
            out.insert(
                "name".into(),
                Value::from(member.names.get(index).copied().unwrap_or("")),
            );
            match ty {
                CoreTy::Variadic(elem) => {
                    out.insert("type".into(), Value::from(ty_string(elem)));
                    out.insert("variadic".into(), Value::Bool(true));
                }
                _ => {
                    out.insert("type".into(), Value::from(ty_string(ty)));
                    if index >= required
                        && let Some(default) = member.defaults.get(index - required)
                    {
                        out.insert("default".into(), Value::from(const_string(default)));
                    }
                }
            }
            if let Some(qual) = ty.classification() {
                out.insert("qualifier".into(), Value::from(qual_string(qual)));
            }
            Value::Object(out)
        })
        .collect()
}

/// One option of a trailing bag: its name, type and the value an omitted one
/// takes.
fn option_json(option: &CoreOption) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(option.name));
    out.insert("type".into(), Value::from(ty_string(&option.ty)));
    out.insert("default".into(), Value::from(const_string(&option.default)));
    if let Some(qual) = option.ty.classification() {
        out.insert("qualifier".into(), Value::from(qual_string(qual)));
    }
    Value::Object(out)
}

/// The spec's own spelling of a row — `name(type $a, type $b = 1, {opt?: T}): ret`,
/// with a variadic tail as `type ...$rest` and a written type-argument list
/// as `name<T>(…)`. Never the class or the `$receiver->`: the consumer knows
/// which class it is reading and `kind` says the rest.
fn signature(member: &CoreMethod) -> String {
    let positional = member.positional();
    let required = positional.len().saturating_sub(member.defaults.len());
    let mut parts: Vec<String> = positional
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let name = member.names.get(index).copied().unwrap_or("");
            match ty {
                CoreTy::Variadic(elem) => format!("{} ...${name}", ty_string(elem)),
                _ if index >= required => match member.defaults.get(index - required) {
                    Some(default) => {
                        format!("{} ${name} = {}", ty_string(ty), const_string(default))
                    }
                    None => format!("{} ${name}", ty_string(ty)),
                },
                _ => format!("{} ${name}", ty_string(ty)),
            }
        })
        .collect();
    if let Some(options) = member.options() {
        parts.push(ty_string(&CoreTy::Options(options)));
    }
    let written = member.written();
    let type_args = if written.is_empty() {
        String::new()
    } else {
        format!("<{}>", written.join(", "))
    };
    format!(
        "{}{type_args}({}): {}",
        member.name,
        parts.join(", "),
        ty_string(&member.return_ty)
    )
}

/// A [`CoreTy`] in the spelling a program writes it — the spec's column, so
/// `Text(Sink)` is `string` and `Iterated(T)` is the three shapes `foreach`
/// accepts. The wildcard arm is the one `#[non_exhaustive]` requires, and it
/// is where a variant this crate has not learned to spell would show up.
fn ty_string(ty: &CoreTy) -> String {
    match ty {
        CoreTy::Bool => "bool".into(),
        CoreTy::Int => "int".into(),
        CoreTy::Uint => "uint".into(),
        CoreTy::Float => "float".into(),
        CoreTy::Decimal => "decimal".into(),
        CoreTy::Str | CoreTy::Text(_) => "string".into(),
        CoreTy::Bytes | CoreTy::Blob(_) => "bytes".into(),
        CoreTy::Void => "void".into(),
        CoreTy::Mixed => "mixed".into(),
        CoreTy::Array(elem) => format!("array<{}>", ty_string(elem)),
        CoreTy::Callable | CoreTy::CallableTo(_) => "callable".into(),
        CoreTy::CallableShapeTo(_) => "{name: callable, ...}".into(),
        CoreTy::Var(name) | CoreTy::Written(name) => (*name).into(),
        CoreTy::Union(members) => members.iter().map(ty_string).collect::<Vec<_>>().join("|"),
        CoreTy::IntLiteral(value) => value.to_string(),
        CoreTy::Nullable(inner) => format!("?{}", ty_string(inner)),
        CoreTy::Enum(name) | CoreTy::Instance(name) => (*name).into(),
        CoreTy::EnumCase(owner, case) => format!("{owner}::{case}"),
        CoreTy::Iterated(elem) => {
            let elem = ty_string(elem);
            format!("array<{elem}>|Iterable<{elem}>|Iterator<{elem}>")
        }
        CoreTy::Variadic(elem) => format!("{} ...", ty_string(elem)),
        CoreTy::Options(options) => {
            let fields: Vec<String> = options
                .iter()
                .map(|option| format!("{}?: {}", option.name, ty_string(&option.ty)))
                .collect();
            format!("{{{}}}", fields.join(", "))
        }
        _ => "?".into(),
    }
}

/// A [`Const`] as a program would write it: a scalar as its literal, a
/// string JSON-quoted, an enum case by its qualified spelling, and a built
/// instance as the call that builds it.
fn const_string(value: &Const) -> String {
    match value {
        Const::Null => "null".into(),
        Const::Bool(b) => b.to_string(),
        Const::Int(i) => i.to_string(),
        Const::Uint(u) => u.to_string(),
        Const::Float(f) => format!("{f:?}"),
        Const::Str(s) => serde_json::to_string(s).unwrap_or_default(),
        Const::Bytes(bytes) => format!(
            "bytes({})",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ),
        Const::EmptyArray => "[]".into(),
        Const::EnumCase(owner, case) => format!("{owner}::{case}"),
        Const::Built { symbol, args } => {
            // The call that builds it, spelled as a program would write it —
            // the symbol names a member of some registered class, so the
            // class and member are looked up rather than the helper's name
            // printed.
            let call = CLASSES
                .iter()
                .find_map(|class| {
                    class
                        .members()
                        .find(|member| member.symbol == *symbol)
                        .map(|member| format!("{}::{}", class.name, member.name))
                })
                .unwrap_or_else(|| (*symbol).to_owned());
            format!(
                "{call}({})",
                args.iter().map(const_string).collect::<Vec<_>>().join(", ")
            )
        }
        _ => "?".into(),
    }
}

/// A [`Qual`] as the spec's *Qualifier* column spells it.
fn qual_string(qual: Qual) -> &'static str {
    match qual {
        Qual::Contagious => "contagious",
        Qual::Sink => "sink",
        Qual::Neutral => "neutral",
        Qual::Launder => "launder",
        Qual::Reveal => "reveal",
    }
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

/// One constant: its name, its type and value, and its one-sentence card as
/// `doc` only when written — a constant's whole card is one string, so there
/// is no object to leave empty.
fn constant_json(constant: &CoreConst) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), Value::from(constant.name));
    out.insert("type".into(), Value::from(ty_string(&constant.ty)));
    out.insert("value".into(), Value::from(const_string(&constant.value)));
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
        assert_eq!(
            constant_json(&unwritten),
            json!({ "name": "X", "type": "int", "value": "1" })
        );
        let written = CoreConst {
            desc: "One.",
            ..unwritten
        };
        assert_eq!(
            constant_json(&written),
            json!({ "name": "X", "type": "int", "value": "1", "doc": "One." })
        );
    }

    /// The signature half spells a row the way the spec's column does:
    /// defaults aligned to the end, a bag last, a written type argument on
    /// the name — read off three rows the registry ships.
    #[test]
    fn a_signature_is_the_specs_own_spelling() {
        let find = |class: &str, member: &str| -> &'static CoreMethod {
            nvs_stdlib::registry::class(class)
                .expect("a registered class")
                .members()
                .find(|row| row.name == member)
                .expect("a registered member")
        };
        assert_eq!(
            signature(find(r"Core\Str", "length")),
            "length(string $s): uint"
        );
        assert_eq!(
            signature(find(r"Core\Str", "format")),
            "format(string $template, mixed ...$arguments): string"
        );
        assert!(signature(find(r"Core\Arr", "sort")).starts_with("sort(array<T> $a, {"));
        assert!(signature(find(r"Core\Json", "decodeAs")).starts_with("decodeAs<T>("));
    }

    /// Every member of every class carries the four signature keys, and the
    /// four rosters beside the registry are non-empty tables.
    #[test]
    fn every_member_carries_its_signature_half_and_every_roster_is_present() {
        let document = document();
        for class in document["classes"].as_array().expect("an array") {
            for member in class["members"].as_array().expect("an array") {
                assert!(member["kind"].is_string(), "{member}");
                assert!(member["signature"].is_string(), "{member}");
                assert!(member["returns"].is_string(), "{member}");
            }
        }
        for roster in ["exceptions", "interfaces", "attributes", "directives"] {
            assert!(
                !document[roster].as_array().expect("an array").is_empty(),
                "{roster} is empty"
            );
        }
        assert_eq!(document["exceptions"][0]["name"], "Throwable");
        assert!(document["exceptions"][0]["properties"].is_array());
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
