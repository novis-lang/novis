//! `rule:concurrency/after-response-outlives-the-connection`'s
//! after-response work, at the seam that runs it: the completion path of the
//! isolate a request is.
//!
//! **Why this crate.** The queue and the drain are `nvs-runtime`'s and the
//! connection is `nvs-server`'s, but what § 6 promises is about neither on its
//! own — it is about a *task* still running once the task that was waiting for
//! it has gone. `nvs-host` owns both halves of that: it is where the isolate's
//! completion path files an answer and where a task's death cancels what it
//! left running. A joiner that takes the answer and then **returns** is exactly
//! what a connection ending is, and nothing about the claim needs a socket to
//! be true.
//!
//! It could not be filed against `nvs-server` in any case: a registration is a
//! `callable`, the only way to build one with no compiler in front of it is a
//! leaked `ClassTable` carrying an `invoke` address, and that crate inherits the
//! workspace's `unsafe_code = "forbid"` (its `Cargo.toml` says so and says why).
//! `nvs-stdlib`'s `allocation_policy.rs` owns the closure shape and why the
//! table is leaked.

use std::sync::Mutex;

use nvs_host::{Isolate, Output, Program, Scheduler};
use nvs_runtime::{Ctx, OutputSink, TaskRoot, Value};

/// What happened, in the order it happened.
///
/// The order is the whole assertion: both entries appearing says the work ran,
/// and only their order says it ran *after* the answer was taken rather than as
/// part of producing it.
static ORDER: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

fn note(what: &'static str) {
    ORDER
        .lock()
        .expect("no test panics holding this")
        .push(what);
}

/// The registered closure, as the callee side of `call_closure`'s contract: it
/// records that it ran, sweeps the one reference it was handed — the receiver,
/// since it declares no parameters — and answers nothing.
#[expect(
    unsafe_code,
    reason = "`call_closure` passes one live value this callee owes a release, \
              and the address of a live `Value` for the result; neither is \
              expressible in the signature compiled code is called through"
)]
unsafe extern "C" fn records_that_it_ran(
    _ctx: *mut Ctx,
    args: *const Value,
    out: *mut Value,
) -> i32 {
    note("deferred");
    unsafe {
        (*args).release();
        *out = Value::null();
    }
    nvs_runtime::OK
}

/// A zero-parameter closure calling `invoke`, owned by the caller.
///
/// The table is leaked because a descriptor's address is its identity and it
/// must outlive every instance made from it; the process exiting is what
/// reclaims it.
fn closure_of(invoke: nvs_runtime::NvsFn) -> Value {
    let mut table = nvs_runtime::ClassTable::new();
    let id = table.define("{closure}", &["arity", "params"], &[]);
    table.set_methods(
        id,
        vec![nvs_runtime::MethodRow {
            name: nvs_runtime::CLOSURE_INVOKE.to_owned(),
            code: invoke as *const u8,
            // Read off the object's own slots below, never off this row.
            arity: 0,
            param_tags: 0,
            public: true,
            native: false,
        }],
    );
    let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
    #[expect(
        unsafe_code,
        reason = "the table is leaked, so the descriptor outlives every \
                  instance made from it — `NvsObj::new`'s whole obligation"
    )]
    let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
    object.set_field(nvs_runtime::CLOSURE_ARITY_SLOT, Value::int(0));
    // No parameters, so no nibble to fill: `call_closure` reads this word and
    // finds nothing to hold an argument to.
    object.set_field(nvs_runtime::CLOSURE_PARAM_TAGS_SLOT, Value::int(0));
    Value::object(object)
}

/// `rule:concurrency/after-response-outlives-the-connection`: the connection ends, the request tree does not.
///
/// The joiner here is a stand-in for `nvs_server::serve`'s connection and does
/// the two things one does — it takes the answer, and then it returns — with
/// the return being the load-bearing half. A task's death cancels whatever it
/// left running (`nvs_host::scheduler`), so a tree that were still this
/// joiner's child would be torn down by the very event § 6 says it must
/// outlive, and the registered closure would never record anything.
///
/// The ordering assertion is what separates this from "the work ran at all": a
/// drain placed before the answer was filed would leave both entries in the log
/// and put `deferred` first, which is a request that still holds its connection
/// while it does its after-response work.
#[test]
fn an_after_response_tree_outlives_its_connection() {
    let mut sched = Scheduler::new();
    sched.spawn(
        Ctx::new(OutputSink::Sink),
        TaskRoot::Request,
        |connection: &mut Ctx| {
            let program: Program = Box::new(|request: &mut Ctx, _args| {
                assert_eq!(
                    request.defer(closure_of(records_that_it_ran), 0),
                    Ok(()),
                    "an isolate's queue refused a registration, so it is still sealed"
                );
                request.write_output(b"answered").expect("a buffer");
                Value::null()
            });
            let running = Isolate::new(program, Value::null(), Output::Capture)
                .start(connection)
                .expect("the isolate was refused before it started");
            let completion = running.join(connection);
            assert!(completion.ok, "the request did not return ordinarily");
            assert_eq!(
                completion.output, b"answered",
                "the answer the joiner took is not the one the request produced"
            );
            note("answered");
            // The connection returns here, which is what retires it and cancels
            // the children it still has.
        },
    );
    sched.run();

    let order = ORDER.lock().expect("no test panics holding this").clone();
    assert_eq!(
        order,
        ["answered", "deferred"],
        "after-response work did not run after the answer was taken and the \
         connection had gone"
    );
}

/// What the completion path handed the seam, and the code beside it.
///
/// A static rather than a capture because the seam is a bare `fn` that captures
/// nothing — `Ctx::set_exit_drain` owns why it is one word and not a closure —
/// so a static is the only way a test reads what it was handed.
static DRAINED: Mutex<Vec<(Result<(), i32>, i64)>> = Mutex::new(Vec::new());

/// `nvs_stdlib::script::run_exit_hooks`'s stand-in: this crate may not depend on
/// `nvs-stdlib`, and the half guarded here is that the isolate's completion path
/// *reaches* the seam with a faithful ending. Turning that ending into a
/// `Core\Script\ExitReport` is the other half, and it is guarded by
/// `rule:observability/three-endings-fire-the-exit-queue`'s own cases.
fn records_the_ending(
    ctx: &mut Ctx,
    outcome: Result<(), i32>,
    _thrown: Option<&nvs_runtime::Thrown>,
) {
    DRAINED
        .lock()
        .expect("no test panics holding this")
        .push((outcome, ctx.exit_code()));
}

/// `rule:observability/three-endings-fire-the-exit-queue` on the served path: an
/// isolate that ended at an `exit` drains its queue, with the `exit`'s own
/// ending and the status it named.
///
/// The ending is the whole assertion. An `exit` leaves no pending message, and
/// `Ctx::exit_code` reads `0` both for `exit(0)` and for a script that never
/// called `exit` — so a classifier reading only the context cannot tell one
/// from a normal end, and before the status was recorded there nothing on this
/// path reached the queue at all: the registrations were released unrun at
/// teardown.
#[test]
fn an_exited_isolate_drains_its_exit_queue_as_an_exit() {
    let mut sched = Scheduler::new();
    sched.spawn(
        Ctx::new(OutputSink::Sink),
        TaskRoot::Request,
        |connection: &mut Ctx| {
            let program: Program = Box::new(|request: &mut Ctx, _args| {
                // The two halves the seam is made of, as the production pair
                // does them: `Core\Script::onExit` fills the pointer on its
                // first registration, and `nvs-cli`'s `program_over` records
                // the status its frame came back with.
                request.set_exit_drain(records_the_ending);
                request.set_exit_code(3);
                request.set_ending(nvs_runtime::EXITED);
                Value::null()
            });
            let running = Isolate::new(program, Value::null(), Output::Capture)
                .start(connection)
                .expect("the isolate was refused before it started");
            let completion = running.join(connection);
            assert!(completion.ok, "an `exit` is not a failure");
        },
    );
    sched.run();

    let drained = DRAINED.lock().expect("no test panics holding this").clone();
    assert_eq!(
        drained,
        [(Err(nvs_runtime::EXITED), 3)],
        "the served path either never reached the exit queue, or reached it \
         with an ending that is not the `exit`"
    );
}

/// Raises the marker `Core\Script::finish()` raises, from a [`Program`] written
/// in Rust rather than compiled.
///
/// The lowering seals the calling block with a `Terminator::Throw` of an
/// instance of this class (`nvs_ir::lower::exception`), and what reaches the
/// completion path is the pending object that throw left — so a raise here is a
/// faithful stand-in for the whole compiled half, and needs neither a front end
/// nor a JIT in this crate.
///
/// The table is the narrowest one that makes the raise legal: `Throwable` for
/// the class `Ctx::set_runtime_error_class` installs, and the marker beside it
/// with no parent and no slots, which is what `nvs_hir::errors::TREE` declares
/// it as. Leaked for [`closure_of`]'s reason.
fn raise_the_finish_marker(ctx: &mut Ctx) {
    const THROWABLE: [&str; 4] = ["message", "previous", "backtrace", "location"];
    let mut table = nvs_runtime::ClassTable::new();
    let root = table.define("Throwable", &THROWABLE, &[]);
    table.define(nvs_runtime::FINISH_MARKER_NAME, &[] as &[&str], &[]);
    ctx.set_runtime_error_class(nvs_runtime::ErrorClass::new(
        std::sync::Arc::new(table),
        root,
    ));
    let desc = ctx
        .class_desc(nvs_runtime::FINISH_MARKER_NAME)
        .expect("the table installed a line above declares it");
    #[expect(
        unsafe_code,
        reason = "the descriptor comes out of the table this context now holds, \
                  so it outlives the object, whose one reference is handed to \
                  the `Thrown`"
    )]
    let marker =
        unsafe { nvs_runtime::Thrown::from_raw(nvs_runtime::NvsObj::new(desc).into_raw()) };
    ctx.raise(marker);
}

/// Whether the registered closure ran, for the two tests below that ask
/// opposite questions of it.
///
/// One counter rather than [`ORDER`]'s log: these cases are about the drain
/// happening at all, and a separate static is what lets them run beside the
/// ordering case above in the same binary.
static AFTER_A_FINISH: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// [`records_that_it_ran`]'s twin, writing to [`AFTER_A_FINISH`].
#[expect(
    unsafe_code,
    reason = "`call_closure` passes one live value this callee owes a release, \
              and the address of a live `Value` for the result; neither is \
              expressible in the signature compiled code is called through"
)]
unsafe extern "C" fn records_that_it_ran_after(
    _ctx: *mut Ctx,
    args: *const Value,
    out: *mut Value,
) -> i32 {
    AFTER_A_FINISH
        .lock()
        .expect("no test panics holding this")
        .push("deferred");
    unsafe {
        (*args).release();
        *out = Value::null();
    }
    nvs_runtime::OK
}

/// `rule:concurrency/after-response-outlives-the-connection` for the fourth
/// ending: a request that called `Core\Script::finish()` ended *ordinarily*, so
/// its after-response work runs like any other finished request's.
///
/// This is the claim the whole ending rests on. The marker travels the throw
/// path — that is what runs every `finally` between the call and the root — and
/// a completion path reading "something is pending" as "this request failed"
/// would leave the registration released unrun, which is the migration trap the
/// member exists to close.
#[test]
fn a_finished_request_runs_its_after_response_work() {
    let mut sched = Scheduler::new();
    sched.spawn(
        Ctx::new(OutputSink::Sink),
        TaskRoot::Request,
        |connection: &mut Ctx| {
            let program: Program = Box::new(|request: &mut Ctx, _args| {
                assert_eq!(
                    request.defer(closure_of(records_that_it_ran_after), 0),
                    Ok(()),
                    "an isolate's queue refused a registration, so it is still sealed"
                );
                raise_the_finish_marker(request);
                Value::null()
            });
            let running = Isolate::new(program, Value::null(), Output::Capture)
                .start(connection)
                .expect("the isolate was refused before it started");
            let completion = running.join(connection);
            assert!(completion.ok, "a finish is an ordinary end, not a failure");
        },
    );
    sched.run();

    let ran = AFTER_A_FINISH
        .lock()
        .expect("no test panics holding this")
        .clone();
    assert_eq!(
        ran,
        ["deferred"],
        "a finished request's after-response work did not run"
    );
}

/// Whether the registered closure ran for the `exit` case, which is the control
/// the case above needs: its own static, because the two run concurrently.
static AFTER_AN_EXIT: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// [`records_that_it_ran`]'s twin, writing to [`AFTER_AN_EXIT`].
#[expect(
    unsafe_code,
    reason = "`call_closure` passes one live value this callee owes a release, \
              and the address of a live `Value` for the result; neither is \
              expressible in the signature compiled code is called through"
)]
unsafe extern "C" fn records_that_it_ran_after_an_exit(
    _ctx: *mut Ctx,
    args: *const Value,
    out: *mut Value,
) -> i32 {
    AFTER_AN_EXIT
        .lock()
        .expect("no test panics holding this")
        .push("deferred");
    unsafe {
        (*args).release();
        *out = Value::null();
    }
    nvs_runtime::OK
}

/// The bound's other side, and the reason the case above is not just "the gate
/// says yes more often now": `exit` still runs none of its after-response work.
///
/// `rule:concurrency/after-response-outlives-the-connection` names the three
/// endings that run none of it, and `exit` is the one that looks most like an
/// ordinary end from the completion path — it is not a failure, so the answer
/// crosses and `Completion::ok` is true. Only the ending the program recorded
/// tells the two apart, which is what the gate reads.
#[test]
fn an_exited_request_still_runs_no_after_response_work() {
    let mut sched = Scheduler::new();
    sched.spawn(
        Ctx::new(OutputSink::Sink),
        TaskRoot::Request,
        |connection: &mut Ctx| {
            let program: Program = Box::new(|request: &mut Ctx, _args| {
                assert_eq!(
                    request.defer(closure_of(records_that_it_ran_after_an_exit), 0),
                    Ok(()),
                    "an isolate's queue refused a registration, so it is still sealed"
                );
                request.set_ending(nvs_runtime::EXITED);
                Value::null()
            });
            let running = Isolate::new(program, Value::null(), Output::Capture)
                .start(connection)
                .expect("the isolate was refused before it started");
            let completion = running.join(connection);
            assert!(completion.ok, "an `exit` is not a failure");
        },
    );
    sched.run();

    assert!(
        AFTER_AN_EXIT
            .lock()
            .expect("no test panics holding this")
            .is_empty(),
        "an `exit` ran after-response work, which is the ending's whole \
         difference from a finish"
    );
}

/// `rule:security/response-body-is-one-typed-member`'s declaration survives the
/// fourth ending: the status a handler set before it finished is the status the
/// completion carries.
///
/// A request finishes *in order to* answer early — it has already declared what
/// its response means and is skipping the rest of the work. So this is not a
/// property the status field shares with the failure path, where the
/// declaration is carried for a reporter rather than for a peer: here it is the
/// answer, and the assertion beside it is that nothing overwrote it with a
/// failure's.
#[test]
fn a_finished_request_reports_the_status_its_handler_declared() {
    let mut sched = Scheduler::new();
    sched.spawn(
        Ctx::new(OutputSink::Sink),
        TaskRoot::Request,
        |connection: &mut Ctx| {
            let program: Program = Box::new(|request: &mut Ctx, _args| {
                request.declare_status(201);
                request.write_output(b"created").expect("a buffer");
                raise_the_finish_marker(request);
                Value::null()
            });
            let running = Isolate::new(program, Value::null(), Output::Capture)
                .start(connection)
                .expect("the isolate was refused before it started");
            let completion = running.join(connection);
            assert!(completion.ok, "a finish is an ordinary end, not a failure");
            assert_eq!(
                completion.status,
                Some(201),
                "the declaration did not stand"
            );
            assert_eq!(
                completion.output, b"created",
                "the bytes written before the finish are not the ones that crossed"
            );
        },
    );
    sched.run();
}

/// The peer-facing half of the same claim: nothing about a finish reaches the
/// answering side as a failure.
///
/// `Completion::error` is what `nvs-server` turns into a 500 — its own field doc
/// ties it to `ok` — so a finish that left either of them set would be answered
/// as an internal error however well the ending was classified everywhere else.
/// Asserted separately from the status because the two fail apart: a gate that
/// reads the ending but takes nothing off the context leaves the error behind.
#[test]
fn a_finished_request_is_not_answered_five_hundred() {
    let mut sched = Scheduler::new();
    sched.spawn(
        Ctx::new(OutputSink::Sink),
        TaskRoot::Request,
        |connection: &mut Ctx| {
            let program: Program = Box::new(|request: &mut Ctx, _args| {
                raise_the_finish_marker(request);
                Value::null()
            });
            let running = Isolate::new(program, Value::null(), Output::Capture)
                .start(connection)
                .expect("the isolate was refused before it started");
            let completion = running.join(connection);
            assert!(completion.ok, "a finish is an ordinary end, not a failure");
            assert_eq!(
                completion.error, None,
                "the marker crossed as a failure, which is a 500 to the peer"
            );
        },
    );
    sched.run();
}
