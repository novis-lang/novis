//! `nvs build --openapi` — [ADR 0085](../../../docs/adr/0085-openapi-is-generated-from-the-route-table.md)'s
//! document, built from the finished route table.
//!
//! The whole of this module is a *rendering*. Every fact in the document was
//! decided by the front end and is read off a [`nvs_types::Route`] row: nothing
//! here parses, resolves or infers, which is what ADR 0085 means by a document
//! that cannot drift from the code. A row that is missing something the
//! document wants is a gap in what the table carries, and the fix goes in
//! `nvs_types::routes` rather than here.
//!
//! A [`Value`] rather than a `String`, and `serde_json` rather than a
//! hand-written writer, for the reason the workspace manifest already gives
//! that crate: JSON is an external specification, so it is a dependency. § 4's
//! `nvs api diff` compares two of these, and comparing them as documents rather
//! than as text is what keeps the diff from having a second opinion about JSON.
//!
//! ## Determinism (§ 3)
//!
//! Two builds of the same source produce byte-identical output, because § 4's
//! diff is worthless otherwise. The emitter has exactly two orderings and both
//! are [`BTreeMap`]s built here rather than left to the JSON map's own: paths
//! sorted by their written text, and the operations within a path sorted by
//! lowercased verb. Everything else is a fixed member list or the row order the
//! table was built in, which ADR 0061 § 3 already makes independent of
//! filesystem enumeration.
//!
//! ## What § 1's table does not supply yet
//!
//! Each of these is a row of § 1 whose source exists but does not reach this
//! module, and each is *absent* from the document rather than guessed at:
//!
//! 1. **Response schemas.** The handler's declared return type is not on the
//!    row, and rendering a class as a schema is [ADR 0071](../../../docs/adr/0071-derived-codecs.md)'s
//!    codec, which the emitter has no access to. An operation therefore carries
//!    no `responses` member — legal in 3.1, where it is optional.
//! 2. **Request body schemas**, for the same reason.
//! 3. **Summary and description.** Doc comments are parsed but not carried on
//!    the row.
//! 4. **Everything `#[Api]` supplies** (§ 2) — tags, errors, security, examples,
//!    deprecation — which is a compiler-recognized attribute that does not exist
//!    yet.
//! 5. **`info.version`.** The document has to carry one (3.1 requires it) and
//!    nothing in the program declares one, so it is a fixed `0.0.0` until
//!    `nvs.toml` grows the key M6's reader would own.
//! 6. **A type outside [`schema`]'s list** — an enum, a literal union,
//!    `Core\Uuid` — is emitted with the empty schema, which in JSON Schema means
//!    *any*. That is the honest rendering of "the compiler knows this type and
//!    the emitter has no mapping for it yet".

use std::collections::BTreeMap;

use nvs_types::{ParamIn, Route, RouteParam, RouteTable};
use serde_json::{Map, Value, json};

/// The OpenAPI version every document claims. The ADR's consequence list makes
/// this the only one: there is no 3.0 downgrade and no Swagger 2.
const VERSION: &str = "3.1.0";

/// The whole document for `routes`.
///
/// `title` is the entry point's own file stem — the one name the compiler has
/// for a program, since nothing in the language declares one.
pub(crate) fn document(routes: &RouteTable, title: &str) -> Value {
    let ids = operation_ids(routes);

    // Both orderings § 3 requires, in the type that holds them, before anything
    // is handed to the JSON map — so the sort is this module's own fact rather
    // than a property of whichever map `serde_json` was compiled with.
    //
    // A path and a verb identify at most one row of a program that compiles:
    // § 3's duplicate-route error is exactly that pair. `or_insert_with` is
    // written rather than asserted because this is a renderer and the
    // diagnostic is the front end's.
    let mut paths: BTreeMap<&str, BTreeMap<String, Value>> = BTreeMap::new();
    for (row, id) in routes.rows().iter().zip(&ids) {
        paths
            .entry(row.path.as_str())
            .or_default()
            .entry(row.verb.to_lowercase())
            .or_insert_with(|| operation(row, id));
    }

    let paths: Map<String, Value> = paths
        .into_iter()
        .map(|(path, operations)| {
            let object = operations.into_iter().collect::<Map<String, Value>>();
            ((*path).to_owned(), Value::Object(object))
        })
        .collect();

    json!({
        "openapi": VERSION,
        "info": {
            "title": title,
            "version": "0.0.0",
        },
        "paths": Value::Object(paths),
    })
}

/// Every row's `operationId`, by the row's position in the table.
///
/// § 1: the id is `#[Route]`'s `name`. A route that declared none falls back to
/// its handler label, which is unique by construction — a method declares one
/// route per verb, and `Class::method` names it — and a *shared* name takes
/// [ADR 0110](../../../docs/adr/0110-one-methods-repeated-routes-share-a-name-when-they-share-a-path.md)
/// § 3's lowercased-verb suffix, which is what keeps the id unique when one
/// method's repeated routes deliberately share one name.
///
/// By position rather than keyed by name because both the fallback and the
/// suffix have to be decided over the whole table before any row is rendered.
fn operation_ids(routes: &RouteTable) -> Vec<String> {
    let mut shared: BTreeMap<&str, usize> = BTreeMap::new();
    for row in routes.rows() {
        *shared.entry(base_id(row)).or_default() += 1;
    }
    routes
        .rows()
        .iter()
        .map(|row| {
            let base = base_id(row);
            if shared.get(base).copied().unwrap_or(0) > 1 {
                format!("{base}.{}", row.verb.to_lowercase())
            } else {
                base.to_owned()
            }
        })
        .collect()
}

/// The id a row would carry if no other row shared it.
fn base_id(row: &Route) -> &str {
    row.name
        .as_ref()
        .map_or(row.handler.as_str(), |(name, _)| name.as_str())
}

/// One operation object.
///
/// `parameters` is omitted rather than written empty where the route declares
/// none: an empty array and an absent member mean the same thing to a reader,
/// and the shorter one is what a hand-written document would carry.
fn operation(row: &Route, id: &str) -> Value {
    let mut operation = Map::new();
    operation.insert("operationId".to_owned(), json!(id));
    if !row.params.is_empty() {
        let params: Vec<Value> = row.params.iter().map(parameter).collect();
        operation.insert("parameters".to_owned(), Value::Array(params));
    }
    Value::Object(operation)
}

/// One parameter object, in declaration order within its route.
fn parameter(param: &RouteParam) -> Value {
    json!({
        "name": param.name,
        "in": match param.source {
            ParamIn::Path => "path",
            ParamIn::Query => "query",
        },
        "required": param.required,
        "schema": schema(param.ty.as_deref()),
    })
}

/// § 1's "by declared type": the JSON Schema one Novis type renders as.
///
/// The empty schema is *any*, and is what a type with no mapping gets. The list
/// is deliberately short — these are the types a path capture or a `#[Query]`
/// may be declared at
/// ([ADR 0102](../../../docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
/// §§ 3 and 5) that have an unambiguous JSON Schema, and nothing else is
/// guessed at. `tainted string` renders as `string` because the qualifier is a
/// fact about the compiler's tracking, not about the wire
/// ([ADR 0024](../../../docs/adr/0024-taint-tracking.md)).
fn schema(ty: Option<&str>) -> Value {
    match ty {
        Some("bool") => json!({"type": "boolean"}),
        Some("int") => json!({"type": "integer"}),
        // A `uint` is an integer with a floor, and saying so is the difference
        // between a generated client that rejects `-1` and one that does not.
        Some("uint") => json!({"type": "integer", "minimum": 0}),
        Some("float") => json!({"type": "number"}),
        // Not `number`: a decimal that survives a JSON round trip is a string,
        // which is `Core\Json`'s own reading of ADR 0054's type.
        Some("decimal") => json!({"type": "string", "format": "decimal"}),
        Some("string" | "tainted string") => json!({"type": "string"}),
        _ => json!({}),
    }
}
