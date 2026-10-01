//! `Core\Script\Handle` — what `spawn script` hands back, and the one `Core`
//! class registered so that a program can name a value it may not touch.
//!
//! `rule:security/isolate-shares-nothing`'s spawn
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
//! (`rule:concurrency/cancellation-runs-no-user-code`
//! ), and every "is it finished yet" answer is stale before the caller reads
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
//! `rule:observability/script-on-exit`'s
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
//! spend `rule:observability/script-on-exit` states — and one report allocation per ending, released
//! the moment the drain is over. `Ctx::exit_hooks` is the home of the first
//! half.
//!
//! `Core\Script` itself lands in this module beside its handle, the way
//! `Core\Task` and `Core\Task\Channel<T>` already sit together under
//! `rule:classes/no-free-functions-or-constants`.
//! [`CLASS`] is that class; `args()` is its one row so far, and the
//! `valueOrThrow($result)` that a shape cannot carry as a method is still owed.
//!
//! # `args()` answers `mixed`, and `null` for a script nobody spawned
//!
//! `rule:core-classes/script-args` is what
//! replaced `rule:security/isolate-shares-nothing`'s `$_ARGS` with a method call, and its body states the
//! return type this module implements: **`mixed`**, not an array of anything.
//! `spawn script`'s `args:` is checked against no expected type at all —
//! `nvs_types::expr::isolate`'s `check_spawn_script` says why, and it is
//! `rule:classes/graph-copy`'s rule that whether a graph may cross is a run-time question —
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

use nvs_config::capability::{Cap, Scope};
use nvs_runtime::Tag;
use nvs_runtime::host::{Completion, Entry, Narrowing, Output, Placement, StartError};
use nvs_runtime::script::ResolveError;
use nvs_runtime::{Fault, NvsObj, NvsStr, ThrownClass, Value};

use crate::registry::{
    CaseDoc, ClassDoc, CoreClass, CoreMethod, CoreTy, EnumDoc, MethodDoc, ParamDoc,
};

/// The handle class's fully-qualified name, as
/// [`CoreTy::Instance`] spells it.
///
/// `pub` because `nvs-types` names this class when it types a `spawn script`
/// expression, and a name written out in two crates is a name that can drift.
pub const HANDLE_NAME: &str = r"Core\Script\Handle";

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const HANDLE: CoreClass = CoreClass {
    name: HANDLE_NAME,
    doc: Some(&HANDLE_CARD),
    methods: &[],
    instance: &[],
    slots: &["pending"],
    constants: &[],
};

/// `Core\Script\Handle`'s class card — `rule:core-api/reference-card`.
const HANDLE_CARD: ClassDoc = ClassDoc {
    short: "A script that `spawn script` started. The only thing you can do with it is `await` \
            it, which waits until that script ends.",
};

/// [`HANDLE`]'s one slot: the key of the started isolate this handle names, or
/// `0` for a spawn that never started one.
const PENDING: usize = 0;

/// `Core\Script`'s registry row — the class `rule:core-classes/script-args` named when it
/// replaced `$_ARGS` with a method call. See [`crate::registry::CLASSES`].
///
/// A class in its own right beside `Core\Script\Handle`, exactly as
/// `Core\Time` sits beside `Core\Time\Instant`: the handle is what a spawn
/// answers with, and this is what the child asks.
///
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Script",
    doc: Some(&CARD),
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
            params: &[CoreTy::CallableSig(
                &[CoreTy::Instance(EXIT_REPORT_NAME)],
                &CoreTy::Mixed,
            )],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: ON_EXIT_SYMBOL,
            doc: Some(&ON_EXIT_DOC),
        },
        CoreMethod {
            name: "finish",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: FINISH_SYMBOL,
            doc: Some(&FINISH_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Script`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "The script that is running now. `args()` returns the value the script was started \
            with. `onExit()` adds a function that runs when the script ends, and `finish()` ends \
            the script at once.",
};

/// `Core\Script::onExit`'s reference card — `rule:core-api/reference-card`.
const ON_EXIT_DOC: MethodDoc = MethodDoc {
    short: "Adds a function that runs when this script ends. It runs at a normal end, after \
            `exit`, after `Core\\Script::finish()` and after an exception that nothing caught. \
            The functions run once each, in the order they were added. A `FATAL` error and a \
            cancelled script run none of them.",
    params: &[ParamDoc {
        name: "hook",
        desc: "The function to run. It receives one `Core\\Script\\ExitReport` that describes \
               the ending, and its return value is not used. It may also take no parameter. If \
               it throws an error, the error is logged and the next function still runs.",
        shape: &[],
    }],
    ret: "Nothing. The list belongs to this request only. Adding the same function twice runs it \
          twice. A function added while the list runs is added to the end of the same list. A \
          function cannot change the ending, because the report is fixed before the first one \
          runs. `exit` or `Core\\Script::finish()` inside one of these functions stops that \
          function, and the error is logged as a `RuntimeError`.",
    errors: &[],
};

/// `Core\Script::finish`'s reference card — `rule:core-api/reference-card`.
const FINISH_DOC: MethodDoc = MethodDoc {
    short: "Ends this script at once. Every `finally` block between the call and the top of the \
            script runs first. Then the functions added with `Core\\Script::onExit()` run. A \
            request handler calls it when its response is complete.",
    params: &[],
    ret: "Nothing. The call does not return, no code after it runs, and no `catch` block \
          catches it. The `Core\\Script\\ExitReport` has the reason `Finish`, the status `0` and \
          no error. On a server, the response that the handler set is sent, and work added with \
          `Core\\Task::afterResponse` still runs. Under `nvs run`, the script ends.",
    errors: &[],
};

/// `Core\Script\ExitReason`'s fully-qualified name — `rule:observability/script-on-exit`.
///
pub(crate) const EXIT_REASON_NAME: &str = r"Core\Script\ExitReason";

/// The three endings that fire the queue, as the report's own field — `rule:observability/three-endings-fire-the-exit-queue`
/// 's table, in its order. See [`crate::registry::ENUMS`].
///
/// `ExitCall` carries the suffix `rule:observability/script-on-exit` names: a case spelled `Exit`
/// would share a spelling with the keyword, and a program writing
/// `ExitReason::Exit` beside an `exit;` two lines down is reading one word two
/// ways. There is deliberately no case for a `FATAL` or a cancellation — § 3
/// makes those the two terminations no hook observes, so a case for either
/// would be a value nothing can ever produce.
pub(crate) const EXIT_REASON: crate::registry::CoreEnum = crate::registry::CoreEnum {
    name: EXIT_REASON_NAME,
    cases: &[
        ("Normal", 0),
        ("ExitCall", 1),
        ("UncaughtThrow", 2),
        ("Finish", 3),
    ],
    doc: Some(&EXIT_REASON_DOC),
};

/// [`EXIT_REASON`]'s reference card — `rule:core-api/reference-card`.
const EXIT_REASON_DOC: EnumDoc = EnumDoc {
    short: "How a script ended, as `Core\\Script\\ExitReport::reason()` returns it. A `FATAL` \
            error and a cancelled script have no case, because they run no exit function.",
    cases: &[
        CaseDoc {
            name: "Normal",
            desc: "The last top-level statement ran.",
        },
        CaseDoc {
            name: "ExitCall",
            desc: "`exit`, `exit($n)` or `exit(\"msg\")` ended the script. No `finally` block \
                   runs after `exit`.",
        },
        CaseDoc {
            name: "UncaughtThrow",
            desc: "An exception reached the top of the script and nothing caught it. \
                   `Core\\Script\\ExitReport::error()` returns that exception.",
        },
        CaseDoc {
            name: "Finish",
            desc: "`Core\\Script::finish()` ended the script. Every `finally` block runs, and no \
                   `catch` block catches it. The status is `0` and there is no error.",
        },
    ],
};

/// The `Core`-owned class of `rule:observability/script-on-exit`'s report, and the report's own name.
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

/// `rule:observability/script-on-exit`'s report, as the instance each hook is handed.
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
    doc: Some(&EXIT_REPORT_CARD),
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
            // `rule:core-api/shape-rules` R4's "absence is `?T`": three of the four endings have no
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

/// `Core\Script\ExitReport`'s class card — `rule:core-api/reference-card`.
const EXIT_REPORT_CARD: ClassDoc = ClassDoc {
    short: "Describes how a script ended. Each function added with `Core\\Script::onExit()` \
            receives one. `reason()` returns the kind of ending, `status()` returns the exit \
            status, and `error()` returns the exception that ended the script, if there was one. \
            The values cannot change.",
};

/// `Core\Script\ExitReport::reason`'s reference card — `rule:core-api/reference-card`.
const REASON_DOC: MethodDoc = MethodDoc {
    short: "Returns how the script ended, as a `Core\\Script\\ExitReason` case.",
    params: &[],
    ret: "`Normal` when the last statement ran, `ExitCall` after `exit`, `UncaughtThrow` after \
          an exception that nothing caught, and `Finish` after `Core\\Script::finish()`.",
    errors: &[],
};

/// `Core\Script\ExitReport::status`'s reference card — `rule:core-api/reference-card`.
const STATUS_DOC: MethodDoc = MethodDoc {
    short: "Returns the status that the process exits with.",
    params: &[],
    ret: "`0` for a normal end and after `Core\\Script::finish()`. After `exit($n)`, it is `$n`. \
          After an exception that nothing caught, it is `1`. The status is fixed before the \
          first function runs.",
    errors: &[],
};

/// `Core\Script\ExitReport::error`'s reference card — `rule:core-api/reference-card`.
const ERROR_DOC: MethodDoc = MethodDoc {
    short: "Returns the exception that ended the script, if there was one.",
    params: &[],
    ret: "When the reason is `UncaughtThrow`, the `Throwable` that nothing caught, with its own \
          class, message and backtrace. For every other ending, `null`.",
    errors: &[],
};

/// `Core\Script::args`'s reference card — `rule:core-api/reference-card`.
const ARGS_DOC: MethodDoc = MethodDoc {
    short: "Returns the value that this script was started with. The parent script writes it as \
            `spawn script … with(args: …)`, and this script receives a copy.",
    params: &[],
    ret: "The value the parent passed, with the same shape. It is `null` when the parent gave no \
          `args:`, and in the first script, which no other script started.",
    errors: &[],
};

/// The symbol [`CLASS`]'s `args` row is reached through.
const ARGS_SYMBOL: &str = "nvs_core_script_args";

/// The symbol [`CLASS`]'s `onExit` row is reached through.
const ON_EXIT_SYMBOL: &str = "nvs_core_script_on_exit";

/// The symbol [`CLASS`]'s `finish` row carries, which `nvs_ir::lower` reads to
/// recognise the call and no compiled program ever reaches.
///
/// `pub` for that reason, exactly as [`SPAWN_SYMBOL`] and [`AWAIT_SYMBOL`] are:
/// `nvs_types` re-exports it as `CORE_SCRIPT_FINISH` and the lowering matches
/// on it. The row and its address exist because a registered member is resolved
/// through a symbol and `every_registered_member_has_an_implementation_address`
/// holds every row to one — see [`nvs_core_script_finish`] for what happens if
/// the lowering ever stops intercepting the call.
pub const FINISH_SYMBOL: &str = "nvs_core_script_finish";

/// The class `Core\Script::finish` raises, and the one place a class name is
/// compared against it — both of them `nvs_runtime::throwable`'s, re-exported
/// here because this module is the spelling every caller outside a host uses.
///
/// **The home is that crate rather than this one** because `nvs-host` has to
/// classify a served request's ending and cannot name `nvs-stdlib`: this crate
/// depends on `nvs-host` for `Core\Http\Client`'s socket, so the edge back would
/// close a cycle. What is restated where, and against which test, is
/// [`is_finish`]'s own doc and [`THROWABLE`]'s.
pub use nvs_runtime::{FINISH_MARKER_NAME, is_finish};

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

/// The symbol `spawn script Class::method with(…)` lowers to — ADR 0006
/// § *Decision*'s second entry form.
///
/// A symbol of its own rather than a fourth argument to [`SPAWN_SYMBOL`]'s:
/// the two forms differ in what the first argument *means* and in nothing a
/// caller writes, so a flag argument would be read by this module and by
/// nobody else, and every dump of the IR would carry a constant whose only
/// job is to pick a branch. `nvs_ir::lower`'s `lower_spawn_script` decides
/// between them syntactically, exactly as `nvs_types` decided the operand
/// rule.
pub const SPAWN_METHOD_SYMBOL: &str = "nvs_core_script_spawn_method";

/// The symbol `await <handle>` lowers to.
pub const AWAIT_SYMBOL: &str = "nvs_core_script_await";

/// `rule:security/isolate-shares-nothing`'s `ScriptResult`, as a descriptor name — see
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
        SPAWN_METHOD_SYMBOL => (nvs_core_script_spawn_method as *const ()).cast(),
        AWAIT_SYMBOL => (nvs_core_script_await as *const ()).cast(),
        ARGS_SYMBOL => (nvs_core_script_args as *const ()).cast(),
        ON_EXIT_SYMBOL => (nvs_core_script_on_exit as *const ()).cast(),
        FINISH_SYMBOL => (nvs_core_script_finish as *const ()).cast(),
        REASON_SYMBOL => (nvs_core_script_exit_report_reason as *const ()).cast(),
        STATUS_SYMBOL => (nvs_core_script_exit_report_status as *const ()).cast(),
        ERROR_SYMBOL => (nvs_core_script_exit_report_error as *const ()).cast(),
        _ => return None,
    })
}

/// One ending, as the report `rule:observability/three-endings-fire-the-exit-queue`'s table describes it.
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

/// Runs `rule:observability/script-on-exit`'s
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
/// - `Err(THROWN)` carrying the marker [`is_finish`] names — `Core\Script::
///   finish()`: `Finish`, status `0`, no error. It arrives on the throw path
///   because that is the path every `finally` lives on, and it is an *ordinary*
///   ending all the same, which is why it is read before the arm below.
/// - `Err(THROWN)` — an uncaught throw: `UncaughtThrow`, status `1`, and the
///   live `Throwable`.
/// - **anything else** — a `FATAL`, which runs no hook at all. § 3 is the
///   argument: a resource-limit breach stopped the script *for exceeding its
///   budget*, and `Core\Fatal::onLimit` on `rule:errors/on-limit`'s reserved slice stays
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
        Err(status)
            if status == nvs_runtime::THROWN
                && thrown.is_some_and(|thrown| is_finish(&thrown.class_name())) =>
        {
            report_of(FINISH, 0, None)
        }
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
const FINISH: i64 = 3;

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
/// Refused rather than defaulted for an unknown spelling: `rule:security/isolate-shares-nothing` names two
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

/// Decodes the `on:` option, which arrives as the word the program wrote.
///
/// Fatal for anything else, where [`output_of`]'s unknown spelling is a throw,
/// and the difference is who can produce one: that option's set is open to a
/// program's misspelling and this one is not.
fn placement_of(value: &Value) -> Result<Placement, Fault> {
    match value.as_text() {
        Some("here") => Ok(Placement::Here),
        Some("worker") => Ok(Placement::Worker),
        // Unreachable from source: `nvs_types::expr::isolate`'s
        // `check_placement` refuses every spelling but these two where it is
        // written (`E_SPAWN_PLACEMENT_UNKNOWN`), and the lowering writes
        // `'here'` for a spawn that named no placement — so a third word here
        // is the compiler disagreeing with itself rather than a mistake a
        // program could make.
        Some(other) => Err(Fault::fatal(format!(
            "`spawn script`'s `on:` is `'here'` or `'worker'`, got `'{other}'`"
        ))),
        // Unreachable from source for the same reason one step earlier:
        // `check_placement` refuses a value that is not a string literal at all
        // (`E_SPAWN_PLACEMENT_UNKNOWN`), so what the lowering carries is always
        // one of the two words.
        None => Err(Fault::fatal(format!(
            "`spawn script`'s `on:` expected a string, got tag {}",
            value.tag_byte()
        ))),
    }
}

/// Decodes the two narrowing options into the one value the seam takes —
/// `rule:security/isolate-budget-is-the-trees`' sub-cap and
/// `rule:security/isolate-shares-nothing`'s grant list.
///
/// Both arrive as the values the program wrote, and `null` for an option it did
/// not write. One function because both are read at the same two call sites and
/// a spawn narrowed by neither is then one default rather than two.
///
/// # Errors
///
/// A fatal for a value whose shape the type checker already refused, which is
/// what makes the two readers below total.
fn narrowing_of(limits: &Value, grants: &Value) -> Result<Narrowing, Fault> {
    Ok(Narrowing {
        limits: limits_of(limits)?,
        grants: grants_of(grants)?,
    })
}

/// Decodes `limits:` — the `[limits]` keys the spawn site narrowed, paired with
/// the values written beside them.
///
/// The values stay **text**, in the spelling the same directive takes in
/// `nvs.toml`, because what applies them is the child's own configuration
/// overlay and that overlay already refuses one wider than what is in force
/// (`nvs_config::Request::set`). A count is rendered rather than parsed here for
/// the same reason: the number the child is held to is read by one parser, not
/// by this one and then again by that one.
fn limits_of(value: &Value) -> Result<Vec<(String, String)>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(Vec::new());
    }
    let ptr = value.obj_ptr().ok_or_else(|| {
        // Unreachable from source: `nvs_types::expr::isolate`'s `sub_caps`
        // checks this option against a shape and `reject_unknown_sub_cap`
        // refuses a key that is not a sub-cap, so a value that is neither a
        // shape nor absent is `E_TYPE_MISMATCH` where it is written.
        Fault::fatal(format!(
            "`spawn script`'s `limits:` expected a shape, got tag {}",
            value.tag_byte()
        ))
    })?;
    // A shape is an object with one slot per field it names — `crate::instance`'s
    // `shape` owns that layout — so the slots a spawn site wrote *are* the
    // sub-caps it narrowed, and the walk needs no second copy of the sub-cap
    // list on this side of the seam.
    #[expect(
        unsafe_code,
        reason = "the value owns a reference to a live allocation, so it is live \
                  for this borrow; the handle is never dropped, so the reference \
                  is not released twice"
    )]
    let shape = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(ptr) });
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by the unit's class table, which \
                  outlives every instance of the class it describes"
    )]
    let desc = unsafe { &*shape.class() };
    let mut caps = Vec::new();
    for slot in 0..shape.field_count() {
        // Unreachable from source: a descriptor names every slot it has, and
        // the count above is that descriptor's own.
        let Some(key) = desc.field_name(slot) else {
            continue;
        };
        let entry = shape.field(slot);
        let written = match (entry.as_text(), entry.as_int()) {
            (Some(text), _) => text.to_owned(),
            (None, Some(count)) => count.to_string(),
            // Unreachable from source: every field of `sub_caps`' shape is a
            // `string` or an `int`, so a third tag is `E_TYPE_MISMATCH` at the
            // written key.
            (None, None) => {
                return Err(Fault::fatal(format!(
                    "`spawn script`'s `limits: {key}` expected a string or an int, got tag {}",
                    entry.tag_byte()
                )));
            }
        };
        caps.push((key.to_owned(), written));
    }
    Ok(caps)
}

/// Decodes `grants:` — the capability names the child may still ask for, and
/// `None` for a spawn that named none.
///
/// The distinction the `Option` carries is the whole of what this reader
/// decides: an empty list is a child that may ask for nothing, and no list at
/// all is the parent's own set unchanged. The names are not checked against
/// [`Cap::parse`] here — a name no capability has grants nothing, and dropping
/// it from a list that only ever narrows fails closed.
fn grants_of(value: &Value) -> Result<Option<Vec<String>>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let array = value.array_ptr().ok_or_else(|| {
        // Unreachable from source: `nvs_types::expr::isolate` checks this
        // option against `array<string>`, so a value that is neither an array
        // nor absent is `E_TYPE_MISMATCH` where it is written.
        Fault::fatal(format!(
            "`spawn script`'s `grants:` expected an array, got tag {}",
            value.tag_byte()
        ))
    })?;
    let mut names = Vec::new();
    let mut from = 0_usize;
    loop {
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live allocation, \
                      so it is live for the length of this call, and `from` only \
                      ever advances past a slot this same cursor reported"
        )]
        let (slot, entry) = unsafe {
            let slot = nvs_runtime::nvs_array_next_slot(array, from);
            let Ok(slot) = usize::try_from(slot) else {
                break;
            };
            let mut entry = Value::null();
            nvs_runtime::nvs_array_value_at(array, slot, &raw mut entry);
            (slot, entry)
        };
        from = slot + 1;
        let name = entry.as_text().ok_or_else(|| {
            // Unreachable from source: the option is checked against
            // `array<string>`, so a non-string element is `E_TYPE_MISMATCH` at
            // the written element.
            Fault::fatal(format!(
                "`spawn script`'s `grants:` expected a string name, got tag {}",
                entry.tag_byte()
            ))
        })?;
        names.push(name.to_owned());
    }
    Ok(Some(names))
}

nvs_runtime::nvs_helper! {
    /// `spawn script <path> with(args: …, output: …)` — starts the isolate and
    /// answers the handle that `await` collects.
    ///
    /// Six arguments in the order the lowering builds them: the path as
    /// written, the `args:` value (`null` when the option was not given), the
    /// `output:` spelling, the `on:` placement, which is `'here'` for a spawn
    /// that named none, and the `limits:` and `grants:` narrowings, each `null`
    /// for a spawn that wrote neither. The argument is **transferred** to the
    /// isolate, which is why the lowering hands it over rather than borrowing it
    /// as an ordinary `Core` call would; the two narrowings are borrowed,
    /// because what crosses is what [`narrowing_of`] reads out of them.
    ///
    /// A path that does not resolve is a **throw in the parent**, not an
    /// `ok = false`: `nvs_runtime::script::ResolveError` is one step before the
    /// boundary and that module's doc says so, and the same is true of an
    /// argument that cannot cross — it was built by the parent before any child
    /// existed. What the boundary turns into a value is only what the *child*
    /// produced.
    ///
    /// **`script.spawn` is checked inside `resolve`**, not here — `rule:security/capability-check-at-the-door`
    /// . A spawn the parent was never allowed to attempt therefore never
    /// reaches a child at all, which is what makes it a value on *this* side
    /// while a child that fails on its own is an `ok = false` on the other.
    fn nvs_core_script_spawn(ctx, args: [6]) {
        // Unreachable from source: the path expression is checked against
        // `string` by `nvs_types::expr::isolate`'s `check_spawn_script`, so a
        // non-string is `E_TYPE_MISMATCH` where it is written.
        let path = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "`spawn script` expected a string path, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        // `rule:programs/path-literals-resolve-from-their-file`: a relative
        // literal arrives joined to its file's folder, so a relative path here
        // was built while the program ran, and nothing resolves it from the
        // working directory.
        if let Some(message) =
            nvs_runtime::capability::relative(std::path::Path::new(path), "`spawn script`")
        {
            return Err(Fault::thrown_as(ThrownClass::Runtime, message));
        }
        let output = output_of(&args[2])?;
        let placement = placement_of(&args[3])?;
        let narrowing = narrowing_of(&args[4], &args[5])?;
        // The argument is handed over here: one reference goes to the isolate
        // and the lowering emitted no release for it.
        let crossing = args[1];
        // The **path**, not a program: `nvs_runtime::host::Entry`'s doc owns why
        // the name crosses the seam and the closure cannot, and the resolve —
        // with `rule:security/capability-check-at-the-door`'s `script.spawn`
        // door inside it — happens on the core that is going to run the child.
        let started = nvs_runtime::host::with_current(|host| {
            host.start_isolate(
                ctx,
                Entry::Path(path.to_owned()),
                crossing,
                output,
                placement,
                narrowing,
            )
        });
        let running = match started {
            // A spawn past `[limits] max_script_depth` is refused by
            // `Isolate::start` before it builds anything, and the refusal is
            // recorded on *this* context rather than answered as an error:
            // the ceiling is the tree's and the error below is the argument's,
            // and `rule:errors/on-limit` makes a limit breach a `FATAL` no `catch` sees
            // while that error is a throw. `Fault::Pending` is how a helper
            // says the status is already on the context; the handle is dropped
            // unjoined, which `host::Running`'s own doc allows.
            Some(Ok(_)) if ctx.pending().is_some() => {
                return Err(Fault::Pending(nvs_runtime::FATAL));
            }
            Some(Ok(running)) => running,
            // An embedder that installed no resolver. Not the program's
            // mistake, and not something a `catch` should be able to paper over.
            Some(Err(StartError::Entry(ResolveError::NoResolver))) => {
                return Err(Fault::fatal(format!(
                    "`spawn script '{path}'` needs a script resolver on this thread and there is none"
                )));
            }
            Some(Err(StartError::Entry(ResolveError::Refused(message)))) => {
                return Err(Fault::thrown_as(
                    ThrownClass::Runtime,
                    format!("`spawn script '{path}'`: {message}"),
                ));
            }
            // Already a whole sentence naming the capability and the path —
            // `rule:security/denial-is-a-runtime-error` — so it is thrown as written rather than wrapped in
            // this member's own framing, which would say `spawn script` twice.
            Some(Err(StartError::Entry(ResolveError::Denied(message)))) => {
                return Err(Fault::thrown_as(ThrownClass::Runtime, message));
            }
            Some(Err(StartError::Argument(error))) => {
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

/// ADR 0006 § *Decision*'s named-argument agreement between a method entry's
/// parameters and the `args:` map it was spawned with: every parameter the
/// entry declares is a key of the map, and the map holds no key the entry does
/// not declare. `Err` carries the message the throw is worded with.
///
/// `site` is the whole spelling a refusal names — `spawn script Chat::run` for
/// the construct, `Core\Socket::upgrade` for
/// `rule:concurrency/an-upgrade-is-spawn-shaped`'s
/// door — because the rule is `rule:security/isolate-shares-nothing`'s and the two entry forms it governs
/// are written at three sites now. `crate::socket` is the other caller and the
/// one home of why a connection asks this question here rather than in the
/// child.
///
/// Judged on the **parent's** map rather than on the child's copy, because the
/// two hold the same keys — `rule:classes/two-copy-depths`'s graph copy preserves them — and only
/// this side still has a frame for the ADR's "reported … at the spawn" to
/// happen in. The other half of that sentence, reporting a *literal* map's
/// mismatch at compile time, is `nvs_types::expr::isolate`'s one known gap.
///
/// A value that is not an array at all names no parameter, so it reads here as
/// a map with no keys: `null` is what a spawn with no `args:` passes, and
/// anything else is a value only `Core\Script::args()` could have wanted.
pub(crate) fn entry_names_agree(site: &str, names: &[String], map: Value) -> Result<(), String> {
    let keys = match map.array_ptr() {
        Some(ptr) => crate::arr::borrowed(ptr).keys(),
        None => Vec::new(),
    };
    let missing: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|name| !keys.iter().any(|key| key.as_slice() == name.as_bytes()))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "`{site}`: `args:` has no entry for parameter(s) `{}`",
            missing.join("`, `")
        ));
    }
    let unknown: Vec<String> = keys
        .iter()
        .filter(|key| !names.iter().any(|name| name.as_bytes() == key.as_slice()))
        .map(|key| String::from_utf8_lossy(key).into_owned())
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "`{site}`: `args:` holds `{}`, which the entry does not declare",
            unknown.join("`, `")
        ));
    }
    Ok(())
}

nvs_runtime::nvs_helper! {
    /// `spawn script Class::method with(output: …)` — ADR 0006 § *Decision*'s
    /// **method entry**, started as a fresh isolate over the unit this context
    /// is already running.
    ///
    /// [`nvs_core_script_spawn`]'s arguments in its order, with its ownership
    /// rules, plus a last one this form alone takes; and one
    /// difference: argument 0 is a **constant label**
    /// the lowering wrote — `Class::method`, resolved by `nvs_types` at the
    /// spawn site — rather than a path the program computed. Nothing is
    /// resolved and no unit is compiled, because the class is in the unit
    /// already running: `nvs_runtime::script`'s module doc is the one home of
    /// why this form never reaches the `Resolver` seam, and of why the
    /// capability is asked here with `Scope::Unscoped` where the path form asks
    /// `resolve` to ask it with the path.
    ///
    /// The child's statics are armed by `Ctx::method_isolate` at construction
    /// rather than by the program's own prologue — there is no second unit to
    /// run an `install_in` from — so what runs is only what a compiled unit's
    /// entry does *after* that: take the argument into the isolate's ownership
    /// root, and call. That is `nvs_runtime::script::method_program`, built out
    /// of the two names this frame hands the seam rather than here, because the
    /// core that runs a child is the core that prepares it.
    ///
    /// **Argument 4 is the entry's parameter names**, comma-separated in
    /// declaration order and empty for an entry that declares none — another
    /// constant the lowering wrote, off the resolved call `nvs_types` recorded
    /// (`nvs_ir::lower`'s `spawn_method_entry` is the one home of the
    /// encoding). It is what ADR 0006 § *Decision*'s `args:` binding needs and
    /// the one thing the child cannot ask the runtime for: a
    /// `nvs_runtime::MethodRow` carries arity and parameter tags, never names.
    ///
    /// **`args:` binds by name**, which is that binding in two halves and two
    /// places. [`entry_names_agree`] judges the names *here*, before anything
    /// crosses, because a map naming a parameter the entry does not declare —
    /// or omitting one it does — is the ordinary named-argument error the ADR
    /// says it is, and this frame is the spawn it names as where to report it.
    /// [`bound_arguments`] then reads the values out in declaration order
    /// inside the child, where the copy is, and `nvs_runtime::call_static_bound`
    /// judges each against the slot it is about to fill. The map still crosses
    /// whole and `Core\Script::args()` still answers it, which is `rule:security/isolate-shares-nothing`'s
    /// accessor rule for both forms.
    fn nvs_core_script_spawn_method(ctx, args: [7]) {
        // Unreachable from source: the lowering emits this as a `ConstStr`, so
        // a non-string here is a compiler bug rather than a program's.
        let label = args[0]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "`spawn script Class::method` expected a constant label, got tag {}",
                    args[0].tag_byte()
                ))
            })?
            .to_owned();
        // Unreachable from source for the same reason argument 0 is: the
        // lowering emits this as a `ConstStr` too, so a non-string here is a
        // compiler bug rather than a program's.
        let names: Vec<String> = args[6]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "`spawn script Class::method` expected constant parameter names, got tag {}",
                    args[6].tag_byte()
                ))
            })?
            .split(',')
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect();
        let output = output_of(&args[2])?;
        let placement = placement_of(&args[3])?;
        let narrowing = narrowing_of(&args[4], &args[5])?;
        // Nothing is released here for the transferred argument 1, and that is
        // the spawn's own ownership rule rather than an omission: a
        // `TemporaryKind::Transferred` value is still on the lowering's
        // temporaries stack when `emit_fallible` builds this call's fault edge
        // (`nvs_ir::lower`'s `lower_spawn_script` is the one `CoreCall` site
        // that does not forget it first), so the frame releases it on every edge
        // this helper returns `Err` through.
        if let Err(message) = entry_names_agree(&format!("spawn script {label}"), &names, args[1]) {
            return Err(Fault::thrown_as(ThrownClass::Logic, message));
        }
        // `rule:security/capability-check-at-the-door`'s door for this form. The path form's is inside
        // `resolve`, which is the effect there; here the effect is the call
        // below and there is no intermediate to hang it on.
        nvs_runtime::capability::require(ctx, Cap::ScriptSpawn, Scope::Unscoped, "`spawn script`")?;
        // The two **names** and not a program: the label
        // `nvs_runtime::call_static_bound` looks the method up by, and the
        // parameters its `args:` map binds by. `nvs_runtime::script`'s
        // `method_program` is the code they become, built on the core that runs
        // the child — the seam's `Entry` owns why that is where it happens.
        let entry = Entry::Method {
            label: label.clone(),
            names,
        };
        // Handed over here: one reference goes to the isolate and the lowering
        // emitted no release for it.
        let crossing = args[1];
        let started = nvs_runtime::host::with_current(|host| {
            host.start_isolate(ctx, entry, crossing, output, placement, narrowing)
        });
        let running = match started {
            // Every arm is `nvs_core_script_spawn`'s, for its reasons — the
            // depth ceiling, the argument that could not cross and the missing
            // scheduler are all facts about the boundary rather than about
            // which form named the entry.
            Some(Ok(_)) if ctx.pending().is_some() => {
                return Err(Fault::Pending(nvs_runtime::FATAL));
            }
            Some(Ok(running)) => running,
            // Unreachable from source: a method entry names code the parent's
            // own unit already holds, so `Entry::program` asks no resolver and
            // has nothing to refuse — `nvs_runtime::script`'s module doc.
            Some(Err(StartError::Entry(error))) => {
                return Err(Fault::fatal(format!("`spawn script {label}`: {error}")));
            }
            Some(Err(StartError::Argument(error))) => {
                return Err(Fault::thrown_as(
                    ThrownClass::Logic,
                    format!("`spawn script {label}`: {error}"),
                ));
            }
            None => {
                return Err(Fault::fatal(format!(
                    "`spawn script {label}` needs a scheduler on this thread and there is none"
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
    /// `rule:security/isolate-shares-nothing`'s `ScriptResult`.
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
    /// `Core\Script::args(): mixed` — `rule:core-classes/script-args`'s replacement for `$_ARGS`,
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
    /// `Core\Script::onExit(callable $hook): void` — `rule:observability/script-on-exit`.
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
        // The routing half of the same registration, and the reason it is here
        // rather than at either ending: `nvs-host`'s isolate teardown is where
        // a served request ends and it may not name this crate, so the seam is
        // inverted through `nvs_runtime` and the one member that registers a
        // hook is what fills it — `Ctx::exit_drain`'s own doc owns the crate
        // graph behind that. Filling it again on a second registration writes
        // the same pointer.
        ctx.set_exit_drain(run_exit_hooks);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Script::finish(): void` — the ending whose raise belongs to the
    /// lowering and not to this body.
    ///
    /// `nvs_ir::lower` recognises the call by [`FINISH_SYMBOL`] and seals the
    /// block with a `Terminator::Throw` of a [`FINISH_MARKER_NAME`] instance, so
    /// nothing emits a call to this address. That is what buys the whole
    /// feature: the throw path is the path every `finally` lives on, and a
    /// member that returned a status instead would be `exit` under another name.
    fn nvs_core_script_finish(_ctx, _args: [0]) {
        // The row exists so the compiler can resolve and type the call, and
        // this body so the row has an address. The call itself is
        // unreachable from source: the lowering emits none to this symbol.
        // What it catches is a lowering that stopped intercepting `finish`,
        // which would otherwise return here and let the script carry on past
        // the ending it named.
        Err(Fault::fatal(
            "Core\\Script::finish reached its helper: the lowering raises the finish marker and \
             emits no call to this symbol"
                .to_string(),
        ))
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
        // `rule:security/response-body-is-one-typed-member`'s declaration is a *response's*, and `ScriptResult` is
        // not one: a `spawn script` answers with what the child wrote, and
        // what that output was declared to be is read by the connection that
        // is answering a peer or by nobody at all.
        content_type: _,
        // And the file a child named its body, ignored for the declaration's
        // own reason and with one thing more to say: a `spawn script` child is
        // not answering a request, so `Core\Response::sendFile` in one wrote the
        // bytes into the output above rather than leaving a name for a server —
        // `Ctx::declare_file_body` is where that fork is, and this field is what
        // the child said on its way past it.
        file_body: _,
        // Spec § 15's status, ignored on the same reasoning: a `ScriptResult`
        // is not a response, so a child that set one said it to whoever is
        // answering a peer, which a `spawn script`'s collector is not.
        status: _,
        // And its headers, on the same reasoning again: a `ScriptResult` has no
        // header line for a pair to reach, so what a child declared is read by
        // whoever is answering a peer or dropped here with the rest of it.
        headers: _,
        // And the child's own wall time, which is the parent's to spend: the
        // `spawn` event's overhead split reads it off the completion before
        // this shape is built (`rule:observability/spawn-is-its-own-event`),
        // and a program that never turned a debug bit on would read `null`
        // here for a clock nobody was allowed to consult.
        wall: _,
        // And what a sampled child filed, which is an exporter's and not a
        // program's: `rule:observability/four-kinds-become-a-span`'s spans are
        // derived at the door that answers a peer, and a `spawn script`
        // collector reading them here would be a second consumer of one
        // request's trace.
        trace: _,
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

    /// `Core\Script::args` answers the value this isolate was handed, as a
    /// reference of its own: the context keeps the one it holds, so every read
    /// is the same string and none of them is a copy. A context that was handed
    /// nothing answers `null`.
    // covers: Core\Script::args
    #[test]
    fn args_answers_the_isolates_own_value_as_a_second_reference() {
        let mut ctx = Ctx::buffered();
        let none = call(super::nvs_core_script_args, &mut ctx, &[]).expect("`args` cannot fail");
        assert_eq!(
            none.bits(),
            Value::null().bits(),
            "nothing spawned this context"
        );

        ctx.set_isolate_argument(Value::str(nvs_runtime::NvsStr::new(b"orders.csv")));
        let held = ctx
            .isolate_argument()
            .str_ptr()
            .expect("the argument is a string");
        for read in 1..=3 {
            let answer =
                call(super::nvs_core_script_args, &mut ctx, &[]).expect("`args` cannot fail");
            assert_eq!(answer.str_ptr(), Some(held), "read {read} is not a copy");
            #[expect(
                unsafe_code,
                reason = "the context still owns its reference, so the string is live, \
                          and this frame owns the one the call handed back"
            )]
            // SAFETY: `held` is kept alive by the context for the whole test,
            // and `answer` is this frame's own reference, released once.
            unsafe {
                assert_eq!(
                    nvs_runtime::NvsStr::refcount_of(held),
                    2,
                    "read {read}: the context's reference and this one"
                );
                answer.release();
            }
        }
        ctx.set_isolate_argument(Value::null());
    }

    /// `rule:observability/three-endings-fire-the-exit-queue`'s first row, and § 1's FIFO: two hooks registered in order
    /// run in that order, once each, and are handed `Normal` with status `0`.
    // covers: Core\Script::onExit
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

    /// `rule:observability/three-endings-fire-the-exit-queue`'s second row — the ending that exists to be observed: `exit`
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

    /// `rule:observability/a-hook-observes-and-never-steers`: an `exit` inside a
    /// hook is refused, and the process still exits with the status the report
    /// named. The hook's `exit` wrote its own status before it unwound, so the
    /// hook behind it and the process would otherwise disagree.
    // covers: Core\Script\ExitReport::status
    #[test]
    fn an_exit_inside_a_hook_leaves_the_process_status_the_report_named() {
        let mut ctx = Ctx::buffered();
        let first = register(&mut ctx, 1, exits);
        let second = register(&mut ctx, 1, records_second);
        ctx.set_exit_code(3);

        SEEN.with(|seen| seen.borrow_mut().clear());
        super::run_exit_hooks(&mut ctx, Err(nvs_runtime::EXITED), None);

        assert_eq!(
            SEEN.with(|seen| seen
                .borrow()
                .iter()
                .map(|saw| (saw.who, saw.status))
                .collect::<Vec<_>>()),
            [("exits", 3), ("second", 3)],
            "both hooks read the status the ending was fixed with"
        );
        assert_eq!(ctx.exit_code(), 3, "the process exits with it too");
        release(first);
        release(second);
    }

    /// `rule:observability/three-endings-fire-the-exit-queue`'s third row, asked as an **identity** exactly as `rule:errors/on-uncaught-throw`'s tier-2 handler is: the report carries the very allocation the
    /// program threw, so a hook can read its class, message and backtrace back
    /// through the ordinary members rather than a copy of what it said.
    #[test]
    fn an_uncaught_throw_reaches_the_queue_with_the_error() {
        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut classes = ClassTable::new();
        let root = classes.define("RuntimeError", &SLOTS, &[]);
        let mut ctx = Ctx::buffered();
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(classes), root));
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

    /// `rule:observability/a-fatal-and-a-cancellation-run-no-exit-hook`'s first termination: a resource-limit breach runs **no**
    /// hook, and `Core\Fatal::onLimit` on `rule:errors/on-limit`'s reserved slice stays
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

    /// `rule:observability/a-hook-observes-and-never-steers`: a hook that throws is abandoned where it stands and the
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

    /// `rule:observability/three-endings-fire-the-exit-queue`'s fourth row: a
    /// request that called `Core\Script::finish()` is `Finish` at status `0`,
    /// with **no error beside it**.
    ///
    /// The pairing is the assertion. The marker arrives on the `THROWN` path,
    /// which is the same status an uncaught throw arrives on, so a classifier
    /// reading the status alone reports every finish as a failure at status 1
    /// carrying a `Throwable` a hook could then read — and the object it would
    /// hand over is an implementation detail of how a finish unwinds, not
    /// something the program ever named.
    // covers: Core\Script::finish
    #[test]
    fn the_exit_report_names_the_finish_ending_with_a_zero_status_and_no_error() {
        let mut ctx = Ctx::buffered();
        let marker = finish_marker(&mut ctx);
        let hook = register(&mut ctx, 1, records_first);

        SEEN.with(|seen| seen.borrow_mut().clear());
        super::run_exit_hooks(&mut ctx, Err(nvs_runtime::THROWN), Some(&marker));

        assert_eq!(
            SEEN.with(|seen| (seen.borrow()[0].reason, seen.borrow()[0].status)),
            (super::FINISH, 0),
            "§ 2's fourth row: a finish is an ordinary end, so status 0"
        );
        assert_eq!(
            SEEN.with(|seen| seen.borrow()[0].error),
            Value::null().bits(),
            "the marker is how a finish unwinds and never the report's payload: \
             only an uncaught throw carries an error"
        );
        release(hook);
    }

    /// `rule:observability/a-hook-observes-and-never-steers`, for the ending
    /// this goal adds: `Core\Script::finish()` inside a hook is refused the way
    /// `exit` is, and the queue behind it still runs.
    ///
    /// A hook that could finish would suppress every hook behind it, which is
    /// the reasoning that section gives for `exit` — and here it is stronger,
    /// since the queue *is* what a finish delays the end of, so the marker would
    /// cut short the very drain it was raised inside. The second hook's own
    /// report is asserted beside the refusal: the ending was fixed before the
    /// first hook ran and the refusal does not restate it either.
    #[test]
    fn finish_inside_an_exit_hook_throws_and_the_drain_continues() {
        let mut ctx = Ctx::buffered();
        // The refusal's own words leave through the floor, which writes to the
        // diagnostic channel when `[log] target` names nothing —
        // `Ctx::write_log_record` is the routing.
        ctx.set_diagnostic_sink(nvs_runtime::OutputSink::Buffer(Vec::new()));
        let bad = register(&mut ctx, 1, finishes);
        let good = register(&mut ctx, 1, records_second);

        SEEN.with(|seen| seen.borrow_mut().clear());
        super::run_exit_hooks(&mut ctx, Ok(()), None);

        assert_eq!(
            SEEN.with(|seen| seen.borrow().iter().map(|saw| saw.who).collect::<Vec<_>>()),
            ["finishes", "second"],
            "§ 5: the refusal is logged and abandoned, and the hook behind it runs"
        );
        assert_eq!(
            SEEN.with(|seen| (seen.borrow()[1].reason, seen.borrow()[1].status)),
            (super::NORMAL, 0),
            "the ending was fixed before the first hook ran, and no hook renames it"
        );
        assert!(
            ctx.take_thrown().is_none(),
            "the refusal is reported through the floor rather than left pending"
        );
        let reported = String::from_utf8(
            ctx.take_buffered_diagnostic()
                .expect("the diagnostic channel was given a buffer"),
        )
        .expect("a record renders as UTF-8");
        assert!(
            reported.contains(r"Core\\Script::finish()"),
            "the record does not name what the hook did: {reported}"
        );
        assert!(
            reported.contains(r#""class":"RuntimeError""#),
            "the marker was reported as itself rather than as the \
             `RuntimeError` § 5 names, which is the whole of the refusal — a \
             record naming the marker's own class says an internal one \
             escaped: {reported}"
        );
        release(bad);
        release(good);
    }

    /// `Core\Script::finish()` inside a `Core\Task` child ends **that child's
    /// frame**, and the request goes on to its own ending —
    /// `rule:concurrency/after-response-outlives-the-connection`, whose
    /// trigger is the request task's own frame returning and never a child's.
    ///
    /// Both halves are the test, and each passes the other's failure. The
    /// marker is an ordinary throw, so it lands on the pending slot of the
    /// context the raise ran against and reaches *that* frame's root; nothing
    /// carries it across, so the request's queue is still undrained and the
    /// report it eventually gets names the ending the request itself reached.
    /// A child is sealed besides ([`mod@nvs_runtime::deferred`]'s *only the
    /// request's own task may register*), and the two belong together: a child
    /// able to end the request would be ending a response it may not write —
    /// `rule:concurrency/deferred-work-cannot-write-the-response`.
    #[test]
    fn finish_in_a_task_child_ends_that_child_and_not_the_request() {
        let mut ctx = Ctx::buffered();
        let marker = finish_marker(&mut ctx);
        let hook = register(&mut ctx, 1, records_first);

        SEEN.with(|seen| seen.borrow_mut().clear());

        #[expect(
            unsafe_code,
            reason = "the child is dropped inside this frame, so the parent \
                      outlives it as `Ctx::child` requires"
        )]
        // The child a `Core\Task` body runs on.
        let mut child = unsafe { ctx.child() };
        assert_eq!(
            // The seal is read before the queue is touched, so a null stands
            // in for the closure a registration would have carried.
            child.defer(Value::null(), 0),
            Err(nvs_runtime::deferred::DeferError::Sealed),
            "a child may not register after-response work, which is the work a \
             finish brings the end of the request forward to"
        );
        assert_eq!(
            child.exit_hook_count(),
            0,
            "and the exit queue is the request's: a child holds none of it"
        );

        // The raise the lowering of `Core\Script::finish()` emits, on the
        // context the child's frame is running against.
        child.raise(marker);
        assert!(
            child
                .pending_class()
                .is_some_and(|class| super::is_finish(&class)),
            "the marker is what the child's own root receives, and the ending \
             it reads there is the same one a request root reads"
        );
        drop(child);

        assert!(
            ctx.take_thrown().is_none(),
            "the marker is the child's ending and never crosses into the \
             request, which is what would make a finish anywhere in the tree \
             the request's own"
        );
        assert!(
            !ctx.exit_hooks_drained(),
            "the queue is the request's, and a child ending spends none of its \
             one drain"
        );

        super::run_exit_hooks(&mut ctx, Ok(()), None);
        assert_eq!(
            SEEN.with(|seen| (seen.borrow()[0].reason, seen.borrow()[0].status)),
            (super::NORMAL, 0),
            "the request reached its own ending afterwards, and that is the \
             one the report names"
        );
        release(hook);
    }

    /// The marker `Core\Script::finish()` raises, as the object that reaches a
    /// root — built here because no compiler is in front of these cases.
    ///
    /// The table is the narrowest one the raise needs: `RuntimeError` for the
    /// class the context installs, and the marker beside it with no parent and
    /// no slots, which is what `nvs_hir::errors::TREE` declares it as. Leaked
    /// for [`closure_of`]'s reason.
    fn finish_marker(ctx: &mut Ctx) -> nvs_runtime::Thrown {
        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut classes = ClassTable::new();
        let root = classes.define("RuntimeError", &SLOTS, &[]);
        classes.define(super::FINISH_MARKER_NAME, &[] as &[&str], &[]);
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(classes), root));
        let desc = ctx
            .class_desc(super::FINISH_MARKER_NAME)
            .expect("the table installed a line above declares it");
        #[expect(
            unsafe_code,
            reason = "the descriptor comes out of the table this context now \
                      holds, so it outlives the object, whose one reference is \
                      handed to the `Thrown`"
        )]
        unsafe {
            nvs_runtime::Thrown::from_raw(NvsObj::new(desc).into_raw())
        }
    }

    /// A hook that records that it ran and then exits with status 7, the way
    /// compiled `exit(7)` does: the status is written first, then the unwind.
    #[expect(unsafe_code, reason = "[`throws`]'s reason, on its twin")]
    unsafe extern "C" fn exits(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        unsafe { record("exits", args, out) };
        let ctx = unsafe { &mut *ctx };
        ctx.set_exit_code(7);
        nvs_runtime::EXITED
    }

    /// A hook that records that it ran and then finishes, for § 5's second
    /// refusal.
    #[expect(unsafe_code, reason = "[`throws`]'s reason, on its twin")]
    unsafe extern "C" fn finishes(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        unsafe { record("finishes", args, out) };
        let ctx = unsafe { &mut *ctx };
        let marker = finish_marker(ctx);
        ctx.raise(marker);
        nvs_runtime::THROWN
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
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        table.set_closure(id);
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

    /// `reason` answers the ending the report was built for, as the enum's
    /// case index, and the same answer on every read.
    // covers: Core\Script\ExitReport::reason
    #[test]
    fn reason_answers_the_ending_the_report_was_built_for_on_every_read() {
        let mut ctx = Ctx::buffered();
        for reason in [
            super::NORMAL,
            super::EXIT_CALL,
            super::UNCAUGHT_THROW,
            super::FINISH,
        ] {
            let report = super::report_of(reason, 0, None);
            for read in 1..=2 {
                let answer = call(
                    super::nvs_core_script_exit_report_reason,
                    &mut ctx,
                    &[report],
                )
                .expect("`reason` cannot fail");
                assert_eq!(answer.as_int(), Some(reason), "read {read}");
            }
            release(report);
        }
    }

    /// `status` answers the number the report was built with, unchanged, at
    /// both far ends of the range as well as the ones `exit` usually names.
    // covers: Core\Script\ExitReport::status
    #[test]
    fn status_answers_the_number_it_was_built_with_unchanged() {
        let mut ctx = Ctx::buffered();
        for status in [0, 1, 42, 255, -1, i64::MIN, i64::MAX] {
            let report = super::report_of(super::EXIT_CALL, status, None);
            let answer = call(
                super::nvs_core_script_exit_report_status,
                &mut ctx,
                &[report],
            )
            .expect("`status` cannot fail");
            assert_eq!(answer.as_int(), Some(status));
            release(report);
        }
    }

    /// `error` answers `null` for a report with no exception, and for an
    /// uncaught throw the very object the program threw, as a reference of its
    /// own: the report and the `Thrown` each keep theirs, and giving the answer
    /// back leaves both.
    // covers: Core\Script\ExitReport::error
    #[test]
    fn error_answers_the_thrown_object_itself_as_a_reference_of_its_own() {
        let mut ctx = Ctx::buffered();
        let none = super::report_of(super::EXIT_CALL, 2, None);
        let answer = call(super::nvs_core_script_exit_report_error, &mut ctx, &[none])
            .expect("`error` cannot fail");
        assert_eq!(
            answer.bits(),
            Value::null().bits(),
            "no exception, no error"
        );
        release(none);

        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut classes = ClassTable::new();
        let root = classes.define("RuntimeError", &SLOTS, &[]);
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(classes), root));
        ctx.set_pending("the store said no");
        ctx.push_frame("Main::main");
        let thrown = ctx.take_thrown();
        let object = thrown
            .as_value()
            .obj_ptr()
            .expect("an installed class promotes the failure to an object");
        let report = super::report_of(super::UNCAUGHT_THROW, 1, Some(&thrown));
        let answer = call(
            super::nvs_core_script_exit_report_error,
            &mut ctx,
            &[report],
        )
        .expect("`error` cannot fail");
        assert_eq!(answer.bits(), thrown.as_value().bits(), "the object itself");
        #[expect(
            unsafe_code,
            reason = "`thrown` owns a reference for the whole test, so the object \
                      is live at every read"
        )]
        // SAFETY: `thrown` keeps the object alive until the end of the test.
        unsafe {
            assert_eq!(
                NvsObj::refcount_of(object),
                3,
                "the throw, the report, the answer"
            );
            answer.release();
            release(report);
            assert_eq!(NvsObj::refcount_of(object), 1, "the throw's own is left");
        }
        assert_eq!(thrown.message(), "the store said no");
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
