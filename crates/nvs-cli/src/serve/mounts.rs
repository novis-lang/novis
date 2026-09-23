//! `rule:http-server/a-mount-table-expands-at-boot`'s table while the server
//! runs: the rows every core answers from, and the background expansion that
//! keeps them following the disk.
//!
//! **One set of rows for the process, and a table per core.** [`Mounts`] holds
//! the rows and a generation number. A core keeps its own
//! `nvs_server::mount::Table` over them, because a `Table` belongs to the
//! thread that reads it, and builds a new one when the generation moved
//! ([`Local::table`]). While the rows stand still, a request pays one atomic
//! load for this and takes no lock.
//!
//! **The expansion runs on `crate::script::watch`'s thread**, after each pass
//! of the compiled-unit check ([`Rescan::pass`]). It keeps the stamp of every
//! directory the last expansion looked in ([`Stamping`]), taken before it
//! looked. While none of them moved, a pass is one `stat` per directory. When
//! one moved, the pass waits until the newest of them has been still for
//! `[opcache] settle`, and then expands the configuration's blocks again with
//! `nvs_config::mount::expand_again`:
//!
//! - A match boot would refuse is logged and left out, and the rest of the
//!   table is published.
//! - A row the table did not hold is compiled first, and
//!   `rule:routing/an-origin-is-per-mount-and-checked-at-boot`'s check is asked
//!   of it. A row that fails the check is logged and left out.
//! - A row whose entry does not compile is kept, so its own requests fail and
//!   no other row's do (`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`).
//! - A row the disk no longer has is gone from the table, and its requests
//!   answer `404`.
//!
//! No path is derived from a URL here either
//! (`rule:http-server/a-path-is-never-derived-from-a-url`): the rows are still
//! enumerated from the configuration's globs, only more than once.
//!
//! **What it spends:** one `stat` per directory the expansion looked in, per
//! `revalidate_freq`, off the request path. Two sets of rows live during a
//! swap, and each core rebuilds its table once per swap.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use nvs_config::mount::Mounted;
use nvs_config::resolve::{Files, Origin};
use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap};
use nvs_server::Table;

use crate::config::LocalFiles;
use crate::script::Compiler;

/// The rows every core answers from, and how many times they were replaced.
#[derive(Debug)]
pub(super) struct Mounts {
    /// Moved once per [`Mounts::publish`], always under [`Self::rows`]' lock, so
    /// a reader that sees it move and then takes the lock reads the rows of
    /// that generation or a later one.
    generation: AtomicU64,
    rows: Mutex<(u64, Arc<[Mounted]>)>,
}

impl Mounts {
    /// The rows boot expanded.
    pub(super) fn new(rows: Vec<Mounted>) -> Self {
        Self {
            generation: AtomicU64::new(0),
            rows: Mutex::new((0, rows.into())),
        }
    }

    /// The rows standing now, with their generation.
    fn rows(&self) -> (u64, Arc<[Mounted]>) {
        let held = self.rows.lock().unwrap_or_else(PoisonError::into_inner);
        (held.0, Arc::clone(&held.1))
    }

    /// Replaces the rows. A request already running keeps the row it
    /// selected, and the next request on each core reads the new ones.
    fn publish(&self, rows: Vec<Mounted>) {
        let mut held = self.rows.lock().unwrap_or_else(PoisonError::into_inner);
        let generation = held.0 + 1;
        *held = (generation, rows.into());
        self.generation.store(generation, Ordering::Release);
    }
}

/// One core's table over the process's [`Mounts`].
pub(super) struct Local {
    mounts: Arc<Mounts>,
    /// The configuration the table's switches are read from: the dispatch,
    /// the static-file switch and the health path. All three are `Boot`-class.
    snapshot: Arc<nvs_config::Snapshot>,
    built: RefCell<(u64, Rc<Table>)>,
}

impl Local {
    pub(super) fn new(mounts: Arc<Mounts>, snapshot: Arc<nvs_config::Snapshot>) -> Self {
        let (generation, rows) = mounts.rows();
        let table = Rc::new(Table::from_config(rows.to_vec(), &snapshot.config));
        Self {
            mounts,
            snapshot,
            built: RefCell::new((generation, table)),
        }
    }

    /// The table a request selects its mount from: the one this core built,
    /// unless the rows were replaced since, and then one built over the new
    /// rows.
    pub(super) fn table(&self) -> Rc<Table> {
        let standing = self.mounts.generation.load(Ordering::Acquire);
        if self.built.borrow().0 != standing {
            let (generation, rows) = self.mounts.rows();
            let table = Rc::new(Table::from_config(rows.to_vec(), &self.snapshot.config));
            *self.built.borrow_mut() = (generation, table);
        }
        Rc::clone(&self.built.borrow().1)
    }
}

/// The stamp a directory is compared by: its modification time, or `None`
/// where it is not there.
type Stamp = Option<SystemTime>;

/// The disk, read through `crate::config::LocalFiles`, recording the stamp of
/// every directory an expansion looks in before it looks.
///
/// A directory's modification time moves when an entry is added to it,
/// removed or renamed. So the stamps of every directory a `*` listed and of
/// every directory a literal segment or an entry file was looked for in are
/// enough to know when an expansion could give a different answer. Where a
/// path's directory is not there, the nearest one above it that is gets
/// stamped, because that is where the missing one will appear.
#[derive(Default)]
pub(super) struct Stamping {
    seen: RefCell<BTreeMap<PathBuf, Stamp>>,
}

impl Stamping {
    /// What was recorded, one stamp per directory.
    pub(super) fn stamps(self) -> BTreeMap<PathBuf, Stamp> {
        self.seen.into_inner()
    }

    /// Records `dir`'s stamp, unless it has one already, and returns whether
    /// `dir` is there.
    fn stamp(&self, dir: &Path) -> bool {
        self.seen
            .borrow_mut()
            .entry(dir.to_path_buf())
            .or_insert_with(|| modified(dir))
            .is_some()
    }

    /// Stamps each directory above `path`, up to and including the first one
    /// that is there.
    fn above(&self, path: &Path) {
        for dir in path.ancestors().skip(1) {
            if self.stamp(dir) {
                break;
            }
        }
    }
}

impl Files for Stamping {
    fn trust(&self, path: &Path) -> Result<PathBuf, nvs_config::trust::Untrusted> {
        LocalFiles.trust(path)
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        self.above(path);
        LocalFiles.canonical(path)
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        LocalFiles.read(path)
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        LocalFiles.read_bytes(path)
    }

    fn exposure(&self, path: &Path) -> Option<String> {
        LocalFiles.exposure(path)
    }

    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        if !self.stamp(dir) {
            self.above(dir);
        }
        LocalFiles.list(dir)
    }

    fn exists(&self, path: &Path) -> bool {
        self.above(path);
        LocalFiles.exists(path)
    }
}

/// `path`'s modification time, or `None` where it is not there.
fn modified(path: &Path) -> Stamp {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// The background expansion: the configuration it expands, the rows it
/// publishes to, and the stamps that say when to expand again.
pub(super) struct Rescan {
    /// The tree this process booted on. `[[server.mount]]` and `[server] root`
    /// are read from it, as boot read them.
    snapshot: Arc<nvs_config::Snapshot>,
    origins: BTreeMap<String, Origin>,
    /// The boot's source map, which a refusal's line is rendered from.
    sources: SourceMap,
    mounts: Arc<Mounts>,
    stamps: BTreeMap<PathBuf, Stamp>,
    /// What the last expansion logged. An expansion logs only what is not in
    /// here, so a match that stays refused is logged once and not on every
    /// pass.
    said: BTreeSet<String>,
}

impl Rescan {
    /// The expansion that follows boot's, which took `stamps`.
    pub(super) fn new(
        snapshot: Arc<nvs_config::Snapshot>,
        origins: BTreeMap<String, Origin>,
        sources: SourceMap,
        mounts: Arc<Mounts>,
        stamps: BTreeMap<PathBuf, Stamp>,
    ) -> Self {
        Self {
            snapshot,
            origins,
            sources,
            mounts,
            stamps,
            said: BTreeSet::new(),
        }
    }

    /// One pass: nothing while no directory moved, and otherwise the table
    /// expanded again and published where it changed. The module doc lists
    /// what happens to each row.
    ///
    /// Answers how long until a change it held back is quiet, and `None`
    /// where it held none back, as `Compiler::revalidate` does.
    pub(super) fn pass(&mut self, compiler: &Compiler) -> Option<Duration> {
        let now = SystemTime::now();
        let moved: Vec<Stamp> = self
            .stamps
            .iter()
            .map(|(dir, was)| (modified(dir), was))
            .filter(|(is, was)| is != *was)
            .map(|(is, _)| is)
            .collect();
        if moved.is_empty() {
            return None;
        }
        if let Some(newest) = moved.iter().flatten().filter(|at| **at <= now).max()
            && let Ok(wait) = (*newest + compiler.settle()).duration_since(now)
            && !wait.is_zero()
        {
            return Some(wait);
        }
        let stamping = Stamping::default();
        let expanded =
            nvs_config::mount::expand_again(&self.snapshot.config, &self.origins, &stamping);
        self.stamps = stamping.stamps();
        let (mut rows, left_out) = match expanded {
            Ok(both) => both,
            // The configuration's own problem, which is `[server] root` gone:
            // the table stays as it is until that directory is back.
            Err(diagnostic) => {
                self.log(vec![diagnostic], Vec::new());
                return None;
            }
        };
        super::fall_back_to(&mut rows, self.snapshot.origin.as_deref());
        let (_, standing) = self.mounts.rows();
        let mut unreached = Vec::new();
        rows.retain(|row| {
            if standing.contains(row) {
                return true;
            }
            match compiler.compiled(&row.entry.to_string_lossy()) {
                Ok((_program, routes)) => match super::unreached_origin(row, &routes) {
                    Some(sentence) => {
                        unreached.push(sentence);
                        false
                    }
                    None => true,
                },
                // The front end has printed why. The row stays, and a request
                // that selects it gets that failure.
                Err(_) => true,
            }
        });
        self.log(left_out, unreached);
        if *standing != *rows {
            self.mounts.publish(rows);
        }
        None
    }

    /// Writes each refusal and each origin sentence the last pass did not
    /// already write.
    fn log(&mut self, refused: Vec<Diagnostic>, unreached: Vec<String>) {
        let mut said = BTreeSet::new();
        let mut diags = Diagnostics::new();
        for diagnostic in refused {
            let message = diagnostic.message.clone();
            if !self.said.contains(&message) {
                diags.report(diagnostic);
            }
            said.insert(message);
        }
        if !diags.is_empty() {
            eprintln!(
                "warning: `[[server.mount]]` left a match out of the table, and serves the rest:"
            );
            crate::render_diagnostics(&mut diags, &self.sources);
        }
        for sentence in unreached {
            if !self.said.contains(&sentence) {
                eprintln!("warning: left out of the mount table: {sentence}");
            }
            said.insert(sentence);
        }
        self.said = said;
    }
}
