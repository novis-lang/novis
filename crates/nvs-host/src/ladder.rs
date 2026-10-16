//! `rule:errors/handler-script`'s **tier
//! 3** — the operator's own `.nvs`, run before the floor reports.
//!
//! # Why this is a module of `nvs-host` and not of `nvs-runtime`
//!
//! Tier 3 *is* an isolate: § 3 says "the exact mechanism `rule:security/isolate-shares-nothing` already
//! defines", so it is [`crate::isolate::Isolate`] with a different program and
//! nothing else. That type lives here, and `nvs-runtime` is below it, so the
//! spawn cannot live beside the record it reports. What does live beside the
//! record is the argument — [`nvs_runtime::floor::report_argument`] — because
//! turning a [`Record`] into a `Value` is a question about the record and not
//! about isolates. The split is the same one [`nvs_runtime::script`] makes for
//! a `spawn script`: the crate that *can* do a thing does it, and the crate
//! that owns the meaning keeps the meaning.
//!
//! # What "before reporting" means, and what the caller writes instead
//!
//! [`escalate`] answers `true` when the handler ran to completion, and the
//! caller then writes nothing: tier 4 is the floor *beneath* tier 3, not a
//! second line beside it. Every other outcome — no `[log] handler` configured,
//! a path the `script.spawn` capability does not cover, a handler that will not
//! compile, a handler that threw, was cancelled or ran out of its own reserve —
//! is
//! `false`, and the caller reports the same record through
//! [`nvs_runtime::floor::report`] as it would have with no handler at all. That
//! is § 3's **zero retries** in the only form it can take here: there is one
//! call, its failure is not inspected, and nothing is attempted twice.
//!
//! # Recursion is stopped by a thread-local, not by a flag on the context
//!
//! A handler that throws reaches whatever classified the failure that ran it,
//! which would run the handler again. The guard is a `Cell<bool>` on the
//! thread, and that is the right scope rather than a convenience: an isolate
//! runs on the core that spawned it (`rule:security/isolate-shares-nothing`), so "the ladder is already
//! running" and "this thread is inside a handler" are the same fact. Holding it
//! on [`Ctx`] instead would put a cold field on the struct whose layout
//! compiled code indexes, and would still have to be copied into the isolate's
//! context by hand.
//!
//! # What it spends
//!
//! One isolate for the length of the report, which is [`crate::isolate`]'s own
//! accounting, plus one `bool` per thread. Nothing is O(failures reported): the
//! handler's context is dropped as its task ends, exactly as any other child's
//! is.
//!
//! Its budget is **not** the failing request's: the spawn asks for
//! [`Isolate::charged_to_the_engine_reserve`], so the handler runs under § 3's
//! engine-owned allotment and a request already at its ceiling still has a
//! report. [`nvs_runtime::Ctx::handler_isolate`] is the one home of what that
//! reserve is and of what it changes; nothing about that exception is decided
//! here.

use std::cell::Cell;

use nvs_render::Record;
use nvs_runtime::Ctx;

use crate::isolate::{Isolate, Output};

thread_local! {
    /// Whether a tier-3 handler is running on this thread — the module doc's
    /// zero-retries guard.
    ///
    /// A `Cell<bool>`, `const`-initialized and carrying no destructor, for the
    /// reason [`nvs_runtime::script`]'s own thread-local is one.
    static RUNNING: Cell<bool> = const { Cell::new(false) };
}

/// Clears [`RUNNING`] however the handler ended, an unwind included.
struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        RUNNING.with(|running| running.set(false));
    }
}

/// Runs the `[log] handler` script for `record`, and says whether it reported.
///
/// `false` means the floor still owes the line — the module doc lists every way
/// that happens and why none of them is retried.
///
/// The handler's own output is [`Output::Inherit`]'s, so it joins whatever
/// stream the failing context was already writing to: the process's standard
/// output for a CLI run, and the child's buffer for an isolate, which then
/// crosses at that isolate's own await. A report that appeared somewhere other
/// than the failing program's stream would be a second channel, and § 6 has
/// exactly one.
pub fn escalate(ctx: &mut Ctx, record: &Record) -> bool {
    if RUNNING.with(Cell::get) {
        return false;
    }
    let Some(path) = ctx.config().and_then(|config| config.get("log.handler")) else {
        return false;
    };
    // `rule:security/capability-check-at-the-door`'s spawn door is inside this call, so a handler outside the
    // `script.spawn` grant is refused here exactly as an ordinary `spawn
    // script` target would be. The refusal is not re-worded and not reported:
    // the floor is about to write the failure that got us here, and a second
    // sentence about the configuration would bury it.
    let Ok(program) = nvs_runtime::script::resolve(ctx, &path) else {
        return false;
    };
    // The record, the copy that crosses into the isolate and the script itself,
    // all inside `rule:errors/on-limit`'s reserve. The request that got here is
    // past its ceiling, so the pre-check in front of each of this array's
    // strings would answer an empty one and the script would be handed a record
    // naming nothing — tier 1's own report is lent the same thing for the same
    // reason. See `nvs_runtime::budget::Reporting`.
    let _reserve = nvs_runtime::budget::Reporting::begin();
    let args = nvs_runtime::floor::report_argument(record);
    RUNNING.with(|running| running.set(true));
    let guard = Guard;
    let completion = Isolate::new(program, args, Output::Inherit)
        .charged_to_the_engine_reserve()
        .run(ctx);
    drop(guard);
    // An `Err` is the *argument* refusing to cross, which a report built by
    // `report_argument` cannot do — it is a keyed array of strings. It is
    // handled rather than asserted because the record's field roster is not
    // this module's to fix.
    matches!(completion, Ok(completion) if completion.ok)
}
