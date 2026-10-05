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
//! **The table follows a reload.** Each pass reads `[server] root` and
//! `[[server.mount]]` from the host's snapshot a reload published last. Where
//! either changed, the pass expands the published tree at once
//! ([`Rescan::rewritten`]), with no wait for `settle`, and renders a refusal
//! against the source map that reload recorded
//! (`crate::control::Process::sources_of`). After any expansion and after any
//! publish, each row takes the `[[app]] origin` of its own entry's snapshot in
//! the set standing then (`super::fold_origins`, ADR 0271). A row the rescan
//! adds is folded into that set as it is read. Either way the pass asks the
//! origin check of every row that changed, and publishes. A reload folds the
//! entries of the rows standing when it runs ([`Mounts::entries`]). A named file over a tree that writes no
//! `[[server.mount]]` is one row that is never expanded again, and a reload
//! that removes the last block leaves that row. Boot's check that a named
//! file is one of the table's entries is not asked again.
//!
//! **The table's three switches follow a reload.** `[server] dispatch` and
//! `static`, resolved against the mode by
//! `nvs_config::server::switches_for`, and `health_path` are read from the
//! snapshot the request cloned ([`Local::table`]). A core builds a new table
//! when one of them differs from the table it holds, and otherwise keeps it,
//! so a reload that changes only `[mode] default` builds a new one too.
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
use nvs_config::resolve::Files;
use nvs_config::tree::Mount;
use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap};
use nvs_server::Table;

use crate::config::LocalFiles;
use crate::script::Compiler;

/// The rows every core answers from, and how many times they were replaced.
#[derive(Debug)]
pub(crate) struct Mounts {
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

    /// The entry file of every row standing now, which a reload folds before
    /// it publishes (`crate::control`).
    pub(crate) fn entries(&self) -> Vec<PathBuf> {
        self.rows().1.iter().map(|row| row.entry.clone()).collect()
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
    built: RefCell<Built>,
}

/// The table a core built, and what it was built from.
struct Built {
    /// The generation of the rows it holds.
    generation: u64,
    /// The snapshot its switches were read from. Compared by pointer first,
    /// so a request under the same snapshot compares nothing else.
    of: Arc<nvs_config::Snapshot>,
    table: Rc<Table>,
}

/// What a table is built with besides its rows: the dispatch and the
/// static-file switch as the mode resolves them, and the health path. The
/// switches are compared resolved, so a reload that changes only the mode
/// builds a new table.
fn switches(config: &nvs_config::Config) -> (nvs_config::server::Switches, Option<&str>) {
    (
        nvs_config::server::switches_for(config),
        config
            .server
            .as_ref()
            .and_then(|server| server.health_path.as_deref()),
    )
}

impl Local {
    pub(super) fn new(mounts: Arc<Mounts>, snapshot: Arc<nvs_config::Snapshot>) -> Self {
        let (generation, rows) = mounts.rows();
        let table = Rc::new(Table::from_config(rows.to_vec(), &snapshot.config));
        Self {
            mounts,
            built: RefCell::new(Built {
                generation,
                of: snapshot,
                table,
            }),
        }
    }

    /// The table a request under `snapshot` selects its mount from. This is
    /// the table the core built, unless the rows were replaced since or
    /// `snapshot` sets different switches. Then the core builds a new one.
    ///
    /// A request under the snapshot the table was built from pays one atomic
    /// load and one pointer comparison.
    pub(super) fn table(&self, snapshot: &Arc<nvs_config::Snapshot>) -> Rc<Table> {
        let standing = self.mounts.generation.load(Ordering::Acquire);
        let mut built = self.built.borrow_mut();
        let same_rows = built.generation == standing;
        if same_rows && Arc::ptr_eq(&built.of, snapshot) {
            return Rc::clone(&built.table);
        }
        if same_rows && switches(&built.of.config) == switches(&snapshot.config) {
            built.of = Arc::clone(snapshot);
            return Rc::clone(&built.table);
        }
        let (generation, rows) = self.mounts.rows();
        *built = Built {
            generation,
            of: Arc::clone(snapshot),
            table: Rc::new(Table::from_config(rows.to_vec(), &snapshot.config)),
        };
        Rc::clone(&built.table)
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
/// every directory a fixed segment or an entry file was looked for in are
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

    fn canonical_block(&self, path: &Path) -> Result<PathBuf, String> {
        self.above(path);
        LocalFiles.canonical_block(path)
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        LocalFiles.read(path)
    }

    fn read_config(&self, path: &Path) -> Result<String, String> {
        LocalFiles.read_config(path)
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
    /// The tree the rows were last expanded from. `[server] root`,
    /// `[[server.mount]]` and the origins of both are read from it.
    expanded_from: Arc<nvs_config::Snapshot>,
    /// The source map [`Self::expanded_from`]'s origins point into, which a
    /// refusal's line is rendered from.
    sources: Arc<SourceMap>,
    /// The file the command named, or `None` where it named none. Over a tree
    /// that writes no `[[server.mount]]` it is the one row.
    named: Option<PathBuf>,
    /// The tree a reload publishes, which `[[app]] origin`, `[server] root`
    /// and `[[server.mount]]` are read from.
    current: Arc<nvs_config::Current>,
    /// The rows the last expansion gave, before `[[app]] origin` was folded
    /// into them.
    written: Vec<Mounted>,
    /// The publish whose entry snapshots the rows standing now took their
    /// `[[app]] origin` from.
    folded: Arc<nvs_config::Published>,
    mounts: Arc<Mounts>,
    stamps: BTreeMap<PathBuf, Stamp>,
    /// The refusals the last expansion logged. An expansion logs only what is
    /// not in here, so a match that stays refused is logged once and not on
    /// every pass.
    refused: BTreeSet<String>,
    /// The origin sentences the last fold logged, kept for the same reason.
    unreached: BTreeSet<String>,
}

/// What an expansion reads from a tree: `[server] root`, the file that wrote
/// it, which a relative root is resolved against, and `[[server.mount]]`.
fn mounting(snapshot: &nvs_config::Snapshot) -> (Option<&str>, Option<&Path>, &[Mount]) {
    let server = snapshot.config.server.as_ref();
    (
        server.and_then(|server| server.root.as_deref()),
        snapshot
            .origins
            .get("server.root")
            .map(|origin| origin.path.as_path()),
        server.map_or(&[][..], |server| server.mount.as_slice()),
    )
}

impl Rescan {
    /// The expansion that follows boot's, which expanded `snapshot` into
    /// `written` and took `stamps`. Boot folded each row's own `[[app]]
    /// origin` out of the set `current` serves.
    pub(super) fn new(
        snapshot: Arc<nvs_config::Snapshot>,
        sources: SourceMap,
        named: Option<PathBuf>,
        current: Arc<nvs_config::Current>,
        mounts: Arc<Mounts>,
        written: Vec<Mounted>,
        stamps: BTreeMap<PathBuf, Stamp>,
    ) -> Self {
        let folded = current.published();
        Self {
            expanded_from: snapshot,
            sources: Arc::new(sources),
            named,
            current,
            written,
            folded,
            mounts,
            stamps,
            refused: BTreeSet::new(),
            unreached: BTreeSet::new(),
        }
    }

    /// One pass: nothing while no directory moved and no reload published,
    /// and otherwise the rows folded again and published where they changed.
    /// The module doc lists what happens to each row.
    ///
    /// Answers how long until a change it held back is quiet, and `None`
    /// where it held none back, as `Compiler::revalidate` does.
    pub(super) fn pass(&mut self, compiler: &Compiler) -> Option<Duration> {
        let published = self.current.published();
        let serving = Arc::clone(published.host());
        let (expanded, wait) = if mounting(&serving) == mounting(&self.expanded_from) {
            self.expand(compiler.settle())
        } else {
            (self.rewritten(serving), None)
        };
        if !expanded && Arc::ptr_eq(&published, &self.folded) {
            return wait;
        }
        self.folded = published;
        let mut rows = self.written.clone();
        super::fold_origins(&mut rows, &self.folded);
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
        let mut said = BTreeSet::new();
        for sentence in unreached {
            if !self.unreached.contains(&sentence) {
                eprintln!("warning: left out of the mount table: {sentence}");
            }
            said.insert(sentence);
        }
        self.unreached = said;
        if *standing != *rows {
            self.mounts.publish(rows);
        }
        wait
    }

    /// Expands the configuration's blocks again into [`Self::written`] where a
    /// directory moved and has been still for `settle`.
    ///
    /// Answers whether it expanded, and how long until a directory that moved
    /// is still. A named file over a tree that writes no `[[server.mount]]`
    /// stamped no directory, so it is never expanded again.
    fn expand(&mut self, settle: Duration) -> (bool, Option<Duration>) {
        let now = SystemTime::now();
        let moved: Vec<Stamp> = self
            .stamps
            .iter()
            .map(|(dir, was)| (modified(dir), was))
            .filter(|(is, was)| is != *was)
            .map(|(is, _)| is)
            .collect();
        if moved.is_empty() {
            return (false, None);
        }
        if let Some(newest) = moved.iter().flatten().filter(|at| **at <= now).max()
            && let Ok(wait) = (*newest + settle).duration_since(now)
            && !wait.is_zero()
        {
            return (false, Some(wait));
        }
        (self.expanded(), None)
    }

    /// Expands `serving`, a published tree whose `[server] root` or
    /// `[[server.mount]]` differ from the tree the rows were expanded from, at
    /// once and without waiting for `settle`: the operator saved the change.
    ///
    /// Answers whether it expanded. A tree whose source map the reload has not
    /// recorded yet is expanded on the next pass.
    fn rewritten(&mut self, serving: Arc<nvs_config::Snapshot>) -> bool {
        let Some(sources) =
            crate::control::installed().and_then(|process| process.sources_of(serving.generation))
        else {
            return false;
        };
        self.expanded_from = serving;
        self.sources = sources;
        self.expanded()
    }

    /// Expands [`Self::expanded_from`] into [`Self::written`], and answers
    /// whether it did.
    ///
    /// A tree that writes `[[server.mount]]` is expanded against the disk,
    /// and its directories are stamped. A tree that writes none is the named
    /// file's one row, which stamps nothing. Where no file was named, or the
    /// named one cannot be served, the rows stay as they are, and the reason
    /// is logged.
    fn expanded(&mut self) -> bool {
        let snapshot = Arc::clone(&self.expanded_from);
        if !super::writes_mounts(&snapshot) {
            self.stamps = BTreeMap::new();
            let row = self.named.as_deref().map(super::one_mount);
            return match row {
                Some(Ok(row)) => {
                    self.written = vec![row];
                    true
                }
                Some(Err(sentence)) => {
                    eprintln!("warning: the mount table stays as it was: {sentence}");
                    false
                }
                None => {
                    eprintln!(
                        "warning: the mount table stays as it was: no file was named and the \
                         configuration writes no `[[server.mount]]`"
                    );
                    false
                }
            };
        }
        let stamping = Stamping::default();
        let expanded =
            nvs_config::mount::expand_again(&snapshot.config, &snapshot.origins, &stamping);
        self.stamps = stamping.stamps();
        match expanded {
            Ok((rows, left_out)) => {
                self.log(left_out);
                self.written = rows;
                true
            }
            // The configuration's own problem, which is `[server] root` gone:
            // the rows stay as they are until that directory is back.
            Err(diagnostic) => {
                self.log(vec![diagnostic]);
                false
            }
        }
    }

    /// Writes each refusal the last expansion did not already write.
    fn log(&mut self, refused: Vec<Diagnostic>) {
        let mut said = BTreeSet::new();
        let mut diags = Diagnostics::new();
        for diagnostic in refused {
            let message = diagnostic.message.clone();
            if !self.refused.contains(&message) {
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
        self.refused = said;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A published snapshot over the configuration file `text`.
    fn snapshot(text: &str) -> Arc<nvs_config::Snapshot> {
        Arc::new(nvs_config::Snapshot {
            config: toml::from_str(text).expect("the fixture did not deserialize"),
            ..nvs_config::Snapshot::default()
        })
    }

    /// `rule:config/a-startup-default-is-never-flipped` at a reload: a snapshot
    /// that differs only in `[mode] default` resolves different switches, so
    /// the core builds a new table. A new snapshot with the same mode keeps
    /// the table the core holds.
    #[test]
    fn a_snapshot_that_differs_only_in_its_mode_rebuilds_the_table() {
        let local = Local::new(
            Arc::new(Mounts::new(Vec::new())),
            snapshot("[mode]\ndefault = \"production\"\n"),
        );
        let first = local.table(&snapshot("[mode]\ndefault = \"production\"\n"));
        let same = local.table(&snapshot("[mode]\ndefault = \"production\"\n"));
        assert!(
            Rc::ptr_eq(&first, &same),
            "a snapshot with the same switches rebuilt the table"
        );
        let development = local.table(&snapshot("[mode]\ndefault = \"development\"\n"));
        assert!(
            !Rc::ptr_eq(&first, &development),
            "a snapshot that changed only the mode kept the old table"
        );
    }
}
