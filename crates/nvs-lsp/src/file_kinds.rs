//! What each file in the workspace declares, for an editor's file list.
//!
//! `nvs/fileKinds` is the seventh request of Novis's own
//! (`rule:ide/the-request-set-is-closed`), and it is
//! `rule:ide/a-file-shows-what-it-declares`: every file the index holds that
//! declares exactly one class, interface, enum or `type` alias and nothing
//! else, with that kind. The editor shows the kind beside the file's name.
//!
//! **The answer is the index's, never a second walk.** Which files have that
//! shape is `nvs_hir::autoload::sole_declaration`, the scan the compiler holds
//! an autoloaded file to, run once per file where the index already reads it
//! ([`crate::SymbolIndex::sole_kinds`]). A file reached by `require` that
//! happens to have the same shape is in the list too, because it declares one
//! thing just as plainly.
//!
//! The request takes no parameters and answers the whole list. A client asks
//! again whenever the index may have changed — at start, after a workspace
//! pass, and after an open document changes — and each answer costs one pass
//! over the index's file map, with no analysis.

use serde::Serialize;

use crate::document::uri_of;
use crate::index::{DeclKind, SymbolIndex};

/// The method this request is asked under.
///
/// Namespaced for [`crate::redactions::METHOD`]'s reason: it is not LSP's.
pub const METHOD: &str = "nvs/fileKinds";

/// One file and the kind of the one type it declares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileKind {
    /// The file, as a `file:` URI.
    pub uri: String,
    /// `class`, `interface`, `enum` or `type`.
    pub kind: &'static str,
}

/// Every file in `index` that declares exactly one type, in path order.
#[must_use]
pub fn answer(index: &SymbolIndex) -> Vec<FileKind> {
    index
        .sole_kinds()
        .filter_map(|(path, kind)| {
            Some(FileKind {
                uri: uri_of(path)?.as_str().to_owned(),
                kind: word(kind)?,
            })
        })
        .collect()
}

/// The word the wire carries for a type-level kind, and `None` for a member,
/// which no file declares on its own.
const fn word(kind: DeclKind) -> Option<&'static str> {
    match kind {
        DeclKind::Class => Some("class"),
        DeclKind::Interface => Some("interface"),
        DeclKind::Enum => Some("enum"),
        DeclKind::TypeAlias => Some("type"),
        DeclKind::Method | DeclKind::Property | DeclKind::Const | DeclKind::EnumCase => None,
    }
}
