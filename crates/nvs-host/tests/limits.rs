//! What a resource limit does to a request in flight — [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
//! § 1's ladder, asked at the safepoint poll that is the only place a program
//! allocating without calling anything can be stopped.
//!
//! **Why this crate.** The poll and the ladder are `nvs-runtime`'s, but the
//! thing being pinned is what happens to a *task the host is running*: a
//! runaway is stopped as a `FATAL` rather than as something a `catch` can
//! swallow, and the last thing it gets to do is the handler it registered.
//! `nvs-host` is the crate that owns a request's life, and it is the shallowest
//! place both halves are reachable from without a compiler in front of them.
//!
//! A closure here is built by hand rather than compiled: `call_closure` reads
//! exactly three things off a closure value, so a `ClassTable` and a plain
//! `extern "C"` function are a whole one. `nvs-stdlib`'s `allocation_policy.rs`
//! owns that shape and the reason the table is leaked.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use nvs_runtime::{Ctx, OutputSink, Value, nvs_safepoint};

/// How many times the handler below has been entered, across every test in
/// this binary. Each test asserts on a *difference* it reads for itself, so
/// nothing here depends on the order cargo runs them in or on their not
/// sharing a thread.
static ENTERED: AtomicUsize = AtomicUsize::new(0);

/// A registered `Core\Fatal::onLimit` handler, as the callee side of
/// `call_closure`'s contract: it counts its own entry, sweeps the one
/// reference it was handed — the receiver, since it declares no parameters —
/// and answers nothing.
#[expect(
    unsafe_code,
    reason = "`call_closure` passes one live value this callee owes a release, \
              and the address of a live `Value` for the result; neither is \
              expressible in the signature compiled code is called through"
)]
unsafe extern "C" fn counts_its_entry(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
    ENTERED.fetch_add(1, Ordering::SeqCst);
    unsafe {
        (*args).release();
        *out = Value::null();
    }
    nvs_runtime::OK
}

/// What the handler below read off the context it was entered with: the
/// ceiling in force, and whether the request was still over it. `usize::MAX`
/// and `true` are the readings of a handler that has not run.
static SEEN_CEILING: AtomicUsize = AtomicUsize::new(usize::MAX);
static SEEN_OVER: AtomicBool = AtomicBool::new(true);

/// A handler that reports the budget it was given, which is the only thing a
/// reserved slice is observable as from inside one.
#[expect(
    unsafe_code,
    reason = "the same callee contract as `counts_its_entry`, plus the context \
              pointer compiled code is called with — live for this call by the \
              ABI it arrives under"
)]
unsafe extern "C" fn reports_its_budget(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
    let ctx = unsafe { &*ctx };
    SEEN_CEILING.store(ctx.memory_limit(), Ordering::SeqCst);
    SEEN_OVER.store(ctx.over_memory_limit(), Ordering::SeqCst);
    unsafe {
        (*args).release();
        *out = Value::null();
    }
    nvs_runtime::OK
}

/// A handler that records nothing at all, for the one test that reads the
/// emptied slot instead of a counter: a handler sharing `ENTERED` with the test
/// beside it would land between that test's two reads of it and turn its
/// difference into two.
#[expect(
    unsafe_code,
    reason = "the same callee contract as `counts_its_entry`: one live value to \
              release, and the address of a live `Value` for the result"
)]
unsafe extern "C" fn answers_nothing(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
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
    object.set_field(nvs_runtime::CLOSURE_PARAM_TAGS_SLOT, Value::int(0));
    Value::object(object)
}

/// A context with a ceiling this thread has already allocated past, and the
/// allocation keeping it there.
///
/// The `Vec` is returned rather than dropped because the breach is the point:
/// `Ctx::memory_used` is what this thread holds *now* against what it held when
/// the context was made (`crate::budget`'s counters, which are maintained in
/// every profile).
fn breached() -> (Ctx, Vec<u8>) {
    let mut ctx = Ctx::new(OutputSink::Sink);
    ctx.set_memory_limit(4096);
    let hog = vec![0_u8; 1 << 20];
    (ctx, hog)
}

/// ADR 0020 § 1: a request past its memory ceiling is stopped, and stopped as
/// a `FATAL`.
///
/// The status is the assertion, because the status *is* the catchability:
/// `THROWN` is what unwinds into a Novis `catch` and `FATAL` is what no `catch`
/// can name, so a poll answering `FATAL` is the whole of "never reaches an
/// ordinary `catch`" — there is no second mechanism suppressing a handler.
#[test]
fn a_memory_cap_terminates_a_runaway_script_as_a_fatal() {
    let (mut ctx, hog) = breached();

    #[expect(
        unsafe_code,
        reason = "the safepoint is the ABI compiled code calls, so it takes \
                  the context by pointer; this one is a live local"
    )]
    let status = unsafe { nvs_safepoint(&raw mut ctx) };

    assert_eq!(status, nvs_runtime::FATAL);
    assert_ne!(
        status,
        nvs_runtime::THROWN,
        "a resource limit is not a `Throwable`",
    );
    drop(hog);
}

/// ADR 0020 § 1 lists CPU time beside memory: a request the CPU-time flag has
/// been raised on is stopped at the next poll, stopped as a `FATAL`, and taken
/// through the same tier-1 ladder on the way out.
///
/// The ladder is asserted by the emptied slot rather than by a counter, because
/// the slot is the mechanism: `Ctx::run_limit_handler` takes the registration
/// out on the way in, so a slot that is empty afterwards is a handler that was
/// entered, and reading it needs no state shared with the tests running beside
/// this one.
#[test]
fn a_cpu_cap_terminates_a_runaway_script_as_a_fatal() {
    let mut ctx = Ctx::new(OutputSink::Sink);
    ctx.request_safepoint(nvs_runtime::SafepointFlags::CPU_LIMIT);
    ctx.set_limit_handler(closure_of(answers_nothing));

    #[expect(
        unsafe_code,
        reason = "as above: the safepoint's ABI takes a context pointer, and \
                  this one is a live local"
    )]
    let status = unsafe { nvs_safepoint(&raw mut ctx) };

    assert_eq!(status, nvs_runtime::FATAL);
    assert_ne!(
        status,
        nvs_runtime::THROWN,
        "a resource limit is not a `Throwable`, whichever limit it is",
    );
    assert!(
        !ctx.has_limit_handler(),
        "the CPU branch reaches § 1's tier 1 rather than returning above it",
    );
}

/// ADR 0020 § 1: the registered handler runs, it runs *before* the breach
/// becomes the status the request reports, and it runs **once** — a handler
/// that breaches again is abandoned rather than called a second time.
///
/// The second poll is the zero-retry half and it is asserted on a count rather
/// than on a flag: a ladder that cleared the slot *after* the call instead of
/// before it would still pass an "it ran" assertion, and would call the handler
/// once per poll for the rest of a request that cannot go below its ceiling
/// again.
#[test]
fn a_limit_fatal_reaches_on_limit_and_never_a_catch() {
    let (mut ctx, hog) = breached();
    let before = ENTERED.load(Ordering::SeqCst);
    // The registration is an owned reference, which the context then holds.
    ctx.set_limit_handler(closure_of(counts_its_entry));
    assert!(ctx.has_limit_handler());

    #[expect(
        unsafe_code,
        reason = "as above: the safepoint's ABI takes a context pointer, and \
                  this one is a live local"
    )]
    let (first, second) = unsafe { (nvs_safepoint(&raw mut ctx), nvs_safepoint(&raw mut ctx)) };

    assert_eq!(first, nvs_runtime::FATAL, "the breach is still a `FATAL`");
    assert_eq!(second, nvs_runtime::FATAL, "and it stays one");
    assert_eq!(
        ENTERED.load(Ordering::SeqCst) - before,
        1,
        "§ 1's zero retries: one call for the request, however many polls see \
         the breach",
    );
    assert!(
        !ctx.has_limit_handler(),
        "the slot is cleared on the way in, which is what makes the second \
         poll find nothing",
    );
    drop(hog);
}

/// ADR 0020 § 1: the handler runs out of a slice of the request's own budget
/// reserved for it and unavailable to ordinary execution.
///
/// Asserted from *inside* the handler, because that is the only place the
/// reserve is observable: from outside, a request that has breached its ceiling
/// and a request whose handler was given room look identical. The two readings
/// together are the claim — the ceiling in force is exactly the ordinary one
/// plus the reserve, and against it the request is no longer over — and the
/// third, taken after the call returns, is that the room went away again rather
/// than becoming a request that had a larger ceiling all along.
#[test]
fn a_fatal_handler_runs_inside_its_reserved_slice() {
    let (mut ctx, hog) = breached();
    let ordinary = ctx.memory_limit();
    ctx.set_fatal_reserve(4 << 20);
    ctx.set_limit_handler(closure_of(reports_its_budget));

    #[expect(
        unsafe_code,
        reason = "as above: the safepoint's ABI takes a context pointer, and \
                  this one is a live local"
    )]
    let status = unsafe { nvs_safepoint(&raw mut ctx) };

    assert_eq!(status, nvs_runtime::FATAL);
    assert_eq!(
        SEEN_CEILING.load(Ordering::SeqCst),
        ordinary + (4 << 20),
        "the handler is entered against the ordinary ceiling plus the reserve",
    );
    assert!(
        !SEEN_OVER.load(Ordering::SeqCst),
        "and therefore with room: a handler that cannot allocate cannot report",
    );
    assert_eq!(
        ctx.memory_limit(),
        ordinary,
        "the reserve is the handler's, not the request's",
    );
    assert!(
        ctx.over_memory_limit(),
        "which is still over its own ceiling"
    );
    drop(hog);
}
