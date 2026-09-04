//! `Core\Script\Handle` — what `spawn script` hands back, and the one `Core`
//! class registered so that a program can name a value it may not touch.
//!
//! [ADR 0006](../../../../docs/adr/0006-isolated-script-execution.md)'s spawn
//! is an expression, so its answer needs a type, and `await` is the only thing
//! a program may do with that answer. `crates/nvs-types/src/expr/isolate.rs`'s
//! module doc is the one home of *why* this is a `Core` class rather than a
//! shape — a `Core` instance has no property a program can reach, which is
//! exactly what a handle is — and this module does not restate the argument.
//!
//! # Registered with no members, on purpose
//!
//! [`crate::attributes`] registers members whose bodies never run; this
//! registers no members at all, which is a different thing and a stronger one.
//! There is no `$job->cancel()`, no `$job->isDone()` and no `$job->id()`:
//! cancellation is the parent's own teardown reaching its children
//! ([ADR 0072](../../../../docs/adr/0072-core-task-structured-concurrency.md)
//! § 5), and every "is it finished yet" answer is stale before the caller reads
//! it — the same reasoning [`crate::channel`] records for `count`/`isFull`.
//! `await` is the whole surface, and it is a keyword rather than a member
//! because it suspends the calling task.
//!
//! The class is absent from [`crate::registry::CONSTRUCTORS`] for the same
//! reason, so `new Core\Script\Handle()` is refused where it is written: the
//! only thing that may build one is the lowering of a `spawn script`, which
//! knows what isolate it names.
//!
//! # One slot, holding a key and not the isolate
//!
//! A handle's single slot holds an `int`: the key
//! [`Ctx::hold_started_script`](nvs_runtime::Ctx::hold_started_script) filed
//! the running isolate under. It cannot hold the isolate itself — a task id, a
//! completion slot and a `Wake` are not Novis values — and it may not be an
//! entry in a table *this* module keeps, because a `Core` instance has no
//! native drop, so nothing would ever tell such a table that the last reference
//! to a handle had gone and its footprint would be O(spawns served). That
//! method's own doc is the one home of the accounting; `crate::channel` records
//! the identical argument for the queue it keeps in slots instead.
//!
//! # The two symbols, and why they have no rows
//!
//! [`SPAWN_SYMBOL`] and [`AWAIT_SYMBOL`] are reached by `nvs-ir` through
//! `nvs_types`, exactly as `Core\Router::url`'s prepared-path helper is, and
//! neither has a [`CoreMethod`] row. That is the
//! point: `spawn script` and `await` are *syntax*, so the only thing that may
//! call either is the lowering of the construct that spells it, and a row would
//! make both reachable as `Core\Script\Handle::…()` from source. A symbol with
//! no row is invisible to `spec_registry_coverage.rs` and to
//! `conformance_coverage.rs` for the same reason, and the construct's own
//! `.nvst` cases are what cover it instead.
//!
//! # The end of a script, and the two halves it is written in
//!
//! [ADR 0127](../../../../docs/adr/0127-the-end-of-a-script-is-observable.md)'s
//! `onExit` queue is split across this crate and `nvs_runtime` along the seam
//! [`crate::fatal`] already uses: **registration and routing** are here, and
//! the queue itself is [`Ctx`](nvs_runtime::Ctx)'s, because the queue is
//! request-local state and the ending is what *has* the outcome in hand.
//!
//! What is unusual is that the routing is here rather than at the call site.
//! [`run_exit_hooks`] is the one door between an ending and a hook: it turns
//! the script frame's own ABI status into § 2's report and answers *early* for
//! a `FATAL`, so § 3's "a limit breach runs no hook" is one branch in one
//! function rather than a rule three call sites in `nvs-cli` each have to
//! remember. It is here and not in `nvs_runtime` because the report is a
//! `Core` instance and this crate is what lays those out.
//!
//! **What it spends:** one reference per registration plus the hook's own
//! captures, held to the end of the script — O(registrations) per request, the
//! spend ADR 0127 § 1 states — and one report allocation per ending, released
//! the moment the drain is over. `Ctx::exit_hooks` is the home of the first
//! half.
//!
//! `Core\Script` itself lands in this module beside its handle, the way
//! `Core\Task` and `Core\Task\Channel<T>` already sit together under
//! [ADR 0011](../../../../docs/adr/0011-functions-and-constants-are-class-members.md).
//! [`CLASS`] is that class; `args()` is its one row so far, and the
//! `valueOrThrow($result)` that a shape cannot carry as a method is still owed.
//!
//! # `args()` answers `mixed`, and `null` for a script nobody spawned
//!
//! [ADR 0012](../../../../docs/adr/0012-no-superglobals.md) § 6 is what
//! replaced ADR 0006's `$_ARGS` with a method call, and its body states the
//! return type this module implements: **`mixed`**, not an array of anything.
//! `spawn script`'s `args:` is checked against no expected type at all —
//! `nvs_types::expr::isolate`'s `check_spawn_script` says why, and it is
//! ADR 0023 § 2's rule that whether a graph may cross is a run-time question —
//! so `with(args: 5)` is accepted where it is written, and an `array<mixed>`
//! return type would be a lie at the one position anybody reads it from.
//!
//! A child that was passed nothing reads **`null`** rather than an empty array.
//! That is `nvs_ir::lower::expr`'s lowering of a missing option, which already
//! records why: a program that wrote `args: []` said something a program that
//! wrote no option at all did not, and an empty array collapses the two. The
//! root script reads `null` for the same reason — nothing spawned it, so there
//! is no argument rather than an empty one.
//!
//! The value is already on this side of the boundary by the time the member
//! runs: `nvs_host::Isolate::start` copied the graph at the spawn, and the
//! child's program discharged that one reference into
//! [`Ctx::set_isolate_argument`](nvs_runtime::Ctx::set_isolate_argument), whose
//! own doc owns the accounting. So a read is a retain and nothing else — no
//! second crossing, and no copy per call, however often the child asks.

use nvs_runtime::host::{Completion, Output};
use nvs_runtime::script::ResolveError;
use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::{CaseDoc, CoreClass, CoreMethod, CoreTy, EnumDoc, MethodDoc, ParamDoc};

/// The handle class's fully-qualified name, as
/// [`CoreTy::Instance`] spells it.
///
/// `pub` because `nvs-types` names this class when it types a `spawn script`
/// expression, and a name written out in two crates is a name that can drift.
pub const HANDLE_NAME: &str = r"Core\Script\Handle";

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const HANDLE: CoreClass = CoreClass {
    name: HANDLE_NAME,
    methods: &[],
    instance: &[],
    slots: &["pending"],
    constants: &[],
};

/// [`HANDLE`]'s one slot: the key of the started isolate this handle names, or
/// `0` for a spawn that never started one.
const PENDING: usize = 0;

/// `Core\Script`'s registry row — the class [ADR 0012] § 6 named when it
/// replaced `$_ARGS` with a method call. See [`crate::registry::CLASSES`].
///
/// A class in its own right beside `Core\Script\Handle`, exactly as
/// `Core\Time` sits beside `Core\Time\Instant`: the handle is what a spawn
/// answers with, and this is what the child asks.
///
/// [ADR 0012]: ../../../../docs/adr/0012-no-superglobals.md
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Script",
    methods: &[
        CoreMethod {
            name: "args",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: ARGS_SYMBOL,
            doc: Some(&ARGS_DOC),
        },
        CoreMethod {
            name: "onExit",
            names: &["hook"],
            params: &[CoreTy::Callable],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: ON_EXIT_SYMBOL,
            doc: Some(&ON_EXIT_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Script::onExit`'s reference card — ADR 0117.
const ON_EXIT_DOC: MethodDoc = MethodDoc {
    short: "Registers a closure to run as the last user code of this script — at a normal end, at \
            an `exit`, and when a throw reaches the root with nothing left to catch it. Hooks run \
            in registration order, once, and a `FATAL` or a cancellation runs none of them.",
    params: &[ParamDoc {
        name: "hook",
        desc: "What to run. It is handed one readonly `Core\\Script\\ExitReport` saying which \
               ending this was, and answers nothing; declaring no parameter is allowed. A hook \
               that throws is logged and abandoned, and the hooks behind it still run.",
        shape: &[],
    }],
    ret: "Nothing. Registering is request-local, registering twice registers twice, and a hook \
          registered by a hook joins the tail of the same drain. Nothing a hook does changes the \
          ending: the report is fixed before the first one runs, and `exit` inside a hook is a \
          `RuntimeError` rather than a second ending.",
    errors: &[],
};

/// `Core\Script\ExitReason`'s fully-qualified name — [ADR 0127] § 1.
///
/// [ADR 0127]: ../../../../docs/adr/0127-the-end-of-a-script-is-observable.md
pub(crate) const EXIT_REASON_NAME: &str = r"Core\Script\ExitReason";

/// The three endings that fire the queue, as the report's own field — ADR 0127
/// § 2's table, in its order. See [`crate::registry::ENUMS`].
///
/// `ExitCall` carries the suffix ADR 0127 § 1 names: a case spelled `Exit`
/// would share a spelling with the keyword, and a program writing
/// `ExitReason::Exit` beside an `exit;` two lines down is reading one word two
/// ways. There is deliberately no case for a `FATAL` or a cancellation — § 3
/// makes those the two terminations no hook observes, so a case for either
/// would be a value nothing can ever produce.
pub(crate) const EXIT_REASON: crate::registry::CoreEnum = crate::registry::CoreEnum {
    name: EXIT_REASON_NAME,
    cases: &[("Normal", 0), ("ExitCall", 1), ("UncaughtThrow", 2)],
    doc: Some(&EXIT_REASON_DOC),
};

/// [`EXIT_REASON`]'s reference card — ADR 0117.
const EXIT_REASON_DOC: EnumDoc = EnumDoc {
    short: "Which of the three endings ran the exit hooks. A `FATAL` and a cancellation have no \
            case here because they run no hook at all.",
    cases: &[
        CaseDoc {
            name: "Normal",
            desc: "The last top-level statement ran and the script ended of its own accord.",
        },
        CaseDoc {
            name: "ExitCall",
            desc: "`exit`, `exit($n)` or `exit(\"msg\")` ended the script — the one ending no \
                   `finally` observes.",
        },
        CaseDoc {
            name: "UncaughtThrow",
            desc: "A throw reached the root of the script with nothing left to catch it; the \
                   report carries the `Throwable` itself.",
        },
    ],
};

/// The `Core`-owned class of ADR 0127 § 1's report, and the report's own name.
pub(crate) const EXIT_REPORT_NAME: &str = r"Core\Script\ExitReport";

/// The `Throwable` an `UncaughtThrow` report carries, as a type a row can name.
///
/// The one [`CoreTy::Instance`] in this crate naming a class the *exception
/// tree* owns rather than [`crate::registry::CLASSES`] — spec § 10's root,
/// declared in `nvs_hir::errors::TREE` and seeded into the checker's class
/// table by `nvs_types::error_lib`, so it interns exactly as any other class
/// type does. `every_instance_type_names_a_registered_class` names it for that
/// reason.
const THROWABLE: CoreTy = CoreTy::Instance("Throwable");

/// ADR 0127 § 1's report, as the instance each hook is handed.
///
/// Three accessors rather than the three *properties* § 1 sketches, which is
/// the same departure `Core\RateLimit\Decision` makes and for the identical
/// reason: a `Core`-owned instance has no property a program can reach
/// ([`CoreTy::Instance`] is the home of that rule), so `$report->reason` would
/// resolve a class, find no member and reach `nvs-ir` with nothing to call.
/// The ADR's own § 1 spells the accessors now.
///
/// **Readonly is structural here rather than enforced.** There is no member
/// that writes a slot, and a `Core` class has no constructor a program may
/// reach, so the only thing that builds one is [`report_of`] — which is
/// downstream of the ending having been decided, and that is § 5's "fixed
/// before the first hook runs" as structure rather than as a rule.
pub(crate) const EXIT_REPORT: CoreClass = CoreClass {
    name: EXIT_REPORT_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "reason",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(EXIT_REASON_NAME),
            symbol: REASON_SYMBOL,
            doc: Some(&REASON_DOC),
        },
        CoreMethod {
            name: "status",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: STATUS_SYMBOL,
            doc: Some(&STATUS_DOC),
        },
        CoreMethod {
            name: "error",
            names: &[],
            params: &[],
            defaults: &[],
            // ADR 0063 R4's "absence is `?T`": two of the three endings have no
            // exception, and a `Throwable` with an empty message would be a
            // different claim from having none.
            return_ty: CoreTy::Nullable(&THROWABLE),
            symbol: ERROR_SYMBOL,
            doc: Some(&ERROR_DOC),
        },
    ],
    slots: &["reason", "status", "error"],
    constants: &[],
};

/// `Core\Script\ExitReport::reason`'s reference card — ADR 0117.
const REASON_DOC: MethodDoc = MethodDoc {
    short: "Which ending is running the hooks.",
    params: &[],
    ret: "`Normal` for the last statement having run, `ExitCall` for an `exit`, `UncaughtThrow` \
          for a throw that reached the root.",
    errors: &[],
};

/// `Core\Script\ExitReport::status`'s reference card — ADR 0117.
const STATUS_DOC: MethodDoc = MethodDoc {
    short: "The status the process will exit with, decided before the first hook ran.",
    params: &[],
    ret: "`0` for a normal end, the `exit($n)` argument for an `exit`, `1` for an uncaught throw. \
          Reading it changes nothing — a hook observes the ending it was given.",
    errors: &[],
};

/// `Core\Script\ExitReport::error`'s reference card — ADR 0117.
const ERROR_DOC: MethodDoc = MethodDoc {
    short: "The exception that ended the script, for the one ending that has one.",
    params: &[],
    ret: "The live `Throwable` for an `UncaughtThrow` — the object the program threw, with its \
          own class, message and backtrace — and `null` for the other two endings.",
    errors: &[],
};

/// `Core\Script::args`'s reference card — ADR 0117.
const ARGS_DOC: MethodDoc = MethodDoc {
    short: "Answers the value this script was spawned with — `spawn script … with(args: …)` as \
            the child sees it, already copied into this isolate's own arena.",
    params: &[],
    ret: "Whatever the parent passed, unchanged in shape; `null` for a child spawned with no \
          `args:` and for the root script, which nothing spawned.",
    errors: &[],
};

/// The symbol [`CLASS`]'s `args` row is reached through.
const ARGS_SYMBOL: &str = "nvs_core_script_args";

/// The symbol [`CLASS`]'s `onExit` row is reached through.
const ON_EXIT_SYMBOL: &str = "nvs_core_script_on_exit";

/// The symbols [`EXIT_REPORT`]'s three accessors are reached through.
const REASON_SYMBOL: &str = "nvs_core_script_exit_report_reason";
const STATUS_SYMBOL: &str = "nvs_core_script_exit_report_status";
const ERROR_SYMBOL: &str = "nvs_core_script_exit_report_error";

/// [`EXIT_REPORT`]'s slots, by index — the layout its `slots` names.
const REASON_SLOT: usize = 0;
const STATUS_SLOT: usize = 1;
const ERROR_SLOT: usize = 2;

/// The symbol `spawn script <path> with(…)` lowers to.
pub const SPAWN_SYMBOL: &str = "nvs_core_script_spawn";

/// The symbol `await <handle>` lowers to.
pub const AWAIT_SYMBOL: &str = "nvs_core_script_await";

/// ADR 0006's `ScriptResult`, as a descriptor name — see
/// [`crate::instance::SHAPE_ROSTER`], whose fields are in **sorted** order
/// because that is the slot order `nvs_types::ty::Ty::Shape` canonicalizes to.
pub(crate) const RESULT_SHAPE: &str = r"Core\Script\Result";
/// [`RESULT_SHAPE`]'s fields, sorted. `crates/nvs-types/src/expr/isolate.rs` is
/// the one home of what each means and why the set is this one.
pub(crate) const RESULT_FIELDS: &[&str] = &["error", "ok", "output", "value"];

/// The nested `{class, message}` a failed `ScriptResult` carries.
pub(crate) const FAILURE_SHAPE: &str = r"Core\Script\Failure";
/// [`FAILURE_SHAPE`]'s fields, sorted.
pub(crate) const FAILURE_FIELDS: &[&str] = &["class", "message"];

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        SPAWN_SYMBOL => (nvs_core_script_spawn as *const ()).cast(),
        AWAIT_SYMBOL => (nvs_core_script_await as *const ()).cast(),
        ARGS_SYMBOL => (nvs_core_script_args as *const ()).cast(),
        ON_EXIT_SYMBOL => (nvs_core_script_on_exit as *const ()).cast(),
        REASON_SYMBOL => (nvs_core_script_exit_report_reason as *const ()).cast(),
        STATUS_SYMBOL => (nvs_core_script_exit_report_status as *const ()).cast(),
        ERROR_SYMBOL => (nvs_core_script_exit_report_error as *const ()).cast(),
        _ => return None,
    })
}

/// One ending, as the report ADR 0127 § 2's table describes it.
///
/// The **only** place this module builds a report, which is § 5's "fixed before
/// the first hook runs" as structure rather than as a rule: every route to a
/// hook runs through [`run_exit_hooks`], and this is the one thing it hands
/// them.
///
/// `error` is retained on the way in because the report holds a reference of
/// its own: the caller's `Thrown` is what keeps the object alive at the ending,
/// and the report outlives neither by design — [`run_exit_hooks`] releases it
/// the moment the drain is over.
fn report_of(reason: i64, status: i64, error: Option<&nvs_runtime::Thrown>) -> Value {
    let error = error.map_or_else(Value::null, |thrown| {
        let value = thrown.as_value();
        #[expect(
            unsafe_code,
            reason = "the report keeps this object past the call that built it, \
                      so it needs a reference of its own — which the release \
                      below then gives back"
        )]
        // SAFETY: the caller's `Thrown` owns a reference for the whole of the
        // ending, so the object is live at the moment this second one is taken.
        unsafe {
            value.retain();
        }
        value
    });
    crate::instance::build(
        &EXIT_REPORT,
        [Value::int(reason), Value::int(status), error],
    )
}

/// Runs [ADR 0127](../../../../docs/adr/0127-the-end-of-a-script-is-observable.md)'s
/// end-of-script queue for the ending `outcome` names — the **one** door
/// between an ending and a hook.
///
/// `outcome` is the script frame's own answer, as `nvs_runtime`'s ABI statuses
/// spell it, and `thrown` is the exception for the one ending that has one.
/// The routing is § 2's table and § 3's two terminations, in one `match`:
///
/// - `Ok(())` — the last top-level statement ran: `Normal`, status `0`.
/// - `Err(EXITED)` — `exit`: `ExitCall`, and the status the call named, which
///   `Ctx::exit_code` is already holding.
/// - `Err(THROWN)` — an uncaught throw: `UncaughtThrow`, status `1`, and the
///   live `Throwable`.
/// - **anything else** — a `FATAL`, which runs no hook at all. § 3 is the
///   argument: a resource-limit breach stopped the script *for exceeding its
///   budget*, and `Core\Fatal::onLimit` on ADR 0020 § 1's reserved slice stays
///   its one observer. A cancellation never reaches here at all, because a
///   cancelled task runs no user code and so never returns an ending.
///
/// Placing that decision here rather than at the call site is what makes it
/// testable and what keeps it from being three decisions: `nvs-cli` calls this
/// with whatever the frame answered and asks nothing about it.
pub fn run_exit_hooks(
    ctx: &mut nvs_runtime::Ctx,
    outcome: Result<(), i32>,
    thrown: Option<&nvs_runtime::Thrown>,
) {
    let report = match outcome {
        Ok(()) => report_of(NORMAL, 0, None),
        Err(status) if status == nvs_runtime::EXITED => report_of(EXIT_CALL, ctx.exit_code(), None),
        Err(status) if status == nvs_runtime::THROWN => report_of(UNCAUGHT_THROW, 1, thrown),
        Err(_) => return,
    };
    ctx.run_exit_hooks(report);
    #[expect(
        unsafe_code,
        reason = "the report was built by this frame and handed to every hook \
                  borrowed, so this frame still owns the only reference to it"
    )]
    // SAFETY: `Ctx::run_exit_hooks` borrows the report and releases nothing of
    // it; releasing this reference releases the `Throwable` slot's with it.
    unsafe {
        report.release();
    }
}

/// [`EXIT_REASON`]'s cases, as the integers a report's slot holds.
const NORMAL: i64 = 0;
const EXIT_CALL: i64 = 1;
const UNCAUGHT_THROW: i64 = 2;

/// A slot of the receiving [`EXIT_REPORT`], retained because it is being
/// answered — `crate::ratelimit`'s `slot_of`, over this class's layout.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not an object — unreachable from
/// source, since an instance member's receiver is typed and `E0401` refuses a
/// call on anything else.
fn slot_of(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &EXIT_REPORT, member)?;
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot's reference belongs to the receiver, which is live for \
                  the length of the call, and this value is being handed to the \
                  caller — which is exactly `Value::retain`'s obligation"
    )]
    // SAFETY: the receiver owns the slot's reference and outlives this call.
    unsafe {
        held.retain();
    }
    Ok(held)
}

/// Decodes the `output:` option, which arrives as the string the program wrote.
///
/// Refused rather than defaulted for an unknown spelling: ADR 0006 names two
/// and a third would silently pick one of them, which is the failure mode a
/// misspelled `'inherait'` is most likely to be. A `LogicError` and not a
/// fatal, because the program supplied the value and can be written to handle
/// having supplied a bad one.
fn output_of(value: &Value) -> Result<Output, Fault> {
    match value.as_text() {
        Some("capture") => Ok(Output::Capture),
        Some("inherit") => Ok(Output::Inherit),
        Some(other) => Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("`spawn script`'s `output:` is `'capture'` or `'inherit'`, got `'{other}'`"),
        )),
        // Unreachable from source: `nvs_types::expr::isolate`'s
        // `check_spawn_script` checks this option against `string`, so a
        // non-string argument is `E_TYPE_MISMATCH` at the written option and
        // never reaches the helper. The guard is what makes the read above
        // total.
        None => Err(Fault::fatal(format!(
            "`spawn script`'s `output:` expected a string, got tag {}",
            value.tag_byte()
        ))),
    }
}

nvs_runtime::nvs_helper! {
    /// `spawn script <path> with(args: …, output: …)` — starts the isolate and
    /// answers the handle that `await` collects.
    ///
    /// Three arguments in the order the lowering builds them: the path as
    /// written, the `args:` value (`null` when the option was not given), and
    /// the `output:` spelling. The argument is **transferred** to the isolate,
    /// which is why the lowering hands it over rather than borrowing it as an
    /// ordinary `Core` call would.
    ///
    /// A path that does not resolve is a **throw in the parent**, not an
    /// `ok = false`: `nvs_runtime::script::ResolveError` is one step before the
    /// boundary and that module's doc says so, and the same is true of an
    /// argument that cannot cross — it was built by the parent before any child
    /// existed. What the boundary turns into a value is only what the *child*
    /// produced.
    ///
    /// **`script.spawn` is checked inside `resolve`**, not here — ADR 0118
    /// § 2. A spawn the parent was never allowed to attempt therefore never
    /// reaches a child at all, which is what makes it a value on *this* side
    /// while a child that fails on its own is an `ok = false` on the other.
    fn nvs_core_script_spawn(ctx, args: [3]) {
        // Unreachable from source: the path expression is checked against
        // `string` by `nvs_types::expr::isolate`'s `check_spawn_script`, so a
        // non-string is `E_TYPE_MISMATCH` where it is written.
        let path = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "`spawn script` expected a string path, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let output = output_of(&args[2])?;
        let program = nvs_runtime::script::resolve(ctx, path).map_err(|error| match error {
            // An embedder that installed none. Not the program's mistake, and
            // not something a `catch` should be able to paper over.
            ResolveError::NoResolver => Fault::fatal(format!(
                "`spawn script '{path}'` needs a script resolver on this thread and there is none"
            )),
            ResolveError::Refused(message) => {
                Fault::thrown_as(ThrownClass::Runtime, format!("`spawn script '{path}'`: {message}"))
            }
            // Already a whole sentence naming the capability and the path —
            // ADR 0118 § 5 — so it is thrown as written rather than wrapped in
            // this member's own framing, which would say `spawn script` twice.
            ResolveError::Denied(message) => Fault::thrown_as(ThrownClass::Runtime, message),
        })?;
        // The argument is handed over here: one reference goes to the isolate
        // and the lowering emitted no release for it.
        let crossing = args[1];
        let started = nvs_runtime::host::with_current(|host| {
            host.start_isolate(ctx, program, crossing, output)
        });
        let running = match started {
            // A spawn past `[limits] max_script_depth` is refused by
            // `Isolate::start` before it builds anything, and the refusal is
            // recorded on *this* context rather than answered as an error:
            // the ceiling is the tree's and the error below is the argument's,
            // and ADR 0020 § 1 makes a limit breach a `FATAL` no `catch` sees
            // while that error is a throw. `Fault::Pending` is how a helper
            // says the status is already on the context; the handle is dropped
            // unjoined, which `host::Running`'s own doc allows.
            Some(Ok(_)) if ctx.pending().is_some() => {
                return Err(Fault::Pending(nvs_runtime::FATAL));
            }
            Some(Ok(running)) => running,
            Some(Err(error)) => {
                return Err(Fault::thrown_as(
                    ThrownClass::Logic,
                    format!("`spawn script '{path}'`: {error}"),
                ));
            }
            None => {
                return Err(Fault::fatal(format!(
                    "`spawn script '{path}'` needs a scheduler on this thread and there is none"
                )));
            }
        };
        let key = ctx.hold_started_script(running);
        let key = i64::try_from(key).unwrap_or(i64::MAX);
        Ok(crate::instance::build(&HANDLE, [Value::int(key)]))
    }
}

nvs_runtime::nvs_helper! {
    /// `await <handle>` — waits for the isolate the handle names and answers
    /// ADR 0006's `ScriptResult`.
    ///
    /// The receiver is **borrowed**, like every other `Core` argument: what is
    /// consumed is the table entry, not the handle object, so a handle awaited
    /// twice is an ordinary throw rather than a second wait on a child that has
    /// already been collected.
    fn nvs_core_script_await(ctx, args: [1]) {
        let handle = crate::instance::receiver(args[0], &HANDLE, "await")?;
        let key = crate::instance::slot(handle, PENDING).as_int().unwrap_or(0);
        let key = u64::try_from(key).unwrap_or(0);
        let Some(running) = ctx.take_started_script(key) else {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "`await` was given a handle a previous `await` already collected".to_owned(),
            ));
        };
        Ok(result_of(running.join(ctx)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Script::args(): mixed` — ADR 0012 § 6's replacement for `$_ARGS`,
    /// and the read half of `spawn script <path> with(args: …)`.
    ///
    /// The module doc is the one home of what this answers and why the type is
    /// `mixed`. What is here is only the accounting: the isolate's context
    /// holds one reference, that reference is the arena's rather than this
    /// call's, and a caller keeping the value needs one of its own.
    fn nvs_core_script_args(ctx, _args: [0]) {
        let held = ctx.isolate_argument();
        #[expect(
            unsafe_code,
            reason = "the value belongs to this isolate's ownership root, which \
                      outlives the call, so the reference handed back has to be \
                      a second one"
        )]
        // SAFETY: `Ctx::isolate_argument` borrows and hands over nothing; the
        // context keeps its own reference until the isolate's wholesale release.
        unsafe {
            held.retain();
        }
        Ok(held)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Script::onExit(callable $hook): void` — ADR 0127 § 1.
    ///
    /// The retain is the whole body's reason for existing, exactly as it is in
    /// `crate::fatal`'s two registrations: a helper's arguments are borrowed
    /// from the caller's frame and this one outlives the call by the whole of
    /// the script. What differs is on the other side of the boundary —
    /// `Ctx::push_exit_hook` appends where `Ctx::set_limit_handler` replaces,
    /// which is § 1's FIFO queue rather than a slot.
    fn nvs_core_script_on_exit(ctx, args: [1]) {
        // `crate::fatal`'s guard, for its reason: the row's one parameter is
        // `CoreTy::Callable`, so `E0401` refuses a `null` at the call and this
        // is unreachable from source — what it catches is a lowering bug
        // putting a value on the queue that the drain would then try to call.
        if args[0].tag() == Some(nvs_runtime::Tag::Null) {
            return Err(Fault::fatal(
                "Core\\Script::onExit expected a closure for its hook, got null".to_string(),
            ));
        }
        #[expect(
            unsafe_code,
            reason = "the argument is borrowed from the caller's frame and the \
                      context keeps it past this call, so it needs a reference \
                      of its own — which the queue then owns"
        )]
        // SAFETY: the value is owned by the caller's argument slot, which
        // outlives this call; the reference taken here is the one the context
        // releases when the queue drains or the request ends.
        unsafe {
            args[0].retain();
        }
        ctx.push_exit_hook(args[0]);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Script\ExitReport::reason(): Core\Script\ExitReason` — § 2's first
    /// column, as the enum integer the slot holds.
    fn nvs_core_script_exit_report_reason(_ctx, args: [1]) {
        slot_of(args, REASON_SLOT, "reason")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Script\ExitReport::status(): int` — § 2's second column, fixed
    /// before the first hook ran.
    fn nvs_core_script_exit_report_status(_ctx, args: [1]) {
        slot_of(args, STATUS_SLOT, "status")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Script\ExitReport::error(): ?Throwable` — § 2's third column, and
    /// the live object rather than a report built from it.
    fn nvs_core_script_exit_report_error(_ctx, args: [1]) {
        slot_of(args, ERROR_SLOT, "error")
    }
}

/// One [`Completion`] as the shape the language surface reads.
fn result_of(completion: Completion) -> Value {
    let Completion {
        ok,
        value,
        output,
        // ADR 0088 § 4's declaration is a *response's*, and `ScriptResult` is
        // not one: a `spawn script` answers with what the child wrote, and
        // what that output was declared to be is read by the connection that
        // is answering a peer or by nobody at all.
        content_type: _,
        // Spec § 15's status, ignored on the same reasoning: a `ScriptResult`
        // is not a response, so a child that set one said it to whoever is
        // answering a peer, which a `spawn script`'s collector is not.
        status: _,
        error,
    } = completion;
    let error = error.map_or_else(Value::null, |failure| {
        crate::instance::shape(
            FAILURE_SHAPE,
            [
                Value::str(NvsStr::new(failure.class.as_bytes())),
                Value::str(NvsStr::new(failure.message.as_bytes())),
            ],
        )
    });
    crate::instance::shape(
        RESULT_SHAPE,
        [
            error,
            Value::bool(ok),
            Value::str(NvsStr::new(&output)),
            value,
        ],
    )
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use nvs_runtime::{
        CLOSURE_ARITY_SLOT, CLOSURE_INVOKE, CLOSURE_PARAM_TAG_ANY, CLOSURE_PARAM_TAGS_SLOT,
        ClassTable, Ctx, ErrorClass, MethodRow, NvsFn, NvsObj, OK, Value, call,
    };

    use super::{ERROR_SLOT, EXIT_REPORT, REASON_SLOT, STATUS_SLOT, nvs_core_script_on_exit};

    thread_local! {
        /// What each hook saw, appended in the order the hooks ran — the only
        /// channel a plain `extern "C"` callback has, since a closure with no
        /// captured state has nowhere else to put it.
        static SEEN: RefCell<Vec<Saw>> = const { RefCell::new(Vec::new()) };
    }

    /// One hook's reading of the report it was handed.
    #[derive(Debug, PartialEq, Eq)]
    struct Saw {
        /// Which callback ran, so registration order is asserted rather than
        /// inferred from a count.
        who: &'static str,
        reason: i64,
        status: i64,
        /// The payload bits of the report's `error` slot — compared as an
        /// *identity* against the object the program threw, for the reason
        /// `crate::fatal`'s own test states.
        error: u64,
    }

    /// ADR 0127 § 2's first row, and § 1's FIFO: two hooks registered in order
    /// run in that order, once each, and are handed `Normal` with status `0`.
    #[test]
    fn on_exit_hooks_run_in_registration_order_at_a_clean_end() {
        let mut ctx = Ctx::buffered();
        let first = register(&mut ctx, 1, records_first);
        let second = register(&mut ctx, 1, records_second);
        assert_eq!(
            ctx.exit_hook_count(),
            2,
            "registering twice registers twice — this is a queue, not a slot"
        );

        SEEN.with(|seen| seen.borrow_mut().clear());
        super::run_exit_hooks(&mut ctx, Ok(()), None);

        assert_eq!(
            SEEN.with(|seen| seen.borrow().iter().map(|saw| saw.who).collect::<Vec<_>>()),
            ["first", "second"],
            "§ 1: hooks run FIFO in registration order"
        );
        assert_eq!(
            SEEN.with(|seen| (seen.borrow()[0].reason, seen.borrow()[0].status)),
            (super::NORMAL, 0),
            "§ 2's first row: the last statement ran, so `Normal` and status 0"
        );
        assert_eq!(
            SEEN.with(|seen| seen.borrow()[0].error),
            Value::null().bits(),
            "§ 2: only an uncaught throw carries an error"
        );

        // § 2's "once, at most once per script": a second ending finds an
        // already-drained queue and runs nothing, which is what keeps a report
        // that was fixed for one ending from being handed out at another.
        SEEN.with(|seen| seen.borrow_mut().clear());
        super::run_exit_hooks(&mut ctx, Ok(()), None);
        assert!(
            SEEN.with(|seen| seen.borrow().is_empty()),
            "the queue drains once"
        );
        assert_eq!(
            ctx.exit_hook_count(),
            0,
            "and gives its references back as it goes"
        );
        release(first);
        release(second);
    }

    /// ADR 0127 § 2's second row — the ending that exists to be observed: `exit`
    /// runs no `finally`, and this queue is the only thing that sees it. The
    /// status is the one the call named rather than a zero.
    #[test]
    fn an_exit_call_still_drains_the_on_exit_queue() {
        let mut ctx = Ctx::buffered();
        let hook = register(&mut ctx, 1, records_first);
        ctx.set_exit_code(3);

        SEEN.with(|seen| seen.borrow_mut().clear());
        super::run_exit_hooks(&mut ctx, Err(nvs_runtime::EXITED), None);

        assert_eq!(
            SEEN.with(|seen| (seen.borrow()[0].reason, seen.borrow()[0].status)),
            (super::EXIT_CALL, 3),
            "§ 2: `exit($n)` is `ExitCall` carrying `$n`, not a `Normal` end"
        );
        release(hook);
    }

    /// ADR 0127 § 2's third row, asked as an **identity** exactly as ADR 0020
    /// § 2's tier-2 handler is: the report carries the very allocation the
    /// program threw, so a hook can read its class, message and backtrace back
    /// through the ordinary members rather than a copy of what it said.
    #[test]
    fn an_uncaught_throw_reaches_the_queue_with_the_error() {
        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut classes = ClassTable::new();
        let root = classes.define("RuntimeError", &SLOTS, &[]);
        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::rc::Rc::new(classes), root));
        ctx.set_pending("the store said no");
        ctx.push_frame("Main::main");
        let thrown = ctx.take_thrown();
        assert!(
            !thrown.is_none(),
            "an installed class is what promotes a bare failure to an object"
        );
        let hook = register(&mut ctx, 1, records_first);

        SEEN.with(|seen| seen.borrow_mut().clear());
        super::run_exit_hooks(&mut ctx, Err(nvs_runtime::THROWN), Some(&thrown));

        assert_eq!(
            SEEN.with(|seen| (seen.borrow()[0].reason, seen.borrow()[0].status)),
            (super::UNCAUGHT_THROW, 1),
            "§ 2: an uncaught throw is `UncaughtThrow` at status 1"
        );
        assert_eq!(
            SEEN.with(|seen| seen.borrow()[0].error),
            thrown.as_value().bits(),
            "§ 1: the report carries the real `Throwable`, not a record built \
             from it"
        );
        // The report borrowed the exception rather than consuming it: this
        // frame still owns a reference, and the object is still readable after
        // the drain that was handed it.
        assert_eq!(thrown.message(), "the store said no");
        assert_eq!(thrown.class_name(), "RuntimeError");
        release(hook);
    }

    /// ADR 0127 § 3's first termination: a resource-limit breach runs **no**
    /// hook, and `Core\Fatal::onLimit` on ADR 0020 § 1's reserved slice stays
    /// its one observer.
    ///
    /// Asserted at [`super::run_exit_hooks`] because that is the one door
    /// between an ending and a hook — every caller hands it whatever the script
    /// frame answered, so a `FATAL` that ran a hook could only get there
    /// through this function.
    #[test]
    fn a_limit_fatal_runs_no_on_exit_hook() {
        let mut ctx = Ctx::buffered();
        let hook = register(&mut ctx, 1, records_first);

        SEEN.with(|seen| seen.borrow_mut().clear());
        super::run_exit_hooks(&mut ctx, Err(nvs_runtime::FATAL), None);

        assert!(
            SEEN.with(|seen| seen.borrow().is_empty()),
            "§ 3: a `FATAL` fires no hook"
        );
        assert!(
            !ctx.exit_hooks_drained(),
            "and does not spend the one drain either — a `FATAL` is not an \
             ending this queue has a report for"
        );
        assert_eq!(
            ctx.exit_hook_count(),
            1,
            "the registration is still held, and the request's own teardown is \
             what releases it"
        );
        release(hook);
    }

    /// ADR 0127 § 5: a hook that throws is abandoned where it stands and the
    /// queue *continues*, which is the half a first implementation gets wrong —
    /// one bad hook must not silence every hook behind it.
    #[test]
    fn a_hook_that_throws_is_abandoned_and_the_queue_continues() {
        let mut ctx = Ctx::buffered();
        let bad = register(&mut ctx, 1, throws);
        let good = register(&mut ctx, 1, records_second);

        SEEN.with(|seen| seen.borrow_mut().clear());
        super::run_exit_hooks(&mut ctx, Ok(()), None);

        assert_eq!(
            SEEN.with(|seen| seen.borrow().iter().map(|saw| saw.who).collect::<Vec<_>>()),
            ["throws", "second"],
            "§ 5: the throw is logged and abandoned, and the hook behind it runs"
        );
        assert!(
            ctx.take_thrown().is_none(),
            "the failure is reported through the floor rather than left pending: \
             the ending a hook throws during is still the ending that was fixed \
             before it ran"
        );
        release(bad);
        release(good);
    }

    /// Registers one callback as a hook, through the member itself, and answers
    /// the reference this frame keeps.
    ///
    /// Driving `Core\Script::onExit` rather than `Ctx::push_exit_hook` is what
    /// makes these tests cover the boundary as well as the queue: the member
    /// takes a reference of its own, so the caller still owns what it passed.
    fn register(ctx: &mut Ctx, arity: usize, invoke: NvsFn) -> Value {
        let hook = closure_of(arity, invoke);
        call(nvs_core_script_on_exit, ctx, &[hook])
            .expect("registering answers `void` and cannot fail");
        hook
    }

    /// Reads the report in slot 1 and appends what it says under `who`.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes exactly two live values, each retained \
                  for this callee to release, and the report is an object this \
                  crate laid out"
    )]
    unsafe fn record(who: &'static str, args: *const Value, out: *mut Value) -> i32 {
        // Slot 0 is the closure itself and slot 1 its one parameter, which is
        // the report — `nvs_runtime::call_closure` builds the frame that way
        // for a compiled callee and for this one alike.
        let report = unsafe { *args.add(1) };
        let object = report
            .obj_ptr()
            .expect("a hook is handed the report object");
        SEEN.with(|seen| {
            seen.borrow_mut().push(Saw {
                who,
                reason: crate::instance::slot(object, REASON_SLOT)
                    .as_int()
                    .expect("the reason slot holds an enum integer"),
                status: crate::instance::slot(object, STATUS_SLOT)
                    .as_int()
                    .expect("the status slot holds an int"),
                error: crate::instance::slot(object, ERROR_SLOT).bits(),
            });
        });
        for index in 0..2 {
            release(unsafe { *args.add(index) });
        }
        unsafe {
            *out = Value::null();
        }
        OK
    }

    /// The first hook, and the one every single-hook case registers.
    #[expect(
        unsafe_code,
        reason = "the signature compiled code is called through cannot express \
                  that its two pointers are live for the call"
    )]
    unsafe extern "C" fn records_first(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        unsafe { record("first", args, out) }
    }

    /// The second hook, so registration order is read off names.
    #[expect(unsafe_code, reason = "[`records_first`]'s reason, on its twin")]
    unsafe extern "C" fn records_second(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        unsafe { record("second", args, out) }
    }

    /// A hook that records that it ran and then throws, for § 5.
    #[expect(
        unsafe_code,
        reason = "[`records_first`]'s reason, plus a context pointer the ABI \
                  guarantees is the live request's"
    )]
    unsafe extern "C" fn throws(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        unsafe { record("throws", args, out) };
        let ctx = unsafe { &mut *ctx };
        ctx.set_pending("the hook itself failed");
        nvs_runtime::THROWN
    }

    /// A closure value whose `invoke` is a plain Rust function —
    /// `crates/nvs-stdlib/tests/allocation_policy.rs`'s `closure_of`, whose doc
    /// comment is the home of why this is a whole closure. The table is leaked
    /// because a descriptor's *address* is its identity.
    fn closure_of(arity: usize, invoke: NvsFn) -> Value {
        let mut table = ClassTable::new();
        let id = table.define("{closure}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![MethodRow {
                name: CLOSURE_INVOKE.to_owned(),
                code: invoke as *const u8,
                arity: 0,
                param_tags: 0,
                public: true,
                native: false,
            }],
        );
        let table: &'static ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives \
                      every instance made from it — `NvsObj::new`'s whole \
                      obligation"
        )]
        let object = unsafe { NvsObj::new(table.desc(id)) };
        object.set_field(
            CLOSURE_ARITY_SLOT,
            Value::int(i64::try_from(arity).expect("a small arity")),
        );
        let mut tags: u64 = 0;
        for parameter in 0..arity {
            tags |= u64::from(CLOSURE_PARAM_TAG_ANY) << (parameter * 4);
        }
        object.set_field(
            CLOSURE_PARAM_TAGS_SLOT,
            Value::int(i64::from_ne_bytes(tags.to_ne_bytes())),
        );
        Value::object(object)
    }

    /// One reference, given back through the ABI's own entry point.
    #[expect(
        unsafe_code,
        reason = "every value released here is one this test or its callee \
                  owns a reference to"
    )]
    fn release(value: Value) {
        unsafe {
            nvs_runtime::nvs_value_release(u64::from(value.tag_byte()), value.bits());
        }
    }

    /// The layout the report is built against is the one its accessors read —
    /// asserted here rather than trusted, since the slot constants and the
    /// `slots` roster are two statements of one fact.
    #[test]
    fn the_reports_slots_are_the_layout_its_accessors_read() {
        assert_eq!(EXIT_REPORT.slot("reason"), REASON_SLOT);
        assert_eq!(EXIT_REPORT.slot("status"), STATUS_SLOT);
        assert_eq!(EXIT_REPORT.slot("error"), ERROR_SLOT);
    }
}
