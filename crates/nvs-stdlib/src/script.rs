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
//! neither has a [`CoreMethod`](crate::registry::CoreMethod) row. That is the
//! point: `spawn script` and `await` are *syntax*, so the only thing that may
//! call either is the lowering of the construct that spells it, and a row would
//! make both reachable as `Core\Script\Handle::…()` from source. A symbol with
//! no row is invisible to `spec_registry_coverage.rs` and to
//! `conformance_coverage.rs` for the same reason, and the construct's own
//! `.nvst` cases are what cover it instead.
//!
//! `Core\Script` itself — item 22's `args()` and the `valueOrThrow($result)`
//! that a shape cannot carry as a method — lands in this module beside its
//! handle, the way `Core\Task` and `Core\Task\Channel<T>` already sit together
//! under [ADR 0011](../../../../docs/adr/0011-functions-and-constants-are-class-members.md).

use nvs_runtime::host::{Completion, Output};
use nvs_runtime::script::ResolveError;
use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::CoreClass;

/// The handle class's fully-qualified name, as
/// [`CoreTy::Instance`](crate::registry::CoreTy::Instance) spells it.
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
        _ => return None,
    })
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

/// One [`Completion`] as the shape the language surface reads.
fn result_of(completion: Completion) -> Value {
    let Completion {
        ok,
        value,
        output,
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
