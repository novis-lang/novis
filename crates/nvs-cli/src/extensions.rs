//! `rule:packaging/extension-loading-is-root-controlled` in `nvs serve`: every `[[extension]]`
//! entry loaded at boot and again at every reload, and the set that loaded kept as the live one.
//! And [`Calls`], the host a compiled call into an extension reaches in `nvs run`.
//!
//! **One entry that does not load refuses the whole set.** A boot stops with that entry's
//! refusal, `E0652` at its line. A reload returns it before anything is swapped, so the running
//! configuration and the set loaded with it both stay. A reload reads and verifies every file
//! again, so a file that changed under its pin is refused even where its entry did not change.
//!
//! **The engine is made for the first entry.** A configuration with no `[[extension]]` creates no
//! engine, and its boot and reload pay one empty loop. The engine is the process's one
//! `nvs_ext::call::Host`'s, because a component runs only in the engine that compiled it. The host
//! has room for [`SLOTS`] instances at once and links only the world's own imports; a guest is
//! granted no WASI interface yet.
//!
//! **A call** (`rule:packaging/extension-calls-are-statically-typed`). [`Calls`] implements
//! `nvs_runtime::extension::Extensions` over one `nvs_ext::call::Request`, made at the run's first
//! call. It copies each argument into an `nvs_ext::convert::Value` with no type in hand, and
//! `Request::call_values` converts it by the manifest. The call is a future polled on the calling
//! task: between two polls the task parks for up to one epoch tick, so the other tasks on the core
//! run (`rule:packaging/a-guest-call-yields-on-its-core`). A failure throws the class
//! `Failure::outcome` names (`rule:packaging/a-guest-crash-throws`), and a limit is a `FATAL`.
//! [`Calls::end`] runs `Request::end` once the run's tasks are finished.
//!
//! The request's budget is an `nvs_ext::call::Meter` made at the first call from the request's
//! `[limits]`: its CPU time from then on, and its memory as a second allowance beside the one the
//! runtime counts. A value with no row in the table — an enum case, a `Core` value class, a
//! resource — is refused as `ExtensionError` before the guest runs.
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

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

use nvs_config::cache::{Digest, artifact_key, content_hash, env_hash};
use nvs_config::resolve::Origin;
use nvs_config::tree::{Config, Extension};
use nvs_config::value::{Quantity, Unit};
use nvs_diagnostics::{Diagnostic, SourceMap};
use nvs_ext::call::{Host, Meter, Outcome, Request};
use nvs_ext::convert::{Key, Value as Crossed};
use nvs_ext::load::{CacheKey, Entry, Loader, ModuleCache, Set};
use nvs_runtime::{Ctx, Fault, HelperResult, NvsArray, NvsStr, SlotKey, Tag, ThrownClass, Value};

use crate::cache::Cache;

/// How many instances the process's host has room for at once, across every request. Each slot
/// reserves address space for one instance, not committed memory.
const SLOTS: u32 = 64;

/// The CPU time a request with no `[limits] cpu_time` gives its guests.
const UNCAPPED: Duration = Duration::from_secs(365 * 24 * 60 * 60);

/// The process's one host, made by the first [`loaded`] that has an entry to load.
static HOST: OnceLock<Host> = OnceLock::new();

/// The process's one loader, compiling into [`HOST`]'s engine.
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
    let host = host()?;
    Ok(LOADER.get_or_init(|| Loader::new(host.engine())))
}

/// The host, made on the first call.
fn host() -> Result<&'static Host, String> {
    if let Some(host) = HOST.get() {
        return Ok(host);
    }
    let host = Host::new(SLOTS, |_| Ok(()))
        .map_err(|err| format!("the WebAssembly engine did not start: {err:#}"))?;
    Ok(HOST.get_or_init(|| host))
}

/// The extensions a run loaded, and the request their calls run in.
pub(crate) struct Calls {
    set: Set,
    request: OnceCell<Request>,
}

impl Calls {
    /// The host of calls into `set`.
    pub(crate) fn new(set: Set) -> Self {
        Self {
            set,
            request: OnceCell::new(),
        }
    }

    /// Ends the request: every resource a guest returned is dropped in its guest. A failure of a
    /// destructor is the extension's, and the run has already finished, so it is not reported.
    pub(crate) fn end(self) {
        if let Some(request) = self.request.into_inner() {
            drop(drive(request.end()));
        }
    }

    /// The request, made at the first call from `ctx`'s limits.
    fn request(&self, ctx: &Ctx) -> Result<&Request, Fault> {
        if let Some(request) = self.request.get() {
            return Ok(request);
        }
        let host = host().map_err(|reason| Fault::thrown_as(ThrownClass::Extension, reason))?;
        let cpu = match ctx.cpu_limit() {
            0 => UNCAPPED,
            nanos => Duration::from_nanos(nanos),
        };
        let memory = match ctx.memory_limit() {
            0 => None,
            bytes => u64::try_from(bytes).ok(),
        };
        Ok(self
            .request
            .get_or_init(|| host.request(Arc::new(Meter::new(cpu, memory)))))
    }
}

impl nvs_runtime::extension::Extensions for Calls {
    fn call(&self, ctx: &mut Ctx, class: &str, method: &str, args: &[Value]) -> HelperResult {
        let refused = |reason: String| {
            Fault::thrown_as(
                ThrownClass::Extension,
                format!("`{class}::{method}`: {reason}"),
            )
        };
        let extension = self
            .set
            .extensions()
            .iter()
            .find(|extension| extension.manifest.class == class)
            .ok_or_else(|| refused("the extension is not loaded".to_owned()))?;
        let args = args
            .iter()
            .map(|arg| crossed(*arg))
            .collect::<Result<Vec<_>, _>>()
            .map_err(refused)?;
        let request = self.request(ctx)?;
        match drive(request.call_values(extension, method, args)) {
            Ok(None) => Ok(Value::null()),
            Ok(Some(result)) => runtime(result).map_err(refused),
            Err(failure) => Err(match failure.outcome() {
                Outcome::Throw(class, message) => {
                    let thrown = ThrownClass::ALL
                        .iter()
                        .copied()
                        .find(|thrown| thrown.name() == class.name())
                        .unwrap_or(ThrownClass::Extension);
                    Fault::thrown_as(thrown, message)
                }
                Outcome::Fatal(_, message) => Fault::fatal(message),
            }),
        }
    }
}

/// Runs `call` to the end on the calling task, parking it for up to a tick between polls.
fn drive<T>(call: impl Future<Output = T>) -> T {
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut call = std::pin::pin!(call);
    loop {
        if let Poll::Ready(out) = call.as_mut().poll(&mut cx) {
            return out;
        }
        let deadline = Instant::now() + nvs_ext::call::TICK;
        if nvs_runtime::host::with_current(|host| host.park(Some(deadline))).is_none() {
            std::thread::park_timeout(nvs_ext::call::TICK);
        }
    }
}

/// A waker that unparks the thread that polls.
struct Unpark(std::thread::Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// `value` as it crosses into a guest.
fn crossed(value: Value) -> Result<Crossed, String> {
    Ok(match value.tag() {
        Some(Tag::Null) => Crossed::Null,
        Some(Tag::Bool) => Crossed::Bool(value.as_bool().unwrap_or_default()),
        Some(Tag::Int) => Crossed::Int(value.as_int().unwrap_or_default()),
        Some(Tag::Uint) => Crossed::Uint(value.as_uint().unwrap_or_default()),
        Some(Tag::Float) => Crossed::Float(value.as_float().unwrap_or_default()),
        Some(Tag::Str) => Crossed::String(value.as_text().unwrap_or_default().to_owned()),
        Some(Tag::Bytes) => Crossed::Bytes(value.as_bytes().unwrap_or_default().to_vec()),
        Some(Tag::Array) => {
            let array = value.as_array().ok_or("an array with no array in it")?;
            let mut entries = Vec::with_capacity(array.count());
            let mut from = 0;
            while let Some(slot) = array.next_slot(from) {
                let key = match array.slot_key(slot) {
                    Some(SlotKey::Index(index)) => Key::Int(index),
                    Some(SlotKey::Str(key)) => key_of(key.as_bytes())?,
                    None => return Err("an array entry with no key".to_owned()),
                };
                let element = array.value_at(slot).ok_or("an array entry with no value")?;
                entries.push((key, crossed(element)?));
                from = slot + 1;
            }
            Crossed::Array(entries)
        }
        Some(Tag::Object) => Crossed::Object(value.class_name().unwrap_or_default()),
        Some(tag) => {
            return Err(format!(
                "a `{}` does not cross into an extension",
                tag.describe()
            ));
        }
        None => return Err("a value with no type does not cross into an extension".to_owned()),
    })
}

/// A hashed array's key: an `int` where the runtime would have stored the text as one.
fn key_of(bytes: &[u8]) -> Result<Key, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "an array key that is not text")?;
    Ok(match text.parse::<i64>() {
        Ok(number) if number.to_string() == text => Key::Int(number),
        _ => Key::String(text.to_owned()),
    })
}

/// `value`, returned by a guest, as a runtime value the caller owns.
fn runtime(value: Crossed) -> Result<Value, String> {
    Ok(match value {
        Crossed::Null => Value::null(),
        Crossed::Bool(flag) => Value::bool(flag),
        Crossed::Int(number) => Value::int(number),
        Crossed::Uint(number) => Value::uint(number),
        Crossed::Float(number) => Value::float(number),
        Crossed::String(text) => Value::str(NvsStr::new(text.as_bytes())),
        Crossed::Bytes(octets) => Value::bytes(NvsStr::new(&octets)),
        Crossed::Array(entries) => {
            let mut array = NvsArray::new();
            for (key, element) in entries {
                let element = runtime(element)?;
                match key {
                    Key::Int(index) => array.set_index(index, element),
                    Key::String(key) => array.set(NvsStr::new(key.as_bytes()), element),
                }
            }
            Value::array(array)
        }
        Crossed::Case(name) => {
            return Err(format!("the enum case `{name}` does not cross back yet"));
        }
        Crossed::Core { class, .. } => {
            return Err(format!("a `{class}` does not cross back yet"));
        }
        Crossed::Object(class) => return Err(format!("a `{class}` object does not cross back")),
        Crossed::Resource { name, .. } => {
            return Err(format!("the resource `{name}` does not cross back yet"));
        }
    })
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
