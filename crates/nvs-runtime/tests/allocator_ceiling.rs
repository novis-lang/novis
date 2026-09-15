//! What the allocator is armed with — `rule:errors/on-limit`'s memory ceiling
//! carried into `nvs_runtime::budget` as an absolute threshold, so that a
//! growing allocation is measured where it happens rather than at the next
//! poll.
//!
//! The cases here ask the *arming*: which requests arm a threshold, what the
//! number is against the balance the request started from, and what a context
//! gives back to the one that made it. What the flag then stops is asked
//! where the stopping is — `crate::nvs_safepoint`'s own cases and the
//! conformance tree — because a `Ctx` on its own has no back edge to be
//! stopped at.
//!
//! Every case asserts both halves of the sentinel, capped and uncapped
//! together. `0` is not a small ceiling but the value `budget::add`
//! short-circuits on, so a tree that armed nothing at all would answer the
//! uncapped half correctly and mean nothing by it.

use std::sync::Arc;

use nvs_config::Snapshot;
use nvs_runtime::{Ctx, OutputSink, SafepointFlags, budget};

/// The snapshot `written` resolves to — both halves of it, because a
/// `[limits]` directive is read out of the table while the typed tree beside it
/// is a different reader's, and a case built on half of it would pin a shape
/// the boot path cannot produce. `tests/configured_limits.rs` holds the same
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

/// The threshold is what an allocation is measured against, and crossing it is
/// published where compiled code already looks.
///
/// Asserted on both sides of the ceiling from one request, because a flag
/// raised by *any* allocation would pass the crossing half on its own. The
/// second half is the agreement that matters: the bit only asks for a poll, so
/// the case asserts that the poll it wakes — `Ctx::memory_breach`, the reader
/// `nvs_safepoint` and `run_helper` share — finds the breach the allocator
/// found.
#[test]
fn a_growing_allocation_past_the_ceiling_sets_the_memory_bit() {
    let ctx = ctx_reading("[limits]\nmemory = \"8M\"\n");
    assert!(
        !ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "a request was born with the memory flag raised",
    );
    let under = vec![0_u8; 1 << 20];
    assert!(
        !ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "an allocation well inside the ceiling raised the memory flag",
    );
    let over = vec![0_u8; 16 << 20];
    assert!(
        ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "an allocation past the ceiling reached no poll and raised nothing, which is the loop this goal exists to stop",
    );
    assert!(
        matches!(ctx.memory_breach(), Some(nvs_runtime::Fault::Fatal(_))),
        "the allocator raised the flag over a breach the poll it wakes does not see",
    );
    drop(over);
    drop(under);
}

/// `Core\Config::set` moves the ceiling mid-request, and the threshold moves
/// with it — the whole reason the arming lives in the pass that computes the
/// ceiling rather than beside it.
///
/// Asserted in both directions from one request. A threshold armed once and
/// never refreshed would answer the raise correctly by doing nothing at all if
/// the case only ever widened, so the restore afterwards is what says the
/// mirror follows rather than drifts.
#[test]
fn a_ceiling_raised_mid_request_rearms_the_threshold() {
    let mut ctx = ctx_reading("[limits]\nmemory = \"16M\"\n");
    let narrow = budget::armed_ceiling();
    assert!(
        narrow > 0,
        "a request stating `[limits] memory` armed no threshold, so nothing below is a move",
    );
    assert!(
        ctx.config_mut()
            .expect("the case set a configuration")
            .set("memory", "64M"),
        "`memory` is the request-changeable directive `Core\\Config::set` exists for, and this case asserts nothing if the set was refused",
    );
    ctx.refresh_limits();
    let wide = budget::armed_ceiling();
    // Exactly the 48 MiB the directive moved by: both ceilings reserve the same
    // 1 MiB default slice, so the difference between the thresholds is the
    // difference between the ceilings and nothing else. Asserting the gap
    // rather than the value is what keeps this a case about the *move* — the
    // balance the request started from is the same in both readings.
    assert_eq!(
        wide - narrow,
        48 << 20,
        "the threshold did not follow the ceiling `Core\\Config::set` raised",
    );
    assert!(
        ctx.config_mut()
            .expect("the case set a configuration")
            .set("memory", "8M"),
        "the same directive refused a lower value",
    );
    ctx.refresh_limits();
    assert_eq!(
        budget::armed_ceiling() - narrow,
        -(8 << 20),
        "the threshold followed a ceiling upwards and not back down, which is a request holding a ceiling it has given up",
    );
}

/// A context that took the thread's arming gives it back, so the request that
/// spawned it is measured against its own ceiling again.
///
/// `rule:security/isolate-shares-nothing` gives a tree one budget to divide and
/// one word to be stopped by, and the thread's threshold is a single number, so
/// only the running context's can stand in it. An isolate's own is the *tree's*:
/// `Ctx::isolate` arms what remains of the root's ceiling, and because the
/// threshold is an absolute balance that is the number the root armed less
/// whatever the child cost to build — never wider, which is the direction
/// `rule:security/isolate-budget-is-the-trees` fails in. A child armed nothing
/// would be refused no allocation at all while a root parked in a join polls
/// none.
///
/// The narrower ceiling written over it inside the block is what makes the
/// restore at the end mean something: a child that displaced nothing would pass
/// that assertion by never having armed anything of its own.
#[test]
fn an_isolate_restores_the_threshold_its_parent_armed() {
    let request = ctx_reading("[limits]\nmemory = \"16M\"\n");
    let armed = budget::armed_ceiling();
    assert!(
        armed > 0,
        "the request stating `[limits] memory` armed no threshold, so nothing below is a restore",
    );
    {
        let mut isolate = request.isolate(OutputSink::Buffer(Vec::new()));
        // The tree's own threshold, to within what this child cost to build:
        // `remaining` is read off the parent after `Ctx::new` has allocated the
        // child and `Ctx::share_safepoint_with` has given the fresh tree state
        // back, so the two numbers differ by those bytes and by nothing else. A
        // child armed a ceiling of its own would sit a whole `[limits] memory`
        // away from this, which is the reading the band refuses.
        let inherited = budget::armed_ceiling();
        assert!(
            inherited.abs_diff(armed) < 4096,
            "an isolate was born under {inherited} rather than under the tree's own {armed}",
        );
        // The ceiling an isolate gets is written by whoever spawned it —
        // `Ctx::set_memory_limit`'s own doc names `nvs-host` as the caller
        // holding no configuration — and that is the arming the end of the
        // block has to undo.
        isolate.set_memory_limit(4 << 20);
        let inner = budget::armed_ceiling();
        assert!(
            inner > 0 && inner < armed,
            "an isolate given its own ceiling armed something other than it",
        );
    }
    assert_eq!(
        budget::armed_ceiling(),
        armed,
        "an isolate that ended left its own threshold armed, so the request that spawned it is measured against a ceiling belonging to nobody",
    );
}

/// The cost half of this goal's *what it spends*: an uncapped request leaves
/// the threshold at the sentinel, which is the one compare a growing allocation
/// pays before it goes back to allocating.
///
/// The capped half is asserted first and is not decoration. Every assertion
/// after it is that a threshold is *absent*, and all of them would hold in a
/// tree where nothing ever armed one — so the case pins the bound on both
/// sides, and the restoring in the middle is what says the absence is this
/// request's own rather than the previous one's leftovers.
#[test]
fn an_uncapped_request_arms_no_threshold_and_pays_one_compare() {
    {
        let capped = ctx_reading("[limits]\nmemory = \"64M\"\n");
        let armed = budget::armed_ceiling();
        assert!(
            armed > 0,
            "a request stating `[limits] memory` armed no threshold, so the uncapped half below asserts nothing",
        );
        // The three readings are taken together and nothing between them
        // allocates, so they describe one moment: the threshold is the balance
        // at which this request's own reading would pass the ceiling the polls
        // are held to, which is the whole claim that the allocator and
        // `memory_breach` are asking one question in two places.
        let live = budget::live_bytes();
        let used = isize::try_from(capped.memory_used()).expect("a request's usage fits");
        let ceiling = isize::try_from(capped.memory_limit()).expect("64 MiB fits");
        assert_eq!(
            armed,
            live - used + ceiling,
            "the armed threshold is not the ceiling `Ctx::memory_breach` reads, measured from the balance this request started at",
        );
    }
    assert_eq!(
        budget::armed_ceiling(),
        0,
        "a context that armed a threshold did not give back what it displaced as it dropped",
    );

    let ctx = ctx_reading("[limits]\ncpu_time = \"2s\"\n");
    assert_eq!(
        ctx.memory_limit(),
        0,
        "a configuration stating no `[limits] memory` produced a ceiling, so this is not the uncapped case",
    );
    assert_eq!(
        budget::armed_ceiling(),
        0,
        "an uncapped request armed a threshold, which is a compare and a counter read per allocation it should not pay",
    );

    // And nothing notices an allocation, because on this request there is
    // nothing to notice: the flag is the allocator's only way to say so, and
    // `memory_breach` is what the poll it would wake goes on to ask.
    let held = vec![0_u8; 8 << 20];
    assert!(
        !ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "an uncapped request's allocation raised the memory flag",
    );
    assert!(
        ctx.memory_breach().is_none(),
        "an uncapped request was reported over a ceiling it does not have",
    );
    drop(held);
}
