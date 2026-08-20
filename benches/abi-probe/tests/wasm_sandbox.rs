//! Containment guarantees of the extension sandbox.
//!
//! ADR 0003 claims an extension cannot forge a host pointer, cannot outlive its
//! CPU budget, and cannot carry state from one request to the next. Those are the
//! reasons wasm was chosen over `dlopen`, so they are worth checking rather than
//! assuming.

#![cfg(feature = "wasm-probe")]

use std::time::Duration;

use mwl_abi_probe::wasm::WasmProbe;

const NO_DEADLINE: u64 = u64::MAX;

#[test]
fn a_guest_can_only_read_the_heap_through_validated_indices() {
    let probe = WasmProbe::new().expect("wasm probe");
    let (mut store, instance) = probe.instantiate(16, NO_DEADLINE).expect("instantiate");
    let sum = instance
        .get_typed_func::<i32, i64>(&mut store, "sum_via_host")
        .expect("sum_via_host export");

    // 0..16 present: the guest sums real values.
    let total = sum.call(&mut store, 16).expect("call");
    assert_eq!(total, (0..16).sum::<i64>());
    assert_eq!(store.data().accessor_calls, 16);

    // Reading past the end yields zero rather than adjacent host memory. A
    // native extension handed a raw pointer would read whatever followed.
    let over = sum.call(&mut store, 64).expect("call");
    assert_eq!(
        over,
        (0..16).sum::<i64>(),
        "out-of-range handles must contribute nothing, not leak host memory"
    );
}

#[test]
fn a_runaway_guest_is_stopped_by_its_deadline() {
    // Set a deadline of one epoch tick, then advance the epoch from a watchdog.
    // This is how a guest becomes subject to the request's CPU budget.
    let probe = WasmProbe::new().expect("wasm probe");
    let (mut store, instance) = probe.instantiate(0, 1).expect("instantiate");
    let spin = instance
        .get_typed_func::<(), ()>(&mut store, "spin")
        .expect("spin export");

    let engine = probe.engine().clone();
    let watchdog = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        engine.increment_epoch();
    });

    let result = spin.call(&mut store, ());
    watchdog.join().expect("watchdog");

    let err = result.expect_err("an infinite guest loop must not return normally");
    // Wasmtime reports this as a trap; the message wording is not contractual,
    // so only the fact of the trap is asserted.
    assert!(
        !err.to_string().is_empty(),
        "the trap should carry a diagnostic"
    );
}

#[test]
fn each_instance_starts_from_a_pristine_state() {
    // The property PHP cannot offer: an extension cannot observe, on one
    // request, anything it stored on a previous one. Each request gets its own
    // instance with its own linear memory.
    let probe = WasmProbe::pooled(64).expect("pooled probe");

    let sentinel = [0xDEu8, 0xAD, 0xBE, 0xEF];
    let mut first = [0u8; 4];

    for round in 0..8 {
        let (mut store, instance) = probe.instantiate(0, NO_DEADLINE).expect("instantiate");
        let mem = instance.get_memory(&mut store, "mem").expect("mem export");

        let mut seen = [0u8; 4];
        mem.read(&store, 0, &mut seen).expect("read");
        if round == 0 {
            first = seen;
        }
        assert_eq!(
            seen, first,
            "round {round}: memory must be identical to a fresh instance"
        );
        assert_eq!(seen, [0, 0, 0, 0], "a fresh linear memory is zeroed");

        // Scribble on it, as a stateful extension would.
        mem.write(&mut store, 0, &sentinel).expect("write");
        let mut check = [0u8; 4];
        mem.read(&store, 0, &mut check).expect("read");
        assert_eq!(check, sentinel, "the write itself should work");
    }
}

#[test]
fn accessor_state_does_not_carry_between_instances() {
    let probe = WasmProbe::pooled(64).expect("pooled probe");

    for _ in 0..4 {
        let (mut store, instance) = probe.instantiate(8, NO_DEADLINE).expect("instantiate");
        assert_eq!(store.data().accessor_calls, 0, "counters start at zero");

        let sum = instance
            .get_typed_func::<i32, i64>(&mut store, "sum_via_host")
            .expect("sum_via_host export");
        sum.call(&mut store, 8).expect("call");
        assert_eq!(store.data().accessor_calls, 8);
    }
}

#[test]
fn a_compiled_guest_is_reusable_across_many_instances() {
    // Compile once, instantiate many: the same arrangement MWL uses for its own
    // compiled units, and what keeps per-request cost to instantiation only.
    let probe = WasmProbe::pooled(128).expect("pooled probe");

    for i in 0..256i64 {
        let (mut store, instance) = probe.instantiate(0, NO_DEADLINE).expect("instantiate");
        let add = instance
            .get_typed_func::<(i64, i64), i64>(&mut store, "add")
            .expect("add export");
        assert_eq!(add.call(&mut store, (i, 1)).expect("call"), i + 1);
    }
}
