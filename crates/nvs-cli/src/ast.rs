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
use nvs_syntax::ast::Stmt;
use nvs_syntax::{check_declarations, parse_file, walk};
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
            eprintln!("error: could not read {}: {err}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let mut diags = Diagnostics::new();
    let file = map.file(id);
    let stmts = parse_file(file, &mut diags);
    check_declarations(&stmts, file, &mut diags);
    let refused = diags.has_errors();

    if !(strict && refused) {
        if as_json {
            println!("{}", document(&stmts, file));
        } else {
            println!("{stmts:#?}");
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
fn document(stmts: &[Stmt], file: &SourceFile) -> Value {
    json!({
        "kind": "File",
        "span": [0, file.text().len()],
        "children": walk::of_stmts(stmts).iter().map(node_json).collect::<Vec<_>>(),
    })
}

/// One node of the tree.
fn node_json(node: &walk::Node) -> Value {
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
        Value::Array(node.children.iter().map(node_json).collect()),
    );
    Value::Object(out)
}
