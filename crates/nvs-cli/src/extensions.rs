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
//! What it spends: each loaded component's compiled code, once per process and shared by every
//! core, and during a reload the old set and the new one together until the swap.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use nvs_config::resolve::Origin;
use nvs_config::tree::{Config, Extension};
use nvs_config::value::{Quantity, Unit};
use nvs_diagnostics::{Diagnostic, SourceMap};
use nvs_ext::load::{Entry, Loader, Set};

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
        let loader = loader().map_err(|reason| refuse(&reason))?;
        let extension = loader
            .load(&entry)
            .map_err(|refused| refuse(&refused.reason))?;
        set.insert(extension)
            .map_err(|refused| refuse(&refused.reason))?;
    }
    Ok(set)
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
