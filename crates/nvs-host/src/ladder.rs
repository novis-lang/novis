//! `rule:errors/handler-script`'s **tier
//! 3** — the operator's own `.nvs`, run before the floor reports.
//!
//! # Why this is a module of `nvs-host` and not of `nvs-runtime`
//!
//! Tier 3 *is* an isolate: that rule runs it as the same `spawn script`
//! isolate `rule:security/isolate-shares-nothing` defines, so it is
//! [`crate::isolate::Isolate`] with a different program and nothing else. That type lives here, and `nvs-runtime` is below it, so the
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
//! is `rule:errors/escalation-ladder`'s **zero retries** in the only form it can take here: there is one
//! call, its failure is not inspected, and nothing is attempted twice.
//!
//! # Server work does not wait for its handler
//!
//! A request, a connection, a scheduled run or a job that fails hands its
//! record to a handler on a root task of its own ([`spawn_root`]) and goes on:
//! its response is written and its in-flight place goes back while the
//! handler runs. Joining the handler there let a client who can make a program
//! throw hold a place, and a share of the core's CPU, for up to
//! `handler_reserve_time` per request. [`escalate`] answers `true` once the
//! handler is started, and the handler's task writes the floor's line itself
//! if the handler fails, so "the handler or the floor, never both" still
//! holds. One core runs at most [`HANDLERS_PER_CORE`] handlers at once. Past
//! that the run is dropped, counted in `nvs_log_handler_dropped_total`, and
//! [`escalate`] answers `false`, so the floor writes the line. A handler off
//! the request writes its output to the diagnostic stream, because no
//! response is waiting for it. A CLI program still joins its handler: the
//! process ends with the program, and a report started after it would be lost.
//!
//! # Recursion is stopped by a thread-local, not by a flag on the context
//!
//! A handler that throws reaches whatever classified the failure that ran it,
//! which would run the handler again. A joined handler is guarded by a
//! `Cell<bool>` on the thread, and that is the right scope rather than a
//! convenience: an isolate runs on the core that spawned it
//! (`rule:security/isolate-shares-nothing`), so "the ladder is already
//! running" and "this thread is inside a handler" are the same fact. A handler
//! on its own task does not hold the thread, so the guard is the list of those
//! tasks, and a failure in a task beneath one of them is not escalated.
//! Holding either on [`Ctx`] instead would put a cold field on the struct whose
//! layout compiled code indexes, and would still have to be copied into the
//! isolate's context by hand.
//!
//! # What it spends
//!
//! One isolate for the length of the report, which is [`crate::isolate`]'s own
//! accounting, plus one `bool` per thread. A handler off the request adds one
//! context for its root task and keeps the failing request's tree state alive
//! until it ends, at most [`HANDLERS_PER_CORE`] of each per core. Nothing is
//! O(failures reported): the handler's context is dropped as its task ends,
//! exactly as any other child's is.
//!
//! Its budget is **not** the failing request's: the spawn asks for
//! [`Isolate::charged_to_the_engine_reserve`], so the handler runs under
//! `rule:errors/handler-script`'s engine-owned allotment and a request already at its ceiling still has a
//! report. [`nvs_runtime::Ctx::handler_isolate`] is the one home of what that
//! reserve is and of what it changes; nothing about that exception is decided
//! here.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use nvs_render::Record;
use nvs_runtime::{Ctx, OutputSink, TaskRoot};

use crate::isolate::{Isolate, Output};
use crate::scheduler::{TaskId, running_beneath, spawn_root};

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
/// than the failing program's stream would be a second channel, and
/// `rule:errors/log-write` has exactly one.
pub fn escalate(ctx: &mut Ctx, record: &Record) -> bool {
    if RUNNING.with(Cell::get) || inside_a_handler() {
        return false;
    }
    let Some(path) = ctx.config().and_then(|config| config.get("log.handler")) else {
        return false;
    };
    if ctx.config().is_some_and(|config| config.serves()) {
        return off_the_request(ctx, &path, record);
    }
    // `rule:security/capability-check-at-the-door`'s spawn door is inside this call, so a handler outside the
    // `script.spawn` grant is refused here exactly as an ordinary `spawn
    // script` target would be. The refusal is not re-worded and not reported:
    // the floor is about to write the failure that got us here, and a second
    // sentence about the configuration would bury it.
    let Ok(program) = nvs_runtime::script::resolve(ctx, &path) else {
        return false;
    };
    // The record and the copy that crosses into the isolate are built inside
    // `rule:errors/on-limit`'s reserve. The request that got here is past its
    // ceiling, so the pre-check in front of each of this array's strings would
    // answer an empty one and the script would be handed a record naming
    // nothing — tier 1's own report is lent the same thing for the same reason.
    // See `nvs_runtime::budget::Reporting`.
    let reporting = nvs_runtime::budget::Reporting::begin();
    let args = nvs_runtime::floor::report_argument(record);
    RUNNING.with(|running| running.set(true));
    let guard = Guard;
    let started = Isolate::new(program, args, Output::Inherit)
        .charged_to_the_engine_reserve()
        .start(ctx);
    // The script itself is user code, and user code never runs inside the
    // reserve: it closes before the wait that lets the handler run, and the
    // handler is held to the engine's allotment alone.
    drop(reporting);
    let completion = started.map(|running| running.join(ctx));
    drop(guard);
    // An `Err` is the *argument* refusing to cross, which a report built by
    // `report_argument` cannot do — it is a keyed array of strings. It is
    // handled rather than asserted because the record's field roster is not
    // this module's to fix.
    matches!(completion, Ok(completion) if completion.ok)
}

/// How many handlers one core runs at once.
///
/// A handler writes a report, so its time is what writing to a log target or a
/// socket blocks on. Four at once covers a burst of different failures. Past
/// that, the failures are most likely one fault repeated, and the floor's line
/// for each is the report: the floor counts repeats of one record
/// (`rule:errors/handler-script`). The bound keeps a flood of forced failures
/// to at most this many handlers' CPU time on a core.
const HANDLERS_PER_CORE: usize = 4;

thread_local! {
    /// The root task of each handler running on this core, at most
    /// [`HANDLERS_PER_CORE`].
    static HANDLERS: RefCell<Vec<TaskId>> = const { RefCell::new(Vec::new()) };
}

/// Whether the running task is a handler [`off_the_request`] started, or a task
/// beneath one: the zero-retries guard for a handler that does not hold the
/// thread while it runs.
fn inside_a_handler() -> bool {
    HANDLERS.with_borrow(|handlers| handlers.iter().any(|&root| running_beneath(root)))
}

/// Starts the handler on a root task of its own and returns `true`, or returns
/// `false` when this core already runs [`HANDLERS_PER_CORE`] of them.
///
/// The failing request does not wait for the handler. Its response is written
/// and its place goes back while the handler runs. So the handler's own
/// outcome is checked on its task: if it fails, the floor writes the record
/// there. A handler that is not started is counted in
/// `nvs_log_handler_dropped_total`, and the caller's floor writes the record.
///
/// The handler's output goes to the diagnostic stream. The failing request's
/// response is not waiting for it.
fn off_the_request(ctx: &mut Ctx, path: &str, record: &Record) -> bool {
    if HANDLERS.with_borrow(Vec::len) >= HANDLERS_PER_CORE {
        nvs_runtime::metrics::count_handler_dropped();
        return false;
    }
    let Ok(program) = nvs_runtime::script::resolve(ctx, path) else {
        return false;
    };
    // See `escalate` for why the argument is built inside the reserve.
    let reporting = nvs_runtime::budget::Reporting::begin();
    let args = nvs_runtime::floor::report_argument(record);
    drop(reporting);
    let record = record.clone();
    let slot = Slot(Rc::new(Cell::new(None)));
    let held = Slot(Rc::clone(&slot.0));
    let started = spawn_root(
        ctx.handler_isolate(OutputSink::Stderr),
        TaskRoot::Request,
        move |root| {
            let completion = Isolate::new(program, args, Output::Inherit)
                .charged_to_the_engine_reserve()
                .start(root)
                .map(|running| running.join(root));
            drop(held);
            if !matches!(completion, Ok(completion) if completion.ok) {
                nvs_runtime::floor::report(root, &record);
            }
        },
    );
    let Some(id) = started else {
        return false;
    };
    slot.0.set(Some(id));
    HANDLERS.with_borrow_mut(|handlers| handlers.push(id));
    true
}

/// A handler's place in [`HANDLERS`], given back when the handler's task
/// drops it. The task owns one copy, so an unwind or a task that never ran
/// gives the place back too. The other copy only sets the id once the task
/// has one.
struct Slot(Rc<Cell<Option<TaskId>>>);

impl Drop for Slot {
    fn drop(&mut self) {
        if Rc::strong_count(&self.0) > 1 {
            return;
        }
        if let Some(id) = self.0.get() {
            HANDLERS.with_borrow_mut(|handlers| handlers.retain(|&handler| handler != id));
        }
    }
}
