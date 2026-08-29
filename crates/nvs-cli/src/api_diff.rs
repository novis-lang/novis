//! `nvs api diff` — [ADR 0085](../../../docs/adr/0085-openapi-is-generated-from-the-route-table.md)
//! § 4's gate: two OpenAPI documents in, one classification per change out, and
//! a non-zero exit on a breaking one.
//!
//! The comparison is over [`Value`]s, not text. The emitter
//! ([`crate::openapi`]) writes through `serde_json` and this reads through it,
//! so the two halves of § 4 cannot end up with different opinions about what
//! JSON is — and a document a *user* produced (the last release's artifact,
//! checked into a repository and reformatted by whatever touched it since) is
//! compared by its content rather than by its whitespace.
//!
//! ## The classification (§ 4)
//!
//! Three classes, and the ADR names most of the members of each:
//!
//! * **Breaking** — a removed operation, a removed parameter or property, one
//!   that became required, a narrowed type, a removed enum case, a removed
//!   status code, a tightened constraint. Exits non-zero.
//! * **Additive** — a new operation, a new optional parameter or property, a
//!   new enum case, a new status code, a widened type, a dropped constraint.
//! * **Cosmetic** — everything else: a title, a version, a summary, a
//!   description, an `operationId`, a tag.
//!
//! **The rule is applied mechanically and errs towards breaking**, which is the
//! ADR's own reading: it says the gate "will occasionally be wrong at the
//! margins — a change that is technically breaking and practically harmless",
//! and deliberately ships no suppression mechanism, because a gate with an
//! escape hatch is an advisory. Two places where that shows: a *removed*
//! parameter is breaking whether or not it was optional, and a changed
//! `format` is breaking, because `{"type": "string"}` and
//! `{"type": "string", "format": "decimal"}` accept different sets of strings.
//!
//! ## What has no producer yet
//!
//! [`crate::openapi`]'s module doc lists the six § 1 rows the route table does
//! not supply, and two of them are the reason the walk below reads members no
//! document Novis writes today carries: `responses` and `requestBody`. They are
//! implemented here rather than left for the session that lands them because
//! § 4's gate is defined over *documents*, not over this emitter's current
//! output — the old side of a diff is whatever a team's last release wrote, and
//! a gate that silently ignored a member it did not recognise would report
//! "no change" over a document that removed every response in it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::ExitCode;

use serde_json::Value;

/// How a single change is classified. Ordered worst-first, which is the order
/// the report prints in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Class {
    Breaking,
    Additive,
    Cosmetic,
}

impl Class {
    /// The word the report prints.
    const fn label(self) -> &'static str {
        match self {
            Self::Breaking => "breaking",
            Self::Additive => "additive",
            Self::Cosmetic => "cosmetic",
        }
    }
}

/// One classified change, at one place in the document.
#[derive(Debug)]
pub(crate) struct Change {
    class: Class,
    /// Where it is, as a dotted path into the document — `paths./items.get`,
    /// `info.title`. A reader has the old and the new document in front of
    /// them, so the location is what they need to find the change; restating
    /// both values is what `git diff` is for.
    at: String,
    /// What changed, in a clause.
    what: String,
}

/// `nvs api diff <old.json> <new.json>` — the whole subcommand.
///
/// Reads both documents, prints one line per change worst-first, and exits
/// non-zero if any of them is breaking. A document that cannot be read or is
/// not JSON is a failure naming the file, not an empty diff: "nothing changed"
/// and "I could not tell" must not share an exit code in a gate.
pub(crate) fn run(old_path: &Path, new_path: &Path) -> ExitCode {
    let (Some(old), Some(new)) = (read(old_path), read(new_path)) else {
        return ExitCode::FAILURE;
    };

    let changes = diff(&old, &new);
    if changes.is_empty() {
        println!("no change");
        return ExitCode::SUCCESS;
    }

    let mut counts: BTreeMap<Class, usize> = BTreeMap::new();
    for change in &changes {
        *counts.entry(change.class).or_default() += 1;
        println!("{}: {} — {}", change.class.label(), change.at, change.what);
    }
    let count = |class: Class| counts.get(&class).copied().unwrap_or(0);
    println!(
        "{} change(s): {} breaking, {} additive, {} cosmetic",
        changes.len(),
        count(Class::Breaking),
        count(Class::Additive),
        count(Class::Cosmetic),
    );

    if counts.contains_key(&Class::Breaking) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// One document, or `None` having said on standard error why not.
fn read(path: &Path) -> Option<Value> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("cannot read {}: {err}", path.display());
            return None;
        }
    };
    match serde_json::from_str(&text) {
        Ok(document) => Some(document),
        Err(err) => {
            eprintln!("{} is not an OpenAPI document: {err}", path.display());
            None
        }
    }
}

/// Every change between two documents, worst class first and then by location.
///
/// The sort is what makes the report deterministic for a reader and for a CI
/// log: the walk itself is already ordered ([`BTreeMap`] and [`BTreeSet`]
/// throughout, never a hash map), and this puts the breaking lines where they
/// are read first.
pub(crate) fn diff(old: &Value, new: &Value) -> Vec<Change> {
    let mut changes = Vec::new();
    paths_diff(
        member(old, "paths"),
        member(new, "paths"),
        "paths",
        &mut changes,
    );
    // § 4 is a gate over what an API *promises*, so `components.schemas` is
    // walked with the same rules: a named schema is reachable from an operation
    // through a `$ref`, and removing one removes whatever referenced it.
    schemas_diff(
        member(old, "components").and_then(|c| member(c, "schemas")),
        member(new, "components").and_then(|c| member(c, "schemas")),
        "components.schemas",
        &mut changes,
    );
    cosmetic_diff(
        member(old, "openapi"),
        member(new, "openapi"),
        "openapi",
        &mut changes,
    );
    // A member at a time rather than the object at once: `info.title` and
    // `info.version` are what actually differ, and "info changed" makes a
    // reader open both files to find out which.
    for (name, before, after) in pairs(member(old, "info"), member(new, "info")) {
        cosmetic_diff(before, after, &format!("info.{name}"), &mut changes);
    }
    changes.sort_by(|a, b| (a.class, &a.at, &a.what).cmp(&(b.class, &b.at, &b.what)));
    changes
}

/// `paths`: one entry per path, each holding one operation per verb.
fn paths_diff(old: Option<&Value>, new: Option<&Value>, at: &str, out: &mut Vec<Change>) {
    for (path, old_item, new_item) in pairs(old, new) {
        let at = format!("{at}.{path}");
        match (old_item, new_item) {
            (Some(old_item), Some(new_item)) => {
                for (verb, old_op, new_op) in pairs(Some(old_item), Some(new_item)) {
                    let at = format!("{at}.{verb}");
                    match (old_op, new_op) {
                        (Some(old_op), Some(new_op)) => operation_diff(old_op, new_op, &at, out),
                        (Some(_), None) => out.push(breaking(&at, "operation removed")),
                        (None, Some(_)) => out.push(additive(&at, "operation added")),
                        (None, None) => {}
                    }
                }
            }
            // A whole path at once, rather than one line per verb under it: a
            // removed path is one edit and reads as one.
            (Some(_), None) => out.push(breaking(&at, "path removed, with every operation on it")),
            (None, Some(_)) => out.push(additive(&at, "path added")),
            (None, None) => {}
        }
    }
}

/// One operation object: its parameters, its request body, its responses, and
/// the members that are prose.
fn operation_diff(old: &Value, new: &Value, at: &str, out: &mut Vec<Change>) {
    parameters_diff(
        member(old, "parameters"),
        member(new, "parameters"),
        at,
        out,
    );
    body_diff(
        member(old, "requestBody"),
        member(new, "requestBody"),
        &format!("{at}.requestBody"),
        out,
    );
    responses_diff(
        member(old, "responses"),
        member(new, "responses"),
        &format!("{at}.responses"),
        out,
    );
    // `deprecated` is here rather than among the breaking members on purpose:
    // it is the *announcement* of a removal and not the removal, and a gate
    // that refused it would leave a team no way to announce one.
    for name in [
        "operationId",
        "summary",
        "description",
        "tags",
        "deprecated",
    ] {
        cosmetic_diff(
            member(old, name),
            member(new, name),
            &format!("{at}.{name}"),
            out,
        );
    }
}

/// `parameters` is an array, so it is keyed by the pair that identifies a
/// parameter in 3.1 — `in` and `name` — before anything is compared. Order
/// within the array carries no meaning and a reordering is not a change.
fn parameters_diff(old: Option<&Value>, new: Option<&Value>, at: &str, out: &mut Vec<Change>) {
    let (old, new) = (by_key(old), by_key(new));
    for key in old.keys().chain(new.keys()).collect::<BTreeSet<_>>() {
        let at = format!("{at}.{key}");
        match (old.get(key), new.get(key)) {
            (Some(old_param), Some(new_param)) => {
                required_diff(old_param, new_param, &at, "parameter", out);
                schema_diff(
                    member(old_param, "schema"),
                    member(new_param, "schema"),
                    &format!("{at}.schema"),
                    out,
                );
                cosmetic_diff(
                    member(old_param, "description"),
                    member(new_param, "description"),
                    &format!("{at}.description"),
                    out,
                );
            }
            // Breaking whether or not it was optional: what the server stops
            // reading, a client is still sending.
            (Some(_), None) => out.push(breaking(&at, "parameter removed")),
            (None, Some(param)) => out.push(if is_required(param) {
                breaking(&at, "required parameter added")
            } else {
                additive(&at, "optional parameter added")
            }),
            (None, None) => {}
        }
    }
}

/// A parameter or a property that became required, in either direction.
fn required_diff(old: &Value, new: &Value, at: &str, noun: &str, out: &mut Vec<Change>) {
    match (is_required(old), is_required(new)) {
        (false, true) => out.push(breaking(at, &format!("optional {noun} became required"))),
        (true, false) => out.push(additive(at, &format!("required {noun} became optional"))),
        _ => {}
    }
}

/// `requestBody`: one `required` flag and one schema per media type.
fn body_diff(old: Option<&Value>, new: Option<&Value>, at: &str, out: &mut Vec<Change>) {
    match (old, new) {
        (Some(old), Some(new)) => {
            required_diff(old, new, at, "request body", out);
            content_diff(
                member(old, "content"),
                member(new, "content"),
                &format!("{at}.content"),
                out,
            );
        }
        (Some(_), None) => out.push(additive(at, "request body no longer read")),
        (None, Some(body)) => out.push(if is_required(body) {
            breaking(at, "required request body added")
        } else {
            additive(at, "optional request body added")
        }),
        (None, None) => {}
    }
}

/// `responses`, keyed by status code. § 4 names a changed status code as
/// breaking, and a removed one is that change seen from the document: the code
/// a client branches on is gone.
fn responses_diff(old: Option<&Value>, new: Option<&Value>, at: &str, out: &mut Vec<Change>) {
    for (status, old_response, new_response) in pairs(old, new) {
        let at = format!("{at}.{status}");
        match (old_response, new_response) {
            (Some(old_response), Some(new_response)) => content_diff(
                member(old_response, "content"),
                member(new_response, "content"),
                &format!("{at}.content"),
                out,
            ),
            (Some(_), None) => out.push(breaking(&at, "status code removed")),
            (None, Some(_)) => out.push(additive(&at, "status code added")),
            (None, None) => {}
        }
    }
}

/// `content`, keyed by media type: one schema each.
fn content_diff(old: Option<&Value>, new: Option<&Value>, at: &str, out: &mut Vec<Change>) {
    for (media, old_entry, new_entry) in pairs(old, new) {
        let at = format!("{at}.{media}");
        match (old_entry, new_entry) {
            (Some(old_entry), Some(new_entry)) => schema_diff(
                member(old_entry, "schema"),
                member(new_entry, "schema"),
                &format!("{at}.schema"),
                out,
            ),
            (Some(_), None) => out.push(breaking(&at, "media type removed")),
            (None, Some(_)) => out.push(additive(&at, "media type added")),
            (None, None) => {}
        }
    }
}

/// `components.schemas`, keyed by name.
fn schemas_diff(old: Option<&Value>, new: Option<&Value>, at: &str, out: &mut Vec<Change>) {
    for (name, old_schema, new_schema) in pairs(old, new) {
        let at = format!("{at}.{name}");
        match (old_schema, new_schema) {
            (Some(old_schema), Some(new_schema)) => {
                schema_diff(Some(old_schema), Some(new_schema), &at, out);
            }
            (Some(_), None) => out.push(breaking(&at, "schema removed")),
            (None, Some(_)) => out.push(additive(&at, "schema added")),
            (None, None) => {}
        }
    }
}

/// The keywords that *narrow* what a value may be. Appearing or changing is a
/// narrowing and disappearing is a widening — the direction is not read out of
/// the values themselves, because a `pattern` and a `maximum` do not compare
/// the same way and guessing wrong in a gate is worse than one honest rule.
const CONSTRAINTS: &[&str] = &[
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "minLength",
    "maxLength",
    "pattern",
    "minItems",
    "maxItems",
    "uniqueItems",
    "format",
];

/// Two JSON Schemas, the recursion every schema-shaped member above reaches.
///
/// § 4's "a narrowed type" and "a removed enum case" both live here, and so
/// does the properties walk that makes "a removed or newly required field"
/// mean something: a field is a property of an object schema.
fn schema_diff(old: Option<&Value>, new: Option<&Value>, at: &str, out: &mut Vec<Change>) {
    let (Some(old), Some(new)) = (old, new) else {
        match (old, new) {
            // The empty schema is *any*, so gaining one is a narrowing and
            // losing one is a widening — the same rule as `type`, one level up.
            (Some(_), None) => out.push(additive(at, "schema dropped: anything is now accepted")),
            (None, Some(_)) => out.push(breaking(at, "schema added where anything was accepted")),
            _ => {}
        }
        return;
    };
    if old == new {
        return;
    }

    types_diff(old, new, at, out);
    enum_diff(old, new, at, out);
    properties_diff(old, new, at, out);
    ref_diff(old, new, at, out);

    for name in CONSTRAINTS {
        match (member(old, name), member(new, name)) {
            (Some(before), Some(after)) if before != after => {
                out.push(breaking(&format!("{at}.{name}"), "constraint changed"));
            }
            (None, Some(_)) => out.push(breaking(&format!("{at}.{name}"), "constraint added")),
            (Some(_), None) => out.push(additive(&format!("{at}.{name}"), "constraint dropped")),
            _ => {}
        }
    }
    for name in ["title", "description", "example", "examples", "default"] {
        cosmetic_diff(
            member(old, name),
            member(new, name),
            &format!("{at}.{name}"),
            out,
        );
    }
    schema_diff(
        member(old, "items"),
        member(new, "items"),
        &format!("{at}.items"),
        out,
    );
}

/// § 4's "a narrowed type". 3.1 writes a nullable string as
/// `"type": ["string", "null"]`, so a type is a *set* and the comparison is a
/// subset one: losing a member narrows, gaining one widens, and swapping one
/// for another is both — reported as the breaking half.
fn types_diff(old: &Value, new: &Value, at: &str, out: &mut Vec<Change>) {
    let (before, after) = (type_set(old), type_set(new));
    if before == after {
        return;
    }
    let at = format!("{at}.type");
    match (before.is_empty(), after.is_empty()) {
        (true, false) => out.push(breaking(&at, "type declared where anything was accepted")),
        (false, true) => out.push(additive(&at, "type dropped: anything is now accepted")),
        _ if after.is_subset(&before) => {
            out.push(breaking(&at, &changed("type narrowed", &before, &after)))
        }
        _ if before.is_subset(&after) => {
            out.push(additive(&at, &changed("type widened", &before, &after)))
        }
        _ => out.push(breaking(&at, &changed("type changed", &before, &after))),
    }
}

/// § 4's enum cases, in both directions: a removed case is breaking, a new one
/// is additive, and a schema that gained an `enum` at all narrowed to it.
fn enum_diff(old: &Value, new: &Value, at: &str, out: &mut Vec<Change>) {
    let (before, after) = (enum_cases(old), enum_cases(new));
    if before == after {
        return;
    }
    let at = format!("{at}.enum");
    match (member(old, "enum"), member(new, "enum")) {
        (None, Some(_)) => out.push(breaking(&at, "enum added: the value is now a closed set")),
        (Some(_), None) => out.push(additive(&at, "enum dropped")),
        _ => {
            for case in before.difference(&after) {
                out.push(breaking(&at, &format!("case `{case}` removed")));
            }
            for case in after.difference(&before) {
                out.push(additive(&at, &format!("case `{case}` added")));
            }
        }
    }
}

/// An object schema's `properties`, which is where § 4's "a removed or newly
/// required field" is decided. `required` is a sibling array in JSON Schema
/// rather than a flag on the property, so it is read from the parent.
fn properties_diff(old: &Value, new: &Value, at: &str, out: &mut Vec<Change>) {
    let (old_required, new_required) = (required_names(old), required_names(new));
    for (name, old_property, new_property) in
        pairs(member(old, "properties"), member(new, "properties"))
    {
        let at = format!("{at}.properties.{name}");
        match (old_property, new_property) {
            (Some(old_property), Some(new_property)) => {
                match (old_required.contains(&name), new_required.contains(&name)) {
                    (false, true) => out.push(breaking(&at, "optional field became required")),
                    (true, false) => out.push(additive(&at, "required field became optional")),
                    _ => {}
                }
                schema_diff(Some(old_property), Some(new_property), &at, out);
            }
            (Some(_), None) => out.push(breaking(&at, "field removed")),
            (None, Some(_)) => out.push(if new_required.contains(&name) {
                breaking(&at, "required field added")
            } else {
                additive(&at, "optional field added")
            }),
            (None, None) => {}
        }
    }
    // A name in `required` with no property beside it is legal and means the
    // field is required with no schema, so the two walks do not fully overlap.
    for name in new_required.difference(&old_required) {
        if member(old, "properties")
            .and_then(|p| member(p, name))
            .is_none()
            && member(new, "properties")
                .and_then(|p| member(p, name))
                .is_none()
        {
            out.push(breaking(
                &format!("{at}.required.{name}"),
                "field newly required",
            ));
        }
    }
}

/// A `$ref` that moved. Pointing somewhere else is pointing at a different
/// schema, and this walk does not follow it — the target's own changes are
/// reported under `components.schemas` instead, once, rather than once per
/// operation that reaches it.
fn ref_diff(old: &Value, new: &Value, at: &str, out: &mut Vec<Change>) {
    match (member(old, "$ref"), member(new, "$ref")) {
        (Some(before), Some(after)) if before != after => {
            out.push(breaking(&format!("{at}.$ref"), "reference retargeted"));
        }
        (None, Some(_)) => out.push(breaking(&format!("{at}.$ref"), "reference added")),
        (Some(_), None) => out.push(breaking(&format!("{at}.$ref"), "reference dropped")),
        _ => {}
    }
}

/// A member whose change breaks nothing: prose, a title, an id.
fn cosmetic_diff(old: Option<&Value>, new: Option<&Value>, at: &str, out: &mut Vec<Change>) {
    if old != new {
        out.push(Change {
            class: Class::Cosmetic,
            at: at.to_owned(),
            what: match (old, new) {
                (Some(_), None) => "removed".to_owned(),
                (None, Some(_)) => "added".to_owned(),
                _ => "changed".to_owned(),
            },
        });
    }
}

/// A schema's `type` as a set, empty where none is declared.
fn type_set(schema: &Value) -> BTreeSet<String> {
    match member(schema, "type") {
        Some(Value::String(one)) => BTreeSet::from([one.clone()]),
        Some(Value::Array(many)) => many
            .iter()
            .filter_map(|ty| ty.as_str().map(ToOwned::to_owned))
            .collect(),
        _ => BTreeSet::new(),
    }
}

/// A schema's `enum` cases as a set, by their JSON text so that a case of any
/// type — a string, a number — compares and prints the same way.
fn enum_cases(schema: &Value) -> BTreeSet<String> {
    match member(schema, "enum") {
        Some(Value::Array(cases)) => cases.iter().map(ToString::to_string).collect(),
        _ => BTreeSet::new(),
    }
}

/// An object schema's `required` names.
fn required_names(schema: &Value) -> BTreeSet<String> {
    match member(schema, "required") {
        Some(Value::Array(names)) => names
            .iter()
            .filter_map(|name| name.as_str().map(ToOwned::to_owned))
            .collect(),
        _ => BTreeSet::new(),
    }
}

/// `"narrowed: {int} -> {int, null}"` — the two sets, for a reader who has the
/// location but not the documents open.
fn changed(what: &str, before: &BTreeSet<String>, after: &BTreeSet<String>) -> String {
    let render = |set: &BTreeSet<String>| set.iter().cloned().collect::<Vec<_>>().join(", ");
    format!("{what}: {{{}}} -> {{{}}}", render(before), render(after))
}

/// Whether a parameter or request body says it is required. Absent means
/// optional, which is 3.1's own default.
fn is_required(value: &Value) -> bool {
    member(value, "required").and_then(Value::as_bool) == Some(true)
}

/// One member of an object, or `None` for a member that is absent — and for a
/// `null`, which a document writes to mean the same thing.
fn member<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value.get(name).filter(|found| !found.is_null())
}

/// Every key of two objects, in sorted order, with each side's value. The one
/// walk every keyed member above shares: an absent object contributes no keys,
/// so `None` against an object reads as "everything was added".
fn pairs<'a>(
    old: Option<&'a Value>,
    new: Option<&'a Value>,
) -> Vec<(String, Option<&'a Value>, Option<&'a Value>)> {
    let keys = |value: Option<&Value>| -> BTreeSet<String> {
        value
            .and_then(Value::as_object)
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default()
    };
    keys(old)
        .union(&keys(new))
        .map(|key| {
            (
                key.clone(),
                old.and_then(|value| member(value, key)),
                new.and_then(|value| member(value, key)),
            )
        })
        .collect()
}

/// `parameters` as a map from 3.1's identifying pair to the parameter object.
fn by_key(parameters: Option<&Value>) -> BTreeMap<String, &Value> {
    let mut keyed = BTreeMap::new();
    for param in parameters.and_then(Value::as_array).into_iter().flatten() {
        let name = member(param, "name").and_then(Value::as_str).unwrap_or("");
        let source = member(param, "in").and_then(Value::as_str).unwrap_or("");
        keyed.insert(format!("{source}.{name}"), param);
    }
    keyed
}

fn breaking(at: &str, what: &str) -> Change {
    Change {
        class: Class::Breaking,
        at: at.to_owned(),
        what: what.to_owned(),
    }
}

fn additive(at: &str, what: &str) -> Change {
    Change {
        class: Class::Additive,
        at: at.to_owned(),
        what: what.to_owned(),
    }
}
