//! A guest call: one instance per extension per request, run as a wasmtime async call on the
//! request's own core, its CPU time and linear memory charged to the request.
//!
//! [`Host`] is the process's half: one engine with epoch interruption and the pooling allocator,
//! the linker every instance is made with, and the ticker thread that advances the epoch every
//! [`TICK`]. A [`Loader`](crate::load::Loader) made over [`Host::engine`] compiles each component
//! once, and every core instantiates that same compiled component.
//!
//! [`Request`] is one request's half (`rule:packaging/a-fresh-instance-per-request`). It holds no
//! instance until the request's first call to an extension, then one per extension it called, and
//! drops them all with itself. A component is not re-entered: while one task's call is running,
//! a second task of the same request calling the same extension waits, parked, until the first
//! call returns. A call that fails drops its instance, so the request's next call to that
//! extension instantiates a fresh one.
//!
//! [`Request::call`] is a future the request's coroutine polls
//! (`rule:packaging/a-guest-call-yields-on-its-core`). wasmtime runs the guest on its own fiber, and
//! the future is pending in two cases: a host import returned a pending future, or an epoch tick
//! reached the guest. At each tick the guest asks [`Budget::cpu_spent`]; a spent budget traps the
//! call as [`Limit::Cpu`], and otherwise the call yields once so the core's other tasks run. The
//! waker is whatever polls the future, which on a server is the core's `RemoteWake`.
//!
//! **Memory** (`rule:packaging/a-guest-runs-under-the-requests-budget`). Every growth of a guest's
//! linear memory, its initial pages included, is charged to the request through
//! [`Budget::charge`], and given back when the instance is dropped. A growth fails the call as
//! [`Limit::Memory`] when the request cannot afford it, or when it takes one memory past the
//! smaller of the entry's `memory` and the manifest's `memory`. So the store's limit is the least
//! of the three. Reaching any of them is the same limit, because the store has one: the request
//! ends, whichever of the three was smallest.
//!
//! **Failures** (`rule:packaging/a-guest-crash-throws`). Every export the loader accepts returns
//! `result<T, error>`, and [`Request::call`] returns the `ok` side's value or a [`Failure`]:
//! [`Failure::Error`] for an `err`, which keeps the instance, and [`Failure::Trap`] or
//! [`Failure::Limit`], which drop it. [`Failure::outcome`] is what the program sees of each: a
//! [`Outcome::Throw`] of the class the world maps an `err` to, or of `ExtensionError` for a trap,
//! or an [`Outcome::Fatal`] for a limit. The class and the limit are named in the spellings
//! `nvs_runtime::ThrownClass::name` and `nvs_runtime::Limit::name` use, which
//! `crates/nvs-ext/tests/failure.rs` pins, so the host that wires a call into a request raises the
//! throw with `Ctx::set_pending_as`, or runs `Ctx::run_limit_handler` and records the `FATAL`,
//! without this crate linking the runtime.
//!
//! **Novis values.** [`Request::call_values`] takes a call's Novis values, converts each by the
//! type the manifest declares for its parameter (`crate::convert`), lends a `mixed` one as a handle
//! (`crate::handle`), and converts the result back. An argument that is not a value of its type is
//! [`Error::Invalid`]: it throws `LogicError` and keeps the instance. A result that does not convert
//! is the extension's fault, a [`Failure::Trap`]. When the call returns, the borrows it was lent
//! are dropped and every handle it made stops being valid, whatever the outcome. It parses the
//! method's types on every call.
//!
//! **A resource lives until the request ends** (`rule:packaging/a-value-crosses-as-its-wit-type`).
//! One a call returns is kept in the store of the instance that made it, under a number the
//! request gives out once, and the program holds [`Value::Resource`]. Passed back, it crosses as a
//! `borrow`, so the request still owns it. [`Request::end`] drops every kept resource in its own
//! instance, which runs the guest's destructor, before the instances go. An instance dropped
//! earlier, by a trap or a limit, takes its resources with it and runs no destructor, and so does a
//! request dropped without [`Request::end`]; a number kept by an instance that is gone is refused
//! as [`Error::Invalid`], because no later instance gives it out again.
//!
//! **The budget is a trait.** [`Budget`] is what the store needs of its request — its CPU
//! deadline, its memory accounting, its log, its clocks and its random generator — so this crate
//! does not link the runtime. The host that wires a call into a request implements it over the
//! request's own; [`Meter`] is a standalone budget for a test and for a run with no request around
//! it, keeps no log, and reads the process's clocks and generator.
//!
//! What it spends: one instance per extension a request calls, its linear memory charged to that
//! request and freed with it, so O(in-flight). The pooling allocator reserves address space for
//! [`Host::new`]'s `slots` instances, not committed memory, and one ticker thread per process.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Poll, Waker};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{fmt, future};

use rand::Rng;
use wasmtime::component::{ComponentExportIndex, Instance, Linker, ResourceAny, Val};
use wasmtime::{
    AsContextMut, Config, Engine, InstanceAllocationStrategy, PoolingAllocationConfig,
    ResourceLimiter, Store, Trap, UpdateDeadline,
};

use crate::convert::{self, Crossing, Value};
use crate::handle::{self, Handles};
use crate::load::Extension;
use crate::manifest::Method;
use crate::wasi;

/// How often the ticker advances the engine's epoch, which is how often a running guest checks
/// its request's CPU deadline and yields to the other tasks on its core.
pub const TICK: Duration = Duration::from_millis(1);

/// What a guest's store needs of the request it runs for.
pub trait Budget: Send + Sync {
    /// Whether the request has used all of its CPU time.
    fn cpu_spent(&self) -> bool;

    /// Charges `bytes` of guest linear memory to the request, or gives them back when negative.
    /// Returns `false`, and charges nothing, when the request cannot afford them.
    fn charge(&self, bytes: i64) -> bool;

    /// Writes `message` to the request's log at `level`, with `channel` as its channel.
    fn log(&self, level: Level, channel: &str, message: &str);

    /// The request's wall clock: the time since the Unix epoch, which a fixed clock fixes.
    fn wall_clock(&self) -> Duration;

    /// The request's monotonic clock, in nanoseconds since an origin the request chooses.
    fn monotonic_clock(&self) -> u64;

    /// Fills `out` from the request's random generator, which a declared seed fixes.
    fn random(&self, out: &mut [u8]);
}

/// The levels of a request's log, as `nvs:ext/log` and `Core\Log` name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// `debug`.
    Debug,
    /// `info`.
    Info,
    /// `warn`.
    Warn,
    /// `error`.
    Error,
    /// `critical`.
    Critical,
}

/// A budget of its own: a CPU deadline and an optional memory limit, with what is charged counted
/// here. Its clocks are the process's, its monotonic clock counting from the meter's creation,
/// and its random bytes come from `rand::rng()`, the thread's ChaCha12 generator `Core\Random`
/// draws from.
#[derive(Debug)]
pub struct Meter {
    started: Instant,
    deadline: Instant,
    memory: Option<u64>,
    charged: AtomicI64,
}

impl Meter {
    /// A budget of `cpu` from now and at most `memory` bytes, or no memory limit for `None`.
    #[must_use]
    pub fn new(cpu: Duration, memory: Option<u64>) -> Self {
        let started = Instant::now();
        Self {
            started,
            deadline: started + cpu,
            memory,
            charged: AtomicI64::new(0),
        }
    }

    /// The bytes charged now.
    #[must_use]
    pub fn charged(&self) -> u64 {
        u64::try_from(self.charged.load(Ordering::Acquire)).unwrap_or(0)
    }
}

impl Budget for Meter {
    fn cpu_spent(&self) -> bool {
        Instant::now() >= self.deadline
    }

    fn charge(&self, bytes: i64) -> bool {
        self.charged
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |now| {
                let next = now.saturating_add(bytes);
                let over = bytes > 0
                    && self
                        .memory
                        .is_some_and(|limit| u64::try_from(next).unwrap_or(0) > limit);
                (!over).then_some(next)
            })
            .is_ok()
    }

    /// A meter has no request log, so it keeps nothing a guest writes.
    fn log(&self, _level: Level, _channel: &str, _message: &str) {}

    fn wall_clock(&self) -> Duration {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
    }

    fn monotonic_clock(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    fn random(&self, out: &mut [u8]) {
        rand::rng().fill_bytes(out);
    }
}

/// The request limit a call reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    /// The request's CPU time.
    Cpu,
    /// The request's memory, or the entry's or the manifest's ceiling where that is smaller.
    Memory,
}

impl Limit {
    /// The `[limits]` directive this limit is, as `nvs_runtime::Limit::name` spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cpu => "cpu_time",
            Self::Memory => "memory",
        }
    }
}

/// An export's `err`: `nvs:ext/types`'s `error` variant, with its message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// `invalid(m)`: the caller passed a value the extension does not accept.
    Invalid(String),
    /// `parse(m)`: an input the extension reads is malformed.
    Parse(String),
    /// `runtime(m)`: the extension failed for a reason of its own.
    Runtime(String),
}

/// A guest that trapped, or did not instantiate, in a call to one export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crash {
    /// The extension's class.
    pub extension: String,
    /// The export the call was for.
    pub export: String,
    /// What the guest did.
    pub reason: String,
}

/// Why a call did not return a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The guest reached a request limit. The request ends.
    Limit(Limit),
    /// The export returned `err`. The instance is kept.
    Error(Error),
    /// The guest trapped, or did not instantiate. The instance is dropped.
    Trap(Crash),
    /// The manifest declares no such method, or the component does not export it.
    NoMethod(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit(Limit::Cpu) => {
                f.write_str("the extension used all of the request's CPU time")
            }
            Self::Limit(Limit::Memory) => {
                f.write_str("the extension used all of the memory it may use")
            }
            Self::Error(
                Error::Invalid(message) | Error::Parse(message) | Error::Runtime(message),
            ) => f.write_str(message),
            Self::Trap(crash) => write!(
                f,
                "the extension `{}` crashed in `{}`: {}",
                crash.extension, crash.export, crash.reason
            ),
            Self::NoMethod(name) => write!(f, "the extension has no method `{name}`"),
        }
    }
}

impl std::error::Error for Failure {}

/// The class a failure throws, as `nvs_hir::errors::TREE` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// `LogicError`, for `err(invalid(m))`.
    Logic,
    /// `ParseError`, for `err(parse(m))`.
    Parse,
    /// `RuntimeError`, for `err(runtime(m))`.
    Runtime,
    /// `ExtensionError`, for a trap.
    Extension,
}

impl Class {
    /// The class's name, as `nvs_runtime::ThrownClass::name` spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Logic => "LogicError",
            Self::Parse => "ParseError",
            Self::Runtime => "RuntimeError",
            Self::Extension => "ExtensionError",
        }
    }
}

/// What the program sees of a failed call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// A throw of the class, with the message. A `catch` sees it, and the request goes on.
    Throw(Class, String),
    /// A resource-limit `FATAL`, with the message. It reaches `Core\Fatal::onLimit` with the
    /// limit's name, no `catch` sees it, and the request ends.
    Fatal(Limit, String),
}

impl Failure {
    /// What the program sees of this failure.
    #[must_use]
    pub fn outcome(&self) -> Outcome {
        let class = match self {
            Self::Limit(limit) => return Outcome::Fatal(*limit, self.to_string()),
            Self::Error(Error::Invalid(_)) => Class::Logic,
            Self::Error(Error::Parse(_)) => Class::Parse,
            Self::Error(Error::Runtime(_)) => Class::Runtime,
            // The loader refuses a manifest that does not match the exports, so a missing method
            // is a component the loader did not check, and that is the extension's fault.
            Self::Trap(_) | Self::NoMethod(_) => Class::Extension,
        };
        Outcome::Throw(class, self.to_string())
    }
}

/// What one instance's store carries: its request's budget, and what it charged.
pub struct Guest {
    budget: Arc<dyn Budget>,
    ceiling: Option<u64>,
    charged: u64,
    reached: Option<Limit>,
    live: Arc<AtomicUsize>,
    pub(crate) handles: Handles,
    kept: Vec<Kept>,
    pub(crate) wasi: wasi::Context,
}

/// A resource the request keeps: its number, its type's short name, and the resource.
struct Kept {
    id: u64,
    name: String,
    resource: ResourceAny,
}

impl fmt::Debug for Guest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Guest")
            .field("ceiling", &self.ceiling)
            .field("charged", &self.charged)
            .finish_non_exhaustive()
    }
}

impl Drop for Guest {
    fn drop(&mut self) {
        self.budget
            .charge(-i64::try_from(self.charged).unwrap_or(i64::MAX));
        self.live.fetch_sub(1, Ordering::AcqRel);
    }
}

impl ResourceLimiter for Guest {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if maximum.is_some_and(|maximum| desired > maximum) {
            return Ok(false);
        }
        let grow = u64::try_from(desired.saturating_sub(current)).unwrap_or(u64::MAX);
        let past_ceiling = self
            .ceiling
            .is_some_and(|ceiling| u64::try_from(desired).unwrap_or(u64::MAX) > ceiling);
        if past_ceiling || !self.budget.charge(i64::try_from(grow).unwrap_or(i64::MAX)) {
            self.reached = Some(Limit::Memory);
            return Err(wasmtime::format_err!(
                "growing a memory to {desired} bytes is past the limit"
            ));
        }
        self.charged = self.charged.saturating_add(grow);
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(maximum.is_none_or(|maximum| desired <= maximum))
    }
}

/// The process's half: the engine, the linker every instance is made with, and the epoch ticker.
/// Cloning it shares all three.
#[derive(Clone)]
pub struct Host {
    shared: Arc<Shared>,
}

struct Shared {
    engine: Engine,
    linker: Linker<Guest>,
    live: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    ticker: Option<JoinHandle<()>>,
}

impl Drop for Shared {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(ticker) = self.ticker.take() {
            // A ticker that panicked has nothing left to stop.
            let _ = ticker.join();
        }
    }
}

impl fmt::Debug for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Host")
            .field("live_instances", &self.live_instances())
            .finish_non_exhaustive()
    }
}

/// The core instances one guest may hold: its own module, and the shim and fixup modules
/// `wit-component` adds beside it to lower an import through the guest's memory, which every
/// toolchain's libc does.
const CORE_INSTANCES: u32 = 3;

/// The tables one guest may hold: its own module's function table and the shim's.
const TABLES: u32 = 2;

impl Host {
    /// A host with room for `slots` instances at once, its linker filled by `imports`, and its
    /// ticker started. Each slot reserves the pool's address space for one memory,
    /// [`CORE_INSTANCES`] core instances and [`TABLES`] tables, so that a componentized guest
    /// fits in one slot.
    ///
    /// # Errors
    ///
    /// When the engine cannot be configured, or `imports` fails.
    pub fn new(
        slots: u32,
        imports: impl FnOnce(&mut Linker<Guest>) -> wasmtime::Result<()>,
    ) -> wasmtime::Result<Self> {
        let mut pool = PoolingAllocationConfig::default();
        pool.total_component_instances(slots);
        pool.total_core_instances(slots.saturating_mul(CORE_INSTANCES));
        pool.total_memories(slots);
        pool.total_tables(slots.saturating_mul(TABLES));
        pool.total_stacks(slots);
        let mut config = Config::new();
        config.epoch_interruption(true);
        // One call per store at a time, on wasmtime's own fiber: a component is not re-entered.
        config.concurrency_support(false);
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        handle::link(&mut linker)?;
        wasi::link(&mut linker)?;
        imports(&mut linker)?;
        let stop = Arc::new(AtomicBool::new(false));
        let ticker = {
            let engine = engine.clone();
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("nvs-ext-epoch".to_owned())
                .spawn(move || {
                    while !stop.load(Ordering::Acquire) {
                        std::thread::sleep(TICK);
                        engine.increment_epoch();
                    }
                })?
        };
        Ok(Self {
            shared: Arc::new(Shared {
                engine,
                linker,
                live: Arc::default(),
                stop,
                ticker: Some(ticker),
            }),
        })
    }

    /// The engine a [`Loader`](crate::load::Loader) compiles this host's extensions into.
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.shared.engine
    }

    /// How many instances exist now, across every request.
    #[must_use]
    pub fn live_instances(&self) -> usize {
        self.shared.live.load(Ordering::Acquire)
    }

    /// The instances of one request, charged to `budget`.
    #[must_use]
    pub fn request(&self, budget: Arc<dyn Budget>) -> Request {
        Request {
            host: self.clone(),
            budget,
            slots: Mutex::default(),
            ids: AtomicU64::new(0),
        }
    }
}

/// One request's instances, one per extension it called, dropped with it.
pub struct Request {
    host: Host,
    budget: Arc<dyn Budget>,
    slots: Mutex<Vec<Slot>>,
    ids: AtomicU64,
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("instances", &self.instances())
            .finish_non_exhaustive()
    }
}

/// One extension's place in a request: its instance, or the task running a call on it.
struct Slot {
    class: String,
    state: State,
    waiters: Vec<Waker>,
}

enum State {
    Idle(Box<Live>),
    Busy,
}

/// An instance and the store it lives in.
struct Live {
    store: Store<Guest>,
    instance: Instance,
    interface: ComponentExportIndex,
}

impl Request {
    /// How many instances this request holds now, a call's own included.
    #[must_use]
    pub fn instances(&self) -> usize {
        self.lock().len()
    }

    /// Calls `method` of `extension` with `args`, and returns its results: the `ok` side's value
    /// where the export returns a `result`, and every result as it is where it does not.
    ///
    /// # Errors
    ///
    /// A [`Failure`]: a limit the guest reached, the export's `err`, a trap, or a method the
    /// extension does not have.
    pub async fn call(
        &self,
        extension: &Extension,
        method: &str,
        args: &[Val],
    ) -> Result<Vec<Val>, Failure> {
        let declared = declared(extension, method)?;
        self.run(
            extension,
            declared,
            |_, _| Ok(args.to_vec()),
            |_, results, _| Ok(results),
        )
        .await
    }

    /// Calls `method` of `extension` with the Novis values `args`, converted by the types its
    /// manifest declares, and returns its result as a Novis value: `None` for a `void` method.
    ///
    /// # Errors
    ///
    /// A [`Failure`], as [`Request::call`] returns one, and [`Error::Invalid`] where `args` are not
    /// values of the parameters' types.
    pub async fn call_values(
        &self,
        extension: &Extension,
        method: &str,
        args: Vec<Value>,
    ) -> Result<Option<Value>, Failure> {
        let declared = declared(extension, method)?;
        let manifest = &extension.manifest;
        let invalid = |err: String| Failure::Error(Error::Invalid(err));
        if args.len() != declared.params.len() {
            return Err(invalid(format!(
                "`{}` takes {} arguments, and the call passed {}",
                declared.name,
                declared.params.len(),
                args.len()
            )));
        }
        let lower = |store: &mut Store<Guest>, _: At<'_>| {
            let mut call = Call {
                store,
                ids: &self.ids,
            };
            declared
                .params
                .iter()
                .zip(args)
                .map(|(param, value)| {
                    let ty = manifest.novis_type(&param.ty).map_err(invalid)?;
                    convert::to_wit_with(&ty, value, &mut call).map_err(invalid)
                })
                .collect()
        };
        let lift = |store: &mut Store<Guest>, results: Vec<Val>, at: At<'_>| {
            if declared.returns == "void" {
                return Ok(None);
            }
            let ty = manifest
                .novis_type(&declared.returns)
                .map_err(|err| at.crash(err))?;
            match <[Val; 1]>::try_from(results) {
                Ok([val]) => {
                    let mut call = Call {
                        store,
                        ids: &self.ids,
                    };
                    convert::from_wit_with(&ty, val, &mut call)
                        .map(Some)
                        .map_err(|err| at.crash(err))
                }
                Err(results) => {
                    Err(at.crash(format!("it returned {} values, not one", results.len())))
                }
            }
        };
        self.run(extension, declared, lower, lift).await
    }

    /// Runs one call of `declared` on this request's instance of `extension`: `args` makes the
    /// arguments in its store, and `results` reads what the export returned.
    async fn run<R>(
        &self,
        extension: &Extension,
        declared: &Method,
        args: impl FnOnce(&mut Store<Guest>, At<'_>) -> Result<Vec<Val>, Failure>,
        results: impl FnOnce(&mut Store<Guest>, Vec<Val>, At<'_>) -> Result<R, Failure>,
    ) -> Result<R, Failure> {
        let export = declared.export_name();
        let at = At {
            extension: &extension.manifest.class,
            export: &export,
        };
        let mut held = self.acquire(&extension.manifest.class).await;
        if held.live.is_none() {
            held.live = Some(Box::new(self.instantiate(extension, at).await?));
        }
        let Some(live) = held.live.as_mut() else {
            unreachable!("the instance was made above");
        };
        let outcome = match args(&mut live.store, at) {
            Ok(args) => match live.call(at, &declared.name, &args).await {
                Ok(out) => results(&mut live.store, out, at),
                Err(failure) => Err(failure),
            },
            Err(failure) => Err(failure),
        };
        let outcome = match (live.end_call().await, outcome) {
            (Err(err), Ok(_)) => Err(at.crash(err)),
            (_, outcome) => outcome,
        };
        if outcome
            .as_ref()
            .is_err_and(|failure| !matches!(failure, Failure::Error(_)))
        {
            held.live = None;
        }
        outcome
    }

    /// Ends the request: drops every resource its calls returned, in the instance that made it,
    /// so each one's destructor runs in the guest, and then drops the instances.
    ///
    /// # Errors
    ///
    /// The first failure a destructor met. An instance whose destructor failed drops no more of
    /// its resources, and every other instance still drops all of its own.
    pub async fn end(self) -> Result<(), Failure> {
        let slots = std::mem::take(&mut *self.lock());
        let mut ended = Ok(());
        for slot in slots {
            let State::Idle(mut live) = slot.state else {
                continue;
            };
            let kept = std::mem::take(&mut live.store.data_mut().kept);
            for kept in kept {
                if let Err(err) = kept.resource.resource_drop_async(&mut live.store).await {
                    let export = format!("[resource-drop]{}", crate::kebab(&kept.name));
                    let at = At {
                        extension: &slot.class,
                        export: &export,
                    };
                    if ended.is_ok() {
                        ended = Err(failure(&live.store, &err, at));
                    }
                    break;
                }
            }
        }
        ended
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Slot>> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The extension `class`'s slot, marked busy, with its instance if it has one. Pending while
    /// another task's call holds it.
    async fn acquire<'r>(&'r self, class: &'r str) -> Held<'r> {
        let live = future::poll_fn(|cx| {
            let mut slots = self.lock();
            let Some(slot) = slots.iter_mut().find(|slot| slot.class == class) else {
                slots.push(Slot {
                    class: class.to_owned(),
                    state: State::Busy,
                    waiters: Vec::new(),
                });
                return Poll::Ready(None);
            };
            match std::mem::replace(&mut slot.state, State::Busy) {
                State::Idle(live) => Poll::Ready(Some(live)),
                State::Busy => {
                    slot.waiters.push(cx.waker().clone());
                    Poll::Pending
                }
            }
        })
        .await;
        Held {
            request: self,
            class,
            live,
        }
    }

    /// A fresh instance of `extension`, in a store charged to this request.
    async fn instantiate(&self, extension: &Extension, at: At<'_>) -> Result<Live, Failure> {
        let ceiling = match (extension.memory, extension.manifest.memory) {
            (Some(entry), Some(manifest)) => Some(entry.min(manifest)),
            (entry, manifest) => entry.or(manifest),
        };
        self.host.shared.live.fetch_add(1, Ordering::AcqRel);
        let mut store = Store::new(
            &self.host.shared.engine,
            Guest {
                budget: Arc::clone(&self.budget),
                ceiling,
                charged: 0,
                reached: None,
                live: Arc::clone(&self.host.shared.live),
                handles: Handles::default(),
                kept: Vec::new(),
                wasi: wasi::Context::new(Arc::clone(&self.budget), &extension.manifest.class),
            },
        );
        store.limiter(|guest| guest);
        store.set_epoch_deadline(1);
        store.epoch_deadline_callback(|mut cx| {
            let guest = cx.data_mut();
            if guest.budget.cpu_spent() {
                guest.reached = Some(Limit::Cpu);
                return Ok(UpdateDeadline::Interrupt);
            }
            Ok(UpdateDeadline::Yield(1))
        });
        let instance = match self
            .host
            .shared
            .linker
            .instantiate_async(&mut store, &extension.component)
            .await
        {
            Ok(instance) => instance,
            Err(err) => return Err(failure(&store, &err, at)),
        };
        let interface = extension
            .component
            .component_type()
            .exports(&self.host.shared.engine)
            .map(|(name, _)| name)
            .find(|name| crate::load::export_names(name, &extension.manifest.interface))
            .and_then(|name| instance.get_export_index(&mut store, None, name))
            .ok_or_else(|| Failure::NoMethod(extension.manifest.interface.clone()))?;
        Ok(Live {
            store,
            instance,
            interface,
        })
    }
}

/// The method `method` of `extension`'s manifest, matched as a Novis method name is.
fn declared<'e>(extension: &'e Extension, method: &str) -> Result<&'e Method, Failure> {
    extension
        .manifest
        .methods
        .iter()
        .find(|declared| declared.name.eq_ignore_ascii_case(method))
        .ok_or_else(|| Failure::NoMethod(method.to_owned()))
}

impl Live {
    /// Drops the borrows the call's `mixed` arguments were lent as, and ends every handle the call
    /// made.
    async fn end_call(&mut self) -> Result<(), String> {
        let lent = self.store.data_mut().handles.take_lent();
        let mut dropped = Ok(());
        for borrow in lent {
            if let Err(err) = borrow.resource_drop_async(&mut self.store).await {
                dropped = Err(format!("a `value` handle did not drop: {err}"));
            }
        }
        self.store.data_mut().handles.end_call();
        dropped
    }

    async fn call(&mut self, at: At<'_>, method: &str, args: &[Val]) -> Result<Vec<Val>, Failure> {
        let func = self
            .instance
            .get_export_index(&mut self.store, Some(&self.interface), at.export)
            .and_then(|index| self.instance.get_func(&mut self.store, index))
            .ok_or_else(|| Failure::NoMethod(method.to_owned()))?;
        let mut results = vec![Val::Bool(false); func.ty(&self.store).results().len()];
        func.call_async(&mut self.store, args, &mut results)
            .await
            .map_err(|err| failure(&self.store, &err, at))?;
        match <[Val; 1]>::try_from(results) {
            Ok([Val::Result(Ok(value))]) => Ok(value.map(|value| vec![*value]).unwrap_or_default()),
            Ok([Val::Result(Err(err))]) => Err(match err.as_deref() {
                Some(Val::Variant(case, Some(message))) => match (case.as_str(), &**message) {
                    ("invalid", Val::String(m)) => Failure::Error(Error::Invalid(m.clone())),
                    ("parse", Val::String(m)) => Failure::Error(Error::Parse(m.clone())),
                    ("runtime", Val::String(m)) => Failure::Error(Error::Runtime(m.clone())),
                    _ => at.crash("its error is not the world's `error` variant"),
                },
                _ => at.crash("its error is not the world's `error` variant"),
            }),
            Ok(result) => Ok(result.into()),
            Err(results) => Ok(results),
        }
    }
}

/// One call's [`Crossing`]: the store of the instance it runs on, and the request's resource
/// numbers.
struct Call<'s> {
    store: &'s mut Store<Guest>,
    ids: &'s AtomicU64,
}

impl Crossing for Call<'_> {
    fn lend(&mut self, value: Value) -> Result<Val, String> {
        handle::lend(self.store.as_context_mut(), value)
    }

    fn pass(&mut self, name: &str, id: u64) -> Result<Val, String> {
        self.store
            .data()
            .kept
            .iter()
            .find(|kept| kept.id == id && kept.name == name)
            .map(|kept| Val::Resource(kept.resource))
            .ok_or_else(|| {
                format!("the request keeps no `{name}` numbered {id} in this extension's instance")
            })
    }

    fn keep(&mut self, name: &str, resource: ResourceAny) -> Result<Value, String> {
        let id = self.ids.fetch_add(1, Ordering::Relaxed);
        self.store.data_mut().kept.push(Kept {
            id,
            name: name.to_owned(),
            resource,
        });
        Ok(Value::Resource {
            name: name.to_owned(),
            id,
        })
    }
}

/// The extension and the export a call is for, which a trap names.
#[derive(Clone, Copy)]
struct At<'a> {
    extension: &'a str,
    export: &'a str,
}

impl At<'_> {
    fn crash(self, reason: impl Into<String>) -> Failure {
        Failure::Trap(Crash {
            extension: self.extension.to_owned(),
            export: self.export.to_owned(),
            reason: reason.into(),
        })
    }
}

/// The failure `err` is, in a store that may have reached a limit.
fn failure(store: &Store<Guest>, err: &wasmtime::Error, at: At<'_>) -> Failure {
    if let Some(limit) = store.data().reached {
        return Failure::Limit(limit);
    }
    match err.downcast_ref::<Trap>() {
        Some(Trap::Interrupt) => Failure::Limit(Limit::Cpu),
        // The trap or the host function's error alone: the error's context is a backtrace of
        // offsets into the guest.
        Some(trap) => at.crash(trap.to_string()),
        None => at.crash(err.root_cause().to_string()),
    }
}

/// A slot one task's call holds. Dropping it puts the instance back, or removes the slot when the
/// call left no instance, and wakes the tasks waiting for it.
struct Held<'r> {
    request: &'r Request,
    class: &'r str,
    live: Option<Box<Live>>,
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        let live = self.live.take();
        let waiters = {
            let mut slots = self.request.lock();
            let Some(index) = slots.iter().position(|slot| slot.class == self.class) else {
                return;
            };
            match live {
                Some(live) => {
                    slots[index].state = State::Idle(live);
                    std::mem::take(&mut slots[index].waiters)
                }
                None => slots.remove(index).waiters,
            }
        };
        for waker in waiters {
            waker.wake();
        }
    }
}
