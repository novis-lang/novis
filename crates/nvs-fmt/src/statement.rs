//! One statement per line, and a closing brace on a line of its own.
//!
//! `rule:tooling/fmt-base-style-is-per` puts each statement and each member on
//! a line of its own, and starts the closing brace of a body on its own line.
//! Both are a line break the author left out after the code that ended the
//! previous statement, so what this answers is the question [`crate::brace`]
//! answers — "what does the run before this byte have to be" — for the first
//! byte of a statement and for a body's `}`. Where the line goes is
//! [`Indent::of_line`]'s answer for that byte, the same one a line the author
//! broke already gets, so a split line and a written one cannot disagree.
//!
//! A statement opens a line here when the code before it on its line is the end
//! of the statement before it — a `;` or a `}` — or the `{` of the body that
//! holds it. A `,` is never that: two items of a list, two cases of an enum
//! and two variables of one property declaration are one statement. A closing
//! brace moves when a `;` or a `}` is in front of it on its line, so an empty
//! body `{}` stays as it is written.
//!
//! An expression's interior is the author's (`rule:tooling/fmt-never-reflows`),
//! and three bodies are written inside one: an anonymous function, an anonymous
//! class and a property hook. Written on one line, each of them keeps its line,
//! with everything inside it — the same half of the rule [`crate::brace`]
//! keeps for an anonymous function's braces. A line [`Indent::of_line`] does not
//! place, inside a `switch` or inline HTML, is the author's too, and nothing
//! here moves a byte of one.
//!
//! # What it spends
//!
//! One [`NodePath`](nvs_syntax::NodePath) per `;`, `{` and `}` of code that has
//! more code after it on its line, built and dropped inside the call, and one
//! offset per line this rule starts, kept for the file. Nothing here runs on
//! the request path.

use nvs_diagnostics::BytePos;
use nvs_syntax::{IndexNode, SyntaxIndex, Trivia};

use crate::indent::{BODIES, Indent, OPAQUE};
use crate::space::code_after;

/// The bodies written inside an expression, spelled as
/// `crates/nvs-syntax/src/walk.rs` spells them: an anonymous function, the
/// `new` of an anonymous class and a property with hooks.
const EXPRESSION_BODIES: &[&str] = &["Fn", "New", "Property"];

/// Every offset of `text` this rule starts a new line at, in source order.
/// `index` and `trivia` must be `text`'s own parse.
///
/// These offsets do not depend on any line's indentation, so [`Indent`] reads
/// them first. A body's contents are indented from the line its opener starts,
/// and an opener this rule moves starts a line the author did not write.
pub(crate) fn breaks(index: &SyntaxIndex, text: &str, trivia: &[Trivia]) -> Vec<usize> {
    let mut found = Vec::new();
    let mut cursor = 0_usize;
    for trivium in trivia {
        scan(
            &mut found,
            (index, text, trivia),
            cursor,
            trivium.span.start as usize,
        );
        cursor = trivium.span.end as usize;
    }
    scan(&mut found, (index, text, trivia), cursor, text.len());
    found
}

/// Everything this rule requires of `indent`'s file: the run that has to
/// precede an offset, by that offset.
pub(crate) fn runs(indent: &Indent<'_>) -> Vec<(usize, String)> {
    let (_, text, _) = indent.parse();
    let line_break = if text.contains("\r\n") { "\r\n" } else { "\n" };
    indent
        .breaks()
        .iter()
        .filter_map(|&at| {
            let opening = indent.of_line(at)?;
            Some((at, format!("{line_break}{opening}")))
        })
        .collect()
}

/// Reads `text[from..to]`, which is code and nothing else, and records each
/// statement and each closing brace that shares a line with the `;`, `{` or
/// `}` in front of it.
fn scan(
    found: &mut Vec<usize>,
    (index, text, trivia): (&SyntaxIndex, &str, &[Trivia]),
    from: usize,
    to: usize,
) {
    let bytes = text.as_bytes();
    for offset in from..to {
        let ended = bytes[offset];
        if !matches!(ended, b';' | b'{' | b'}') {
            continue;
        }
        let next = code_after(trivia, offset + 1);
        if next >= bytes.len() || text[offset + 1..next].contains('\n') {
            continue;
        }
        let Ok(pos) = BytePos::try_from(next) else {
            continue;
        };
        let path = index.at(pos);
        let nodes = path.nodes();
        let moves = if bytes[next] == b'}' && ended != b'{' {
            closes_a_body(nodes, next)
        } else {
            opens_a_statement(nodes, next)
        };
        let kept = nodes.iter().any(|node| {
            OPAQUE.contains(&node.kind)
                || node.span.start as usize != next && on_one_line(text, *node)
        });
        if moves && !kept {
            found.push(next);
        }
    }
}

/// Whether the `}` at `offset` closes a body, which `nodes`, the path at that
/// offset, contains.
///
/// A `try` writes the braces of each of its clauses itself, so a `}` whose
/// innermost node is the `try` closes one of them.
fn closes_a_body(nodes: &[IndexNode], offset: usize) -> bool {
    nodes.first().is_some_and(|node| node.kind == "Try")
        || nodes
            .iter()
            .any(|node| BODIES.contains(&node.kind) && node.span.end as usize == offset + 1)
}

/// Whether a statement of the file, or a statement or a member of a body,
/// starts at `offset`. `nodes` is the path at that offset.
///
/// Only trivia lies between the `;`, `{` or `}` in front of it and the child,
/// so that byte is the end of the child before it or the body's own `{`. A
/// parameter's default value and a `for` header's clauses are children too, and
/// a `=`, a `(` or a parent outside [`BODIES`] tells them apart.
fn opens_a_statement(nodes: &[IndexNode], offset: usize) -> bool {
    nodes
        .last()
        .is_some_and(|outermost| outermost.span.start as usize == offset)
        || nodes
            .windows(2)
            .any(|pair| pair[0].span.start as usize == offset && BODIES.contains(&pair[1].kind))
}

/// Whether `node` is a body written inside an expression and on one line.
fn on_one_line(text: &str, node: IndexNode) -> bool {
    EXPRESSION_BODIES.contains(&node.kind)
        && text
            .get(node.span.start as usize..node.span.end as usize)
            .is_some_and(|written| !written.contains('\n'))
}
