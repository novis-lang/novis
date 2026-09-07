//! Every `require` in one document, and the file it reaches.
//!
//! `textDocument/documentLink` is the third projection with nothing behind it
//! (`rule:ide/the-request-set-is-closed`), and the least work of the three: the
//! edge from a `require`'s path literal to the file it named already exists,
//! because the graph walk that loaded the program is the one place it can
//! exist. `nvs_hir::requires::Loaded::requires` is that edge — a span paired
//! with a `SourceId` — and its own doc says why nothing downstream re-derives
//! it from the literal's text. This module reads it.
//!
//! **The entry file only**, on [`crate::symbols::for_document`]'s terms: a
//! required file's own `require` is a link in *its* document.
//!
//! **A link is a path literal, and `require`'s is the only one with an edge.**
//! A `use` names a class rather than a file, and where a name is declared is
//! `textDocument/definition`'s answer. An `autoload` does write path literals
//! (`rule:programs/autoload`), and the goal names them — but a root is a
//! *directory*, `discover`'s is a glob, and `nvs_hir::autoload::Site` records
//! the declaration's own span rather than one per literal, so there is no
//! literal-to-path edge to read. Deriving one here would be the second copy of
//! the resolution rule that `Loaded::requires`' own doc exists to prevent, so
//! linking one is the slice that adds that edge beside the `require` one.
//!
//! **What has no edge has no link**: a path that is not a literal, one that
//! resolves to nothing loadable, and one that would close a cycle. Each is
//! already a diagnostic or the dynamic fallback that rule leaves alone, and an
//! underline that opened nothing would be a second, quieter report of the same
//! thing.
//!
//! **The target is a path, and the caller spells it.** A client is sent a
//! `file:` URI and a `.lspt` case names the file the way the case wrote it;
//! both are resolution rather than rendering, which is
//! [`crate::render::Link`]'s own reasoning.

use std::path::PathBuf;

use lsp_types::Range;
use nvs_diagnostics::PositionEncoding;

use crate::document::Analysed;
use crate::position::range_at;

/// One `require`'s path literal, and the file the graph walk resolved it to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequireLink {
    /// The path expression's own range — the literal an editor underlines,
    /// with its quotes, since that is the span the walk recorded the edge
    /// under.
    pub range: Range,
    /// The file it names, as the analysis loaded it: absolute and canonical.
    pub target: PathBuf,
}

/// Every link the entry document of `analysed` carries, in `encoding`.
///
/// Sorted by where a link starts, which is the order they were written in —
/// stated rather than inherited from the walk, because a `.lspt` case freezes
/// this list and the order it reads in should be the document's.
#[must_use]
pub fn for_document(analysed: &Analysed, encoding: PositionEncoding) -> Vec<RequireLink> {
    let Some(loaded) = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
    else {
        return Vec::new();
    };
    let file = analysed.map.file(analysed.entry);
    let mut links: Vec<RequireLink> = loaded
        .requires
        .iter()
        .filter_map(|(span, target)| {
            Some(RequireLink {
                range: range_at(file, *span, encoding),
                // A required file was loaded from disk, so it has a path. The
                // filter is a fallback rather than a policy, on the same terms
                // as [`crate::symbols`]': a file the walk reached without one
                // shows no link rather than taking the server down.
                target: analysed.map.file(*target).path()?.to_path_buf(),
            })
        })
        .collect();
    links.sort_by_key(|link| link.range.start);
    links
}
