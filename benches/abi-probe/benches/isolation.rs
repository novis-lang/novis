//! The price of an isolation boundary: a child process versus a task.
//!
//! [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md) gives MWL a
//! language construct for running another `.mwl` file with its own heap, globals
//! and limits inside the same process. PHP can only express that by spawning
//! another interpreter, so the decision rests on the gap between the two numbers
//! measured here.
//!
//! * `os_process/noop` — the floor of the mechanism being replaced: the cheapest
//!   do-nothing process the platform can start, waited to completion. A real
//!   child pays this *plus* an interpreter boot plus a pipe round trip.
//! * `task/create_and_finish` — the floor of the mechanism replacing it. An
//!   isolate is a task with an arena and a fresh set of globals; this is the task
//!   part, and it is deliberately the same measurement as in `coroutine.rs` so
//!   that the ratio comes from one bench run rather than from two.
//!
//! The arena and globals are not built yet (M5/M6). When they are, the isolate's
//! own end-to-end figure belongs here next to the baseline it beats.

// `criterion_group!` expands to an undocumented public function.
#![allow(missing_docs)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use mwl_abi_probe::{Ctx, in_coroutine, process};

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

    group.finish();
}

criterion_group!(benches, isolation_boundary);
criterion_main!(benches);
