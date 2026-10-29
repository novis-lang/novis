//! `rule:packaging/extension-loading-is-root-controlled` in `nvs serve`: every `[[extension]]`
//! entry loaded at boot and again at every reload, and the set that loaded kept as the live one.
//! And [`Calls`], the host a compiled call into an extension reaches in `nvs run`, and
//! [`Served`], which keeps one [`Calls`] per request in `nvs serve`.
//!
//! **One entry that does not load refuses the whole set.** A boot stops with that entry's
//! refusal, `E0652` at its line. A reload returns it before anything is swapped, so the running
//! configuration and the set loaded with it both stay. A reload reads and verifies every file
//! again, so a file that changed under its pin is refused even where its entry did not change.
//!
//! **The engine is made for the first entry.** A configuration with no `[[extension]]` creates no
//! engine, and its boot and reload pay one empty loop. The engine is the process's one
//! `nvs_ext::call::Host`'s, because a component runs only in the engine that compiled it. The host
//! links only the world's own imports. A guest holds the files and hosts its effective grant
//! names, and no other authority.
//!
//! **The host has room for every request in flight.** Its pool of instances is sized once, from
//! the configuration of the first boot ([`size_pool`]): `max_in_flight` times the classes one
//! request can call, so every request in flight can hold an instance of each. [`MOST_SLOTS`] caps
//! it, because each instance reserves address space for its memory. A reload that raises
//! `max_in_flight` or adds a class cannot grow a pool that exists: past the boot's size, a call
//! that needs a new instance throws `ExtensionError`, and the next boot takes the new size.
//!
//! **A call** (`rule:packaging/extension-calls-are-statically-typed`). [`Calls`] implements
//! `nvs_runtime::extension::Extensions` over one `nvs_ext::call::Request`, made at the run's first
//! call. It copies each argument into an `nvs_ext::convert::Value` with no type in hand, and
//! `Request::call_values` converts it by the manifest. The call is a future polled on the calling
//! task: at each epoch tick the task gives the core up for one turn, so the other tasks on the core
//! run (`rule:packaging/a-guest-call-yields-on-its-core`). A failure throws the class
//! `Failure::outcome` names (`rule:packaging/a-guest-crash-throws`). A limit runs the request's
//! limit handler, then is a `FATAL` (`rule:errors/on-limit`).
//! Dropping a [`Calls`] runs `Request::end`, which `nvs run` does once the run's tasks are
//! finished.
//!
//! **In `nvs serve` a request's calls live in its request tree.** Every worker installs
//! [`Served`], which at a request's first call puts a [`Calls`] over the live set into
//! `nvs_runtime::TreeState::extensions`. Every task of the request reaches the same one, and
//! a child script with a narrower `grants:` calls through instances of its own in it. It is
//! dropped, and its instance with it, when the connection waits for its next request and nothing
//! else holds the tree, or else with the tree: when the request and its after-response work
//! have finished, even where a connection gave up on the request first. Its budget holds the
//! tree weakly ([`Spent`]), so nothing keeps a tree alive past its request. A request that started before a
//! reload and makes its first call after it calls into the new set. Its unit was typed
//! against the old one, so a class the reload removed throws `ExtensionError`. The tree is
//! normally dropped on the request's core. A tree whose last task finished on another core is
//! dropped there, and its resources' destructors run on that core.
//!
//! The request's budget is a [`Spent`] made at the first call
//! (`rule:packaging/a-guest-runs-under-the-requests-budget`). A guest's CPU time is the request's
//! own, counted from the request's start, and its linear memory is charged to the memory count the
//! runtime keeps for the request. A guest past either limit ends the request with the
//! resource-limit `FATAL`, after the request's limit handler. A guest's log lines are the request's log records, its wall clock and random bytes are the
//! request's, so a test's fixed clock and seed reach it, and `nvs:ext/settings` reads the
//! `[ext.<name>]` block of the request's snapshot. A shape argument crosses as its set fields by name
//! (`nvs_runtime::Value::shape_fields`). A returned record is converted by the manifest's declared
//! return type, and a shape in it is built as an instance of the class the compiled unit declared
//! for its label (`nvs_runtime::Ctx::new_shape`), which the lowering records for every shape in
//! an extension method's return type. A `Core` value class crosses as its record's fields, which
//! `nvs_stdlib::ext_record` reads from the instance and builds a fresh instance from, with the
//! checks the class's own constructor makes. An enum case is its number, which crosses as an
//! `int` and comes back by `nvs_ext::convert::case_number`. A resource comes back as an object of
//! its class holding the request's number for it, and crosses again as that number
//! (`nvs_types::ext_lib`'s module doc). Only the resource's short name crosses, and the request
//! refuses a number it does not keep for that name in the called extension's instance.
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
//! **The built-in components are always present** (`rule:packaging/the-first-party-components-are-built-in`).
//! [`with_built_in`] puts their manifests in front of the entries' at every compile, read once per
//! process without an engine. A call whose class no loaded entry declares reaches the built-in
//! component that declares it: the first such call makes the process's loader, placed in the
//! artifact cache of the configuration it runs under, and compiles that component, once per
//! process. A tree with no `[[extension]]` is an empty set, so `nvs run` and `nvs test` always
//! install a [`Calls`]. Before a call crosses, `nvs_ext::builtin::cap_pixels` clamps an image
//! call's sources to the `[image] max_pixels` of the request's snapshot
//! (`rule:core-classes/image-pixel-cap`), and `nvs_ext::builtin::cap_decompression` a spreadsheet
//! reader's or template's bound to its `[limits]` (`rule:core-classes/decompression-bound`).
//!
//! What it spends: each loaded component's compiled code, once per process and shared by every
//! core, and during a reload the old set and the new one together until the swap. On disk, one
//! entry per component file per build, under the cache's own size cap.

use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nvs_config::cache::{Digest, artifact_key, content_hash, env_hash};
use nvs_config::resolve::Origin;
use nvs_config::tree::{Config, Extension};
use nvs_config::value::{Quantity, Unit};
use nvs_diagnostics::{Diagnostic, SourceMap};
use nvs_ext::call::{
    Answer, Budget, FileOpen, Host, Incoming, Lender, Lending, Level, Limit, Outbound, Outcome,
    Outgoing, Request, Unsent,
};
use nvs_ext::convert::{Key, Value as Crossed, fits};
use nvs_ext::grants::Caller;
use nvs_ext::load::{Builtin, CacheKey, Entry, Extension as Loaded, Loader, ModuleCache, Set};
use nvs_ext::manifest::Manifest;
use nvs_ext::types::{CORE_CLASSES, CoreRecord, Field, NovisType};
use nvs_runtime::{
    Ctx, Fault, HelperResult, NvsArray, NvsStr, SafepointFlags, SlotKey, Tag, ThrownClass, Value,
    WeakSafepointView,
};
use nvs_stdlib::ext_record::Part;

use crate::cache::Cache;

/// The most instances the process's host has room for at once, across every request. Each slot
/// reserves address space, not committed memory: 6 GiB for its linear memory (wasmtime's 4 GiB
/// reservation, which lets compiled guest code skip bounds checks, and its 2 GiB guard), so 4096
/// slots are 24 TiB of the 128 TiB a 64-bit process can address. The rest stays for task stacks
/// and the JIT.
const MOST_SLOTS: u32 = 4096;

/// The slots of a host made before any configuration was read, as `nvs ext build` makes one to
/// check what it packs. Such a process serves no requests, so a few instances at once are enough.
const UNSIZED_SLOTS: u32 = 64;

/// How many slots [`host`] makes the pool with: [`UNSIZED_SLOTS`] until [`size_pool`] reads a
/// configuration.
static SLOTS: AtomicU32 = AtomicU32::new(UNSIZED_SLOTS);

/// The process's one host, made by the first [`loaded`] that has an entry to load, or by the first
/// call into a built-in component.
static HOST: OnceLock<Host> = OnceLock::new();

/// The process's one loader, compiling into [`HOST`]'s engine.
static LOADER: OnceLock<Loader> = OnceLock::new();

/// The set the configuration now serving loaded, which [`install`] replaces whole.
static LIVE: Mutex<Option<Arc<Set>>> = Mutex::new(None);

/// The built-in components' manifests, read on the first compile.
static BUILT_IN_MANIFESTS: OnceLock<Vec<Manifest>> = OnceLock::new();

/// The built-in components, made on the first call into one, or why the engine did not start.
static BUILT_IN: OnceLock<Result<Vec<Builtin>, String>> = OnceLock::new();

/// The built-in components' manifests, then `manifests`: every class a program is typed against
/// (`rule:packaging/the-first-party-components-are-built-in`). An `[[extension]]` entry cannot
/// declare a `Novis\` class, so no class appears twice.
pub(crate) fn with_built_in(manifests: Vec<Manifest>) -> Vec<Manifest> {
    let built_in = BUILT_IN_MANIFESTS.get_or_init(|| {
        nvs_ext::load::builtin_manifests()
            .unwrap_or_else(|refused| panic!("a built-in component does not read: {refused}"))
    });
    built_in.iter().cloned().chain(manifests).collect()
}

/// The built-in component declaring `class`, compiled on the first call into it. `config` places
/// the compiled form in the artifact cache, when the first call has one. `None` when no built-in
/// component declares `class`.
///
/// The first call is made inside a request, and what it compiles is the process's for as long as
/// it runs, so the compile runs inside a [`Detached`](nvs_runtime::budget::Detached) bracket. A
/// fresh compile allocates on the engine's worker threads and frees on this one, so without the
/// bracket the request's balance fell by those frees, and its ceiling bought it the difference.
fn built_in(class: &str, config: Option<&Config>) -> Option<Result<&'static Loaded, String>> {
    let _process = nvs_runtime::budget::Detached::begin();
    let built_in = BUILT_IN.get_or_init(|| {
        let loader = loader()?.clone();
        let loader = match config.and_then(Modules::placed) {
            Some(modules) => loader.with_cache(Arc::new(modules)),
            None => loader,
        };
        loader.builtins().map_err(|refused| refused.reason)
    });
    let built_in = match built_in {
        Ok(built_in) => built_in,
        Err(reason) => return Some(Err(reason.clone())),
    };
    let builtin = built_in
        .iter()
        .find(|builtin| builtin.manifest().class == class)?;
    Some(
        builtin
            .extension()
            .map_err(|refused| refused.reason.clone()),
    )
}

/// Every entry of `config`'s `[[extension]]` array, loaded into one set.
///
/// `origins` and `sources` are the resolved tree's, so a refusal points at the entry's line.
///
/// # Errors
///
/// `E0652` for the first entry whose file does not load, whose class an earlier entry
/// declares, or whose `[ext.<name>]` block its manifest's settings refuse, and for an engine
/// that cannot be made. `E0601` for an `[ext.<name>]` block no loaded extension declares.
/// Nothing is installed.
pub(crate) fn loaded(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
    sources: &SourceMap,
) -> Result<Set, Diagnostic> {
    size_pool(config);
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
        let extension = match crate::bundle::extension_file(&entry.path) {
            Some(bytes) => loader.load_bytes(&entry, bytes),
            None => loader.load(&entry),
        }
        .map_err(|refused| refuse(&refused.reason))?;
        if let Some(settings) = &extension.manifest.settings
            && let Some(block) = config.ext.get(&settings.name)
        {
            let block = match serde_json::to_value(block) {
                Ok(serde_json::Value::Object(block)) => block,
                _ => return Err(refuse(&format!("`[ext.{}]` is not a table", settings.name))),
            };
            settings.check(&block).map_err(|reason| refuse(&reason))?;
        }
        set.insert(extension)
            .map_err(|refused| refuse(&refused.reason))?;
    }
    let declared: Vec<&str> = set
        .extensions()
        .iter()
        .filter_map(|extension| extension.manifest.settings.as_ref())
        .map(|settings| settings.name.as_str())
        .collect();
    if let Some(name) = config
        .ext
        .keys()
        .find(|name| !declared.contains(&name.as_str()))
    {
        return Err(nvs_config::extension::undeclared_settings(
            name, &declared, origins,
        ));
    }
    Ok(set)
}

/// Sizes the host's pool from `config`, where no host exists yet: `max_in_flight` times the
/// classes one request can call, the built-in components and each `[[extension]]` entry, and at
/// most [`MOST_SLOTS`]. The module doc's *the host has room for every request in flight*.
fn size_pool(config: &Config) {
    if HOST.get().is_some() {
        return;
    }
    let in_flight = config
        .server
        .as_ref()
        .and_then(|server| server.max_in_flight)
        .unwrap_or(nvs_config::defaults::SERVER_MAX_IN_FLIGHT);
    let classes = with_built_in(Vec::new()).len() + config.extension.len();
    let slots = in_flight
        .saturating_mul(u64::try_from(classes.max(1)).unwrap_or(u64::MAX))
        .min(u64::from(MOST_SLOTS));
    SLOTS.store(
        u32::try_from(slots).unwrap_or(MOST_SLOTS),
        Ordering::Relaxed,
    );
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
    nvs_ext::load::read_manifests_with(&entries, |entry| {
        crate::bundle::extension_file(&entry.path)
            .map_or_else(|| std::fs::read(&entry.path), |bytes| Ok(bytes.to_vec()))
    })
    .map_err(|(index, refused)| {
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

/// The loader, made on the first call. `nvs ext build` checks what it packs with this one, so a
/// file it writes passed the checks boot makes.
pub(crate) fn loader() -> Result<&'static Loader, String> {
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
    let host = Host::new(SLOTS.load(Ordering::Relaxed), |_| Ok(()))
        .map_err(|err| format!("the WebAssembly engine did not start: {err:#}"))?;
    Ok(HOST.get_or_init(|| host))
}

/// The set the configuration now serving loaded, or `None` before the first boot.
fn live() -> Option<Arc<Set>> {
    LIVE.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

/// The manifests of the live set, which `nvs serve` types a program against
/// (`rule:packaging/extension-calls-are-statically-typed`). Empty before the first boot and for
/// a configuration with no `[[extension]]`.
///
/// Each carries its extension's source files in [`Manifest::source`], as
/// [`nvs_ext::load::read_manifests`] fills them for `nvs run`, so a class under the extension's
/// namespace resolves with no `autoload` line
/// (`rule:packaging/an-extension-package-carries-two-payloads`). The loader keeps those files on
/// the [`nvs_ext::load::Extension`] and not in its manifest.
pub(crate) fn live_manifests() -> Vec<Manifest> {
    live().map_or_else(Vec::new, |set| {
        set.extensions()
            .iter()
            .map(|extension| Manifest {
                source: extension.source.files.clone(),
                ..extension.manifest.clone()
            })
            .collect()
    })
}

/// The host `nvs serve` installs on every worker: each request's calls run in a [`Calls`] kept
/// in that request's tree (`nvs_runtime::TreeState::extensions`), so the request's resources
/// are dropped when the tree is.
pub(crate) struct Served;

impl nvs_runtime::extension::Extensions for Served {
    fn call(&self, ctx: &mut Ctx, class: &str, method: &str, args: &[Value]) -> HelperResult {
        in_tree(ctx, class, method, args, live)
    }
}

/// The host `nvs test` installs on the suite's thread, on each core it starts and on the
/// `--shared-worker` thread (`crate::shared`): as [`Served`], but over the one set the suite
/// loaded. Each test or case runs under a parent context of its own, so its calls run in a
/// [`Calls`] of its own, under its own budget, and its instances end with it. A [`Calls`] that
/// outlived the tree it was made in would read that tree's spent CPU as all of it.
pub(crate) struct Tested(pub(crate) Arc<Set>);

impl nvs_runtime::extension::Extensions for Tested {
    fn call(&self, ctx: &mut Ctx, class: &str, method: &str, args: &[Value]) -> HelperResult {
        in_tree(ctx, class, method, args, || Some(Arc::clone(&self.0)))
    }
}

/// Calls `class::method` through the [`Calls`] kept in `ctx`'s request tree, made over the set
/// `set` gives at the tree's first call.
fn in_tree(
    ctx: &mut Ctx,
    class: &str,
    method: &str,
    args: &[Value],
    set: impl FnOnce() -> Option<Arc<Set>>,
) -> HelperResult {
    let host = ctx.tree_handle().extensions(|| {
        let set = set().ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Extension,
                format!("`{class}::{method}`: the extension is not loaded"),
            )
        })?;
        Ok::<_, Fault>(Arc::new(Calls::serving(set)) as Arc<dyn std::any::Any + Send + Sync>)
    })?;
    let calls = host
        .downcast_ref::<Calls>()
        .expect("only these hosts set a tree's extensions, and one set them above");
    nvs_runtime::extension::Extensions::call(calls, ctx, class, method, args)
}

/// The extensions a run loaded, and the requests their calls run in. Dropping it ends the
/// requests: every resource a guest returned is dropped in its guest. A failure of a destructor
/// is the extension's, and the request has already finished, so it is not reported.
///
/// **One request per `grants:` narrowing in the tree**
/// (`rule:security/extension-grants-are-an-intersection`). An instance's preopens are fixed
/// when it is made, from its caller's grants. A child script whose `grants:` is narrower than its
/// parent's therefore calls through a request of its own, made at its first call, and never
/// reaches an instance the parent made. Children with the same narrowing share one. So a tree
/// holds an instance of a class per distinct narrowing that called it, each taking its own place
/// in the host's pool.
pub(crate) struct Calls {
    set: Arc<Set>,
    requests: Mutex<Vec<Arc<Held>>>,
}

/// One request of a [`Calls`] and the budget its instances are charged to.
type Held = (Request, Arc<Spent>);

impl Calls {
    /// The host of calls into `set`.
    pub(crate) fn new(set: Set) -> Self {
        Self::serving(Arc::new(set))
    }

    /// The host of one request's calls into `set`, which other requests share.
    fn serving(set: Arc<Set>) -> Self {
        Self {
            set,
            requests: Mutex::new(Vec::new()),
        }
    }

    /// The request and its budget for `ctx`'s `grants:` narrowing, made at the first call from
    /// a context of `ctx`'s request tree that has that narrowing, with that context's
    /// configuration.
    fn request(&self, ctx: &Ctx) -> Result<Arc<Held>, Fault> {
        let mut requests = self.requests.lock().unwrap_or_else(PoisonError::into_inner);
        let narrowed = ctx.grant_filter();
        if let Some(request) = requests
            .iter()
            .find(|request| request.1.narrowed.as_deref() == narrowed)
        {
            return Ok(Arc::clone(request));
        }
        let host = host().map_err(|reason| Fault::thrown_as(ThrownClass::Extension, reason))?;
        let spent = Arc::new(Spent::of(ctx));
        let request = Arc::new((host.request(spent.clone()), spent));
        requests.push(Arc::clone(&request));
        Ok(request)
    }
}

impl Drop for Calls {
    /// A request a running call still holds ends when that call drops it, without
    /// [`Request::end`]; no call runs once the tree that keeps this has ended.
    fn drop(&mut self) {
        let requests = std::mem::take(
            self.requests
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        );
        for request in requests {
            if let Ok((request, _)) = Arc::try_unwrap(request) {
                drop(drive(request.end(), || false));
            }
        }
    }
}

/// A request's budget for its guests: the request's own CPU deadline and memory count, and its
/// own log, clock, generator and configuration.
///
/// The CPU deadline is the request tree's: a guest's time is spent when the tree's safepoint word
/// carries `CPU_LIMIT`, which the watchdog raises against everything the request has run since it
/// started, the guest's turns on its core included. A guest's linear memory is charged to the
/// memory count the runtime keeps for the running request (`nvs_runtime::budget`), so it meets
/// the one `[limits] memory` ceiling the program's own values meet. Memory a tree gives back on
/// another core leaves that core's count, as every other block the tree frees there does.
///
/// A guest runs while the request's context is borrowed by the call, so this keeps what it needs
/// of the context beside it. [`Spent::enter`] copies the context's fixed clock and seed in
/// before each call, and [`Spent::leave`] writes the advanced seed back and the guest's log lines
/// out after it. A test's `#[Test(at: …)]` and `#[Test(seed: …)]` therefore reach a guest, and one
/// seed fixes the draws `Core\Random` and a guest make together. Lines a guest writes while its
/// resources are dropped at the request's end have no context left to write to, and are dropped.
///
/// A guest's outbound HTTP request waits in [`Spent`] until the call that runs the guest polls
/// it, and [`Spent::sends`] sends it from there with the calling task's context, through
/// `nvs_stdlib::extension_send`. The task parks on the socket as a `Core\Http\Client` call does,
/// so the other tasks on its core run while the guest waits.
///
/// A guest's read of a lent source waits in [`Spent`] the same way, and [`Spent::reads`] serves
/// it from the call's own arguments (`rule:core-classes/spreadsheet-reads-as-a-stream`): a range
/// of a `bytes` argument is copied from the runtime's string, and a range of an open
/// `Core\IO\File` is read through `nvs_stdlib::extension_file_range`. So a source is never copied
/// whole as it crosses, and only the ranges the guest reads leave the program's memory.
///
/// What it spends: one copy of the request's configuration, which is an `Arc` of its snapshot and
/// the few keys the request set for itself, the `grants:` list of the isolate that
/// made the request, each logged line until the call that wrote it returns, each outbound
/// request until it is sent, and each read until it is served.
pub(crate) struct Spent {
    /// Weak, because the tree keeps this budget: a strong view would be a cycle, and the tree
    /// and its instance would never be dropped. A tree that has ended reads as spent.
    tree: WeakSafepointView,
    started: Instant,
    config: Option<nvs_config::Request>,
    narrowed: Option<Vec<nvs_config::capability::Cap>>,
    clock: Mutex<Option<i128>>,
    seed: Mutex<Option<u64>>,
    lines: Mutex<Vec<(Level, String, String)>>,
    outbound: Mutex<Vec<(Outgoing, Answer)>>,
    reads: Mutex<Vec<(u64, u64, u64, Lender)>>,
}

impl Spent {
    /// The budget of the request `ctx` runs: its tree, its snapshot and its grants.
    fn of(ctx: &Ctx) -> Self {
        Self {
            tree: ctx.safepoint_view().downgrade(),
            started: Instant::now(),
            config: ctx.config().cloned(),
            narrowed: ctx.grant_filter().map(<[_]>::to_vec),
            clock: Mutex::new(None),
            seed: Mutex::new(None),
            lines: Mutex::new(Vec::new()),
            outbound: Mutex::new(Vec::new()),
            reads: Mutex::new(Vec::new()),
        }
    }

    /// Copies `ctx`'s fixed clock and seed in, before a call.
    fn enter(&self, ctx: &Ctx) {
        *self.clock.lock().unwrap_or_else(PoisonError::into_inner) = ctx.fixed_clock();
        *self.seed.lock().unwrap_or_else(PoisonError::into_inner) = ctx.random_state();
    }

    /// Writes the advanced seed back to `ctx`, and the lines the guest logged to its log, after a
    /// call.
    ///
    /// A configured log target that fails to write is not reported: the line is the guest's, and
    /// the program did not ask for it.
    fn leave(&self, ctx: &mut Ctx) {
        if let Some(state) = *self.seed.lock().unwrap_or_else(PoisonError::into_inner) {
            ctx.set_random_state(state);
        }
        let lines = std::mem::take(&mut *self.lines.lock().unwrap_or_else(PoisonError::into_inner));
        for (level, channel, message) in lines {
            let level = match level {
                Level::Debug => nvs_render::Level::Debug,
                Level::Info => nvs_render::Level::Info,
                Level::Warn => nvs_render::Level::Warn,
                Level::Error => nvs_render::Level::Error,
                Level::Critical => nvs_render::Level::Critical,
            };
            drop(nvs_stdlib::extension_line(ctx, level, &channel, &message));
        }
    }

    /// Sends every outbound request a guest has made since the last poll, with `ctx`, and gives
    /// each its response. Returns whether there was one, so the call is polled again at once.
    fn sends(&self, ctx: &mut Ctx) -> bool {
        let queued =
            std::mem::take(&mut *self.outbound.lock().unwrap_or_else(PoisonError::into_inner));
        let any = !queued.is_empty();
        for (request, answer) in queued {
            answer.fill(sent(ctx, request));
        }
        any
    }

    /// Serves every read a guest has made of a source in `lent` since the last poll, with `ctx`.
    /// Returns whether there was one, so the call is polled again at once.
    fn reads(&self, ctx: &mut Ctx, lent: &[Value]) -> bool {
        let queued =
            std::mem::take(&mut *self.reads.lock().unwrap_or_else(PoisonError::into_inner));
        let any = !queued.is_empty();
        for (id, offset, length, lender) in queued {
            let source = usize::try_from(id)
                .ok()
                .and_then(|id| lent.get(id).copied());
            lender.fill(match source {
                None => Err(format!("no source was lent to this call as {id}")),
                Some(source) => match source.as_bytes() {
                    Some(bytes) => {
                        let start =
                            usize::try_from(offset).map_or(bytes.len(), |at| at.min(bytes.len()));
                        let take = usize::try_from(length).unwrap_or(usize::MAX);
                        Ok(bytes[start..start.saturating_add(take).min(bytes.len())].to_vec())
                    }
                    None => nvs_stdlib::extension_file_range(ctx, source, offset, length)
                        .map_err(fault_text),
                },
            });
        }
        any
    }
}

/// `request` sent through `Core\Http\Client`'s transport, under `ctx`'s grants, following a
/// redirect only to a host the extension may connect to.
fn sent(ctx: &mut Ctx, request: Outgoing) -> Result<Incoming, Unsent> {
    let Outgoing {
        method,
        url,
        headers,
        body,
        connect,
    } = request;
    match nvs_stdlib::extension_send(ctx, &method, &url, headers, body, &connect) {
        Ok(reply) => Ok(Incoming {
            status: u16::try_from(reply.status).unwrap_or(u16::MAX),
            headers: reply.headers,
            body: reply.body,
        }),
        Err(
            Fault::Thrown(ThrownClass::Timeout, message)
            | Fault::ThrownWithSlots(ThrownClass::Timeout, message, _),
        ) => Err(Unsent::Timeout(message.into_owned())),
        Err(
            Fault::Thrown(ThrownClass::Io, message)
            | Fault::ThrownWithSlots(ThrownClass::Io, message, _),
        ) => Err(Unsent::Connection(message.into_owned())),
        Err(
            Fault::Thrown(_, message)
            | Fault::ThrownWithSlots(_, message, _)
            | Fault::Fatal(message),
        ) => Err(Unsent::Refused(message.into_owned())),
        Err(_) => Err(Unsent::Refused(format!("`{method} {url}` was not sent"))),
    }
}

impl Budget for Spent {
    fn cpu_spent(&self) -> bool {
        self.tree
            .flags()
            .is_none_or(|flags| flags.contains(SafepointFlags::CPU_LIMIT))
    }

    /// A growth the request cannot afford charges nothing and records no refusal of its own, so
    /// the `FATAL` is the call's.
    fn charge(&self, bytes: i64) -> bool {
        let delta =
            isize::try_from(bytes).unwrap_or(if bytes > 0 { isize::MAX } else { isize::MIN });
        if delta > 0 && self.left().is_some_and(|left| bytes.unsigned_abs() > left) {
            return false;
        }
        nvs_runtime::budget::carry(delta);
        true
    }

    fn left(&self) -> Option<u64> {
        nvs_runtime::budget::headroom().map(|bytes| u64::try_from(bytes).unwrap_or(u64::MAX))
    }

    fn log(&self, level: Level, channel: &str, message: &str) {
        self.lines
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((level, channel.to_owned(), message.to_owned()));
    }

    fn wall_clock(&self) -> Duration {
        match *self.clock.lock().unwrap_or_else(PoisonError::into_inner) {
            Some(nanos) => u64::try_from(nanos).map_or(Duration::ZERO, Duration::from_nanos),
            None => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default(),
        }
    }

    /// The time since the budget was made: a monotonic reading measures an interval that really
    /// elapsed, so a fixed clock does not fix it, as it does not fix `Core\Time::monotonic`.
    fn monotonic_clock(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    fn random(&self, out: &mut [u8]) {
        match &mut *self.seed.lock().unwrap_or_else(PoisonError::into_inner) {
            Some(state) => nvs_stdlib::random::fill_seeded(state, out),
            None => rand::Rng::fill_bytes(&mut rand::rng(), out),
        }
    }

    /// Queued for [`Spent::sends`], which the call that runs the guest reaches at its next poll.
    fn send(&self, request: Outgoing) -> Outbound {
        let (outbound, answer) = Outbound::pair();
        self.outbound
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((request, answer));
        outbound
    }

    /// Queues the read for [`Spent::reads`], which the call that runs the guest serves.
    fn read(&self, id: u64, offset: u64, length: u64) -> Lending {
        let (lending, lender) = Lending::pair();
        self.reads
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((id, offset, length, lender));
        lending
    }

    /// Opened beneath a handle, as a `Core\IO` write door opens one
    /// (`rule:security/writes-open-beneath-a-handle`).
    fn open_file(&self, path: &Path, how: FileOpen) -> io::Result<File> {
        let how = nvs_runtime::guest_files::Open {
            read: how.read,
            write: how.write,
            append: false,
            create: how.create,
            exclusive: how.exclusive,
            truncate: how.truncate,
        };
        nvs_runtime::guest_files::open_with(path, how)
    }

    /// Created beneath a handle, as [`Budget::open_file`] opens a file.
    fn create_dir(&self, path: &Path) -> io::Result<()> {
        nvs_runtime::guest_files::create_dir(path)
    }

    /// Renamed between two folder handles, as [`Budget::open_file`] opens a file.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        nvs_runtime::guest_files::rename(from, to)
    }

    /// Removed from its folder's handle, as [`Budget::open_file`] opens a file.
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        nvs_runtime::guest_files::remove_file(path)
    }

    /// Removed from its folder's handle, as [`Budget::open_file`] opens a file.
    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        nvs_runtime::guest_files::remove_dir(path)
    }

    /// Read from a handle opened beneath its folder, as [`Budget::open_file`] opens a file.
    fn metadata(&self, path: &Path, follow: bool) -> io::Result<std::fs::Metadata> {
        if follow {
            nvs_runtime::guest_files::metadata(path)
        } else {
            nvs_runtime::guest_files::entry_metadata(path)
        }
    }

    /// Listed from the walked folder's handle, as [`Budget::open_file`] opens a file.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<nvs_ext::call::Listed>> {
        use nvs_ext::call::ListedKind;
        use nvs_runtime::guest_files::Kind;
        Ok(nvs_runtime::guest_files::list(path)?
            .into_iter()
            .map(|entry| nvs_ext::call::Listed {
                name: entry.name,
                kind: match entry.kind {
                    Kind::Dir => ListedKind::Dir,
                    Kind::File => ListedKind::File,
                    Kind::Link => ListedKind::Link,
                    Kind::Other => ListedKind::Other,
                },
            })
            .collect())
    }

    /// The value at `ext.<block>.<key>` in the request's snapshot, as JSON.
    fn setting(&self, block: &str, key: &str) -> Option<serde_json::Value> {
        let value = self
            .config
            .as_ref()?
            .snapshot()
            .table
            .get("ext")?
            .get(block)?
            .get(key)?;
        serde_json::to_value(value).ok()
    }

    /// The `[capabilities]` of the request's snapshot, narrowed by the `grants:` list of the
    /// isolate whose context made the request.
    fn caller(&self) -> Caller<'_> {
        Caller {
            authority: self.config.as_ref().map(nvs_config::Request::authority),
            narrowed: self.narrowed.as_deref(),
        }
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
        let extension = match self
            .set
            .extensions()
            .iter()
            .find(|extension| extension.manifest.class == class)
        {
            Some(extension) => extension,
            None => built_in(class, ctx.config().map(|config| &config.snapshot().config))
                .ok_or_else(|| refused("the extension is not loaded".to_owned()))?
                .map_err(refused)?,
        };
        let manifest = &extension.manifest;
        let declared = manifest
            .methods
            .iter()
            .find(|declared| declared.name == method);
        let mut lent = Vec::new();
        let mut crossing = Vec::with_capacity(args.len());
        for (at, arg) in args.iter().enumerate() {
            // A `mixed` argument, and a third-party export's `bytes` argument, are lent by
            // number, so the guest reads the program's own copy.
            let lends = declared
                .and_then(|declared| declared.params.get(at))
                .is_some_and(|param| match manifest.novis_type(&param.ty) {
                    Ok(NovisType::Mixed) => true,
                    Ok(NovisType::Bytes) => manifest.builtin.is_none(),
                    _ => false,
                });
            let lending = if lends {
                lent_source(ctx, *arg, &mut lent)
            } else {
                None
            };
            crossing.push(lending.unwrap_or_else(|| crossed(*arg)).map_err(refused)?);
        }
        let mut args = crossing;
        let in_force = ctx
            .config()
            .map_or(nvs_config::image::DEFAULT, nvs_config::image::in_force);
        nvs_ext::builtin::cap_pixels(class, method, &mut args, in_force);
        let (bytes, ratio) = nvs_stdlib::decompression_ceiling(ctx);
        let bound = nvs_ext::builtin::Decompression { bytes, ratio };
        nvs_ext::builtin::cap_decompression(class, method, &mut args, bound);
        let held = self.request(ctx)?;
        let (request, spent) = &*held;
        spent.enter(ctx);
        let called = drive(request.call_values(extension, method, args), || {
            let sent = spent.sends(ctx);
            spent.reads(ctx, &lent) || sent
        });
        spent.leave(ctx);
        match called {
            Ok(None) => Ok(Value::null()),
            Ok(Some(result)) => {
                let manifest = &extension.manifest;
                let ty = manifest
                    .methods
                    .iter()
                    .find(|declared| declared.name == method)
                    .map_or(Ok(NovisType::Mixed), |declared| {
                        manifest.novis_type(&declared.returns)
                    })
                    .map_err(refused)?;
                runtime(ctx, manifest, &ty, result).map_err(refused)
            }
            Err(failure) => Err(match failure.outcome() {
                Outcome::Throw(class, message) => {
                    let thrown = ThrownClass::ALL
                        .iter()
                        .copied()
                        .find(|thrown| thrown.name() == class.name())
                        .unwrap_or(ThrownClass::Extension);
                    Fault::thrown_as(thrown, message)
                }
                Outcome::Fatal(limit, message) => past(ctx, limit, message),
            }),
        }
    }
}

/// The request's end for a guest past `limit`: runs the request's limit handler, then is the
/// resource-limit `FATAL` with `message` (`rule:errors/on-limit`).
fn past(ctx: &mut Ctx, limit: Limit, message: String) -> Fault {
    ctx.run_limit_handler(match limit {
        Limit::Cpu => nvs_runtime::Limit::CpuTime,
        Limit::Memory => nvs_runtime::Limit::Memory,
    });
    Fault::fatal(message)
}

/// Runs `call` to the end on the calling task, with `nvs_host::block_on::block_on_yielding`.
/// Between two polls it runs `between`, and polls again at once when that did some work. At each
/// epoch tick the guest wakes its own call, and the task gives the core up for one turn, behind
/// the core's other ready tasks, with no deadline armed. A pending import parks the task until the
/// import's wake, which reaches the core from any thread.
fn drive<T>(call: impl Future<Output = T>, between: impl FnMut() -> bool) -> T {
    nvs_host::block_on::block_on_yielding(call, between)
}

/// `value` lent to a guest by its number in `lent`, when it is a source the guest reads a range at
/// a time: a `bytes` value or an open `Core\IO\File` (`rule:core-classes/spreadsheet-reads-as-a-stream`).
/// `None` for any other value, which crosses as [`crossed`] gives it.
fn lent_source(
    ctx: &mut Ctx,
    value: Value,
    lent: &mut Vec<Value>,
) -> Option<Result<Crossed, String>> {
    let size = match value.as_bytes() {
        Some(bytes) => Ok(u64::try_from(bytes.len()).unwrap_or(u64::MAX)),
        None => nvs_stdlib::extension_file_size(ctx, value)?.map_err(fault_text),
    };
    Some(size.map(|size| {
        let id = u64::try_from(lent.len()).unwrap_or(u64::MAX);
        lent.push(value);
        Crossed::Lent { id, size }
    }))
}

/// The message of `fault`, for a guest that is told why a read of its source failed.
fn fault_text(fault: Fault) -> String {
    match fault {
        Fault::Thrown(_, message) => message.into_owned(),
        other => format!("{other:?}"),
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
        Some(Tag::Object) if let Some((class, id)) = nvs_stdlib::ext_record::resource_of(value) => {
            Crossed::Resource {
                name: class.rsplit('\\').next().unwrap_or_default().to_owned(),
                id,
            }
        }
        Some(Tag::Object) => match (value.shape_fields(), nvs_stdlib::ext_record::parts(value)) {
            (Some(fields), _) => Crossed::Array(
                fields
                    .into_iter()
                    .map(|(name, field)| Ok((Key::String(name), crossed(field)?)))
                    .collect::<Result<_, String>>()?,
            ),
            (None, Some(read)) => {
                let (class, parts) = read?;
                let fields = core_record(class)?
                    .fields
                    .iter()
                    .zip(parts)
                    .map(|((name, _), part)| ((*name).to_owned(), crossed_part(part)))
                    .collect();
                Crossed::Core {
                    class: class.to_owned(),
                    fields,
                }
            }
            (None, None) => Crossed::Object(value.class_name().unwrap_or_default()),
        },
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

/// `value`, returned by a guest where `manifest` declares `ty`, as a runtime value the caller
/// owns. A shape is an instance of the class the compiled unit declared for it, which `ctx` finds
/// by its label; every other array is an array. A resource is an instance of its class in the
/// namespace of `manifest`'s class (`nvs_stdlib::ext_record::resource`).
fn runtime(
    ctx: &Ctx,
    manifest: &Manifest,
    ty: &NovisType,
    value: Crossed,
) -> Result<Value, String> {
    Ok(match value {
        Crossed::Null => Value::null(),
        Crossed::Bool(flag) => Value::bool(flag),
        Crossed::Int(number) => Value::int(number),
        Crossed::Uint(number) => Value::uint(number),
        Crossed::Float(number) => Value::float(number),
        Crossed::String(text) => Value::text(&text),
        Crossed::Bytes(octets) => Value::bytes(NvsStr::new(&octets)),
        Crossed::Array(entries) => match ty {
            NovisType::Optional(inner) => {
                return runtime(ctx, manifest, inner, Crossed::Array(entries));
            }
            NovisType::Shape(fields) => return shape(ctx, manifest, fields, entries),
            NovisType::Union { name, cases } => {
                let (_, fields) = cases
                    .iter()
                    .find(|(_, fields)| fits(fields, &entries))
                    .ok_or_else(|| format!("the guest returned no case of `{name}`"))?;
                return shape(ctx, manifest, fields, entries);
            }
            _ => {
                let element = match ty {
                    NovisType::List(element) | NovisType::Keyed(_, element) => element,
                    _ => &NovisType::Mixed,
                };
                let mut array = NvsArray::new();
                for (key, entry) in entries {
                    let entry = runtime(ctx, manifest, element, entry)?;
                    match key {
                        Key::Int(index) => array.set_index(index, entry),
                        Key::String(key) => array.set(NvsStr::new(key.as_bytes()), entry),
                    }
                }
                Value::array(array)
            }
        },
        // A manifest's enum is `int`-backed (`nvs_types::ext_lib`), so a case is written under
        // `Tag::EnumInt`: in a list, a map or a shape the tag is all a reader has
        // (`rule:enums/representation`).
        Crossed::Case(name) => Value::enum_int(nvs_ext::convert::case_number(ty, &name)?),
        Crossed::Core { class, mut fields } => {
            let record = core_record(&class)?;
            let parts = record
                .fields
                .iter()
                .map(|(name, _)| {
                    let at = fields
                        .iter()
                        .position(|(field, _)| field == name)
                        .ok_or_else(|| format!("a `{class}` with no `{name}`"))?;
                    part(fields.swap_remove(at).1)
                })
                .collect::<Result<_, _>>()?;
            return nvs_stdlib::ext_record::built(&class, parts);
        }
        Crossed::Object(class) => return Err(format!("a `{class}` object does not cross back")),
        Crossed::Lent { .. } => return Err("a lent source does not cross back".to_owned()),
        Crossed::Resource { name, id } => nvs_stdlib::ext_record::resource(
            &nvs_types::ext_lib::declared_name(manifest, &name).to_string(),
            id,
        ),
    })
}

/// The record the `Core` value class `class` crosses as.
fn core_record(class: &str) -> Result<&'static CoreRecord, String> {
    CORE_CLASSES
        .iter()
        .find(|record| record.class == class)
        .ok_or_else(|| format!("a `{class}` does not cross"))
}

/// One field of a `Core` value class's record, as it crosses into a guest.
fn crossed_part(part: Part) -> Crossed {
    match part {
        Part::Bool(flag) => Crossed::Bool(flag),
        Part::Int(number) => Crossed::Int(number),
        Part::Uint(number) => Crossed::Uint(number),
        Part::String(text) => Crossed::String(text),
        Part::Bytes(octets) => Crossed::Bytes(octets),
    }
}

/// One field of a `Core` value class's record, as a guest returned it.
fn part(value: Crossed) -> Result<Part, String> {
    Ok(match value {
        Crossed::Bool(flag) => Part::Bool(flag),
        Crossed::Int(number) => Part::Int(number),
        Crossed::Uint(number) => Part::Uint(number),
        Crossed::String(text) => Part::String(text),
        Crossed::Bytes(octets) => Part::Bytes(octets),
        other => return Err(format!("a record field of no record type: {other:?}")),
    })
}

/// The anonymous object of the shape `fields` whose fields a guest returned as `entries`.
fn shape(
    ctx: &Ctx,
    manifest: &Manifest,
    fields: &[Field],
    entries: Vec<(Key, Crossed)>,
) -> Result<Value, String> {
    let mut names: Vec<String> = fields.iter().map(|(name, ..)| name.clone()).collect();
    names.sort_unstable();
    let label = nvs_types::derive::shape_class_label(&names);
    let mut written = Vec::with_capacity(entries.len());
    for (key, entry) in entries {
        let declared = match &key {
            Key::String(name) => fields.iter().find(|(field, ..)| field == name),
            Key::Int(_) => None,
        };
        let converted = match declared {
            Some((_, _, ty)) => runtime(ctx, manifest, ty, entry),
            None => Err(format!("the shape `{label}` has no field {key:?}")),
        };
        match (key, converted) {
            (Key::String(name), Ok(value)) => written.push((name, value)),
            (_, Err(reason)) => {
                // A label that names no class releases what was already converted.
                let _ = ctx.new_shape("", written);
                return Err(reason);
            }
            (Key::Int(_), Ok(_)) => unreachable!("an `int` key names no field"),
        }
    }
    ctx.new_shape(&label, written)
        .ok_or_else(|| format!("the program declares no class for the shape `{label}`"))
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
        cache.rekey(env_hash(
            &Config::default(),
            &nvs_config::grants::Grants::default(),
        ));
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
pub(crate) fn entry(
    index: usize,
    written: &Extension,
    origins: &BTreeMap<String, Origin>,
) -> Entry {
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
        grants: nvs_config::extension::grants(index, &written.grants, origins),
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
        let cache = Cache::new(
            dir.path(),
            env_hash(&Config::default(), &nvs_config::grants::Grants::default()),
        )
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

    /// A request's budget takes its context's fixed clock, seed, log and snapshot: a guest's
    /// draws continue the seeded sequence and advance it in the context, its wall clock is the
    /// fixed one, its log line is a record of the request with the extension as its channel, and
    /// a setting is read from the snapshot's `[ext.<name>]` block.
    #[test]
    fn a_request_budget_is_its_contexts_clock_seed_log_and_settings() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(Arc::new(nvs_config::Snapshot {
            table: toml::from_str("[ext.geo]\nzoom = 3\n").expect("a table"),
            ..nvs_config::Snapshot::default()
        }));
        ctx.set_fixed_clock(1_500_000_000);
        ctx.set_random_state(7);
        let spent = Spent::of(&ctx);

        spent.enter(&ctx);
        let mut drawn = [0; 12];
        spent.random(&mut drawn);
        spent.log(Level::Info, "geo", "tile loaded");
        assert_eq!(spent.wall_clock(), Duration::from_millis(1500));
        assert_eq!(spent.setting("geo", "zoom"), Some(serde_json::json!(3)));
        assert_eq!(spent.setting("geo", "missing"), None);
        assert_eq!(spent.setting("shop", "zoom"), None);
        spent.leave(&mut ctx);

        let mut state = 7;
        let mut expected = [0; 12];
        nvs_stdlib::random::fill_seeded(&mut state, &mut expected);
        assert_eq!(drawn, expected);
        assert_eq!(ctx.random_state(), Some(state));
        let output = String::from_utf8(ctx.take_buffered_output().expect("a buffer"))
            .expect("a log line is text");
        assert!(output.contains("tile loaded"), "{output}");
        assert!(output.contains("geo"), "{output}");
    }

    /// An enum case a guest returns is written under `Tag::EnumInt` as a scalar, a list element
    /// and a keyed-map value, so a reader that checks the tag finds a case, and the first case is
    /// `0` (`rule:enums/representation`). A shape field is converted by the same arm.
    #[test]
    fn a_returned_enum_case_carries_the_enum_tag_in_every_position() {
        let ctx = Ctx::buffered();
        let manifest = Manifest::parse(
            br#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Plural", "interface": "shop:plural/api"}"#,
        )
        .expect("a manifest");
        let category = NovisType::Enum {
            name: "Category".to_owned(),
            cases: vec!["Zero".to_owned(), "One".to_owned()],
        };
        let case = |name: &str| Crossed::Case(name.to_owned());

        let scalar = runtime(&ctx, &manifest, &category, case("Zero")).expect("a case");
        assert_eq!(scalar.exact_tag(), Some(Tag::EnumInt));
        assert_eq!(scalar.as_int(), Some(0));

        let list = NovisType::List(Box::new(category.clone()));
        let entries = vec![(Key::Int(0), case("Zero")), (Key::Int(1), case("One"))];
        let list = runtime(&ctx, &manifest, &list, Crossed::Array(entries)).expect("a list");
        let list = list.as_array().expect("an array");
        let element = list.get_index(1).expect("an element");
        assert_eq!(element.exact_tag(), Some(Tag::EnumInt));
        assert_eq!(element.as_int(), Some(1));

        let keyed = NovisType::Keyed(Box::new(NovisType::String), Box::new(category));
        let entries = vec![(Key::String("first".to_owned()), case("Zero"))];
        let keyed = runtime(&ctx, &manifest, &keyed, Crossed::Array(entries)).expect("a map");
        let keyed = keyed.as_array().expect("an array");
        let value = keyed.get(b"first").expect("a value");
        assert_eq!(value.exact_tag(), Some(Tag::EnumInt));
        assert_eq!(value.as_int(), Some(0));
    }

    /// A guest's outbound request waits until the call polls it, and is then sent under the
    /// calling context's grants: a context with no `net.connect` grant gets no response.
    #[test]
    fn a_guest_request_is_sent_at_the_next_poll_under_the_contexts_grants() {
        let mut ctx = Ctx::buffered();
        let spent = Spent::of(&ctx);
        let outbound = spent.send(Outgoing {
            method: "GET".to_owned(),
            url: "http://example.com/".to_owned(),
            headers: Vec::new(),
            body: None,
            connect: vec!["example.com".to_owned()],
        });

        assert!(spent.sends(&mut ctx));
        assert!(!spent.sends(&mut ctx));
        let answer = drive(outbound, || false);
        assert!(
            matches!(&answer, Err(Unsent::Refused(message)) if message.contains("example.com")),
            "{answer:?}"
        );
    }

    /// A guest's CPU time is the request's, counted from the request's start: a request whose
    /// time was spent before its first call into an extension gives its guest none. The flag is
    /// the one the watchdog raises when the request passes its `[limits] cpu_time`.
    #[test]
    fn a_guest_spends_the_requests_own_cpu_deadline() {
        let mut ctx = Ctx::buffered();
        ctx.set_cpu_limit(1_000_000);
        let fresh = Spent::of(&ctx);
        assert!(!fresh.cpu_spent());

        ctx.safepoint_view().request(SafepointFlags::CPU_LIMIT);
        assert!(
            fresh.cpu_spent(),
            "a budget made earlier reads the same deadline"
        );
        let late = Spent::of(&ctx);
        assert!(
            late.cpu_spent(),
            "a first call after the deadline has no time left"
        );
    }

    /// A guest's growth is charged to the memory count the runtime keeps for the request, and
    /// given back to it. A growth past the request's `[limits] memory` charges nothing.
    #[test]
    fn a_guest_growth_is_charged_to_the_requests_memory_count() {
        const PAGE: i64 = 65536;
        let mut ctx = Ctx::buffered();
        ctx.set_memory_limit(64 << 20);
        let spent = Spent::of(&ctx);

        let before = ctx.memory_used();
        let left = spent
            .left()
            .expect("a request with a memory limit has a remainder");
        assert!(spent.charge(PAGE));
        assert_eq!(ctx.memory_used(), before + 65536);
        assert_eq!(spent.left(), Some(left - 65536));
        assert!(spent.charge(-PAGE));
        assert_eq!(ctx.memory_used(), before);

        let past = i64::try_from(left).expect("a remainder under 64 MiB") + PAGE;
        assert!(!spent.charge(past));
        assert_eq!(
            ctx.memory_used(),
            before,
            "a refused growth charges nothing"
        );
    }

    /// A guest past either of the request's limits runs the request's limit handler and ends the
    /// request with a resource-limit `FATAL`, which no `catch` sees.
    #[test]
    fn a_guest_past_the_requests_limit_ends_it_with_a_resource_limit_fatal() {
        for (limit, reached) in [
            (Limit::Cpu, nvs_runtime::Limit::CpuTime),
            (Limit::Memory, nvs_runtime::Limit::Memory),
        ] {
            let mut ctx = Ctx::buffered();
            let fault = past(&mut ctx, limit, "the extension ran out".to_owned());
            assert!(
                matches!(&fault, Fault::Fatal(message) if message == "the extension ran out"),
                "{fault:?}"
            );
            assert_eq!(ctx.limit_reached(), Some(reached));
        }
    }
}
