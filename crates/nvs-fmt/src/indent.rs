//! How deep a line sits: four spaces per enclosing `{ … }` body.
//!
//! `rule:tooling/fmt-base-style-is-per` opens with four-space indentation and
//! no tabs, and the depth it counts is the nesting of brace-delimited bodies —
//! a [`Block`](nvs_syntax::ast::Block), a class or interface or enum body, a
//! `try` and its clauses, a function's or an anonymous function's body. The depth of a byte
//! is therefore a question about the tree, and
//! [`SyntaxIndex::at`](nvs_syntax::SyntaxIndex::at) already answers "which
//! nodes contain this offset, innermost first" for
//! `rule:ide/the-index-answers-the-cursor`. Finding the innermost body on that
//! path is the whole computation, and it needs no second walk over the grammar
//! — `crates/nvs-syntax/src/walk.rs`'s module doc is explicit that the grammar
//! is matched in one place, so what lives here is a list of the kinds that walk
//! already spells, never a match on [`nvs_syntax::ast`].
//!
//! What a body's contents are measured from is the line its opener was written
//! on, rather than an absolute count of enclosing braces
//! ([`Indent::opening_of`]). The two agree on every line of ordinary code and
//! part company exactly where `rule:tooling/fmt-never-reflows` says they must:
//! an anonymous function the author wrapped inside a multi-line argument list starts at a
//! column this formatter did not choose and must not move, and its body is one
//! level in from there.
//!
//! # A line is re-indented only where the tree says what it is
//!
//! [`Indent::of_line`] answers [`None`] for a line the tree does not place, and
//! the printer then copies the author's own whitespace. Two kinds of line are
//! never placed, and both are rules rather than gaps:
//!
//! - **A template region's own text**, every byte after a `?>`, is what the
//!   program prints, so changing it would change the program's output
//!   (`rule:tooling/fmt-base-style-is-per`).
//! - **A continuation line that is not a part of a broken list, call chain
//!   or operator chain**, such as the second line of a long condition the
//!   author wrapped. This formatter chooses no line break inside an
//!   expression, so it has no depth of its own to give that line, and
//!   `rule:tooling/fmt-never-reflows` keeps the author's.
//!
//! A line is placed when it opens an item, a comment between two items or the
//! closer of a broken list ([`crate::list`]) or an arrow of a broken call
//! chain ([`crate::chain`]), when it opens a statement or a
//! member that a body directly
//! contains, when it opens that body's own closing brace, which sits at the
//! body's own depth rather than its contents', when it opens with a comment
//! and the code below that comment is placed, or when it opens with the `?>`
//! that leaves code mode. A comment line sits where the code below it sits,
//! and one above a body's closing brace where that body's statements sit. That tag is the last code on its line and sits where
//! the body's statements sit; every byte after it belongs to the text region,
//! which is the program's own output and is copied.
//!
//! # A `match` arm list
//!
//! A `match` is the one expression whose interior is placed here. Its arm list
//! is no brace-delimited body — the arms are the expression's own children — so
//! `Match` is absent from [`BODIES`] and an arm's line comes from
//! [`arm_starts`] instead: an arm sits one level in from the line the `match`
//! keyword was written on, and the brace closing the list sits on that line's
//! own depth (`rule:tooling/fmt-novis-constructs`). Everything else an arm
//! holds is an expression's interior and stays the author's. The break that
//! gives an arm a line of its own, where its author wrote two of them on one,
//! is a run [`crate::space`] requires rather than a depth.
//!
//! # What it spends
//!
//! One [`NodePath`](nvs_syntax::NodePath) per line that opens with whitespace,
//! built and dropped inside the call. The index lookup behind it is a binary
//! search, so a line costs the logarithm of the file's nodes plus its depth,
//! and [`Indent::opening_of`] adds one such lookup and one stack frame per
//! enclosing body. The parser stops at a fixed nesting limit (`E0108`), so
//! that depth is a constant and nothing caches it; `fmt/depth.nvs` under
//! `benches/scaling/` measures it. Nothing here runs on the request path
//! (`rule:tooling/fmt-is-never-a-diagnostic`).

use nvs_diagnostics::BytePos;
use nvs_syntax::{IndexNode, SyntaxIndex, Trivia, TriviaKind};

use crate::{chain, list, statement};

/// One level of indentation.
pub(crate) const UNIT: &str = "    ";

/// The expression whose children are the arms of a `match`, spelled as
/// `crates/nvs-syntax/src/walk.rs` spells it.
pub(crate) const ARM_LIST: &str = "Match";

/// The node kinds whose children are the statements or members of a brace-
/// delimited body, spelled as `crates/nvs-syntax/src/walk.rs` spells them.
///
/// `If`, `While`, `Foreach` and the rest are deliberately absent: a control
/// structure's braces are a `Block` statement of its own, which is the node
/// that carries them. `NamespaceDecl` is here for its bracketed form and costs
/// nothing in the statement form, which has no body to contain anything. `New`
/// carries an anonymous class's members, `Property` its hooks, and
/// `PropertyHook` a hook's body. A `switch` has no `Block`: `Switch` carries
/// its braces and its cases, and each `Case` its statements, which is what
/// puts a case one level in from the `switch` and its statements one further.
pub(crate) const BODIES: &[&str] = &[
    "Block",
    "Try",
    "Switch",
    "Case",
    "ClassDecl",
    "InterfaceDecl",
    "EnumDecl",
    "NamespaceDecl",
    "Function",
    "Method",
    "Fn",
    "Property",
    "PropertyHook",
    "New",
];

/// The node kinds whose interior is never laid out.
///
/// Inline HTML is a program's own output bytes and never a formatter's
/// (`rule:tooling/fmt-base-style-is-per`). A line anywhere inside it is left
/// exactly as its author wrote it.
pub(crate) const OPAQUE: &[&str] = &["InlineHtml"];

/// Where each line of a file sits, answered from that file's parse.
pub(crate) struct Indent<'a> {
    index: &'a SyntaxIndex,
    text: &'a str,
    trivia: &'a [Trivia],
    /// Where [`crate::statement`] starts a line the author did not, in source
    /// order.
    breaks: Vec<usize>,
}

impl<'a> Indent<'a> {
    /// Reads `index` and `trivia`, which must both be `text`'s own parse.
    pub(crate) fn new(index: &'a SyntaxIndex, text: &'a str, trivia: &'a [Trivia]) -> Self {
        Self {
            index,
            text,
            trivia,
            breaks: statement::breaks(index, text, trivia),
        }
    }

    /// The parse this reads: the index, the text and its trivia.
    pub(crate) const fn parse(&self) -> (&'a SyntaxIndex, &'a str, &'a [Trivia]) {
        (self.index, self.text, self.trivia)
    }

    /// The text that should open the line whose first non-whitespace byte is at
    /// `offset`, or [`None`] to keep whatever the author wrote there.
    pub(crate) fn of_line(&self, offset: usize) -> Option<String> {
        let at = BytePos::try_from(offset).ok()?;
        let path = self.index.at(at);
        let nodes = path.nodes();
        if nodes.is_empty() || nodes.iter().any(|node| OPAQUE.contains(&node.kind)) {
            return None;
        }
        if let Some(opening) = list::opening(self, nodes, offset) {
            return Some(opening);
        }
        if let Some(opening) = chain::opening(self, nodes, offset) {
            return Some(opening);
        }

        // A byte closing a body sits where that body's own opening was written,
        // and a statement or member the body contains sits one level in from it.
        // A `match`'s arm list closes the same way, on the line its own `match`
        // keyword was written on. A `try` writes the braces of every clause
        // itself, so each `}` directly inside it closes one.
        let closing = (self.text.as_bytes().get(offset) == Some(&b'}'))
            .then(|| {
                nodes.iter().enumerate().position(|(at, node)| {
                    (BODIES.contains(&node.kind) || node.kind == ARM_LIST)
                        && (node.span.end as usize == offset + 1 || at == 0 && node.kind == "Try")
                })
            })
            .flatten();
        if let Some(body) = closing {
            return Some(self.opening_of(opener(nodes, body)));
        }

        // An arm is the arm list's and no body's, so it is placed before the
        // question about bodies is asked at all.
        let list = nodes.iter().find(|node| node.kind == ARM_LIST);
        if let Some(list) = list.filter(|list| arm_starts(self, **list).contains(&offset)) {
            return Some(self.opening_of(list.span.start as usize) + UNIT);
        }
        // A comment line sits where the line below it sits, which is the line
        // it describes. A comment above a closing brace is inside that body,
        // so it sits where the body's statements sit, one level in from the
        // brace. When the line below is not placed, the comment's line is not
        // placed either.
        if let Some(below) = self.code_below(offset) {
            let placed = self.of_line(below)?;
            return Some(if self.text.as_bytes().get(below) == Some(&b'}') {
                placed + UNIT
            } else {
                placed
            });
        }

        let opens_child = nodes
            .windows(2)
            .any(|pair| pair[0].span.start == at && BODIES.contains(&pair[1].kind))
            || nodes
                .last()
                .is_some_and(|outermost| outermost.span.start == at);
        // A close tag opens a line the way a statement does, and no node starts
        // at one: the inline HTML that follows begins past the tag, so nothing
        // in the path answers for these two bytes and the question is asked of
        // the text itself.
        let closes_code = self
            .text
            .get(offset..)
            .is_some_and(|rest| rest.starts_with("?>"));
        if !opens_child && !closes_code {
            return None;
        }
        let enclosing = nodes
            .iter()
            .position(|node| BODIES.contains(&node.kind) && (node.span.start as usize) < offset);
        Some(match enclosing {
            Some(body) => self.opening_of(opener(nodes, body)) + UNIT,
            None => String::new(),
        })
    }

    /// What will open the line `pos` is on, which is what a brace moved onto a
    /// line of its own is written at ([`crate::brace`]).
    ///
    /// A body indents its contents from the line its own opening was written
    /// on, not from an absolute count of enclosing braces, and that is what
    /// keeps `rule:tooling/fmt-never-reflows` whole: an anonymous function written inside a
    /// multi-line argument list starts at a column the author chose, and its
    /// body belongs one level in from *that* rather than one level in from the
    /// statement four continuation lines above. A line this module places
    /// answers with what it places there; one it does not answers with the
    /// author's own whitespace, which is the anchor those two rules share.
    ///
    /// The recursion ends because a body opens strictly before anything it
    /// contains, so each step asks about an earlier line than the last.
    ///
    /// A line [`crate::statement`] or a broken list ([`crate::list`]) starts is
    /// a line here too: the last of their breaks between the author's line
    /// start and `pos` is where the line opens, and that statement, brace, item
    /// or closer is always one this module places. Without the list's breaks, a
    /// chain inside an item the list moves would be measured from the line
    /// above, and a second format would move it again
    /// (`rule:tooling/fmt-is-idempotent`).
    pub(crate) fn opening_of(&self, pos: usize) -> String {
        let line_start = self.text[..pos].rfind('\n').map_or(0, |brk| brk + 1);
        let written = &self.text[line_start..];
        let width = written.len() - written.trim_start_matches([' ', '\t']).len();
        let first = line_start + width;
        let statement = self.breaks[..self.breaks.partition_point(|&at| at <= pos)]
            .last()
            .copied();
        let moved = statement.max(self.list_break(pos)).filter(|&at| at > first);
        if let Some(opening) = moved.and_then(|at| self.of_line(at)) {
            return opening;
        }
        self.of_line(first)
            .unwrap_or_else(|| self.text[line_start..first].to_owned())
    }

    /// The last item or closer at or before `pos` of a broken list around
    /// `pos`. The list starts a line there, whether or not its author did.
    fn list_break(&self, pos: usize) -> Option<usize> {
        let at = BytePos::try_from(pos).ok()?;
        let path = self.index.at(at);
        path.nodes()
            .iter()
            .filter(|node| list::writes_a_list(node.kind))
            .filter_map(|&node| {
                let list = list::List::of(self.index, self.text, self.trivia, node)?;
                list.broken.then_some(())?;
                list.items
                    .iter()
                    .copied()
                    .chain([list.closer])
                    .filter(|&start| start <= pos)
                    .max()
            })
            .max()
    }

    /// Where the first code after the comment at `offset` starts, or [`None`]
    /// when no comment starts at `offset`.
    ///
    /// Comments and the whitespace between them are skipped together, so a
    /// block of comment lines answers with the line of code under all of them.
    fn code_below(&self, offset: usize) -> Option<usize> {
        let mut at = self
            .trivia
            .partition_point(|trivium| (trivium.span.start as usize) < offset);
        let first = self.trivia.get(at)?;
        if first.span.start as usize != offset || first.kind == TriviaKind::Whitespace {
            return None;
        }
        let mut below = offset;
        while let Some(trivium) = self
            .trivia
            .get(at)
            .filter(|t| t.span.start as usize == below)
        {
            below = trivium.span.end as usize;
            at += 1;
        }
        Some(below)
    }

    /// Every offset [`crate::statement`] starts a new line at, in source order.
    pub(crate) fn breaks(&self) -> &[usize] {
        &self.breaks
    }
}

/// Where each arm of the `match` at `node` begins, in source order.
///
/// An arm is a node to nobody: `crates/nvs-syntax/src/walk.rs`'s own doc
/// decides that a match arm is a property of the expression holding it rather
/// than a production of its own, so a `match`'s children are its subject
/// followed by every condition and every body, flat. What tells one arm from
/// the next is the `=>` between a condition list and the body it answers — the
/// child after a body opens an arm, and the first child after the subject opens
/// the first — and an arm begins where the code after the `{` or the `,` in
/// front of it does. That last step is why this answers offsets rather than
/// children: a `default` arm has no condition, so its first byte is a keyword
/// no node covers.
///
/// A separator is a token, so only the gap's code bytes are read for one: a
/// `,`, a `{` or a `=>` inside a comment is prose, and a boundary taken from
/// one puts an arm's first byte in the middle of that comment — which
/// [`crate::space`] then asks for a line break in front of, inside a run the
/// printer copies whole. What a comment may still be is the first thing after
/// the separator, and then it opens the arm's line and is placed like the arm,
/// which is what a comment written above an arm is.
pub(crate) fn arm_starts(indent: &Indent<'_>, node: IndexNode) -> Vec<usize> {
    let (text, trivia) = (indent.text, indent.trivia);
    let children = indent.index.children_of(node);
    let mut starts = Vec::new();
    let mut opens_an_arm = true;
    for pair in children.windows(2) {
        let from = pair[0].span.end as usize;
        let Some(gap) = text.get(from..pair[1].span.start as usize) else {
            break;
        };
        if opens_an_arm {
            let after = gap
                .bytes()
                .enumerate()
                .rfind(|&(at, byte)| matches!(byte, b',' | b'{') && !commented(trivia, from + at))
                .map_or(0, |(at, _)| at + 1);
            let written = &gap[after..];
            starts.push(from + after + written.len() - written.trim_start().len());
        }
        opens_an_arm = gap
            .match_indices("=>")
            .any(|(at, _)| !commented(trivia, from + at));
    }
    starts
}

/// Whether `offset` is a byte of a comment.
///
/// `trivia` is in source order and no two of them overlap
/// (`rule:ide/tokens-plus-trivia-reproduce-the-file`), so the one trivium that
/// can hold an offset is the last one beginning at or before it.
pub(crate) fn commented(trivia: &[Trivia], offset: usize) -> bool {
    let after = trivia.partition_point(|trivium| trivium.span.start as usize <= offset);
    after > 0 && {
        let trivium = trivia[after - 1];
        trivium.kind != TriviaKind::Whitespace && offset < trivium.span.end as usize
    }
}

/// Where the construct that opened `nodes[body]` starts.
///
/// A `Block` is the braces of whatever holds it, so an `if` whose condition
/// wrapped across three lines still indents its body from the `if` rather than
/// from the continuation line the `{` happened to land on. Every other body in
/// [`BODIES`] is its own opener — an anonymous function's body belongs to the
/// function, and the function is where the author put it. A block held by another body is a
/// block statement of its own and opens where it is written. A `match`'s arm
/// list is the `match` itself, which is what puts its closing brace on the line
/// the keyword was written on.
fn opener(nodes: &[IndexNode], body: usize) -> usize {
    let held_by = nodes.get(body + 1);
    match held_by {
        Some(parent) if nodes[body].kind == "Block" && !BODIES.contains(&parent.kind) => {
            parent.span.start as usize
        }
        _ => nodes[body].span.start as usize,
    }
}
