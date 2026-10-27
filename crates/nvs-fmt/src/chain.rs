//! A `->` call chain is on one line or has one call per line, and the author's
//! line break before any of its arrows is what chooses
//! (`rule:tooling/fmt-a-broken-call-chain-is-one-call-per-line`). A chain of
//! any binary operator is on one line or has one operand per line, operator
//! first, a `.` chain keeps the lines its author wrote, and a broken
//! control-structure condition puts its parentheses on lines of their own
//! (`rule:tooling/fmt-a-broken-operator-chain-is-one-operand-per-line`,
//! `rule:tooling/fmt-every-binary-operator-breaks-operator-first`).
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
//! operand, and every one of them makes a chain. [`OPERATORS`] gives each
//! operator its precedence level, so `+` and `-` are one chain. The chain is
//! the outermost such node and every operand below it written with an operator
//! of the same level, flattened in source order. A parenthesised operand is a
//! `Paren` node, so the flattening never reaches inside one. The parser has
//! already put the lowest-precedence operator at the top of a mixed run, so the
//! chain is the run at the lowest precedence, and an operand made of other
//! operators is a chain of its own, judged on its own. `|>` leaves no node
//! behind, so it is no chain here.
//!
//! An operator chain is broken when a line break sits between two of its
//! operands, on either side of the operator. Each operator then opens a line
//! one level in from the line the first operand starts on, and one space
//! follows it. Where that first operand starts a line of a broken chain around
//! it, the inner chain's lines are one level further in, which is what shows
//! the reader that it binds tighter.
//!
//! A broken `.` chain is the exception: only an operator with a line break on
//! one side of it opens a line, and operands written on one line stay there.
//! A `.` its author ended a line with moves to the start of the next one.
//!
//! # A ternary
//!
//! A `Ternary` is a chain of its own whatever holds it, with its condition,
//! middle and last operand as operands and `?` and `:` as operators
//! (`rule:tooling/fmt-a-broken-ternary-has-a-line-per-branch`). A short
//! ternary has two operands and one operator, `?:`. A line break on either
//! side of either operator breaks it, and it is then laid out as an operator
//! chain is, so a broken chain in a branch is one level further in.
//!
//! # In a list
//!
//! [`is_broken`] tells [`crate::list`] which items break the list holding
//! them (`rule:tooling/fmt-a-broken-item-breaks-its-list`). A chain whose
//! first code is such an item measures its lines from the item's own line,
//! which the list opens even where its author did not.
//!
//! # A control header
//!
//! Every control header has the condition layout
//! (`rule:tooling/fmt-every-control-header-breaks-like-a-condition`). The
//! condition of an `if`, `elseif`, `while`, `do … while`, `switch` or `match`
//! is the child expression written between a `(` that follows the keyword and
//! the `)` that follows the expression. Its top-level chain is the
//! expression's operator chain, or the expression alone. A line break after the
//! `(`, between two of those operands, before the `)`, or inside an operator
//! chain or ternary at any depth below them that is not inside brackets of its
//! own breaks the condition. A broken condition's `(` ends its line, each
//! operand starts a line one level in from the line the `(` is on, operator
//! first after the first, and the `)` starts a line at that line's own depth.
//! [`crate::brace`] then writes ` {` after it.
//!
//! A `foreach` or `for` header is a [`Header`] of parts instead: a `foreach`'s
//! expression and its `as` with the bindings after it, or each clause of a
//! `for`. A line break in front of a part or the `)`, or inside an operator
//! chain of the header, breaks it, and each part then starts a line one level
//! in, as a condition's operands do. A chain in a part is laid out as its own,
//! one level further in.
//!
//! [`seams_on`] names every place a line break sits at a chain's or a header's
//! own level, which is what the editor actions in [`crate::split_join`] add or
//! remove.
//!
//! # The two layout lists
//!
//! [`LAID_OUT`] names every syntax node kind with a broken layout and the rule
//! that gives it, and [`AUTHORS_BREAKS`] every kind whose line breaks are the
//! author's (`rule:tooling/fmt-every-construct-has-a-broken-layout`).
//! `tests/layout_coverage.rs` fails on a kind that is on neither.
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
use crate::list::{self, code_before};
use crate::space::code_after;

/// The productions that are one link of a chain.
const LINKS: &[&str] = &["MethodCall", "PropertyAccess"];

/// The link that is a call.
const CALL: &str = "MethodCall";

/// The production that writes an operator between two operands.
const BINARY: &str = "Binary";

/// Every operator a `Binary` node writes, with its precedence level as
/// `crates/nvs-syntax/src/parser/expr.rs` parses it, lowest first. Operators
/// of one level make one chain. A spelling comes before every spelling it is
/// a prefix of, so the first match is the operator written.
const OPERATORS: &[(&str, u8)] = &[
    ("??", 0),
    ("||", 1),
    ("&&", 2),
    ("<=>", 6),
    ("==", 6),
    ("!=", 6),
    ("<=", 7),
    (">=", 7),
    ("<<", 9),
    (">>", 9),
    ("**", 12),
    ("|", 3),
    ("^", 4),
    ("&", 5),
    ("<", 7),
    (">", 7),
    (".", CONCAT),
    ("+", 10),
    ("-", 10),
    ("*", 11),
    ("/", 11),
    ("%", 11),
];

/// The level of `.`, whose chain keeps the lines its author wrote.
const CONCAT: u8 = 8;

/// The production that writes `?` and `:` between its operands.
const TERNARY: &str = "Ternary";

/// The control structures whose header is one expression, laid out as a
/// condition.
const CONDITIONS: &[&str] = &["If", "While", "DoWhile", "Switch", "Match"];

/// The keywords that write a header [`condition_of`] reads.
const CONDITION_KEYWORDS: &[&str] = &["if", "while", "switch", "match"];

/// The control structures whose header has parts of its own, with the keyword
/// each is written with. Each part opens a line where the header is broken.
const HEADERS: &[(&str, &str)] = &[("Foreach", "foreach"), ("For", "for")];

/// The rule that lays out a list.
const LIST_RULE: &str = "rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line";

/// The rule that lays out a `->` call chain.
const CALL_RULE: &str = "rule:tooling/fmt-a-broken-call-chain-is-one-call-per-line";

/// The rule that lays out a binary operator chain.
const OPERATOR_RULE: &str = "rule:tooling/fmt-every-binary-operator-breaks-operator-first";

/// The rule that lays out a ternary.
const TERNARY_RULE: &str = "rule:tooling/fmt-a-broken-ternary-has-a-line-per-branch";

/// The rule that lays out a control header.
const HEADER_RULE: &str = "rule:tooling/fmt-every-control-header-breaks-like-a-condition";

/// The rule that lays out a `match` arm list.
const ARMS_RULE: &str = "rule:tooling/fmt-novis-constructs";

/// Every syntax node kind with a broken layout, spelled as
/// `crates/nvs-syntax/src/walk.rs` spells it, with the rule that gives the
/// layout. A kind with two layouts is named once for each
/// (`rule:tooling/fmt-every-construct-has-a-broken-layout`).
pub const LAID_OUT: &[(&str, &str)] = &[
    ("AnonObject", LIST_RULE),
    ("ArrayLiteral", LIST_RULE),
    ("Attribute", LIST_RULE),
    ("Binary", OPERATOR_RULE),
    ("Call", LIST_RULE),
    ("DoWhile", HEADER_RULE),
    ("EnumDecl", LIST_RULE),
    ("Fn", LIST_RULE),
    ("For", HEADER_RULE),
    ("Foreach", HEADER_RULE),
    ("Function", LIST_RULE),
    ("If", HEADER_RULE),
    ("Match", HEADER_RULE),
    ("Match", ARMS_RULE),
    ("Method", LIST_RULE),
    ("MethodCall", CALL_RULE),
    ("MethodCall", LIST_RULE),
    ("New", LIST_RULE),
    ("PropertyAccess", CALL_RULE),
    ("ShapeType", LIST_RULE),
    ("StaticCall", LIST_RULE),
    ("Switch", HEADER_RULE),
    ("Ternary", TERNARY_RULE),
    ("While", HEADER_RULE),
];

/// Every syntax node kind whose line breaks are the author's: a construct
/// with no broken layout yet, and a kind with no line break of its own to lay
/// out. A line that continues one starts one level in from the line it starts
/// on (`rule:tooling/fmt-every-construct-has-a-broken-layout`).
pub const AUTHORS_BREAKS: &[&str] = &[
    "Assign",
    "AutoloadDecl",
    "Await",
    "Block",
    "Bool",
    "Break",
    "Case",
    "Catch",
    "ClassConstAccess",
    "ClassDecl",
    "ClassNameConst",
    "Clone",
    "Const",
    "ConstFetch",
    "Continue",
    "Conversion",
    "Destructure",
    "Duration",
    "Echo",
    "Empty",
    "EnumCase",
    "Error",
    "Exit",
    "Expr",
    "File",
    "Float",
    "Global",
    "Goto",
    "HtmlTemplate",
    "Index",
    "InlineHtml",
    "Int",
    "InterfaceDecl",
    "Interpolated",
    "IsInit",
    "Isset",
    "LocalDecl",
    "NamespaceDecl",
    "Null",
    "Paren",
    "ParentExpr",
    "PostIncDec",
    "PreIncDec",
    "Print",
    "Property",
    "PropertyHook",
    "Require",
    "Return",
    "SelfExpr",
    "SpawnScript",
    "StaticExpr",
    "StaticLocal",
    "StaticPropertyAccess",
    "Str",
    "Throw",
    "Try",
    "TypeAliasDecl",
    "TypeTest",
    "Unary",
    "Unset",
    "UseDecl",
    "Variable",
    "Yield",
    "YieldFrom",
];

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

/// Whether `item`, a list's item, is a broken call chain, operator chain or
/// ternary at its own top, which breaks the list that holds it.
pub(crate) fn is_broken(
    index: &SyntaxIndex,
    text: &str,
    trivia: &[Trivia],
    item: IndexNode,
) -> bool {
    if LINKS.contains(&item.kind) {
        return Chain::of(index, text, trivia, item).is_some_and(|chain| chain.broken);
    }
    Operators::of(index, text, trivia, item, None).is_some_and(|chain| chain.broken)
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
        .or_else(|| header_opening(indent, nodes, offset))
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
    (chain.broken && beside).then(|| line_opening(indent, chain.receiver) + UNIT)
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
        let base = line_opening(indent, chain.receiver);
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
    /// The first byte and one past the last byte of every operator between two
    /// operands. A short ternary's `?:` is one operator.
    operators: Vec<(usize, usize)>,
    /// Whether this is a `.` chain, which breaks only where its author did.
    keeps_lines: bool,
    /// The `(` and the `)` around a condition, where this is one.
    condition: Option<(usize, usize)>,
    /// Whether a line break sits at the chain's own level.
    broken: bool,
}

impl Operators {
    /// The chain `top` writes, inside the parentheses `condition` where it is
    /// a condition. [`None`] for an expression that is neither a chain, a
    /// ternary nor a condition.
    fn of(
        index: &SyntaxIndex,
        text: &str,
        trivia: &[Trivia],
        top: IndexNode,
        condition: Option<(usize, usize)>,
    ) -> Option<Self> {
        let mut operands = Vec::new();
        let mut operators = Vec::new();
        let level = match operator_of(index, text, trivia, top) {
            _ if top.kind == TERNARY => {
                operands = index.children_of(top);
                operands.sort_by_key(|operand| operand.span.start);
                operators = ternary_operators(text, trivia, &operands)?;
                None
            }
            Some((_, _, level)) => {
                flatten(
                    index,
                    text,
                    trivia,
                    top,
                    level,
                    &mut operands,
                    &mut operators,
                );
                Some(level)
            }
            None if condition.is_some() => {
                operands.push(top);
                None
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
                    || operands
                        .iter()
                        .any(|&operand| nested_broken(index, text, trivia, operand))
            });
        Some(Self {
            operands,
            operators,
            keeps_lines: level == Some(CONCAT),
            condition,
            broken,
        })
    }

    /// What opens an operator's line: one level in from the line the `(` of a
    /// condition is on, or from the line the first operand starts on.
    fn operator_base(&self, indent: &Indent<'_>) -> String {
        match self.condition {
            Some((open, _)) => indent.opening_of(open) + UNIT,
            None => operand_opening(indent, self.operands[0].span.start as usize) + UNIT,
        }
    }

    /// Whether the operator `operators[at]` of this broken chain starts a line:
    /// every one does, except in a `.` chain, where only one with a line break
    /// on either side of it does.
    fn opens_line(&self, text: &str, trivia: &[Trivia], at: usize) -> bool {
        let (operator, end) = self.operators[at];
        let operand = self.operands[at + 1].span.start as usize;
        !self.keeps_lines
            || text[code_before(trivia, operator)..operator].contains('\n')
            || text[end..operand].contains('\n')
    }

    /// The seams in front of and after each operator, in source order.
    fn own_seams(&self, trivia: &[Trivia]) -> Vec<(SeamKind, usize, usize)> {
        let mut seams = Vec::with_capacity(self.operators.len() * 2);
        for (&(operator, end), operand) in self.operators.iter().zip(&self.operands[1..]) {
            seams.push((SeamKind::Operator, code_before(trivia, operator), operator));
            seams.push((SeamKind::Operand, end, operand.span.start as usize));
        }
        seams
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
        for (at, (&(operator, end), operand)) in
            self.operators.iter().zip(&self.operands[1..]).enumerate()
        {
            if !self.opens_line(text, trivia, at) {
                continue;
            }
            if !text[code_before(trivia, operator)..operator].contains('\n') {
                wanted.push((operator, format!("{line_break}{base}")));
            }
            let start = operand.span.start as usize;
            let after = &text[end..start];
            if after != " " && after.trim().is_empty() {
                wanted.push((start, " ".to_owned()));
            }
        }
    }
}

/// The first byte of the operator `node` writes, its spelling and its level,
/// where `node` is a `Binary`.
fn operator_of(
    index: &SyntaxIndex,
    text: &str,
    trivia: &[Trivia],
    node: IndexNode,
) -> Option<(usize, &'static str, u8)> {
    if node.kind != BINARY {
        return None;
    }
    let lhs = object_of(index, node)?;
    let at = code_after(trivia, lhs.span.end as usize);
    let rest = text.get(at..)?;
    let &(spelling, level) = OPERATORS
        .iter()
        .find(|(spelling, _)| rest.starts_with(spelling))?;
    Some((at, spelling, level))
}

/// Appends the operands and operators of `node`, flattened through every
/// operand written with an operator of the same `level`.
fn flatten(
    index: &SyntaxIndex,
    text: &str,
    trivia: &[Trivia],
    node: IndexNode,
    level: u8,
    operands: &mut Vec<IndexNode>,
    operators: &mut Vec<(usize, usize)>,
) {
    let Some((at, spelling, _)) =
        operator_of(index, text, trivia, node).filter(|&(_, _, own)| own == level)
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
    flatten(index, text, trivia, lhs, level, operands, operators);
    operators.push((at, at + spelling.len()));
    flatten(index, text, trivia, rhs, level, operands, operators);
}

/// The operators of a ternary whose operands are `operands`, in source order:
/// `?` and `:` where it has a middle operand, and `?:` where it has none. A
/// `?:` its author wrote with a run inside it is still one operator.
fn ternary_operators(
    text: &str,
    trivia: &[Trivia],
    operands: &[IndexNode],
) -> Option<Vec<(usize, usize)>> {
    let bytes = text.as_bytes();
    let question = code_after(trivia, operands.first()?.span.end as usize);
    if bytes.get(question) != Some(&b'?') {
        return None;
    }
    match *operands {
        [_, middle, _] => {
            let colon = code_after(trivia, middle.span.end as usize);
            (bytes.get(colon) == Some(&b':'))
                .then(|| vec![(question, question + 1), (colon, colon + 1)])
        }
        [_, _] => {
            let colon = code_after(trivia, question + 1);
            (bytes.get(colon) == Some(&b':')).then(|| vec![(question, colon + 1)])
        }
        _ => None,
    }
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
    CONDITION_KEYWORDS
        .iter()
        .any(|word| keyword.ends_with(word))
        .then_some((open, close))
}

/// Whether an operator chain or a ternary in `node`, at any depth outside
/// brackets of its own, is broken.
fn nested_broken(index: &SyntaxIndex, text: &str, trivia: &[Trivia], node: IndexNode) -> bool {
    Operators::of(index, text, trivia, node, None).is_some_and(|chain| {
        chain.broken
            || chain
                .operands
                .iter()
                .any(|&operand| nested_broken(index, text, trivia, operand))
    })
}

/// The header of a `foreach` or a `for`, read from the lossless tree.
struct Header {
    /// The header's `(`.
    open: usize,
    /// The header's `)`.
    close: usize,
    /// The first byte of each part: a `foreach`'s expression and its `as`, or
    /// each clause of a `for` that is not empty at the end.
    parts: Vec<usize>,
    /// Whether a line break sits after the `(`, before a part or the `)`, or
    /// inside an operator chain of the header.
    broken: bool,
    /// The nodes written between the parentheses.
    inside: Vec<IndexNode>,
}

impl Header {
    /// The header `holder` writes, where it is a structure in [`HEADERS`].
    fn of(index: &SyntaxIndex, text: &str, trivia: &[Trivia], holder: IndexNode) -> Option<Self> {
        let &(_, keyword) = HEADERS.iter().find(|&&(kind, _)| kind == holder.kind)?;
        let start = holder.span.start as usize;
        if !text.get(start..)?.starts_with(keyword) {
            return None;
        }
        let bytes = text.as_bytes();
        let open = code_after(trivia, start + keyword.len());
        let mut children = index.children_of(holder);
        children.sort_by_key(|child| child.span.start);
        let body = children.pop()?;
        let close = code_before(trivia, body.span.start as usize).checked_sub(1)?;
        if bytes.get(open) != Some(&b'(') || bytes.get(close) != Some(&b')') {
            return None;
        }
        let mut parts = vec![code_after(trivia, open + 1)];
        if keyword == "foreach" {
            let subject = children.first()?;
            let as_ = code_after(trivia, subject.span.end as usize);
            if !text.get(as_..)?.starts_with("as") {
                return None;
            }
            parts.push(as_);
        } else {
            // A `;` is between two clauses, never inside one, save the one a
            // declaration ends its own span with.
            let inside = |at: usize| {
                children
                    .iter()
                    .any(|child| (child.span.start as usize..child.span.end as usize).contains(&at))
            };
            let ends = children
                .iter()
                .map(|child| child.span.end as usize - 1)
                .filter(|&at| bytes.get(at) == Some(&b';'));
            let between = (open + 1..close)
                .filter(|&at| bytes[at] == b';' && !inside(at) && !commented(trivia, at));
            let mut semicolons: Vec<usize> = ends.chain(between).collect();
            semicolons.sort_unstable();
            parts.extend(
                semicolons
                    .iter()
                    .map(|&semicolon| code_after(trivia, semicolon + 1))
                    .filter(|&part| part != close),
            );
        }
        let breaks = |at: usize| text[code_before(trivia, at)..at].contains('\n');
        let broken = parts.iter().chain([&close]).any(|&at| breaks(at))
            || children
                .iter()
                .any(|&child| nested_broken(index, text, trivia, child));
        Some(Self {
            open,
            close,
            parts,
            broken,
            inside: children,
        })
    }

    /// The seams of this header: after the `(`, in front of each later part,
    /// and in front of the `)`. A broken header's seams include those of every
    /// broken chain inside it, so a join puts the whole header on one line.
    fn seams(&self, index: &SyntaxIndex, text: &str, trivia: &[Trivia]) -> Seams {
        let mut seams = vec![(SeamKind::Open, self.open + 1, self.parts[0])];
        seams.extend(
            self.parts[1..]
                .iter()
                .map(|&part| (SeamKind::Operator, code_before(trivia, part), part)),
        );
        seams.push((SeamKind::Close, code_before(trivia, self.close), self.close));
        if self.broken {
            for &node in &self.inside {
                nested_seams(index, text, trivia, node, &mut seams);
            }
            seams.sort_unstable_by_key(|&(_, from, _)| from);
        }
        Seams {
            anchor: self.open,
            end: self.close + 1,
            broken: self.broken,
            seams,
        }
    }

    /// The broken header whose parentheses hold `offset`, of the innermost
    /// structure in [`HEADERS`] on `nodes`, a path innermost first.
    fn around(indent: &Indent<'_>, nodes: &[IndexNode], offset: usize) -> Option<Self> {
        let (index, text, trivia) = indent.parse();
        nodes
            .iter()
            .filter_map(|&node| Self::of(index, text, trivia, node))
            .find(|header| header.open < offset && offset <= header.close)
            .filter(|header| header.broken)
    }

    /// Records the runs this broken header requires: each part and the `)`
    /// open a line.
    fn lay_out(&self, wanted: &mut Vec<(usize, String)>, indent: &Indent<'_>, line_break: &str) {
        let (_, text, trivia) = indent.parse();
        let closing = indent.opening_of(self.open);
        let base = closing.clone() + UNIT;
        for &part in &self.parts {
            if !text[code_before(trivia, part)..part].contains('\n') {
                wanted.push((part, format!("{line_break}{base}")));
            }
        }
        if !text[code_before(trivia, self.close)..self.close].contains('\n') {
            wanted.push((self.close, format!("{line_break}{closing}")));
        }
    }
}

/// What opens the line at `offset` where it is a part or the `)` of a broken
/// `foreach` or `for` header, or a comment in front of one.
fn header_opening(indent: &Indent<'_>, nodes: &[IndexNode], offset: usize) -> Option<String> {
    let (_, _, trivia) = indent.parse();
    let header = Header::around(indent, nodes, offset)?;
    let beside = |at: usize| (code_before(trivia, at)..=at).contains(&offset);
    if beside(header.close) {
        return Some(indent.opening_of(header.open));
    }
    header
        .parts
        .iter()
        .any(|&part| beside(part))
        .then(|| indent.opening_of(header.open) + UNIT)
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
    let mut top = at;
    // A ternary is a chain of its own, whatever holds it.
    if nodes[at].kind != TERNARY {
        let (_, _, level) = operator_of(index, text, trivia, nodes[at])?;
        while let Some(&parent) = nodes.get(top + 1) {
            if operator_of(index, text, trivia, parent).is_none_or(|(_, _, own)| own != level) {
                break;
            }
            top += 1;
        }
    }
    let condition = nodes
        .get(top + 1)
        .and_then(|&holder| condition_of(text, trivia, holder, nodes[top]));
    Operators::of(index, text, trivia, nodes[top], condition)
}

/// One place a line break sits at a call chain's or an operator chain's own
/// level, as [`crate::split_join`] reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeamKind {
    /// In front of an arrow, where a split opens a line one level in.
    Arrow,
    /// In front of an operator, where a split opens a line one level in.
    Operator,
    /// After an operator, which is one space in both layouts.
    Operand,
    /// After a condition's `(`, where a split opens a line one level in.
    Open,
    /// In front of a condition's `)`, where a split opens a line at the
    /// depth of the `(` line.
    Close,
}

/// The seams of one call chain or operator chain, in source order.
pub(crate) struct Seams {
    /// The first byte of the code whose line every opened line is indented
    /// from: the receiver, the first operand, or a condition's `(`.
    pub(crate) anchor: usize,
    /// One past the chain's last byte, a condition's `)` included.
    pub(crate) end: usize,
    /// Whether a line break sits at the chain's own level.
    pub(crate) broken: bool,
    /// Each seam's kind and the trivia `from..to` it covers.
    pub(crate) seams: Vec<(SeamKind, usize, usize)>,
}

/// The seams of the call chain or operator chain the node `nodes[at]`
/// belongs to, where `nodes` is a path innermost first, or [`None`] where
/// that node is in no chain.
pub(crate) fn seams_on(
    index: &SyntaxIndex,
    text: &str,
    trivia: &[Trivia],
    nodes: &[IndexNode],
    at: usize,
) -> Option<Seams> {
    if LINKS.contains(&nodes[at].kind) {
        let top = outermost(index, nodes, at);
        let chain = Chain::of(index, text, trivia, top)?;
        let seams = chain
            .arrows
            .iter()
            .map(|&arrow| (SeamKind::Arrow, code_before(trivia, arrow), arrow))
            .collect();
        return Some(Seams {
            anchor: chain.receiver,
            end: top.span.end as usize,
            broken: chain.broken,
            seams,
        });
    }
    if nodes.iter().any(|node| OPAQUE.contains(&node.kind)) {
        return None;
    }
    if let Some(header) = Header::of(index, text, trivia, nodes[at]) {
        return Some(header.seams(index, text, trivia));
    }
    let chain = operators_on(index, text, trivia, nodes, at)?;
    let first = chain.operands[0].span.start as usize;
    let mut seams = Vec::new();
    if let Some((open, _)) = chain.condition {
        seams.push((SeamKind::Open, open + 1, first));
    }
    seams.extend(chain.own_seams(trivia));
    let last = chain.operands.last()?.span.end as usize;
    if let Some((_, close)) = chain.condition {
        seams.push((SeamKind::Close, code_before(trivia, close), close));
        // A broken condition is joined whole, its nested chains with it.
        if chain.broken {
            for &operand in &chain.operands {
                nested_seams(index, text, trivia, operand, &mut seams);
            }
            seams.sort_unstable_by_key(|&(_, from, _)| from);
        }
    }
    Some(Seams {
        anchor: chain.condition.map_or(first, |(open, _)| open),
        end: chain.condition.map_or(last, |(_, close)| close + 1),
        broken: chain.broken,
        seams,
    })
}

/// Appends the seams of every broken operator chain or ternary in `node`, at
/// any depth outside brackets of its own, as [`nested_broken`] finds them.
fn nested_seams(
    index: &SyntaxIndex,
    text: &str,
    trivia: &[Trivia],
    node: IndexNode,
    seams: &mut Vec<(SeamKind, usize, usize)>,
) {
    let Some(chain) = Operators::of(index, text, trivia, node, None) else {
        return;
    };
    if chain.broken {
        seams.extend(chain.own_seams(trivia));
    }
    for &operand in &chain.operands {
        nested_seams(index, text, trivia, operand, seams);
    }
}

/// What opens the line at `offset` where it is an operator of a broken
/// operator chain, or a comment in front of one.
fn operator_opening(indent: &Indent<'_>, nodes: &[IndexNode], offset: usize) -> Option<String> {
    let (index, text, trivia) = indent.parse();
    let chain = operators_on(index, text, trivia, nodes, 0)?;
    let beside = chain
        .operators
        .iter()
        .any(|&(operator, _)| (code_before(trivia, operator)..=operator).contains(&offset));
    (chain.broken && beside).then(|| chain.operator_base(indent))
}

/// What opens the line the operand starting at `start` is on. Where it follows
/// an operator that starts a line of a broken chain, or the `(` of a broken
/// condition, that line is the chain's, and a line the author did not break
/// there is placed the same way.
fn operand_opening(indent: &Indent<'_>, start: usize) -> String {
    let (index, text, trivia) = indent.parse();
    let before = code_before(trivia, start);
    let Ok(pos) = BytePos::try_from(start) else {
        return indent.opening_of(start);
    };
    let path = index.at(pos);
    let nodes = path.nodes();
    for at in 0..nodes.len() {
        let chain = operators_on(index, text, trivia, nodes, at).filter(|chain| chain.broken);
        let follows = chain.as_ref().and_then(|chain| {
            let at = chain.operators.iter().position(|&(_, end)| end == before)?;
            chain.opens_line(text, trivia, at).then_some(chain)
        });
        if let Some(chain) = follows {
            return chain.operator_base(indent);
        }
        let Some(&holder) = nodes.get(at + 1) else {
            continue;
        };
        let condition = condition_of(text, trivia, holder, nodes[at])
            .filter(|&(open, _)| open + 1 == before)
            .and_then(|parens| Operators::of(index, text, trivia, nodes[at], Some(parens)));
        if let Some(chain) = condition.filter(|chain| chain.broken) {
            return chain.operator_base(indent);
        }
        let header = Header::of(index, text, trivia, nodes[at]).filter(|header| header.broken);
        if let Some(header) = header.filter(|header| header.parts.contains(&start)) {
            return indent.opening_of(header.open) + UNIT;
        }
    }
    line_opening(indent, start)
}

/// What opens the line the code at `start` is on. Where that code is an item
/// of a broken list, the item opens a line of its own, so that line is the
/// list's, even where its author wrote the item beside the opener.
fn line_opening(indent: &Indent<'_>, start: usize) -> String {
    let (index, _, _) = indent.parse();
    BytePos::try_from(start)
        .ok()
        .and_then(|pos| list::opening(indent, index.at(pos).nodes(), start))
        .unwrap_or_else(|| indent.opening_of(start))
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
        if !matches!(
            byte,
            b'&' | b'|'
                | b'?'
                | b'.'
                | b'('
                | b'^'
                | b'='
                | b'!'
                | b'<'
                | b'>'
                | b'+'
                | b'-'
                | b'*'
                | b'/'
                | b'%'
        ) {
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
        if byte == b'(' {
            let header = Header::of(index, text, trivia, innermost)
                .filter(|header| header.open == offset && header.broken);
            if let Some(header) = header {
                header.lay_out(wanted, indent, line_break);
                continue;
            }
        }
        let chain = if byte == b'(' {
            index.children_of(innermost).into_iter().find_map(|child| {
                let parens = condition_of(text, trivia, innermost, child)
                    .filter(|&(open, _)| open == offset)?;
                Operators::of(index, text, trivia, child, Some(parens))
            })
        } else {
            operators_on(index, text, trivia, nodes, 0).filter(|chain| {
                chain.condition.is_none()
                    && chain.operators.first().map(|&(operator, _)| operator) == Some(offset)
            })
        };
        if let Some(chain) = chain.filter(|chain| chain.broken) {
            chain.lay_out(wanted, indent, line_break);
        }
    }
}
