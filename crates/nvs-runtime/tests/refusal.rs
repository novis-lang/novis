//! What a refused allocation leaves behind — `rule:errors/on-limit`'s memory
//! ceiling asked *in front of* an allocation rather than after it, so that one
//! operation is bounded and not merely a loop of them.
//!
//! The cases here ask the **verdict**, not the arithmetic. A request that was
//! refused holds fewer bytes than its ceiling, because the bytes it asked for
//! were never handed over, so every reader that goes on counting calls it
//! comfortably inside one. What has to be true instead is that the poll
//! following a refusal reports the breach, and that the refusal belongs to the
//! request it stopped and to no other.
//!
//! The tier-1 handler's side of it — that a refused request can still allocate
//! the report `rule:errors/on-limit` promises, inside the slice reserved for it
//! — is asserted end to end by
//! `tests/conformance/error/one-operation-past-the-ceiling-is-refused-before-it-allocates.nvst`,
//! where there is a registered closure to run. `Ctx::run_limit_handler` runs
//! nothing without one, so a case here could only assert the accessors it is
//! built from.

use nvs_runtime::{Ctx, OutputSink, SafepointFlags, budget};

/// A request under `bytes` of ceiling, holding nothing of its own yet.
///
/// `Ctx::set_memory_limit` rather than a configuration, because nothing here
/// asks which directive the number came from — `tests/allocator_ceiling.rs`
/// holds the cases that do, and both arrive at the same armed threshold.
fn ctx_under(bytes: usize) -> Ctx {
    let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
    ctx.set_memory_limit(bytes);
    ctx
}

/// An ask larger than the whole remaining budget is refused in front of the
/// allocation, and the poll that follows reports it.
///
/// Both sides of the bound, in that order: an ask that fits is answered without
/// recording anything, which is what says the refusal below is the ask's and
/// not the ceiling's mere presence. The reading afterwards is the whole point
/// of the case — the request is *under* its ceiling and over its limit at the
/// same time, which is a breach only the verdict can carry.
#[test]
fn a_single_operation_past_the_ceiling_never_allocates_what_it_asked_for() {
    let ctx = ctx_under(16 << 20);
    assert!(
        budget::affords(1 << 10),
        "an ask a request has room for twenty times over was refused",
    );
    assert!(
        !ctx.over_memory_limit(),
        "a request that has been refused nothing is already over its ceiling, so nothing below is a refusal",
    );
    assert!(
        !ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "an ask that fits raised the memory flag",
    );

    let held = budget::live_bytes();
    let ask = 256 << 20;
    assert!(
        !budget::affords(ask),
        "an ask of sixteen times the whole ceiling was afforded, which is the single operation this stage exists to bound",
    );
    assert_eq!(
        budget::live_bytes(),
        held,
        "a refused ask moved the balance, so something was allocated on the way to refusing it",
    );
    assert!(
        ctx.memory_peak() < ask,
        "the mark holds the ask, which is what a request that allocated first and noticed afterwards leaves behind",
    );

    // The request is inside its ceiling by every measure that counts bytes, and
    // over it all the same. A reader that asked the counter alone would let the
    // program carry on with whatever degenerate value the refusal handed back.
    assert!(
        ctx.memory_used() < ctx.memory_limit(),
        "the refused request holds more than its ceiling, so the assertions below would hold for a runtime that allocated the ask and counted it",
    );
    assert!(
        ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "a refusal asked for no poll, so compiled code that allocates nothing else never reaches the reader below",
    );
    assert!(
        ctx.over_memory_limit(),
        "a refused request reads as inside its ceiling",
    );
    assert!(
        matches!(ctx.memory_breach(), Some(nvs_runtime::Fault::Fatal(_))),
        "the poll a refusal wakes does not report it, so `rule:errors/on-limit`'s `FATAL` never arrives",
    );
}

/// A refusal dies with the request it stopped.
///
/// The verdict is a thread-local, because the allocators that hit one hold no
/// `Ctx` — so without the displacement `Ctx::memory_refused_saved` names, the
/// next request a worker picked up would be stopped by a refusal it never
/// asked for, and a served core would answer one `FATAL` per request for ever.
#[test]
fn a_refusal_dies_with_the_request_it_stopped() {
    {
        let stopped = ctx_under(16 << 20);
        assert!(
            !budget::affords(256 << 20),
            "the ask was afforded, so nothing below is a refusal being confined",
        );
        assert!(
            stopped.over_memory_limit(),
            "the refusal was not recorded against the request that made it",
        );
    }
    let next = ctx_under(16 << 20);
    assert!(
        !next.over_memory_limit(),
        "a request inherited the refusal that stopped the one before it on this thread",
    );
    assert!(
        next.memory_breach().is_none(),
        "a request born clear was reported over a ceiling it has not been refused against",
    );
    assert!(
        budget::affords(1 << 10),
        "an inherited refusal is answering for asks this request has room for",
    );
}
