//! Cost of suspending and resuming a task with JIT frames live on its stack.
//!
//! These two numbers set the price of MWL's "no async colouring" promise: any
//! function may perform I/O and yield, because suspension is a stack switch
//! rather than a compiler transformation.
//!
//! Baseline on x86_64-pc-windows-msvc: ~25 ns per suspend/resume round trip
//! through 2 JIT frames. See `docs/adr/README.md`.

// `criterion_group!` expands to an undocumented public function.
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use mwl_abi_probe::{Ctx, Helper, Probe, Value, call, in_coroutine};

fn suspend_resume(c: &mut Criterion) {
    let mut probe = Probe::new();
    let mut group = c.benchmark_group("coroutine/suspend_resume");

    for depth in [1usize, 2, 8] {
        let chain = probe.compile_chain(depth, Helper::Suspend);

        group.bench_function(depth.to_string(), |b| {
            // One coroutine per batch, suspending `iters` times inside it, so
            // the measurement isolates the stack switch from construction.
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();
                let run = in_coroutine(Ctx::new(), move |ctx| {
                    for _ in 0..iters {
                        black_box(call(chain, ctx, Value::int(1)));
                    }
                });
                let elapsed = start.elapsed();
                debug_assert_eq!(run.suspends as u64, iters);
                elapsed
            });
        });
    }
    group.finish();
}

fn task_creation(c: &mut Criterion) {
    // Per-task setup cost, which bounds how many in-flight requests a process
    // can absorb. MWL targets tens of thousands, so this needs to stay small
    // and, more importantly, must not start allocating a full stack eagerly.
    c.bench_function("coroutine/create_and_finish", |b| {
        b.iter(|| {
            let run = in_coroutine(Ctx::new(), |_ctx| black_box(7i64));
            black_box(run.value)
        });
    });
}

criterion_group!(benches, suspend_resume, task_creation);
criterion_main!(benches);
