//! [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md) § 6's
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

/// ADR 0072 § 6: the connection ends, the request tree does not.
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
