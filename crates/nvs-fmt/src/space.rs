//! The whitespace a construct of Novis's own requires: one space in front of a
//! qualified type, one inside each brace of a one-line anonymous object, and a
//! line of its own for each arm of a `match` written across lines.
//!
//! `rule:tooling/fmt-novis-constructs` gives every construct PER never saw one
//! layout, and three of them are the run between two tokens that the author may
//! have written none, one or several bytes of. So what this answers is the same
//! question [`crate::brace`] answers — "what does the run before this byte have
//! to be" — for the bytes inside a construct rather than the brace that opens a
//! body, and the printer applies both the same way: the whitespace run before
//! the byte is written as this says, or inserted where the author left none.
//!
//! # A qualifier and its type
//!
//! `tainted` and `secret` sit one space before the type they qualify. Both are
//! reserved keywords
//! (`rule:security/tainted-qualifier`, `rule:security/secret-qualifier`), so
//! the word alone is enough to find one — but a keyword is also a member name
//! (`$row->tainted`) and also an English word inside text a program prints, and
//! neither of those is a type's. The first is told by the byte in front of the
//! word and the second by the node the byte is in: everything inside a string
//! literal, an html template or an inline-HTML region is the program's own
//! output, which no formatter writes.
//!
//! What follows the word settles the rest. A qualifier qualifies `string`,
//! `bytes`, a shape of them or a second qualifier and nothing else
//! (`rule:security/tainted-qualifier`), so what comes next is a word or the `{`
//! of a shape, and a keyword written in front of anything else is not a
//! qualifier at all — `case tainted;` and `$row->secret()` both end here. A
//! nullable type is never one of those: `?` wraps the qualified type rather
//! than sitting inside it, so a `?` is written in front of the qualifier and
//! this rule never meets one. A line break between the two is the author's like
//! every other one (`rule:tooling/fmt-never-reflows`), and a gap holding one is
//! left alone.
//!
//! # A one-line anonymous object
//!
//! `{a: 1, b: 2}` goes out as `{ a: 1, b: 2 }`: one space after the opening
//! brace and one before the closing one. Only an object its author wrote on one
//! line is this rule's — across lines it is one field per line, which is
//! [`crate::indent`]'s depth and [`crate::tokens`]'s trailing comma rather than
//! a space. An empty literal has no inside to space, and a literal written
//! inside an attribute is outside every node
//! `crates/nvs-syntax/src/walk.rs` builds, which is a known gap in
//! [`the crate's own doc`](crate).
//!
//! A shape type's braces are the same layout in the same rule, and they are not
//! here: a type is no node in that walk, so the `{` of `tainted {a: string}`
//! cannot be told from the `{` of a block by asking the index what wrote it.
//! That half is the same known gap the trailing comma has over the same fields.
//!
//! # A `match` arm list written across lines
//!
//! Each arm of a list its author wrote across lines gets a line of its own, at
//! one level in from the line the `match` keyword was written on. Only an arm
//! sharing a line with the one before it is this stage's: an arm already
//! opening a line is a depth [`crate::indent`] answers, and a run written here
//! would take the blank line its author left above it along with it. A list
//! written wholly on one line is left on it, which is the half
//! `rule:tooling/fmt-never-reflows` keeps — whether the list spans lines is the
//! author's, and what the arms are laid out as once it does is this rule's.

use nvs_diagnostics::BytePos;
use nvs_syntax::{SyntaxIndex, Trivia};

use crate::brace::is_word;
use crate::indent::{ARM_LIST, Indent, UNIT, arm_starts};

/// What this stage ever requires.
const SPACE: &str = " ";

/// The keywords that qualify a type, spelled as the lexer reads them.
const QUALIFIERS: &[&str] = &["tainted", "secret"];

/// The node kinds whose interior is text a program prints rather than code,
/// spelled as `crates/nvs-syntax/src/walk.rs` spells them.
///
/// A qualifier's spelling is an ordinary English word, so `Is it tainted?` sits
/// inside a string literal looking exactly like a qualifier in front of a
/// nullable type. The innermost node holding the byte is what tells them apart,
/// and these are the kinds that answer "not code".
const TEXT: &[&str] = &["Str", "Interpolated", "HtmlTemplate", "InlineHtml"];

/// Everything these rules require of `text`, which must be `index`'s and
/// `trivia`'s own file: the run that has to precede an offset, by that offset.
pub(crate) fn runs(
    index: &SyntaxIndex,
    indent: &Indent<'_>,
    text: &str,
    trivia: &[Trivia],
) -> Vec<(usize, String)> {
    let mut wanted = Vec::new();
    let mut cursor = 0_usize;
    for trivium in trivia {
        scan(
            &mut wanted,
            index,
            indent,
            text,
            trivia,
            cursor,
            trivium.span.start as usize,
        );
        cursor = trivium.span.end as usize;
    }
    scan(&mut wanted, index, indent, text, trivia, cursor, text.len());
    wanted
}

/// Reads `text[from..to]`, which is code and nothing else, and records what
/// each qualifier, each one-line anonymous object and each `match` arm list in it
/// requires.
fn scan(
    wanted: &mut Vec<(usize, String)>,
    index: &SyntaxIndex,
    indent: &Indent<'_>,
    text: &str,
    trivia: &[Trivia],
    from: usize,
    to: usize,
) {
    for (offset, byte) in text.as_bytes().iter().enumerate().take(to).skip(from) {
        match *byte {
            b'{' => {
                anon_object(wanted, index, text, trivia, offset);
                arm_lines(wanted, index, indent, text, offset);
            }
            b's' | b't' => qualifier(wanted, index, text, trivia, offset),
            _ => {}
        }
    }
}

/// Records the space the type qualified at `offset` requires in front of it,
/// where a qualifier is written there and a type follows it.
fn qualifier(
    wanted: &mut Vec<(usize, String)>,
    index: &SyntaxIndex,
    text: &str,
    trivia: &[Trivia],
    offset: usize,
) {
    let bytes = text.as_bytes();
    if offset > 0 && is_word(bytes[offset - 1]) {
        return;
    }
    let Some(word) = QUALIFIERS.iter().find(|word| {
        bytes[offset..].starts_with(word.as_bytes())
            && !bytes.get(offset + word.len()).copied().is_some_and(is_word)
    }) else {
        return;
    };
    if names_a_member(bytes, offset) || is_text(index, offset) {
        return;
    }
    let after = offset + word.len();
    let at = code_after(trivia, after);
    if !bytes.get(at).copied().is_some_and(starts_a_type) {
        return;
    }
    if text[after..at].contains('\n') {
        return;
    }
    wanted.push((at, SPACE.to_owned()));
}

/// Records the two spaces the anonymous object opened at `offset` requires
/// inside its braces, where one is written there on a single line.
fn anon_object(
    wanted: &mut Vec<(usize, String)>,
    index: &SyntaxIndex,
    text: &str,
    trivia: &[Trivia],
    offset: usize,
) {
    let Ok(pos) = BytePos::try_from(offset) else {
        return;
    };
    let Some(node) = index.at(pos).innermost() else {
        return;
    };
    if node.kind != "AnonObject" || node.span.start as usize != offset {
        return;
    }
    let end = node.span.end as usize;
    if text.as_bytes().get(end - 1) != Some(&b'}') {
        return;
    }
    let Some(whole) = text.get(offset..end) else {
        return;
    };
    if whole.contains('\n') {
        return;
    }
    let first = code_after(trivia, offset + 1);
    if first >= end - 1 {
        return;
    }
    wanted.push((first, SPACE.to_owned()));
    wanted.push((end - 1, SPACE.to_owned()));
}

/// Records the line each arm of the `match` whose list opens at `offset` has to
/// begin on, where its author wrote that list across lines.
///
/// The arm list's `{` is the one brace a `match` writes itself: its subject and
/// every condition and body are children of it, so the innermost node at any
/// other brace inside a `match` is one of them. An arm that already opens a
/// line is left for [`crate::indent`] to place, because the run this would
/// write over is the one holding whatever blank lines its author left there.
fn arm_lines(
    wanted: &mut Vec<(usize, String)>,
    index: &SyntaxIndex,
    indent: &Indent<'_>,
    text: &str,
    offset: usize,
) {
    let Ok(pos) = BytePos::try_from(offset) else {
        return;
    };
    let Some(node) = index.at(pos).innermost() else {
        return;
    };
    let Some(whole) = text.get(offset..node.span.end as usize) else {
        return;
    };
    if node.kind != ARM_LIST || !whole.contains('\n') {
        return;
    }
    let line_break = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let opening = indent.opening_of(node.span.start as usize);
    for at in arm_starts(indent, node) {
        let written = &text[..at];
        if written.trim_end_matches([' ', '\t']).ends_with('\n') {
            continue;
        }
        wanted.push((at, format!("{line_break}{opening}{UNIT}")));
    }
}

/// The offset of the first code byte at or after `from`, which is `from` itself
/// unless a trivium begins there.
///
/// A gap between two tokens is trivia and nothing else, so walking the trivia
/// that abut one another from `from` lands on the next token — and the comment
/// in `{/* kept */ a: 1}` is one of them, because what this looks for is where
/// the code resumes rather than what was skipped to reach it.
pub(crate) fn code_after(trivia: &[Trivia], from: usize) -> usize {
    let mut at = from;
    let mut next = trivia.partition_point(|trivium| (trivium.span.start as usize) < at);
    while let Some(trivium) = trivia.get(next) {
        if trivium.span.start as usize != at {
            break;
        }
        at = trivium.span.end as usize;
        next += 1;
    }
    at
}

/// Whether the word at `offset` is the name of a member rather than a keyword.
///
/// A name written after `->`, `?->` or `::` is any word at all, keyword
/// included, so the last code byte in front of the word is what says which this
/// is. The second `:` of `::` is the whole question there — a single one
/// introduces a return type and a shape's field type, which are exactly the
/// positions this rule exists for. Whitespace is all that can sit between the
/// two: a comment there is a gap this leaves to its author, and reading it as a
/// member name is the safe direction — the file goes out as it came in.
fn names_a_member(bytes: &[u8], offset: usize) -> bool {
    let Some(last) = bytes[..offset]
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
    else {
        return false;
    };
    bytes[last] == b'>' || bytes[last] == b':' && last > 0 && bytes[last - 1] == b':'
}

/// Whether `byte` can open the type a qualifier qualifies.
///
/// The name of a scalar or of a second qualifier, or the `{` of a shape.
/// Everything else a keyword can be followed by — a `(`, a `;`, an operator, a
/// `$` — belongs to some other construct that spelled the same word.
fn starts_a_type(byte: u8) -> bool {
    byte == b'{' || is_word(byte) && byte != b'$'
}

/// Whether the byte at `offset` is inside text a program prints.
///
/// A byte no node contains is read as text: the index answers every byte of a
/// file the parse placed, so the absence is either a file it did not place or a
/// region outside the grammar, and neither is a byte to rewrite.
fn is_text(index: &SyntaxIndex, offset: usize) -> bool {
    let Ok(pos) = BytePos::try_from(offset) else {
        return true;
    };
    index
        .at(pos)
        .innermost()
        .is_none_or(|node| TEXT.contains(&node.kind))
}
