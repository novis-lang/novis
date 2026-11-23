//! One parse per file text, kept for the whole editor session.
//!
//! A keystroke analyses the open document's graph again, and before this cache
//! every analysis parsed every file of that graph. [`ParseCache`] keeps the
//! last parse of each file under its canonical path, with the BLAKE3 hash of
//! the text it parsed. A file whose text hashes the same is not parsed again:
//! its statements are shared, and the diagnostics its parse and
//! `check_declarations` reported are reported again. So a keystroke parses the
//! file it edited, and every file it did not edit is a hash of its text.
//!
//! The file is still read on every analysis, because the walk loads it to know
//! its text. That read is not the measured cost.
//!
//! # Decision: a path is resolved once, and its file is still read
//!
//! A path is made canonical when a document opens and when the index records
//! a file, and [`ParseCache`] keeps each path's key until the file leaves the
//! tree. A request reads the stored key and resolves no path again.
//! **Declined:** skipping the read of each graph file per request. A file that
//! changed outside the editor must still be seen, and only that read sees it.
//!
//! # Decision: an entry is valid for one text and one id
//!
//! A statement's spans name its file by [`SourceId`], and every analysis builds
//! a new [`SourceMap`]. The cache keeps one [`Pins`] table, so each map it
//! builds ([`ParseCache::map`]) loads a path at the id it had before, and an
//! entry is used only where the file's id is the id it was parsed at. A file
//! with no path, such as an extension's carried source, is keyed by its name
//! and is parsed again whenever its id moved.
//!
//! This is not an incremental reparse (`rule:ide/one-grammar-one-tree`): a
//! file whose text changed is parsed whole.
//!
//! # Decision: the workers read the server's parses, and nothing is locked
//!
//! The server's thread keeps the one cache. Before the index sends its first
//! job it pins every file of the tree ([`ParseCache::pin_keys`]), so a file
//! has the same id on every thread. Each job carries a [`Shared`]: the
//! server's pins and its map of parses, both behind an [`Arc`] and read-only.
//! A worker's cache reads a parse from it before it parses
//! ([`ParseCache::adopt`]), and returns the parses it made itself
//! ([`ParseCache::take_made`]). The server files them
//! ([`ParseCache::file`]), so the next job reads them too. The map is copied
//! on write: the server copies it only when it changes while a job still
//! reads the old one. A parse made at an id the server did not pin, such as
//! a file outside the tree, stays with the worker that made it.
//!
//! **Declined:** one cache behind a lock that every worker writes. A lock on
//! the analysis path costs latency, and the latency of the request path ranks
//! above memory.
//!
//! # What it costs
//!
//! Per file of the tree, for as long as the file is in it: the file's
//! statements, its trivia and syntax index (an analysis of the file as an open
//! document needs both), the diagnostics its parse reported, two 32-byte
//! hashes (its text and its declaration surface, `crate::surface`), and one
//! pinned path. Every map the cache builds holds one empty source file
//! per pinned path that analysis does not load, and the whole tree is pinned
//! once the workers run. [`ParseCache::retain`] frees a file once it leaves
//! the tree. On the generated tree of `tests/incremental.rs` (100 files of
//! about 220 bytes each) a parse is about 10 KB a file; `crate::session` gives
//! the whole session's figure.
//!
//! The workers share each parse, so the parses of the tree are kept once, on
//! the server's thread. While entries wait, the server copies its map of
//! parses, two pointers per file, each time it files a parse that a job in
//! flight did not see. A worker copies the server's pins once per change to
//! them. A parse a worker made stays in its own cache until the pool stops.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use nvs_config::cache::{Digest, content_hash};
use nvs_diagnostics::{
    Diagnostic, Diagnostics, Pins, SourceFile, SourceId, SourceMap, canonical_key,
};
use nvs_hir::Parses;
use nvs_syntax::ast::Stmt;
use nvs_syntax::{SyntaxIndex, Trivia, check_declarations, parse};

/// Each file's parse, keyed by canonical path.
type Files = HashMap<Arc<Path>, Arc<Entry>>;

/// The last parse of every file an analysis loaded, keyed by canonical path.
#[derive(Debug, Default)]
pub(crate) struct ParseCache {
    pins: Pins,
    /// The parses this cache made or filed. On the server's thread every job
    /// in flight shares it, and it is copied when it changes under one.
    files: Arc<Files>,
    /// Each path a parse was asked for, with the key it resolved to. A path is
    /// made canonical once, and [`retain`](Self::retain) clears this when the
    /// tree changes. **What it spends:** two paths per file of the tree.
    keys: HashMap<PathBuf, PathBuf>,
    /// How many files were parsed, because the cache held no entry for their
    /// text and id, since the cache was made.
    misses: usize,
    /// The pins [`share`](Self::share) gave out last, while they are the
    /// pins this cache has.
    shared_pins: Option<Arc<Pins>>,
    /// On a worker: the server's parses the job carried, read after
    /// [`files`](Self::files). Empty on the server's thread.
    server: Arc<Files>,
    /// On a worker: the pins it adopted last, so the next job that carries
    /// the same ones copies nothing.
    adopted: Option<Arc<Pins>>,
    /// On a worker: the keys of the files this cache parsed since the last
    /// [`take_made`](Self::take_made).
    made: Vec<Arc<Path>>,
}

/// One file's parse, valid while its text hashes to `hash` and it loads at
/// `id`.
#[derive(Debug)]
pub(crate) struct Entry {
    id: SourceId,
    hash: Digest,
    /// [`crate::surface::of`] the statements, computed once per text.
    surface: Digest,
    stmts: Arc<Vec<Stmt>>,
    trivia: Vec<Trivia>,
    index: SyntaxIndex,
    diags: Vec<Diagnostic>,
}

/// The server's pins and parses, read-only, for one job.
#[derive(Clone, Debug, Default)]
pub(crate) struct Shared {
    pins: Arc<Pins>,
    files: Arc<Files>,
}

/// The parses a worker made for one job, for the server to file.
#[derive(Debug, Default)]
pub(crate) struct Made(Vec<(Arc<Path>, Arc<Entry>)>);

impl Made {
    /// Whether the worker made no parse the server can file.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A job's parses go to a worker, and the worker's go back.
const _: () = {
    const fn sendable<T: Send>() {}
    sendable::<Shared>();
    sendable::<Made>();
};

/// A file's parse as an open document needs it.
pub(crate) struct Parsed {
    pub(crate) stmts: Arc<Vec<Stmt>>,
    pub(crate) trivia: Vec<Trivia>,
    pub(crate) index: SyntaxIndex,
}

impl ParseCache {
    /// A source map that loads every path at the id it had in the last map.
    /// Hand it back to [`keep_pins`](Self::keep_pins) once nothing more loads.
    pub(crate) fn map(&mut self) -> SourceMap {
        SourceMap::pinned(std::mem::take(&mut self.pins))
    }

    /// Takes back the pins `map` added to.
    pub(crate) fn keep_pins(&mut self, map: &mut SourceMap) {
        self.pins = map.take_pins();
        // Pins only grow between two calls to `retain`, so a table of the
        // same length is the table that was shared.
        if self
            .shared_pins
            .as_ref()
            .is_some_and(|shared| shared.len() != self.pins.len())
        {
            self.shared_pins = None;
        }
    }

    /// Pins each of `keys`, paths already under `canonical_key`, so a parse
    /// made on any thread loads them at the same id.
    pub(crate) fn pin_keys<'a>(&mut self, keys: impl IntoIterator<Item = &'a Path>) {
        let mut added = false;
        for key in keys {
            added |= self.pins.pin_key(key);
        }
        if added {
            self.shared_pins = None;
        }
    }

    /// The pins and parses a job reads. Two `Arc`s, and one copy of the pins
    /// when they changed since the last call.
    pub(crate) fn share(&mut self) -> Shared {
        let pins = self
            .shared_pins
            .get_or_insert_with(|| Arc::new(self.pins.clone()));
        Shared {
            pins: Arc::clone(pins),
            files: Arc::clone(&self.files),
        }
    }

    /// Reads `shared` before parsing, from now on. The pins are copied only
    /// when they are not the ones adopted last.
    pub(crate) fn adopt(&mut self, shared: Shared) {
        let Shared { pins, files } = shared;
        if !self
            .adopted
            .as_ref()
            .is_some_and(|adopted| Arc::ptr_eq(adopted, &pins))
        {
            self.pins = Pins::clone(&pins);
            self.adopted = Some(pins);
        }
        self.server = files;
    }

    /// The parses this cache made since the last call, at the id the adopted
    /// pins give their file. Every other parse stays here.
    pub(crate) fn take_made(&mut self) -> Made {
        let made = std::mem::take(&mut self.made);
        let Some(pins) = &self.adopted else {
            return Made::default();
        };
        Made(
            made.into_iter()
                .filter_map(|key| {
                    let entry = self.files.get(&key)?;
                    (pins.of_key(&key) == Some(entry.id)).then(|| (key, Arc::clone(entry)))
                })
                .collect(),
        )
    }

    /// Files the parses a worker made, where this cache has none for the file
    /// and pins it at the id it was parsed at.
    pub(crate) fn file(&mut self, made: Made) {
        let fresh: Vec<(Arc<Path>, Arc<Entry>)> = made
            .0
            .into_iter()
            .filter(|(key, entry)| {
                !self.files.contains_key(key) && self.pins.of_key(key) == Some(entry.id)
            })
            .collect();
        if !fresh.is_empty() {
            Arc::make_mut(&mut self.files).extend(fresh);
        }
    }

    /// How many files were parsed since the cache was made.
    pub(crate) const fn misses(&self) -> usize {
        self.misses
    }

    /// `src` parsed with its trivia and index, which an open document's
    /// analysis reads, and its diagnostics reported into `diags`.
    pub(crate) fn parse_entry(&mut self, src: &SourceFile, diags: &mut Diagnostics) -> Parsed {
        let entry = self.entry(src, diags);
        Parsed {
            stmts: Arc::clone(&entry.stmts),
            trivia: entry.trivia.clone(),
            index: entry.index.clone(),
        }
    }

    /// Frees every file `keep` says is no longer in the tree, and returns the
    /// ids those files were pinned at.
    pub(crate) fn retain(&mut self, keep: impl Fn(&Path) -> bool) -> Vec<SourceId> {
        let gone: Vec<Arc<Path>> = self
            .files
            .keys()
            .filter(|path| !keep(path))
            .cloned()
            .collect();
        if gone.is_empty() {
            return Vec::new();
        }
        self.keys.clear();
        self.shared_pins = None;
        let files = Arc::make_mut(&mut self.files);
        let mut ids = Vec::with_capacity(gone.len());
        for path in gone {
            if let Some(entry) = files.remove(&path) {
                ids.push(entry.id);
            }
            self.pins.forget(&path);
        }
        ids
    }

    /// The hash of `src`'s text and of its declaration surface, where the
    /// cache holds the parse of that text at `src`'s id.
    pub(crate) fn hashes(&self, src: &SourceFile) -> Option<(Digest, Digest)> {
        let key = self.key_of(src);
        let entry = self.find(&key, src.id(), None)?;
        Some((entry.hash, entry.surface))
    }

    /// The parse of `key` at `id`, and of the text that hashes to `hash` where
    /// one is given: this cache's own, else the server's.
    fn find(&self, key: &Path, id: SourceId, hash: Option<&Digest>) -> Option<&Arc<Entry>> {
        [&self.files, &self.server]
            .into_iter()
            .filter_map(|files| files.get(key))
            .find(|entry| entry.id == id && hash.is_none_or(|hash| entry.hash == *hash))
    }

    /// The entry for `src`'s text and id, parsed now if the cache has none,
    /// with its diagnostics reported into `diags` either way.
    fn entry(&mut self, src: &SourceFile, diags: &mut Diagnostics) -> &Entry {
        let hash = content_hash(src.text().as_bytes());
        let key = self.key_of(src).into_owned();
        if let Some(path) = src.path()
            && !self.keys.contains_key(path)
        {
            self.keys.insert(path.to_path_buf(), key.clone());
        }
        if self.find(&key, src.id(), Some(&hash)).is_none() {
            self.misses += 1;
            let mut own = Diagnostics::new();
            let nvs_syntax::Parsed {
                stmts,
                trivia,
                modifiers: _,
                index,
            } = parse(src, &mut own);
            check_declarations(&stmts, src, &mut own);
            let surface = crate::surface::of(&stmts, &trivia, src.text());
            let key: Arc<Path> = Arc::from(key.as_path());
            Arc::make_mut(&mut self.files).insert(
                Arc::clone(&key),
                Arc::new(Entry {
                    id: src.id(),
                    hash,
                    surface,
                    stmts: Arc::new(stmts),
                    trivia,
                    index,
                    diags: own.iter().cloned().collect(),
                }),
            );
            // Only a worker hands its parses back.
            if self.adopted.is_some() {
                self.made.push(key);
            }
        }
        let entry = self
            .find(&key, src.id(), Some(&hash))
            .expect("the parse was found or made above");
        for diagnostic in &entry.diags {
            diags.report(diagnostic.clone());
        }
        entry
    }

    /// The cache's key for `src`: its canonical path, or its name for a file
    /// with no path. A NUL is in no real path, so a file with no path never
    /// takes the key of one that has one. A path [`entry`](Self::entry)
    /// already resolved is not resolved again.
    pub(crate) fn key_of(&self, src: &SourceFile) -> Cow<'_, Path> {
        match src.path() {
            Some(path) => self.keys.get(path).map_or_else(
                || Cow::Owned(canonical_key(path)),
                |key| Cow::Borrowed(key.as_path()),
            ),
            None => Cow::Owned(PathBuf::from(format!("\0{}", src.name()))),
        }
    }
}

impl Parses for ParseCache {
    fn parse(&mut self, src: &SourceFile, diags: &mut Diagnostics) -> Arc<Vec<Stmt>> {
        Arc::clone(&self.entry(src, diags).stmts)
    }
}
