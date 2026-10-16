//! What one keypress expands the selection to, and what the next one does.
//!
//! `textDocument/selectionRange` is the first request in this crate that takes
//! a cursor, and it is the smallest one there will ever be: the answer **is**
//! `SyntaxIndex::at`'s ancestor list, turned from a `Vec` innermost-first into
//! the parent-linked chain the wire carries
//! (`rule:ide/the-index-answers-the-cursor`). Nothing here walks the tree,
//! consults the module, or asks the type phase anything — which is exactly the
//! test `rule:ide/the-request-set-is-closed` admitted this request on.
//!
//! **The chain is the index's, unedited.** Two neighbouring entries can cover
//! the same bytes — an expression statement and the expression that is all of
//! it — and both stay, because the index's vocabulary of nodes is
//! `nvs_syntax::walk`'s and a chain that dropped a link here would disagree
//! with what every other cursor request answers at the same offset. A client
//! that shows the same range twice is showing the tree it is actually
//! navigating.
//!
//! **A caret is between bytes and the index answers about bytes**, which is
//! [`nvs_syntax::SyntaxIndex`]'s own decision and not re-taken here: a cursor
//! just past the end of a node is outside it. So an offset inside no node at
//! all — between two statements, in inline HTML, past the end — answers
//! nothing rather than the whole file, and the caller decides what "nothing"
//! looks like on the wire.
//!
//! **The entry document only**, on [`crate::links`]' terms: the index
//! [`crate::document::Analysed`] carries is the open buffer's, because a
//! required file is read by the graph walk and no cursor is ever in one.

use lsp_types::SelectionRange;
use nvs_diagnostics::{BytePos, PositionEncoding};

use crate::document::Analysed;
use crate::position::range_at;

/// The selection chain at `offset` in the entry document, innermost first.
///
/// `None` when the offset is inside no node. LSP's answer has to stay the same
/// length as the positions it was asked about, so a caller answering a client
/// sends the empty range at that position instead of dropping it — expand
/// selection has nowhere to go, which is a true answer and keeps the array
/// aligned. A `.lspt` case renders the `None` as `none`.
#[must_use]
pub fn at(
    analysed: &Analysed,
    offset: BytePos,
    encoding: PositionEncoding,
) -> Option<SelectionRange> {
    let file = analysed.map.file(analysed.entry);
    let mut chain: Option<SelectionRange> = None;
    // Outermost first, so each link is already built when the one inside it
    // needs it as a parent.
    for node in analysed.index.at(offset).nodes().iter().rev() {
        chain = Some(SelectionRange {
            range: range_at(file, node.span, encoding),
            parent: chain.map(Box::new),
        });
    }
    chain
}
