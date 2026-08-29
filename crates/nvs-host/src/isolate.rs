//! ADR 0006's isolate, as one type: a child task with a heap boundary.
//!
//! [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md) fixes what an
//! isolate *is* — it shares immutable compiled code and its parent's budget and
//! nothing else, values cross by copy or by move at refcount 1, and a child's
//! failure arrives as data rather than as an unwind.
//! [ADR 0116](../../../docs/adr/0116-an-isolates-arena-is-an-ownership-root.md)
//! decides how, and this module is that decision in code: an arena is an
//! ownership root, so building one is [`Ctx::isolate`] and releasing one
//! wholesale is dropping that context.
//!
//! `docs/plan/design.md` § *In-process isolated script execution* asks for
//! **one** `Isolate` type serving both the `spawn script` path and, at M7, an
//! inbound HTTP request. This is that type; what differs between the two is the
//! [`Program`] handed in and the [`Output`] asked for, not the boundary.
//!
//! # The program arrives as a closure, not as a path
//!
//! An isolate runs another `.nvs` file, and turning a path into runnable code
//! is [`nvs_runtime::script`]'s seam rather than anything this module can do —
//! that module owns the decision and the reason. What matters here is only the
//! consequence: a [`Program`] is a boxed closure over an already-prepared unit,
//! exactly as [`nvs_runtime::host::Job`] is for a task. Both crossings, the
//! failure classification and the whole context construction live here
//! regardless, because not one of them depends on where the code came from.
//!
//! # Decision: a refused argument is the parent's fault, a refused answer is the child's
//!
//! One walk refuses at both crossings
//! ([ADR 0023](../../../docs/adr/0023-clone-serialize-and-cross-boundary-copy.md)
//! § 2), and the two refusals mean different things:
//!
//! * An **argument** that cannot cross was built by the parent, before any child
//!   existed. [`Isolate::run`] answers `Err`, no task is started, and the caller
//!   raises it in the parent — which is where the offending value's name and its
//!   path in the graph mean something.
//! * An **answer** that cannot cross was built by the child. That is ADR 0006's
//!   *failure is a value*: it lands as `ok = false` carrying the walk's own
//!   message, beside an uncaught throw and a limit breach, and the parent keeps
//!   running.
//!
//! # What it spends
//!
//! ADR 0116 § 3 is the one home of the accounting. Per **in-flight** isolate:
//! one [`Ctx`], one pooled task stack, and the values the child allocates.
//! Nothing here is O(isolates created) — a finished isolate's context is dropped
//! as its task ends, and that drop is the wholesale release.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use nvs_runtime::graph::{GraphError, copy_graph};
use nvs_runtime::{Ctx, OutputSink, TaskRoot, Value};

use crate::scheduler::{Waiting, Wake, cancel_task, spawn_child, suspend_current};

pub use nvs_runtime::script::Program;

/// Where the child's `echo` ends up — ADR 0006 § *Output is captured by
/// default*.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Output {
    /// On the result, as bytes. The default, because the alternative silently
    /// mixes another script's bytes into a response the parent is responsible
    /// for.
    #[default]
    Capture,
    /// Appended to the parent's own output stream **when the result is
    /// awaited**, which is what keeps the ordering deterministic under
    /// concurrency. The child buffers either way; the two options differ only in
    /// who is handed the bytes at the end.
    Inherit,
}

/// What a child's failure looks like on the parent's side: data, never an
/// exception object (ADR 0006 § *Failure is a value*).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// The rendered class name of what the child threw, or `Error` where the
    /// failure was not a throw at all.
    pub class: String,
    /// The message, rendered on the child's side. Copying the exception
    /// *object* across is what ADR 0006 rejected.
    pub message: String,
}

/// What an isolate answers with — the native half of the `ScriptResult` the
/// language surface will present.
#[derive(Debug)]
pub struct Completion {
    /// True exactly when the child ran to its top-level `return` **and** its
    /// answer crossed.
    pub ok: bool,
    /// The child's returned value, copied into the caller's ownership root.
    /// `null` whenever `ok` is false.
    pub value: Value,
    /// What the child wrote. Emptied by [`Output::Inherit`], which has already
    /// handed the bytes to the parent's own stream.
    pub output: Vec<u8>,
    /// Present exactly when `ok` is false.
    pub error: Option<Failure>,
}

/// One isolate: a program, the argument crossing into it, and where its output
/// goes.
pub struct Isolate {
    program: Program,
    args: Value,
    output: Output,
}

impl std::fmt::Debug for Isolate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hand-written: a `Program` is a closure and has no `Debug`.
        f.debug_struct("Isolate")
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

impl Isolate {
    /// Builds one. **Consumes one reference to `args`**, which [`Isolate::run`]
    /// hands to the walk; dropping an `Isolate` without running it leaks that
    /// reference, the same obligation [`nvs_runtime::host::Job`] documents for a
    /// job that is never called.
    #[must_use]
    pub fn new(program: Program, args: Value, output: Output) -> Self {
        Self {
            program,
            args,
            output,
        }
    }

    /// Runs it to completion and answers with what crossed back.
    ///
    /// The sequence is ADR 0116 §§ 2 and 5: copy the argument in, build the
    /// isolate's own ownership root, run it as a child task, copy the answer out
    /// **before** that root is released, and release it. Control does not return
    /// while the child is still running, exactly as it does not for a group
    /// ([ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
    /// § 4).
    ///
    /// # Errors
    ///
    /// [`GraphError`] when the *argument* has no meaning on the other side. No
    /// child is started in that case; the module doc owns why the same refusal
    /// on the way back is an `ok = false` instead.
    pub fn run(self, ctx: &mut Ctx) -> Result<Completion, GraphError> {
        let Self {
            program,
            args,
            output,
        } = self;
        // In, at the spawn — before anything is built, so a refusal costs
        // nothing and leaves no half-made isolate behind.
        let crossed = copy_graph(args)?;

        // The isolate's own root. Buffered under both options; § 4's fresh
        // statics base is `Ctx::isolate`'s whole reason for existing.
        let isolate_ctx = ctx.isolate(OutputSink::Buffer(Vec::new()));

        let mut completion = match Wake::current() {
            Some(wake) => run_as_task(isolate_ctx, program, crossed, Rc::new(wake)),
            // No task beneath the call, which takes a host installed by
            // something other than a running scheduler — `run_group`'s own
            // case. The program still has to run and the answer still has to be
            // right, so it runs on the caller's stack. Nothing about the
            // boundary weakens: it is the context, not the stack.
            None => run_here(isolate_ctx, program, crossed),
        };

        if output == Output::Inherit {
            // Handed over at the await, which is here — the one point at which
            // the ordering is a fact rather than a race.
            let bytes = std::mem::take(&mut completion.output);
            let _ = ctx.write_output(&bytes);
        }
        Ok(completion)
    }
}

/// Fires when the child's task ends **however** it ended — returning, throwing,
/// or torn down by a forced unwind without its body ever finishing.
///
/// The same shape `group::Child`'s own `Drop` has, and for the same reason: the
/// awaiting side is parked on this, and a cancelled child that only signalled
/// from the end of its body would leave that park with nothing to wake it.
struct Ended {
    done: Rc<Cell<bool>>,
    wake: Rc<Wake>,
}

impl Drop for Ended {
    fn drop(&mut self) {
        self.done.set(true);
        self.wake.wake();
    }
}

/// The child on a stack of its own, with the awaiting side parked until it ends.
///
/// A single-child sibling of `group::run_as_children`'s loop rather than a call
/// into it, because that runner gives every child a [`Ctx::child`] — the
/// *aliased* statics base ADR 0116 § 4 says an isolate may not have. The part
/// that matters is identical: this call does not return while the child is still
/// running.
fn run_as_task(isolate_ctx: Ctx, program: Program, args: Value, wake: Rc<Wake>) -> Completion {
    let slot: Rc<RefCell<Option<Completion>>> = Rc::new(RefCell::new(None));
    let done = Rc::new(Cell::new(false));

    let filed = Rc::clone(&slot);
    let ended = Ended {
        done: Rc::clone(&done),
        wake,
    };
    let spawned = spawn_child(isolate_ctx, TaskRoot::Request, move |child| {
        // Moved in so the guard is dropped with the body — including when the
        // body is torn down half-way through by a forced unwind.
        let ended = ended;
        let answer = program(child, args);
        *filed.borrow_mut() = Some(finish(child, answer));
        drop(ended);
    });

    let Some(id) = spawned else {
        // Unreachable from inside a turn: holding a `Wake` is `current_task`
        // and the tree both answering. Nothing ran, so nothing is owed.
        return cancelled_completion();
    };

    let mut cancelling = false;
    while !done.get() {
        let resumed = suspend_current(Waiting::Parked);
        if !resumed.suspended() {
            // Nothing suspended, so the core was never handed back and the
            // child cannot make progress; looping would spin. The same refusal
            // `suspend` documents.
            break;
        }
        if resumed.cancelled() && !cancelling {
            // This task was cancelled while it waited. The child dies with it,
            // and the loop keeps parking until it has — ADR 0072 § 4's "control
            // does not leave the call with work still running" holds through a
            // cancellation too.
            cancelling = true;
            cancel_task(id);
        }
    }

    let completion = slot.borrow_mut().take();
    completion.unwrap_or_else(cancelled_completion)
}

/// The child on the caller's own stack, for a host with no scheduler under it.
fn run_here(mut isolate_ctx: Ctx, program: Program, args: Value) -> Completion {
    let answer = program(&mut isolate_ctx, args);
    finish(&mut isolate_ctx, answer)
    // `isolate_ctx` is dropped here: ADR 0116 § 2's wholesale release.
}

/// Classifies how the isolate ended and copies its answer **out** of the arena
/// it was built in.
///
/// Called on the child's own stack, while its context is still alive, which is
/// the ordering ADR 0116 § 5 requires: the copy reads the child's graph, and the
/// caller's root owns what comes back.
fn finish(isolate_ctx: &mut Ctx, answer: Value) -> Completion {
    let cancelled = isolate_ctx.cancelled();
    let failed = !cancelled && isolate_ctx.pending().is_some();
    let thrown = failed.then(|| isolate_ctx.take_thrown());
    let output = isolate_ctx.take_buffered_output().unwrap_or_default();

    if let Some(thrown) = thrown {
        // ADR 0006's second row: the class name and the message as copied data.
        // The exception object stays in the arena that is about to go.
        let class = class_name(&thrown);
        release(answer);
        return Completion {
            ok: false,
            value: Value::null(),
            output,
            error: Some(Failure {
                class,
                message: thrown.message(),
            }),
        };
    }
    if cancelled {
        // Not the child's own failure: its parent died or a deadline landed,
        // and ADR 0072 § 5 means nothing of the child's runs on the way out.
        release(answer);
        let mut completion = cancelled_completion();
        completion.output = output;
        return completion;
    }
    // Out, at the await. A refusal here is the child's, so it is a failure
    // value rather than an `Err` — the module doc owns the asymmetry.
    match copy_graph(answer) {
        Ok(value) => Completion {
            ok: true,
            value,
            output,
            error: None,
        },
        Err(refused) => Completion {
            ok: false,
            value: Value::null(),
            output,
            error: Some(Failure {
                class: "Error".to_owned(),
                message: refused.to_string(),
            }),
        },
    }
}

/// The answer for an isolate that never reported: cancelled, or torn down before
/// its body finished.
fn cancelled_completion() -> Completion {
    Completion {
        ok: false,
        value: Value::null(),
        output: Vec::new(),
        error: Some(Failure {
            class: "Error".to_owned(),
            message: "the isolate was cancelled".to_owned(),
        }),
    }
}

/// The rendered class name behind a [`nvs_runtime::Thrown`], or ADR 0020's
/// generic one where there is no descriptor to read.
fn class_name(thrown: &nvs_runtime::Thrown) -> String {
    let desc = thrown.class_desc();
    if desc.is_null() {
        return "Error".to_owned();
    }
    #[expect(
        unsafe_code,
        reason = "a `Thrown`'s descriptor is published by a class table that \
                  outlives every value built from it; see `nvs_runtime::object`"
    )]
    // SAFETY: non-null here means the class table handed it out, and a
    // descriptor's address is its identity for the life of that table.
    unsafe {
        (*desc).name().to_owned()
    }
}

/// One reference, dropped.
fn release(value: Value) {
    #[expect(
        unsafe_code,
        reason = "this is the last owner of the reference the program \
                  transferred; no slot will take it"
    )]
    // SAFETY: the program transferred this reference and the failure path means
    // nothing else will read it.
    unsafe {
        value.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::Scheduler;
    use nvs_runtime::{ClassTable, FieldDefault, MethodRow, NvsObj};

    /// A parent that looks like a request: armed statics, a buffer of its own,
    /// and an error class, so a child's bare-message failure becomes a `Thrown`
    /// carrying a name rather than a null one.
    fn parent() -> Ctx {
        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut table = ClassTable::new();
        let root = table.define("Throwable", &SLOTS, &[]);
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        ctx.set_runtime_error_class(nvs_runtime::ErrorClass::new(Rc::new(table), root));
        ctx.install_statics(&[Some(FieldDefault::Int(7))]);
        ctx
    }

    /// Runs one isolate inside a real task, so `Wake::current` answers and the
    /// child gets a stack of its own — the shape every caller will have.
    fn run(isolate: Isolate, ctx: &mut Ctx) -> Result<Completion, GraphError> {
        let mut sched = Scheduler::new();
        let held: Rc<RefCell<Option<Result<Completion, GraphError>>>> = Rc::new(RefCell::new(None));
        let out = Rc::clone(&held);
        let back: Rc<RefCell<Option<Ctx>>> = Rc::new(RefCell::new(None));
        let returned = Rc::clone(&back);
        // The parent context is the caller's, so it goes into the task and comes
        // back out: every assertion below is about what it looks like after.
        let moved = std::mem::replace(ctx, Ctx::new(OutputSink::Sink));
        let mut cell = Some(isolate);
        sched.spawn(moved, TaskRoot::Request, move |parent_ctx| {
            let answer = cell.take().expect("run once").run(parent_ctx);
            *out.borrow_mut() = Some(answer);
            *returned.borrow_mut() =
                Some(std::mem::replace(parent_ctx, Ctx::new(OutputSink::Sink)));
        });
        sched.run();
        if let Some(parent_ctx) = back.borrow_mut().take() {
            *ctx = parent_ctx;
        }
        held.borrow_mut().take().expect("the task ran to the end")
    }

    /// A closure, spelled the way `nvs_runtime::graph`'s walk recognizes one:
    /// a class carrying the `invoke` method. Leaked, because a descriptor's
    /// address is its identity.
    fn closure_value() -> Value {
        let mut table = ClassTable::new();
        let id = table.define("Closure", &["arity"], &[]);
        table.set_methods(
            id,
            vec![MethodRow {
                name: nvs_runtime::closure::CLOSURE_INVOKE.to_owned(),
                code: std::ptr::dangling(),
                arity: 0,
                param_tags: 0,
                public: true,
                native: false,
            }],
        );
        let table: &'static ClassTable = Box::leak(Box::new(table));
        #[expect(unsafe_code, reason = "the leaked table outlives the object")]
        // SAFETY: the table is leaked, so the descriptor outlives every value.
        unsafe {
            Value::object(NvsObj::new(table.desc(id)))
        }
    }

    /// ADR 0116 § 4, which is ADR 0006's "globals, class statics and
    /// runtime-defined constants are fresh": an isolate's statics base is its
    /// own, so nothing it writes can reach the parent's slot. This is the
    /// assertion the whole boundary rests on, because compiled code reaches a
    /// static through that one word and through nothing else.
    #[test]
    fn a_child_cannot_read_or_write_a_parent_variable_or_static() {
        let mut ctx = parent();
        let parent_base = ctx.statics_base();
        assert!(!parent_base.is_null(), "the parent armed one slot");

        let seen: Rc<Cell<*mut Value>> = Rc::new(Cell::new(std::ptr::dangling_mut()));
        let recorded = Rc::clone(&seen);
        let program: Program = Box::new(move |child: &mut Ctx, _args| {
            // Before the child arms anything of its own the base is null: it is
            // not the parent's, and there is nothing to have aliased it to.
            recorded.set(child.statics_base());
            child.install_statics(&[Some(FieldDefault::Int(1)), Some(FieldDefault::Int(2))]);
            Value::int(i64::try_from(child.statics_len()).expect("two slots"))
        });

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("a null argument crosses");

        assert!(done.ok, "{:?}", done.error);
        assert_eq!(done.value.as_int(), Some(2));
        assert!(
            seen.get().is_null(),
            "an isolate starts with no statics base at all, let alone the parent's"
        );
        assert_eq!(
            ctx.statics_base(),
            parent_base,
            "and the parent's own base is untouched by the child arming two slots"
        );
        assert_eq!(ctx.statics_len(), 1, "the parent still has its one slot");
    }

    /// The child buffers into its own context, so nothing it writes reaches the
    /// parent's stream — under `Capture` the bytes come back as data instead.
    #[test]
    fn a_child_cannot_see_the_parents_output_buffer() {
        let mut ctx = parent();
        ctx.write_output(b"parent first\n").expect("a buffer");

        let program: Program = Box::new(|child: &mut Ctx, _args| {
            child.write_output(b"child stdout").expect("a buffer");
            Value::null()
        });

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("a null argument crosses");

        assert!(done.ok, "{:?}", done.error);
        assert_eq!(done.output, b"child stdout");
        assert_eq!(
            ctx.take_buffered_output().unwrap_or_default(),
            b"parent first\n",
            "the parent's stream carries only the parent's own bytes"
        );
    }

    /// `Output::Inherit` is the same machinery with the other sink, and the
    /// bytes arrive at the *await* — which is what makes their position in the
    /// parent's stream a fact rather than a race.
    #[test]
    fn inherited_output_reaches_the_parents_stream_at_the_await() {
        let mut ctx = parent();
        ctx.write_output(b"before ").expect("a buffer");

        let program: Program = Box::new(|child: &mut Ctx, _args| {
            child.write_output(b"child").expect("a buffer");
            Value::null()
        });

        let done = run(
            Isolate::new(program, Value::null(), Output::Inherit),
            &mut ctx,
        )
        .expect("a null argument crosses");

        assert!(done.ok, "{:?}", done.error);
        assert!(
            done.output.is_empty(),
            "inherited bytes are not also captured"
        );
        ctx.write_output(b" after").expect("a buffer");
        assert_eq!(
            ctx.take_buffered_output().unwrap_or_default(),
            b"before child after"
        );
    }

    /// ADR 0023 § 2's refusals reach this boundary because it is the same walk.
    /// A closure is the shape a test at this level can build without a compiler,
    /// and what is asserted is the *classification*: no child is started, and
    /// the caller is handed the refusal to raise in the parent.
    #[test]
    fn a_closure_a_reference_or_a_resource_is_refused_at_the_boundary() {
        let mut ctx = parent();
        let ran = Rc::new(Cell::new(false));
        let started = Rc::clone(&ran);
        let program: Program = Box::new(move |_: &mut Ctx, _args| {
            started.set(true);
            Value::null()
        });

        let refused = run(
            Isolate::new(program, closure_value(), Output::Capture),
            &mut ctx,
        )
        .expect_err("a closure has no meaning in another arena");

        assert!(
            refused.to_string().contains("closure"),
            "the diagnostic names the offending value: {refused}"
        );
        assert!(
            !ran.get(),
            "the refusal lands before the child exists, so nothing was started"
        );
    }

    /// A child that answers with a value that cannot cross is the *other* half
    /// of the asymmetry the module doc names: its own fault, so `ok = false`
    /// rather than an `Err` the parent raises.
    #[test]
    fn an_answer_that_cannot_cross_is_a_failure_value_and_not_a_refusal() {
        let mut ctx = parent();
        let program: Program = Box::new(|_: &mut Ctx, _args| closure_value());

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crossed; only the answer did not");

        assert!(!done.ok);
        let failure = done.error.expect("a failure is a value");
        assert!(failure.message.contains("closure"), "{failure:?}");
        assert!(ctx.pending().is_none(), "the parent is not failing");
    }

    /// ADR 0006 § *Failure is a value*: the child's throw is data on the result,
    /// the parent is not unwound, and the parent is still usable afterwards —
    /// which is the whole claim the boundary is built for.
    #[test]
    fn an_uncaught_throw_in_a_child_leaves_the_parent_running() {
        let mut ctx = parent();
        let program: Program = Box::new(|child: &mut Ctx, _args| {
            child.write_output(b"got as far as here").expect("a buffer");
            child.set_pending("the child gave up");
            Value::null()
        });

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("a null argument crosses");

        assert!(!done.ok);
        let failure = done.error.expect("a failure is a value");
        assert_eq!(failure.message, "the child gave up");
        assert_eq!(
            failure.class, "Throwable",
            "the name is copied, not the object"
        );
        assert_eq!(
            done.output, b"got as far as here",
            "what it echoed before it failed, it did echo"
        );
        assert!(ctx.pending().is_none(), "the parent is not failing");

        ctx.write_output(b"parent still running").expect("a buffer");
        assert_eq!(
            ctx.take_buffered_output().unwrap_or_default(),
            b"parent still running"
        );
    }
}
