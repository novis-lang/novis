//! Whose bytes a thread's balance is counting — `nvs_runtime::budget::Detached`,
//! the accounting boundary a cross-request store holds around every path that
//! allocates or frees what it holds.
//!
//! The balance is per thread and is read as per request, which is exact only
//! while every byte on the thread belongs to the request running there. A store
//! that outlives a request breaks that in the direction an attacker wants:
//! memory allocated under one request and released under another credits the
//! releasing request with bytes it never held, and a ceiling armed against the
//! balance widens by exactly that much.
//!
//! The cases ask the bracket from both ends — that it moves the process's
//! balance and not the request's, that it closes on an unwind as well as on a
//! return, and that a release inside one buys no headroom against an armed
//! ceiling. What a *store* owes is asked where the store is: `Core\Cache` is
//! the first to take the bracket, and its own cases ask the same property of
//! it. These ask the mechanism, so that a store found later has a proven one to
//! reach for.

use std::panic::catch_unwind;
use std::sync::Arc;

use nvs_config::Snapshot;
use nvs_runtime::{Ctx, OutputSink, budget};

/// The snapshot `written` resolves to — both halves of it, because a
/// `[limits]` directive is read out of the table while the typed tree beside it
/// is a different reader's, and a case built on half of it would pin a shape
/// the boot path cannot produce. `tests/allocator_ceiling.rs` holds the same
/// helper for the same reason.
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

/// A megabyte, as the two types the counters are read in.
const ENTRY: usize = 1 << 20;

/// The same figure signed, for the balances.
fn wide(bytes: usize) -> isize {
    isize::try_from(bytes).expect("a case allocates megabytes, not exabytes")
}

/// Both halves of the boundary, from one allocation: the process's balance
/// moves and every reading the request is measured by does not.
///
/// The release is asserted as well as the allocation, and inside a bracket of
/// its own, because that pairing is the whole contract — a bracket that only
/// diverted allocations would leave the free lowering a balance its allocation
/// never raised, which is the crediting this exists to stop.
#[test]
fn a_detached_bracket_moves_the_process_counter_and_not_the_request_reading() {
    let ctx = Ctx::new(OutputSink::Sink);
    let used = ctx.memory_used();
    let peak = ctx.memory_peak();
    let live = budget::live_bytes();
    let held = budget::detached_bytes();

    let entry = {
        let _bracket = budget::Detached::begin();
        vec![0_u8; ENTRY]
    };

    assert!(
        budget::detached_bytes() >= held + wide(ENTRY),
        "a bracketed allocation reached no balance at all, so nothing holds these bytes to the process"
    );
    assert_eq!(
        budget::live_bytes(),
        live,
        "a bracketed allocation moved the balance every request's ceiling is armed against"
    );
    assert_eq!(
        ctx.memory_used(),
        used,
        "a store's bytes were charged to the request that happened to be running"
    );
    assert_eq!(
        ctx.memory_peak(),
        peak,
        "a store's bytes raised the request's high-water mark"
    );

    {
        let _bracket = budget::Detached::begin();
        drop(entry);
    }

    assert_eq!(
        budget::detached_bytes(),
        held,
        "the release was not taken off the balance its allocation went on"
    );
    assert_eq!(
        budget::live_bytes(),
        live,
        "the release lowered the balance its allocation never raised"
    );
}

/// The bracket is closed by `Drop`, so it is closed on both ways out.
///
/// The unwind half is the one a store cannot discharge by being careful: a
/// panic between a store's first allocation and its last would otherwise leave
/// the thread detached for the rest of the request, and every allocation after
/// it charged to nobody. Each half re-reads the baseline first, because the
/// unwind itself allocates the payload it carries.
#[test]
fn a_bracket_restores_on_unwind_as_well_as_on_return() {
    {
        let _bracket = budget::Detached::begin();
    }
    let base = budget::live_bytes();
    let after_return = vec![0_u8; ENTRY];
    assert!(
        budget::live_bytes() >= base + wide(ENTRY),
        "a bracket that returned left the thread detached, and the request is charged for nothing it allocates"
    );
    drop(after_return);

    // The panic prints to stderr on its way past; what the case asserts is that
    // the guard's `Drop` ran while the stack unwound through it.
    let outcome = catch_unwind(|| {
        let _bracket = budget::Detached::begin();
        panic!("a store panicking mid-write is what leaves a bracket to the unwind");
    });
    assert!(
        outcome.is_err(),
        "the panic this case unwinds through did not happen"
    );

    let base = budget::live_bytes();
    let after_unwind = vec![0_u8; ENTRY];
    assert!(
        budget::live_bytes() >= base + wide(ENTRY),
        "a bracket unwound through stayed open, and every later allocation is the process's"
    );
    drop(after_unwind);
}

/// The attack the boundary exists to stop: a request that releases what an
/// earlier one stored is credited nothing for it.
///
/// The ceiling is an absolute balance, so headroom is the gap between the two
/// and it is the gap the case asserts rather than either number. The store's
/// own balance is asserted to fall by the same release, which is what
/// separates "the free was not credited to the request" from "the free was not
/// counted at all".
#[test]
fn a_request_that_frees_inherited_memory_gains_no_ceiling() {
    // What an earlier request left in a store: allocated inside a bracket, so
    // the process holds it and no request's balance does.
    let inherited = {
        let _bracket = budget::Detached::begin();
        vec![0_u8; 4 * ENTRY]
    };
    let stored = budget::detached_bytes();

    let ctx = ctx_reading("[limits]\nmemory = \"8M\"\n");
    let ceiling = budget::armed_ceiling();
    assert_ne!(
        ceiling, 0,
        "the request armed no ceiling, so there is no headroom here to widen"
    );
    let headroom = ceiling - budget::live_bytes();
    let used = ctx.memory_used();

    {
        let _bracket = budget::Detached::begin();
        drop(inherited);
    }

    assert!(
        budget::detached_bytes() <= stored - wide(4 * ENTRY),
        "the eviction was not counted anywhere, so the process's balance grows with what it has already given back"
    );
    assert_eq!(
        budget::armed_ceiling(),
        ceiling,
        "an eviction moved the threshold the request is measured against"
    );
    assert_eq!(
        budget::armed_ceiling() - budget::live_bytes(),
        headroom,
        "evicting what an earlier request stored bought this request headroom, which is a ceiling any program can widen at will"
    );
    assert_eq!(
        ctx.memory_used(),
        used,
        "an eviction lowered the reading the request's own limit handler reports"
    );
}
