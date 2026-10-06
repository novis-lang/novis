//! Harness for the extension-sandbox probes.
//!
//! Guards the cost and containment claims in
//! `docs/decisions/0003.md`. Only compiled with the `wasm-probe`
//! feature, so day-to-day builds do not pay for Wasmtime.
//!
//! The module under test is deliberately minimal WAT rather than a real
//! extension: the numbers that matter are boundary crossings and instantiation,
//! and a trivial guest isolates those from whatever an extension actually
//! computes.
//!
//! [`AsyncProbe`] is the second guest, a component, and it proves the async
//! bridge of `rule:packaging/a-guest-call-yields-on-its-core`: a guest call is
//! a wasmtime async call that a stackful coroutine polls, a `Pending` poll
//! parks the coroutine, and [`CoreLoop`] — one thread, one run queue, a waker
//! per task — is the smallest model of a core's scheduler that the claim can
//! be checked against. `tests/wasm_async.rs` holds the checks.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use corosensei::stack::DefaultStack;
use corosensei::{Coroutine, CoroutineResult, Yielder};
use wasmtime::component::{Component, Instance as ComponentInstance, Linker as ComponentLinker};
use wasmtime::{
    Caller, Config, Engine, Instance, InstanceAllocationStrategy, Linker, Module,
    PoolingAllocationConfig, Store, UpdateDeadline,
};

use crate::Value;

/// Exports:
///
/// * `add` — the cheapest possible call, isolating host→guest boundary cost.
/// * `sum_via_host` — calls an imported host function in a loop, which is the
///   shape of Novis's value-accessor API: the guest reads host-owned values
///   through validated handles instead of being handed a pointer.
/// * `spin` — an infinite loop, for checking that a runaway extension can be
///   stopped by the request's CPU budget.
pub const PROBE_WAT: &str = r#"
(module
  (import "nvs" "value_int" (func $value_int (param i32) (result i64)))
  (func (export "add") (param i64 i64) (result i64)
    local.get 0
    local.get 1
    i64.add)
  (func (export "sum_via_host") (param i32) (result i64)
    (local $i i32) (local $acc i64)
    (local.set $i (i32.const 0))
    (local.set $acc (i64.const 0))
    (block $done
      (loop $again
        (br_if $done (i32.ge_s (local.get $i) (local.get 0)))
        (local.set $acc (i64.add (local.get $acc)
                                 (call $value_int (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $again)))
    local.get $acc)
  (func (export "spin")
    (loop $forever (br $forever)))
  (memory (export "mem") 2)
)
"#;

/// Stands in for the per-request state a real host call would reach through.
#[derive(Debug, Default)]
pub struct HostState {
    /// The request's Novis values, which the guest may only read by index.
    pub heap: Vec<Value>,
    /// How many accessor calls the guest made.
    pub accessor_calls: u64,
}

/// A compiled guest plus the host imports it links against.
///
/// The engine and module are built once and reused, mirroring how Novis will cache
/// a compiled extension and share it across every core and request.
pub struct WasmProbe {
    engine: Engine,
    module: Module,
    linker: Linker<HostState>,
}

impl std::fmt::Debug for WasmProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmProbe").finish_non_exhaustive()
    }
}

impl WasmProbe {
    /// Builds a probe with the on-demand allocator.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine cannot be configured or the guest fails to
    /// compile — both indicate a broken Wasmtime rather than a runtime condition.
    pub fn new() -> Result<Self> {
        Self::build(None)
    }

    /// Builds a probe with the pooling allocator, which is what makes a fresh
    /// instance per request affordable.
    ///
    /// # Errors
    ///
    /// As [`WasmProbe::new`].
    pub fn pooled(capacity: u32) -> Result<Self> {
        let mut pool = PoolingAllocationConfig::default();
        pool.total_memories(capacity);
        pool.total_tables(capacity);
        pool.total_core_instances(capacity);
        Self::build(Some(pool))
    }

    fn build(pool: Option<PoolingAllocationConfig>) -> Result<Self> {
        let mut config = Config::new();
        // Epoch interruption is how a guest becomes subject to the request's
        // CPU budget. Cheaper than fuel metering and a better match for Novis's
        // safepoint-based interruption model.
        config.epoch_interruption(true);
        if let Some(pool) = pool {
            config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
        }

        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, PROBE_WAT)?;

        let mut linker: Linker<HostState> = Linker::new(&engine);
        linker.func_wrap(
            "nvs",
            "value_int",
            |mut caller: Caller<'_, HostState>, idx: i32| -> i64 {
                let state = caller.data_mut();
                state.accessor_calls += 1;
                // The security property being modelled: the guest presents an
                // index, which the host bounds-checks. A guest cannot forge a
                // pointer into the host heap because it never sees one.
                usize::try_from(idx)
                    .ok()
                    .and_then(|i| state.heap.get(i))
                    .map_or(0, |v| v.as_int())
            },
        )?;

        Ok(Self {
            engine,
            module,
            linker,
        })
    }

    /// The engine, for driving epoch increments from a watchdog thread.
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Creates a pristine instance, as a new request would.
    ///
    /// `heap_len` values are placed in the host state for the guest to read
    /// through the accessor. `deadline` is the epoch budget; pass [`u64::MAX`]
    /// when the probe is not testing interruption.
    ///
    /// # Errors
    ///
    /// Returns an error if instantiation fails, including when the pooling
    /// allocator's capacity is exhausted.
    pub fn instantiate(
        &self,
        heap_len: usize,
        deadline: u64,
    ) -> Result<(Store<HostState>, Instance)> {
        let heap = (0..heap_len)
            .map(|i| Value::int(i64::try_from(i).unwrap_or(i64::MAX)))
            .collect();
        let mut store = Store::new(
            &self.engine,
            HostState {
                heap,
                accessor_calls: 0,
            },
        );
        store.set_epoch_deadline(deadline);
        let instance = self.linker.instantiate(&mut store, &self.module)?;
        Ok((store, instance))
    }
}

/// The async-bridge guest, a component. It imports:
///
/// * `wait` — an async host function that returns twice its argument once the
///   store's [`Gate`] is open, and is pending until then.
/// * `flag` — a sync host function that reports the store's flag.
///
/// It exports:
///
/// * `echo` — calls `wait` with its argument and returns the result, so a call
///   suspends inside wasm, in the middle of a host import.
/// * `spin-until-flag` — loops until `flag` is set and returns how many times
///   it went round. Only another task can set the flag, so the call returns
///   only if an epoch tick yielded it to that task.
/// * `forever` — an infinite loop, for the CPU deadline.
pub const ASYNC_PROBE_WAT: &str = r#"
(component
  (import "wait" (func $wait (param "x" u64) (result u64)))
  (import "flag" (func $flag (result bool)))
  (core func $wait_low (canon lower (func $wait)))
  (core func $flag_low (canon lower (func $flag)))
  (core module $guest
    (import "host" "wait" (func $wait (param i64) (result i64)))
    (import "host" "flag" (func $flag (result i32)))
    (func (export "echo") (param i64) (result i64)
      local.get 0
      call $wait)
    (func (export "spin-until-flag") (result i64)
      (local $n i64)
      (block $done
        (loop $again
          (local.set $n (i64.add (local.get $n) (i64.const 1)))
          (br_if $done (call $flag))
          (br $again)))
      local.get $n)
    (func (export "forever")
      (loop $again (br $again))))
  (core instance $host
    (export "wait" (func $wait_low))
    (export "flag" (func $flag_low)))
  (core instance $inst (instantiate $guest (with "host" (instance $host))))
  (func (export "echo") (param "x" u64) (result u64)
    (canon lift (core func $inst "echo")))
  (func (export "spin-until-flag") (result u64)
    (canon lift (core func $inst "spin-until-flag")))
  (func (export "forever")
    (canon lift (core func $inst "forever")))
)
"#;

/// A condition a host import waits on, opened by another task or thread.
///
/// It stands in for whatever a real waiting import parks on — a file read on
/// the blocking pool, an outbound HTTP call — and its waiter is woken exactly
/// as the reactor wakes a task whose socket became ready.
#[derive(Debug, Default)]
pub struct Gate {
    open: AtomicBool,
    waiter: Mutex<Option<Waker>>,
}

impl Gate {
    /// Opens the gate and wakes the import waiting on it, if there is one.
    pub fn open(&self) {
        self.open.store(true, Ordering::Release);
        let waiter = self
            .waiter
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(waker) = waiter {
            waker.wake();
        }
    }

    /// Whether [`Gate::open`] has run.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }
}

/// The future the `wait` import returns: pending until its gate opens.
struct GateWait(Arc<Gate>);

impl Future for GateWait {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.is_open() {
            return Poll::Ready(());
        }
        *self.0.waiter.lock().unwrap_or_else(PoisonError::into_inner) = Some(cx.waker().clone());
        // Checked again under the stored waker, so an `open` that ran between
        // the first check and the store is not lost.
        if self.0.is_open() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// What one guest call's store carries: the request-side state an import or
/// the epoch callback reads.
#[derive(Debug)]
pub struct GuestState {
    /// What the `wait` import waits on.
    pub gate: Arc<Gate>,
    /// What the `flag` import reports.
    pub flag: Arc<AtomicBool>,
    /// The request's CPU deadline. At an epoch tick past it the call traps.
    pub cpu_deadline: Instant,
    /// How many epoch ticks yielded this store's call to the core.
    pub yields: u64,
}

impl GuestState {
    /// A closed gate, a clear flag, and a CPU deadline `cpu_budget` from now.
    #[must_use]
    pub fn new(cpu_budget: Duration) -> Self {
        Self {
            gate: Arc::default(),
            flag: Arc::default(),
            cpu_deadline: Instant::now() + cpu_budget,
            yields: 0,
        }
    }
}

/// A compiled component guest plus its host imports, configured the way
/// `rule:packaging/a-guest-call-yields-on-its-core` runs one: pooled
/// instances, epoch interruption, and every call async.
pub struct AsyncProbe {
    engine: Engine,
    component: Component,
    linker: ComponentLinker<GuestState>,
}

impl std::fmt::Debug for AsyncProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncProbe").finish_non_exhaustive()
    }
}

impl AsyncProbe {
    /// Builds the probe with a pooling allocator of `capacity` instances.
    ///
    /// Concurrency support is off, so `call_async` runs the call on
    /// wasmtime's own fiber, one call per store at a time. That is the model
    /// the rule describes: a component is not re-entered, and a second task
    /// of the same request waits for the first call to return.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine cannot be configured or the guest fails
    /// to compile — both indicate a broken Wasmtime rather than a runtime
    /// condition.
    pub fn pooled(capacity: u32) -> Result<Self> {
        let mut pool = PoolingAllocationConfig::default();
        pool.total_component_instances(capacity);
        pool.total_core_instances(capacity);
        pool.total_memories(capacity);
        pool.total_tables(capacity);
        pool.total_stacks(capacity);

        let mut config = Config::new();
        config.epoch_interruption(true);
        config.concurrency_support(false);
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

        let engine = Engine::new(&config)?;
        let component = Component::new(&engine, ASYNC_PROBE_WAT)?;

        let mut linker: ComponentLinker<GuestState> = ComponentLinker::new(&engine);
        let mut root = linker.root();
        root.func_wrap_async("wait", |store, (x,): (u64,)| {
            let gate = Arc::clone(&store.data().gate);
            Box::new(async move {
                GateWait(gate).await;
                Ok((x.saturating_mul(2),))
            })
        })?;
        root.func_wrap("flag", |store, (): ()| {
            Ok((store.data().flag.load(Ordering::Acquire),))
        })?;

        Ok(Self {
            engine,
            component,
            linker,
        })
    }

    /// The engine, for an [`EpochTicker`].
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// A store for one call, as a request's first call would create it.
    ///
    /// The epoch deadline is one tick away and renewed at every tick by a
    /// callback that traps past the state's CPU deadline and otherwise yields.
    #[must_use]
    pub fn store(&self, state: GuestState) -> Store<GuestState> {
        let mut store = Store::new(&self.engine, state);
        store.set_epoch_deadline(1);
        store.epoch_deadline_callback(|mut cx| {
            let state = cx.data_mut();
            if Instant::now() >= state.cpu_deadline {
                return Ok(UpdateDeadline::Interrupt);
            }
            state.yields += 1;
            Ok(UpdateDeadline::Yield(1))
        });
        store
    }

    /// Instantiates the guest in `store`.
    ///
    /// # Errors
    ///
    /// Returns an error if instantiation fails, including when the pooling
    /// allocator's capacity is exhausted.
    pub async fn instantiate(&self, store: &mut Store<GuestState>) -> Result<ComponentInstance> {
        Ok(self
            .linker
            .instantiate_async(store, &self.component)
            .await?)
    }
}

/// Advances an engine's epoch on its own thread until dropped, as the host's
/// epoch ticker does about once a millisecond.
#[derive(Debug)]
pub struct EpochTicker {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl EpochTicker {
    /// Starts a ticker that advances `engine`'s epoch every `every`.
    #[must_use]
    pub fn start(engine: &Engine, every: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let engine = engine.clone();
        let flag = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            while !flag.load(Ordering::Acquire) {
                std::thread::sleep(every);
                engine.increment_epoch();
            }
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for EpochTicker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            // A ticker thread that panicked has nothing left to stop.
            let _ = thread.join();
        }
    }
}

/// Each task's coroutine stack. The wasm frames live on wasmtime's own fiber
/// stack, so this one holds only the poll loop and the host's frames.
const TASK_STACK: usize = 256 * 1024;

/// How long [`CoreLoop::run`] waits for a wake with every task parked before
/// it reports the tasks as stuck.
const STALL: Duration = Duration::from_secs(10);

/// The ready queue a [`CoreLoop`] runs from. A waker may push from any
/// thread, as a `RemoteWake` does.
#[derive(Debug, Default)]
struct RunQueue {
    ready: Mutex<VecDeque<usize>>,
    wakeup: Condvar,
}

impl RunQueue {
    fn push(&self, id: usize) {
        self.ready
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(id);
        self.wakeup.notify_one();
    }

    fn pop(&self, wait: Duration) -> Option<usize> {
        let ready = self.ready.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut ready, _) = self
            .wakeup
            .wait_timeout_while(ready, wait, |ready| ready.is_empty())
            .unwrap_or_else(PoisonError::into_inner);
        ready.pop_front()
    }
}

/// One task's waker: it puts the task back on the run queue.
struct TaskWake {
    id: usize,
    queue: Arc<RunQueue>,
}

impl Wake for TaskWake {
    fn wake(self: Arc<Self>) {
        self.queue.push(self.id);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.queue.push(self.id);
    }
}

/// What a task's body polls a future through.
pub struct Park<'a> {
    yielder: &'a Yielder<(), ()>,
    waker: Waker,
}

impl std::fmt::Debug for Park<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Park").finish_non_exhaustive()
    }
}

impl Park<'_> {
    /// Polls `fut` to completion, suspending the task's coroutine at every
    /// `Pending` until its waker puts it back on the run queue.
    pub fn block_on<F: Future>(&self, fut: F) -> F::Output {
        let mut fut = pin!(fut);
        let mut cx = Context::from_waker(&self.waker);
        loop {
            if let Poll::Ready(out) = fut.as_mut().poll(&mut cx) {
                return out;
            }
            self.yielder.suspend(());
        }
    }
}

/// A task [`CoreLoop::run`] finished.
#[derive(Debug)]
pub struct Finished<R> {
    /// What the task's body returned.
    pub out: R,
    /// How many times the task's coroutine suspended.
    pub parks: u64,
}

struct Task<R> {
    co: Coroutine<(), (), R, DefaultStack>,
    parks: u64,
}

/// One core's scheduler, reduced to what the bridge needs: stackful
/// coroutines on one thread, a run queue, and a waker per task.
pub struct CoreLoop<R> {
    tasks: Vec<Option<Task<R>>>,
    queue: Arc<RunQueue>,
}

impl<R> std::fmt::Debug for CoreLoop<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreLoop")
            .field("tasks", &self.tasks.len())
            .finish_non_exhaustive()
    }
}

impl<R> Default for CoreLoop<R> {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            queue: Arc::default(),
        }
    }
}

impl<R: 'static> CoreLoop<R> {
    /// An empty loop.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a task to the back of the run queue and returns its index in what
    /// [`CoreLoop::run`] returns.
    ///
    /// # Errors
    ///
    /// Returns an error if the coroutine's stack cannot be allocated.
    pub fn spawn(&mut self, body: impl FnOnce(&Park<'_>) -> R + 'static) -> Result<usize> {
        let id = self.tasks.len();
        let waker = Waker::from(Arc::new(TaskWake {
            id,
            queue: Arc::clone(&self.queue),
        }));
        let stack = DefaultStack::new(TASK_STACK)?;
        let co = Coroutine::with_stack(stack, move |yielder: &Yielder<(), ()>, ()| {
            body(&Park { yielder, waker })
        });
        self.tasks.push(Some(Task { co, parks: 0 }));
        self.queue.push(id);
        Ok(id)
    }

    /// Runs every task to its end, in spawn order.
    ///
    /// # Errors
    ///
    /// Returns an error if every task left is parked and nothing wakes one
    /// within [`STALL`].
    pub fn run(mut self) -> Result<Vec<Finished<R>>> {
        let mut done: Vec<Option<Finished<R>>> = self.tasks.iter().map(|_| None).collect();
        let mut live = self.tasks.len();
        while live > 0 {
            let Some(id) = self.queue.pop(STALL) else {
                bail!("{live} task(s) parked and nothing woke one");
            };
            // A wake can arrive for a task that already finished, or twice
            // for one park; the second resume polls once more and parks again.
            let Some(task) = self.tasks.get_mut(id).and_then(Option::as_mut) else {
                continue;
            };
            match task.co.resume(()) {
                CoroutineResult::Yield(()) => task.parks += 1,
                CoroutineResult::Return(out) => {
                    let parks = task.parks;
                    self.tasks[id] = None;
                    done[id] = Some(Finished { out, parks });
                    live -= 1;
                }
            }
        }
        Ok(done.into_iter().flatten().collect())
    }
}
