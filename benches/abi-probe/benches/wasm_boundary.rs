//! Cost of the extension sandbox boundary.
//!
//! These four numbers decide whether extensions can be sandboxed wasm rather
//! than `dlopen`'d native code. Baselines from the M0 spike on
//! x86_64-pc-windows-msvc, against a 1.3 ns built-in call frame:
//!
//! | measurement | baseline |
//! |---|---|
//! | host → guest call | 11.5 ns |
//! | guest → host accessor call | 9.0 ns |
//! | 1 KiB bulk copy into guest memory | 11.7 ns |
//! | fresh pooled instance + one call | 7.57 µs |
//!
//! See `docs/decisions/0003.md`.

// `criterion_group!` expands to an undocumented public function.
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use nvs_abi_probe::wasm::WasmProbe;

const HEAP_LEN: usize = 1024;
const NO_DEADLINE: u64 = u64::MAX;

fn host_to_guest(c: &mut Criterion) {
    let probe = WasmProbe::new().expect("wasm probe");
    let (mut store, instance) = probe
        .instantiate(HEAP_LEN, NO_DEADLINE)
        .expect("instantiate");
    let add = instance
        .get_typed_func::<(i64, i64), i64>(&mut store, "add")
        .expect("add export");

    c.bench_function("wasm/host_to_guest", |b| {
        b.iter(|| black_box(add.call(&mut store, black_box((1, 2))).expect("call")));
    });
}

fn guest_to_host(c: &mut Criterion) {
    // Measured per accessor call, since this is the pattern Novis's value API
    // uses: the guest pulls what it needs rather than being handed the heap.
    let probe = WasmProbe::new().expect("wasm probe");
    let (mut store, instance) = probe
        .instantiate(HEAP_LEN, NO_DEADLINE)
        .expect("instantiate");
    let sum = instance
        .get_typed_func::<i32, i64>(&mut store, "sum_via_host")
        .expect("sum_via_host export");

    let len = i32::try_from(HEAP_LEN).expect("heap fits in i32");
    let mut group = c.benchmark_group("wasm/guest_to_host");
    group.throughput(criterion::Throughput::Elements(HEAP_LEN as u64));
    group.bench_function("accessor_calls", |b| {
        b.iter(|| black_box(sum.call(&mut store, black_box(len)).expect("call")));
    });
    group.finish();
}

fn bulk_copy(c: &mut Criterion) {
    let probe = WasmProbe::new().expect("wasm probe");
    let (mut store, instance) = probe.instantiate(0, NO_DEADLINE).expect("instantiate");
    let mem = instance.get_memory(&mut store, "mem").expect("mem export");
    let payload = vec![0xABu8; 1024];

    let mut group = c.benchmark_group("wasm/bulk_copy");
    group.throughput(criterion::Throughput::Bytes(payload.len() as u64));
    group.bench_function("1kib_into_guest", |b| {
        b.iter(|| {
            mem.write(&mut store, 0, black_box(&payload))
                .expect("write")
        });
    });
    group.finish();
}

fn instantiation(c: &mut Criterion) {
    // The number that decides whether a fresh instance per request — and
    // therefore per-request isolation for extensions — is affordable.
    let mut group = c.benchmark_group("wasm/instantiate");

    let pooled = WasmProbe::pooled(1000).expect("pooled probe");
    group.bench_function("pooled_plus_one_call", |b| {
        b.iter(|| {
            let (mut store, instance) = pooled.instantiate(0, NO_DEADLINE).expect("instantiate");
            let add = instance
                .get_typed_func::<(i64, i64), i64>(&mut store, "add")
                .expect("add export");
            black_box(add.call(&mut store, (1, 2)).expect("call"));
        });
    });

    let on_demand = WasmProbe::new().expect("on-demand probe");
    group.bench_function("on_demand_plus_one_call", |b| {
        b.iter(|| {
            let (mut store, instance) = on_demand.instantiate(0, NO_DEADLINE).expect("instantiate");
            let add = instance
                .get_typed_func::<(i64, i64), i64>(&mut store, "add")
                .expect("add export");
            black_box(add.call(&mut store, (1, 2)).expect("call"));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    host_to_guest,
    guest_to_host,
    bulk_copy,
    instantiation
);
criterion_main!(benches);
