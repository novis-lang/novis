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

use anyhow::Result;
use wasmtime::{
    Caller, Config, Engine, Instance, InstanceAllocationStrategy, Linker, Module,
    PoolingAllocationConfig, Store,
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
