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
//! A **cancellation** reaches that same `ok = false` by two different routes,
//! and which one a child takes is a fact about its stack rather than about the
//! boundary. A child that is parked, or running with nothing of `nvs-stdlib` on
//! its stack, is force-unwound by the scheduler and never reaches [`finish`]:
//! its slot is unfiled and [`cancelled_completion`] is what the join answers
//! with. A child standing on a [`nvs_runtime::HelperFrame`] may not be unwound
//! there, so it is *told*, answers with [`Ctx::cancel`] and returns — [`finish`]
//! sees [`Ctx::cancelled`] and classifies it, which is the route that keeps
//! what the child echoed before the safepoint. Both are ADR 0072 § 5, and
//! neither runs a line of the child's own code on the way out.
//!
//! § 2's *unresolvable class* is asked of the answer and not of the argument,
//! and the asymmetry is a known gap rather than a decision. The rule needs the
//! receiving side's class table: at the join the parent's is in hand
//! ([`Ctx::class_table`], taken at the spawn because by then the parent's
//! context is borrowed by the frame parked on the join), while at the spawn
//! the child's does not exist yet — its program's prologue installs it. Closing
//! it means the [`nvs_runtime::script`] seam answering with a unit's table
//! beside its entry point, which is a change to that seam and not to this walk.
//! What the rule *is* — the same class, by descriptor identity, not a class of
//! the same name — is `nvs_runtime::graph`'s `Live::admit`, its one home.
//!
//! # What it spends
//!
//! ADR 0116 § 3 is the one home of the accounting. Per **in-flight** isolate:
//! one [`Ctx`], one pooled task stack, and the values the child allocates.
//! Nothing here is O(isolates created) — a finished isolate's context is dropped
//! as its task ends, and that drop is the wholesale release.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use nvs_runtime::graph::{GraphError, copy_graph, copy_graph_into};
use nvs_runtime::{Ctx, ErrorClass, Fault, Limit, OutputSink, TaskRoot, Value};

use crate::scheduler::{TaskId, Waiting, Wake, cancel_task, spawn_child, suspend_current};

pub use nvs_runtime::script::Program;

// The three shapes the crossing is described in are declared on the seam
// itself, in `nvs_runtime::host`, because that is where a `Core` member reaches
// them from and a name written out in two crates is a name that can drift. They
// are re-exported here because this module is where they *mean* something: the
// seam fixes the shape, and everything below decides the behaviour.
pub use nvs_runtime::host::{Completion, Failure, Output, Running};

/// One isolate: a program, the argument crossing into it, and where its output
/// goes.
pub struct Isolate {
    program: Program,
    args: Value,
    output: Output,
    charge: Charge,
}

/// Whose budget an isolate spends: ADR 0006's answer, and ADR 0020 § 3's one
/// named exception to it.
///
/// Private, and a builder rather than a parameter of [`Isolate::new`], because
/// § 3 states that the exception is not a precedent. Nothing a program writes
/// can reach it — `nvs_runtime::host`'s seam is what `spawn script` arrives
/// through, and it never calls the builder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Charge {
    /// The tree it was spawned from, which is every isolate a program spawns.
    Tree,
    /// The engine's own reserve, which is [`crate::ladder`]'s handler and
    /// nothing else.
    EngineReserve,
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
            charge: Charge::Tree,
        }
    }

    /// Charges it to ADR 0020 § 3's engine-owned reserve instead of to the tree
    /// that spawned it.
    ///
    /// [`Ctx::handler_isolate`] is the one home of what that changes and why
    /// § 3 wants it. Here it changes two things and no more: the context this
    /// isolate runs under, and the tree-depth refusal below, which is asked of
    /// the tree only.
    #[must_use]
    pub fn charged_to_the_engine_reserve(mut self) -> Self {
        self.charge = Charge::EngineReserve;
        self
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
        Ok(self.start(ctx)?.join(ctx))
    }

    /// Starts it and answers with the handle that collects it later —
    /// `spawn script`'s half of [`Isolate::run`].
    ///
    /// The argument crosses here, and the child is a runnable task before this
    /// returns, which is what makes `spawn` and `await` two constructs: a
    /// parent that spawns three and awaits three overlaps them. What is
    /// deferred to [`Running::join`] is only the wait, the answer's crossing
    /// and the [`Output::Inherit`] hand-over — the last of those deliberately,
    /// since the await is the one point at which the ordering against the
    /// parent's own output is a fact rather than a race.
    ///
    /// # Errors
    ///
    /// [`GraphError`] when the *argument* has no meaning on the other side. No
    /// child is started in that case; the module doc owns why the same refusal
    /// on the way back is an `ok = false` instead. A chain already at
    /// `[limits] max_script_depth` is **not** one of these — see the refusal at
    /// the top of the body.
    pub fn start(self, ctx: &mut Ctx) -> Result<Box<dyn Running>, GraphError> {
        let Self {
            program,
            args,
            output,
            charge,
        } = self;
        // ADR 0020 § 1's ceiling on the tree, ahead of everything else in this
        // body: `Ctx::script_depth_breach` owns why the question belongs to the
        // parent and why it is asked once here rather than polled at a
        // safepoint. Nothing has been built yet at this point, which is the
        // whole reason the check stands above the crossing below — a refusal
        // costs one comparison and leaves no half-made isolate behind.
        let depth_breach = match charge {
            Charge::Tree => ctx.script_depth_breach(),
            // ADR 0020 § 3's handler is not part of the tree this ceiling
            // bounds, and a chain that reached the ceiling is one of the
            // failures it exists to report — so asking here would refuse the
            // report on the grounds of the thing being reported.
            // `Ctx::handler_isolate` owns that reading and gives it a fresh
            // depth to start from.
            Charge::EngineReserve => None,
        };
        if let Some(Fault::Fatal(message)) = depth_breach {
            // The safepoint's memory branch, in its order and for its reason
            // (`nvs_runtime::nvs_safepoint`): § 1's tier 1 runs before the
            // breach becomes the message the ladder prints, so a throw of the
            // handler's own is overwritten by `set_pending` rather than
            // reported in place of the limit that stopped the request. The
            // report it is handed reads `max_script_depth`, which is the one
            // thing separating this from the out-of-memory the heap used to
            // deliver instead (`docs/plan/m6.md`'s *Verify*).
            ctx.run_limit_handler(Limit::ScriptDepth);
            ctx.set_pending(message.clone());
            // Not a `GraphError`: that error is the *argument's* and this
            // refusal is the tree's, and the module doc holds the argument's to
            // its own meaning. The parent is fatal from here, so the handle
            // this answers with carries the failure the child never ran to
            // produce — a `spawn` whose `await` is never reached sees the
            // `FATAL` first either way.
            return Ok(Box::new(Collected {
                completion: Some(refused_completion(&message)),
                output,
            }));
        }
        // In, at the spawn — before anything is built, so a refusal costs
        // nothing and leaves no half-made isolate behind. No receiving table
        // is named on this side: the child's own is installed by its program's
        // prologue and does not exist yet, which is the module doc's known gap.
        let crossed = copy_graph(args)?;
        // Taken here rather than at the join, because the answer crosses on the
        // child's own stack and this context is borrowed by then
        // (`Ctx::class_table`).
        let receiving = ctx.class_table();

        // The isolate's own root. Buffered under both options; § 4's fresh
        // statics base is `Ctx::isolate`'s whole reason for existing.
        let isolate_ctx = match charge {
            Charge::Tree => ctx.isolate(OutputSink::Buffer(Vec::new())),
            Charge::EngineReserve => ctx.handler_isolate(OutputSink::Buffer(Vec::new())),
        };

        Ok(match Wake::current() {
            Some(wake) => start_as_task(
                isolate_ctx,
                program,
                crossed,
                Rc::new(wake),
                output,
                receiving,
            ),
            // No task beneath the call, which takes a host installed by
            // something other than a running scheduler — `run_group`'s own
            // case. The program still has to run and the answer still has to be
            // right, so it runs on the caller's stack, at the spawn rather than
            // at the await: with no scheduler there is no concurrency to defer
            // it for. Nothing about the boundary weakens — it is the context,
            // not the stack.
            None => Box::new(Collected {
                completion: Some(run_here(isolate_ctx, program, crossed, receiving)),
                output,
            }),
        })
    }
}

/// A child already on a stack of its own, with nobody parked on it yet.
///
/// The state `run_as_task` used to hold across its own park loop, named and
/// handed to the parent instead — which is the whole of what splitting the
/// spawn from the await took.
struct Started {
    /// The child's task, so that a cancelled parent can reach it.
    id: TaskId,
    /// Where the child files its answer, once.
    slot: Rc<RefCell<Option<Completion>>>,
    /// Set by [`Ended`] however the child's body ended.
    done: Rc<Cell<bool>>,
    /// Whose stream the child's bytes go to at the join.
    output: Output,
}

impl std::fmt::Debug for Started {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Started")
            .field("done", &self.done.get())
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

/// A child that already ran, held until it is joined — the no-scheduler case,
/// where "started" and "finished" are the same moment.
#[derive(Debug)]
struct Collected {
    completion: Option<Completion>,
    output: Output,
}

impl Running for Collected {
    fn join(mut self: Box<Self>, ctx: &mut Ctx) -> Completion {
        let completion = self.completion.take().unwrap_or_else(cancelled_completion);
        hand_over(completion, self.output, ctx)
    }
}

impl Running for Started {
    fn join(self: Box<Self>, ctx: &mut Ctx) -> Completion {
        let mut cancelling = false;
        while !self.done.get() {
            let resumed = suspend_current(Waiting::Parked);
            if !resumed.suspended() {
                // Nothing suspended, so the core was never handed back and the
                // child cannot make progress; looping would spin. The same
                // refusal `suspend` documents.
                break;
            }
            if resumed.cancelled() && !cancelling {
                // This task was cancelled while it waited. The child dies with
                // it, and the loop keeps parking until it has — ADR 0072 § 4's
                // "control does not leave the call with work still running"
                // holds through a cancellation too.
                cancelling = true;
                cancel_task(self.id);
            }
        }

        let completion = self.slot.borrow_mut().take();
        hand_over(
            completion.unwrap_or_else(cancelled_completion),
            self.output,
            ctx,
        )
    }
}

/// Gives an [`Output::Inherit`] child's bytes to the parent's own stream, at the
/// await and nowhere else.
fn hand_over(mut completion: Completion, output: Output, ctx: &mut Ctx) -> Completion {
    if output == Output::Inherit {
        let bytes = std::mem::take(&mut completion.output);
        let _ = ctx.write_output(&bytes);
    }
    completion
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

/// The child onto a stack of its own, runnable before this returns.
///
/// A single-child sibling of `group::run_as_children`'s loop rather than a call
/// into it, because that runner gives every child a [`Ctx::child`] — the
/// *aliased* statics base ADR 0116 § 4 says an isolate may not have. What is
/// **not** here any more is the park: waiting is [`Started::join`]'s, and the
/// guarantee that control does not leave with the child still running is the
/// awaiting call's rather than this one's.
fn start_as_task(
    isolate_ctx: Ctx,
    program: Program,
    args: Value,
    wake: Rc<Wake>,
    output: Output,
    receiving: Option<ErrorClass>,
) -> Box<dyn Running> {
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
        *filed.borrow_mut() = Some(finish(child, answer, receiving.as_ref()));
        drop(ended);
    });

    let Some(id) = spawned else {
        // Unreachable from inside a turn: holding a `Wake` is `current_task`
        // and the tree both answering. Nothing ran, so nothing is owed.
        return Box::new(Collected {
            completion: Some(cancelled_completion()),
            output,
        });
    };

    Box::new(Started {
        id,
        slot,
        done,
        output,
    })
}

/// The child on the caller's own stack, for a host with no scheduler under it.
fn run_here(
    mut isolate_ctx: Ctx,
    program: Program,
    args: Value,
    receiving: Option<ErrorClass>,
) -> Completion {
    let answer = program(&mut isolate_ctx, args);
    finish(&mut isolate_ctx, answer, receiving.as_ref())
    // `isolate_ctx` is dropped here: ADR 0116 § 2's wholesale release.
}

/// Classifies how the isolate ended and copies its answer **out** of the arena
/// it was built in.
///
/// Called on the child's own stack, while its context is still alive, which is
/// the ordering ADR 0116 § 5 requires: the copy reads the child's graph, and the
/// caller's root owns what comes back.
fn finish(isolate_ctx: &mut Ctx, answer: Value, receiving: Option<&ErrorClass>) -> Completion {
    let cancelled = isolate_ctx.cancelled();
    let failed = !cancelled && isolate_ctx.pending().is_some();
    let thrown = failed.then(|| isolate_ctx.take_thrown());
    if let Some(thrown) = &thrown {
        // ADR 0020 § 3's tier 3, climbed here for the same reason `nvs run`
        // climbs it at the end of a program: an isolate *is* a program, and
        // this throw reached the top of it with nothing left to catch it. It
        // runs **before** the buffer is taken below, so the handler's own
        // output joins the child's and crosses at the await like every other
        // byte the child wrote.
        //
        // **Tier 4 is deliberately not reached from here.** ADR 0006's
        // failure-is-a-value already carries this throw back to the parent,
        // which is a reporter the CLI's own root task does not have, so a line
        // on `stderr` beside it would be a second report of one failure rather
        // than the floor beneath a missing one. What the operator asked for is
        // the handler; what the parent asked for is the value; neither is the
        // floor.
        // ADR 0020 § 2's tier 2, at the *isolate* root that section names
        // beside the request one: this child's own registration, fired before
        // the tier below it and before the buffer is taken, so its output
        // crosses at the await with everything else the child wrote. A parent's
        // handler is not reached from here — a registration is request-local
        // (`Ctx::set_uncaught_handler`), and ADR 0006's failure-is-a-value is
        // how this throw reaches the parent at all.
        isolate_ctx.run_uncaught_handler(thrown);
        let record = nvs_runtime::floor::uncaught(thrown);
        crate::ladder::escalate(isolate_ctx, &record);
    }
    let output = isolate_ctx.take_buffered_output().unwrap_or_default();
    // ADR 0088 § 4's declaration, taken beside the bytes it describes and on
    // every path out of here — a child that threw still wrote what it wrote,
    // and whether that reaches a peer is the collector's call rather than
    // this one's.
    let content_type = isolate_ctx.take_content_type();

    if let Some(thrown) = thrown {
        // ADR 0006's second row: the class name and the message as copied data.
        // The exception object stays in the arena that is about to go.
        let class = class_name(&thrown);
        release(answer);
        return Completion {
            ok: false,
            value: Value::null(),
            output,
            content_type,
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
        completion.content_type = content_type;
        return completion;
    }
    // Out, at the await. A refusal here is the child's, so it is a failure
    // value rather than an `Err` — the module doc owns the asymmetry. The
    // parent's table is named, so ADR 0023 § 2's third bullet is asked here
    // and not only of the payload carrier.
    let resolve = |name: &str| -> Option<*const nvs_runtime::ClassDesc> {
        Some(receiving?.sibling(name)?.desc())
    };
    match copy_graph_into(answer, Some(&resolve)) {
        Ok(value) => Completion {
            ok: true,
            value,
            output,
            content_type,
            error: None,
        },
        Err(refused) => Completion {
            ok: false,
            value: Value::null(),
            output,
            content_type,
            error: Some(Failure {
                class: "Error".to_owned(),
                message: refused.to_string(),
            }),
        },
    }
}

/// The answer for an isolate that was refused before it was built — today the
/// `[limits] max_script_depth` ceiling, and whatever else `Isolate::start`
/// learns to refuse without starting anything.
///
/// `Error` for the class rather than a named one, as [`Failure`]'s own field doc
/// asks: this failure is not a throw, and ADR 0020's rule that a `FATAL` never
/// reaches a `catch` is the reason there is no class here that a program could
/// name.
fn refused_completion(message: &str) -> Completion {
    Completion {
        ok: false,
        value: Value::null(),
        output: Vec::new(),
        content_type: None,
        error: Some(Failure {
            class: "Error".to_owned(),
            message: message.to_string(),
        }),
    }
}

/// The answer for an isolate that never reported: cancelled, or torn down before
/// its body finished.
fn cancelled_completion() -> Completion {
    Completion {
        ok: false,
        value: Value::null(),
        output: Vec::new(),
        content_type: None,
        error: Some(Failure {
            class: "Error".to_owned(),
            message: "the isolate was cancelled".to_owned(),
        }),
    }
}

/// The rendered class name behind a [`nvs_runtime::Thrown`], or ADR 0020's
/// generic one where there is no descriptor to read.
///
/// The answer is `nvs_runtime::Thrown`'s own, read from below rather than
/// derefed a second time here: ADR 0020 § 6's tier-4 floor asks the same
/// question of the same value, and the `unsafe` behind it belongs to the crate
/// that publishes the descriptor.
fn class_name(thrown: &nvs_runtime::Thrown) -> String {
    thrown.class_name()
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
    use crate::scheduler::{Scheduler, current_task};
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

    /// ADR 0116 § 5's copy is rooted in the **collector's** ownership, so a
    /// completion still holds the answer after the child's arena has gone and
    /// discarding it is what frees the graph. The collector that has to act on
    /// that is the one with nobody to answer — `nvs queue`'s worker, which runs
    /// a job and reports only whether it threw.
    ///
    /// Counted rather than read off a field, because a boundary that handed
    /// back a *borrow* of the child's arena would answer `ok` and `value`
    /// exactly as this one does while leaking one graph per job served. The
    /// null-answering half is the control: the same run owning nothing frees
    /// nothing, so the bytes below are the answer's and not the fixture's.
    #[test]
    fn a_completions_answer_is_held_here_until_it_is_discarded() {
        let freed_by_discarding = |answer: fn() -> Value| -> isize {
            let mut ctx = parent();
            let program: Program = Box::new(move |_: &mut Ctx, _args| answer());
            let mut done = run(
                Isolate::new(program, Value::null(), Output::Capture),
                &mut ctx,
            )
            .expect("a null argument crosses");
            assert!(done.ok, "{:?}", done.error);
            let holding = nvs_runtime::budget::live_bytes();
            done.discard_value();
            holding - nvs_runtime::budget::live_bytes()
        };

        let array = freed_by_discarding(|| {
            let mut out = nvs_runtime::NvsArray::new();
            out.set(nvs_runtime::NvsStr::new(b"receipted"), Value::bool(true));
            Value::array(out)
        });
        assert!(
            array > 0,
            "the child is gone and its answer is alive in this frame, so the \
             discard is what frees it — it freed {array} bytes"
        );
        assert_eq!(
            freed_by_discarding(Value::null),
            0,
            "and an answer owning nothing costs nothing to discard"
        );
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

    /// An instance of `name`, from a class table of its own. Leaked, for
    /// `closure_value`'s reason: a descriptor's address is its identity, and
    /// this fixture exists to have an address the parent's table never
    /// hands out.
    fn instance_of(name: &str) -> Value {
        let mut table = ClassTable::new();
        let id = table.define(name, &["x"], &[]);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        #[expect(unsafe_code, reason = "the leaked table outlives the object")]
        // SAFETY: the table is leaked, so the descriptor outlives every value.
        unsafe {
            Value::object(NvsObj::new(table.desc(id)))
        }
    }

    /// ADR 0023 § 2's third bullet, at this boundary: an object whose class the
    /// receiving side does not have is refused naming the class, and never
    /// arrives as a stub.
    ///
    /// Three legs, because the rule `Live::admit` records is *identity* and not
    /// the name: a class the parent never declared is refused, a class of a
    /// name the parent does declare but from another table is refused too —
    /// the copy keeps the descriptor it was built with, so admitting it would
    /// hand the parent the child's layout and the child's compiled methods —
    /// and the parent's own class crosses, which is what keeps the first two
    /// from passing for the wrong reason.
    #[test]
    fn an_unresolvable_class_is_refused_at_the_boundary() {
        let mut ctx = parent();

        let program: Program = Box::new(|_: &mut Ctx, _args| instance_of("Point"));
        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crossed; only the answer did not");
        assert!(!done.ok, "a class the parent cannot name may not cross");
        let failure = done.error.expect("a failure is a value, not an `Err`");
        assert!(
            failure.message.contains("Point"),
            "the refusal names the class: {failure:?}"
        );
        assert!(ctx.pending().is_none(), "the parent is not failing");

        let program: Program = Box::new(|_: &mut Ctx, _args| instance_of("Throwable"));
        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crossed; only the answer did not");
        assert!(!done.ok, "the same name is not the same class");
        let failure = done.error.expect("a failure is a value, not an `Err`");
        assert!(
            failure.message.contains("Throwable"),
            "the refusal names the class: {failure:?}"
        );

        let desc = ctx
            .class_desc("Throwable")
            .expect("the parent's own table declares it");
        let program: Program = Box::new(move |_: &mut Ctx, _args| {
            #[expect(unsafe_code, reason = "the parent's table outlives this isolate")]
            // SAFETY: the descriptor came out of the parent's own live table.
            unsafe {
                Value::object(NvsObj::new(desc))
            }
        });
        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crossed");
        assert!(done.ok, "the parent's own class crosses: {:?}", done.error);
        release(done.value);
    }

    /// ADR 0106 § 2's third fault, at this boundary: a **panic** inside the
    /// child is contained by the child's own task, so it arrives as one more
    /// `ok = false` beside an uncaught throw and a refused answer, and the
    /// parent is still running and still usable afterwards.
    ///
    /// Nothing here catches anything: the child's task is the containment
    /// boundary (`scheduler::run_task`), and what tells the awaiting side that
    /// the child is over is `Ended`'s `Drop` — which fires *because* it is a
    /// drop, half-way through the unwind, where a line at the end of the body
    /// would never be reached. The join then finds an unfiled slot, which is
    /// the same state a cancelled child leaves and is reported as such: the
    /// panic's own message is the scheduler's to carry, and does not reach a
    /// `Failure` a program can read.
    #[test]
    fn a_contained_panic_in_a_child_leaves_the_parent_running() {
        let mut ctx = parent();
        let program: Program = Box::new(|_: &mut Ctx, _args| panic!("the child gave up loudly"));

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("a panic is not an argument that could not cross");

        assert!(!done.ok, "a panicking child did not answer");
        assert!(done.error.is_some(), "the failure is a value");
        assert!(ctx.pending().is_none(), "the parent is not failing");

        // Still usable: the parent runs a second isolate to the end on the
        // same context, which is the whole of "leaves the parent running".
        let program: Program = Box::new(|child: &mut Ctx, _args| {
            child.write_output(b"still here").expect("a buffer");
            Value::null()
        });
        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crossed");
        assert!(done.ok, "{:?}", done.error);
        assert_eq!(done.output, b"still here");
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

    /// ADR 0072 §§ 4 and 5 from the *parent's* side: the task parked in
    /// [`Started::join`] is cancelled, and the boundary owes three things
    /// afterwards — the child's task is out of the tree, the join did not
    /// return while the child was still running, and what the child was holding
    /// is released rather than outliving the request charged for it.
    ///
    /// The parent stands on a [`nvs_runtime::HelperFrame`] because a real one
    /// does — `await` lowers to a `CoreCall` — so its stack is one no forced
    /// unwind may cross and the cancellation arrives as the notice
    /// [`Started::join`]'s loop reads rather than as a teardown of the parent.
    /// That is what makes this a test of the loop and not only of the
    /// scheduler's own sweep.
    #[test]
    fn a_cancelled_parent_leaves_no_orphan_and_no_leaked_arena() {
        /// What the child holds, and the flag it sets when it lets go. An `Rc`
        /// stands in for the arena because it is the one holder a test can
        /// count — the same stand-in `scheduler`'s own
        /// `a_cancelled_tasks_arena_is_released` uses.
        struct Held {
            released: Rc<Cell<bool>>,
            _arena: Rc<Vec<u8>>,
        }
        impl Drop for Held {
            fn drop(&mut self) {
                self.released.set(true);
            }
        }

        let arena = Rc::new(vec![0_u8; 64]);
        let held = Rc::clone(&arena);
        let released = Rc::new(Cell::new(false));
        let in_child = Rc::clone(&released);
        let at_join = Rc::clone(&released);
        let answer: Rc<RefCell<Option<Completion>>> = Rc::new(RefCell::new(None));
        let filed = Rc::clone(&answer);
        let over_at_join = Rc::new(Cell::new(false));
        let observed = Rc::clone(&over_at_join);

        let mut sched = Scheduler::new();
        let parent_id = sched.spawn(parent(), TaskRoot::Request, move |parent_ctx| {
            let program: Program = Box::new(move |_child: &mut Ctx, _args| {
                let _holding = Held {
                    released: in_child,
                    _arena: held,
                };
                // Parked on something that never arrives: the child is alive
                // and owes nothing, which is the state a cancellation has to
                // reach through the parent rather than through the child.
                suspend_current(Waiting::Parked);
                Value::null()
            });
            let _frame = nvs_runtime::HelperFrame::enter();
            let done = Isolate::new(program, Value::null(), Output::Capture)
                .run(parent_ctx)
                .expect("a null argument crosses");
            // Read at the join and nowhere else: § 4's "control does not leave
            // the call with work still running", holding through a
            // cancellation rather than only through a return.
            observed.set(at_join.get());
            *filed.borrow_mut() = Some(done);
        });

        sched.run();
        assert_eq!(
            sched.tracked_tasks(),
            2,
            "the isolate is a child of the awaiting task"
        );
        assert_eq!(sched.parked_count(), 2, "both are waiting on the other");
        assert_eq!(
            Rc::strong_count(&arena),
            2,
            "the child is not holding what it allocated"
        );

        assert_eq!(
            sched.cancel(parent_id),
            2,
            "the mark stopped at the awaiting task"
        );
        let report = sched.run();

        assert!(
            over_at_join.get(),
            "the join returned while the child was still running"
        );
        assert_eq!(report.cancelled, 1, "the child was not torn down");
        assert_eq!(
            sched.tracked_tasks(),
            0,
            "an orphan outlived the task that spawned it"
        );
        assert_eq!(sched.parked_count(), 0, "an orphan is still parked");
        assert_eq!(
            Rc::strong_count(&arena),
            1,
            "the cancelled isolate's memory outlived it"
        );

        let finished = sched.take_finished();
        assert_eq!(
            finished.len(),
            1,
            "the child's `Ctx` was filed instead of going down with its stack"
        );
        assert_eq!(
            finished[0].id, parent_id,
            "the parent did not reach its end"
        );

        let done = answer
            .borrow_mut()
            .take()
            .expect("the awaiting task ran to its end");
        assert!(!done.ok, "a cancelled isolate answered as if it had run");
        assert!(done.error.is_some(), "the failure is a value");
    }

    /// ADR 0072 § 5 from the *child's* side. A child that is running rather
    /// than parked is not unwound out of anybody's frame: it is told, and it
    /// dies at the next safepoint it reaches, having run nothing of its own on
    /// the way out. `scheduler`'s module doc § *The task tree, and what
    /// cancelling one costs* is the mechanism; what this pins at the boundary
    /// is the three consequences — the child stops at the safepoint and takes
    /// no further step, [`Ctx::cancelled`] is what its own body reads to know
    /// it must stop, and [`finish`] therefore classifies it as cancelled rather
    /// than as an answer, so the awaiting parent gets ADR 0006's failure value.
    ///
    /// The cancellation comes from a **sibling** task, because that is the only
    /// place it can come from while the child is still runnable: the parent is
    /// parked on the join, and a caller outside the scheduler can only act once
    /// the whole core has gone idle.
    #[test]
    fn a_child_is_cancelled_at_its_next_safepoint() {
        let steps = Rc::new(Cell::new(0_u32));
        let counted = Rc::clone(&steps);
        let watched = Rc::clone(&steps);
        let read_back = Rc::new(Cell::new(false));
        let noticed = Rc::clone(&read_back);
        let child_id: Rc<Cell<Option<TaskId>>> = Rc::new(Cell::new(None));
        let published = Rc::clone(&child_id);
        let at_cancel = Rc::new(Cell::new(0_u32));
        let stopped_at = Rc::clone(&at_cancel);
        let answer: Rc<RefCell<Option<Completion>>> = Rc::new(RefCell::new(None));
        let filed = Rc::clone(&answer);

        let mut sched = Scheduler::new();
        sched.spawn(parent(), TaskRoot::Request, move |parent_ctx| {
            let program: Program = Box::new(move |child_ctx: &mut Ctx, _args| {
                published.set(current_task());
                child_ctx.write_output(b"as far as here").expect("a buffer");
                // A `Core` member's stack is one no forced unwind may cross, so
                // a cancellation reaches this child as the notice its next
                // suspension answers rather than as a teardown — and answering
                // it is `Ctx::cancel`, ADR 0002's return status, with no
                // cleanup of the child's own running on the way out.
                let _frame = nvs_runtime::HelperFrame::enter();
                loop {
                    if suspend_current(Waiting::Yielded).cancelled() {
                        let _status = child_ctx.cancel();
                        break;
                    }
                    counted.set(counted.get() + 1);
                }
                noticed.set(child_ctx.cancelled());
                Value::null()
            });
            let _frame = nvs_runtime::HelperFrame::enter();
            let done = Isolate::new(program, Value::null(), Output::Capture)
                .run(parent_ctx)
                .expect("a null argument crosses");
            *filed.borrow_mut() = Some(done);
        });
        // The sibling: it lets the child get going, cancels it while it is on
        // the run queue, and records how far it had got — which is the count
        // the child may not move past.
        sched.spawn(parent(), TaskRoot::Request, move |_| {
            loop {
                if watched.get() > 0 {
                    let id = child_id.get().expect("the child never published an id");
                    assert_eq!(cancel_task(id), 1, "the child was already marked");
                    stopped_at.set(watched.get());
                    return;
                }
                suspend_current(Waiting::Yielded);
            }
        });

        sched.run();

        assert!(at_cancel.get() > 0, "the child never ran at all");
        assert_eq!(
            steps.get(),
            at_cancel.get(),
            "the child took a step past the safepoint it was cancelled at"
        );
        assert!(
            read_back.get(),
            "the child's own body could not tell it had been cancelled"
        );
        assert_eq!(sched.tracked_tasks(), 0, "the tree kept a dead task");
        assert_eq!(sched.parked_count(), 0, "the awaiting task was left parked");

        let done = answer
            .borrow_mut()
            .take()
            .expect("the awaiting task ran to its end");
        assert!(!done.ok, "a cancelled child answered as if it had finished");
        let failure = done.error.expect("the failure is a value");
        assert!(failure.message.contains("cancelled"), "{failure:?}");
        assert_eq!(
            done.output, b"as far as here",
            "what it echoed before the safepoint, it did echo"
        );
    }
}
