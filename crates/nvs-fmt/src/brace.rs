//! Where a brace sits: on its own line after a declaration, on the keyword's
//! line after a control structure.
//!
//! `rule:tooling/fmt-base-style-is-per` splits braces by the *kind of body*
//! they open, and both halves of that are whitespace on one side of a `{` — a
//! declaration's opener is preceded by a line break and the declaration's own
//! indentation, a control structure's by exactly one space, and `elseif`,
//! `else`, `catch` and `finally` continue one space after the brace that closed
//! the clause above them. So what this answers is not "where does the brace
//! go" but "what does the run before this byte have to be", which is the
//! printer's own frame ([`crate::print`]).
//!
//! Which body a brace opens is a question about the tree, and
//! [`SyntaxIndex::at`](nvs_syntax::SyntaxIndex::at) answers it: the innermost
//! node containing a `{` is the production that wrote it, because every
//! construct that could hold a brace of its own — a nested block, an
//! expression, a member — is a node in its own right. That is also what keeps a
//! brace inside a string literal or an attribute's object literal out of this:
//! a literal is an expression node, so the innermost node at its `{` is never a
//! declaration. The kinds below are the ones
//! `crates/nvs-syntax/src/walk.rs` spells, never a match on
//! [`nvs_syntax::ast`], for the reason [`crate::indent`]'s doc gives.
//!
//! The line break a declaration's brace moves onto is the one its own file
//! writes: a file with a `\r\n` anywhere in it gets `\r\n` here, because a
//! formatter that left one line of a file ending differently from the rest
//! would be corrupting it rather than normalizing it. The repository's own
//! corpus is LF (`.gitattributes`), so this is about files this tool is pointed
//! at, not about the tree.
//!
//! A closure, an anonymous class, a property hook and a bracketed `namespace`
//! are deliberately absent. The rule names a class, an interface, an enum and a
//! named function or method, and the first two of those sit inside an
//! expression, whose interior is the author's for good
//! (`rule:tooling/fmt-never-reflows`).

use nvs_diagnostics::BytePos;
use nvs_syntax::{IndexNode, SyntaxIndex, Trivia, TriviaKind};

use crate::indent::Indent;
use crate::print::Rewrite;

/// One level of the brace rule: the declarations whose body's `{` starts a line
/// of its own, at the declaration's own indentation.
const DECLARATIONS: &[&str] = &[
    "ClassDecl",
    "InterfaceDecl",
    "EnumDecl",
    "Function",
    "Method",
];

/// The control structures that write every brace of theirs inside one node.
///
/// A `try` holds its own body's braces and each `catch`'s and the `finally`'s,
/// and a `switch` holds the pair around its arms, so a `{` whose innermost node
/// is one of these is a body opener with nothing further to check.
const CARRIES_ITS_BRACES: &[&str] = &["Try", "Switch"];

/// The control structures whose body is a [`Block`](nvs_syntax::ast::Block) of
/// its own, which is the node that carries the braces.
const TAKES_A_BLOCK: &[&str] = &["If", "While", "DoWhile", "For", "Foreach"];

/// Each keyword that continues on the closing brace's line, and the node whose
/// clause it opens.
///
/// The node is what tells a continuation from the same word written anywhere
/// else: `catch` is also an expression in Novis, and a `} else ` inside a
/// string literal is bytes a program prints.
const CONTINUATIONS: &[(&str, &str)] = &[
    ("elseif", "If"),
    ("else", "If"),
    ("catch", "Try"),
    ("finally", "Try"),
];

/// The whitespace this stage requires before a byte, for one file.
pub(crate) struct Placements {
    /// What must precede an offset, by that offset, in source order.
    wanted: Vec<(usize, String)>,
    /// Which of those the author wrote no whitespace run at all before, as
    /// indices into `wanted`.
    absent: Vec<usize>,
}

impl Placements {
    /// The whitespace that belongs immediately before `offset`, or [`None`]
    /// where this stage decides nothing and the author's own run stands.
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

/// Everything the brace rule requires of `text`, which must be `index`'s and
/// `trivia`'s own file.
pub(crate) fn placements(
    index: &SyntaxIndex,
    indent: &Indent<'_>,
    text: &str,
    trivia: &[Trivia],
) -> Placements {
    let mut wanted = Vec::new();
    let mut bodies = Vec::new();
    let mut cursor = 0_usize;
    for trivium in trivia {
        scan(
            &mut wanted,
            &mut bodies,
            index,
            text,
            cursor,
            trivium.span.start as usize,
        );
        cursor = trivium.span.end as usize;
    }
    scan(&mut wanted, &mut bodies, index, text, cursor, text.len());

    let line_break = if text.contains("\r\n") { "\r\n" } else { "\n" };
    for (node, at) in bodies {
        let opening = indent.opening_of(node.span.start as usize);
        wanted.push((at, format!("{line_break}{opening}")));
    }
    wanted.sort_by_key(|(at, _)| *at);

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

    Placements { wanted, absent }
}

/// Reads `text[from..to]`, which is code and nothing else, and records what
/// each brace and each continuation keyword in it wants written before it.
///
/// A declaration's body opener is collected apart from the rest: an attribute's
/// object literal is written inside the declaration it decorates and outside
/// every node the walk builds, so it lands here as a candidate too, and the one
/// that opens the body is the last of them — the members that follow it are
/// nodes of their own. A declaration whose last byte is not `}` has no body at
/// all, and then none of its candidates is one.
fn scan(
    wanted: &mut Vec<(usize, String)>,
    bodies: &mut Vec<(IndexNode, usize)>,
    index: &SyntaxIndex,
    text: &str,
    from: usize,
    to: usize,
) {
    let bytes = text.as_bytes();
    for offset in from..to {
        let continues = matches!(bytes[offset], b'e' | b'c' | b'f')
            .then(|| continues_a_clause(bytes, offset))
            .flatten();
        if bytes[offset] != b'{' && continues.is_none() {
            continue;
        }
        let Ok(pos) = BytePos::try_from(offset) else {
            continue;
        };
        let path = index.at(pos);
        let nodes = path.nodes();
        let Some(node) = nodes.first() else {
            continue;
        };
        if let Some(owner) = continues {
            if owner == node.kind {
                wanted.push((offset, " ".to_owned()));
            }
        } else if node.kind == "Block" {
            let held_by = nodes.get(1);
            if node.span.start as usize == offset
                && held_by.is_some_and(|held_by| TAKES_A_BLOCK.contains(&held_by.kind))
            {
                wanted.push((offset, " ".to_owned()));
            }
        } else if CARRIES_ITS_BRACES.contains(&node.kind) {
            wanted.push((offset, " ".to_owned()));
        } else if DECLARATIONS.contains(&node.kind) && has_a_body(bytes, *node) {
            match bodies.last_mut() {
                Some((written, brace)) if written.span == node.span => *brace = offset,
                _ => bodies.push((*node, offset)),
            }
        }
    }
}

/// Whether `node`'s last byte is the closing brace of a body.
fn has_a_body(bytes: &[u8], node: IndexNode) -> bool {
    (node.span.end as usize)
        .checked_sub(1)
        .and_then(|last| bytes.get(last))
        == Some(&b'}')
}

/// The node a continuation keyword written at `offset` would belong to, if one
/// is written there with a closed body behind it.
///
/// The last byte before the word has to be the `}` the clause above ended on:
/// anything else there — a comment, a semicolon, an operand — is either a
/// layout this stage does not decide or a different construct entirely.
fn continues_a_clause(bytes: &[u8], offset: usize) -> Option<&'static str> {
    if offset > 0 && is_word(bytes[offset - 1]) {
        return None;
    }
    let (_, owner) = CONTINUATIONS.iter().find(|(keyword, _)| {
        bytes[offset..].starts_with(keyword.as_bytes())
            && !bytes
                .get(offset + keyword.len())
                .copied()
                .is_some_and(is_word)
    })?;
    let closed = bytes[..offset]
        .iter()
        .rev()
        .find(|byte| !byte.is_ascii_whitespace())
        == Some(&b'}');
    closed.then_some(*owner)
}

/// Whether `byte` can sit inside a name, which a keyword's neighbour may not.
///
/// Every byte of a multi-byte character counts as one: a keyword is ASCII, so a
/// word boundary never falls inside one of those.
fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' || byte >= 0x80
}
