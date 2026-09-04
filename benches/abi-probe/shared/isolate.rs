//! The in-process isolate that [ADR 0006](/docs/adr/0006-isolated-script-execution.md)
//! replaces a child process with, as one measurable operation.
//!
//! [`nvs_abi_probe::process`] is the other half of the same comparison and is
//! shaped the same way on purpose: one function doing the cheapest complete
//! instance of the thing, so that `benches/isolation.rs` and the guard in
//! `tests/perf_guards.rs` measure the same code rather than two spellings of
//! it. Everything about *what* is measured — why the child is a closure rather
//! than a compiled unit, and what the number does and does not include — is the
//! bench's own module doc.
//!
//! It sits here rather than in `src/` and is reached by `#[path]` from both,
//! because it needs `nvs-runtime` and `nvs-host` and this package keeps every
//! compiler crate in `[dev-dependencies]` (the manifest says why). `shared/` is
//! a directory cargo does not scan for targets, which is the whole reason it is
//! named that and not `benches/`.
//!
//! # Why a batch, and not one round trip a caller can time itself
//!
//! Because the boundary has **two** implementations and only one of them is the
//! one a program reaches. `Isolate::start` asks `Wake::current()` whether there
//! is a task beneath the call: under a scheduler the child gets a stack of its
//! own, and with nothing installed it runs inline on the caller's stack instead
//! (`nvs_host::isolate`). A round trip timed from `#[test]` or from criterion's
//! own loop takes the second path and reads about 80 ns on this box — a true
//! figure about a boundary nobody crosses that way. So the scheduler and the
//! task are built here, outside the clock, and the loop runs *inside* the task.

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;
use std::time::{Duration, Instant};

use nvs_host::{Isolate, Output, Scheduler};
use nvs_runtime::script::Program;
use nvs_runtime::{Ctx, OutputSink, TaskRoot, Value};

/// Round trips run before the clock starts, inside the same task.
///
/// What the first crossings pay for is whatever the parent's ownership root and
/// the pooled allocator's size classes allocate once — which a real request has
/// already paid before its first `spawn script`, and which is what "on a warm
/// cache" means in [M5's acceptance](/docs/plan/m5.md).
const WARM: u64 = 200;

/// `iters` spawn-to-result round trips inside one task, and how long they took.
///
/// The scheduler, the task and the parent context are built outside the clock;
/// what is timed is only the crossing, the child and the crossing back.
///
/// # Panics
///
/// If the task does not run to its end, or if a null argument does not cross —
/// either would mean the boundary is broken rather than that it is slow.
pub(crate) fn spawn_to_result_batch(iters: u64) -> Duration {
    let mut sched = Scheduler::new();
    let elapsed = Rc::new(Cell::new(Duration::ZERO));
    let out = Rc::clone(&elapsed);

    // One parent for the whole batch: a `spawn script` in a request spawns from
    // the context that request already has, and a fresh one per round trip
    // would price a request instead of a child.
    let parent = Ctx::new(OutputSink::Buffer(Vec::new()));
    sched.spawn(parent, TaskRoot::Request, move |ctx| {
        for _ in 0..WARM {
            black_box(spawn_to_result(ctx));
        }
        let start = Instant::now();
        for _ in 0..iters {
            black_box(spawn_to_result(ctx));
        }
        out.set(start.elapsed());
    });
    sched.run();

    let took = elapsed.get();
    assert!(took > Duration::ZERO, "the task did not run to its end");
    took
}

/// One round trip: cross a null argument into a trivial child, run it, and
/// cross its answer back.
///
/// Answers with the child's returned value, which is always the integer `7` —
/// an integer, so what crosses back owns no heap and the parent's ownership
/// root has nothing to release when it is dropped. That matters over a batch of
/// twenty thousand: a heap answer would make this a growth measurement.
fn spawn_to_result(parent: &mut Ctx) -> Value {
    // A fresh `Box` per round trip because a `Program` is `FnOnce` — and
    // because a real `spawn script` allocates exactly one here too, out of the
    // resolver's `program_over`.
    let program: Program = Box::new(|ctx, args| {
        // The argument is transferred, so it goes to the one place the isolate's
        // wholesale release will reach (`nvs_runtime::script::Program`).
        ctx.set_isolate_argument(args);
        Value::int(7)
    });
    let completion = Isolate::new(program, Value::null(), Output::Capture)
        .run(parent)
        .expect("a null argument crosses");
    completion.value
}
