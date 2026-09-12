//! The rules that rewrite a token rather than the whitespace around it.
//!
//! Every other stage here decides what a whitespace run has to be. These decide
//! what a *code* byte has to be, which is the smaller and the more dangerous
//! half of the formatter: a whitespace run carries no meaning, and a token
//! carries all of it. So each rule states the bytes it is allowed to touch, and
//! a literal it cannot prove is spelled differently without also *meaning*
//! differently is left exactly as its author wrote it.
//!
//! What lands here is the quote rule (`rule:tooling/fmt-quotes`). A
//! double-quoted literal goes out single-quoted when the two spellings name the
//! same string, and stays as it is otherwise:
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
//! nodes. A comment is not code, so the walk this shares with [`crate::brace`]
//! — the runs between two trivia — never reaches one.
//!
//! Which literal a byte belongs to is
//! [`SyntaxIndex::at`](nvs_syntax::SyntaxIndex::at)'s to answer, the same way
//! [`crate::brace`] asks which body a brace opens. A `"` is an opening quote
//! only when the innermost node containing it is a `Str` that *starts* there,
//! so a quote written inside a heredoc's body or inside an interpolation site's
//! own expression is never mistaken for one.

use nvs_diagnostics::BytePos;
use nvs_syntax::{SyntaxIndex, Trivia};

use crate::print::Rewrite;

/// The quote a rewritten literal is delimited by.
const SINGLE: &str = "'";

/// Every edit the token rules make to `text`, which must be `index`'s and
/// `trivia`'s own file, in source order.
pub(crate) fn rewrites<'t>(
    index: &SyntaxIndex,
    text: &'t str,
    trivia: &[Trivia],
) -> Vec<Rewrite<'t>> {
    let mut out = Vec::new();
    let mut cursor = 0_usize;
    for trivium in trivia {
        quotes(&mut out, index, text, cursor, trivium.span.start as usize);
        cursor = trivium.span.end as usize;
    }
    quotes(&mut out, index, text, cursor, text.len());
    out
}

/// Reads `text[from..to]`, which is code and nothing else, and records the
/// delimiters of each literal in it that the quote rule respells.
///
/// The edit is the two delimiters and never the body: a literal that qualifies
/// has a body meaning the same in both quotings, so copying it is what keeps
/// the rewrite value-preserving by construction rather than by an unescaping
/// routine this file would then own.
fn quotes<'t>(
    out: &mut Vec<Rewrite<'t>>,
    index: &SyntaxIndex,
    text: &'t str,
    from: usize,
    to: usize,
) {
    let bytes = text.as_bytes();
    for offset in from..to {
        if bytes[offset] != b'"' {
            continue;
        }
        let Ok(pos) = BytePos::try_from(offset) else {
            continue;
        };
        let Some(node) = index.at(pos).innermost() else {
            continue;
        };
        if node.kind != "Str" || node.span.start as usize != offset {
            continue;
        }
        let end = node.span.end as usize;
        if bytes.get(end - 1) != Some(&b'"') {
            continue;
        }
        let Some(body) = text.get(offset + 1..end - 1) else {
            continue;
        };
        if body.bytes().any(|byte| byte == b'\'' || byte == b'\\') {
            continue;
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
}
