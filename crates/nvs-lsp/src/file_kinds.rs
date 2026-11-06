//! What each file in the workspace declares, for an editor's file list.
//!
//! `nvs/fileKinds` is the seventh request of Novis's own
//! (`rule:ide/the-request-set-is-closed`), and it is
//! `rule:ide/a-file-shows-what-it-declares`: every file the index holds that
//! declares exactly one class, interface, enum or `type` alias and nothing
//! else, with that kind, and every file with a route handler in it, with its
//! routes. The editor shows the kind, or that the file serves routes, beside
//! the file's name.
//!
//! **The answer is the index's, never a second walk.** Which files have that
//! shape is `nvs_hir::autoload::sole_declaration`, the scan the compiler holds
//! an autoloaded file to, run once per file where the index already reads it
//! ([`crate::SymbolIndex::sole_kinds`]). A file reached by `require` that
//! happens to have the same shape is in the list too, because it declares one
//! thing just as plainly. A file's routes are the ones the index recorded on
//! its methods, read off the route table of the analysis that indexed it
//! ([`crate::index::Declaration::routes`]), so a route is listed whatever
//! shape its file has.
//!
//! The request takes no parameters and answers the whole list. A client asks
//! again whenever the index may have changed — at start, after a workspace
//! pass, after an open document changes, and when the server sends
//! [`CHANGED`] — and each answer costs one pass over the index's file map,
//! with no analysis.
//!
//! **The answer is what the index holds now.** Entries still waiting to be
//! read are not in it, so the list at start is short until the index is
//! settled. The server sends [`CHANGED`] each time the queue empties, and the
//! client asks again. Waiting for the queue instead would hold every other
//! request behind the whole tree's analysis, because a client asks for this
//! list the moment the server starts.
//!
//! **The two fields are optional on the wire, so an older client keeps
//! working.** A file with routes but not the one-type shape has no `kind`, and
//! a client that knows only `kind` draws nothing for it, as it did before.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::document::uri_of;
use crate::index::{DeclKind, SymbolIndex};

/// The method this request is asked under.
///
/// Namespaced for [`crate::redactions::METHOD`]'s reason: it is not LSP's.
pub const METHOD: &str = "nvs/fileKinds";

/// The notification the server sends when the index has settled, so the
/// client asks [`METHOD`] again. It has no parameters.
pub const CHANGED: &str = "nvs/fileKindsChanged";

/// One file, the kind of the one type it declares, and the routes its methods
/// handle. At least one of the two is present.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileKind {
    /// The file, as a `file:` URI.
    pub uri: String,
    /// `class`, `interface`, `enum` or `type`, and absent for a file that
    /// declares anything but exactly one type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
    /// Every route a method in this file handles, in source order, each as its
    /// verb in capitals and its path as written (`GET /users/{id}`). Absent
    /// when there is none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<String>,
}

/// Every file in `index` that declares exactly one type or has a route
/// handler, in path order.
///
/// **Memory:** one map entry per one-type file for the length of the answer,
/// in the editor process.
#[must_use]
pub fn answer(index: &SymbolIndex) -> Vec<FileKind> {
    let sole: BTreeMap<_, _> = index.sole_kinds().collect();
    index
        .files()
        .filter_map(|path| {
            let kind = sole.get(path).copied().and_then(word);
            let routes: Vec<String> = index
                .declarations_in(path)
                .iter()
                .flat_map(|declared| declared.routes.iter().cloned())
                .collect();
            if kind.is_none() && routes.is_empty() {
                return None;
            }
            Some(FileKind {
                uri: uri_of(path)?.as_str().to_owned(),
                kind,
                routes,
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
