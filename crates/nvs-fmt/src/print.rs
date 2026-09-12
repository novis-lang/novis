//! The printer: a parsed file written back out, one run at a time.
//!
//! A file is a tiling of two kinds of run, which is
//! `rule:ide/tokens-plus-trivia-reproduce-the-file` read as a program. One kind
//! is the trivia [`nvs_syntax::parse`] collected — every comment and every
//! whitespace run, in source order. The other is what lies between two
//! consecutive trivia, which is tokens and nothing else, since a trivium covers
//! every byte the grammar skips. Walking the two alternately is the whole file,
//! and that walk is the frame each layout rule lands in: a rule about
//! indentation or spacing rewrites a whitespace run, and one about where a
//! brace sits rewrites the runs on either side of it.
//!
//! A code run is copied byte for byte, save for the one rewrite that only
//! reorders what is already in it: a modifier list goes out in its canonical
//! order ([`crate::modifiers`]). `rule:tooling/fmt-never-reflows` leaves what is
//! inside an expression to the author, and bytes a program prints — inline
//! HTML, a heredoc body, a markup literal's body — are never a formatter's to
//! touch, so a rule that appears to require rewriting one is being misread.

use std::iter::Peekable;
use std::vec::IntoIter;

use nvs_diagnostics::SourceFile;
use nvs_syntax::{Parsed, Trivia, TriviaKind};

use crate::indent::Indent;
use crate::modifiers::{self, Rewrite};

/// Writes `parsed` back out as the text of a formatted file.
///
/// `parsed` must be `file`'s own parse: the spans are offsets into its text.
pub(crate) fn print(file: &SourceFile, parsed: &Parsed) -> String {
    let text = file.text();
    let indent = Indent::new(&parsed.index, text);
    let mut out = String::with_capacity(text.len());
    let mut moved = modifiers::rewrites(parsed, text).into_iter().peekable();
    let mut cursor = 0_usize;
    for trivium in &parsed.trivia {
        let start = trivium.span.start as usize;
        let end = trivium.span.end as usize;
        debug_assert!(
            cursor <= start && start <= end,
            "trivia arrive in source order and no two of them overlap"
        );
        push_code(&mut out, text, cursor, start, &mut moved);
        push_trivium(&mut out, &indent, text, *trivium);
        cursor = end;
    }
    push_code(&mut out, text, cursor, text.len(), &mut moved);
    out
}

/// Writes the code run `text[from..to]`, each modifier in it going out where
/// the canonical order puts it.
///
/// `moved` is every keyword the file writes somewhere other than where it was
/// written, in source order; each one lies inside a single run, so what this
/// takes from the front is what belongs to this run and the rest waits for a
/// later one.
fn push_code<'t>(
    out: &mut String,
    text: &'t str,
    from: usize,
    to: usize,
    moved: &mut Peekable<IntoIter<Rewrite<'t>>>,
) {
    let mut cursor = from;
    while let Some(rewrite) = moved.next_if(|rewrite| rewrite.end <= to) {
        debug_assert!(
            cursor <= rewrite.start,
            "a rewritten keyword lies inside one code run, and they arrive in source order"
        );
        out.push_str(&text[cursor..rewrite.start]);
        out.push_str(rewrite.written);
        cursor = rewrite.end;
    }
    out.push_str(&text[cursor..to]);
}

/// Writes one trivium, re-indenting the line it leaves the printer on.
///
/// A whitespace run that carries a line break ends by opening a line, and what
/// opens that line is the one thing in the run this stage decides: everything
/// up to and including the last break is the author's — blank lines and all —
/// and what follows it is four spaces per enclosing body
/// (`rule:tooling/fmt-base-style-is-per`). A run the tree does not place, and
/// every comment, go out exactly as they came in.
fn push_trivium(out: &mut String, indent: &Indent<'_>, text: &str, trivium: Trivia) {
    let start = trivium.span.start as usize;
    let end = trivium.span.end as usize;
    let run = &text[start..end];
    let opens_a_line = trivium.kind == TriviaKind::Whitespace;
    let placed = opens_a_line
        .then(|| run.rfind('\n'))
        .flatten()
        .zip(indent.of_line(end));
    match placed {
        Some((last_break, opening)) => {
            out.push_str(&run[..=last_break]);
            out.push_str(&opening);
        }
        None => out.push_str(run),
    }
}
