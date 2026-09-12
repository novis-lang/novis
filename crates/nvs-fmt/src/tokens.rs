//! The rules that rewrite a token rather than the whitespace around it.
//!
//! Every other stage here decides what a whitespace run has to be. These decide
//! what a *code* byte has to be, which is the smaller and the more dangerous
//! half of the formatter: a whitespace run carries no meaning, and a token
//! carries all of it. So each rule states the bytes it is allowed to touch, and
//! a literal it cannot prove is spelled differently without also *meaning*
//! differently is left exactly as its author wrote it.
//!
//! Both rules read the runs [`crate::print`] tiles the file into — the bytes
//! between two trivia, which are tokens and nothing else, so a comment is never
//! one of them. The walk carries the last code byte it has passed across the
//! gaps between those runs, because that byte is the one thing the comma rule
//! asks about and the trivia in front of it are what the question is.
//!
//! # The quote rule
//!
//! A double-quoted literal goes out single-quoted when the two spellings name
//! the same string (`rule:tooling/fmt-quotes`), and stays as it is otherwise:
//!
//! - **It interpolates.** The parse says so and nothing here has to look: an
//!   interpolating literal is [`Interpolated`](nvs_syntax::ast::ExprKind), a
//!   node of its own, and only a [`Str`](nvs_syntax::ast::ExprKind) is a
//!   candidate at all.
//! - **It holds a `'`**, which single-quoting would force into an escape —
//!   trading one escape for another is not a canonical spelling.
//! - **It holds a `\`.** The two quotings do not share an escape grammar:
//!   `\n` is a newline inside double quotes and two characters inside single
//!   ones ([`nvs_types::string_lit`]), so rewriting one is a change to the
//!   string's value, which is the one thing a formatter never does. The rule's
//!   own fragment records this as the reading of its "would force to be
//!   escaped" clause.
//!
//! A heredoc, a nowdoc and a markup literal are never candidates: a heredoc's
//! span opens on `<<<` rather than on a quote, and the other two are their own
//! nodes.
//!
//! Which literal a byte belongs to is
//! [`SyntaxIndex::at`](nvs_syntax::SyntaxIndex::at)'s to answer, the same way
//! [`crate::brace`] asks which body a brace opens. A `"` is an opening quote
//! only when the innermost node containing it is a `Str` that *starts* there,
//! so a quote written inside a heredoc's body or inside an interpolation site's
//! own expression is never mistaken for one.
//!
//! # The trailing comma
//!
//! A comma-separated list whose closing delimiter opens a line of its own ends
//! with a comma, and a list whose closer follows the last element on that
//! element's own line ends without one (`rule:tooling/fmt-trailing-commas`).
//! Where the closer sits is the author's, like every other line break in an
//! expression (`rule:tooling/fmt-never-reflows`), and the comma follows from it
//! mechanically in both directions: a missing one is inserted and one written
//! where the closer trails its element is deleted, so the same list has one
//! spelling either way round.
//!
//! [`LISTS`] is what makes a closer a list's rather than a block's: the
//! productions whose own last byte is a closing delimiter, so a `)` that ends a
//! `Call` is one and a `]` that ends an `Index` is not. The index carries no
//! span for an element, and it does not need to — the last element ends at the
//! last code byte before the closer, which is the byte the walk has in hand.
//! A parameter list, an enum-case list and a shape type's fields are written
//! inside a node that ends somewhere else entirely, and are a known gap in
//! [`the crate's own doc`](crate).

use nvs_diagnostics::BytePos;
use nvs_syntax::{SyntaxIndex, Trivia};

use crate::print::Rewrite;

/// The quote a rewritten literal is delimited by.
const SINGLE: &str = "'";

/// What a multi-line list ends its last element with.
const COMMA: &str = ",";

/// What a deleted byte goes out as.
const NOTHING: &str = "";

/// The productions whose own last byte closes a comma-separated list.
///
/// Each of these ends at the delimiter that closes the list it wrote, so the
/// node the index answers at a closer *is* the list. What that keeps out is
/// everything else a `)`, `]` or `}` ends: a parenthesised expression, an
/// index, a block, a class body and a closure separate nothing with commas, and
/// none of them is here.
const LISTS: &[&str] = &[
    "ArrayLiteral",
    "Call",
    "Match",
    "MethodCall",
    "New",
    "ObjectLiteral",
    "StaticCall",
];

/// Every edit the token rules make to `text`, which must be `index`'s and
/// `trivia`'s own file, in source order.
pub(crate) fn rewrites<'t>(
    index: &SyntaxIndex,
    text: &'t str,
    trivia: &[Trivia],
) -> Vec<Rewrite<'t>> {
    let mut out = Vec::new();
    let mut cursor = 0_usize;
    let mut previous = None;
    for trivium in trivia {
        scan(
            &mut out,
            &mut previous,
            index,
            text,
            cursor,
            trivium.span.start as usize,
        );
        cursor = trivium.span.end as usize;
    }
    scan(&mut out, &mut previous, index, text, cursor, text.len());
    out
}

/// Reads `text[from..to]`, which is code and nothing else, records every edit
/// the token rules make inside it, and leaves `previous` on its last byte.
fn scan<'t>(
    out: &mut Vec<Rewrite<'t>>,
    previous: &mut Option<usize>,
    index: &SyntaxIndex,
    text: &'t str,
    from: usize,
    to: usize,
) {
    for (offset, byte) in text.as_bytes().iter().enumerate().take(to).skip(from) {
        match *byte {
            b'"' => quote(out, index, text, offset),
            b')' | b']' | b'}' => comma(out, index, text, *previous, offset),
            _ => {}
        }
        *previous = Some(offset);
    }
}

/// Records the delimiters of the literal beginning at `offset`, where one
/// begins there and the quote rule respells it.
///
/// The edit is the two delimiters and never the body: a literal that qualifies
/// has a body meaning the same in both quotings, so copying it is what keeps
/// the rewrite value-preserving by construction rather than by an unescaping
/// routine this file would then own.
fn quote<'t>(out: &mut Vec<Rewrite<'t>>, index: &SyntaxIndex, text: &'t str, offset: usize) {
    let Ok(pos) = BytePos::try_from(offset) else {
        return;
    };
    let Some(node) = index.at(pos).innermost() else {
        return;
    };
    if node.kind != "Str" || node.span.start as usize != offset {
        return;
    }
    let end = node.span.end as usize;
    if text.as_bytes().get(end - 1) != Some(&b'"') {
        return;
    }
    let Some(body) = text.get(offset + 1..end - 1) else {
        return;
    };
    if body.bytes().any(|byte| byte == b'\'' || byte == b'\\') {
        return;
    }
    out.push(Rewrite {
        start: offset,
        end: offset + 1,
        written: SINGLE,
    });
    out.push(Rewrite {
        start: end - 1,
        end,
        written: SINGLE,
    });
}

/// Records the comma the list closed at `offset` owes, or the one it owes the
/// deletion of, where `previous` is the last code byte in front of the closer.
///
/// That byte is the last element's own last byte, because everything between an
/// element and its list's closer is trivia. A closer with no code byte at all
/// in front of it is inside no list, and one whose element is the opening
/// delimiter closes an empty list, which has no last element to put a comma
/// after.
fn comma<'t>(
    out: &mut Vec<Rewrite<'t>>,
    index: &SyntaxIndex,
    text: &'t str,
    previous: Option<usize>,
    offset: usize,
) {
    let Some(last) = previous else {
        return;
    };
    let Ok(pos) = BytePos::try_from(offset) else {
        return;
    };
    let Some(node) = index.at(pos).innermost() else {
        return;
    };
    if node.span.end as usize != offset + 1 || !LISTS.contains(&node.kind) {
        return;
    }
    let bytes = text.as_bytes();
    if matches!(bytes[last], b'(' | b'[' | b'{') {
        return;
    }
    let Some(between) = text.get(last + 1..offset) else {
        return;
    };
    match (bytes[last] == b',', between.contains('\n')) {
        (false, true) => out.push(Rewrite {
            start: last + 1,
            end: last + 1,
            written: COMMA,
        }),
        (true, false) => out.push(Rewrite {
            start: last,
            end: last + 1,
            written: NOTHING,
        }),
        _ => {}
    }
}
