//! A list is on one line or has one item per line, and the author's line break
//! at the list's own level is what chooses
//! (`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`).
//!
//! # Where a list's own level is
//!
//! A call's arguments, an array literal and an anonymous object are each a
//! production in [`LISTS`] whose own last byte is the delimiter that closes it. Its items are the node's children, and a child covers every
//! byte of the expression it is, so a byte of the list's span that no child
//! covers is the list's own: the opener, the commas, and what an item writes in
//! front of its expression — an array key's `=>`, a named argument's name, an
//! anonymous object's field name, a spread's `...`. A delimiter or a comma
//! inside a child belongs to that child, however deep, and that is the whole of
//! "a break inside an item never breaks the list around it".
//!
//! The opener is the last opening delimiter in those uncovered bytes before the
//! first item, which is why the search runs backwards from the closer: a call's
//! callee, a method's receiver and `new`'s class name all come first and are
//! never between the opener and an item. Every comma in them separates two
//! items. A byte inside a comment is skipped in both searches, since a comment
//! is prose.
//!
//! Two lists are written inside a node that ends somewhere else. An enum's
//! cases are its body, so its braces are the list's opener and closer, as long
//! as the body holds cases and nothing else. A parameter is no node at all, and
//! only its default value is a child, so a parameter list is read from the
//! signature's own bytes ([`parameters`]): its first `(` at depth zero, the
//! `)` that matches it, and the commas at depth one. A string an attribute
//! writes is no node either, and its quotes are followed so a bracket inside
//! one never counts.
//!
//! # Broken
//!
//! A list is broken when a line break sits next to one of its separators —
//! after the opener, on either side of a comma, or before the closer — in the
//! trivia between that separator and the code beside it. A line comment there
//! breaks the list, because the line break after it is one of those.
//!
//! A broken list's opener ends its line, each item opens a line one level in
//! from the opener's line, and the closer opens a line at the opener line's own
//! depth. The level is [`crate::indent`]'s, which places a line that already
//! opens with an item or the closer; [`runs`] writes the line break where an
//! item or the closer shares a line with what is in front of it. The trailing
//! comma is [`crate::tokens`]'s, which reads [`List::broken`] for the lists
//! this module lays out.
//!
//! A comment keeps the line it was written on. One written after a comma on
//! the previous item's line stays there, and one on a line of its own is
//! placed at the items' depth. An item's own lines after its first are its own
//! rules', and a line no rule places keeps its author's whitespace.
//!
//! # Not yet a list here
//!
//! A shape type is no node, so nothing tells its `{` from a block's, and a
//! `match` arm list is [`crate::indent`]'s. An enum on one line keeps its `{`
//! on the `enum` line, which [`crate::brace`] writes from [`List::on_one_line`]. A list inside a `switch` or inline HTML is left as it
//! is written, as every line there is ([`crate::indent`]'s `OPAQUE`).
//!
//! # What it spends
//!
//! One [`List`] per question, built from one node's children and dropped
//! after it: a list's items, commas and the trivia next to them. Nothing is
//! cached across lines, so a line inside several lists asks each one again.

use nvs_diagnostics::BytePos;
use nvs_syntax::{IndexNode, SyntaxIndex, Trivia};

use crate::indent::{Indent, OPAQUE, UNIT, commented};
use crate::space::code_after;

/// The productions that write a list, spelled as
/// `crates/nvs-syntax/src/walk.rs` spells them, with the delimiters that open
/// and close it.
const LISTS: &[(&str, u8, u8)] = &[
    ("Call", b'(', b')'),
    ("MethodCall", b'(', b')'),
    ("StaticCall", b'(', b')'),
    ("New", b'(', b')'),
    ("ArrayLiteral", b'[', b']'),
    ("AnonObject", b'{', b'}'),
];

/// The productions whose parameter list is a list: the first `(` of code at
/// the signature's own level, and the `)` that matches it.
const PARAMETERS: &[&str] = &["Function", "Method", "Fn"];

/// The production whose body is its case list.
const ENUM: &str = "EnumDecl";

/// Whether a node of `kind` writes a list this module lays out.
fn writes_a_list(kind: &str) -> bool {
    kind == ENUM || PARAMETERS.contains(&kind) || LISTS.iter().any(|(list, ..)| *list == kind)
}

/// Where a line opened inside a broken list sits.
pub(crate) enum Level {
    /// One level in from the opener's line: an item, or a comment between two.
    Item,
    /// The opener's line itself: the closer.
    Closer,
}

/// One list, read from the lossless tree.
pub(crate) struct List {
    /// The opening delimiter.
    opener: usize,
    /// The closing delimiter.
    pub(crate) closer: usize,
    /// Every comma between two items, and the trailing one if it is written.
    commas: Vec<usize>,
    /// The first code byte of each item.
    items: Vec<usize>,
    /// Whether a line break sits at the list's own level.
    pub(crate) broken: bool,
}

impl List {
    /// The list `node` writes, or [`None`] where it writes none this module
    /// lays out: another production, an empty list, or one inside an opaque
    /// region.
    pub(crate) fn of(
        index: &SyntaxIndex,
        text: &str,
        trivia: &[Trivia],
        node: IndexNode,
    ) -> Option<Self> {
        let bytes = text.as_bytes();
        // A pipeline's `$_` is a child written somewhere else entirely, at the
        // pipeline's left side, so the children are put in source order first.
        let mut children = index.children_of(node);
        children.sort_by_key(|child| child.span.start);
        let (opener, closer, commas) = if PARAMETERS.contains(&node.kind) {
            parameters(bytes, trivia, node, &children)?
        } else if node.kind == ENUM {
            cases(bytes, trivia, node, &children)?
        } else {
            delimited(bytes, trivia, node, &children)?
        };
        let at = BytePos::try_from(opener).ok()?;
        if index
            .at(at)
            .nodes()
            .iter()
            .any(|node| OPAQUE.contains(&node.kind))
        {
            return None;
        }

        let items: Vec<usize> = std::iter::once(opener)
            .chain(commas.iter().copied())
            .map(|separator| code_after(trivia, separator + 1))
            .filter(|&item| item < closer)
            .collect();
        if items.is_empty() {
            return None;
        }
        let breaks =
            |from: usize, to: usize| text.get(from..to).is_some_and(|gap| gap.contains('\n'));
        let broken = breaks(opener + 1, items[0])
            || commas.iter().any(|&comma| {
                breaks(code_before(trivia, comma), comma)
                    || breaks(comma + 1, code_after(trivia, comma + 1))
            })
            || breaks(code_before(trivia, closer), closer);
        Some(Self {
            opener,
            closer,
            commas,
            items,
            broken,
        })
    }

    /// The runs an enum's case list on one line requires, where its body's
    /// opener is at `brace`: one space in front of the `{`, one after it and
    /// one in front of the `}`. [`None`] for a list that is broken, or whose
    /// opener is not `brace`.
    pub(crate) fn on_one_line(&self, brace: usize) -> Option<[(usize, String); 3]> {
        (!self.broken && self.opener == brace).then(|| {
            [
                (self.opener, " ".to_owned()),
                (self.items[0], " ".to_owned()),
                (self.closer, " ".to_owned()),
            ]
        })
    }

    /// Where the line opening at `offset` sits, where this broken list places
    /// it: an item, a comment beside a separator, or the closer.
    fn level(&self, trivia: &[Trivia], offset: usize) -> Option<Level> {
        if !self.broken {
            return None;
        }
        if offset == self.closer {
            return Some(Level::Closer);
        }
        let after = |separator: usize| separator + 1..=code_after(trivia, separator + 1);
        let beside = after(self.opener).contains(&offset)
            || self.commas.iter().any(|&comma| {
                after(comma).contains(&offset)
                    || (code_before(trivia, comma)..comma).contains(&offset)
            })
            || (code_before(trivia, self.closer)..self.closer).contains(&offset);
        beside.then_some(Level::Item)
    }
}

/// What opens the line whose first byte is `offset`, where that byte is an
/// item, a comment beside a separator or the closer of a broken list on
/// `nodes`, the path to it innermost first.
pub(crate) fn opening(indent: &Indent<'_>, nodes: &[IndexNode], offset: usize) -> Option<String> {
    let (index, text, trivia) = indent.parse();
    nodes
        .iter()
        .filter(|node| writes_a_list(node.kind))
        .find_map(|&node| {
            let list = List::of(index, text, trivia, node)?;
            let level = list.level(trivia, offset)?;
            let base = indent.opening_of(list.opener);
            Some(match level {
                Level::Item => base + UNIT,
                Level::Closer => base,
            })
        })
}

/// Every run a broken list requires in `text`, which must be `index`'s and
/// `trivia`'s own file: the line break in front of an item or a closer that
/// shares a line with what precedes it, and no whitespace in front of a comma.
///
/// An item or closer that already opens a line is left to [`crate::indent`],
/// because the run in front of it holds the blank lines its author left there.
/// One with a comment between it and its separator on that line keeps it.
pub(crate) fn runs(
    index: &SyntaxIndex,
    indent: &Indent<'_>,
    text: &str,
    trivia: &[Trivia],
) -> Vec<(usize, String)> {
    let line_break = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut wanted = Vec::new();
    let mut cursor = 0_usize;
    let scan = |from: usize, to: usize, wanted: &mut Vec<(usize, String)>| {
        for (offset, byte) in text.as_bytes().iter().enumerate().take(to).skip(from) {
            if !matches!(byte, b')' | b']' | b'}') {
                continue;
            }
            let Ok(pos) = BytePos::try_from(offset) else {
                continue;
            };
            let Some(node) = index.at(pos).innermost() else {
                continue;
            };
            if !writes_a_list(node.kind) {
                continue;
            }
            let Some(list) = List::of(index, text, trivia, node)
                .filter(|list| list.closer == offset && list.broken)
            else {
                continue;
            };
            lay_out(wanted, indent, text, trivia, &list, line_break);
        }
    };
    for trivium in trivia {
        scan(cursor, trivium.span.start as usize, &mut wanted);
        cursor = trivium.span.end as usize;
    }
    scan(cursor, text.len(), &mut wanted);
    wanted
}

/// Records the runs one broken list requires.
fn lay_out(
    wanted: &mut Vec<(usize, String)>,
    indent: &Indent<'_>,
    text: &str,
    trivia: &[Trivia],
    list: &List,
    line_break: &str,
) {
    let base = indent.opening_of(list.opener);
    let separators = std::iter::once(list.opener).chain(list.commas.iter().copied());
    for (separator, &item) in separators.zip(&list.items) {
        if !text[separator + 1..item].contains('\n') {
            wanted.push((item, format!("{line_break}{base}{UNIT}")));
        }
    }
    for &comma in &list.commas {
        let from = code_before(trivia, comma);
        if from < comma && text[from..comma].trim().is_empty() {
            wanted.push((comma, String::new()));
        }
    }
    let from = code_before(trivia, list.closer);
    if !text[from..list.closer].contains('\n') {
        wanted.push((list.closer, format!("{line_break}{base}")));
    }
}

/// The opener, closer and commas of a list whose closer is `node`'s last byte.
fn delimited(
    bytes: &[u8],
    trivia: &[Trivia],
    node: IndexNode,
    children: &[IndexNode],
) -> Option<(usize, usize, Vec<usize>)> {
    let &(_, open, close) = LISTS.iter().find(|(kind, ..)| *kind == node.kind)?;
    let closer = (node.span.end as usize).checked_sub(1)?;
    if bytes.get(closer) != Some(&close) {
        return None;
    }
    let mut opener = None;
    let mut until = closer;
    for child in children.iter().rev() {
        if child.span.end as usize > until {
            continue;
        }
        opener = last_code(bytes, trivia, child.span.end as usize, until, open);
        if opener.is_some() {
            break;
        }
        until = child.span.start as usize;
    }
    let opener =
        opener.or_else(|| last_code(bytes, trivia, node.span.start as usize, until, open))?;
    let commas = commas_between(bytes, trivia, children, opener, closer);
    Some((opener, closer, commas))
}

/// The opener, closer and commas of an enum's case list: the body's braces,
/// where the body holds cases and nothing else.
///
/// The `{` is the first one of code at the declaration's own level, so a
/// brace inside an attribute in front of the `enum` keyword is not it.
fn cases(
    bytes: &[u8],
    trivia: &[Trivia],
    node: IndexNode,
    children: &[IndexNode],
) -> Option<(usize, usize, Vec<usize>)> {
    if children.iter().any(|child| child.kind != "EnumCase") {
        return None;
    }
    let closer = (node.span.end as usize).checked_sub(1)?;
    if bytes.get(closer) != Some(&b'}') {
        return None;
    }
    let mut depth = 0_usize;
    let opener = own_code(bytes, trivia, children, node.span.start as usize..closer).find(
        |&at| match bytes[at] {
            b'{' if depth == 0 => true,
            b'(' | b'[' | b'{' => {
                depth += 1;
                false
            }
            b')' | b']' | b'}' => {
                depth = depth.saturating_sub(1);
                false
            }
            _ => false,
        },
    )?;
    let commas = commas_between(bytes, trivia, children, opener, closer);
    Some((opener, closer, commas))
}

/// The opener, closer and commas of a function's parameter list.
///
/// A parameter is no node, and only its default value is a child, so the list
/// is read from the bytes: the first `(` of code at the signature's own level
/// opens it, and the `)` that brings the depth back to that level closes it. A
/// default value is skipped whole, and a type's own brackets and `<` … `>`
/// count as depth, so a comma inside a shape type or a generic type is not
/// one of the list's.
fn parameters(
    bytes: &[u8],
    trivia: &[Trivia],
    node: IndexNode,
    children: &[IndexNode],
) -> Option<(usize, usize, Vec<usize>)> {
    let mut depth = 0_usize;
    let mut opener = None;
    let mut commas = Vec::new();
    let span = node.span.start as usize..node.span.end as usize;
    for at in own_code(bytes, trivia, children, span) {
        match bytes[at] {
            b'(' if depth == 0 && opener.is_none() => {
                opener = Some(at);
                depth = 1;
            }
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => {
                depth = depth.checked_sub(1)?;
                if depth == 0
                    && let Some(opener) = opener
                {
                    return Some((opener, at, commas));
                }
            }
            b',' if depth == 1 && opener.is_some() => commas.push(at),
            _ => {}
        }
    }
    None
}

/// Every offset in `span` whose byte is code a node writes itself: not inside
/// a comment, a child or a string literal.
///
/// A string an attribute writes is no node, so its quotes are followed here,
/// and a bracket or a comma inside one is never a delimiter.
fn own_code<'a>(
    bytes: &'a [u8],
    trivia: &'a [Trivia],
    children: &'a [IndexNode],
    span: std::ops::Range<usize>,
) -> impl Iterator<Item = usize> + 'a {
    let mut quote = None;
    let mut escaped = false;
    span.filter(move |&at| {
        if commented(trivia, at)
            || children
                .iter()
                .any(|child| (child.span.start as usize..child.span.end as usize).contains(&at))
        {
            return false;
        }
        let byte = bytes[at];
        if let Some(open) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == open {
                quote = None;
            }
            return false;
        }
        if matches!(byte, b'"' | b'\'') {
            quote = Some(byte);
            return false;
        }
        true
    })
}

/// Every comma of code between `opener` and `closer` that no child covers.
fn commas_between(
    bytes: &[u8],
    trivia: &[Trivia],
    children: &[IndexNode],
    opener: usize,
    closer: usize,
) -> Vec<usize> {
    let mut commas = Vec::new();
    let mut from = opener + 1;
    for child in children
        .iter()
        .filter(|child| child.span.start as usize > opener)
    {
        push_commas(&mut commas, bytes, trivia, from, child.span.start as usize);
        from = from.max(child.span.end as usize);
    }
    push_commas(&mut commas, bytes, trivia, from, closer);
    commas
}

/// The last `wanted` byte of code in `bytes[from..to]`, skipping comments.
fn last_code(bytes: &[u8], trivia: &[Trivia], from: usize, to: usize, wanted: u8) -> Option<usize> {
    (from..to)
        .rev()
        .find(|&at| bytes[at] == wanted && !commented(trivia, at))
}

/// Appends every comma of code in `bytes[from..to]`, skipping comments.
fn push_commas(out: &mut Vec<usize>, bytes: &[u8], trivia: &[Trivia], from: usize, to: usize) {
    out.extend((from..to).filter(|&at| bytes[at] == b',' && !commented(trivia, at)));
}

/// One past the last code byte before `to`, which is `to` itself unless a
/// trivium ends there.
///
/// The mirror of [`code_after`]: the trivia that abut one another backwards
/// from `to` are what lies between it and the code in front of it.
fn code_before(trivia: &[Trivia], to: usize) -> usize {
    let mut at = to;
    let mut next = trivia.partition_point(|trivium| (trivium.span.end as usize) < at);
    while let Some(trivium) = trivia.get(next) {
        if trivium.span.end as usize != at {
            break;
        }
        at = trivium.span.start as usize;
        match next.checked_sub(1) {
            Some(before) => next = before,
            None => break,
        }
    }
    at
}
