//! The compiled-component cache the tests that load built-in components share, kept in files
//! under `target/test-scratch/module-cache/` so it outlives the process.
//!
//! nextest runs every test in a process of its own, and each one compiled the same built-in
//! components again. With this cache the first process to need a component compiles it, and the
//! rest deserialize what it stored, through the same [`ModuleCache`] seam the product's artifact
//! cache fills.
//!
//! An entry is `<environment>/<pin>`, the two halves of the [`CacheKey`], so a component compiled
//! by another engine configuration is never looked at. A store writes a file of its own and
//! renames it into place, so a process reading an entry never sees half of another's write, and
//! two processes storing one key both leave a whole entry. The loader checks every entry's
//! checksum, so a damaged file is a miss and is overwritten.
//!
//! What it spends: the serialized components on disk, one per built-in component and engine
//! configuration a test uses, tens of MiB, which `bun nv disk` counts and `cargo clean` deletes.
//! An entry a rebuilt component no longer has stays until then.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use nvs_ext::load::{CacheKey, ModuleCache};

/// The cache, one per process.
pub(crate) fn shared() -> Arc<dyn ModuleCache> {
    static CACHE: OnceLock<Arc<Files>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            Arc::new(Files {
                root: nvs_repo::shared_scratch("module-cache"),
            })
        })
        .clone()
}

struct Files {
    root: PathBuf,
}

impl Files {
    /// Where `key`'s entry is. Both halves are lower-case hexadecimal, so neither can leave the
    /// root.
    fn entry(&self, key: &CacheKey) -> Option<PathBuf> {
        let hex = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit());
        (hex(&key.environment) && hex(&key.pin))
            .then(|| self.root.join(&key.environment).join(&key.pin))
    }
}

impl ModuleCache for Files {
    fn get(&self, key: &CacheKey) -> Option<Vec<u8>> {
        std::fs::read(self.entry(key)?).ok()
    }

    fn put(&self, key: &CacheKey, bytes: &[u8]) {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let Some(entry) = self.entry(key) else {
            return;
        };
        let Some(dir) = entry.parent() else {
            return;
        };
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        let partial = dir.join(format!(
            "{}.{}-{}.partial",
            key.pin,
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // A rename that fails, as one onto a file another process has open on Windows can, leaves
        // that process's whole entry in place, and this one is deleted.
        if std::fs::write(&partial, bytes).is_err() || std::fs::rename(&partial, &entry).is_err() {
            let _ = std::fs::remove_file(&partial);
        }
    }
}
