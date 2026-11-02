//! What one file's walk produced, kept so a later check can fold it in
//! place of walking the file again.
//!
//! [`crate::check::check_program_partial`] walks the bodies of the files its
//! caller names, and takes every other file's [`FileOutput`] from a
//! [`CheckCache`]. Signatures and the whole-program tables are built for
//! every file each time. Only the walk of a file's bodies is skipped. Every
//! pass after the walk runs over the folded result, so a route collision or a
//! link between a checked file and a cached one is still found.
//!
//! A file's walk writes nothing another file's walk reads: each read it makes
//! of the expression table is of a record its own file made, or of a record
//! every file computes the same way from the signatures. So a file walked
//! alone into an empty table records what it records in one walk over the
//! whole program, and the fold, in file order, gives the same table.
//!
//! # When a cached output is valid
//!
//! This module decides nothing about validity. The caller does, with these
//! inputs in its key:
//!
//! - the file's own text,
//! - what every other file declares, since a body reads other files'
//!   signatures,
//! - the extension set and the configuration the check was given,
//! - the [`crate::ty::TypeInterner`]: a cached output names its types by id,
//!   so it is valid only against the interner it was checked with,
//! - the [`nvs_diagnostics::SourceId`] of every file, which a cached span
//!   names; `nvs_diagnostics::SourceMap::pinned` keeps them across maps.
//!
//! One input is checked here: the count of anonymous functions at file scope
//! before the file. Their labels are numbered across files, so a cached output
//! whose starting count moved is walked again.
//!
//! # What it costs
//!
//! A [`CheckCache`] holds one [`FileOutput`] per file it was given: the
//! file's part of the expression table, its walk's diagnostics and its rows.
//! That is about the size of the table a whole check builds, held for as long
//! as the cache lives. A partial check copies every cached output it folds,
//! because the fold moves records into the program's table and the cache
//! keeps its own.
//!
//! A whole check, `nvs check` and every compile, walks each file into its
//! own part too, and stores nothing. It pays one more insert per record,
//! when the part is moved into the program's table. A compile runs once per
//! changed program, never per request.

use nvs_diagnostics::{Diagnostic, SourceId};
use rustc_hash::FxHashMap;

use crate::expr_table::ExprTypeTable;

/// What the walk of one file's bodies wrote: its part of the expression
/// table, its diagnostics, and its rows of the tables the passes after the
/// walk read.
///
/// It is stored before any pass after the walk runs, so it never carries a
/// record one of those passes makes.
#[derive(Clone, Debug)]
pub struct FileOutput {
    /// The count of anonymous functions at file scope before this file.
    pub(crate) start_seq: u32,
    /// The same count after this file.
    pub(crate) end_seq: u32,
    pub(crate) exprs: ExprTypeTable,
    pub(crate) diags: Vec<Diagnostic>,
    pub(crate) routes: crate::routes::RouteTable,
    pub(crate) commands: crate::commands::CommandTable,
    pub(crate) links: Vec<crate::links::LinkSite>,
    pub(crate) codec_sites: Vec<crate::derive::CodecFieldSite>,
    pub(crate) row_sites: Vec<crate::derive::RowSite>,
    pub(crate) json_sites: Vec<crate::derive::JsonSite>,
    pub(crate) decode_sites: Vec<crate::derive::DecodeSite>,
    pub(crate) encode_sites: Vec<crate::derive::EncodeSite>,
}

impl FileOutput {
    /// An empty output for a file walked after `start_seq` anonymous
    /// functions, writing into `exprs`.
    pub(crate) fn empty(start_seq: u32, exprs: ExprTypeTable) -> Self {
        Self {
            start_seq,
            end_seq: start_seq,
            exprs,
            diags: Vec::new(),
            routes: crate::routes::RouteTable::default(),
            commands: crate::commands::CommandTable::default(),
            links: Vec::new(),
            codec_sites: Vec::new(),
            row_sites: Vec::new(),
            json_sites: Vec::new(),
            decode_sites: Vec::new(),
            encode_sites: Vec::new(),
        }
    }

    /// The count of anonymous functions at file scope before this file, which
    /// the output is valid for.
    #[must_use]
    pub fn anon_fn_start(&self) -> u32 {
        self.start_seq
    }

    /// The diagnostics the walk of this file's bodies reported.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diags
    }
}

/// One [`FileOutput`] per file, by the file's id.
///
/// [`crate::check::check_program_partial`] fills it with each file it walks
/// and reads it for each file it does not. The caller removes the output of
/// a file that left the program.
#[derive(Clone, Debug, Default)]
pub struct CheckCache {
    pub(crate) outputs: FxHashMap<SourceId, FileOutput>,
    /// How many files' bodies were walked into this cache since it was made,
    /// so a caller can count a check's work.
    pub(crate) walked: usize,
}

impl CheckCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many files' bodies were walked into this cache since it was made.
    /// [`clear`](Self::clear) does not reset it.
    #[must_use]
    pub const fn walked(&self) -> usize {
        self.walked
    }

    /// How many files have an output.
    #[must_use]
    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    /// Whether no file has an output.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }

    /// The output of the file `id`, if it has one.
    #[must_use]
    pub fn get(&self, id: SourceId) -> Option<&FileOutput> {
        self.outputs.get(&id)
    }

    /// Deletes the output of the file `id`, and returns it.
    pub fn remove(&mut self, id: SourceId) -> Option<FileOutput> {
        self.outputs.remove(&id)
    }

    /// Keeps only the outputs of the files `keep` returns `true` for.
    pub fn retain(&mut self, mut keep: impl FnMut(SourceId) -> bool) {
        self.outputs.retain(|id, _| keep(*id));
    }

    /// Deletes every output.
    pub fn clear(&mut self) {
        self.outputs.clear();
    }
}
