//! What a resource limit does to a request in flight — [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
//! § 1's ladder, asked at the safepoint poll that is the only place a program
//! allocating without calling anything can be stopped — and, for the one
//! ceiling that bounds a *tree* rather than a request, at the `spawn script`
//! that would pass it, because nothing is over `max_script_depth` until an
//! isolate is asked for.
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

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use nvs_config::Snapshot;
use nvs_config::capability::{Cap, Scope};
use nvs_runtime::{Ctx, NvsStr, OutputSink, Value, nvs_safepoint};

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

/// What the handler below read off the CPU half of § 1's slice: the ceiling in
/// force, and whether the flag that stopped the request was still raised.
/// `u64::MAX` and `true` are the readings of a handler that has not run.
static SEEN_CPU_CEILING: AtomicU64 = AtomicU64::new(u64::MAX);
static SEEN_CPU_FLAG: AtomicBool = AtomicBool::new(true);

/// A handler that reports the time slice it was given. The flag is read beside
/// the ceiling because on this half the two are one mechanism: a ceiling
/// nothing consults buys a handler nothing while the flag that stopped it is
/// still up.
#[expect(
    unsafe_code,
    reason = "the same callee contract as `reports_its_budget`, over the same \
              context pointer"
)]
unsafe extern "C" fn reports_its_time(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
    let ctx = unsafe { &*ctx };
    SEEN_CPU_CEILING.store(ctx.cpu_limit(), Ordering::SeqCst);
    SEEN_CPU_FLAG.store(
        ctx.safepoint_flags()
            .contains(nvs_runtime::SafepointFlags::CPU_LIMIT),
        Ordering::SeqCst,
    );
    unsafe {
        (*args).release();
        *out = Value::null();
    }
    nvs_runtime::OK
}

/// What the handler below read out of § 1's report, or `None` before it has
/// run.
///
/// **A slot per test, not one shared by all of them**, which is why the spawn
/// case further down has its own beside this one rather than reusing this
/// recorder: cargo runs these on threads of its own, and a slot that is read
/// with `take` empties it for whoever else was about to. The counter at the top
/// of this file is a *difference* for the same reason.
static SEEN_LIMIT: Mutex<Option<String>> = Mutex::new(None);

/// A handler that declares § 1's `LimitReport` parameter and records the limit
/// it names — the callee side of a report that has to arrive as an argument
/// rather than as a message the ladder prints afterwards.
#[expect(
    unsafe_code,
    reason = "the callee contract of `counts_its_entry`, over two live values \
              this time: the receiver and the report, both of which this owes a \
              release"
)]
unsafe extern "C" fn records_the_report(
    _ctx: *mut Ctx,
    args: *const Value,
    out: *mut Value,
) -> i32 {
    unsafe {
        let report = *args.add(1);
        let array = report
            .array_ptr()
            .expect("§ 1's report reaches the handler as one array");
        // Borrowed out of the array, which owns it for the length of this call.
        let key = NvsStr::new(b"limit").into_raw();
        let mut named = Value::null();
        nvs_runtime::nvs_array_get(array, key, &raw mut named);
        *SEEN_LIMIT.lock().expect("no test panics holding this") =
            named.as_text().map(str::to_owned);
        drop(NvsStr::from_raw(key));

        (*args).release();
        report.release();
        *out = Value::null();
    }
    nvs_runtime::OK
}

/// [`SEEN_LIMIT`]'s twin for the spawn-depth case, and its doc owns why there
/// are two.
static SEEN_SPAWN_LIMIT: Mutex<Option<String>> = Mutex::new(None);

/// [`records_the_report`] writing into [`SEEN_SPAWN_LIMIT`] instead.
///
/// A copy rather than a parameter because compiled code is called through a bare
/// `extern "C"` pointer, which captures nothing: "the same handler, aimed at
/// another slot" is not a thing this ABI can express.
#[expect(
    unsafe_code,
    reason = "the same callee contract as `records_the_report`, over the same \
              two live values"
)]
unsafe extern "C" fn records_the_spawn_report(
    _ctx: *mut Ctx,
    args: *const Value,
    out: *mut Value,
) -> i32 {
    unsafe {
        let report = *args.add(1);
        let array = report
            .array_ptr()
            .expect("§ 1's report reaches the handler as one array");
        let key = NvsStr::new(b"limit").into_raw();
        let mut named = Value::null();
        nvs_runtime::nvs_array_get(array, key, &raw mut named);
        *SEEN_SPAWN_LIMIT
            .lock()
            .expect("no test panics holding this") = named.as_text().map(str::to_owned);
        drop(NvsStr::from_raw(key));

        (*args).release();
        report.release();
        *out = Value::null();
    }
    nvs_runtime::OK
}

/// A zero-parameter closure calling `invoke`, owned by the caller.
fn closure_of(invoke: nvs_runtime::NvsFn) -> Value {
    closure_taking(invoke, 0)
}

/// A closure declaring one parameter of any representation, which is what a
/// handler written to § 1's `closure(LimitReport)` signature is: `callable`
/// carries no parameter list (ADR 0031 § 1), so the nibble a written `fn`
/// literal would record is the only thing `call_closure` checks against.
fn closure_taking_the_report(invoke: nvs_runtime::NvsFn) -> Value {
    closure_taking(invoke, 1)
}

/// A closure of `arity` parameters calling `invoke`, owned by the caller.
///
/// The table is leaked because a descriptor's address is its identity and it
/// must outlive every instance made from it; the process exiting is what
/// reclaims it.
fn closure_taking(invoke: nvs_runtime::NvsFn, arity: i64) -> Value {
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
    object.set_field(nvs_runtime::CLOSURE_ARITY_SLOT, Value::int(arity));
    // One nibble per parameter, and `ANY` in each: what this file's callees
    // read off their argument slots is the report's own tag, so the check the
    // nibbles exist for has nothing to add here.
    let mut tags: i64 = 0;
    for slot in 0..arity {
        tags |= i64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY) << (slot * 4);
    }
    object.set_field(nvs_runtime::CLOSURE_PARAM_TAGS_SLOT, Value::int(tags));
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

/// ADR 0020 § 1's `closure(LimitReport)`: the handler is handed a report, and
/// the report names the limit that stopped *this* request rather than the one
/// the ladder happens to be written around.
///
/// Both stops are asserted in one case because the claim is that they differ:
/// a ladder building the report at one shared point would answer `memory` to a
/// request stopped for CPU time and look right against either half alone.
#[test]
fn a_limit_handler_is_handed_a_report_naming_the_limit() {
    let seen = || {
        SEEN_LIMIT
            .lock()
            .expect("no test panics holding this")
            .take()
    };

    let (mut ctx, hog) = breached();
    ctx.set_limit_handler(closure_taking_the_report(records_the_report));
    #[expect(
        unsafe_code,
        reason = "as above: the safepoint's ABI takes a context pointer, and \
                  this one is a live local"
    )]
    let status = unsafe { nvs_safepoint(&raw mut ctx) };
    assert_eq!(status, nvs_runtime::FATAL);
    assert_eq!(seen().as_deref(), Some("memory"));
    drop(hog);

    let mut ctx = Ctx::new(OutputSink::Sink);
    ctx.request_safepoint(nvs_runtime::SafepointFlags::CPU_LIMIT);
    ctx.set_limit_handler(closure_taking_the_report(records_the_report));
    #[expect(
        unsafe_code,
        reason = "the same live local, one branch of the poll further down"
    )]
    let status = unsafe { nvs_safepoint(&raw mut ctx) };
    assert_eq!(status, nvs_runtime::FATAL);
    assert_eq!(seen().as_deref(), Some("cpu_time"));
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

/// ADR 0020 § 1's reserved slice has a CPU half, and on that half the ceiling
/// is only half the mechanism: a handler entered under `CPU_LIMIT` and left
/// under it is stopped again at its own first back edge, so its slice is zero
/// wide however many nanoseconds `fatal_reserve_time` names.
///
/// Four readings, because the claim is that the widening is *temporary and
/// exact*. Inside, the ceiling is the ordinary one plus the reserve and the
/// flag is down; outside, both are back as they were. A ladder that lowered the
/// flag and left it down would pass the first two on its own while leaving a
/// request that burned its whole ceiling looking as though it never had one.
#[test]
fn a_fatal_handler_runs_inside_its_reserved_time_slice() {
    let mut ctx = Ctx::new(OutputSink::Sink);
    ctx.set_cpu_limit(2_000_000_000);
    ctx.set_fatal_reserve_time(300_000_000);
    ctx.request_safepoint(nvs_runtime::SafepointFlags::CPU_LIMIT);
    ctx.set_limit_handler(closure_of(reports_its_time));

    #[expect(
        unsafe_code,
        reason = "as above: the safepoint's ABI takes a context pointer, and \
                  this one is a live local"
    )]
    let status = unsafe { nvs_safepoint(&raw mut ctx) };

    assert_eq!(status, nvs_runtime::FATAL);
    assert_eq!(
        SEEN_CPU_CEILING.load(Ordering::SeqCst),
        2_300_000_000,
        "the handler is entered against the ordinary ceiling plus the reserve",
    );
    assert!(
        !SEEN_CPU_FLAG.load(Ordering::SeqCst),
        "and with the flag that stopped the request lowered, or the slice it \
         was given is zero wide",
    );
    assert_eq!(
        ctx.cpu_limit(),
        2_000_000_000,
        "the slice is the handler's, not the request's",
    );
    assert!(
        ctx.safepoint_flags()
            .contains(nvs_runtime::SafepointFlags::CPU_LIMIT),
        "which is still the request that exceeded its CPU time",
    );
}

/// `docs/plan/m6.md`'s *Verify*, and the whole reason `[limits] max_script_depth` has a default at
/// all: a recursive `spawn script` is stopped by that ceiling and **reported as that ceiling**
/// rather than as an out-of-memory.
///
/// Both halves the name promises are asserted, because either one alone is green against the
/// failure this replaces. An unbounded recursion of isolates already stopped — it exhausted the
/// tree's heap — and already stopped as a `FATAL`, so a case asking only whether the spawn was
/// refused would have passed before any of this was written. What separates the two is the word the
/// handler is handed, so it is read out of ADR 0020 § 1's report rather than out of the message: a
/// program branches on `max_script_depth`, never on a sentence.
///
/// The refusal is asked of `Isolate::start` directly because that is the single point it lives at —
/// `Ctx::script_depth_breach`'s doc owns why the question belongs to the parent, and
/// `Core\Script::spawn` is the one caller that turns the recorded breach into the status a program
/// sees.
#[test]
fn a_recursive_spawn_is_reported_as_max_script_depth_and_not_as_memory() {
    let mut request = Ctx::new(OutputSink::Sink);
    request.set_max_script_depth(2);
    // Depths 0, 1 and 2 under a ceiling of 2, the ceiling carried down by `Ctx::isolate` rather than
    // re-read: the grandchild is the deepest context allowed to exist, so its own spawn is the first
    // one refused and this is the bound's far side.
    let mut deepest = request.isolate(OutputSink::Sink).isolate(OutputSink::Sink);
    assert_eq!(deepest.script_depth(), 2);
    deepest.set_limit_handler(closure_taking_the_report(records_the_spawn_report));

    let refused = nvs_host::Isolate::new(
        Box::new(|_ctx: &mut Ctx, _args: Value| Value::null()),
        Value::null(),
        nvs_host::Output::Capture,
    )
    .start(&mut deepest)
    .expect("a depth refusal is the tree's, not the argument's `GraphError`");

    assert_eq!(
        SEEN_SPAWN_LIMIT
            .lock()
            .expect("no test panics holding this")
            .take()
            .as_deref(),
        Some("max_script_depth"),
        "and not `memory`, which is what the heap filling up would have reported",
    );
    assert!(
        deepest.pending().is_some(),
        "the request is failing from the refusal onwards — `Core\\Script::spawn` reads this and \
         answers ADR 0020 § 1's `FATAL`, which no `catch` sees",
    );

    let completion = refused.join(&mut deepest);
    assert!(!completion.ok, "no child was built to succeed");
    let failure = completion.error.expect("a failed completion names why");
    assert!(
        failure.message.contains("ceiling of 2"),
        "the failure names the ceiling the operator wrote: {failure:?}",
    );
}

/// `docs/plan/m6.md`'s *Verify*: N concurrent isolates cannot together exceed the tree's budget —
/// because the budget is the **root's**, and a child is never handed one of its own to spend.
///
/// The claim is a comparison and not a threshold, because the threshold half is green against the
/// shape this pins. `Ctx::isolate` builds the child through `Ctx::new`, which re-bases
/// `memory_base` off `crate::budget`'s counter as it stands *now*, so a child reads back only what
/// it allocated itself and no one of four one-megabyte isolates is ever over a three-megabyte
/// ceiling. What bounds the tree anyway is that the parent's base was taken before any of them
/// existed and the counter is one per thread, so every byte a child holds is still on the parent's
/// reading — `Ctx::isolate`'s own "a budget is accounted at the root of the request tree and never
/// per isolate" (`crates/nvs-runtime/src/ctx.rs:1781`), asserted rather than stated.
///
/// Each child's reading is taken **before the next one exists**, which is the only order that can
/// tell the two apart: read at the end, every context in the tree answers the same number and a
/// runtime accounting each isolate separately would look identical.
///
/// The row names three budgets and this asks two. CPU is the deadline word, which crosses by value
/// in that same constructor: a child spawned under an expired deadline is born expired rather than
/// clocked afresh, which is the direction a spawn could have widened. Output is not asserted
/// because there is nothing to assert against — `[limits] max_output` is a configuration key
/// `Ctx` holds no ceiling for, so no isolate can be over it yet.
#[test]
fn n_concurrent_isolates_cannot_together_exceed_the_trees_budget() {
    /// What each isolate holds, well under the ceiling on its own.
    const SHARE: usize = 1 << 20;
    /// The tree's ceiling: over three shares, so no child reaches it and four together pass it.
    const CEILING: usize = 3 << 20;
    const CHILDREN: usize = 4;

    let mut root = Ctx::new(OutputSink::Sink);
    root.set_memory_limit(CEILING);

    // Both halves are held to the end of the case: a share freed early is a byte off the root's
    // reading, which is the thing being measured.
    let mut tree: Vec<(Ctx, Vec<u8>)> = Vec::with_capacity(CHILDREN);
    let mut readings: Vec<usize> = Vec::with_capacity(CHILDREN);
    for _ in 0..CHILDREN {
        let child = root.isolate(OutputSink::Sink);
        let share = vec![0_u8; SHARE];
        readings.push(child.memory_used());
        tree.push((child, share));
    }

    for (n, reading) in readings.iter().enumerate() {
        assert!(
            *reading < CEILING,
            "isolate {n} is under the ceiling by its own reading — {reading} bytes against \
             {CEILING} — which is what makes the root's the only reading that bounds the tree",
        );
    }
    assert!(
        root.memory_used() >= CHILDREN * SHARE,
        "the root carries every isolate's share: {} bytes against {CHILDREN} × {SHARE}",
        root.memory_used(),
    );
    assert!(
        root.over_memory_limit(),
        "and is therefore over a ceiling none of its isolates individually reached",
    );

    // ADR 0020 § 1's other half of a tree-wide budget: the clock. A child built after the
    // deadline passed is born expired, so a spawn cannot buy the tree more CPU time than the
    // request that started it was given.
    root.expire_deadline();
    assert!(
        root.isolate(OutputSink::Sink).deadline_expired(),
        "an isolate inherits the deadline in force where it was spawned, not a fresh one",
    );

    #[expect(
        unsafe_code,
        reason = "as above: the safepoint's ABI takes a context pointer, and \
                  this one is a live local"
    )]
    let status = unsafe { nvs_safepoint(&raw mut root) };
    assert_eq!(
        status,
        nvs_runtime::FATAL,
        "the tree's breach stops the request that owns it, as any other breach of the same \
         ceiling would",
    );
    drop(tree);
}

/// The snapshot `written` resolves to — the typed tree and the table beside it, because a request
/// reads a directive out of one and a capability out of the other.
///
/// The same three lines as `crates/nvs-runtime/tests/configured_limits.rs`'s `snapshot`, which is
/// where a `[limits]` case wants it. Two test binaries in two crates cannot share a helper without
/// a crate to put it in, and a shared crate for three lines would cost more than the copy.
fn snapshot_of(written: &str) -> Arc<Snapshot> {
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

/// `docs/plan/m6.md`'s *Verify*: a child cannot widen a capability its parent narrowed.
///
/// A capability is narrowed by the operator and never by the request — `nvs_config::Request::set`
/// refuses the `capabilities` block whichever way it is asked, because that row is
/// `Class::RuntimeTighten` and a grant is a list with no quantity to narrow *by*
/// (`crates/nvs-config/src/request.rs:106`). So the grant in force where the spawn happened is the
/// whole of what a child has, and this case asks the two ways a child could have got more.
///
/// **By inheriting a default.** The grant crosses in the cloned snapshot (`Ctx::isolate`,
/// `crates/nvs-runtime/src/ctx.rs:1802`), so both answers are asserted rather than the refusal
/// alone: a child that had crossed with no configuration at all would be refused `script.spawn`
/// too — `nvs_runtime::capability::refusal` denies by default — and would look right here while
/// having inherited nothing.
///
/// **By setting it.** Both directions are refused, and they fail for different reasons that a case
/// asking one of them would not separate: widening what is in force is a comparison that cannot be
/// shown to narrow, and granting what was never granted has nothing in force to narrow *from*.
///
/// The snapshot's identity is the third assertion. A child that re-resolved the tree from disk
/// would answer every question above correctly against an unchanged file and silently widen the
/// moment the file differed from what the parent was serving — which is the failure ADR 0078 § 1's
/// one-clone-at-start exists to prevent, asked here of the second context in a tree rather than of
/// the first.
#[test]
fn a_child_cannot_widen_a_capability_its_parent_narrowed() {
    let mut parent = Ctx::new(OutputSink::Sink);
    parent.set_config(snapshot_of(
        "[capabilities.fs]\nread = true\n[capabilities.script]\nspawn = false\n",
    ));
    let mut child = parent.isolate(OutputSink::Sink);

    assert!(
        nvs_runtime::capability::require(&child, Cap::FsRead, Scope::Unscoped, "Core\\File::read")
            .is_ok(),
        "the grant itself crossed, not an empty configuration that denies everything",
    );
    assert!(
        nvs_runtime::capability::require(&child, Cap::ScriptSpawn, Scope::Unscoped, "spawn script")
            .is_err(),
        "and what the parent may not do, the child may not do",
    );

    let config = child
        .config_mut()
        .expect("a child of a configured request is configured");
    assert!(
        !config.set("capabilities.script.spawn", "true"),
        "a request cannot widen the capability in force for it",
    );
    assert!(
        !config.set("capabilities.net.connect", "example.test"),
        "nor grant itself one the operator granted nobody",
    );

    assert!(
        nvs_runtime::capability::require(&child, Cap::ScriptSpawn, Scope::Unscoped, "spawn script")
            .is_err(),
        "the refusal outlives the attempt, which is what makes `set`'s `false` a refusal rather \
         than a reading",
    );
    assert!(
        Arc::ptr_eq(
            child.config().expect("as above").snapshot(),
            parent
                .config()
                .expect("the request kept its own")
                .snapshot(),
        ),
        "and the child reads the snapshot in force at the spawn rather than re-resolving the \
         tree, which is the other direction it could have widened from",
    );
}
