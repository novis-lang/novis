//! The price of an isolation boundary: a child process versus a task.
//!
//! [ADR 0006](/docs/decisions/0006.md) gives Novis a
//! language construct for running another `.nvs` file with its own heap, globals
//! and limits inside the same process. PHP can only express that by spawning
//! another interpreter, so the decision rests on the gap between the numbers
//! measured here.
//!
//! * `os_process/noop` — the floor of the mechanism being replaced: the cheapest
//!   do-nothing process the platform can start, waited to completion. A real
//!   child pays this *plus* an interpreter boot plus a pipe round trip.
//! * `task/create_and_finish` — the floor of the mechanism replacing it. An
//!   isolate is a task with an arena and a fresh set of globals; this is the task
//!   part, and it is deliberately the same measurement as in `coroutine.rs` so
//!   that the ratio comes from one bench run rather than from two.
//! * `isolate/spawn_to_result` — the thing itself: `nvs_host::Isolate::run`,
//!   which copies the argument across the heap boundary, builds the child's own
//!   ownership root and context, runs it as a child task, copies the answer back
//!   and releases the root. It is the
//!   figure [M5's acceptance](/docs/plan/m5.md) asks for, and the gap
//!   between it and `task/create_and_finish` is what the boundary itself costs
//!   over the coroutine underneath it.
//! * `isolate/worker_fan_out` and `isolate/fan_out_on_one_core` — the other
//!   figure that acceptance asks for, which is a ratio rather than a cost: the
//!   same CPU-bound children placed `on: "worker"` and run one after another
//!   where they stand. Near-linear means the first is close to the second
//!   divided by the number of children, and the shared module's own doc owns why
//!   both halves are measured here rather than one of them being quoted.
//!
//! The child here is a Rust closure rather than a compiled unit, deliberately: a
//! compiled one would measure `nvs-cli`'s unit cache instead, and the cache is
//! warm by construction in the shape ADR 0006 sells — one path spawned many
//! times. What is left is the boundary, plus the one `Box` a resolver allocates
//! per spawn, which a real `spawn script` pays too.

// `criterion_group!` expands to an undocumented public function.
#![allow(missing_docs)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use nvs_abi_probe::{Ctx, in_coroutine, process};

// The same source the guard in `tests/perf_guards.rs` compiles, so the bench and
// the guard cannot drift apart; that file's own doc owns why it lives outside
// `src/`.
#[path = "../shared/isolate.rs"]
mod isolate;

fn isolation_boundary(c: &mut Criterion) {
    let mut group = c.benchmark_group("isolation");

    // A process spawn is milliseconds, so criterion's defaults would spend
    // minutes here for no extra precision.
    group
        .sample_size(20)
        .measurement_time(Duration::from_secs(10));
    group.bench_function("os_process/noop", |b| {
        b.iter(process::spawn_noop);
    });

    group.bench_function("task/create_and_finish", |b| {
        b.iter(|| {
            let run = in_coroutine(Ctx::new(), |_ctx| black_box(7i64));
            black_box(run.value)
        });
    });

    // `iter_custom` rather than `iter`: the round trip has to happen inside a
    // task or it takes the boundary's no-scheduler path instead, and the shared
    // module's doc owns that argument. Criterion asks for a batch and is told
    // what the batch cost.
    group.bench_function("isolate/spawn_to_result", |b| {
        b.iter_custom(isolate::spawn_to_result_batch);
    });

    // `iter_custom` again, and for a second reason on top of that one: a
    // placement is refused off a core, so a fan-out timed by criterion's own
    // loop would run every child on this thread and report a speedup of one.
    // The two rows are one measurement — the claim is the ratio between them.
    group.bench_function("isolate/worker_fan_out", |b| {
        b.iter_custom(isolate::worker_fan_out_batch);
    });
    group.bench_function("isolate/fan_out_on_one_core", |b| {
        b.iter_custom(isolate::one_core_batch);
    });

    group.finish();
}

criterion_group!(benches, isolation_boundary);
criterion_main!(benches);
