//! Where the name under the cursor was declared.
//!
//! `textDocument/definition` is the first request in this crate that needs two
//! things at once: [`nvs_syntax::SyntaxIndex`] to say which node the cursor is
//! in, and the type phase's own table to say what the name in that node
//! resolved to (`rule:ide/the-index-answers-the-cursor`). The index alone
//! cannot answer it — a type name is a *property* of the node that writes it
//! and never a node of its own, which is `nvs_syntax::walk`'s decision and the
//! reason nothing here looks for a node called `TypeName`.
//!
//! **The resolution is the checker's, read back rather than redone.**
//! `nvs_types::ExprInfo` already carries the fully-qualified name each `new`,
//! `instanceof` and enum-case access resolved to, with the namespace and the
//! imports of the site that wrote it applied
//! (`rule:classes/names-resolve-case-sensitively`), and
//! [`crate::document::Analysed`] now keeps that table. A server that resolved a
//! written name for itself would be a second implementation of the rule the
//! checker already applies, and it would disagree the first time an import was
//! involved.
//!
//! **A type name, and not yet a member.** The four entries read here each name
//! a class, an interface or an enum, and `nvs_hir::SymbolTable` is where a
//! declaration of one has its span. A method or a property is
//! `nvs_hir::MemberTable`'s, keyed by its class's label rather than by the span
//! the cursor is in, so jumping to one is its own slice rather than another arm
//! of the match below.
//!
//! **An enum case answers its enum.** `Mode::Read` under the cursor jumps to
//! `enum Mode`, because a case is a member and the symbol table holds
//! declarations. That is a true answer to a narrower question than was asked,
//! which is the same trade the member slice above will close.
//!
//! **The whole graph, not the entry alone.** The cursor is always in the open
//! document ([`crate::selection`]'s reasoning), but what it names may be
//! declared in a required file — so the span answered here carries its own
//! `SourceId` and the caller spells whichever file that is, exactly as
//! [`crate::links`] spells a `require`'s target.

use std::path::PathBuf;

use lsp_types::Range;
use nvs_diagnostics::{BytePos, PositionEncoding};
use nvs_hir::QName;
use nvs_types::ExprInfo;

use crate::document::Analysed;
use crate::position::range_at;

/// One declaration, and the file it was written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// The file, as the analysis loaded it: absolute and canonical, on
    /// [`crate::links::RequireLink`]'s terms.
    pub path: PathBuf,
    /// The declared name's own range — not the whole declaration's, because
    /// that is the span `nvs_hir::Symbol` records and it is what an editor
    /// wants to put a caret on.
    pub range: Range,
}

/// Where the name at `offset` in the entry document is declared.
///
/// `None` when the cursor is inside no node, when the node it is in resolved to
/// no name this can follow, when the name is declared nowhere the analysis
/// reached, or when it is declared in a file with no path — a `Core` class is
/// the last of those, and it has no Novis declaration to open.
#[must_use]
pub fn at(analysed: &Analysed, offset: BytePos, encoding: PositionEncoding) -> Option<Declared> {
    let symbol = analysed.module.symbols.get(named_at(analysed, offset)?)?;
    let file = analysed.map.file(symbol.decl_span.file);
    Some(Declared {
        path: file.path()?.to_path_buf(),
        range: range_at(file, symbol.decl_span, encoding),
    })
}

/// The declared name the cursor is on, innermost node first.
///
/// Innermost first because the nodes nest: a cursor on the class name of
/// `new User()` inside `echo (new User())->name;` is inside both, and the
/// nearer answer is the one it is pointing at.
fn named_at(analysed: &Analysed, offset: BytePos) -> Option<&QName> {
    analysed.index.at(offset).nodes().iter().find_map(|node| {
        match analysed.exprs.lookup(node.span)? {
            ExprInfo::New { class, .. } | ExprInfo::InstanceOf { class } => Some(class),
            // The bound, which is the only class this site named — the one
            // allocated is whatever descriptor is in hand, and no compile
            // knows it (`nvs_types::ExprInfo::NewDynamic`).
            ExprInfo::NewDynamic { bound, .. } => Some(bound),
            ExprInfo::EnumCase { enum_, .. } => Some(enum_),
            _ => None,
        }
    })
}
