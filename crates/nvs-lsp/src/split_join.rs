//! *Put on separate lines* and *Join onto one line*, offered inside a list, a
//! call chain or an operator chain
//! (`rule:ide/a-list-splits-onto-lines-and-joins-onto-one`).
//!
//! The edit is [`nvs_fmt::relayout`]'s, so which constructs exist, what breaks one
//! and the layout it ends in are the formatter's rules and nowhere else
//! (`rule:ide/one-server-two-thin-clients`). This module only places the edit
//! in the client's encoding. The action is computed from the entry document's
//! text alone, with no type or module question.
//!
//! # What it spends
//!
//! What [`nvs_fmt::relayout`] spends, once per request whose cursor is inside
//! a list or a chain, dropped with the answer.

use nvs_diagnostics::{BytePos, PositionEncoding, Span};

use crate::document::Analysed;
use crate::position::range_at;
use crate::render::Action;

/// The kind both actions are filed under. Neither is a fix, so neither is
/// `quickfix` or `source.fixAll.nvs`.
pub const KIND: &str = "refactor.rewrite";

/// The split or join offered with the cursor at `start` of the entry document,
/// if a list or a chain is around it.
#[must_use]
pub fn at(analysed: &Analysed, start: BytePos, encoding: PositionEncoding) -> Option<Action> {
    let file = analysed.map.file(analysed.entry);
    let edit = nvs_fmt::relayout(file, start as usize)?;
    let span = Span::new(
        analysed.entry,
        BytePos::try_from(edit.start).ok()?,
        BytePos::try_from(edit.end).ok()?,
    );
    Some(Action {
        title: edit.toggle.title().to_owned(),
        kind: KIND.to_owned(),
        range: range_at(file, span, encoding),
        replacement: edit.replacement,
    })
}
