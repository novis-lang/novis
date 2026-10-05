//! A `->` call chain is on one line or has one call per line, and the author's
//! line break before any of its arrows is what chooses
//! (`rule:tooling/fmt-a-broken-call-chain-is-one-call-per-line`). An `&&`,
//! `||`, `??` or `.` chain is on one line or has one operand per line, operator
//! first, and a broken control-structure condition puts its parentheses on
//! lines of their own
//! (`rule:tooling/fmt-a-broken-operator-chain-is-one-operand-per-line`).
//!
//! # What a chain is
//!
//! A link is a `MethodCall` or a `PropertyAccess`, spelled as
//! `crates/nvs-syntax/src/walk.rs` spells them, and its object is its first
//! child in source order. Its arrow, `->` or `?->`, is the first code byte
//! after that object, which no child covers. A chain is a link whose object is
//! a link, and so on down to an object that is not one, read from the
//! outermost link inwards.
//!
//! The chain's arrows start at its first call. A property read in front of it
//! belongs to the receiver, so `$this->orders->where(…)->first()` keeps
//! `$this->orders` together, and a property read after it is a link like a
//! call. A chain with fewer than two calls is no chain, and every line inside
//! it stays the author's.
//!
//! # Broken
//!
//! A chain is broken when a line break sits in the trivia in front of one of
//! its arrows. A broken chain keeps its receiver where it was written, and each
//! arrow opens a line one level in from the line the receiver starts on. The
//! arguments of each call are a list of their own ([`crate::list`]): a broken
//! chain does not break them, and a broken argument list does not break the
//! chain.
//!
//! [`runs`] writes the line break in front of an arrow that shares a line with
//! the code before it, and [`opening`] places a line that already opens with
//! one, which keeps the blank lines and the comments its author wrote there.
//!
//! # An operator chain
//!
//! A `Binary` node writes its operator as the first code after its left
//! operand, and only `&&`, `||`, `??` and `.` make a chain. The chain is the
//! outermost such node and every operand below it written with the same
//! operator, flattened in source order. A parenthesised operand is a `Paren`
//! node, so the flattening never reaches inside one. The parser has already
//! put the lowest-precedence operator at the top of a mixed run, so the chain
//! is the run at the lowest precedence, and an operand made of other operators
//! is a chain of its own, judged on its own.
//!
//! An operator chain is broken when a line break sits between two of its
//! operands, on either side of the operator. Each operator then opens a line
//! one level in from the line the first operand starts on, and one space
//! follows it.
//!
//! # A condition
//!
//! The condition of an `if`, `elseif`, `while` or `do … while` is the child
//! expression written between a `(` that follows the keyword and the `)` that
//! follows the expression. Its top-level chain is the expression's operator
//! chain, or the expression alone. A line break after the `(`, between two of
//! those operands or before the `)` breaks the condition. A broken condition's
//! `(` ends its line, each operand starts a line one level in from the line
//! the `(` is on, operator first after the first, and the `)` starts a line at
//! that line's own depth. [`crate::brace`] then writes ` {` after it.
//!
//! # What it spends
//!
//! One [`Chain`] or [`Operators`] per question, built from the chain's nodes
//! and dropped after it. A line that opens with an arrow or an operator walks
//! its chain once from the top, which is one index lookup per link or operand,
//! and a line inside a condition's parentheses reads that condition's chain.

use nvs_diagnostics::BytePos;
use nvs_syntax::{IndexNode, SyntaxIndex, Trivia};

use crate::indent::{Indent, OPAQUE, UNIT, commented};
use crate::list::code_before;
use crate::space::code_after;

/// The productions that are one link of a chain.
const LINKS: &[&str] = &["MethodCall", "PropertyAccess"];

/// The link that is a call.
const CALL: &str = "MethodCall";

/// The production that writes an operator between two operands.
const BINARY: &str = "Binary";

/// The operators whose run is a chain.
const OPERATORS: &[&str] = &["&&", "||", "??", "."];

/// The control structures whose condition a broken chain lays out.
const CONDITIONS: &[&str] = &["If", "While", "DoWhile"];

/// One call chain, read from the lossless tree.
struct Chain {
    /// The first byte of the receiver, the object of the innermost link.
    receiver: usize,
    /// The first byte of every arrow from the first call on, in source order.
    arrows: Vec<usize>,
    /// Whether a line break sits in front of one of those arrows.
    broken: bool,
}

impl Chain {
    /// The chain whose outermost link is `outermost`, or [`None`] where it
    /// holds fewer than two calls or sits in an opaque region.
    fn of(
        index: &SyntaxIndex,
        text: &str,
        trivia: &[Trivia],
        outermost: IndexNode,
    ) -> Option<Self> {
        let mut links = Vec::new();
        let mut node = outermost;
        while LINKS.contains(&node.kind) {
            let object = object_of(index, node)?;
            links.push((node.kind, arrow_of(text, trivia, object)?));
            node = object;
        }
        links.reverse();
        let first_call = links.iter().position(|&(kind, _)| kind == CALL)?;
        let links = &links[first_call..];
        if links.iter().filter(|&&(kind, _)| kind == CALL).count() < 2 {
            return None;
        }
        let at = BytePos::try_from(links[0].1).ok()?;
        if index
            .at(at)
            .nodes()
            .iter()
            .any(|node| OPAQUE.contains(&node.kind))
        {
            return None;
        }
        let arrows: Vec<usize> = links.iter().map(|&(_, arrow)| arrow).collect();
        let broken = arrows
            .iter()
            .any(|&arrow| text[code_before(trivia, arrow)..arrow].contains('\n'));
        Some(Self {
            receiver: node.span.start as usize,
            arrows,
            broken,
        })
    }
}

/// The object of the link `node`: its first child in source order.
fn object_of(index: &SyntaxIndex, node: IndexNode) -> Option<IndexNode> {
    index
        .children_of(node)
        .into_iter()
        .min_by_key(|child| child.span.start)
}

/// The first byte of the arrow written after `object`, or [`None`] where the
/// code there is no arrow.
fn arrow_of(text: &str, trivia: &[Trivia], object: IndexNode) -> Option<usize> {
    let arrow = code_after(trivia, object.span.end as usize);
    let rest = text.get(arrow..)?;
    (rest.starts_with("->") || rest.starts_with("?->")).then_some(arrow)
}

/// The outermost link of the chain `nodes[link]` belongs to, where `nodes` is a
/// path innermost first: each step outwards is a link whose object is the
/// step before.
fn outermost(index: &SyntaxIndex, nodes: &[IndexNode], link: usize) -> IndexNode {
    let mut at = link;
    while let Some(&parent) = nodes.get(at + 1) {
        if !LINKS.contains(&parent.kind) || object_of(index, parent) != Some(nodes[at]) {
            break;
        }
        at += 1;
    }
    nodes[at]
}

/// What opens the line whose first byte is `offset`, where that byte is an
/// arrow of a broken call chain, an operator of a broken operator chain, or the
/// first operand or the `)` of a broken condition on `nodes`, the path to it
/// innermost first.
pub(crate) fn opening(indent: &Indent<'_>, nodes: &[IndexNode], offset: usize) -> Option<String> {
    arrow_opening(indent, nodes, offset)
        .or_else(|| operator_opening(indent, nodes, offset))
        .or_else(|| condition_opening(indent, nodes, offset))
}

/// What opens the line at `offset` where it is an arrow of a broken call chain,
/// or a comment in front of one.
fn arrow_opening(indent: &Indent<'_>, nodes: &[IndexNode], offset: usize) -> Option<String> {
    let (index, text, trivia) = indent.parse();
    let link = *nodes.first()?;
    if !LINKS.contains(&link.kind) {
        return None;
    }
    let chain = Chain::of(index, text, trivia, outermost(index, nodes, 0))?;
    // A comment between an arrow and the code before it sits where the arrow
    // does.
    let beside = chain
        .arrows
        .iter()
        .any(|&arrow| (code_before(trivia, arrow)..=arrow).contains(&offset));
    (chain.broken && beside).then(|| indent.opening_of(chain.receiver) + UNIT)
}

/// Every run a broken chain requires in `text`, which must be `index`'s and
/// `trivia`'s own file: the line break in front of an arrow or an operator that
/// shares a line with the code before it, one space after an operator, and the
/// line breaks after a broken condition's `(` and before its `)`.
pub(crate) fn runs(
    index: &SyntaxIndex,
    indent: &Indent<'_>,
    text: &str,
    trivia: &[Trivia],
) -> Vec<(usize, String)> {
    let line_break = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut wanted = arrow_runs(index, indent, text, trivia, line_break);
    let mut cursor = 0_usize;
    for trivium in trivia {
        operator_runs(
            &mut wanted,
            indent,
            cursor,
            trivium.span.start as usize,
            line_break,
        );
        cursor = trivium.span.end as usize;
    }
    operator_runs(&mut wanted, indent, cursor, text.len(), line_break);
    wanted
}

/// The runs every broken call chain in `text` requires.
fn arrow_runs(
    index: &SyntaxIndex,
    indent: &Indent<'_>,
    text: &str,
    trivia: &[Trivia],
    line_break: &str,
) -> Vec<(usize, String)> {
    let mut wanted = Vec::new();
    for (offset, _) in text.match_indices("->") {
        // A `?->` is read from its `?`, which is where its link's arrow starts.
        let arrow = if offset > 0 && text.as_bytes()[offset - 1] == b'?' {
            offset - 1
        } else {
            offset
        };
        if commented(trivia, arrow) {
            continue;
        }
        let Ok(pos) = BytePos::try_from(arrow) else {
            continue;
        };
        let path = index.at(pos);
        let nodes = path.nodes();
        let Some(&link) = nodes.first() else {
            continue;
        };
        // A chain is laid out once, from the arrow of its outermost link.
        if !LINKS.contains(&link.kind) || outermost(index, nodes, 0) != link {
            continue;
        }
        let Some(chain) = Chain::of(index, text, trivia, link).filter(|chain| chain.broken) else {
            continue;
        };
        let base = indent.opening_of(chain.receiver);
        for &arrow in &chain.arrows {
            if !text[code_before(trivia, arrow)..arrow].contains('\n') {
                wanted.push((arrow, format!("{line_break}{base}{UNIT}")));
            }
        }
    }
    wanted
}

/// One operator chain, or one condition, read from the lossless tree.
struct Operators {
    /// Every operand, in source order.
    operands: Vec<IndexNode>,
    /// The first byte of every operator between two operands.
    operators: Vec<usize>,
    /// The operators' spelling.
    spelling: &'static str,
    /// The `(` and the `)` around a condition, where this is one.
    condition: Option<(usize, usize)>,
    /// Whether a line break sits at the chain's own level.
    broken: bool,
}

impl Operators {
    /// The chain `top` writes, inside the parentheses `condition` where it is
    /// a condition. [`None`] for an expression that is neither a chain nor a
    /// condition.
    fn of(
        index: &SyntaxIndex,
        text: &str,
        trivia: &[Trivia],
        top: IndexNode,
        condition: Option<(usize, usize)>,
    ) -> Option<Self> {
        let mut operands = Vec::new();
        let mut operators = Vec::new();
        let spelling = match operator_of(index, text, trivia, top) {
            Some((_, spelling)) => {
                flatten(
                    index,
                    text,
                    trivia,
                    top,
                    spelling,
                    &mut operands,
                    &mut operators,
                );
                spelling
            }
            None if condition.is_some() => {
                operands.push(top);
                ""
            }
            None => return None,
        };
        let breaks =
            |from: usize, to: usize| text.get(from..to).is_some_and(|gap| gap.contains('\n'));
        let broken = operands
            .windows(2)
            .any(|pair| breaks(pair[0].span.end as usize, pair[1].span.start as usize))
            || condition.is_some_and(|(open, close)| {
                breaks(open + 1, operands[0].span.start as usize)
                    || breaks(code_before(trivia, close), close)
            });
        Some(Self {
            operands,
            operators,
            spelling,
            condition,
            broken,
        })
    }

    /// What opens an operator's line: one level in from the line the `(` of a
    /// condition is on, or from the line the first operand starts on.
    fn operator_base(&self, indent: &Indent<'_>) -> String {
        let from = self
            .condition
            .map_or(self.operands[0].span.start as usize, |(open, _)| open);
        indent.opening_of(from) + UNIT
    }

    /// Records the runs this broken chain requires.
    fn lay_out(&self, wanted: &mut Vec<(usize, String)>, indent: &Indent<'_>, line_break: &str) {
        let (_, text, trivia) = indent.parse();
        let base = self.operator_base(indent);
        if let Some((open, close)) = self.condition {
            let first = self.operands[0].span.start as usize;
            if !text[open + 1..first].contains('\n') {
                wanted.push((first, format!("{line_break}{base}")));
            }
            if !text[code_before(trivia, close)..close].contains('\n') {
                let closing = indent.opening_of(open);
                wanted.push((close, format!("{line_break}{closing}")));
            }
        }
        for (&operator, operand) in self.operators.iter().zip(&self.operands[1..]) {
            if !text[code_before(trivia, operator)..operator].contains('\n') {
                wanted.push((operator, format!("{line_break}{base}")));
            }
            let start = operand.span.start as usize;
            let after = &text[operator + self.spelling.len()..start];
            if after != " " && after.trim().is_empty() {
                wanted.push((start, " ".to_owned()));
            }
        }
    }
}

/// The first byte of the operator `node` writes and its spelling, where `node`
/// is a `Binary` whose operator makes a chain.
fn operator_of(
    index: &SyntaxIndex,
    text: &str,
    trivia: &[Trivia],
    node: IndexNode,
) -> Option<(usize, &'static str)> {
    if node.kind != BINARY {
        return None;
    }
    let lhs = object_of(index, node)?;
    let at = code_after(trivia, lhs.span.end as usize);
    let rest = text.get(at..)?;
    let spelling = OPERATORS
        .iter()
        .find(|spelling| rest.starts_with(**spelling))?;
    Some((at, spelling))
}

/// Appends the operands and operators of `node`, flattened through every
/// operand written with the same `spelling`.
fn flatten(
    index: &SyntaxIndex,
    text: &str,
    trivia: &[Trivia],
    node: IndexNode,
    spelling: &str,
    operands: &mut Vec<IndexNode>,
    operators: &mut Vec<usize>,
) {
    let Some((at, _)) = operator_of(index, text, trivia, node).filter(|&(_, own)| own == spelling)
    else {
        operands.push(node);
        return;
    };
    let mut sides = index.children_of(node);
    sides.sort_by_key(|side| side.span.start);
    let [lhs, rhs] = sides[..] else {
        operands.push(node);
        return;
    };
    flatten(index, text, trivia, lhs, spelling, operands, operators);
    operators.push(at);
    flatten(index, text, trivia, rhs, spelling, operands, operators);
}

/// The `(` and `)` around `expr`, where it is the condition of `holder`, a
/// control structure in [`CONDITIONS`].
fn condition_of(
    text: &str,
    trivia: &[Trivia],
    holder: IndexNode,
    expr: IndexNode,
) -> Option<(usize, usize)> {
    if !CONDITIONS.contains(&holder.kind) {
        return None;
    }
    let bytes = text.as_bytes();
    let open = code_before(trivia, expr.span.start as usize).checked_sub(1)?;
    let close = code_after(trivia, expr.span.end as usize);
    if bytes.get(open) != Some(&b'(') || bytes.get(close) != Some(&b')') {
        return None;
    }
    let keyword = &text[..code_before(trivia, open)];
    (keyword.ends_with("if") || keyword.ends_with("while")).then_some((open, close))
}

/// The chain the operator node `nodes[at]` belongs to, read from its top: each
/// step outwards is an operand written with the same operator.
fn operators_on(
    index: &SyntaxIndex,
    text: &str,
    trivia: &[Trivia],
    nodes: &[IndexNode],
    at: usize,
) -> Option<Operators> {
    let (_, spelling) = operator_of(index, text, trivia, nodes[at])?;
    let mut top = at;
    while let Some(&parent) = nodes.get(top + 1) {
        if operator_of(index, text, trivia, parent).is_none_or(|(_, own)| own != spelling) {
            break;
        }
        top += 1;
    }
    let condition = nodes
        .get(top + 1)
        .and_then(|&holder| condition_of(text, trivia, holder, nodes[top]));
    Operators::of(index, text, trivia, nodes[top], condition)
}

/// What opens the line at `offset` where it is an operator of a broken
/// operator chain, or a comment in front of one.
fn operator_opening(indent: &Indent<'_>, nodes: &[IndexNode], offset: usize) -> Option<String> {
    let (index, text, trivia) = indent.parse();
    let chain = operators_on(index, text, trivia, nodes, 0)?;
    let beside = chain
        .operators
        .iter()
        .any(|&operator| (code_before(trivia, operator)..=operator).contains(&offset));
    (chain.broken && beside).then(|| chain.operator_base(indent))
}

/// What opens the line at `offset` where it is the first operand or the `)`
/// of a broken condition, or a comment in front of one.
fn condition_opening(indent: &Indent<'_>, nodes: &[IndexNode], offset: usize) -> Option<String> {
    let (index, text, trivia) = indent.parse();
    let holder = *nodes.iter().find(|node| CONDITIONS.contains(&node.kind))?;
    let (expr, (open, close)) = index.children_of(holder).into_iter().find_map(|child| {
        condition_of(text, trivia, holder, child)
            .filter(|&(open, close)| open < offset && offset <= close)
            .map(|parens| (child, parens))
    })?;
    let chain = Operators::of(index, text, trivia, expr, Some((open, close)))?;
    if !chain.broken {
        return None;
    }
    if (code_before(trivia, close)..=close).contains(&offset) {
        return Some(indent.opening_of(open));
    }
    (offset <= chain.operands[0].span.start as usize).then(|| chain.operator_base(indent))
}

/// Records the runs every broken operator chain and condition in the code run
/// `from..to` requires. A chain is laid out once, from its first operator, and
/// a condition from its `(`.
fn operator_runs(
    wanted: &mut Vec<(usize, String)>,
    indent: &Indent<'_>,
    from: usize,
    to: usize,
    line_break: &str,
) {
    let (index, text, trivia) = indent.parse();
    for (offset, &byte) in text.as_bytes().iter().enumerate().take(to).skip(from) {
        if !matches!(byte, b'&' | b'|' | b'?' | b'.' | b'(') {
            continue;
        }
        let Ok(pos) = BytePos::try_from(offset) else {
            continue;
        };
        let path = index.at(pos);
        let nodes = path.nodes();
        let Some(&innermost) = nodes.first() else {
            continue;
        };
        if nodes.iter().any(|node| OPAQUE.contains(&node.kind)) {
            continue;
        }
        let chain = if byte == b'(' {
            index.children_of(innermost).into_iter().find_map(|child| {
                let parens = condition_of(text, trivia, innermost, child)
                    .filter(|&(open, _)| open == offset)?;
                Operators::of(index, text, trivia, child, Some(parens))
            })
        } else {
            operators_on(index, text, trivia, nodes, 0).filter(|chain| {
                chain.condition.is_none() && chain.operators.first() == Some(&offset)
            })
        };
        if let Some(chain) = chain.filter(|chain| chain.broken) {
            chain.lay_out(wanted, indent, line_break);
        }
    }
}
