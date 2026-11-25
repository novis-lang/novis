//! Where the cursor writes a value whose expected type is an enum.
//!
//! Completion offers that enum's cases first at four positions
//! (`rule:ide/completion-is-asked-where-a-spelling-ends`): an arm condition of a
//! `match` over `E` or `?E`, a subscript of an `array<E, T>`, a key of an array
//! literal written where an `array<E, T>` is expected, and an argument at a
//! parameter of type `E`. At an arm and at a literal's key, the cases already
//! written there are left out, so the list is what is still missing.
//!
//! The innermost of a `match`, a subscript, an array literal and a call in the
//! cursor's path decides which of the four it is. Which half of an arm or an
//! element the cursor is in is read off the lexer's tokens between the node's
//! opening bracket and the cursor, at the node's own depth: a `,` starts a
//! condition or a key, and a `=>` starts a body or a value. The document is
//! usually half-written while this is asked, and the tokens are there when the
//! tree is not.
//!
//! The enum of a literal's key is the one the checker recorded for it
//! (`nvs_types::ExprTypeTable::keyed_literal`), because a literal's target is
//! the declaration it is written at, which only the checker resolves.
//!
//! What it spends: one tokenization of the entry document per request that
//! reaches a `match` or a literal, dropped with the answer.

use nvs_diagnostics::{BytePos, Diagnostics, Span};
use nvs_hir::QName;
use nvs_syntax::{IndexNode, NodePath, Token, TokenKind, tokenize};
use nvs_types::{Ty, TypeId};
use rustc_hash::FxHashSet;

use crate::arguments::{CALLS, param_at};
use crate::completion::receiver_ty;
use crate::definition::text_of;
use crate::document::Analysed;

/// The enum expected at the cursor, and what is already written beside it.
pub(crate) struct Expected {
    /// The enum whose cases are offered.
    pub enum_: QName,
    /// Whether `null` is expected too, at an arm of a `match` over `?E`.
    pub nullable: bool,
    /// The case names, and `null`, that an arm or a key already writes.
    pub written: FxHashSet<String>,
}

/// The enum expected where the cursor is, as the module doc says, or `None`.
pub(crate) fn enum_at(analysed: &Analysed, path: &NodePath, offset: BytePos) -> Option<Expected> {
    let node = *path.nodes().iter().find(|node| {
        matches!(node.kind, "Match" | "Index" | "ArrayLiteral") || CALLS.contains(&node.kind)
    })?;
    match node.kind {
        "Match" => at_arm(analysed, node, offset),
        "Index" => at_subscript(analysed, node, offset),
        "ArrayLiteral" => at_key(analysed, node, offset),
        _ => {
            let (call, index) = param_at(analysed, path, offset)?;
            let (enum_, _) = enum_of(analysed, *call.param_tys.get(index)?)?;
            Some(Expected {
                enum_,
                nullable: false,
                written: FxHashSet::default(),
            })
        }
    }
}

/// The enum `ty` is, and whether it is `?E`.
fn enum_of(analysed: &Analysed, ty: TypeId) -> Option<(QName, bool)> {
    let interner = &analysed.interner;
    match interner.get(ty) {
        Ty::Enum(qname, _) => Some((qname.clone(), false)),
        Ty::Union(members) => {
            let mut found = None;
            let mut nullable = false;
            for &member in members {
                match interner.get(member) {
                    Ty::Null => nullable = true,
                    Ty::Enum(qname, _) if found.is_none() => found = Some(qname.clone()),
                    _ => return None,
                }
            }
            found.map(|qname| (qname, nullable))
        }
        _ => None,
    }
}

/// An arm condition of a `match` over an enum.
fn at_arm(analysed: &Analysed, node: IndexNode, offset: BytePos) -> Option<Expected> {
    let subject = *analysed.index.children_of(node).first()?;
    if offset <= subject.span.end {
        return None;
    }
    let (enum_, nullable) = enum_of(analysed, receiver_ty(analysed, subject.span, offset)?)?;
    let tokens = tokens_in(analysed, node.span);
    let open = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LBrace && token.span.start >= subject.span.end)?;
    let (at_start, written) = halves(analysed, &tokens[open + 1..], offset)?;
    at_start.then_some(Expected {
        enum_,
        nullable,
        written,
    })
}

/// A key of an array literal written where an `array<E, T>` is expected.
fn at_key(analysed: &Analysed, node: IndexNode, offset: BytePos) -> Option<Expected> {
    let (enum_, _) = enum_of(analysed, analysed.exprs.keyed_literal(node.span)?)?;
    let tokens = tokens_in(analysed, node.span);
    let open = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LBracket)?;
    let (at_start, written) = halves(analysed, &tokens[open + 1..], offset)?;
    at_start.then_some(Expected {
        enum_,
        nullable: false,
        written,
    })
}

/// A subscript of an `array<E, T>`.
fn at_subscript(analysed: &Analysed, node: IndexNode, offset: BytePos) -> Option<Expected> {
    let base = *analysed.index.children_of(node).first()?;
    if offset <= base.span.end {
        return None;
    }
    let key = analysed
        .interner
        .array_enum_key(receiver_ty(analysed, base.span, offset)?)?;
    let (enum_, _) = enum_of(analysed, key)?;
    Some(Expected {
        enum_,
        nullable: false,
        written: FxHashSet::default(),
    })
}

/// The lexer's tokens inside `span`.
fn tokens_in(analysed: &Analysed, span: Span) -> Vec<Token> {
    let file = analysed.map.file(analysed.entry);
    tokenize(file, &mut Diagnostics::new())
        .into_iter()
        .filter(|token| token.span.start >= span.start && token.span.end <= span.end)
        .collect()
}

/// Whether the cursor is in the first half of an arm or an element, before
/// its `=>`, and the case names and `null` that first halves write.
///
/// `tokens` starts after the opening bracket. A `,` at depth zero starts a
/// first half and a `=>` a second one. A name being written at the cursor is
/// not counted as written. `None` where the cursor is past the closing
/// bracket.
fn halves(
    analysed: &Analysed,
    tokens: &[Token],
    offset: BytePos,
) -> Option<(bool, FxHashSet<String>)> {
    use TokenKind as T;
    let file = analysed.map.file(analysed.entry);
    let mut written = FxHashSet::default();
    let mut depth = 0usize;
    let mut first = true;
    let mut at_cursor = None;
    for (i, token) in tokens.iter().enumerate() {
        if at_cursor.is_none() && token.span.end > offset {
            at_cursor = Some(first);
        }
        match token.kind {
            T::LParen | T::LBracket | T::LBrace => depth += 1,
            T::RParen | T::RBracket | T::RBrace if depth == 0 => {
                if at_cursor.is_none() && token.span.start < offset {
                    return None;
                }
                break;
            }
            T::RParen | T::RBracket | T::RBrace => depth -= 1,
            T::Comma if depth == 0 => first = true,
            T::FatArrow if depth == 0 => first = false,
            _ if depth == 0 && first && token.span.end != offset => {
                let text = text_of(file, token.span);
                let after_colons = i
                    .checked_sub(1)
                    .is_some_and(|before| tokens[before].kind == T::DoubleColon);
                if text.eq_ignore_ascii_case("null") {
                    written.insert("null".to_owned());
                } else if after_colons {
                    written.insert(text.to_owned());
                }
            }
            _ => {}
        }
    }
    Some((at_cursor.unwrap_or(first), written))
}
