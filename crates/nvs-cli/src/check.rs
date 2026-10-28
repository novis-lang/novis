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
//! **The document is the record, not the rendering.** Each diagnostic is
//! rendered from the [`nvs_render::Record`] that [`nvs_diagnostics::record`]
//! builds, the one the terminal text is rendered from, and the only thing this
//! module decides is how to spell its fields. Nothing is read off the
//! `Diagnostic` itself, so a field here and not in the text, or the reverse,
//! would have to be a field of the record. Its text has had the record's
//! control-byte substitution, as the terminal's has.
//!
//! **A position is given twice**, as the byte `span` the compiler works in and
//! as one-based `line`/`column` pairs, because the two consumers want
//! different ones: an editor applying a suggestion wants the offsets, and a CI
//! log linking to a file wants what the `--> path:line:col` header shows. The
//! columns count `char`s rather than bytes, which is
//! [`nvs_diagnostics::SourceFile::line_col`]'s own decision and the one the
//! text header is already rendered from, so the two cannot disagree.
//!
//! **`help` is its own array** rather than notes a reader has to sort out. The
//! record keys each note `note` or `help`, and a consumer reading `help` gets
//! the text without the `help: ` prefix, since the prefix is the plaintext
//! rendering's and not the record's.
//!
//! A label in a file the source map does not hold is not in the record, so it
//! is in neither rendering. A diagnostic reported against a file the compiler
//! loaded has none.
//!
//! Every field is always present. An absent code is `null` and an empty list
//! is `[]`, so nothing downstream has to tell "no suggestions" from "this
//! version did not emit suggestions". `todos` is the one list a flag fills:
//! it is `--todos`' list, and `[]` without it.
//!
//! The rest of the module is `rule:tooling/a-todo-is-a-comment-the-tools-list`'s
//! half of `nvs check`: the todo list ([`todos`]), what `--deny` reads after a
//! run ([`Found`]), and the edits `--fix` writes ([`safe_edits`], [`apply`]).
//! A file in a directory named `vendor` is listed but never denied or written
//! ([`vendored`]).

use nvs_diagnostics::{Diagnostic, SourceFile, SourceMap, Suggestion, record};
use nvs_render::{Node, Record, Rendered, Scalar};
use serde_json::{Map, Value, json};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::config;

/// Parse, resolve and type-check a Novis file, reporting every diagnostic
/// found.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// The file or directory to check.
    ///
    /// A directory is checked as one program, the program `nvs test` runs for it.
    path: PathBuf,
    /// Print the diagnostics as the frozen JSON document rather than
    /// rendering them for a terminal.
    // Refused beside `--autoload-map` rather than one of them winning:
    // both write to standard output, and a document with a map printed
    // after it is not a document.
    #[arg(long, conflicts_with = "autoload_map")]
    json: bool,
    /// Print the resolved `autoload` map instead of `no errors`,
    /// including what a `discover` glob skipped and what was shadowed.
    #[arg(long)]
    autoload_map: bool,
    /// Also report every public member with no `///` doc comment above it.
    /// Off by default, in every project.
    #[arg(long)]
    strict_docs: bool,
    /// Print one line on stderr with how much work the compiler did:
    /// tokens, syntax nodes, names, typed expressions.
    ///
    /// The numbers are the same on every machine for the same program.
    /// Compare them for a small and a large program to see how compile
    /// time grows.
    #[arg(long)]
    count: bool,
    /// Also print every `// TODO:` comment in the program, one per line,
    /// as `path:line: text`. With `--json`, the document has them in its
    /// `todos` array.
    #[arg(long)]
    todos: bool,
    /// Exit with an error when the program has this kind of finding in a
    /// file that is not in a `vendor` directory. The kinds are
    /// `deprecated` and `todo`. You can give this option more than once.
    #[arg(long, value_enum)]
    deny: Vec<Deny>,
    /// Apply every safe fix the diagnostics suggest, then check the
    /// program again, until no fix is left. Files in a `vendor` directory
    /// are not changed.
    #[arg(long, conflicts_with = "json")]
    fix: bool,
    /// The folder to search for the program that gives this file its
    /// `autoload` map. Use it when you check a class file from outside
    /// the project folder.
    ///
    /// The default is the folder of the first `--config` file, or else
    /// the current folder.
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,
}

impl Args {
    /// Runs `nvs check` with the arguments clap parsed, the global `--config`
    /// files and the project's init gate.
    pub(crate) fn run(self, config: &[PathBuf], init: config::Init) -> ExitCode {
        crate::run_check(
            config,
            &self.path,
            &crate::CheckFlags {
                json: self.json,
                autoload_map: self.autoload_map,
                strict_docs: self.strict_docs,
                count: self.count,
                todos: self.todos,
                deny: self.deny,
                fix: self.fix,
                root: self.root,
            },
            init,
        )
    }
}

/// The closed set of kinds `nvs check --deny` takes
/// (`rule:tooling/a-todo-is-a-comment-the-tools-list`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
pub(crate) enum Deny {
    /// A use of deprecated code: a `W1003` warning.
    Deprecated,
    /// A `// TODO:` comment.
    Todo,
}

/// The shape below, frozen. It goes up when a field is removed or its meaning
/// changes; a field added beside the others does not move it, because a
/// consumer reading by name is unaffected by one.
const SCHEMA_VERSION: u64 = 1;

/// Every diagnostic as one document, pretty-printed.
///
/// Pretty rather than compact — the opposite of `nvs ast --json`, and for the
/// opposite reason: an AST document is thousands of nodes nobody reads by eye,
/// while this one is the handful of things wrong with a file and is read in a
/// CI log as often as it is parsed.
///
/// `todos` is `--todos`' list, and empty without the flag, so the key is
/// present in every document like every other field.
pub(crate) fn document<'a>(
    diagnostics: impl Iterator<Item = &'a Diagnostic>,
    map: &SourceMap,
    todos: &[Listed],
) -> String {
    let document = json!({
        "schemaVersion": SCHEMA_VERSION,
        "diagnostics": diagnostics
            .map(|diagnostic| diagnostic_json(&record::record(diagnostic, map)))
            .collect::<Vec<_>>(),
        "todos": todos
            .iter()
            .map(|todo| json!({ "file": todo.file, "line": todo.line, "text": todo.text }))
            .collect::<Vec<_>>(),
    });
    // It cannot fail: the document holds strings, bools and integers, and
    // `serde_json` only errors on a non-string map key or a non-finite float.
    serde_json::to_string_pretty(&document).expect("the document holds no unserializable value")
}

/// One `// TODO:` comment of the checked program
/// (`rule:tooling/a-todo-is-a-comment-the-tools-list`), as `--todos` prints it.
#[derive(Debug)]
pub(crate) struct Listed {
    /// The file's name as a diagnostic's `-->` header shows it.
    pub(crate) file: String,
    /// One-based.
    pub(crate) line: usize,
    pub(crate) text: String,
    /// Whether the file is under a `vendor` directory, which `--deny todo`
    /// does not look at.
    pub(crate) vendored: bool,
}

impl Listed {
    /// `path:line: text`, the line `--todos` prints.
    pub(crate) fn line(&self) -> String {
        format!("{}:{}: {}", self.file, self.line, self.text)
    }
}

/// Every todo in every file `map` holds, in path and line order.
///
/// Each file is lexed once more for its trivia, which only a caller that
/// lists todos pays for.
pub(crate) fn todos(map: &SourceMap) -> Vec<Listed> {
    let mut out: Vec<Listed> = map
        .files()
        .flat_map(|file| {
            let vendored = file.path().is_some_and(vendored);
            nvs_syntax::todos(file).into_iter().map(move |todo| Listed {
                file: file.name().to_string(),
                line: file.line_col(todo.span.start).0 + 1,
                text: todo.text,
                vendored,
            })
        })
        .collect();
    // Stable, so two todos on one line keep their source order.
    out.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    out
}

/// Whether `path` has a directory named `vendor` in it: code a project pulls
/// in rather than writes, which `--deny` and `--fix` leave alone.
pub(crate) fn vendored(path: &std::path::Path) -> bool {
    path.parent()
        .is_some_and(|dir| dir.components().any(|part| part.as_os_str() == "vendor"))
}

/// What one front-end run of `nvs check` found that the command reads once the
/// run is over: `--deny`'s two questions and `--fix`'s edits.
///
/// It is kept beside the run rather than returned from it because a run that
/// fails returns no program, and a fix is wanted most in a program that fails.
#[derive(Debug, Default)]
pub(crate) struct Found {
    /// Whether a `W1003` was reported in a file outside `vendor`.
    pub(crate) deprecated: bool,
    /// Whether a todo was read in a file outside `vendor`.
    pub(crate) todo: bool,
    /// The safe edits `--fix` applies, per file, none overlapping another.
    pub(crate) edits: Vec<FileEdits>,
}

/// The edits one `--fix` pass makes to one file.
#[derive(Debug)]
pub(crate) struct FileEdits {
    pub(crate) path: PathBuf,
    /// Byte range and replacement, in the order they were taken.
    pub(crate) edits: Vec<(u32, u32, String)>,
}

thread_local! {
    /// The last run's [`Found`]. One `nvs check` runs one front end at a time,
    /// on the main thread.
    static FOUND: RefCell<Found> = RefCell::new(Found::default());
}

/// Replaces what the last run found.
pub(crate) fn keep(found: Found) {
    FOUND.with(|slot| *slot.borrow_mut() = found);
}

/// What the last run found, leaving nothing behind.
pub(crate) fn take() -> Found {
    FOUND.with(|slot| std::mem::take(&mut *slot.borrow_mut()))
}

/// Whether a `W1003` in `diagnostics` points into a file outside `vendor`.
pub(crate) fn uses_deprecated<'a>(
    mut diagnostics: impl Iterator<Item = &'a Diagnostic>,
    map: &SourceMap,
) -> bool {
    diagnostics.any(|diagnostic| {
        diagnostic.code == Some(nvs_diagnostics::code::W_DEPRECATED)
            && diagnostic
                .primary_span()
                .and_then(|span| map.get(span.file))
                .is_some_and(|file| !file.path().is_some_and(vendored))
    })
}

/// Every `safe` suggestion in `diagnostics` that `--fix` applies, per file.
///
/// A diagnostic's suggestions are one fix, such as a rewrite and the `use` line
/// it needs, so they are taken together or not at all. A fix that overlaps
/// one taken before it waits for the next pass, which checks the program
/// again. An edit equal to one already taken is the same change made once:
/// two uses of one deprecated member each carry the same `use` line. A choice
/// between alternatives is never taken, and nothing is taken in a file with no
/// path or in a `vendor` directory.
pub(crate) fn safe_edits<'a>(
    diagnostics: impl Iterator<Item = &'a Diagnostic>,
    map: &SourceMap,
) -> Vec<FileEdits> {
    let mut out: Vec<FileEdits> = Vec::new();
    for diagnostic in diagnostics {
        let fix: Vec<&Suggestion> = diagnostic
            .suggestions
            .iter()
            .filter(|suggestion| suggestion.safe && !suggestion.alternative)
            .collect();
        let writable = |suggestion: &&Suggestion| {
            map.get(suggestion.span.file)
                .and_then(SourceFile::path)
                .is_some_and(|path| !vendored(path))
        };
        if fix.is_empty() || !fix.iter().all(writable) {
            continue;
        }
        let overlaps = fix.iter().any(|suggestion| {
            let (start, end) = (suggestion.span.start, suggestion.span.end);
            let path = map.get(suggestion.span.file).and_then(SourceFile::path);
            out.iter()
                .filter(|file| Some(file.path.as_path()) == path)
                .flat_map(|file| &file.edits)
                .any(|(s, e, text)| {
                    let same = *s == start && *e == end && *text == suggestion.replacement;
                    // Two ranges that share a byte, or two edits at one
                    // offset where either inserts: the order would decide
                    // the text.
                    let shared = start < *e && *s < end;
                    let one_point = start == *s && (start == end || s == e);
                    (shared || one_point) && !same
                })
        });
        if overlaps {
            continue;
        }
        for suggestion in fix {
            let path = map
                .get(suggestion.span.file)
                .and_then(SourceFile::path)
                .expect("checked writable above");
            let edit = (
                suggestion.span.start,
                suggestion.span.end,
                suggestion.replacement.clone(),
            );
            match out.iter_mut().find(|file| file.path == path) {
                Some(file) if file.edits.contains(&edit) => {}
                Some(file) => file.edits.push(edit),
                None => out.push(FileEdits {
                    path: path.to_path_buf(),
                    edits: vec![edit],
                }),
            }
        }
    }
    out
}

/// Writes `file`'s edits into it, and returns how many it made.
///
/// The text is read again rather than taken from the run, so the offsets are
/// applied to the bytes they were computed from only if nothing changed the
/// file in between: a file whose length no longer covers an edit is left as
/// it is.
pub(crate) fn apply(file: &FileEdits) -> std::io::Result<usize> {
    let mut text = std::fs::read_to_string(&file.path)?;
    let mut edits: Vec<&(u32, u32, String)> = file.edits.iter().collect();
    // From the end, so an edit never moves the offsets of one still to come.
    edits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    let fits = edits.iter().all(|(start, end, _)| {
        let (start, end) = (*start as usize, *end as usize);
        start <= end
            && end <= text.len()
            && text.is_char_boundary(start)
            && text.is_char_boundary(end)
    });
    if !fits {
        return Ok(0);
    }
    for (start, end, replacement) in &edits {
        text.replace_range(*start as usize..*end as usize, replacement);
    }
    std::fs::write(&file.path, text)?;
    Ok(edits.len())
}

/// One diagnostic's record, as the block the terminal renderer prints from
/// the same record.
fn diagnostic_json(record: &Record) -> Value {
    let notes = match record::field(record, record::NOTES) {
        Some(Node::Sequence(notes)) => notes.as_slice(),
        _ => &[],
    };
    let note_texts = |kind: &str| {
        notes
            .iter()
            .filter_map(|note| match note {
                Node::Map(entries) => entries
                    .iter()
                    .find(|(key, _)| key.as_str() == kind)
                    .and_then(|(_, text)| scalar_text(text)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let suggestions = match record::field(record, record::SUGGESTIONS) {
        Some(Node::Sequence(suggestions)) => suggestions.as_slice(),
        _ => &[],
    };
    json!({
        // The same spelling the text head carries, so `severity` and
        // `error[E0401]` name one thing.
        "severity": record::field_text(record, record::SEVERITY),
        "code": record::field_text(record, record::CODE),
        "message": record.envelope.message.as_ref().map(Rendered::as_str),
        "labels": record.nodes.iter().filter_map(label_json).collect::<Vec<_>>(),
        "notes": note_texts("note"),
        "help": note_texts("help"),
        "suggestions": suggestions.iter().filter_map(suggestion_json).collect::<Vec<_>>(),
    })
}

/// One span with something to say about it, or `None` when `node` is not a span.
fn label_json(node: &Node) -> Option<Value> {
    let Node::Span { primary, label, .. } = node else {
        return None;
    };
    let mut out = location_json(Some(node));
    out.insert(
        "style".into(),
        Value::from(if *primary { "primary" } else { "secondary" }),
    );
    out.insert("message".into(), Value::from(label.as_str()));
    Some(Value::Object(out))
}

/// One machine-applicable edit, with the text to put there, or `None` when
/// `node` is not a suggestion's map.
fn suggestion_json(node: &Node) -> Option<Value> {
    let Node::Map(entries) = node else {
        return None;
    };
    let entry = |name: &str| {
        entries
            .iter()
            .find(|(key, _)| key.as_str() == name)
            .map(|(_, value)| value)
    };
    let flag = |name: &str| matches!(entry(name), Some(Node::Scalar(Scalar::Bool(true))));
    let mut out = location_json(entry("at"));
    out.insert(
        "replacement".into(),
        Value::from(entry("replacement").and_then(scalar_text)),
    );
    out.insert(
        "message".into(),
        Value::from(entry("message").and_then(scalar_text)),
    );
    out.insert("safe".into(), Value::from(flag("safe")));
    out.insert("alternative".into(), Value::from(flag("alternative")));
    Some(Value::Object(out))
}

/// Where a span is, in both of the two ways a consumer asks.
///
/// `file`, `span` and the two positions are `null` together when the record
/// has no span: a suggestion in a file the map does not hold, which a
/// diagnostic reported against a file the compiler loaded cannot be. A label
/// in such a file is not in the record, and the text leaves it out too.
fn location_json(span: Option<&Node>) -> Map<String, Value> {
    let mut out = Map::new();
    let Some(Node::Span {
        file,
        line,
        column,
        end_line,
        end_column,
        offset,
        end_offset,
        ..
    }) = span
    else {
        for key in ["file", "span", "start", "end"] {
            out.insert(key.into(), Value::Null);
        }
        return out;
    };
    out.insert("file".into(), Value::from(file.as_str()));
    out.insert("span".into(), json!([offset, end_offset]));
    out.insert("start".into(), json!({ "line": line, "column": column }));
    out.insert(
        "end".into(),
        json!({ "line": end_line, "column": end_column }),
    );
    out
}

/// The text of a string scalar, or `None` for any other node.
fn scalar_text(node: &Node) -> Option<&str> {
    match node {
        Node::Scalar(Scalar::Str { text, .. }) => Some(text.as_str()),
        _ => None,
    }
}
