//! `nvs ast` — the parse tree, printed for a person or for a tool.
//!
//! Two renderings of one parse, and only one of them is a contract.
//! `{stmts:#?}` is Rust's derived `Debug`: any field reordering in
//! [`nvs_syntax::ast`] changes it, so nothing may be built on it, and it stays
//! the default because it is what a person reading a tree by hand wants.
//! `--json` is the shape `rule:ide/ast-json-schema-is-frozen` specifies — a
//! node is its `kind`, its `span` as `[start, end]`, its own scalar fields and
//! its `children` — and it is what the AST panel of
//! `rule:ide/the-ast-panel-shells-out-to-the-cli` renders. That panel shells
//! out to this command rather than reaching for `Core\Ast`, which is a running
//! program's reflective parse and not an editor's.
//!
//! The scalar fields are siblings of `kind` rather than an object under it,
//! and which scalars a production has is [`nvs_syntax::walk`]'s third
//! decision — including why a literal's text is not one of them, which is the
//! one thing a reader of this output has to know it will not find.
//!
//! **A trivium is a node like any other** — its kind, its span, no children —
//! sitting under the innermost node whose span contains it, so a comment
//! between two methods is a child of the class rather than of the file. All
//! four kinds are there, whitespace included: `rule:ide/one-grammar-one-tree`
//! closes that set at four, a consumer wanting only comments filters on
//! `kind`, and dropping a kind here would decide that for every consumer of a
//! schema that is frozen. What it spends is one node per whitespace run and
//! per comment, which is the larger half of the document.
//!
//! **Resilient is the default**, because the file a developer opens the panel
//! for is usually the one that does not compile: the tree is printed whatever
//! the parser reported, and `--strict` is the opt-in that prints nothing once
//! an error was reported. Either way the diagnostics go to standard error and
//! an error is a non-zero exit, so a pipeline reading standard output sees a
//! document and a shell sees a failure.
//!
//! The tree itself is [`nvs_syntax::walk`]'s and is never matched a second time
//! here. That module's own doc says why a production is described in one place:
//! [`nvs_syntax::ast::ExprKind`] is `#[non_exhaustive]`, so a renderer matching
//! it from this crate would need a wildcard arm and would walk a variant added
//! later as a leaf.

use std::path::Path;
use std::process::ExitCode;

use nvs_diagnostics::{Diagnostics, SourceFile, SourceMap};
use nvs_syntax::{Parsed, Trivia, TriviaKind, check_declarations, parse, walk};
use serde_json::{Map, Value, json};

/// Parses `path` and prints its tree, as JSON when `as_json` is set.
///
/// `strict` is the opt-in half of the default above: it refuses to print a
/// tree for source the parser reported an error in, which is what a pipeline
/// that only wants trees for source that compiles asks for. Without it the
/// tree is printed and the diagnostics are reported beside it.
pub(crate) fn run(path: &Path, as_json: bool, strict: bool) -> ExitCode {
    let mut map = SourceMap::new();
    let id = match map.load(path) {
        Ok(id) => id,
        Err(err) => {
            crate::report_unreadable(path, &err, crate::Sink::Text);
            return ExitCode::FAILURE;
        }
    };

    let mut diags = Diagnostics::new();
    let file = map.file(id);
    // The lossless entry point for both renderings rather than only for
    // `--json`: it is the same grammar and the same diagnostics, so a second
    // parse path here would be one more thing to keep in step for a command
    // that parses one file and exits.
    let parsed = parse(file, &mut diags);
    check_declarations(&parsed.stmts, file, &mut diags);
    let refused = diags.has_errors();

    if !(strict && refused) {
        if as_json {
            println!("{}", document(&parsed, file));
        } else {
            println!("{:#?}", parsed.stmts);
        }
    }

    // After the tree rather than before it: the two streams interleave at a
    // terminal and a tree is long, so a reader who ran this to see a
    // diagnostic gets it under the tree instead of scrolled off the top by it.
    crate::render_diagnostics(&mut diags, &map);

    if refused {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// The whole file as one node: the root the panel hangs its tree from.
///
/// Its span is the file rather than the reduction of its statements' spans,
/// because a file extends past its last statement whenever it ends in a
/// comment or a newline, and a root that stopped there would not contain the
/// trivia the panel is about to be given.
fn document(parsed: &Parsed, file: &SourceFile) -> Value {
    json!({
        "kind": "File",
        "span": [0, file.text().len()],
        "children": children_json(&walk::of_stmts(&parsed.stmts), &parsed.trivia),
    })
}

/// One parent's children: its nodes and the trivia between them, in offset
/// order.
///
/// `trivia` is the run the parent's own span contains, and what is left after
/// each child has taken the trivia inside *its* span belongs to the parent —
/// which is the innermost-node placement the module doc promises, arrived at
/// without asking any node whether it contains an offset. Both sequences are
/// already sorted by start, so this is a merge rather than a search.
fn children_json(nodes: &[walk::Node], trivia: &[Trivia]) -> Vec<Value> {
    let mut out = Vec::new();
    let mut rest = trivia;
    for node in nodes {
        let (before, from) =
            rest.split_at(rest.partition_point(|t| t.span.start < node.span.start));
        out.extend(before.iter().map(trivia_json));
        let (inside, after) = from.split_at(from.partition_point(|t| t.span.start < node.span.end));
        out.push(node_json(node, inside));
        rest = after;
    }
    out.extend(rest.iter().map(trivia_json));
    out
}

/// One trivium as a node: its kind's own spelling, its span, no children.
fn trivia_json(trivium: &Trivia) -> Value {
    let kind = match trivium.kind {
        TriviaKind::Whitespace => "Whitespace",
        TriviaKind::LineComment => "LineComment",
        TriviaKind::BlockComment => "BlockComment",
        TriviaKind::DocComment => "DocComment",
    };
    json!({
        "kind": kind,
        "span": [trivium.span.start, trivium.span.end],
        "children": [],
    })
}

/// One node of the tree, with the trivia its span contains distributed
/// through it.
fn node_json(node: &walk::Node, trivia: &[Trivia]) -> Value {
    let mut out = Map::new();
    out.insert("kind".into(), Value::from(node.kind));
    out.insert("span".into(), json!([node.span.start, node.span.end]));
    for (name, value) in &node.fields {
        let value = match value {
            walk::Field::Word(word) => Value::from(*word),
            walk::Field::Flag(flag) => Value::from(*flag),
        };
        out.insert((*name).into(), value);
    }
    out.insert(
        "children".into(),
        Value::Array(children_json(&node.children, trivia)),
    );
    Value::Object(out)
}
