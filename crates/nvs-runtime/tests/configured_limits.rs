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
#[test]
fn cpu_time_is_read_as_nanoseconds() {
    assert_eq!(
        ctx_reading("[limits]\ncpu_time = \"2s\"\n").cpu_limit(),
        2_000_000_000,
    );
    assert_eq!(
        ctx_reading("[limits]\ncpu_time = \"250ms\"\n").cpu_limit(),
        250_000_000,
        "a suffixed value is the same measurement, not a different one",
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
