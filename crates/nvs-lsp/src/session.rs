//! What the editor keeps from one analysis to the next: the parse of every
//! file, the checked output of every file's bodies, and the one interner both
//! name their types against.
//!
//! [`Session`] lives in the document store for as long as the server runs, and
//! every analysis borrows it. A keystroke then parses the file it changed and
//! checks the bodies of that file alone. Every other file is taken from the
//! session, or checked by the index's analysis that owns it.
//!
//! # Decision: a checked output is valid for one key
//!
//! A file's bodies are checked again when its key differs from the key its
//! output was stored under. The key is the file's own text and the surface
//! rows (`crate::surface`) of what the file's last check used and failed to
//! find, and of every class above the classes it declares. `crate::file_key`
//! says what it hashes. The key is computed from the uses its stored output
//! recorded, so a check stores it after the walk. A body-only edit keeps every
//! row, so it checks the edited file's bodies alone. A surface edit checks the
//! files that read the rows it changed. A file added to the graph, a file
//! that left it, a graph row and an attribute move every file's key, and the
//! settle after them checks each file once. The count of anonymous functions
//! before a file is the one input `nvs_types` checks itself.
//!
//! Two entries whose graphs differ have different keys. So an analysis
//! checks only the bodies of the files it will own: the index's analysis of
//! an entry leaves out every file another entry indexed, and an open
//! document's analysis leaves out every file but the document
//! (`crate::document::analyse_through`). A file left out whose key moved is
//! not checked and keeps the key it had, so the next analysis that owns it
//! checks it. A cold settle therefore checks each file once, whatever the
//! number of entries that read it, and a keystroke checks the document's
//! bodies alone.
//!
//! A file left out without a current output is missing from the analysis's
//! expressions, diagnostics and rows. The open document's diagnostics that
//! compare the rows of several files, such as two routes on one path, see
//! only the files whose output is current. `nvs check` reports them whole.
//! A diagnostic that a row is missing would be false there, so a link to a
//! method of a file left out is not checked (`nvs_types::links`).
//!
//! # Decision: one interner, shared and copied on write
//!
//! A cached output names its types by [`TypeId`](nvs_types::TypeId), so every
//! check runs against the one interner of the session. An [`Analysed`] keeps
//! an [`Rc`] of it, because a reader looks a type up long after the check. A
//! check interns into the session's copy with [`Rc::make_mut`], which copies
//! the interner only while a kept analysis still shares it. An id is never
//! renumbered, so every analysis reads its own ids in the copy it holds.
//!
//! The interner grows with the distinct types the project and its edits
//! produce, and an edit that writes a new literal type adds one. So the
//! session drops it, with every checked output, once it has twice the types it
//! had when the tree was last settled. The next analysis of each graph checks
//! every body again, which is one full settle, far apart.
//!
//! # What it costs
//!
//! [`ParseCache`] says what a parse costs. Per file of the tree, the session
//! also keeps its checked output, which is the file's part of the expression
//! table, its diagnostics and its rows, and one 32-byte key. Measured with the
//! runtime's live-byte counter on the generated tree of `tests/incremental.rs`,
//! twenty programs and a forty-class library (100 files of about 220 bytes of
//! text each): the session keeps about 19 KB a file. A parse is about 10 KB of
//! it, and the checked output, its key and the file's share of the interner
//! are the other 9.5 KB. The index entry (`crate::index`) is another 5 KB. All
//! of it grows with a file's text, at roughly 45 times its size for the parse
//! alone. Each part is freed when the file leaves the tree. The interner is
//! bounded by twice its size at the last settle.
//!
//! Because an analysis checks only the bodies it will own, a session on a
//! worker keeps the checked output of its own parts, and the workers together
//! keep about one copy of the tree's. On twelve copies of the tree
//! `bun nv lsp-start` generates (972 files) with 15 workers, a cold index
//! checks 972 files and peaks at 786 MB paged. When every worker's session
//! checked every body its entries read, it peaked at 5752 MB. The live bytes
//! of a settled tree are about 228 MB (`crate::workers` gives the rest).
//!
//! The rows of each file and what its check used and failed to find
//! (`crate::file_key`) cost memory for the time they buy. On the same tree,
//! measured with `bun nv lsp-start --copies 12 --cold --edits 5 --kind <kind>`
//! on a build before them and on one after, four runs each: after the cold
//! index, the server held about 460 MB private and peaked at about 900 MB
//! paged before, and about 550 MB and 1015 MB after. That is about 90 MB, or
//! 95 KB a file, which does not grow with edits. Before, five edits to a
//! member, a private constant or a public constant each checked the 972 files
//! again, 4861 files in all, and the server then held 680 to 1000 MB. After,
//! a new member and a private constant check no other file, five edits to a
//! public constant that one file reads check 11, and the server then holds
//! about 500 MB. Five comment edits check no other file on either build.
//!
//! [`Analysed`]: crate::Analysed

use std::path::Path;
use std::rc::Rc;

use nvs_diagnostics::{SourceFile, SourceId};
use nvs_hir::ClassGraph;
use nvs_types::{CheckCache, TypeInterner};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::file_key::Surfaces;
use crate::parse_cache::ParseCache;

/// The parses, the checked outputs and the interner one server keeps across
/// analyses.
#[derive(Debug, Default)]
pub(crate) struct Session {
    pub(crate) parses: ParseCache,
    /// The checked output of each file's bodies, by the file's pinned id.
    pub(crate) checks: CheckCache,
    /// The key each output in [`checks`](Self::checks) was stored under.
    keys: FxHashMap<SourceId, blake3::Hash>,
    /// The interner every output in [`checks`](Self::checks) names its types
    /// against.
    interner: Rc<TypeInterner>,
    /// The interner's own size when the tree was last settled.
    settled_len: usize,
    /// How many times the interner was dropped for its size.
    rebuilds: usize,
}

impl Session {
    /// Runs `check` with the files of `files` whose key moved, the cache and
    /// the interner, and then stores the key of every file it walked. `base`
    /// and `graph` are what [`Surfaces::new`] takes. A file in `left_out` is
    /// not walked, so it keeps the key it had, and the next analysis that
    /// owns it checks it.
    ///
    /// The interner is copied first if a kept analysis still shares it. It is
    /// dropped, with every checked output, once it has grown to twice its
    /// size at the last settle.
    pub(crate) fn check<R>(
        &mut self,
        files: &[&SourceFile],
        base: &blake3::Hash,
        graph: &ClassGraph,
        left_out: &FxHashSet<SourceId>,
        check: impl FnOnce(&FxHashSet<SourceId>, &mut CheckCache, &mut TypeInterner) -> R,
    ) -> R {
        let len = self.interner.own_len();
        if self.settled_len > 0 && len >= 2 * self.settled_len {
            self.interner = Rc::default();
            self.checks.clear();
            self.keys.clear();
            self.settled_len = 0;
            self.rebuilds += 1;
        }
        let surfaces = Surfaces::new(&self.parses, files.iter().copied(), base, graph);
        let bodies: FxHashSet<SourceId> = files
            .iter()
            .map(|src| src.id())
            .filter(|id| match (self.keys.get(id), self.checks.get(*id)) {
                (Some(stored), Some(output)) => surfaces.key(*id, output.uses()) != *stored,
                _ => true,
            })
            .collect();
        let out = check(&bodies, &mut self.checks, Rc::make_mut(&mut self.interner));
        for id in bodies.iter().filter(|id| !left_out.contains(id)) {
            match self.checks.get(*id) {
                Some(output) => self.keys.insert(*id, surfaces.key(*id, output.uses())),
                None => self.keys.remove(id),
            };
        }
        out
    }

    /// The session's interner, for an analysis to keep.
    pub(crate) fn shared_interner(&self) -> Rc<TypeInterner> {
        Rc::clone(&self.interner)
    }

    /// How many times the interner was dropped for its size.
    pub(crate) const fn rebuilds(&self) -> usize {
        self.rebuilds
    }

    /// The interner's own size.
    pub(crate) fn interner_len(&self) -> usize {
        self.interner.own_len()
    }

    /// Frees the parse and the checked output of every file `keep` says has
    /// left the tree, and takes the interner's size as the settled one.
    pub(crate) fn settled(&mut self, keep: impl Fn(&Path) -> bool) {
        for id in self.parses.retain(keep) {
            self.checks.remove(id);
            self.keys.remove(&id);
        }
        self.settled_len = self.interner.own_len();
    }
}
