//! The configuration tree and the extension set each analysis types against,
//! read once per change to the files they came from.
//!
//! An analysis types its program against the extension set the document's
//! `nvs.toml` pins ([`crate::document`]'s `extension_set`). Reading that set
//! means finding the tree, parsing every file of it, and reading and hashing
//! every `.nvsx` it pins. A cold index analyses every file of the workspace, and
//! nearly all of them share one tree, so reading it per analysis parses the same
//! `nvs.toml` once per file.
//!
//! # Decision: a cached answer is checked against every file it read
//!
//! Every analysis finds its root files again, as `nvs check` finds them from
//! the directory it runs in: the `nvs.toml` beside the document, else the data
//! folder's. That is one or two tests that a file exists. So an `nvs.toml`
//! created beside a document reaches the next analysis.
//!
//! The tree read from those root files is kept per list of root files, so a
//! settings file many directories share is parsed once. Each kept tree has
//! every path the reader touched while it read it: each file it read, each
//! file it looked for and did not find, and each directory it listed. Each
//! path has its size and modification time, or none when it was not there. A
//! lookup reads the metadata of those paths again and uses the tree only when
//! all of them match. So an edit to `nvs.toml` or to an include, or a file
//! added to an included directory, reaches the next analysis with no edit to
//! the source.
//!
//! The snapshot for the document is built from the kept tree on every
//! analysis, because an `[[app]]` block matches one entry file and not a
//! directory. It reads no configuration file.
//!
//! An extension set is kept per list of entries, each a `.nvsx` path and its
//! `sha256`, with the metadata of every `.nvsx` it read. Its digest is hashed
//! once, when it is read, and the graph key of every analysis takes the digest.
//!
//! A file written twice within the file system's time resolution with the same
//! size is the one change a lookup misses, the same trade the index file's stamp
//! makes (`rule:ide/the-index-is-incremental-and-persistent`).
//!
//! # What it costs
//!
//! One kept tree per distinct list of root files, and one kept set per
//! distinct list of entries, for the session. A lookup tests whether one or two
//! root files exist and reads the metadata of each path the tree touched,
//! usually one or two, in place of reading and parsing the tree and hashing
//! each `.nvsx`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

use nvs_config::resolve::{Files, Unowned, resolve, roots};
use nvs_config::trust::Untrusted;
use nvs_diagnostics::SourceMap;
use nvs_ext::manifest::Manifest;

/// One extension set, and the digest the graph key hashes in its place.
#[derive(Clone, Debug)]
pub(crate) struct Extensions {
    /// The built-in components' manifests, then the tree's.
    pub(crate) manifests: Arc<Vec<Manifest>>,
    /// The BLAKE3 hash of `manifests`, computed once when the set was read.
    pub(crate) digest: blake3::Hash,
}

impl Extensions {
    fn of(manifests: Vec<Manifest>) -> Self {
        let digest = blake3::hash(format!("{manifests:?}").as_bytes());
        Self {
            manifests: Arc::new(manifests),
            digest,
        }
    }
}

/// A path's size and modification time, or `None` when it is not there.
type Stamp = Option<(u64, Option<SystemTime>)>;

fn stamp(path: &Path) -> Stamp {
    std::fs::metadata(path)
        .ok()
        .map(|meta| (meta.len(), meta.modified().ok()))
}

/// Whether every path still has the stamp it had when it was read.
fn current(stamps: &[(PathBuf, Stamp)]) -> bool {
    stamps.iter().all(|(path, was)| stamp(path) == *was)
}

/// Each path in `seen` once, with its stamp now.
fn stamps_of(mut seen: Vec<PathBuf>) -> Vec<(PathBuf, Stamp)> {
    seen.sort();
    seen.dedup();
    seen.into_iter()
        .map(|path| {
            let now = stamp(&path);
            (path, now)
        })
        .collect()
}

/// One list of root files, read: `None` for a tree that does not resolve, whose
/// documents take the built-in set.
struct KeptTree {
    stamps: Vec<(PathBuf, Stamp)>,
    tree: Option<Box<nvs_config::Resolved>>,
}

struct KeptSet {
    stamps: Vec<(PathBuf, Stamp)>,
    set: Extensions,
}

/// The kept trees and sets of one document store.
#[derive(Default)]
pub(crate) struct TreeCache {
    trees: HashMap<Vec<PathBuf>, KeptTree>,
    sets: HashMap<blake3::Hash, KeptSet>,
}

impl std::fmt::Debug for TreeCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeCache")
            .field("trees", &self.trees.len())
            .field("sets", &self.sets.len())
            .finish()
    }
}

/// The built-in components' manifests, read once per process from bytes the
/// binary carries. A built-in component that does not read is a broken build,
/// which every `nvs check` reports, and the editor types against the rest.
fn built_in() -> &'static Extensions {
    static BUILT_IN: OnceLock<Extensions> = OnceLock::new();
    BUILT_IN.get_or_init(|| Extensions::of(nvs_ext::load::builtin_manifests().unwrap_or_default()))
}

impl TreeCache {
    /// The extension set for the document at `path`, with `data` as the data
    /// folder's `nvs.toml`. `crate::document`'s `extension_set` says which.
    pub(crate) fn extensions(&mut self, path: &Path, data: Option<&Path>) -> Extensions {
        let Some(dir) = path.parent() else {
            return built_in().clone();
        };
        let nvs_config::Roots::Files(files) = roots(&[], dir, data, &Unowned) else {
            return built_in().clone();
        };
        if !self
            .trees
            .get(&files)
            .is_some_and(|kept| current(&kept.stamps))
        {
            let kept = read_tree(files.clone());
            self.trees.insert(files.clone(), kept);
        }
        let Some(KeptTree {
            tree: Some(resolved),
            ..
        }) = self.trees.get(&files)
        else {
            return built_in().clone();
        };
        let snapshot = nvs_config::Snapshot::build(resolved, path, &Unowned)
            .or_else(|_| nvs_config::Snapshot::host(resolved, &Unowned));
        let Ok(snapshot) = snapshot else {
            return built_in().clone();
        };
        let entries: Vec<nvs_ext::load::Entry> = snapshot
            .config
            .extension
            .iter()
            .enumerate()
            .map(|(index, written)| nvs_ext::load::Entry {
                path: nvs_config::extension::file(
                    index,
                    written.path.as_deref().unwrap_or_default(),
                    &resolved.origins,
                ),
                sha256: written.sha256.clone().unwrap_or_default(),
                memory: None,
                grants: nvs_config::extension::Granted::default(),
            })
            .collect();
        if entries.is_empty() {
            return built_in().clone();
        }
        let mut hasher = blake3::Hasher::new();
        for entry in &entries {
            hasher.update(entry.path.as_os_str().as_encoded_bytes());
            hasher.update(&[0]);
            hasher.update(entry.sha256.as_bytes());
            hasher.update(&[0]);
        }
        let named = hasher.finalize();
        if let Some(kept) = self.sets.get(&named)
            && current(&kept.stamps)
        {
            return kept.set.clone();
        }
        let stamps = stamps_of(entries.iter().map(|entry| entry.path.clone()).collect());
        let mut manifests = built_in().manifests.as_ref().clone();
        manifests.extend(nvs_ext::load::read_manifests(&entries).unwrap_or_default());
        let set = Extensions::of(manifests);
        self.sets.insert(
            named,
            KeptSet {
                stamps,
                set: set.clone(),
            },
        );
        set
    }
}

/// The tree read from `files`, and every path the reader touched.
fn read_tree(files: Vec<PathBuf>) -> KeptTree {
    let recording = Recording::default();
    let tree = resolve(
        &nvs_config::Roots::Files(files),
        &mut SourceMap::new(),
        &recording,
    )
    .ok()
    .map(Box::new);
    KeptTree {
        stamps: stamps_of(recording.seen.into_inner()),
        tree,
    }
}

/// [`Unowned`], and every path it was asked about.
#[derive(Default)]
struct Recording {
    seen: RefCell<Vec<PathBuf>>,
}

impl Recording {
    fn saw(&self, path: &Path) {
        self.seen.borrow_mut().push(path.to_path_buf());
    }
}

impl Files for Recording {
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        self.saw(path);
        Unowned.trust(path)
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        self.saw(path);
        Unowned.canonical(path)
    }

    fn canonical_block(&self, path: &Path) -> Result<PathBuf, String> {
        self.saw(path);
        Unowned.canonical_block(path)
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        self.saw(path);
        Unowned.read(path)
    }

    fn read_config(&self, path: &Path) -> Result<String, String> {
        self.saw(path);
        Unowned.read_config(path)
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.saw(path);
        Unowned.read_bytes(path)
    }

    fn exposure(&self, path: &Path) -> Option<String> {
        Unowned.exposure(path)
    }

    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        self.saw(dir);
        Unowned.list(dir)
    }

    fn exists(&self, path: &Path) -> bool {
        self.saw(path);
        Unowned.exists(path)
    }

    fn missing(&self, path: &Path) -> bool {
        self.saw(path);
        Unowned.missing(path)
    }
}
