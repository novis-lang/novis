//! Cost of Novis's checked-return calling convention.
//!
//! Benchmarked by frame depth rather than as a single number, because the figure
//! that matters is the *slope* — the marginal cost of one more frame, which is
//! what a deep Novis call stack pays. The intercept is call-out overhead from the
//! benchmark harness and is not interesting.
//!
//! Baselines on x86_64-pc-windows-msvc: ~0.85 ns marginal cost per frame,
//! measured as the slope between depth 2 and depth 18. See
//! `docs/adr/0002-error-propagation.md`.

// `criterion_group!` expands to an undocumented public function, and a
// benchmark harness has no public API worth documenting anyway.
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use nvs_abi_probe::{Ctx, Helper, Probe, THROWN, Value, call};

fn frame_cost(c: &mut Criterion) {
    let mut probe = Probe::new();
    let mut group = c.benchmark_group("abi/frame_depth");

    for depth in [1usize, 2, 4, 8, 16] {
        let chain = probe.compile_chain(depth, Helper::Double);
        let mut ctx = Ctx::new();
        let arg = Value::int(3);

        group.bench_function(depth.to_string(), |b| {
            b.iter(|| {
                let (status, out) = call(chain, &mut ctx, black_box(arg));
                black_box((status, out));
            });
        });
    }
    group.finish();
}

fn throw_cost(c: &mut Criterion) {
    // A throw should cost about the same as a return. PHP frameworks use
    // exceptions for control flow, so an expensive throw would be a cliff that
    // real applications fall off. Compare against `abi/frame_depth/4`.
    let mut probe = Probe::new();
    let mut group = c.benchmark_group("abi/throw_depth");

    for depth in [1usize, 4, 16] {
        let chain = probe.compile_chain(depth, Helper::Double);
        let mut ctx = Ctx::new();
        // 42 makes the innermost helper raise an Novis-level exception.
        let arg = Value::int(42);

        group.bench_function(depth.to_string(), |b| {
            b.iter(|| {
                let (status, _) = call(chain, &mut ctx, black_box(arg));
                debug_assert_eq!(status, THROWN);
                black_box(status);
            });
        });
    }
    group.finish();
}

fn compile_cost(c: &mut Criterion) {
    // Not part of any ADR claim, but it bounds how bad cold-start can get:
    // Novis compiles on load, so this is the per-function floor before any
    // caching. Tracked so a Cranelift upgrade that halves compile throughput
    // is visible rather than discovered in production.
    c.bench_function("abi/compile_4_frames", |b| {
        b.iter_batched(
            Probe::new,
            |mut probe| black_box(probe.compile_chain(4, Helper::Double)),
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, frame_cost, throw_cost, compile_cost);
criterion_main!(benches);
