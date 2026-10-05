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
//! What a whitespace run has to be is [`Runs`], which every stage that decides
//! one contributes to: the line a brace sits on ([`crate::brace`]) and the run
//! a qualifier, a one-line anonymous object or a `match` arm wants
//! ([`crate::space`]) are the same answer to "what must precede this byte", and
//! the run that is not there to rewrite is written into the code run instead.
//!
//! A code run is copied byte for byte, save for the edits that write something
//! other than what is there, each of them a [`Rewrite`]: a modifier list goes
//! out in its canonical order ([`crate::modifiers`]), a run of imports in path
//! order ([`crate::imports`]), a run [`Runs`] requires where its author wrote
//! none, and a literal the quote rule respells
//! gets the delimiters — a multi-line list the comma, and a mis-cased reserved
//! spelling its lower-case letters — that [`crate::tokens`] decides.
//! `rule:tooling/fmt-never-reflows` leaves what is inside an expression to the
//! author, and bytes a program prints — inline HTML, a heredoc body, an html
//! template's body — are never a formatter's to touch, so a rule that appears to
//! require rewriting one is being misread.

use std::iter::Peekable;
use std::vec::IntoIter;

use nvs_diagnostics::{Diagnostics, SourceFile, Span};
use nvs_syntax::{Parsed, Trivia, TriviaKind};

use crate::indent::Indent;
use crate::{brace, chain, imports, list, modifiers, space, tokens};

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

/// The whitespace the layout rules require before a byte, for one file.
///
/// Every stage that decides a run hands in the same pair — the offset the run
/// precedes, and what has to be there — and the two questions the printer then
/// asks are which run to write over ([`Runs::before`]) and which one has no run
/// to write over at all ([`Runs::insertions`]). A byte is one stage's or
/// another's and never both, so the answer for an offset is a single string.
pub(crate) struct Runs {
    /// What must precede an offset, by that offset, in source order.
    wanted: Vec<(usize, String)>,
    /// Which of those the author wrote no whitespace run at all before, as
    /// indices into `wanted`.
    absent: Vec<usize>,
}

impl Runs {
    /// What `wanted` requires of the file `trivia` came from.
    ///
    /// A run is the author's unless a stage asked for it, so the offsets no
    /// stage named are simply absent from here, and an offset two of them named
    /// keeps the first answer rather than writing two runs at one byte.
    pub(crate) fn new(mut wanted: Vec<(usize, String)>, trivia: &[Trivia]) -> Self {
        wanted.sort_by_key(|(at, _)| *at);
        wanted.dedup_by_key(|(at, _)| *at);
        let after_whitespace: Vec<usize> = trivia
            .iter()
            .filter(|trivium| trivium.kind == TriviaKind::Whitespace)
            .map(|trivium| trivium.span.end as usize)
            .collect();
        let absent = wanted
            .iter()
            .enumerate()
            .filter(|(_, (at, _))| after_whitespace.binary_search(at).is_err())
            .map(|(index, _)| index)
            .collect();
        Self { wanted, absent }
    }

    /// The whitespace that belongs immediately before `offset`, or [`None`]
    /// where every stage decides nothing and the author's own run stands.
    pub(crate) fn before(&self, offset: usize) -> Option<&str> {
        let at = self
            .wanted
            .binary_search_by_key(&offset, |(at, _)| *at)
            .ok()?;
        Some(&self.wanted[at].1)
    }

    /// The runs to write where the author left no whitespace to rewrite.
    ///
    /// `class Queue{` has no trivium between the name and the brace, so the
    /// line break the rule requires there is an insertion into a code run
    /// rather than a rewritten one — the same edit with an empty range.
    pub(crate) fn insertions(&self) -> Vec<Rewrite<'_>> {
        self.absent
            .iter()
            .map(|&at| Rewrite {
                start: self.wanted[at].0,
                end: self.wanted[at].0,
                written: &self.wanted[at].1,
            })
            .collect()
    }
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
/// `parsed` and `reported` must be `file`'s own parse and what it reported: the
/// spans on both sides are offsets into its text.
pub(crate) fn print(file: &SourceFile, parsed: &Parsed, reported: &Diagnostics) -> String {
    let text = file.text();
    let indent = Indent::new(&parsed.index, text, &parsed.trivia);
    let mut wanted = brace::placements(&parsed.index, &indent, text, &parsed.trivia);
    wanted.extend(space::runs(&parsed.index, &indent, text, &parsed.trivia));
    wanted.extend(list::runs(&parsed.index, &indent, text, &parsed.trivia));
    wanted.extend(chain::runs(&parsed.index, &indent, text, &parsed.trivia));
    let runs = Runs::new(wanted, &parsed.trivia);
    let mut edits = modifiers::rewrites(parsed, text);
    edits.extend(imports::rewrites(parsed, text));
    edits.extend(tokens::rewrites(&parsed.index, text, &parsed.trivia));
    edits.extend(tokens::spellings(reported, text));
    // An insertion sorts before the run that starts where it does, and a
    // trailing comma before the line break that moves its list's closer.
    edits.extend(runs.insertions());
    edits.sort_by_key(|edit| (edit.start, edit.end));
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
        push_trivium(&mut out, &indent, &runs, text, *trivium);
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
/// A whitespace run a stage requires is that stage's, whole: [`Runs`] decides
/// both how many bytes there are and what follows the last of them, because a
/// declaration's opener, a control structure's and the space in front of a
/// qualified type are one run written several ways. Every other run that
/// carries a
/// line break ends by opening a line, and what opens that line is the one thing
/// in the run this stage decides: everything up to and including the last break
/// is the author's — blank lines and all — and what follows it is four spaces
/// per enclosing body (`rule:tooling/fmt-base-style-is-per`). A run the tree
/// does not place, and every comment, go out exactly as they came in.
fn push_trivium(out: &mut String, indent: &Indent<'_>, runs: &Runs, text: &str, trivium: Trivia) {
    let start = trivium.span.start as usize;
    let end = trivium.span.end as usize;
    let run = &text[start..end];
    let opens_a_line = trivium.kind == TriviaKind::Whitespace;
    if let Some(carried) = opens_a_line.then(|| runs.before(end)).flatten() {
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
