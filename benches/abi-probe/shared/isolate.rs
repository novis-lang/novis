//! The in-process isolate that [ADR 0006](/docs/decisions/0006.md)
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
//! own loop takes the second path and reads a small fraction of a real
//! crossing — a true figure about a boundary nobody crosses that way. So the
//! scheduler and the
//! task are built here, outside the clock, and the loop runs *inside* the task.
//!
//! # The fan-out, and why it is two batches rather than one number
//!
//! [`worker_fan_out_batch`] and [`one_core_batch`] are the same [`WIDTH`]
//! children of the same arithmetic, placed on worker cores and run one after
//! another where they stand. Neither figure means anything alone — what
//! [M5's acceptance](/docs/plan/m5.md) claims is *near-linear speedup*, which is
//! a ratio, and a ratio taken across two machines or two builds is not one. So
//! both halves are measured through one driver, one after the other, and the
//! bench rows sit side by side for the same reason `task/create_and_finish` sits
//! beside `os_process/noop`.
//!
//! What the ratio prices is cores, not the boundary: the crossing itself is
//! [`spawn_to_result_batch`]'s figure, and a child here is a closure the far
//! core runs as a task rather than an isolate rebuilt from parts
//! (`nvs_host::placed` is that, and it takes a method entry a bench has no
//! compiled unit to name). The placement is the real one — the parent's core is
//! handed back, the work runs as a task on another core, and the answer comes
//! home through a slot and a wake — so what is left out of the numerator is
//! also left out of the denominator.
//!
//! A placement that is **refused** is the trap this would otherwise fall into:
//! off a core, or with no reactor to issue a wake, `nvs_host::worker::post`
//! hands the closure back and a caller that simply ran it would time four
//! children on one core and call the result a fan-out. So the refusal panics
//! here instead, and the driver installs the reactor a placement parks on.

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;
use std::time::{Duration, Instant};

use nvs_host::worker::{self, Answer};
use nvs_host::{Isolate, Output, Reactor, Scheduler, reactor, run_until_idle};
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
/// root has nothing to release when it is dropped. That matters over a long
/// batch: a heap answer would make this a growth measurement.
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

/// How many children one fan-out places at once.
///
/// Four, because four cores is what [M5's acceptance](/docs/plan/m5.md) makes
/// its near-linear claim about. A machine with fewer CPUs than this has fewer
/// cores to place them on — `nvs_host::worker`'s bound is the CPU count — so the
/// guard in `tests/perf_guards.rs` asks `nvs_host::cpus` how many cores there
/// are, and then [`burn`] on plain threads whether they are free, before it
/// believes a ratio.
pub(crate) const WIDTH: usize = 4;

/// The dependent multiply chain one child of a fan-out runs.
///
/// Sized so that the arithmetic is large beside a placement's own cost, which is
/// what makes the ratio a statement about cores rather than about the transport;
/// `spawn_to_result_batch` is where the transport is the subject.
const ROUNDS: u64 = 1_000_000;

/// `iters` fan-outs of [`WIDTH`] CPU-bound children across worker cores, and how
/// long they took.
///
/// # Panics
///
/// If a core refuses the child or the child does not run to its end — either
/// would mean this is timing something other than a fan-out.
pub(crate) fn worker_fan_out_batch(iters: u64) -> Duration {
    fan_out_batch(iters, on_worker_cores)
}

/// [`worker_fan_out_batch`]'s denominator: the same [`WIDTH`] children of the
/// same work, run one after another on the core that asked for them.
///
/// # Panics
///
/// If the task does not run to its end.
pub(crate) fn one_core_batch(iters: u64) -> Duration {
    fan_out_batch(iters, on_this_core)
}

/// `iters` fan-outs inside one task, timing `fan_out` and nothing around it.
///
/// The scheduler, the reactor, the task and the worker cores themselves are all
/// outside the clock, so what is timed is the placement, the work and the join.
fn fan_out_batch(iters: u64, fan_out: fn() -> u64) -> Duration {
    let mut sched = Scheduler::new();
    let elapsed = Rc::new(Cell::new(Duration::ZERO));
    let out = Rc::clone(&elapsed);

    let parent = Ctx::new(OutputSink::Buffer(Vec::new()));
    sched.spawn(parent, TaskRoot::Request, move |_ctx| {
        // One fan-out before the clock starts, for the same reason `WARM` exists
        // above: the first placement is what starts a core and reserves a stack
        // on it, and a request that fans out twice has already paid for both.
        black_box(fan_out());
        let start = Instant::now();
        for _ in 0..iters {
            black_box(fan_out());
        }
        out.set(start.elapsed());
    });
    // A placed child answers through a wake from another thread, so the parent
    // parks on a reactor rather than on the scheduler alone — and `spawn` off a
    // core is exactly the refusal the module doc says would quietly turn this
    // measurement into its own denominator.
    let installed = reactor::install(Reactor::new().expect("a reactor starts"));
    run_until_idle(&mut sched).expect("the scheduler finishes");
    drop(installed);

    let took = elapsed.get();
    assert!(took > Duration::ZERO, "the task did not run to its end");
    took
}

/// Places [`WIDTH`] children on worker cores, then collects them.
///
/// Every child is posted before any is collected, which is the whole of the
/// fan-out: `nvs_host::worker`'s picker hands each post a core that is idle, so
/// a batch posted eagerly spreads and a batch posted one-and-joined would not.
fn on_worker_cores() -> u64 {
    let placed: Vec<_> = (0..WIDTH)
        .map(|child| {
            worker::post(move || burn(child as u64))
                .unwrap_or_else(|_| panic!("a core takes child {child}"))
        })
        .collect();
    placed
        .into_iter()
        .map(|child| match child.collect() {
            Answer::Value(answer) => answer,
            other => panic!("a child came back as {other:?} rather than with its answer"),
        })
        .fold(0, u64::wrapping_add)
}

/// The same [`WIDTH`] children, one after another on this core.
fn on_this_core() -> u64 {
    (0..WIDTH)
        .map(|child| burn(child as u64))
        .fold(0, u64::wrapping_add)
}

/// [`ROUNDS`] steps of arithmetic, each needing the one before it.
///
/// No allocation, no syscall and no line shared with another core, so the only
/// thing the two batches differ by is where the work ran. The chain is what
/// keeps a core busy for the whole of it: an independent loop would retire
/// several steps at once and price the machine's issue width instead.
///
/// Reachable from `tests/perf_guards.rs` because the guard runs these same
/// children on plain threads to find out what the machine it is on can give
/// before it believes a ratio. A child of another shape there would price a
/// different machine.
pub(crate) fn burn(seed: u64) -> u64 {
    let mut acc = seed | 1;
    for step in 0..ROUNDS {
        acc = acc
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(step | 1);
    }
    acc
}
