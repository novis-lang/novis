//! `nvs build --openapi` — `rule:routing/api-document-is-generated-from-the-route-table`'s
//! document, built from the finished route table.
//!
//! The whole of this module is a *rendering*. Every fact in the document was
//! decided by the front end and is read off a [`nvs_types::Route`] row: nothing
//! here parses, resolves or infers, which is what `rule:routing/api-document-is-generated-from-the-route-table` means by a document
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
//! diff is worthless otherwise. Every ordering the emitter has is a
//! [`BTreeMap`] built here rather than left to the JSON map's own: paths
//! sorted by their written text, and the operations within a path sorted by
//! lowercased verb. Everything else is a fixed member list or the row order the
//! table was built in, which `rule:programs/implementing` already makes independent of
//! filesystem enumeration.
//!
//! ## Known gaps
//!
//! Each of these is a row of § 1 whose source exists but does not reach this
//! module, and each is *absent* from the document rather than guessed at:
//!
//! 1. **A response body that is an object.** The declared return type is on the
//!    row and [`responses`] renders it, but only through [`schema`] — so a
//!    handler answering with a *class* gets the empty schema, because the fields
//!    of one are `rule:core-classes/derive-attribute`'s codec
//!    and the codec is not on the row. It is that roster and not the class's
//!    declared properties: a property map holds the private ones too, and a
//!    document that published those would be leaking exactly what
//!    `#[Json\Derive]` exists to decide.
//!    — owner: unowned
//! 2. **Request body schemas**, for the same reason and one more: which
//!    parameter *is* the body is a question the row does not answer either.
//!    — owner: unowned
//! 3. **`components.securitySchemes`.** § 2's `security` names reach the
//!    operation ([`operation`] writes them), but *what* a named scheme is —
//!    bearer, an API key, OAuth2 and its flows — is nowhere in the tree:
//!    nothing in this compiler or in `nvs.toml` declares one, which is the same
//!    absence `nvs_types::routes`' `check_api` records for the half of § 2's
//!    scheme rule it cannot ask. So the document names schemes it does not
//!    define, and a strict validator says so. Writing a guessed definition
//!    would be the emitter stating a fact about deployment that no one wrote,
//!    which is the one thing `rule:routing/api-document-is-generated-from-the-route-table` is against; the component object lands
//!    here, with no change to [`operation`], on the day a scheme has a home.
//!    — owner: unowned
//! 4. **`info.version`.** The document has to carry one (3.1 requires it) and
//!    nothing in the program declares one, so it is a fixed `0.0.0` until
//!    `nvs.toml` grows the key M6's reader would own.
//!    — owner: unowned
//! 5. **An enum-case subset**, which is the half of § 1's *Enumerations* row a
//!    literal union does not cover. A capture declared at one gets the empty
//!    schema — *any* — because `nvs_types::routes`' `closed_set` returns no set
//!    for it on purpose: a case's segment spelling is `Core\Router::match`'s to
//!    decide, and that member is out of scope. When the row carries the set,
//!    [`schema`] emits it with no further work, exactly as a literal union's
//!    already is. Anything else outside [`schema`]'s list is the same honest
//!    rendering of "the compiler knows this type and the emitter has no mapping
//!    for it yet".
//!    — owner: unowned

use std::collections::BTreeMap;

use nvs_types::{ConstArg, ParamIn, Route, RouteParam, RouteTable};
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

    // The orderings § 3 requires, in the type that holds them, before anything
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
/// `rule:routing/a-shared-name-is-one-endpoint-everywhere`
/// 's lowercased-verb suffix, which is what keeps the id unique when one
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
    // § 2's `tags`, in the order they were written: the row carries them
    // already grouped, and an empty list is written as no member for
    // `parameters`' reason above.
    if !row.tags.is_empty() {
        operation.insert("tags".to_owned(), json!(row.tags));
    }
    // § 1's last row, already split into its two halves by
    // `nvs_types::routes`'s `doc_comment`: a method with no doc comment carries
    // neither member, for `parameters`' reason above.
    if let Some(summary) = &row.summary {
        operation.insert("summary".to_owned(), json!(summary));
    }
    if let Some(description) = &row.description {
        operation.insert("description".to_owned(), json!(description));
    }
    if !row.params.is_empty() {
        let params: Vec<Value> = row.params.iter().map(parameter).collect();
        operation.insert("parameters".to_owned(), Value::Array(params));
    }
    // § 2's `security`, as 3.1's Security Requirement Object: one object per
    // scheme name, each holding the empty scope list. The scopes are empty
    // because a scheme name is all the row carries and all the compiler ever
    // saw — `nvs_types::routes`' `check_api` owns why a name is uninterpreted —
    // and an OAuth2 scope list is a fact about a configured scheme, which is
    // the same thing that is missing there.
    if !row.security.is_empty() {
        let schemes: Vec<Value> = row
            .security
            .iter()
            .map(|name| {
                let mut requirement = Map::new();
                requirement.insert(name.clone(), Value::Array(Vec::new()));
                Value::Object(requirement)
            })
            .collect();
        operation.insert("security".to_owned(), Value::Array(schemes));
    }
    operation.insert("responses".to_owned(), responses(row));
    Value::Object(operation)
}

/// § 1's response body: the handler's declared return type, plus § 2's
/// `errors` — the responses the declared type cannot state.
///
/// **`200` is § 1's and outranks an `errors` entry that names it.** § 2 may add
/// and may not contradict, so where both speak for one status the declared
/// return type is the one the code stated; the entry is dropped rather than
/// reported, because this module is a renderer and every diagnostic `rule:routing/api-document-is-generated-from-the-route-table`
/// has is the front end's.
///
/// An error response carries its class as the `description` and no `content`.
/// The class is what the row knows about that response, and a *schema* for one
/// is gap 1 above — a class's fields are `rule:core-classes/derive-attribute`'s codec, which does not reach
/// this module for an error type any more than it does for a success type.
///
/// `description` is required of every response object in 3.1, and `success` is
/// what a generated one can honestly say: the *prose* about an operation is its
/// doc comment, which the operation already carries, and repeating it here would
/// put the same sentence in a document twice.
///
/// A `void` handler answers with no body, so the response carries no `content`
/// rather than an empty schema — the two mean different things to a generated
/// client. Everything else is JSON, which is § 1's "through the same codec":
/// the codec a return type reaches this document through is
/// `rule:core-classes/derive-attribute`'s, and that codec is
/// JSON.
fn responses(row: &Route) -> Value {
    let returns = row.returns.as_deref();
    let mut success = Map::new();
    success.insert("description".to_owned(), json!("success"));
    if !matches!(returns, None | Some("void")) {
        let mut media = Map::new();
        // `false`: a return type is a body the codec encodes, not a segment a
        // `parse` is given, so a class here is the class-body gap this module
        // records and never a string.
        media.insert("schema".to_owned(), schema(returns, None, false));
        // § 2's `example` sits beside the schema it is an example of, which is
        // 3.1's own place for one. A handler answering with nothing has no
        // media type to hang it on, so an example written there reaches no
        // document — as it should: `nvs_types::routes`' `check_api_example`
        // holds an example to the return type's fields, and `void` has none.
        if let Some(example) = row.example.as_ref().and_then(example_value) {
            media.insert("example".to_owned(), example);
        }
        success.insert(
            "content".to_owned(),
            json!({"application/json": Value::Object(media)}),
        );
    }
    let mut responses = Map::new();
    responses.insert("200".to_owned(), Value::Object(success));
    for error in &row.errors {
        responses
            .entry(error.status.to_string())
            .or_insert_with(|| json!({"description": error.class}));
    }
    Value::Object(responses)
}

/// § 2's `example` as JSON, or `None` where some part of it has no JSON form at
/// all.
///
/// **All of it or none of it.** A folded value with no JSON form is a `Core`
/// instance or a `bytes` constant — [`nvs_types::ConstArg`] names which
/// variants those are — and rendering one as `null` would be the document
/// stating a value the code did not; dropping the field alone would be an
/// example that is missing a member the class declares, which is a worse lie
/// than no example. So the whole member is absent, which is this module's
/// answer everywhere else it has less than the document wants.
///
/// An array renders as a JSON list when its keys are `0, 1, …, n-1` and as an
/// object otherwise. That is the test `nvs_stdlib::json`'s encoder applies to a
/// runtime array — its own docs own the rule — asked here of a folded one,
/// because an example is decoded by the codec that would apply that test and
/// the two have to agree about what the author wrote.
fn example_value(value: &ConstArg) -> Option<Value> {
    Some(match value {
        ConstArg::Null => Value::Null,
        ConstArg::Bool(value) => json!(value),
        ConstArg::Int(value) => json!(value),
        ConstArg::Uint(value) => json!(value),
        ConstArg::Float(value) => json!(value),
        ConstArg::Str(value) => json!(value),
        ConstArg::EmptyArray => Value::Array(Vec::new()),
        ConstArg::Shape(fields) => Value::Object(members(fields)?),
        ConstArg::Array(entries) => {
            if entries
                .iter()
                .enumerate()
                .all(|(index, (key, _))| *key == index.to_string())
            {
                let mut list = Vec::with_capacity(entries.len());
                for (_, value) in entries {
                    list.push(example_value(value)?);
                }
                Value::Array(list)
            } else {
                Value::Object(members(entries)?)
            }
        }
        ConstArg::Bytes(_)
        | ConstArg::Options(_)
        | ConstArg::RequiredShape(_)
        // Not a value, so there is no JSON for it: a schema says a key may be
        // absent by leaving it out of `required`, which is what an omitted
        // nullable option already is.
        | ConstArg::NeverWritten
        | ConstArg::Built { .. } => return None,
    })
}

/// The keyed half of [`example_value`], which owns why a member with no JSON
/// form takes the whole example with it.
fn members(fields: &[(String, ConstArg)]) -> Option<Map<String, Value>> {
    let mut object = Map::new();
    for (name, value) in fields {
        object.insert(name.clone(), example_value(value)?);
    }
    Some(object)
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
        "schema": schema(param.ty.as_deref(), param.admits().as_deref(), param.parses),
    })
}

/// § 1's "by declared type": the JSON Schema one Novis type renders as, and its
/// *Enumerations* row where the declared type is a closed set.
///
/// The empty schema is *any*, and is what a type with no mapping gets. The list
/// is deliberately short — these are the types a path capture or a `#[Query]`
/// may be declared at
/// (`rule:routing/a-query-parameter-is-declared-like-a-capture` and `rule:routing/a-capture-narrows-to-a-closed-set`
/// ) that have an unambiguous JSON Schema, and nothing else is
/// guessed at. `tainted string` renders as `string` because the qualifier is a
/// fact about the compiler's tracking, not about the wire
/// (`rule:security/tainted-qualifier`).
///
/// `allowed` is [`RouteParam::allowed`], and it *joins* the type mapping rather
/// than replacing it: `enum` constrains a value, it does not describe one. A
/// union of literal types has no entry in the list above and never will — its
/// rendering is `"en"|"de"|"fr"`, which names no JSON Schema type — so the set
/// joins the empty schema and is the whole of what the parameter says,
/// which is exactly
/// `rule:routing/a-capture-narrows-to-a-closed-set`
/// 's `enum: [en, de, fr]`.
///
/// **The members are emitted as JSON strings, including an `int` literal's.**
/// The row carries the set as the segment text each value is written with — its
/// own doc comment owns why — so `1|2|3` emits `["1", "2", "3"]` and a generated
/// client puts `1` in the URL either way. Emitting them as numbers would mean
/// deciding a *JSON* type the row does not carry, in the module whose whole
/// premise is that it decides nothing; and no `"type"` is emitted beside the set
/// for the same reason, JSON Schema taking the type from the members.
///
/// `parses` is [`RouteParam::parses`], and it is what a class-typed capture is
/// read from rather than its name: `describe` renders a class and an enum the
/// same way, as the qualified name, so the row has to say which. Every
/// implementor answers the bare `{"type": "string"}` its `parse` is given, and
/// the one arm above that says more is a format the *engine* registers for a
/// type it ships — a class declaring its own is a feature this module does not
/// have, and guessing one from a name would be the contradiction
/// `rule:attributes/api-adds-and-cannot-contradict` forbids. An enum names no
/// JSON Schema type either, and its set is the whole of what it says: the case
/// spellings
/// `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`
/// decided while compiling, which [`RouteParam::admits`] hands over beside a
/// literal union's members — one list, because a document listing anything else
/// is a generated client building links the router answers `404` to.
fn schema(ty: Option<&str>, allowed: Option<&[&str]>, parses: bool) -> Value {
    let mut rendered = match ty {
        Some("bool") => json!({"type": "boolean"}),
        Some("int") => json!({"type": "integer"}),
        // A `uint` is an integer with a floor, and saying so is the difference
        // between a generated client that rejects `-1` and one that does not.
        Some("uint") => json!({"type": "integer", "minimum": 0}),
        Some("float") => json!({"type": "number"}),
        // Not `number`: a decimal that survives a JSON round trip is a string,
        // which is `Core\Json`'s own reading of `rule:types/decimal`'s type.
        Some("decimal") => json!({"type": "string", "format": "decimal"}),
        Some("string" | "tainted string") => json!({"type": "string"}),
        // The one class a capture may be declared at (§ 5), and the format
        // registered for it in the JSON Schema dialect 3.1 uses. The spelling is
        // `nvs_stdlib::uuid::NAME` as `describe` renders it — matched as text
        // like every arm above, because this crate depends on `nvs-types` alone.
        Some(r"Core\Uuid") => json!({"type": "string", "format": "uuid"}),
        // Every other class a capture may stand at: the text its `parse` is
        // given, and nothing narrower, because the contract says the text
        // either parses or does not and never which texts do.
        _ if parses => json!({"type": "string"}),
        _ => json!({}),
    };
    if let (Some(values), Some(object)) = (allowed, rendered.as_object_mut()) {
        object.insert(
            "enum".to_owned(),
            Value::Array(values.iter().map(|value| json!(value)).collect()),
        );
    }
    rendered
}
