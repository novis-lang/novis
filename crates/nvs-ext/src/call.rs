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
//! **The budget is a trait.** [`Budget`] is the two questions the store asks of its request, so
//! this crate does not link the runtime. The host that wires a call into a request implements it
//! over the request's own deadline and memory accounting; [`Meter`] is a standalone budget for a
//! test and for a run with no request around it.
//!
//! What it spends: one instance per extension a request calls, its linear memory charged to that
//! request and freed with it, so O(in-flight). The pooling allocator reserves address space for
//! [`Host::new`]'s `slots` instances, not committed memory, and one ticker thread per process.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Poll, Waker};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::{fmt, future};

use wasmtime::component::{ComponentExportIndex, Instance, Linker, Val};
use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, PoolingAllocationConfig, ResourceLimiter, Store,
    Trap, UpdateDeadline,
};

use crate::load::Extension;

/// How often the ticker advances the engine's epoch, which is how often a running guest checks
/// its request's CPU deadline and yields to the other tasks on its core.
pub const TICK: Duration = Duration::from_millis(1);

/// The questions a guest's store asks of the request it runs for.
pub trait Budget: Send + Sync {
    /// Whether the request has used all of its CPU time.
    fn cpu_spent(&self) -> bool;

    /// Charges `bytes` of guest linear memory to the request, or gives them back when negative.
    /// Returns `false`, and charges nothing, when the request cannot afford them.
    fn charge(&self, bytes: i64) -> bool;
}

/// A budget of its own: a CPU deadline and an optional memory limit, with what is charged counted
/// here.
#[derive(Debug)]
pub struct Meter {
    deadline: Instant,
    memory: Option<u64>,
    charged: AtomicI64,
}

impl Meter {
    /// A budget of `cpu` from now and at most `memory` bytes, or no memory limit for `None`.
    #[must_use]
    pub fn new(cpu: Duration, memory: Option<u64>) -> Self {
        Self {
            deadline: Instant::now() + cpu,
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
}

/// The request limit a call reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    /// The request's CPU time.
    Cpu,
    /// The request's memory, or the entry's or the manifest's ceiling where that is smaller.
    Memory,
}

/// Why a call did not return a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The guest reached a request limit. The request ends.
    Limit(Limit),
    /// The guest trapped, or did not instantiate.
    Trap(String),
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
            Self::Trap(reason) => write!(f, "the extension crashed: {reason}"),
            Self::NoMethod(name) => write!(f, "the extension has no method `{name}`"),
        }
    }
}

impl std::error::Error for Failure {}

/// What one instance's store carries: its request's budget, and what it charged.
pub struct Guest {
    budget: Arc<dyn Budget>,
    ceiling: Option<u64>,
    charged: u64,
    reached: Option<Limit>,
    live: Arc<AtomicUsize>,
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

impl Host {
    /// A host with room for `slots` instances at once, its linker filled by `imports`, and its
    /// ticker started.
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
        pool.total_core_instances(slots);
        pool.total_memories(slots);
        pool.total_tables(slots);
        pool.total_stacks(slots);
        let mut config = Config::new();
        config.epoch_interruption(true);
        // One call per store at a time, on wasmtime's own fiber: a component is not re-entered.
        config.concurrency_support(false);
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
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
        }
    }
}

/// One request's instances, one per extension it called, dropped with it.
pub struct Request {
    host: Host,
    budget: Arc<dyn Budget>,
    slots: Mutex<Vec<Slot>>,
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

    /// Calls `method` of `extension` with `args`, and returns its results.
    ///
    /// # Errors
    ///
    /// A [`Failure`]: a limit the guest reached, a trap, or a method the extension does not have.
    pub async fn call(
        &self,
        extension: &Extension,
        method: &str,
        args: &[Val],
    ) -> Result<Vec<Val>, Failure> {
        let declared = extension
            .manifest
            .methods
            .iter()
            .find(|declared| declared.name.eq_ignore_ascii_case(method))
            .ok_or_else(|| Failure::NoMethod(method.to_owned()))?;
        let export = declared.export_name();
        let mut held = self.acquire(&extension.manifest.class).await;
        if held.live.is_none() {
            held.live = Some(Box::new(self.instantiate(extension).await?));
        }
        let Some(live) = held.live.as_mut() else {
            unreachable!("the instance was made above");
        };
        let outcome = live.call(&export, method, args).await;
        if outcome.is_err() {
            held.live = None;
        }
        outcome
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
    async fn instantiate(&self, extension: &Extension) -> Result<Live, Failure> {
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
            Err(err) => return Err(failure(&store, &err)),
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

impl Live {
    async fn call(
        &mut self,
        export: &str,
        method: &str,
        args: &[Val],
    ) -> Result<Vec<Val>, Failure> {
        let func = self
            .instance
            .get_export_index(&mut self.store, Some(&self.interface), export)
            .and_then(|index| self.instance.get_func(&mut self.store, index))
            .ok_or_else(|| Failure::NoMethod(method.to_owned()))?;
        let mut results = vec![Val::Bool(false); func.ty(&self.store).results().len()];
        func.call_async(&mut self.store, args, &mut results)
            .await
            .map_err(|err| failure(&self.store, &err))?;
        Ok(results)
    }
}

/// The failure `err` is, in a store that may have reached a limit.
fn failure(store: &Store<Guest>, err: &wasmtime::Error) -> Failure {
    if let Some(limit) = store.data().reached {
        return Failure::Limit(limit);
    }
    if err.downcast_ref::<Trap>() == Some(&Trap::Interrupt) {
        return Failure::Limit(Limit::Cpu);
    }
    Failure::Trap(format!("{err:#}"))
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
