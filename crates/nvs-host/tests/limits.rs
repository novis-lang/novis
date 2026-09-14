//! What a resource limit does to a request in flight — `rule:errors/on-limit`'s ladder, asked at the safepoint poll that is the only place a program
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
//! little enough off a closure value that a `ClassTable` and a plain
//! `extern "C"` function are a whole one. `nvs-stdlib`'s `allocation_policy.rs`
//! owns that shape and the reason the table is leaked.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};
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

/// [`SEEN_LIMIT`]'s twin for the spawn-depth case, and its doc owns why they
/// are separate.
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

/// [`SEEN_LIMIT`]'s twin for the child isolate stopped by its memory ceiling,
/// and that slot's doc owns why every case here has one of its own.
static SEEN_CHILD_MEMORY_LIMIT: Mutex<Option<String>> = Mutex::new(None);

/// [`records_the_report`] writing into [`SEEN_CHILD_MEMORY_LIMIT`] instead, a
/// copy for [`records_the_spawn_report`]'s reason: the ABI captures nothing.
#[expect(
    unsafe_code,
    reason = "the same callee contract as `records_the_report`, over the same \
              two live values"
)]
unsafe extern "C" fn records_the_child_memory_report(
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
        *SEEN_CHILD_MEMORY_LIMIT
            .lock()
            .expect("no test panics holding this") = named.as_text().map(str::to_owned);
        drop(NvsStr::from_raw(key));

        (*args).release();
        report.release();
        *out = Value::null();
    }
    nvs_runtime::OK
}

/// The same slot again for the child isolate the CPU-time flag stops, separate
/// from the one above for the reason [`SEEN_LIMIT`] gives.
static SEEN_CHILD_CPU_LIMIT: Mutex<Option<String>> = Mutex::new(None);

/// [`records_the_report`] writing into [`SEEN_CHILD_CPU_LIMIT`] instead, and a
/// copy for the reason the copy above is one.
#[expect(
    unsafe_code,
    reason = "the same callee contract as `records_the_report`, over the same \
              two live values"
)]
unsafe extern "C" fn records_the_child_cpu_report(
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
        *SEEN_CHILD_CPU_LIMIT
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
/// carries no parameter list (`rule:types/closure-literal`), so the nibble a written `fn`
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

/// A parent as a request looks to a child that fails: a context carrying an
/// error class, so what stopped the child crosses as a rendered class and
/// message rather than as an empty one.
///
/// The class is the *parent's* and `Ctx::isolate` copies it down, which is what
/// makes it the renderer on the child's side too — a fixture with none is how a
/// `Failure` comes back saying `Error` and nothing else.
fn parent_of_isolates() -> Ctx {
    const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
    let mut table = nvs_runtime::ClassTable::new();
    let root = table.define("Throwable", &SLOTS, &[]);
    let mut ctx = Ctx::new(OutputSink::Sink);
    ctx.set_runtime_error_class(nvs_runtime::ErrorClass::new(
        std::sync::Arc::new(table),
        root,
    ));
    ctx
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

/// `rule:errors/on-limit`: a request past its memory ceiling is stopped, and stopped as
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

/// `rule:errors/on-limit` lists CPU time beside memory: a request the CPU-time flag has
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

/// `rule:errors/on-limit`'s `closure(LimitReport)`: the handler is handed a report, and
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

/// `rule:errors/on-limit`: the registered handler runs, it runs *before* the breach
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

/// `rule:errors/on-limit`: the handler runs out of a slice of the request's own budget
/// reserved for it and unavailable to ordinary execution.
///
/// Asserted from *inside* the handler, because that is the only place the
/// reserve is observable: from outside, a request that has breached its ceiling
/// and a request whose handler was given room look identical. The readings
/// taken inside are the claim — the ceiling in force is exactly the ordinary
/// one plus the reserve, and against it the request is no longer over — and the
/// reading after the call returns is that the room went away again rather
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

/// `rule:errors/on-limit`'s reserved slice has a CPU half, and on that half the ceiling
/// is only half the mechanism: a handler entered under `CPU_LIMIT` and left
/// under it is stopped again at its own first back edge, so its slice is zero
/// wide however many nanoseconds `fatal_reserve_time` names.
///
/// The readings come in pairs, because the claim is that the widening is
/// *temporary and exact*. Inside, the ceiling is the ordinary one plus the
/// reserve and the flag is down; outside, both are back as they were. A ladder
/// that lowered the flag and left it down would pass the inside pair on its own
/// while leaving a request that burned its whole ceiling looking as though it
/// never had one.
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
/// `docs/plan/m8.md`'s *Verify*, `rule:errors/handler-script`: the tier-3 handler runs charged to the engine's own
/// reserve, and still runs when the request reporting itself is at its own ceiling.
///
/// Asserted on the **context**, because that is where the exception to `rule:security/isolate-shares-nothing` lives and where a
/// regression would land: `Ctx::handler_isolate` is what parts from `Ctx::isolate`, and running a
/// real `.nvs` through `ladder::escalate` would assert the same fields through a compiler, a
/// capability and a path resolver, none of which is what this case is about.
///
/// The parent is left in the state the ladder actually meets it in — past its memory ceiling *and*
/// past its wall clock — because those are two independent ways it could take the handler down with
/// it, and a constructor that inherited either one is green against a case that arranges only the
/// other.
#[test]
fn the_configured_handler_script_runs_charged_to_the_engines_own_reserve() {
    let (mut ctx, hog) = breached();
    // Stated the way an operator states it rather than through `set_memory_limit`, because
    // `Ctx::set_config` recomputes the ceiling out of `[limits]` and would otherwise uncap the
    // request this case exists to have capped. The hog `breached` is still holding is what puts it
    // over, whichever way the number arrived.
    ctx.set_config(snapshot_of(
        "[limits]\nmemory = '1M'\n\n[log]\nhandler = 'report.nvs'\nhandler_reserve_memory = '32M'\nhandler_reserve_time = '2s'\n",
    ));
    ctx.expire_deadline();
    assert!(
        ctx.over_memory_limit(),
        "the case only means anything with the request already out of memory"
    );

    let handler = ctx.handler_isolate(OutputSink::Buffer(Vec::new()));

    assert_eq!(
        handler.memory_limit(),
        32 << 20,
        "the ceiling is the reserve the operator wrote, not what the request had left",
    );
    assert!(
        !handler.over_memory_limit(),
        "so the handler has room where the request it reports has none",
    );
    assert_eq!(
        handler.cpu_limit(),
        2_000_000_000,
        "and the same on the time half",
    );
    assert!(
        !handler.deadline_expired(),
        "a request stopped by its wall clock still gets a report: the deadline word is the \
         handler's own",
    );
    drop(hog);
}

/// The same exception where the configuration states neither ceiling — the engine's own numbers,
/// which is what "engine-owned" means when nobody wrote one down.
///
/// The reading that matters is the second: an unconfigured handler must not fall back to the
/// failing request's ceiling, which is the shape `Ctx::isolate` has and the one this constructor
/// exists to break.
#[test]
fn an_unstated_handler_reserve_is_the_engines_own_and_never_the_requests() {
    let (ctx, hog) = breached();

    let handler = ctx.handler_isolate(OutputSink::Buffer(Vec::new()));

    assert_eq!(handler.memory_limit(), Ctx::DEFAULT_HANDLER_RESERVE_MEMORY);
    assert_eq!(handler.cpu_limit(), Ctx::DEFAULT_HANDLER_RESERVE_TIME);
    assert!(
        !handler.over_memory_limit(),
        "the request's 4 KiB ceiling is not the handler's",
    );
    drop(hog);
}

/// What the tier-3 handler below read, and how many times it ran. `usize::MAX`, `true` and
/// `FATAL` are the readings of a handler that has not run, so a case that never reached one fails
/// on every line rather than on the counter alone.
static HANDLER_RUNS: AtomicUsize = AtomicUsize::new(0);
static HANDLER_CEILING: AtomicUsize = AtomicUsize::new(usize::MAX);
static HANDLER_OVER: AtomicBool = AtomicBool::new(true);
static HANDLER_POLL: AtomicI32 = AtomicI32::new(nvs_runtime::FATAL);

/// The record's own message as it reached the handler, and the path the resolver was asked for —
/// a slot each, on [`SEEN_LIMIT`]'s reasoning.
static HANDLER_SAW: Mutex<Option<String>> = Mutex::new(None);
static RESOLVED_PATH: Mutex<Option<String>> = Mutex::new(None);

/// The `[log] handler` script, as the [`nvs_runtime::script::Program`] a resolver answers with: it
/// reads the record it was handed, then spends a megabyte of the reserve and polls.
///
/// The allocation and the poll are the point rather than decoration. A reserve is a *ceiling*, so
/// a handler that reads one and returns cannot tell a widened ceiling from one it never needed —
/// what separates them is whether the next safepoint lets it keep going, which is exactly what a
/// handler running on the failing request's own budget would fail.
///
/// `nvs-cli`'s `program_over` is the production shape and this is its skeleton: the argument is
/// discharged into the isolate's own ownership root the same way, which is what
/// [`Ctx::set_isolate_argument`] is for, rather than released here.
#[expect(
    unsafe_code,
    reason = "two ABIs compiled code reaches and a test cannot express otherwise: \
              `nvs_array_get` over a raw array pointer, a borrowed key and the \
              address of a live `Value` for the answer, and the safepoint, which \
              takes the context by pointer"
)]
fn reports_from_inside_the_reserve(ctx: &mut Ctx, args: Value) -> Value {
    HANDLER_RUNS.fetch_add(1, Ordering::SeqCst);
    HANDLER_CEILING.store(ctx.memory_limit(), Ordering::SeqCst);

    let array = args
        .array_ptr()
        .expect("`floor::report_argument` builds the record as one array");
    // Borrowed out of the array, which owns it for the length of this call.
    let key = NvsStr::new(b"message").into_raw();
    let mut named = Value::null();
    unsafe {
        nvs_runtime::nvs_array_get(array, key, &raw mut named);
        *HANDLER_SAW.lock().expect("no test panics holding this") =
            named.as_text().map(str::to_owned);
        drop(NvsStr::from_raw(key));
    }

    let held = vec![0_u8; 1 << 20];
    HANDLER_OVER.store(ctx.over_memory_limit(), Ordering::SeqCst);
    let status = unsafe { nvs_safepoint(&raw mut *ctx) };
    HANDLER_POLL.store(status, Ordering::SeqCst);
    drop(held);

    ctx.write_output(b"reported")
        .expect("`OutputSink::Buffer` never fails");
    ctx.set_isolate_argument(args);
    Value::null()
}

/// This thread's resolver for the case below: one program for whatever path `[log] handler`
/// named, and a note of the path it was asked for.
///
/// A type rather than a closure because [`nvs_runtime::script::Resolver`] is a trait, and it
/// builds a fresh program per call because `resolve` only borrows `self` while a `Program` is a
/// `Box<dyn FnOnce>` that has to be moved out.
#[derive(Debug)]
struct ResolvesToTheProbe;

impl nvs_runtime::script::Resolver for ResolvesToTheProbe {
    fn resolve(&self, path: &str) -> Result<nvs_runtime::script::Program, String> {
        *RESOLVED_PATH.lock().expect("no test panics holding this") = Some(path.to_owned());
        Ok(Box::new(reports_from_inside_the_reserve))
    }
}

/// `docs/plan/m8.md`'s *Verify*, `rule:errors/handler-script`'s second clause: the configured handler **still
/// fires** when the request reporting itself is at its own memory ceiling.
///
/// The sibling above asks [`Ctx::handler_isolate`] for the fields it parts from
/// `Ctx::isolate` on. This one asks the **ladder**, end to end and from a request that has already
/// breached: `nvs_host::ladder::escalate` reading `[log] handler`, passing `rule:security/capability-check-at-the-door`'s spawn
/// door, resolving the path, running the program under `Charge::EngineReserve` and answering
/// `true` — which is its contract for "the handler reported, so tier 4 owes nothing".
///
/// Either half alone is green against the failure this pins. A constructor that widens correctly
/// buys nothing if the escalation never reaches it, and an escalation that reaches a handler
/// charged to the failing request stops that handler at its first safepoint — which is why the
/// readings taken *inside* are a spent megabyte and the poll after it rather than the ceiling
/// alone. [`reports_from_inside_the_reserve`] owns that reasoning.
///
/// The output is asserted too, because the handler's bytes are `Output::Inherit`'s: a report that
/// appeared on a stream of its own would be the second channel `rule:errors/log-write` does not have.
#[test]
fn the_handler_still_fires_when_the_reporting_request_is_at_its_memory_ceiling() {
    // `Buffer` rather than `breached`'s `Sink`, because the handler's own output joins this
    // stream at the join and a discarded one cannot be read back.
    let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
    ctx.set_config(snapshot_of(
        "[limits]\nmemory = '1M'\n\n[log]\nhandler = 'report.nvs'\nhandler_reserve_memory = '32M'\n\n[capabilities.script]\nspawn = true\n",
    ));
    // Held to the end of the case: a hog freed early is a byte off the reading that puts this
    // request over, which is the whole premise.
    let hog = vec![0_u8; 4 << 20];
    assert!(
        ctx.over_memory_limit(),
        "the case only means anything with the request already out of memory"
    );

    let record =
        nvs_runtime::floor::note(nvs_render::Level::Error, "the request ran out of memory");
    let before = HANDLER_RUNS.load(Ordering::SeqCst);
    let reported = nvs_runtime::script::scoped(&ResolvesToTheProbe, || {
        nvs_host::ladder::escalate(&mut ctx, &record)
    });

    assert!(
        reported,
        "the handler ran to completion, which is `ladder::escalate`'s whole answer: tier 4 is \
         the floor beneath tier 3, not a second line beside it",
    );
    assert_eq!(
        HANDLER_RUNS.load(Ordering::SeqCst) - before,
        1,
        "and ran once — § 3's zero retries, with nothing attempted twice",
    );
    assert_eq!(
        RESOLVED_PATH
            .lock()
            .expect("no test panics holding this")
            .take()
            .as_deref(),
        Some("report.nvs"),
        "resolved from `[log] handler`, through the spawn door the grant above opens",
    );
    assert_eq!(
        HANDLER_CEILING.load(Ordering::SeqCst),
        32 << 20,
        "under the reserve the operator wrote, not under what the failing request had left",
    );
    assert!(
        !HANDLER_OVER.load(Ordering::SeqCst),
        "with a megabyte of it actually spent, where the request cannot spend a byte",
    );
    assert_eq!(
        HANDLER_POLL.load(Ordering::SeqCst),
        nvs_runtime::OK,
        "and its own safepoint lets it carry on, which is what a handler charged to the request \
         it reports would fail: a `FATAL` here is a report that stops before it is written",
    );
    assert_eq!(
        HANDLER_SAW
            .lock()
            .expect("no test panics holding this")
            .take()
            .as_deref(),
        Some("the request ran out of memory"),
        "the record reaches the handler as § 1's argument rather than as a sentence printed \
         after it",
    );
    assert_eq!(
        ctx.take_buffered_output(),
        Some(b"reported".to_vec()),
        "and what it wrote joins the failing program's own stream at the await",
    );
    assert!(
        ctx.over_memory_limit(),
        "the reserve was the handler's: the request is still the one that ran out",
    );
    drop(hog);
}

/// Both halves the name promises are asserted, because either one alone is green against the
/// failure this pins. An unbounded recursion of isolates stops on its own — it exhausts the
/// tree's heap — and stops as a `FATAL`, so a case asking only whether the spawn was
/// refused would pass with no depth ceiling at all. What separates the two is the word the
/// handler is handed, so it is read out of `rule:errors/on-limit`'s report rather than out of the message: a
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
         answers `rule:errors/on-limit`'s `FATAL`, which no `catch` sees",
    );

    let completion = refused.join(&mut deepest);
    assert!(!completion.ok, "no child was built to succeed");
    let failure = completion.error.expect("a failed completion names why");
    assert!(
        failure.message.contains("ceiling of 2"),
        "the failure names the ceiling the operator wrote: {failure:?}",
    );
}

/// `rule:security/isolate-failure-is-a-value`: a child stopped by its memory ceiling reaches its
/// parent as **data** — `ok = false` carrying the breach — and the parent runs the statement after
/// the await rather than being unwound into.
///
/// The child is handed a ceiling of its own because `Ctx::isolate` deliberately carries none across
/// (`crates/nvs-host/src/isolate.rs:487`): the budget is the root's, so a child that is over on its
/// own reading is the one arrangement in which the breach being pinned is the child's.
///
/// Which limit stopped it is read out of `rule:errors/on-limit`'s report rather than out of the
/// message, for the case above's reason: a program branches on `memory`, never on a sentence. Both
/// halves are asserted, because a parent left standing beside a completion naming nothing is a
/// boundary that dropped the failure on the floor, and a named failure beside a parent the breach
/// took down with it is the unwind the rule exists to forbid.
#[test]
fn a_childs_memory_breach_arrives_as_ok_false_and_the_parent_keeps_running() {
    let mut parent = parent_of_isolates();

    let completion = nvs_host::Isolate::new(
        Box::new(|child: &mut Ctx, _args: Value| {
            child.set_memory_limit(4096);
            child.set_limit_handler(closure_taking_the_report(records_the_child_memory_report));
            // Held across the poll, as `breached` holds its own and for its reason: the reading is
            // what this thread holds now against what it held when the child's context was made.
            let hog = vec![0_u8; 1 << 20];

            #[expect(
                unsafe_code,
                reason = "the safepoint's ABI takes a context pointer, and this is the child's \
                          own, live for the length of the program running on it"
            )]
            let status = unsafe { nvs_safepoint(&raw mut *child) };
            assert_eq!(
                status,
                nvs_runtime::FATAL,
                "the child is stopped at its own poll, which is where a program allocating \
                 without calling anything can be stopped at all",
            );

            drop(hog);
            Value::null()
        }),
        Value::null(),
        nvs_host::Output::Capture,
    )
    .run(&mut parent)
    .expect("a null argument has a meaning on the other side");

    assert!(
        !completion.ok,
        "the breach is how this child ended, and it ends as a value",
    );
    let failure = completion.error.expect("a failed completion names why");
    assert!(
        failure.message.contains("memory limit"),
        "carrying the breach the child was stopped on: {failure:?}",
    );
    assert_eq!(
        SEEN_CHILD_MEMORY_LIMIT
            .lock()
            .expect("no test panics holding this")
            .take()
            .as_deref(),
        Some("memory"),
        "and the handler inside the child was handed the same limit by name",
    );
    assert!(
        parent.pending().is_none(),
        "nothing unwound into the parent: the completion is the whole of what it was told",
    );

    #[expect(unsafe_code, reason = "the same ABI over the parent's own live local")]
    let after = unsafe { nvs_safepoint(&raw mut parent) };
    assert_eq!(
        after,
        nvs_runtime::OK,
        "which is what keeps running means here: the ceiling was the child's, so the parent's own \
         poll carries on from where the child's ended it",
    );
}

/// `rule:security/isolate-failure-is-a-value` on the other half of the pair the depth case above
/// separates itself from: a child stopped by the CPU-time flag also crosses as `ok = false`, and
/// also leaves the parent running the statement after the await.
///
/// What differs from the memory half is the *word*, not the boundary. `Ctx::isolate` shares the
/// safepoint word with the child (`crates/nvs-runtime/src/ctx/safepoint.rs:56`) while carrying no
/// ceiling across, so raising the flag from inside the child is the store the watchdog makes while
/// the child runs, on the tree's own word.
///
/// That sharing is why the last assertion is here rather than left to a reader: the parent keeps
/// running in the sense this rule means — nothing unwound into it and the failure arrived as data —
/// while the flag it shared all along is still up, so its own next poll ends it too. That second
/// fact is `rule:security/isolate-budget-is-the-trees` and not this boundary, and a case asserting
/// only the first would read as a child buying the tree more wall time than it had.
#[test]
fn a_childs_cpu_breach_arrives_as_ok_false_and_the_parent_keeps_running() {
    let mut parent = parent_of_isolates();
    parent.set_cpu_limit(2_000_000_000);

    let completion = nvs_host::Isolate::new(
        Box::new(|child: &mut Ctx, _args: Value| {
            child.set_limit_handler(closure_taking_the_report(records_the_child_cpu_report));
            child.request_safepoint(nvs_runtime::SafepointFlags::CPU_LIMIT);

            #[expect(
                unsafe_code,
                reason = "the same ABI over the child's own context, live for the length of the \
                          program running on it"
            )]
            let status = unsafe { nvs_safepoint(&raw mut *child) };
            assert_eq!(
                status,
                nvs_runtime::FATAL,
                "the CPU branch of the poll stops the child as a `FATAL`, whichever limit it is",
            );

            Value::null()
        }),
        Value::null(),
        nvs_host::Output::Capture,
    )
    .run(&mut parent)
    .expect("a null argument has a meaning on the other side");

    assert!(
        !completion.ok,
        "the stop is this child's ending, as a value"
    );
    let failure = completion.error.expect("a failed completion names why");
    assert!(
        failure.message.contains("CPU-time limit"),
        "carrying the limit the child was stopped on: {failure:?}",
    );
    assert_eq!(
        SEEN_CHILD_CPU_LIMIT
            .lock()
            .expect("no test panics holding this")
            .take()
            .as_deref(),
        Some("cpu_time"),
        "and not `memory`, which is the branch above the one that stopped it",
    );
    assert!(
        parent.pending().is_none(),
        "nothing unwound into the parent: the completion is the whole of what it was told",
    );
    assert!(
        parent
            .safepoint_flags()
            .contains(nvs_runtime::SafepointFlags::CPU_LIMIT),
        "on a word the parent shares, so the tree is still the thing that ran out of time",
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
/// per isolate" (`crates/nvs-runtime/src/ctx/isolate.rs:188`), asserted rather than stated.
///
/// Each child's reading is taken **before the next one exists**, which is the only order that can
/// tell the two apart: read at the end, every context in the tree answers the same number and a
/// runtime accounting each isolate separately would look identical.
///
/// This asks each budget the row names. CPU is the deadline word, which crosses as
/// the *word* in that same constructor: one store expires the whole tree, so a spawn cannot buy the
/// tree more wall time than the request that started it was given — asked below of a child built
/// before the timer fired, which is the order a copied flag would have failed. Output is the memory
/// arrangement one field along — `nvs_runtime::budget`'s output count is per thread too and
/// `Ctx::new` re-bases it too — so it is built in the same loop and asked with the same
/// readings, which is `rule:security/isolate-shares-nothing`'s "child output against the root's `max_output`".
///
/// The safepoint at the end can only ever answer for memory, because that is the branch the poll
/// reaches first. So the output branch of that same poll is pinned below on a context over one
/// ceiling only, which is the one arrangement that can tell it apart from its neighbour.
#[test]
fn n_concurrent_isolates_cannot_together_exceed_the_trees_budget() {
    /// What each isolate holds, well under the ceiling on its own.
    const SHARE: usize = 1 << 20;
    /// The tree's ceiling: over three shares, so no child reaches it and four together pass it.
    const CEILING: usize = 3 << 20;
    /// What each isolate writes, in the same proportion to its own ceiling as `SHARE` is.
    const WRITTEN: usize = 1 << 10;
    /// The tree's output ceiling: over three of those, and under four.
    const OUT_CEILING: usize = 3 << 10;
    const CHILDREN: usize = 4;

    let mut root = Ctx::new(OutputSink::Sink);
    root.set_memory_limit(CEILING);
    root.set_output_limit(OUT_CEILING);

    // Both halves are held to the end of the case: a share freed early is a byte off the root's
    // reading, which is the thing being measured.
    let mut tree: Vec<(Ctx, Vec<u8>)> = Vec::with_capacity(CHILDREN);
    let mut readings: Vec<usize> = Vec::with_capacity(CHILDREN);
    let mut writings: Vec<usize> = Vec::with_capacity(CHILDREN);
    for _ in 0..CHILDREN {
        let mut child = root.isolate(OutputSink::Sink);
        let share = vec![0_u8; SHARE];
        child
            .write_output(&[b'.'; WRITTEN])
            .expect("`OutputSink::Sink` never fails");
        readings.push(child.memory_used());
        writings.push(child.output_used());
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

    for (n, written) in writings.iter().enumerate() {
        assert!(
            *written < OUT_CEILING,
            "isolate {n} is under the output ceiling by its own reading — {written} bytes against \
             {OUT_CEILING} — for the same reason it is under the memory one",
        );
    }
    assert!(
        root.output_used() >= CHILDREN * WRITTEN,
        "the root carries every isolate's output: {} bytes against {CHILDREN} × {WRITTEN}",
        root.output_used(),
    );
    assert!(
        root.output_breach().is_some(),
        "and is therefore over an output ceiling none of its isolates individually reached",
    );

    // `rule:errors/on-limit`'s other half of a tree-wide budget: the clock. Both orders, because they fail
    // differently — a copied flag reaches the second and never the first.
    let early = root.isolate(OutputSink::Sink);
    assert!(!early.deadline_expired(), "nothing has fired yet");
    root.expire_deadline();
    assert!(
        early.deadline_expired(),
        "an isolate spawned before the timer fired stops with the tree rather than running on",
    );
    assert!(
        root.isolate(OutputSink::Sink).deadline_expired(),
        "and one spawned after it inherits the deadline in force where it was spawned",
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

    // The output branch of that same poll, asked where memory is not also over — the tree above
    // could never reach it, since `nvs_safepoint` asks memory first and answers there. This
    // context's base is taken after everything above wrote, so its reading is its own ten bytes.
    let mut writer = Ctx::new(OutputSink::Sink);
    writer.set_output_limit(8);
    writer
        .write_output(b"ten bytes.")
        .expect("`OutputSink::Sink` never fails");
    #[expect(
        unsafe_code,
        reason = "as above: the safepoint's ABI takes a context pointer, and \
                  this one is a live local"
    )]
    let status = unsafe { nvs_safepoint(&raw mut writer) };
    assert_eq!(
        status,
        nvs_runtime::FATAL,
        "a request past `[limits] max_output` is stopped by the poll, as `rule:errors/on-limit`'s other \
         resource limits are",
    );
}

/// The snapshot `written` resolves to — the typed tree and the table beside it, because a request
/// reads a directive out of one and a capability out of the other.
///
/// The same lines as `crates/nvs-runtime/tests/configured_limits.rs`'s `snapshot`, which is
/// where a `[limits]` case wants it. Two test binaries in two crates cannot share a helper without
/// a crate to put it in, and a shared crate for a helper this small would cost more than the copy.
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
/// whole of what a child has, and this case asks the ways a child could have got more.
///
/// **By inheriting a default.** The grant crosses in the cloned snapshot (`Ctx::isolate`,
/// `crates/nvs-runtime/src/ctx/isolate.rs:188`), so both answers are asserted rather than the refusal
/// alone: a child that had crossed with no configuration at all would be refused `script.spawn`
/// too — `nvs_runtime::capability::refusal` denies by default — and would look right here while
/// having inherited nothing.
///
/// **By setting it.** Both directions are refused, and they fail for different reasons that a case
/// asking one of them would not separate: widening what is in force is a comparison that cannot be
/// shown to narrow, and granting what was never granted has nothing in force to narrow *from*.
///
/// The snapshot's identity is asserted too. A child that re-resolved the tree from disk
/// would answer every question above correctly against an unchanged file and silently widen the
/// moment the file differed from what the parent was serving — which is the failure `rule:config/the-config-is-an-immutable-snapshot`'s
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
