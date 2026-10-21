//! A compiler diagnostic as `rule:errors/diagnostic-record`'s record —
//! `rule:errors/record-producers`' compiler producer.
//!
//! [`record`] is the one place a [`Diagnostic`] becomes content, and every
//! rendering of one reads the [`Record`] it builds: the terminal text in
//! [`crate::Renderer`], and any other rendering `nvs-render` gives.
//!
//! # The record's shape
//!
//! * The envelope's `level` is the closest log level, and its `message` is the
//!   headline.
//! * [`SEVERITY`] carries [`Severity::label`], because five severities do not
//!   map one to one onto five log levels: `help` and `note` are both `Info`.
//! * [`CODE`] carries the code, absent when the diagnostic has none.
//! * Its nodes are one [`Node::Span`] per label whose file the map holds, in
//!   the order the diagnostic wrote them. Each carries its first line's text,
//!   so the record renders without the map.
//! * [`NOTES`] is one single-key map per note, the key `note` or `help`, in
//!   the order written, absent when there are none.
//! * [`SUGGESTIONS`] is one map per suggestion, absent when there are none.
//!
//! # What it spends
//!
//! One record per diagnostic, built on the compile path only when there is a
//! diagnostic to report, and freed when it has been rendered: a `String` per
//! text and one copy of each labelled source line.

use nvs_render::{Level, Node, Record, Rendered, Scalar};

use crate::diagnostic::{Diagnostic, LabelStyle, Severity};
use crate::source::SourceMap;
use crate::span::Span;

/// The field naming the diagnostic's severity, as [`Severity::label`] writes it.
pub const SEVERITY: &str = "severity";
/// The field carrying the diagnostic's code.
pub const CODE: &str = "code";
/// The field carrying the diagnostic's notes and help lines.
pub const NOTES: &str = "notes";
/// The field carrying the diagnostic's suggested edits.
pub const SUGGESTIONS: &str = "suggestions";

/// What a note starts with when it is a help line rather than a note.
const HELP_PREFIX: &str = "help: ";

/// `d` as a record, its spans read over `map`.
#[must_use]
pub fn record(d: &Diagnostic, map: &SourceMap) -> Record {
    let mut record = Record::at(level(d.severity));
    record.envelope.message = Some(Rendered::new(&d.message));
    record
        .envelope
        .fields
        .push((SEVERITY.to_owned(), text(d.severity.label())));
    if let Some(code) = d.code {
        record
            .envelope
            .fields
            .push((CODE.to_owned(), text(code.as_str())));
    }
    if !d.notes.is_empty() {
        let notes = d
            .notes
            .iter()
            .map(|note| match note.strip_prefix(HELP_PREFIX) {
                Some(rest) => Node::Map(vec![(Rendered::new("help"), text(rest))]),
                None => Node::Map(vec![(Rendered::new("note"), text(note))]),
            })
            .collect();
        record
            .envelope
            .fields
            .push((NOTES.to_owned(), Node::Sequence(notes)));
    }
    if !d.suggestions.is_empty() {
        let suggestions = d
            .suggestions
            .iter()
            .map(|s| {
                let mut entries = vec![
                    (Rendered::new("message"), text(&s.message)),
                    (Rendered::new("replacement"), text(&s.replacement)),
                    (Rendered::new("safe"), Node::Scalar(Scalar::Bool(s.safe))),
                    (
                        Rendered::new("alternative"),
                        Node::Scalar(Scalar::Bool(s.alternative)),
                    ),
                ];
                if let Some(at) = span(s.span, false, "", map) {
                    entries.push((Rendered::new("at"), at));
                }
                Node::Map(entries)
            })
            .collect();
        record
            .envelope
            .fields
            .push((SUGGESTIONS.to_owned(), Node::Sequence(suggestions)));
    }
    record.nodes = d
        .labels
        .iter()
        .filter_map(|l| span(l.span, l.style == LabelStyle::Primary, &l.message, map))
        .collect();
    record
}

/// The severity a record's [`SEVERITY`] field names, or `None` when it names
/// none or the record is not a diagnostic's.
#[must_use]
pub fn severity(record: &Record) -> Option<Severity> {
    let label = field_text(record, SEVERITY)?;
    [
        Severity::Note,
        Severity::Help,
        Severity::Warning,
        Severity::Error,
        Severity::Bug,
    ]
    .into_iter()
    .find(|s| s.label() == label)
}

/// The text of the string field `name`, or `None` when the record has no
/// such field or it is not a string.
#[must_use]
pub fn field_text<'a>(record: &'a Record, name: &str) -> Option<&'a str> {
    match field(record, name)? {
        Node::Scalar(Scalar::Str { text, .. }) => Some(text.as_str()),
        _ => None,
    }
}

/// The field `name`, or `None` when the record has none.
#[must_use]
pub fn field<'a>(record: &'a Record, name: &str) -> Option<&'a Node> {
    record
        .envelope
        .fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, node)| node)
}

/// The log level closest to `severity`.
const fn level(severity: Severity) -> Level {
    match severity {
        Severity::Note | Severity::Help => Level::Info,
        Severity::Warning => Level::Warn,
        Severity::Error => Level::Error,
        Severity::Bug => Level::Critical,
    }
}

/// A string scalar.
fn text(value: &str) -> Node {
    Node::Scalar(Scalar::Str {
        text: Rendered::new(value),
        bytes: value.len(),
    })
}

/// `at` as a span node, or `None` when the map does not hold its file.
fn span(at: Span, primary: bool, label: &str, map: &SourceMap) -> Option<Node> {
    let file = map.get(at.file)?;
    let (line, column) = file.line_col(at.start);
    // An empty span still points somewhere, so its end is never before its start.
    let (end_line, end_column) = file.line_col(at.end.max(at.start));
    Some(Node::Span {
        file: file.name().to_owned(),
        line: one_based(line),
        column: one_based(column),
        end_line: one_based(end_line),
        end_column: one_based(end_column),
        primary,
        label: Rendered::new(label),
        source: file.line_text(line).map(Rendered::new),
    })
}

/// A zero-based position as the one-based number a record carries.
fn one_based(zero_based: usize) -> u32 {
    u32::try_from(zero_based + 1).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code;

    /// The record carries everything the text prints: the severity, the code,
    /// one span per label with its line's text, the notes and the suggestions.
    #[test]
    fn a_diagnostic_is_a_record() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "$a = 1;\n$a = 2;\n");
        let d = Diagnostic::error(code::E_EVAL_UNSUPPORTED, "redefined")
            .with_secondary(Span::new(f, 0, 2), "first here")
            .with_primary(Span::new(f, 8, 10), "again here")
            .with_help("rename one");
        let r = record(&d, &map);

        assert_eq!(r.envelope.level, Level::Error);
        assert_eq!(r.envelope.message, Some(Rendered::new("redefined")));
        assert_eq!(field_text(&r, SEVERITY), Some("error"));
        assert_eq!(severity(&r), Some(Severity::Error));
        assert_eq!(
            field_text(&r, CODE),
            Some(code::E_EVAL_UNSUPPORTED.as_str())
        );
        assert_eq!(
            r.nodes[1],
            Node::Span {
                file: "t.nvs".to_owned(),
                line: 2,
                column: 1,
                end_line: 2,
                end_column: 3,
                primary: true,
                label: Rendered::new("again here"),
                source: Some(Rendered::new("$a = 2;")),
            }
        );
        assert_eq!(
            field(&r, NOTES),
            Some(&Node::Sequence(vec![Node::Map(vec![(
                Rendered::new("help"),
                text("rename one")
            )])]))
        );
        assert_eq!(field(&r, SUGGESTIONS), None);
    }

    /// A label in a file the map does not hold has nowhere to point, so the
    /// record leaves it out, as the text always has.
    #[test]
    fn a_label_outside_the_map_is_left_out() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "x\n");
        let d = Diagnostic::error(code::E_EVAL_UNSUPPORTED, "m")
            .with_primary(Span::new(f, 0, 1), "")
            .with_secondary(Span::new(crate::SourceId(7), 0, 1), "elsewhere");
        assert_eq!(record(&d, &map).nodes.len(), 1);
    }
}
