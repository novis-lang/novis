//! Which program lends its `autoload` map to a file that is not where a
//! program starts (`rule:ide/an-autoloaded-file-borrows-its-programs-map`).
//!
//! A class file a program autoloads may not declare `autoload` itself
//! (`rule:programs/autoload`), and a file a program requires usually does not.
//! Walked as its own entry point, either resolves none of the names its program
//! resolves. A **lender** is a program whose `require` chain declares
//! `autoload`, and [`Lender::lends_to`] is the one test of which files it
//! lends its map to. [`resolve_program_borrowing`](crate::resolve_program_borrowing)
//! and [`resolve_program_linted`](crate::resolve_program_linted) take its
//! [`sites`](Lender::sites) as the borrowed declarations.
//!
//! Two callers survey a tree for lenders, and both go through this module so
//! they agree on what a lender is. `nvs-lsp` walks every file under the
//! workspace once and keeps the lenders for the session. `nvs check` walks the
//! files under the project root in path order with [`first_lender`] and stops
//! at the first lender that lends to the checked file. It does that only when
//! the checked file declares no map of its own and names something no file
//! declares.

use std::fs;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostics, SourceMap, canonical_key};
use nvs_syntax::parse_file;

use crate::autoload::{AutoloadMap, Site};
use crate::hierarchy::CoreRoster;
use crate::requires::resolve_program;

/// One program that declares `autoload`, as the files it autoloads or requires
/// borrow it.
#[derive(Debug)]
pub struct Lender {
    /// The file the walk started from, under `canonical_key`.
    entry: PathBuf,
    /// Every file of its `require` chain that wrote a declaration, under
    /// `canonical_key`. An edit to one of them makes this lender stale.
    declaring: Vec<PathBuf>,
    /// Every other file its `require` chain read, under `canonical_key`. These
    /// files borrow this map without lying under a root.
    reads: Vec<PathBuf>,
    /// The directory of its entry, under `canonical_key`. A plain file under
    /// it, one holding neither `require` nor `autoload`, borrows this map too.
    /// That is how a directory `nvs test` runs as one program is analysed the
    /// way it runs.
    dir: PathBuf,
    /// The map those declarations built. It says what the program claims and
    /// carries the sites it lends.
    map: AutoloadMap,
}

impl Lender {
    /// `path` as a lender, or `None` for a file whose program declares no
    /// `autoload`.
    ///
    /// `map` is the source map the walk reads through: empty for files on
    /// disk, or holding the open buffers an editor overlays on them. The text
    /// is searched for `autoload` and `require` before anything is parsed, so a
    /// tree pays one read per file and one walk per file that starts a chain. A
    /// program whose declaration sits in a file it requires holds neither
    /// declaration itself, and only the walk finds it. The walk is name
    /// resolution without the type phase, and it reads every file the program
    /// reaches. Its diagnostics are dropped: the program's own check is where
    /// they are reported.
    #[must_use]
    pub fn walk(mut map: SourceMap, path: &Path) -> Option<Self> {
        let entry = map.load(path).ok()?;
        if is_plain(map.file(entry).text()) {
            return None;
        }

        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(entry), &mut diags);
        let (_, loaded, autoload) =
            resolve_program(entry, stmts, &mut map, CoreRoster::Trusted, &mut diags);
        if autoload.sites().is_empty() {
            return None;
        }

        let mut declaring: Vec<PathBuf> = autoload
            .sites()
            .iter()
            .filter_map(|site| map.file(site.span.file).path().map(canonical_key))
            .collect();
        declaring.sort();
        declaring.dedup();
        let mut reads: Vec<PathBuf> = loaded
            .iter()
            .skip(1)
            .filter_map(|file| map.file(file.id).path().map(canonical_key))
            .collect();
        reads.sort();
        reads.dedup();
        let key = canonical_key(path);
        let dir = key.parent().map_or_else(PathBuf::new, Path::to_path_buf);
        Some(Self {
            entry: key,
            declaring,
            reads,
            dir,
            map: autoload,
        })
    }

    /// The file the program starts from, under `canonical_key`.
    #[must_use]
    pub fn entry(&self) -> &Path {
        &self.entry
    }

    /// Every file of the program's `require` chain that wrote a declaration,
    /// under `canonical_key`, sorted.
    #[must_use]
    pub fn declaring(&self) -> &[PathBuf] {
        &self.declaring
    }

    /// Every other file the program's `require` chain read, under
    /// `canonical_key`, sorted.
    #[must_use]
    pub fn reads(&self) -> &[PathBuf] {
        &self.reads
    }

    /// The declarations a borrowing walk places behind its own.
    #[must_use]
    pub fn sites(&self) -> &[Site] {
        self.map.sites()
    }

    /// Whether this program lends its map to `path`: its walk read the file,
    /// the file is under a root one of its own declarations names, or the file
    /// is under the directory of its entry and is plain.
    ///
    /// `plain` says whether the text of `path` holds neither `require` nor
    /// `autoload` ([`is_plain`]). It is asked only when the directory test
    /// needs it, so a caller can read the text lazily. Whether `path` is itself
    /// a lender is the caller's question: a lender never borrows.
    pub fn lends_to(&self, path: &Path, plain: impl FnOnce() -> bool) -> bool {
        let key = canonical_key(path);
        self.reads.binary_search(&key).is_ok()
            || self.map.claims(path)
            || (key.starts_with(&self.dir) && plain())
    }
}

/// Whether `text` holds neither `require` nor `autoload`, so the file starts no
/// program of its own.
#[must_use]
pub fn is_plain(text: &str) -> bool {
    !text.contains("require") && !text.contains("autoload")
}

/// The first lender, in entry-path order, among the `.nvs` files under `root`
/// that lends its map to `path`.
///
/// The files are walked one by one and the search stops at the first lender
/// that lends, so the cost is one read per file and one walk per program up to
/// that lender. `plain` is [`is_plain`] for the text of `path`. `path` itself
/// is skipped: the caller asks only for a file that declares no map of its
/// own.
#[must_use]
pub fn first_lender(root: &Path, path: &Path, plain: bool) -> Option<Lender> {
    let key = canonical_key(path);
    let mut found: Vec<PathBuf> = sources(root)
        .iter()
        .map(|source| canonical_key(source))
        .filter(|source| *source != key)
        .collect();
    found.sort();
    found.dedup();
    found
        .into_iter()
        .filter_map(|source| Lender::walk(SourceMap::new(), &source))
        .find(|lender| lender.lends_to(path, || plain))
}

/// Every `.nvs` file under `root`, recursively, in no particular order.
///
/// A build directory and a dot-directory hold no source anybody wrote, and
/// `vendor` holds source nobody here edits, so none of them is entered. A
/// directory this process cannot read is skipped: a tree with one unreadable
/// directory in it is still a tree.
#[must_use]
pub fn sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect_sources(root, &mut found);
    found
}

fn collect_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let skip = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "target" || name == "vendor" || name.starts_with('.'));
            if !skip {
                collect_sources(&path, found);
            }
        } else if path.extension().is_some_and(|ext| ext == "nvs") {
            found.push(path);
        }
    }
}
