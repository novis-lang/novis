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
//! has room for [`SLOTS`] instances at once and links only the world's own imports. A guest holds
//! the files and hosts its effective grant names, and no other authority.
//!
//! **A call** (`rule:packaging/extension-calls-are-statically-typed`). [`Calls`] implements
//! `nvs_runtime::extension::Extensions` over one `nvs_ext::call::Request`, made at the run's first
//! call. It copies each argument into an `nvs_ext::convert::Value` with no type in hand, and
//! `Request::call_values` converts it by the manifest. The call is a future polled on the calling
//! task: between two polls the task parks for up to one epoch tick, so the other tasks on the core
//! run (`rule:packaging/a-guest-call-yields-on-its-core`). A failure throws the class
//! `Failure::outcome` names (`rule:packaging/a-guest-crash-throws`). A limit runs the request's
//! limit handler, then is a `FATAL` (`rule:errors/on-limit`).
//! Dropping a [`Calls`] runs `Request::end`, which `nvs run` does once the run's tasks are
//! finished.
//!
//! **In `nvs serve` a request's calls live in its request tree.** Every worker installs
//! [`Served`], which at a request's first call puts a [`Calls`] over the live set into
//! `nvs_runtime::TreeState::extensions`. Every task of the request reaches the same one, and
//! it is dropped with the tree: when the request and its after-response work have finished,
//! even where a connection gave up on the request first. A request that started before a
//! reload and makes its first call after it calls into the new set. Its unit was typed
//! against the old one, so a class the reload removed throws `ExtensionError`. The tree is
//! normally dropped on the request's core. A tree whose last task finished on another core is
//! dropped there, and its resources' destructors run on that core.
//!
//! The request's budget is a [`Spent`] made at the first call from the request's `[limits]`: its
//! CPU time from then on, and its memory as a second allowance beside the one the runtime counts.
//! A guest's log lines are the request's log records, its wall clock and random bytes are the
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
//! (`rule:core-classes/image-pixel-cap`).
//!
//! What it spends: each loaded component's compiled code, once per process and shared by every
//! core, and during a reload the old set and the new one together until the swap. On disk, one
//! entry per component file per build, under the cache's own size cap.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

use nvs_config::cache::{Digest, artifact_key, content_hash, env_hash};
use nvs_config::resolve::Origin;
use nvs_config::tree::{Config, Extension};
use nvs_config::value::{Quantity, Unit};
use nvs_diagnostics::{Diagnostic, SourceMap};
use nvs_ext::call::{
    Answer, Budget, Host, Incoming, Level, Limit, Meter, Outbound, Outcome, Outgoing, Request,
    Unsent,
};
use nvs_ext::convert::{Key, Value as Crossed, fits};
use nvs_ext::grants::Caller;
use nvs_ext::load::{Builtin, CacheKey, Entry, Extension as Loaded, Loader, ModuleCache, Set};
use nvs_ext::manifest::Manifest;
use nvs_ext::types::{CORE_CLASSES, CoreRecord, Field, NovisType};
use nvs_runtime::{Ctx, Fault, HelperResult, NvsArray, NvsStr, SlotKey, Tag, ThrownClass, Value};
use nvs_stdlib::ext_record::Part;

use crate::cache::Cache;

/// How many instances the process's host has room for at once, across every request. Each slot
/// reserves address space for one instance, not committed memory.
const SLOTS: u32 = 64;

/// The CPU time a request with no `[limits] cpu_time` gives its guests.
const UNCAPPED: Duration = Duration::from_secs(365 * 24 * 60 * 60);

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
fn built_in(class: &str, config: Option<&Config>) -> Option<Result<&'static Loaded, String>> {
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
    let host = Host::new(SLOTS, |_| Ok(()))
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
pub(crate) fn live_manifests() -> Vec<Manifest> {
    live().map_or_else(Vec::new, |set| {
        set.extensions()
            .iter()
            .map(|extension| extension.manifest.clone())
            .collect()
    })
}

/// The host `nvs serve` installs on every worker: each request's calls run in a [`Calls`] kept
/// in that request's tree (`nvs_runtime::TreeState::extensions`), so the request's resources
/// are dropped when the tree is.
pub(crate) struct Served;

impl nvs_runtime::extension::Extensions for Served {
    fn call(&self, ctx: &mut Ctx, class: &str, method: &str, args: &[Value]) -> HelperResult {
        let tree = ctx.tree_handle();
        let slot = tree.extensions();
        if slot.get().is_none() {
            let set = live().ok_or_else(|| {
                Fault::thrown_as(
                    ThrownClass::Extension,
                    format!("`{class}::{method}`: the extension is not loaded"),
                )
            })?;
            drop(slot.set(Box::new(Calls::serving(set))));
        }
        let calls = slot
            .get()
            .and_then(|calls| calls.downcast_ref::<Calls>())
            .expect("only this host sets a tree's extensions, and it set them above");
        nvs_runtime::extension::Extensions::call(calls, ctx, class, method, args)
    }
}

/// The extensions a run loaded, and the request their calls run in. Dropping it ends the
/// request: every resource a guest returned is dropped in its guest. A failure of a destructor
/// is the extension's, and the request has already finished, so it is not reported.
pub(crate) struct Calls {
    set: Arc<Set>,
    request: OnceLock<(Request, Arc<Spent>)>,
}

impl Calls {
    /// The host of calls into `set`.
    pub(crate) fn new(set: Set) -> Self {
        Self::serving(Arc::new(set))
    }

    /// The host of one request's calls into `set`, which other requests share.
    fn serving(set: Arc<Set>) -> Self {
        Self {
            set,
            request: OnceLock::new(),
        }
    }

    /// The request and its budget, made at the first call from `ctx`'s limits and configuration.
    fn request(&self, ctx: &Ctx) -> Result<&(Request, Arc<Spent>), Fault> {
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
        Ok(self.request.get_or_init(|| {
            let spent = Arc::new(Spent {
                meter: Meter::new(cpu, memory),
                snapshot: ctx.config().map(|config| Arc::clone(config.snapshot())),
                narrowed: ctx.grant_filter().map(<[_]>::to_vec),
                clock: Mutex::new(None),
                seed: Mutex::new(None),
                lines: Mutex::new(Vec::new()),
                outbound: Mutex::new(Vec::new()),
            });
            (host.request(spent.clone()), spent)
        }))
    }
}

impl Drop for Calls {
    fn drop(&mut self) {
        if let Some((request, _)) = self.request.take() {
            drop(drive(request.end(), || false));
        }
    }
}

/// A request's budget for its guests: [`Meter`]'s CPU deadline and memory count, and the
/// request's own log, clock, generator and configuration.
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
/// What it spends: one `Arc` of the request's snapshot, the `grants:` list of the isolate that
/// made the request, each logged line until the call that wrote it returns, and each outbound
/// request until it is sent.
pub(crate) struct Spent {
    meter: Meter,
    snapshot: Option<Arc<nvs_config::Snapshot>>,
    narrowed: Option<Vec<nvs_config::capability::Cap>>,
    clock: Mutex<Option<i128>>,
    seed: Mutex<Option<u64>>,
    lines: Mutex<Vec<(Level, String, String)>>,
    outbound: Mutex<Vec<(Outgoing, Answer)>>,
}

impl Spent {
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
}

/// `request` sent through `Core\Http\Client`'s transport, under `ctx`'s grants.
fn sent(ctx: &mut Ctx, request: Outgoing) -> Result<Incoming, Unsent> {
    let Outgoing {
        method,
        url,
        headers,
        body,
    } = request;
    match nvs_stdlib::extension_send(ctx, &method, &url, headers, body) {
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
        self.meter.cpu_spent()
    }

    fn charge(&self, bytes: i64) -> bool {
        self.meter.charge(bytes)
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
            None => self.meter.wall_clock(),
        }
    }

    /// The meter's: a monotonic reading measures an interval that really elapsed, so a fixed
    /// clock does not fix it, as it does not fix `Core\Time::monotonic`.
    fn monotonic_clock(&self) -> u64 {
        self.meter.monotonic_clock()
    }

    fn random(&self, out: &mut [u8]) {
        match &mut *self.seed.lock().unwrap_or_else(PoisonError::into_inner) {
            Some(state) => nvs_stdlib::random::fill_seeded(state, out),
            None => self.meter.random(out),
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

    /// The value at `ext.<block>.<key>` in the request's snapshot, as JSON.
    fn setting(&self, block: &str, key: &str) -> Option<serde_json::Value> {
        let value = self
            .snapshot
            .as_ref()?
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
            capabilities: self
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.config.capabilities.as_ref()),
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
                .map_err(&refused)?,
        };
        let mut args = args
            .iter()
            .map(|arg| crossed(*arg))
            .collect::<Result<Vec<_>, _>>()
            .map_err(refused)?;
        let in_force = ctx
            .config()
            .map_or(nvs_config::image::DEFAULT, nvs_config::image::in_force);
        nvs_ext::builtin::cap_pixels(class, method, &mut args, in_force);
        let (request, spent) = self.request(ctx)?;
        spent.enter(ctx);
        let called = drive(request.call_values(extension, method, args), || {
            spent.sends(ctx)
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
                    .map_err(&refused)?;
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
                Outcome::Fatal(limit, message) => {
                    ctx.run_limit_handler(match limit {
                        Limit::Cpu => nvs_runtime::Limit::CpuTime,
                        Limit::Memory => nvs_runtime::Limit::Memory,
                    });
                    Fault::fatal(message)
                }
            }),
        }
    }
}

/// Runs `call` to the end on the calling task. Between two polls it runs `between`, and polls
/// again at once when that did some work, or parks the task for up to a tick.
fn drive<T>(call: impl Future<Output = T>, mut between: impl FnMut() -> bool) -> T {
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut call = std::pin::pin!(call);
    loop {
        if let Poll::Ready(out) = call.as_mut().poll(&mut cx) {
            return out;
        }
        if between() {
            continue;
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
        Crossed::String(text) => Value::str(NvsStr::new(text.as_bytes())),
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
        Crossed::Case(name) => Value::int(nvs_ext::convert::case_number(ty, &name)?),
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
        let spent = Spent {
            meter: Meter::new(UNCAPPED, None),
            snapshot: ctx.config().map(|config| Arc::clone(config.snapshot())),
            narrowed: None,
            clock: Mutex::new(None),
            seed: Mutex::new(None),
            lines: Mutex::new(Vec::new()),
            outbound: Mutex::new(Vec::new()),
        };

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

    /// A guest's outbound request waits until the call polls it, and is then sent under the
    /// calling context's grants: a context with no `net.connect` grant gets no response.
    #[test]
    fn a_guest_request_is_sent_at_the_next_poll_under_the_contexts_grants() {
        let mut ctx = Ctx::buffered();
        let spent = Spent {
            meter: Meter::new(UNCAPPED, None),
            snapshot: None,
            narrowed: None,
            clock: Mutex::new(None),
            seed: Mutex::new(None),
            lines: Mutex::new(Vec::new()),
            outbound: Mutex::new(Vec::new()),
        };
        let outbound = spent.send(Outgoing {
            method: "GET".to_owned(),
            url: "http://example.com/".to_owned(),
            headers: Vec::new(),
            body: None,
        });

        assert!(spent.sends(&mut ctx));
        assert!(!spent.sends(&mut ctx));
        let answer = drive(outbound, || false);
        assert!(
            matches!(&answer, Err(Unsent::Refused(message)) if message.contains("example.com")),
            "{answer:?}"
        );
    }
}
