//! `nvs check --json` — every diagnostic the text renderer prints, as one
//! document.
//!
//! `rule:ide/check-json-is-the-diagnostic-record-as-a-document` is what this
//! is: the same [`nvs_diagnostics::Diagnostic`] records, in the same order,
//! under a schema frozen the way `nvs ast --json`'s is. The text rendering
//! stays the default and nothing about it changes, because the Tasks
//! `problemMatcher` of `rule:ide/tasks-carry-a-problem-matcher` reads it; this
//! document exists for CI and for the agents that drive this compiler, which
//! both want a field rather than a line to parse.
//!
//! **The document is the record, not the rendering.** Every field here is one
//! the `Diagnostic` already carries, and the only thing this module decides is
//! how to spell it. Nothing is recovered from rendered text and nothing is
//! computed that the terminal renderer would not also print — a record that
//! appears here and not there, or the reverse, would be a second diagnostic
//! surface to keep in step.
//!
//! **A position is given twice**, as the byte `span` the compiler works in and
//! as one-based `line`/`column` pairs, because the two consumers want
//! different ones: an editor applying a suggestion wants the offsets, and a CI
//! log linking to a file wants what the `--> path:line:col` header shows. The
//! columns count `char`s rather than bytes, which is
//! [`nvs_diagnostics::SourceFile::line_col`]'s own decision and the one the
//! text header is already rendered from, so the two cannot disagree.
//!
//! **`help` is its own array** rather than notes a reader has to sort out.
//! `Diagnostic::with_help` is the only thing that writes the `help: ` prefix
//! into a note, so splitting on it here recovers the distinction the builder
//! made rather than guessing at one — and a consumer reading `help` gets the
//! text without the prefix, since the prefix is the plaintext rendering's and
//! not the record's.
//!
//! Every field is always present. An absent code is `null` and an empty list
//! is `[]`, so nothing downstream has to tell "no suggestions" from "this
//! version did not emit suggestions".

use nvs_diagnostics::{
    BytePos, Diagnostic, Label, LabelStyle, SourceFile, SourceMap, Span, Suggestion,
};
use serde_json::{Map, Value, json};

/// The shape below, frozen. It goes up when a field is removed or its meaning
/// changes; a field added beside the others does not move it, because a
/// consumer reading by name is unaffected by one.
const SCHEMA_VERSION: u64 = 1;

/// What [`Diagnostic::with_help`] writes in front of a note, and the only
/// marker there is that one was a help line.
const HELP_PREFIX: &str = "help: ";

/// Every diagnostic as one document, pretty-printed.
///
/// Pretty rather than compact — the opposite of `nvs ast --json`, and for the
/// opposite reason: an AST document is thousands of nodes nobody reads by eye,
/// while this one is the handful of things wrong with a file and is read in a
/// CI log as often as it is parsed.
pub(crate) fn document<'a>(
    diagnostics: impl Iterator<Item = &'a Diagnostic>,
    map: &SourceMap,
) -> String {
    let document = json!({
        "schemaVersion": SCHEMA_VERSION,
        "diagnostics": diagnostics
            .map(|diagnostic| diagnostic_json(diagnostic, map))
            .collect::<Vec<_>>(),
    });
    // It cannot fail: the document holds strings, bools and integers, and
    // `serde_json` only errors on a non-string map key or a non-finite float.
    serde_json::to_string_pretty(&document).expect("the document holds no unserializable value")
}

/// One record: what the terminal renderer prints as one block.
fn diagnostic_json(diagnostic: &Diagnostic, map: &SourceMap) -> Value {
    let (help, notes): (Vec<&str>, Vec<&str>) = diagnostic
        .notes
        .iter()
        .map(String::as_str)
        .partition(|note| note.starts_with(HELP_PREFIX));
    json!({
        // The same spelling the text head carries, so `severity` and
        // `error[E0104]` name one thing.
        "severity": diagnostic.severity.label(),
        "code": diagnostic.code.map(|code| code.as_str()),
        "message": diagnostic.message,
        "labels": diagnostic
            .labels
            .iter()
            .map(|label| label_json(label, map))
            .collect::<Vec<_>>(),
        "notes": notes,
        "help": help
            .iter()
            .map(|line| &line[HELP_PREFIX.len()..])
            .collect::<Vec<_>>(),
        "suggestions": diagnostic
            .suggestions
            .iter()
            .map(|suggestion| suggestion_json(suggestion, map))
            .collect::<Vec<_>>(),
    })
}

/// One span with something to say about it.
fn label_json(label: &Label, map: &SourceMap) -> Value {
    let mut out = location_json(label.span, map);
    out.insert(
        "style".into(),
        Value::from(match label.style {
            LabelStyle::Primary => "primary",
            LabelStyle::Secondary => "secondary",
        }),
    );
    out.insert("message".into(), Value::from(label.message.clone()));
    Value::Object(out)
}

/// One machine-applicable edit, with the text to put there.
fn suggestion_json(suggestion: &Suggestion, map: &SourceMap) -> Value {
    let mut out = location_json(suggestion.span, map);
    out.insert(
        "replacement".into(),
        Value::from(suggestion.replacement.clone()),
    );
    out.insert("message".into(), Value::from(suggestion.message.clone()));
    out.insert("safe".into(), Value::from(suggestion.safe));
    out.insert("alternative".into(), Value::from(suggestion.alternative));
    Value::Object(out)
}

/// Where a span is, in both of the two ways a consumer asks.
///
/// `file` and the two positions are `null` together when the map does not hold
/// the span's file, which a diagnostic reported against a file the compiler
/// loaded cannot be. The text renderer drops such a label entirely; this keeps
/// it, because a record missing from the document is the one failure this
/// surface must not have.
fn location_json(span: Span, map: &SourceMap) -> Map<String, Value> {
    let file = map.get(span.file);
    let mut out = Map::new();
    out.insert(
        "file".into(),
        file.map_or(Value::Null, |file| Value::from(file.name())),
    );
    out.insert("span".into(), json!([span.start, span.end]));
    out.insert(
        "start".into(),
        file.map_or(Value::Null, |file| position_json(file, span.start)),
    );
    out.insert(
        "end".into(),
        file.map_or(Value::Null, |file| position_json(file, span.end)),
    );
    out
}

/// A byte offset as the one-based line and column the `-->` header shows.
fn position_json(file: &SourceFile, pos: BytePos) -> Value {
    let (line, column) = file.line_col(pos);
    json!({ "line": line + 1, "column": column + 1 })
}
