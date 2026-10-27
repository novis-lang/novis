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
//! that crate: JSON is an external specification, so it is a dependency.
//! `nvs api diff` (`rule:routing/api-diff-fails-a-breaking-change`) compares two of
//! these, and comparing them as documents rather
//! than as text is what keeps the diff from having a second opinion about JSON.
//!
//! ## Determinism
//!
//! Two builds of the same source produce byte-identical output
//! (`rule:routing/api-document-is-a-deterministic-build-artifact`), because
//! `nvs api diff` is worthless otherwise. Every ordering the emitter has is a
//! [`BTreeMap`] built here rather than left to the JSON map's own: paths
//! sorted by their written text, and the operations within a path sorted by
//! lowercased verb. Everything else is a fixed member list or the row order the
//! table was built in, which `rule:programs/implementing` already makes independent of
//! filesystem enumeration.
//!
//! ## Known gaps
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-cli/src/openapi.rs` lists them.

use std::collections::BTreeMap;

use nvs_types::{
    ApiScheme, BodyKind, BodyTy, CodecElement, CodecTy, ConstArg, DerivedCodec, DerivedField,
    EnumValue, ExprTypeTable, OAuthFlow, ParamIn, Route, RouteParam, RouteTable, SchemeKind,
};
use serde_json::{Map, Value, json};

/// The OpenAPI version every document claims, and the only one:
/// `rule:routing/api-document-is-a-deterministic-build-artifact` has no 3.0
/// downgrade and no Swagger 2.
const VERSION: &str = "3.1.0";

/// The whole document for the checked program whose tables are `exprs`: its
/// route table, and the JSON codec of each class a response names.
///
/// `title` is the entry point's own file stem — the one name the compiler has
/// for a program, since nothing in the language declares one.
pub(crate) fn document(exprs: &ExprTypeTable, title: &str) -> Value {
    let routes = exprs.routes();
    let ids = operation_ids(routes);
    let mut schemas = Schemas {
        codecs: exprs,
        entries: BTreeMap::new(),
    };

    // The orderings determinism requires, in the type that holds them, before anything
    // is handed to the JSON map — so the sort is this module's own fact rather
    // than a property of whichever map `serde_json` was compiled with.
    //
    // A path and a verb identify at most one row of a program that compiles:
    // the duplicate-route error is exactly that pair. `or_insert_with` is
    // written rather than asserted because this is a renderer and the
    // diagnostic is the front end's.
    let mut paths: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    for (row, id) in routes.rows().iter().zip(&ids) {
        for (path, left_out) in templates(&row.path) {
            let id = match left_out {
                Some(name) => format!("{id}.without-{name}"),
                None => id.clone(),
            };
            paths
                .entry(path)
                .or_default()
                .entry(row.verb.to_lowercase())
                .or_insert_with(|| operation(row, &id, left_out, &mut schemas));
        }
    }

    let paths: Map<String, Value> = paths
        .into_iter()
        .map(|(path, operations)| {
            let object = operations.into_iter().collect::<Map<String, Value>>();
            (path, Value::Object(object))
        })
        .collect();

    let mut document = json!({
        "openapi": VERSION,
        "info": {
            "title": title,
            "version": "0.0.0",
        },
        "paths": Value::Object(paths),
    });
    let mut components = Map::new();
    if !schemas.entries.is_empty() {
        let entries: Map<String, Value> = schemas.entries.into_iter().collect();
        components.insert("schemas".to_owned(), Value::Object(entries));
    }
    if !routes.schemes().is_empty() {
        components.insert("securitySchemes".to_owned(), security_schemes(routes));
    }
    if !components.is_empty() {
        document["components"] = Value::Object(components);
    }
    document
}

/// `components.schemas`, collected while the operations are rendered: one
/// entry per class a response or a request body reaches, keyed by
/// [`component_name`].
///
/// A class's schema is its `#[Json\Derive]` codec's field list, read from the
/// checked program's [`ExprTypeTable::codec`]. That list is what the codec
/// writes, so a skipped field and a private property the codec does not name
/// never reach the document. A class with no codec has no JSON form to
/// describe and gets the empty schema.
struct Schemas<'a> {
    codecs: &'a ExprTypeTable,
    entries: BTreeMap<String, Value>,
}

impl Schemas<'_> {
    /// The schema for a declared body type.
    fn body(&mut self, ty: &BodyTy) -> Value {
        match ty {
            // A `void` response has no media type, so `responses` never asks.
            BodyTy::Void => json!({}),
            BodyTy::Nullable(inner) => nullable(self.body(inner)),
            BodyTy::List(inner) => json!({"type": "array", "items": self.body(inner)}),
            BodyTy::Class(label) => self.class(label),
            BodyTy::Enum(values) => enum_schema(values.iter().map(|value| match value {
                EnumValue::Int(number) => json!(number),
                EnumValue::Uint(number) => json!(number),
            })),
            BodyTy::Named(name) => schema(Some(name), None, false),
        }
    }

    /// Whether a form read into `ty` throws when a field is missing: `ty` is a
    /// class whose codec has a required field.
    fn has_required_field(&self, ty: &BodyTy) -> bool {
        let BodyTy::Class(label) = ty else {
            return false;
        };
        self.codecs
            .codec(label)
            .is_some_and(|codec| codec.fields.iter().any(|field| field.required))
    }

    /// A `$ref` to the class's entry, which is written the first time a class
    /// is reached. The entry is reserved before its fields are read, so a
    /// class with a field of its own type ends at the `$ref`.
    fn class(&mut self, label: &str) -> Value {
        // `Core\Uuid` is a value with a registered format, and no codec.
        if label == r"Core\Uuid" {
            return schema(Some(label), None, false);
        }
        let codecs = self.codecs;
        let Some(codec) = codecs.codec(label) else {
            return json!({});
        };
        let name = component_name(label);
        if !self.entries.contains_key(&name) {
            self.entries.insert(name.clone(), Value::Null);
            let object = self.object(codec);
            self.entries.insert(name.clone(), object);
        }
        json!({"$ref": format!("#/components/schemas/{name}")})
    }

    /// An object schema over a codec's fields, with `required` in declaration
    /// order. A field is in `required` when a document must carry its key,
    /// which is independent of whether its value may be `null`.
    fn object(&mut self, codec: &DerivedCodec) -> Value {
        let mut properties = Map::new();
        let mut required = Vec::new();
        for field in &codec.fields {
            let schema = self.wire(field.ty, field.element.as_ref(), field);
            let schema = if field.nullable {
                nullable(schema)
            } else {
                schema
            };
            properties.insert(field.key.clone(), schema);
            if field.required {
                required.push(json!(field.key));
            }
        }
        let mut object = Map::new();
        object.insert("type".to_owned(), json!("object"));
        object.insert("properties".to_owned(), Value::Object(properties));
        if !required.is_empty() {
            object.insert("required".to_owned(), Value::Array(required));
        }
        Value::Object(object)
    }

    /// One codec wire type. `field` carries what the wire type leaves out: a
    /// nested class's label, an enum's values and a shape's own fields, each
    /// for the terminal element where `ty` is a list.
    fn wire(&mut self, ty: CodecTy, element: Option<&CodecElement>, field: &DerivedField) -> Value {
        match ty {
            CodecTy::Bool => schema(Some("bool"), None, false),
            CodecTy::Int => schema(Some("int"), None, false),
            CodecTy::Uint => schema(Some("uint"), None, false),
            CodecTy::Float => schema(Some("float"), None, false),
            CodecTy::Decimal => schema(Some("decimal"), None, false),
            CodecTy::Str => schema(Some("string"), None, false),
            CodecTy::Class => field
                .class
                .as_deref()
                .map_or_else(|| json!({}), |label| self.class(label)),
            CodecTy::List => {
                let items = element.map_or_else(
                    || json!({}),
                    |element| self.wire(element.ty, element.element.as_deref(), field),
                );
                json!({"type": "array", "items": items})
            }
            CodecTy::Enum => field.cases.as_ref().map_or_else(
                || json!({}),
                |cases| {
                    enum_schema(cases.values.iter().map(|value| {
                        if cases.unsigned {
                            u64::try_from(*value).map_or(Value::Null, |number| json!(number))
                        } else {
                            i64::try_from(*value).map_or(Value::Null, |number| json!(number))
                        }
                    }))
                },
            ),
            CodecTy::Shape => field
                .shape
                .as_deref()
                .map_or_else(|| json!({}), |shape| self.object(shape)),
            // `Core\Json` writes none of these with a fixed JSON type, so the
            // document says nothing it cannot back.
            CodecTy::Bytes | CodecTy::Instant | CodecTy::Mixed | CodecTy::Opaque => json!({}),
        }
    }
}

/// A class label as a component name. 3.1 allows letters, digits, `.`, `-`
/// and `_` there, so each namespace separator becomes a `.`.
fn component_name(label: &str) -> String {
    label.replace('\\', ".")
}

/// An enum's backing values as an integer schema: `Core\Json` writes a case as
/// its backing value.
fn enum_schema(values: impl Iterator<Item = Value>) -> Value {
    json!({"type": "integer", "enum": values.collect::<Vec<_>>()})
}

/// `schema` with `null` allowed as well.
///
/// A schema with one `type` takes `null` into that list, and into its `enum`
/// where it has one. The empty schema already allows `null`. Anything else,
/// a `$ref` among them, is wrapped in an `anyOf` with `{"type": "null"}`.
fn nullable(schema: Value) -> Value {
    let Value::Object(mut object) = schema else {
        return schema;
    };
    if object.is_empty() {
        return Value::Object(object);
    }
    if let Some(Value::String(single)) = object.get("type") {
        let types = json!([single, "null"]);
        object.insert("type".to_owned(), types);
        if let Some(Value::Array(values)) = object.get_mut("enum") {
            values.push(Value::Null);
        }
        return Value::Object(object);
    }
    json!({"anyOf": [Value::Object(object), {"type": "null"}]})
}

/// `components.securitySchemes`: every declared scheme under its name, sorted
/// by name, as `rule:routing/a-security-scheme-is-declared-in-code` requires.
/// Every spelling is already the specification's on the row.
fn security_schemes(routes: &RouteTable) -> Value {
    let sorted: BTreeMap<&str, &ApiScheme> = routes
        .schemes()
        .iter()
        .map(|scheme| (scheme.name.as_str(), scheme))
        .collect();
    let schemes: Map<String, Value> = sorted
        .into_iter()
        .map(|(name, scheme)| (name.to_owned(), security_scheme(scheme)))
        .collect();
    Value::Object(schemes)
}

/// One Security Scheme Object.
fn security_scheme(scheme: &ApiScheme) -> Value {
    let mut object = Map::new();
    object.insert("type".to_owned(), json!(scheme.kind.type_name()));
    if let Some(description) = &scheme.description {
        object.insert("description".to_owned(), json!(description));
    }
    match &scheme.kind {
        SchemeKind::ApiKey {
            location,
            parameter,
        } => {
            object.insert("in".to_owned(), json!(location));
            object.insert("name".to_owned(), json!(parameter));
        }
        SchemeKind::Http { scheme, format } => {
            object.insert("scheme".to_owned(), json!(scheme));
            if let Some(format) = format {
                object.insert("bearerFormat".to_owned(), json!(format));
            }
        }
        SchemeKind::MutualTls => {}
        SchemeKind::OAuth2 { flows } => {
            let flows: Map<String, Value> = flows
                .iter()
                .map(|flow| (flow.kind.to_owned(), oauth_flow(flow)))
                .collect();
            object.insert("flows".to_owned(), Value::Object(flows));
        }
        SchemeKind::OpenIdConnect { url } => {
            object.insert("openIdConnectUrl".to_owned(), json!(url));
        }
    }
    Value::Object(object)
}

/// One OAuth Flow Object.
fn oauth_flow(flow: &OAuthFlow) -> Value {
    let mut object = Map::new();
    for (key, url) in [
        ("authorizationUrl", &flow.authorization_url),
        ("tokenUrl", &flow.token_url),
        ("refreshUrl", &flow.refresh_url),
    ] {
        if let Some(url) = url {
            object.insert(key.to_owned(), json!(url));
        }
    }
    let scopes: Map<String, Value> = flow
        .scopes
        .iter()
        .map(|(name, description)| (name.clone(), json!(description)))
        .collect();
    object.insert("scopes".to_owned(), Value::Object(scopes));
    Value::Object(object)
}

/// Every row's `operationId`, by the row's position in the table.
///
/// The id is the route's method, `Class.method`
/// ([`Route::operation_base`]), or the
/// `operationId` its `#[Api]` writes in its place. A method that serves
/// several verbs has one row per verb, so a *shared* id takes
/// `rule:routing/a-method-is-one-endpoint-everywhere`
/// 's lowercased-verb suffix, which is what keeps the id unique.
///
/// By position rather than keyed by method because the suffix has to be
/// decided over the whole table before any row is rendered.
fn operation_ids(routes: &RouteTable) -> Vec<String> {
    let bases: Vec<String> = routes.rows().iter().map(Route::operation_base).collect();
    let mut shared: BTreeMap<&str, usize> = BTreeMap::new();
    for base in &bases {
        *shared.entry(base.as_str()).or_default() += 1;
    }
    routes
        .rows()
        .iter()
        .zip(&bases)
        .map(|(row, base)| {
            if shared.get(base.as_str()).copied().unwrap_or(0) > 1 {
                format!("{base}.{}", row.verb.to_lowercase())
            } else {
                base.to_owned()
            }
        })
        .collect()
}

/// The OpenAPI path templates one route is written under, each with the name
/// of the capture it leaves out.
///
/// 3.1's template is `{name}` alone, and a path parameter is always required.
/// So a `{name...}` is written `{name}`, and a route with a `{name?}` is two
/// paths: one with `{name}`, and one that ends before that segment and has no
/// such parameter. The grammar writes a `{name?}` last and at most once, so
/// there are never more than two.
fn templates(path: &str) -> Vec<(String, Option<&str>)> {
    let mut full = String::with_capacity(path.len());
    let mut optional = None;
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|at| open + at) else {
            break;
        };
        let inner = &rest[open + 1..close];
        let name = if let Some(name) = inner.strip_suffix('?') {
            optional = Some((full.len() + open, name));
            name
        } else {
            inner.strip_suffix("...").unwrap_or(inner)
        };
        full.push_str(&rest[..open]);
        full.push('{');
        full.push_str(name);
        full.push('}');
        rest = &rest[close + 1..];
    }
    full.push_str(rest);
    let Some((at, name)) = optional else {
        return vec![(full, None)];
    };
    let bare = full[..at].trim_end_matches('/');
    let bare = if bare.is_empty() { "/" } else { bare };
    vec![(bare.to_owned(), Some(name)), (full, None)]
}

/// One operation object.
///
/// `parameters` is omitted rather than written empty where the route declares
/// none: an empty array and an absent member mean the same thing to a reader,
/// and the shorter one is what a hand-written document would carry.
///
/// `left_out` is the `{name?}` capture this path does not have, which
/// [`templates`] owns.
fn operation(row: &Route, id: &str, left_out: Option<&str>, schemas: &mut Schemas<'_>) -> Value {
    let mut operation = Map::new();
    operation.insert("operationId".to_owned(), json!(id));
    // `#[Api]`'s `tags`, in the order they were written: the row carries them
    // already grouped, and an empty list is written as no member for
    // `parameters`' reason above.
    if !row.tags.is_empty() {
        operation.insert("tags".to_owned(), json!(row.tags));
    }
    // The summary and description from the doc comment, already split into its two halves by
    // `nvs_types::routes`'s `doc_comment`: a method with no doc comment carries
    // neither member, for `parameters`' reason above.
    if let Some(summary) = &row.summary {
        operation.insert("summary".to_owned(), json!(summary));
    }
    if let Some(description) = &row.description {
        operation.insert("description".to_owned(), json!(description));
    }
    let params: Vec<Value> = row
        .params
        .iter()
        .filter(|param| !(param.source == ParamIn::Path && Some(param.name.as_str()) == left_out))
        .map(parameter)
        .collect();
    if !params.is_empty() {
        operation.insert("parameters".to_owned(), Value::Array(params));
    }
    // `#[Api]`'s `security`, as 3.1's Security Requirement Object: one object per
    // scheme name, each holding the empty scope list. Every name is one of
    // `components.securitySchemes`, because the front end refuses any other.
    // The scopes are empty because `#[Api]` has no field to name them yet.
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
    if !row.bodies.is_empty() {
        operation.insert("requestBody".to_owned(), request_body(row, schemas));
    }
    operation.insert("responses".to_owned(), responses(row, schemas));
    Value::Object(operation)
}

/// The operation's `requestBody`, from the body reads the handler wrote
/// (`rule:routing/a-request-body-is-read-from-the-handlers-own-call`).
///
/// Each kind is one media type, and two reads of one kind are a `oneOf` of
/// their schemas. The body is `required` when a JSON read is there, because
/// `jsonAs` throws on an empty body, or when a form read's class has a
/// required field, because `postAs` throws when one is missing.
fn request_body(row: &Route, schemas: &mut Schemas<'_>) -> Value {
    let mut content = Map::new();
    let mut required = false;
    for (kind, media) in [
        (BodyKind::Json, "application/json"),
        (BodyKind::Form, "application/x-www-form-urlencoded"),
    ] {
        let mut found: Vec<Value> = Vec::new();
        for body in row.bodies.iter().filter(|body| body.kind == kind) {
            required |= match kind {
                BodyKind::Json => true,
                BodyKind::Form => schemas.has_required_field(&body.ty),
            };
            let schema = schemas.body(&body.ty);
            if !found.contains(&schema) {
                found.push(schema);
            }
        }
        let schema = match found.len() {
            0 => continue,
            1 => found.remove(0),
            _ => json!({"oneOf": found}),
        };
        content.insert(media.to_owned(), json!({"schema": schema}));
    }
    json!({"required": required, "content": Value::Object(content)})
}

/// The response body: the handler's declared return type, plus `#[Api]`'s
/// `errors` — the responses the declared type cannot state.
///
/// **`200` is the return type's and outranks an `errors` entry that names it.** `#[Api]` may add
/// and may not contradict, so where both speak for one status the declared
/// return type is the one the code stated; the entry is dropped rather than
/// reported, because this module is a renderer and every diagnostic `rule:routing/api-document-is-generated-from-the-route-table`
/// has is the front end's.
///
/// An error response carries its class as the `description`, and the class's
/// schema as its `content` where the class has a JSON codec. A class without
/// one has no body a document can describe, so that response has no
/// `content`.
///
/// `description` is required of every response object in 3.1, and `success` is
/// what a generated one can honestly say: the *prose* about an operation is its
/// doc comment, which the operation already carries, and repeating it here would
/// put the same sentence in a document twice.
///
/// A `void` handler answers with no body, so the response carries no `content`
/// rather than an empty schema — the two mean different things to a generated
/// client. Everything else is JSON, through the same codec:
/// the codec a return type reaches this document through is
/// `rule:core-classes/derive-attribute`'s, and that codec is
/// JSON.
fn responses(row: &Route, schemas: &mut Schemas<'_>) -> Value {
    let mut success = Map::new();
    success.insert("description".to_owned(), json!("success"));
    if let Some(returns) = row.returns.as_ref().filter(|ty| **ty != BodyTy::Void) {
        let mut media = Map::new();
        media.insert("schema".to_owned(), schemas.body(returns));
        // `#[Api]`'s `example` sits beside the schema it is an example of, which is
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
        let status = error.status.to_string();
        if responses.contains_key(&status) {
            continue;
        }
        let mut response = Map::new();
        response.insert("description".to_owned(), json!(error.class));
        if schemas.codecs.codec(&error.class).is_some() {
            let schema = schemas.class(&error.class);
            response.insert(
                "content".to_owned(),
                json!({"application/json": {"schema": schema}}),
            );
        }
        responses.insert(status, Value::Object(response));
    }
    Value::Object(responses)
}

/// `#[Api]`'s `example` as JSON, or `None` where some part of it has no JSON form at
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
        ConstArg::Int(value) | ConstArg::EnumInt(value) => json!(value),
        ConstArg::Uint(value) | ConstArg::EnumUint(value) => json!(value),
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
        // An attached example is folded with no declared type, so a fraction
        // in it is a `float` and never reaches this arm.
        | ConstArg::Decimal { .. }
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
///
/// A path parameter is always `required`, because 3.1 allows nothing else. A
/// `{name?}` that may be absent is the second path [`templates`] writes.
fn parameter(param: &RouteParam) -> Value {
    json!({
        "name": param.name,
        "in": match param.source {
            ParamIn::Path => "path",
            ParamIn::Query => "query",
        },
        "required": param.required || param.source == ParamIn::Path,
        "schema": schema(param.ty.as_deref(), param.admits().as_deref(), param.parses),
    })
}

/// A parameter "by declared type": the JSON Schema one Novis type renders as, and its
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
/// set of allowed values has no entry in the list above and never will — its
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
/// A type the list above has no arm for joins that same empty schema and says
/// nothing further, which is what a document that decides nothing looks like
/// where the compiler knows a type this module has no mapping for.
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
/// decided while compiling, which [`RouteParam::admits`] hands over beside the
/// members of a set of allowed values — one list, because a document listing anything else
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
        // The one class a capture may be declared at
        // (`rule:routing/a-capture-narrows-to-a-closed-set`), and the format
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
