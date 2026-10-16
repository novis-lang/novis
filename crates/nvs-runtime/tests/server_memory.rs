//! What every core holds, as admission reads it, and what happens when the
//! process passes the server's memory limit after a request was admitted.
//!
//! `nvs_runtime::budget::held_by_cores` is the sum of one gauge per core, each
//! published by its own thread once its balance has moved a whole step. These
//! cases ask the cadence, the sum, and the stop a crossing raises.
//!
//! The gauges and the limit belong to the process, so the cases here run one
//! at a time under [`SERIAL`], each on threads of its own, and this file is a
//! binary of its own so no other suite's threads publish into the same sum.

use std::sync::{Arc, Barrier, Mutex, mpsc};

use nvs_config::Snapshot;
use nvs_runtime::{Ctx, Fault, OutputSink, SafepointFlags, budget};

/// One case at a time: every case reads the process's sum.
static SERIAL: Mutex<()> = Mutex::new(());

const MIB: usize = 1 << 20;

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A context reading `written` as its configuration, output discarded.
fn ctx_reading(written: &str) -> Ctx {
    let table: toml::Table = written.parse().expect("the case writes valid TOML");
    let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
    ctx.set_config(Arc::new(Snapshot {
        config: table
            .clone()
            .try_into()
            .expect("the case writes a block this tree has"),
        table,
        ..Snapshot::default()
    }));
    ctx
}

fn step() -> isize {
    budget::GAUGE_STEP
}

fn bytes(count: usize) -> isize {
    isize::try_from(count).expect("a case's size fits")
}

/// A core publishes once its balance has moved a whole step, and not per
/// allocation. A request's start and end publish whatever the move.
#[test]
fn a_core_publishes_only_once_its_balance_has_moved_a_step() {
    let _serial = serial();
    std::thread::spawn(|| {
        let _gauge = budget::CoreGauge::claim();
        let start = budget::held_by_cores();

        let small = vec![0_u8; 64 * 1024];
        assert_eq!(
            budget::held_by_cores(),
            start,
            "an allocation under one step was published, so the allocation path writes the gauge every time",
        );

        let large = vec![0_u8; 2 * MIB];
        assert!(
            budget::held_by_cores() >= start + bytes(2 * MIB),
            "an allocation of two steps was not published",
        );
        drop(large);
        assert!(
            budget::held_by_cores() < start + step(),
            "a release of two steps was not published",
        );

        budget::publish_held();
        assert!(
            budget::held_by_cores() >= start + bytes(64 * 1024),
            "a publish at a request's start or end left a move under one step out",
        );
        drop(small);
    })
    .join()
    .expect("the core thread panicked");
}

/// The sum is every core's gauge, and a core that ends gives its gauge back.
/// A thread that is not a core publishes nothing.
#[test]
fn the_sum_counts_every_core_and_no_other_thread() {
    let _serial = serial();
    let base = budget::held_by_cores();
    let holding = Arc::new(Barrier::new(4));
    let read = Arc::new(Barrier::new(4));
    let mut threads = Vec::new();
    for core in [true, true, false] {
        let holding = Arc::clone(&holding);
        let read = Arc::clone(&read);
        threads.push(std::thread::spawn(move || {
            let _gauge = core.then(budget::CoreGauge::claim);
            let held = vec![0_u8; 3 * MIB];
            holding.wait();
            read.wait();
            drop(held);
        }));
    }
    holding.wait();
    let summed = budget::held_by_cores();
    read.wait();
    for thread in threads {
        thread.join().expect("a core thread panicked");
    }
    // Above five steps rather than at six: a core's balance is its whole
    // thread's, and a thread that frees the closure it was started with is a
    // few bytes below zero before it allocates anything.
    assert!(
        summed > base + bytes(5 * MIB),
        "two cores holding three steps each read as {} over the base",
        summed - base,
    );
    assert!(
        summed < base + bytes(9 * MIB),
        "a thread that is not a core was counted: {} over the base",
        summed - base,
    );
    assert!(
        budget::held_by_cores() < base + step(),
        "a core that ended left its gauge in the sum",
    );
}

/// The server's limit, crossed after admission, stops the request that grew
/// with its own message. A request on another core keeps running, and so does
/// work with no request armed.
#[test]
fn a_crossing_stops_the_request_that_grew_and_no_other() {
    let _serial = serial();
    let (ready_tx, ready_rx) = mpsc::channel::<()>();
    let (crossed_tx, crossed_rx) = mpsc::channel::<()>();
    let (checked_tx, checked_rx) = mpsc::channel::<(bool, bool)>();

    // A capped request on another core, idle while the first one grows.
    let other = std::thread::spawn(move || {
        let _gauge = budget::CoreGauge::claim();
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        ctx.set_memory_limit(1 << 30);
        ready_tx.send(()).expect("the case is listening");
        crossed_rx.recv().expect("the growing request crossed");
        let small = vec![0_u8; 64 * 1024];
        let flagged = ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT);
        let breached = ctx.memory_breach().is_some();
        drop(small);
        checked_tx
            .send((flagged, breached))
            .expect("the case is listening");
    });
    ready_rx.recv().expect("the other core is ready");

    budget::set_process_limit(Some(
        u64::try_from(budget::held_by_cores()).unwrap_or(0) + 4 * MIB as u64,
    ));

    let growing = std::thread::spawn(|| {
        let _gauge = budget::CoreGauge::claim();
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        ctx.set_memory_limit(1 << 30);
        let under = vec![0_u8; 2 * MIB];
        assert!(
            !ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT)
                && ctx.memory_breach().is_none(),
            "a request under the server's limit was stopped",
        );
        let over = vec![0_u8; 8 * MIB];
        assert!(
            ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
            "a growth past the server's limit raised no poll",
        );
        let Some(Fault::Fatal(message)) = ctx.memory_breach() else {
            panic!("a growth past the server's limit was not a breach");
        };
        assert!(
            message.starts_with(
                "the server's memory is full, so the request was stopped while it held "
            ),
            "the stop did not say the server's memory is full: {message}",
        );
        drop((under, over));
    });
    growing
        .join()
        .expect("the growing request's thread panicked");
    crossed_tx.send(()).expect("the other core is listening");
    let (flagged, breached) = checked_rx.recv().expect("the other core checked");
    other.join().expect("the other core panicked");
    assert!(
        !flagged && !breached,
        "a request on another core was stopped by a crossing it did not make",
    );

    // An uncapped request is stopped too: its own ceiling is not the limit.
    std::thread::spawn(|| {
        let _gauge = budget::CoreGauge::claim();
        let ctx = ctx_reading("[limits]\nmemory = false\n");
        let over = vec![0_u8; 8 * MIB];
        let Some(Fault::Fatal(message)) = ctx.memory_breach() else {
            panic!("an uncapped request past the server's limit was not stopped");
        };
        assert!(
            message.starts_with("the server's memory is full"),
            "{message}"
        );
        drop(over);
    })
    .join()
    .expect("the uncapped request's thread panicked");

    // Work with no request armed is never stopped.
    std::thread::spawn(|| {
        let _gauge = budget::CoreGauge::claim();
        let ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        let over = vec![0_u8; 8 * MIB];
        assert!(
            !ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT)
                && ctx.memory_breach().is_none(),
            "work with no request armed was stopped by the server's limit",
        );
        drop(over);
    })
    .join()
    .expect("the engine thread panicked");

    budget::set_process_limit(None);
    assert_eq!(budget::process_limit(), None);
}
