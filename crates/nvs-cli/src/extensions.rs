//! `rule:packaging/extension-loading-is-root-controlled` in `nvs serve`: every `[[extension]]`
//! entry loaded at boot and again at every reload, and the set that loaded kept as the live one.
//!
//! **One entry that does not load refuses the whole set.** A boot stops with that entry's
//! refusal, `E0652` at its line. A reload returns it before anything is swapped, so the running
//! configuration and the set loaded with it both stay. A reload reads and verifies every file
//! again, so a file that changed under its pin is refused even where its entry did not change.
//!
//! **The engine is made for the first entry.** A configuration with no `[[extension]]` creates no
//! engine, and its boot and reload pay one empty loop. The engine has the call path's settings —
//! epoch interruption, one call per store — without its pooling allocator. Nothing calls a guest
//! from here yet: a component runs only in the engine that compiled it, so the host that links the
//! world's imports is the one whose engine this loader must compile into once it exists.
//!
//! **A compiled component is kept in the artifact cache a program's units are kept in**
//! (`rule:packaging/a-wasm-module-cache-reuses-the-artifact-cache`). [`Modules`] is
//! `nvs_ext::load::ModuleCache` over [`crate::cache::Cache`], placed where `[opcache]` places it,
//! so a component's entry has a unit's path layout, header, checksum, ownership check and
//! eviction, and a bad one is a miss. Its key is `artifact_key(content_hash(pin ‖ wasmtime's
//! environment), build environment)`, where the build environment is `env_hash` over an empty
//! `[[extension]]` array: a compiled component depends on the compiler build and on wasmtime, never
//! on which other extensions are loaded, so a reload that changes the set recompiles programs and
//! no component whose file did not change. A configuration with no file cache compiles every
//! component at every boot and reload.
//!
//! What it spends: each loaded component's compiled code, once per process and shared by every
//! core, and during a reload the old set and the new one together until the swap. On disk, one
//! entry per component file per build, under the cache's own size cap.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use nvs_config::cache::{Digest, artifact_key, content_hash, env_hash};
use nvs_config::resolve::Origin;
use nvs_config::tree::{Config, Extension};
use nvs_config::value::{Quantity, Unit};
use nvs_diagnostics::{Diagnostic, SourceMap};
use nvs_ext::load::{CacheKey, Entry, Loader, ModuleCache, Set};

use crate::cache::Cache;

/// The process's one loader, made by the first [`loaded`] that has an entry to load.
static LOADER: OnceLock<Loader> = OnceLock::new();

/// The set the configuration now serving loaded, which [`install`] replaces whole.
static LIVE: Mutex<Option<Arc<Set>>> = Mutex::new(None);

/// Every entry of `config`'s `[[extension]]` array, loaded into one set.
///
/// `origins` and `sources` are the resolved tree's, so a refusal points at the entry's line.
///
/// # Errors
///
/// `E0652` for the first entry whose file does not load or whose class an earlier entry
/// declares, and for an engine that cannot be made. Nothing is installed.
pub(crate) fn loaded(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
    sources: &SourceMap,
) -> Result<Set, Diagnostic> {
    let mut set = Set::default();
    let mut cached: Option<Loader> = None;
    for (index, written) in config.extension.iter().enumerate() {
        let entry = entry(index, written, origins);
        let refuse = |reason: &str| {
            nvs_config::extension::not_loaded(
                index,
                &entry.path.to_string_lossy(),
                reason,
                origins,
                sources,
            )
        };
        if cached.is_none() {
            let loader = loader().map_err(|reason| refuse(&reason))?.clone();
            cached = Some(match Modules::placed(config) {
                Some(modules) => loader.with_cache(Arc::new(modules)),
                None => loader,
            });
        }
        let loader = cached.as_ref().expect("the loader is made above");
        let extension = loader
            .load(&entry)
            .map_err(|refused| refuse(&refused.reason))?;
        set.insert(extension)
            .map_err(|refused| refuse(&refused.reason))?;
    }
    Ok(set)
}

/// The manifests of `config`'s `[[extension]]` entries, read without compiling a component: what
/// the compiler types an extension call against (`rule:packaging/extension-calls-are-statically-typed`).
///
/// # Errors
///
/// `E0652` at the first entry whose file, pin or manifest does not read, or whose class an
/// earlier entry declares.
pub(crate) fn manifests(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
    sources: &SourceMap,
) -> Result<Vec<nvs_ext::manifest::Manifest>, Diagnostic> {
    let entries: Vec<Entry> = config
        .extension
        .iter()
        .enumerate()
        .map(|(index, written)| entry(index, written, origins))
        .collect();
    nvs_ext::load::read_manifests(&entries).map_err(|(index, refused)| {
        nvs_config::extension::not_loaded(
            index,
            &entries[index].path.to_string_lossy(),
            &refused.reason,
            origins,
            sources,
        )
    })
}

/// Makes `set` the live one, replacing the set the previous configuration loaded.
pub(crate) fn install(set: Set) {
    *LIVE.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(set));
}

/// The loader, made on the first call.
fn loader() -> Result<&'static Loader, String> {
    if let Some(loader) = LOADER.get() {
        return Ok(loader);
    }
    let mut config = wasmtime::Config::new();
    config.epoch_interruption(true);
    config.concurrency_support(false);
    let engine = wasmtime::Engine::new(&config)
        .map_err(|err| format!("the WebAssembly engine did not start: {err:#}"))?;
    Ok(LOADER.get_or_init(|| Loader::new(&engine)))
}

/// The artifact cache, holding compiled components.
struct Modules {
    cache: Cache,
}

impl Modules {
    /// The cache `config`'s `[opcache]` places, keyed by the build alone. A refused
    /// `file_cache_dir` is no cache here, and the program path is the one that reports it.
    fn placed(config: &Config) -> Option<Self> {
        let mut cache = crate::cache::placed(config).ok().flatten()?;
        cache.rekey(env_hash(&Config::default()));
        Some(Self { cache })
    }

    /// The artifact key `key` is stored under.
    fn key(&self, key: &CacheKey) -> Digest {
        let content = content_hash(format!("{}{}", key.pin, key.environment).as_bytes());
        artifact_key(content, self.cache.env())
    }
}

impl ModuleCache for Modules {
    fn get(&self, key: &CacheKey) -> Option<Vec<u8>> {
        Some(self.cache.load(self.key(key))?.payload().to_vec())
    }

    fn put(&self, key: &CacheKey, bytes: &[u8]) {
        drop(self.cache.store(self.key(key), bytes));
    }
}

/// The loader's entry for the written `[[extension]]` block `index`, its path resolved against
/// the file that wrote it. `nvs_config::extension::validate` has already refused one with no
/// `path`, no pin or a `memory` that is not a size.
fn entry(index: usize, written: &Extension, origins: &BTreeMap<String, Origin>) -> Entry {
    let memory = written.memory.as_ref().and_then(|memory| {
        match Quantity::parse("extension.memory", Unit::Bytes, memory) {
            Ok(Quantity::Bytes(bytes)) => Some(bytes),
            _ => None,
        }
    });
    Entry {
        path: nvs_config::extension::file(
            index,
            written.path.as_deref().unwrap_or_default(),
            origins,
        ),
        sha256: written.sha256.clone().unwrap_or_default(),
        memory,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// The key of a component pinned `pin`, compiled by `environment`.
    fn key(pin: &str, environment: &str) -> CacheKey {
        CacheKey {
            pin: pin.to_owned(),
            environment: environment.to_owned(),
        }
    }

    /// A compiled component is one artifact under the build's environment, read back under its
    /// pin and wasmtime's environment together. A checksum that does not match is a miss, and the
    /// entry is deleted.
    #[test]
    fn a_compiled_component_is_an_artifact_keyed_by_its_pin_and_the_engine() {
        let dir = nvs_repo::scratch_private("ext-modules");
        let cache = Cache::new(dir.path(), env_hash(&Config::default()))
            .expect("a scratch directory of this test's own");
        let modules = Modules { cache };
        let pin = "a".repeat(64);
        modules.put(&key(&pin, "engine"), b"compiled");

        assert_eq!(
            modules.get(&key(&pin, "engine")).as_deref(),
            Some(b"compiled".as_slice())
        );
        assert_eq!(modules.get(&key(&pin, "another engine")), None);
        assert_eq!(modules.get(&key(&"b".repeat(64), "engine")), None);

        let path = modules.cache.path(modules.key(&key(&pin, "engine")));
        let mut bytes = fs::read(&path).expect("the entry is on disk");
        *bytes.last_mut().expect("a payload") ^= 1;
        fs::write(&path, bytes).expect("the entry is writable");
        assert_eq!(modules.get(&key(&pin, "engine")), None);
        assert!(!path.exists(), "a corrupt entry is deleted");
    }
}
