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
//! A code run is copied byte for byte, save for the edits that write something
//! other than what is there, each of them a [`Rewrite`]: a modifier list goes
//! out in its canonical order ([`crate::modifiers`]), a run of imports in path
//! order ([`crate::imports`]), a brace the author left no room in front of gets
//! the run [`crate::brace`] requires, and a literal the quote rule respells
//! gets the delimiters — and a multi-line list the comma — that
//! [`crate::tokens`] decides.
//! `rule:tooling/fmt-never-reflows` leaves what is inside an expression to the
//! author, and bytes a program prints — inline HTML, a heredoc body, a markup
//! literal's body — are never a formatter's to touch, so a rule that appears to
//! require rewriting one is being misread.

use std::iter::Peekable;
use std::vec::IntoIter;

use nvs_diagnostics::{SourceFile, Span};
use nvs_syntax::{Parsed, Trivia, TriviaKind};

use crate::brace::{self, Placements};
use crate::indent::Indent;
use crate::{imports, modifiers, tokens};

/// One run of bytes, and what the printer writes in its place.
///
/// An empty range inserts: a rule that has to put a byte where its author wrote
/// none — the line break a declaration's brace needs in `class Queue{` — is
/// this same edit with `start` and `end` equal.
pub(crate) struct Rewrite<'t> {
    /// Where the run this replaces begins.
    pub(crate) start: usize,
    /// One past that run's last byte, and equal to `start` for an insertion.
    pub(crate) end: usize,
    /// What belongs there.
    pub(crate) written: &'t str,
}

/// Whether `span` covers one run of code with nothing skipped inside it.
///
/// A file is tiled into trivia and the code between two of them, so every
/// [`Rewrite`] lies inside one run: a rule that moves bytes from one place to
/// another asks this about the bytes it is about to move, because a span with a
/// comment or a line break inside it is two runs, and half of one is not
/// something to write at another's place.
pub(crate) fn one_code_run(trivia: &[Trivia], span: Span) -> bool {
    let after = trivia.partition_point(|trivium| trivium.span.start < span.start);
    !trivia
        .get(after)
        .is_some_and(|next| next.span.start < span.end)
}

/// Writes `parsed` back out as the text of a formatted file.
///
/// `parsed` must be `file`'s own parse: the spans are offsets into its text.
pub(crate) fn print(file: &SourceFile, parsed: &Parsed) -> String {
    let text = file.text();
    let indent = Indent::new(&parsed.index, text);
    let braces = brace::placements(&parsed.index, &indent, text, &parsed.trivia);
    let mut edits = modifiers::rewrites(parsed, text);
    edits.extend(braces.insertions());
    edits.extend(imports::rewrites(parsed, text));
    edits.extend(tokens::rewrites(&parsed.index, text, &parsed.trivia));
    edits.sort_by_key(|edit| edit.start);
    let mut out = String::with_capacity(text.len());
    let mut moved = edits.into_iter().peekable();
    let mut cursor = 0_usize;
    for trivium in &parsed.trivia {
        let start = trivium.span.start as usize;
        let end = trivium.span.end as usize;
        debug_assert!(
            cursor <= start && start <= end,
            "trivia arrive in source order and no two of them overlap"
        );
        push_code(&mut out, text, cursor, start, &mut moved);
        push_trivium(&mut out, &indent, &braces, text, *trivium);
        cursor = end;
    }
    push_code(&mut out, text, cursor, text.len(), &mut moved);
    out
}

/// Writes the code run `text[from..to]`, each edit inside it applied where it
/// falls.
///
/// `moved` is every range of the file that goes out as something other than
/// what is there, in source order and covering no byte twice; each one lies
/// inside a single run, so what this takes from the front is what belongs to
/// this run and the rest waits for a later one.
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
            "a rewritten range lies inside one code run, and they arrive in source order"
        );
        out.push_str(&text[cursor..rewrite.start]);
        out.push_str(rewrite.written);
        cursor = rewrite.end;
    }
    out.push_str(&text[cursor..to]);
}

/// Writes one trivium, re-indenting the line it leaves the printer on.
///
/// A whitespace run carrying a brace's own line break is that brace's, whole:
/// [`crate::brace`] decides both how many of them there are and what follows
/// the last one, because a declaration's opener and a control structure's are
/// the same run written two ways. Every other whitespace run that carries a
/// line break ends by opening a line, and what opens that line is the one thing
/// in the run this stage decides: everything up to and including the last break
/// is the author's — blank lines and all — and what follows it is four spaces
/// per enclosing body (`rule:tooling/fmt-base-style-is-per`). A run the tree
/// does not place, and every comment, go out exactly as they came in.
fn push_trivium(
    out: &mut String,
    indent: &Indent<'_>,
    braces: &Placements,
    text: &str,
    trivium: Trivia,
) {
    let start = trivium.span.start as usize;
    let end = trivium.span.end as usize;
    let run = &text[start..end];
    let opens_a_line = trivium.kind == TriviaKind::Whitespace;
    if let Some(carried) = opens_a_line.then(|| braces.before(end)).flatten() {
        out.push_str(carried);
        return;
    }
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
