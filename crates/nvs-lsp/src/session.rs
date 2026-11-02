//! What the editor keeps from one analysis to the next: the parse of every
//! file, the checked output of every file's bodies, and the one interner both
//! name their types against.
//!
//! [`Session`] lives in the document store for as long as the server runs, and
//! every analysis borrows it. A keystroke then parses the file it changed and
//! checks the bodies of the files whose inputs changed. Every other file is
//! taken from the session.
//!
//! # Decision: a checked output is valid for one key
//!
//! A file's bodies are checked again when its key differs from the key its
//! output was stored under. The key is the hash of the file's own text and of
//! its graph's key. The graph's key hashes the extension set, the files whose
//! `autoload` declarations the analysis borrowed, and the path, id and
//! declaration surface (`crate::surface`) of every file of the graph, in the
//! order the graph has them. A body-only edit keeps every surface, so it
//! checks the edited file's bodies alone. A surface edit, a file added to the
//! graph or a file that left it checks every file. A finer key, the surfaces
//! a body actually reads, is not built: a surface edit is much rarer than a
//! body edit, and one combined hash is simpler to trust. The count of
//! anonymous functions before a file is the one input `nvs_types` checks
//! itself.
//!
//! Two entries whose graphs differ have different graph keys, so an analysis
//! of one entry after the other checks every file of its graph again. A
//! keystroke analyses the open document again, and its graph's key is the one
//! it stored.
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
//! table, its diagnostics and its rows, and one 32-byte key. On the generated
//! tree of `tests/incremental.rs`, twenty programs and a forty-class library,
//! a parse costs about 7 KB a file and a checked output about 6 KB.
//! Both are freed when the file leaves the tree. The interner is bounded by
//! twice its size at the last settle.
//!
//! [`Analysed`]: crate::Analysed

use std::path::Path;
use std::rc::Rc;

use nvs_diagnostics::SourceId;
use nvs_types::{CheckCache, TypeInterner};
use rustc_hash::{FxHashMap, FxHashSet};

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

/// The keys of one analysis's files, and the files whose bodies it checks.
pub(crate) struct Keys {
    keys: Vec<(SourceId, blake3::Hash)>,
    pub(crate) bodies: FxHashSet<SourceId>,
}

impl Session {
    /// The cache and the interner a check runs against. The interner is
    /// copied first if a kept analysis still shares it. It is dropped, with
    /// every checked output, once it has grown to twice its size at the last
    /// settle.
    pub(crate) fn check_parts(&mut self) -> (&mut CheckCache, &mut TypeInterner) {
        let len = self.interner.own_len();
        if self.settled_len > 0 && len >= 2 * self.settled_len {
            self.interner = Rc::default();
            self.checks.clear();
            self.keys.clear();
            self.settled_len = 0;
            self.rebuilds += 1;
        }
        (&mut self.checks, Rc::make_mut(&mut self.interner))
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

    /// The key of each of `files`, which share the graph key `graph`, and the
    /// files whose key differs from the one their output was stored under.
    pub(crate) fn keys(&self, graph: &blake3::Hash, files: &[(SourceId, [u8; 32])]) -> Keys {
        let keys: Vec<(SourceId, blake3::Hash)> = files
            .iter()
            .map(|(id, text)| {
                let mut hasher = blake3::Hasher::new();
                hasher.update(graph.as_bytes());
                hasher.update(text);
                (*id, hasher.finalize())
            })
            .collect();
        let bodies = keys
            .iter()
            .filter(|(id, key)| self.keys.get(id) != Some(key))
            .map(|(id, _)| *id)
            .collect();
        Keys { keys, bodies }
    }

    /// Stores the keys of a check that ran, so the next check of the same
    /// inputs takes every output.
    pub(crate) fn store(&mut self, keys: Keys) {
        self.keys.extend(keys.keys);
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
