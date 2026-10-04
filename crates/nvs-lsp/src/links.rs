//! Every path literal in one document that names something on disk, and what
//! it names.
//!
//! `textDocument/documentLink` is one of the projections with nothing behind it
//! (`rule:ide/the-request-set-is-closed`): each edge it reports was already
//! resolved by the walk that loaded the program, and this module reads it.
//!
//! - **A `require`'s path literal links to the file it loaded.**
//!   `nvs_hir::requires::Loaded::requires` is that edge, a span paired with a
//!   `SourceId`, and its own doc says why nothing downstream re-derives it from
//!   the literal's text.
//! - **An `autoload` root links to the directory it names, and a `discover`
//!   glob to the directory it lists** — the part before its `*` segment
//!   (`rule:programs/autoload`). The walk keeps each declaration as an
//!   `nvs_hir::autoload::Site` with every literal's span, and
//!   `Site::directories` resolves those literals with the same functions the
//!   autoload map is built with, so a link and the map cannot disagree.
//! - **An `autoload` prefix links to the first of its roots that exists**,
//!   which is the first directory a name under it is looked for in. A prefix
//!   is a namespace and has no declaration to jump to, so the link is its
//!   ctrl-click, and a hover lists every root (`crate::hover`).
//! - **An argument at a path parameter links to the file or directory it
//!   names** (`rule:programs/relative-paths-resolve-from-their-file`). The
//!   checker joined a relative literal to the folder of its file and kept the
//!   result (`nvs_types::ExprTypeTable::path_literal`), and an absolute
//!   literal names itself; [`crate::arguments::target`] reads the two. Unlike
//!   a `require`, nothing was loaded through this edge, so whether the target
//!   exists, and which kind it is, is asked of the disk when the link is made.
//!
//! **The entry file only**, on [`crate::symbols::for_document`]'s terms: a
//! required file's own `require` is a link in *its* document.
//!
//! **What has no edge has no link**: a `require` path that is not a literal,
//! one that resolves to nothing loadable, one that would close a cycle, an
//! `autoload` root that does not exist, a glob of the wrong shape or over a
//! missing directory, and a path argument that names nothing on disk. Each is
//! already a diagnostic, the dynamic fallback, a root `rule:programs/autoload`
//! allows to be missing, or a file the program may be about to create, and an
//! underline that opened nothing would be a second, quieter report of the
//! same thing.
//!
//! **A directory is a target of its own kind.** An editor opens a file in a tab
//! and cannot open a directory there, so [`PathLink::directory`] says which one
//! a link names. The server sends a directory as a `file:` URI ending in `/`,
//! which is how a URI names a directory, and the VS Code client reveals such a
//! target in its Explorer instead of opening it.
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

/// One path literal, and the file or directory the walk resolved it to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathLink {
    /// The literal's own range, quotes included — the span the walk recorded
    /// the edge under, and the text an editor underlines.
    pub range: Range,
    /// What it names, as the analysis resolved it: absolute and canonical.
    pub target: PathBuf,
    /// Whether [`target`](Self::target) is a directory: an `autoload` root or
    /// a `discover` glob's base, where a `require` names a file, and a path
    /// argument either.
    pub directory: bool,
}

/// Every link the entry document of `analysed` carries, in `encoding`.
///
/// Sorted by where a link starts, which is the order they were written in —
/// stated rather than inherited from the walk, because a `.lspt` case freezes
/// this list and the order it reads in should be the document's.
#[must_use]
pub fn for_document(analysed: &Analysed, encoding: PositionEncoding) -> Vec<PathLink> {
    let file = analysed.map.file(analysed.entry);
    let mut links: Vec<PathLink> = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
        .map(|loaded| loaded.requires.as_slice())
        .unwrap_or_default()
        .iter()
        .filter_map(|(span, target)| {
            Some(PathLink {
                range: range_at(file, *span, encoding),
                // A required file was loaded from disk, so it has a path. The
                // filter is a fallback rather than a policy, on the same terms
                // as [`crate::symbols`]': a file the walk reached without one
                // shows no link rather than taking the server down.
                target: analysed.map.file(*target).path()?.to_path_buf(),
                directory: false,
            })
        })
        .collect();
    links.extend(
        analysed
            .autoloads
            .iter()
            .flat_map(nvs_hir::autoload::Site::directories)
            .map(|(span, target)| PathLink {
                range: range_at(file, span, encoding),
                target,
                directory: true,
            }),
    );
    links.extend(
        crate::arguments::in_document(analysed)
            .into_iter()
            .filter(|argument| argument.text == nvs_stdlib::registry::ParamText::Path)
            .filter_map(|argument| {
                let target = crate::arguments::target(analysed, &argument)?;
                let metadata = std::fs::metadata(&target).ok()?;
                Some(PathLink {
                    range: range_at(file, argument.span, encoding),
                    target,
                    directory: metadata.is_dir(),
                })
            }),
    );
    links.sort_by_key(|link| link.range.start);
    links
}
