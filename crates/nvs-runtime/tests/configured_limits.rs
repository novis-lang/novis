//! What a `[limits]` directive is by the time a request reads it — [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
//! § 1's ceilings, resolved once by `Ctx::set_config` and read as bare integers
//! everywhere after it.
//!
//! The cases here ask only the *reader*: a directive written as a string with a
//! suffix becomes the number the enforcement path compares against. What each
//! ceiling then does to a request in flight is `nvs-host`'s `tests/limits.rs`,
//! which needs a task to stop and a safepoint to stop it at.

use std::sync::Arc;

use nvs_config::Snapshot;
use nvs_runtime::{Ctx, OutputSink};

/// The snapshot `written` resolves to — both halves of it, because the typed tree and the table a
/// directive is read out of are two readers over one file and a case built on half of it would be
/// pinning a shape the boot path cannot produce.
fn snapshot(written: &str) -> Arc<Snapshot> {
    let table: toml::Table = written.parse().expect("the case writes valid TOML");
    Arc::new(Snapshot {
        config: table
            .clone()
            .try_into()
            .expect("the case writes a block this tree has"),
        table,
        ..Snapshot::default()
    })
}

/// The context reading that configuration, output discarded.
fn ctx_reading(written: &str) -> Ctx {
    let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
    ctx.set_config(snapshot(written));
    ctx
}

/// ADR 0020 § 1 lists CPU time beside memory: `[limits] cpu_time` is a duration, and what the
/// request holds is nanoseconds — `Ctx::cpu_limit`'s field doc owns why it is cached rather than
/// re-derived, and what measures the time against it.
/// The reading is asserted as the two parts it is split into rather than as `cpu_limit` alone,
/// because the directive is not the ceiling ordinary execution gets: § 1's slice is carved out of it
/// exactly as the memory half is carved out of `[limits] memory`, and the case below is the one that
/// pins the split.
#[test]
fn cpu_time_is_read_as_nanoseconds() {
    let plain = ctx_reading("[limits]\ncpu_time = \"2s\"\n");
    assert_eq!(
        plain.cpu_limit() + plain.fatal_reserve_time(),
        2_000_000_000
    );
    let suffixed = ctx_reading("[limits]\ncpu_time = \"250ms\"\n");
    assert_eq!(
        suffixed.cpu_limit() + suffixed.fatal_reserve_time(),
        250_000_000,
        "a suffixed value is the same measurement, not a different one",
    );
}

/// ADR 0020 § 1 names `fatal_reserve_time` beside `fatal_reserve_memory`, and `Ctx`'s
/// `reserve_time_within` owns the two numbers this asserts: a 50 ms default, and a quarter of the
/// ceiling wherever a quarter is less.
///
/// Every reading is taken as the pair, because "carved out of" is the whole claim and a reserve
/// that was *added to* the ceiling would answer the same `fatal_reserve_time()` on its own.
#[test]
fn the_fatal_reserve_time_is_carved_out_of_the_cpu_ceiling() {
    let defaulted = ctx_reading("[limits]\ncpu_time = \"2s\"\n");
    assert_eq!(defaulted.fatal_reserve_time(), 50_000_000);
    assert_eq!(
        defaulted.cpu_limit(),
        1_950_000_000,
        "the slice comes out of the request's own ceiling, not out of the machine",
    );

    let asked = ctx_reading("[limits]\ncpu_time = \"2s\"\nfatal_reserve_time = \"300ms\"\n");
    assert_eq!(asked.fatal_reserve_time(), 300_000_000);
    assert_eq!(asked.cpu_limit(), 1_700_000_000);

    // The clamp on both sides of where it starts to bite: a quarter of 200ms is exactly the
    // default, and a quarter of anything shorter is less than it. A reserve that stopped one step
    // early would read plausibly against either half alone.
    assert_eq!(
        ctx_reading("[limits]\ncpu_time = \"200ms\"\n").fatal_reserve_time(),
        50_000_000,
        "the shortest ceiling the default still fits inside a quarter of",
    );
    assert_eq!(
        ctx_reading("[limits]\ncpu_time = \"100ms\"\n").fatal_reserve_time(),
        25_000_000,
        "shorter, so the quarter wins over the default",
    );

    // An asked-for reserve is clamped rather than refused, and a request under no cap has nothing
    // to carve however loudly its configuration asks.
    let greedy = ctx_reading("[limits]\ncpu_time = \"1s\"\nfatal_reserve_time = \"10s\"\n");
    assert_eq!(greedy.fatal_reserve_time(), 250_000_000);
    assert_eq!(greedy.cpu_limit(), 750_000_000);
    assert_eq!(
        ctx_reading("[limits]\nfatal_reserve_time = \"1s\"\n").fatal_reserve_time(),
        0,
        "no ceiling is no slice: there is no room for a handler to be given past",
    );
}

/// The three spellings of "no ceiling" answer the same `0`, because a request that may burn any
/// amount of CPU and one whose limit nothing states are the same request downstream. The malformed
/// case is ADR 0064 § 3's rule seen from here: the file was refused once already, at the boundary
/// that could name the line, so this is not a second place to refuse it.
#[test]
fn an_unstated_uncapped_or_malformed_cpu_time_is_no_ceiling() {
    assert_eq!(
        Ctx::new(OutputSink::Buffer(Vec::new())).cpu_limit(),
        0,
        "a request with no configuration at all",
    );
    assert_eq!(ctx_reading("[limits]\nmemory = \"128M\"\n").cpu_limit(), 0);
    assert_eq!(ctx_reading("[limits]\ncpu_time = false\n").cpu_limit(), 0);
    assert_eq!(
        ctx_reading("[limits]\ncpu_time = \"every other tuesday\"\n").cpu_limit(),
        0,
    );
}
