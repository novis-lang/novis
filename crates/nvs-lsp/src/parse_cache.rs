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
//! # What it costs
//!
//! Per file of the tree, for as long as the file is in it: the file's
//! statements, its trivia and syntax index (an analysis of the file as an open
//! document needs both), the diagnostics its parse reported, two 32-byte
//! hashes (its text and its declaration surface, `crate::surface`), and one
//! pinned path. Every map the cache builds holds one empty source file
//! per pinned path that analysis does not load. [`ParseCache::retain`] frees a
//! file once it leaves the tree.

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

/// The last parse of every file an analysis loaded, keyed by canonical path.
#[derive(Debug, Default)]
pub(crate) struct ParseCache {
    pins: Pins,
    files: HashMap<PathBuf, Entry>,
    /// How many files were parsed, because the cache held no entry for their
    /// text and id, since the cache was made.
    misses: usize,
}

/// One file's parse, valid while its text hashes to `hash` and it loads at
/// `id`.
#[derive(Debug)]
struct Entry {
    id: SourceId,
    hash: Digest,
    /// [`crate::surface::of`] the statements, computed once per text.
    surface: Digest,
    stmts: Arc<Vec<Stmt>>,
    trivia: Vec<Trivia>,
    index: SyntaxIndex,
    diags: Vec<Diagnostic>,
}

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
        let gone: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|path| !keep(path))
            .cloned()
            .collect();
        let mut ids = Vec::with_capacity(gone.len());
        for path in gone {
            if let Some(entry) = self.files.remove(&path) {
                ids.push(entry.id);
            }
            self.pins.forget(&path);
        }
        ids
    }

    /// The hash of `src`'s text and of its declaration surface, where the
    /// cache holds the parse of that text at `src`'s id.
    pub(crate) fn hashes(&self, src: &SourceFile) -> Option<(Digest, Digest)> {
        let entry = self.files.get(&key_of(src))?;
        (entry.id == src.id()).then_some((entry.hash, entry.surface))
    }

    /// The entry for `src`'s text and id, parsed now if the cache has none,
    /// with its diagnostics reported into `diags` either way.
    fn entry(&mut self, src: &SourceFile, diags: &mut Diagnostics) -> &Entry {
        let hash = content_hash(src.text().as_bytes());
        let key = key_of(src);
        let fresh = self
            .files
            .get(&key)
            .is_none_or(|entry| entry.hash != hash || entry.id != src.id());
        if fresh {
            self.misses += 1;
            let mut own = Diagnostics::new();
            let nvs_syntax::Parsed {
                stmts,
                trivia,
                modifiers: _,
                index,
            } = parse(src, &mut own);
            check_declarations(&stmts, src, &mut own);
            let surface = crate::surface::of(&stmts, src.text());
            self.files.insert(
                key.clone(),
                Entry {
                    id: src.id(),
                    hash,
                    surface,
                    stmts: Arc::new(stmts),
                    trivia,
                    index,
                    diags: own.iter().cloned().collect(),
                },
            );
        }
        let entry = &self.files[&key];
        for diagnostic in &entry.diags {
            diags.report(diagnostic.clone());
        }
        entry
    }
}

/// The cache's key for `src`: its canonical path, or its name for a file with
/// no path. A NUL is in no real path, so a file with no path never takes the
/// key of one that has one.
fn key_of(src: &SourceFile) -> PathBuf {
    src.path()
        .map_or_else(|| PathBuf::from(format!("\0{}", src.name())), canonical_key)
}

impl Parses for ParseCache {
    fn parse(&mut self, src: &SourceFile, diags: &mut Diagnostics) -> Arc<Vec<Stmt>> {
        Arc::clone(&self.entry(src, diags).stmts)
    }
}
