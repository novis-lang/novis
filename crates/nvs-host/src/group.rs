//! The one implementor of [`nvs_runtime::host::Host`]: a group of children run
//! on this core, with nothing of it still running when the call returns.
//!
//! [`nvs_runtime::host`]'s module doc owns the *seam* — why the trait is
//! declared in `nvs-runtime`, why it is reached through a thread-local, and why
//! what crosses is a whole group rather than a `spawn`/`wait`/`cancel` for
//! `nvs-stdlib` to sequence. This module is the other side of it, and what it
//! owns is the decisions that only the implementor can make.
//!
//! # 1. What a child gets for a `Ctx`
//!
//! [`crate::spawn_child`] takes an **owned** context, and
//! `rule:concurrency/a-child-belongs-to-the-calling-task`'s
//! children *share the request*. Those two only meet one way:
//! [`Ctx::child`](nvs_runtime::Ctx::child) builds a fresh context that **aliases
//! the request's static-property base** and owns everything else itself. The
//! line runs between what a *request* owns and what a *task* owns:
//!
//! - **Shared, by aliasing the pointer**: the static-property store. Compiled
//!   code loads a static inline through `Ctx`'s `statics` word, so a child with
//!   a store of its own would give one request two copies of every static and
//!   `Gauge::$live += 1` inside a child would be invisible to its siblings and
//!   to the parent. That is the field the acceptance program's `limit` block
//!   measures a peak with, so getting it wrong is not subtle.
//! - **Copied**: the origin, the debug flags, the runtime error class, the
//!   request's deadline word. Each is request-wide and small, and a copy of a
//!   value nothing writes during the group cannot diverge from its source.
//! - **Fresh, and spliced back**: output and the assertion ledger. A child gets
//!   an [`OutputSink::Buffer`](nvs_runtime::OutputSink) and this module appends
//!   the buffers to the parent **in job order** once the group is over. That
//!   is deliberate: a child's `echo` interleaved by completion order would make
//!   the output of a `Task::map` depend on which child finished first, which is
//!   exactly the non-determinism § 2's "preserving the input's keys and order"
//!   removes from the result. Assertions travel the same way, so an assertion
//!   inside a child still reaches `rule:testing/failure-ledger`'s ledger, which is the one
//!   record a `catch` cannot erase.
//! - **Fresh, and dropped**: the capture stack, the pending failure, the
//!   yielder, the stack bounds. A `Core\Out::capture` inside a child is scoped
//!   to that child, and the stack bounds are re-armed by [`crate::Scheduler`]
//!   from the child's own stack, which is the only code that knows it.
//!
//! The alias is what makes this module's structure load-bearing rather than
//! tidy. A child holds a bare pointer into its parent's storage, so **the
//! parent must outlive it**, and that is discharged twice over: `run_group`
//! does not return while a child is still running (§ 4), and a parent torn down
//! first cancels its children through [`crate::Scheduler`]'s tree, which then
//! unwinds them without resuming any of them — so no child ever executes an
//! instruction after its parent's context is gone.
//!
//! # 2. The sequence, which is the whole guarantee
//!
//! § 4's promise is that *control does not leave the call with work still
//! running*, and it is a property of the order these steps happen in:
//!
//! 1. Start children while the `limit` allows one and nothing has gone wrong.
//! 2. Park. Every child holds a guard whose [`Drop`] counts it out and wakes
//!    the parent — a guard rather than a line at the end of the body, because a
//!    **cancelled** child never reaches the end of its body and a child the
//!    scheduler tore down before its first resume never entered it.
//! 3. On the first throw, or on the deadline, cancel every child still running
//!    and drop every job not yet started.
//! 4. **Keep parking until the count reaches zero**, whichever of those ended
//!    it. This is the step that costs something and the one § 4 exists for: the
//!    call waits for the cancellations rather than returning as soon as it has
//!    decided what to answer.
//!
//! A `limit` of `0` is clamped to `1`. A bound of zero would mean the call can
//! never start anything and therefore can never return, and § 4 is a promise
//! that work does not outlive the call — not that the call may never happen.
//! Whether a `{limit: 0}` deserves a diagnostic is the member's question, not
//! this seam's.
//!
//! § 4's second throw is *written* rather than swallowed, which is all
//! `rule:concurrency/nothing-is-still-running-when-a-call-returns` asks. It goes to the failing
//! child's diagnostic channel — [`OutputSink::Stderr`](nvs_runtime::OutputSink) unless a test
//! moved it — and not to `Core\Log` (`nvs_stdlib::log`): pointing this seam at that class is
//! owed, and the destination is the only thing it changes.
//!
//! # What it spends
//!
//! Per group: one `Ctx` and one 1 MiB reserved (not resident) stack per child
//! *in flight* — the `limit` is what bounds that, and without one it is the
//! number of jobs — plus one answer slot per job and the output each child
//! buffered. All of it is freed when the call returns, so it is O(in-flight)
//! and not O(children ever spawned), per
//! `rule:programs/memory-priority`. Per thread: one
//! word, the installed pointer [`crate::Scheduler::run`] publishes.
//!
//! # Where a placement goes instead
//!
//! A child spawned `on: "worker"` reaches another core whichever entry form it
//! names: a method is code the compiled unit already holds and every core reads
//! that unit, and a path is compiled on the far core through the resolver the
//! process published. [`crate::placed`] is the one home of what each needs.
//! What is left here is the fall-through — a context with no class table, a
//! process that published no resolver, no core free — so
//! [`SchedulerHost::start_isolate`] asks [`crate::placed::destination_for`] and
//! runs the child in the body below on any answer but a core.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use nvs_runtime::host::{
    Bounds, Entry, Host, Job, Narrowing, Outcome, Output, Placement, Running, StartError, Woken,
};
use nvs_runtime::{AssertionOutcome, Ctx, OpenSpawn, SpawnForm, TaskRoot, Thrown, Value};

use crate::isolate::Isolate;

use crate::reactor;
use crate::scheduler::{
    TaskId, Waiting, Wake, cancel_task, current_task, spawn_child, suspend_current,
};

/// This crate's scheduler, seen through [`nvs_runtime::host`]'s seam.
///
/// A unit struct because every scrap of a scheduler's state is already
/// reachable from the thread it belongs to — the task tree, the reactor and the
/// timers are all thread-locals of this crate, and a host that carried a
/// pointer to any of them would be a second, staler route to the same thing.
#[derive(Debug)]
pub struct SchedulerHost;

/// The one instance, so that [`nvs_runtime::host::install`]'s `&'static` is a
/// borrow of a `static` rather than a leak.
static SCHEDULER_HOST: SchedulerHost = SchedulerHost;

/// Publishes this crate as the thread's host for as long as the guard lives.
///
/// Called by [`crate::Scheduler::run`] beside the task tree's own guard, so the
/// two have the same life: a `Core` member can reach a host exactly when it can
/// reach a tree to put children in.
pub(crate) fn install() -> nvs_runtime::host::Installed {
    nvs_runtime::host::install(&SCHEDULER_HOST)
}

impl Host for SchedulerHost {
    fn sleep(&self, duration: Duration) -> Woken {
        // The whole implementation, because the mechanism is already the one a
        // deadline uses: `crate::timer` arms this task's own deadline on the
        // reactor the core is about to poll, and falls back to blocking when
        // there is no task to park — see that module's docs.
        crate::timer::sleep(duration)
    }

    fn run_group(&self, ctx: &mut Ctx, jobs: Vec<Job>, bounds: Bounds) -> Outcome {
        if jobs.is_empty() {
            return Outcome::Completed(Vec::new());
        }
        match Wake::current() {
            Some(wake) => run_as_children(ctx, jobs, bounds, Rc::new(wake)),
            // No task beneath the call, which takes a host installed by
            // something other than a running scheduler. The jobs still have to
            // run and the answer still has to be right, so they run here, in
            // order, on the caller's own context — § 4's guarantee holds
            // trivially when there was never a second stack.
            None => run_here(ctx, jobs, bounds),
        }
    }

    fn waker(&self) -> Option<nvs_runtime::host::Waker> {
        // `Wake` is already the one-shot, fire-from-anywhere handle the seam
        // describes, so this is a box and nothing else. `None` outside a turn
        // is `Wake::current`'s own refusal, unchanged: there is no task here to
        // be woken later, and the member that asked has to say so rather than
        // park a core.
        let wake = Wake::current()?;
        Some(Box::new(move || wake.wake()))
    }

    fn park(&self, deadline: Option<Instant>) -> Woken {
        // A bound on the wait is a deadline like any other, so it is armed on
        // the reactor the core is about to poll rather than kept anywhere here
        // — `crate::timer` is the one clock, and its `wait_until` is `park`'s
        // own shape: a wake ends it, and so does the instant.
        if let Some(at) = deadline {
            return crate::timer::wait_until(at);
        }
        // `Waiting::Parked` rather than `Waiting::Yielded`: the task is off the
        // run queue until someone fires the wake it handed out, which is the
        // whole difference between waiting and being polite.
        let parked = suspend_current(Waiting::Parked);
        if parked.cancelled() {
            Woken::Cancelled
        } else {
            // Both a real wake and a park that could not be entered at all —
            // the seam's doc owns why those are one answer.
            Woken::Elapsed
        }
    }

    fn start_isolate(
        &self,
        ctx: &mut Ctx,
        entry: Entry,
        args: Value,
        output: Output,
        placement: Placement,
        narrowing: Narrowing,
    ) -> Result<Box<dyn Running>, StartError> {
        // The whole implementation: `crate::isolate` is `rule:security/isolate-shares-nothing`'s boundary and
        // decides everything about it, and what this seam adds is only that a
        // `Core` member can reach it without naming this crate. There is no
        // group here and no `Bounds` — an isolate is one child, and what bounds
        // it is the tree's budget rather than a per-call limit (ADR 0006
        // § *Budgets are accounted at the root of the request tree*).
        // The placement, asked before anything is built and before the argument
        // is consumed: `crate::placed::destination_for` answers `None` for every
        // reason a child cannot go to another core, and each of them is a
        // fall-through to the body below rather than a failure. A depth breach is
        // one of them — the refusal it produces is `Isolate::start`'s, written
        // once, and a child that may not start does not need a core first.
        //
        // The narrowing crosses with the child rather than deciding whether it
        // may: `nvs_runtime::PlacedIsolate` carries it, and the far core applies
        // it to the context it builds against ceilings this core resolved. That
        // is the only shape `rule:security/isolate-budget-is-the-trees` allows a
        // placement to take, because a narrowing silently not applied is the one
        // thing it cannot trade for a core.
        if placement == Placement::Worker
            && ctx.script_depth_breach().is_none()
            && let Some(destination) = crate::placed::destination_for(ctx, &entry)
        {
            let started =
                crate::placed::start(destination, ctx, entry, own(args), output, narrowing);
            return given_up_once_started(args, started);
        }
        let method = entry.is_method();
        // The name becomes code **here**, on the core that is about to run the
        // child, which is the whole of what the seam carrying a name rather
        // than a `Program` buys: a worker placement's `Entry` crosses to
        // another core and this line happens there instead. It stands ahead of
        // everything else for `Entry::program`'s own reason — the
        // `script.spawn` grant is asked inside it, and a spawn the grant does
        // not cover may not build so much as a context.
        // Nothing is released for `args` on an `Err` out of here, and that is the
        // spawn's ownership rule rather than an omission: `nvs_ir::lower`'s
        // `lower_spawn_script` leaves the transferred temporary on its stack
        // across this call — the one `CoreCall` site that does not forget it
        // first — so the fault edge `emit_fallible` built releases it on every
        // `Err` this answers with. [`given_up_once_started`] is how the two
        // starts below keep to it.
        let program = entry.program(ctx).map_err(StartError::Entry)?;
        let isolate = Isolate::new(program, own(args), output).narrowed_by(narrowing);
        let isolate = if method {
            isolate.running_a_method_of_the_parents_unit()
        } else {
            isolate
        };
        // `Placement::Here`, and every worker placement the arm above handed
        // back: a child the far core could not have prepared is still a child,
        // and running it here keeps every promise the placement makes but one —
        // it is its parent's child, charged to its tree, dying with it — and
        // loses only the core it asked for. The docs in `crate::placed` and
        // `crate::worker` say which placements those are.
        given_up_once_started(args, isolate.start(ctx))
    }
}

/// A second reference to the argument of a spawn, for the start to consume.
///
/// Both starts consume the reference they are handed on a refusal as well as on
/// a crossing, because the graph walk releases what it was given either way. The
/// seam leaves the caller's reference with the caller on an `Err`, so each start
/// is handed one of its own. **What it spends:** a temporary argument is
/// copied at the crossing where it could have been moved, because the walk sees
/// two references rather than one — one extra transient copy of that graph per
/// spawn, freed when the child's arena is.
fn own(args: Value) -> Value {
    #[expect(
        unsafe_code,
        reason = "the caller transferred a live reference to this seam, so the \
                  allocation is live while a second one is taken"
    )]
    // SAFETY: `args` is the reference the spawn transferred, live for the whole
    // call; [`given_up_once_started`] gives exactly one of the two back.
    unsafe {
        args.retain();
    }
    args
}

/// The seam's answer for a start that was handed [`own`]'s reference: the
/// caller's own reference is released once the child has started, which is the
/// transfer the success path promises, and left with the caller on a refusal.
fn given_up_once_started(
    args: Value,
    started: Result<Box<dyn Running>, nvs_runtime::graph::GraphError>,
) -> Result<Box<dyn Running>, StartError> {
    let running = started.map_err(StartError::Argument)?;
    #[expect(
        unsafe_code,
        reason = "the start consumed its own reference, so this one is the \
                  caller's, which the success path transfers here"
    )]
    // SAFETY: the lowering emits no release on the success path, so this is
    // the one release of the caller's reference.
    unsafe {
        args.release();
    }
    Ok(running)
}

/// Why a group stopped starting children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ending {
    /// A child threw. The first one by completion order is in [`Group::thrown`].
    Threw,
    /// The deadline passed before the last child returned.
    TimedOut,
    /// The **calling** task was cancelled while it waited here — `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s
    /// last row. It is an ending like the other two rather than an unwind
    /// through the call, because the call is standing on a helper frame and no
    /// unwind may cross one; `crate::scheduler`'s `Resume` owns that decision.
    /// The children are cancelled and waited for exactly as they are for the
    /// other two, and it is the *member* that then stops the request.
    Cancelled,
}

/// One job's answer and the request-wide state its child accumulated.
#[derive(Debug, Default)]
struct Slot {
    /// `Some` once the child returned without throwing. Still `None` at the end
    /// means the child was cancelled, or never started.
    answer: Option<Value>,
    output: Vec<u8>,
    assertions: Vec<AssertionOutcome>,
    /// How long the child's own body ran, which is what
    /// `rule:observability/spawn-is-its-own-event`'s split is computed against.
    /// `None` for a child nobody was observing, which is the same `None` that
    /// says no clock was read for it.
    wall: Option<Duration>,
}

/// What the parent and its children share for the length of the group.
#[derive(Debug)]
struct Group {
    slots: Vec<Slot>,
    /// Children spawned and not yet counted out by [`Child`]'s `Drop`. The
    /// module doc's step 4 is this reaching zero.
    outstanding: usize,
    /// The **first** throw by completion order; every later one is written
    /// rather than kept.
    thrown: Option<Thrown>,
}

/// A child's half of the group, moved into its body.
///
/// The `Drop` is the mechanism, not the bookkeeping: it fires when the body
/// finishes, when a forced unwind tears the body's stack down mid-way, and when
/// the scheduler drops a body it never resumed — the three ways a child can
/// stop existing, all of which the parent has to see.
struct Child {
    group: Rc<RefCell<Group>>,
    wake: Rc<Wake>,
    index: usize,
    /// Whether the parent opened a spawn event for this child, and therefore
    /// whether its body is timed at all.
    timed: bool,
}

impl Child {
    /// Runs this child's job and files what it produced.
    fn run(&self, job: Job, ctx: &mut Ctx) {
        // The child's half of the event's split, and the reason it is a flag
        // rather than a flags read here: a request nobody is observing must not
        // read a clock, and the parent already asked that question once.
        let began = self.timed.then(Instant::now);
        let answer = job(ctx);
        let wall = began.map(|at| at.elapsed());
        // A cancelled child did not fail. It ran until a member told it it was
        // cancelled and then stopped by `rule:errors/propagation`'s return status, so its
        // context carries a pending message the way every stopped request does
        // — and § 5 means that message is neither a `Throwable` the group may
        // propagate nor a second throw to write down. Its slot stays empty,
        // exactly as it does for a child a forced unwind tore down.
        let cancelled = ctx.cancelled();
        // Two statements, because `pending()` borrows the context and
        // `take_thrown` needs it back.
        let failed = !cancelled && ctx.pending().is_some();
        let thrown = failed.then(|| ctx.take_thrown());
        let output = ctx.take_buffered_output().unwrap_or_default();
        let assertions = ctx.take_assertions();

        let mut group = self.group.borrow_mut();
        let slot = &mut group.slots[self.index];
        slot.output = output;
        slot.assertions = assertions;
        slot.wall = wall;
        if cancelled {
            // What it echoed before it was stopped, it did echo, so the two
            // fields above are still filed. The value is not an answer.
            #[expect(
                unsafe_code,
                reason = "the job transferred this reference and no slot will \
                          take it"
            )]
            unsafe {
                answer.release();
            }
            return;
        }
        let Some(thrown) = thrown else {
            slot.answer = Some(answer);
            return;
        };
        // A helper that threw may still have handed back a value; it is not an
        // answer, and this is the one owner of it.
        #[expect(
            unsafe_code,
            reason = "the job transferred this reference and the throw means no \
                      slot will take it"
        )]
        unsafe {
            answer.release();
        }
        if group.thrown.is_none() {
            group.thrown = Some(thrown);
            return;
        }
        // `rule:concurrency/nothing-is-still-running-when-a-call-returns`: the second throw propagates nowhere and is never
        // swallowed. The module doc owns why this channel and not `Core\Log`.
        let message = format!("uncaught in a cancelled sibling: {}\n", thrown.message());
        drop(group);
        let _ = ctx.write_diagnostic(message.as_bytes());
    }
}

impl std::fmt::Debug for Child {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hand-written: deriving would print the whole group behind the `Rc`,
        // which every sibling also holds.
        f.debug_struct("Child").field("index", &self.index).finish()
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        let mut group = self.group.borrow_mut();
        group.outstanding = group.outstanding.saturating_sub(1);
        drop(group);
        // Unconditionally, including for a child that was torn down: the parent
        // is parked on this count and a wake naming a task that is already
        // runnable does nothing.
        self.wake.wake();
    }
}

/// The module doc's § 2 sequence, which is `rule:concurrency/nothing-is-still-running-when-a-call-returns`.
fn run_as_children(ctx: &mut Ctx, jobs: Vec<Job>, bounds: Bounds, wake: Rc<Wake>) -> Outcome {
    let count = jobs.len();
    let group = Rc::new(RefCell::new(Group {
        slots: (0..count).map(|_| Slot::default()).collect(),
        outstanding: 0,
        thrown: None,
    }));
    let mut queue: VecDeque<(usize, Job)> = jobs.into_iter().enumerate().collect();
    let mut running: Vec<TaskId> = Vec::new();
    let mut spawns: Vec<Option<OpenSpawn>> = (0..count).map(|_| None).collect();
    let limit = bounds
        .limit
        .map_or(usize::MAX, |limit| (limit as usize).max(1));
    let expires = bounds.deadline.map(|after| Instant::now() + after);
    let mut ending: Option<Ending> = None;

    loop {
        // 1. Start what the limit allows.
        while ending.is_none() && group.borrow().outstanding < limit {
            let Some((index, job)) = queue.pop_front() else {
                break;
            };
            // SAFETY: `Ctx::child` requires the parent to outlive the child and
            // no child to run after it is gone. This loop does not return while
            // `outstanding` is non-zero, and a parent torn down before that
            // cancels its children, which the scheduler unwinds without
            // resuming — the module doc's § 1 is the whole argument.
            #[expect(
                unsafe_code,
                reason = "the parent-outlives-child obligation is discharged by \
                          this function's own structure; see the module docs"
            )]
            let child_ctx = unsafe { ctx.child() };
            // A `Core\Task` child is `rule:observability/spawn-is-its-own-event`'s
            // `spawn`, and this is where one is started. The event, the clock
            // read behind it and the child's own timing all wait on the same
            // answer: `None` while both debug bits are off.
            let open = ctx.open_spawn(SpawnForm::Task);
            let child = Child {
                group: Rc::clone(&group),
                wake: Rc::clone(&wake),
                index,
                timed: open.is_some(),
            };
            spawns[index] = open;
            group.borrow_mut().outstanding += 1;
            let spawned = spawn_child(child_ctx, TaskRoot::Request, move |child_ctx| {
                child.run(job, child_ctx);
            });
            match spawned {
                Some(id) => running.push(id),
                // Unreachable from inside a turn: holding a `Wake` is exactly
                // `current_task` and the tree both answering. The guard has
                // already counted the child back out, so the slot stays empty
                // and the collector below fills it rather than losing a
                // position in the answer vector.
                None => break,
            }
        }

        // 2. Are we there?
        if group.borrow().outstanding == 0 && (ending.is_some() || queue.is_empty()) {
            break;
        }

        // 3. Park until a child reports, the deadline lands, or this task is
        //    itself cancelled.
        let armed = arm_deadline(ending, expires);
        let resumed = suspend_current(Waiting::Parked);
        let suspended = resumed.suspended();
        if armed {
            disarm_deadline();
        }
        if resumed.cancelled() && ending.is_none() {
            ending = Some(Ending::Cancelled);
        }
        if !suspended {
            // Nothing suspended, so the core was never handed back and no child
            // can make progress; looping would spin. Same refusal `suspend`
            // documents, and it cannot be reached with a `Wake` in hand.
            break;
        }

        // 4. Decide whether this is the wake that ends the group.
        if ending.is_none() && group.borrow().thrown.is_some() {
            ending = Some(Ending::Threw);
        }
        if ending.is_none() && expires.is_some_and(|at| Instant::now() >= at) {
            ending = Some(Ending::TimedOut);
        }
        if ending.is_some() && !queue.is_empty() {
            // Dropped rather than run: § 4 has already decided this call's
            // answer, and `Job`'s own docs carry the release obligation that
            // makes dropping one safe.
            queue.clear();
        }
        if ending.is_some() {
            for id in running.drain(..) {
                cancel_task(id);
            }
        }
    }

    // Every child's join is this one point, and
    // `rule:concurrency/nothing-is-still-running-when-a-call-returns` is what
    // makes it one: the loop above does not reach here while any child of this
    // group is still running.
    for (index, open) in spawns.into_iter().enumerate() {
        let Some(open) = open else {
            continue;
        };
        let wall = group.borrow().slots[index].wall;
        ctx.close_spawn(open, wall);
    }
    collect(ctx, &group, ending)
}

/// Files the parent's own deadline with the core's timers, so the wait ends on
/// the clock and not only on a child.
///
/// `false` when there is nothing to arm or no reactor to arm it on — a group
/// with a deadline still notices it, one wake later, because the check in step 4
/// reads the clock rather than the timer.
fn arm_deadline(ending: Option<Ending>, expires: Option<Instant>) -> bool {
    if ending.is_some() {
        return false;
    }
    let Some(at) = expires else {
        return false;
    };
    let Some(me) = current_task() else {
        return false;
    };
    reactor::with_current(|reactor| reactor.timers().arm(me, at)).is_some()
}

/// Takes this task's deadline back off the core's timers.
fn disarm_deadline() {
    if let Some(me) = current_task() {
        reactor::with_current(|reactor| reactor.timers().disarm(me));
    }
}

/// Splices what the children produced back into the request and answers § 4's
/// table.
fn collect(ctx: &mut Ctx, group: &Rc<RefCell<Group>>, ending: Option<Ending>) -> Outcome {
    let mut group = group.borrow_mut();

    // Job order, not completion order — the module doc's § 1 owns why, and it
    // happens on every path because a child that echoed before a sibling threw
    // did echo.
    for slot in &mut group.slots {
        if !slot.output.is_empty() {
            let output = std::mem::take(&mut slot.output);
            let _ = ctx.write_output(&output);
        }
        for assertion in std::mem::take(&mut slot.assertions) {
            ctx.record_assertion(assertion.member, assertion.failure);
        }
    }

    let mut failed = group.thrown.take();
    if ending == Some(Ending::Cancelled)
        && let Some(thrown) = failed.take()
    {
        // The caller is being torn down, so there is nobody left to propagate
        // to — and § 4's "never swallowed" holds all the same, through the
        // channel a second throw already uses.
        let message = format!("uncaught in a cancelled group: {}\n", thrown.message());
        let _ = ctx.write_diagnostic(message.as_bytes());
    }
    if failed.is_some() || matches!(ending, Some(Ending::TimedOut | Ending::Cancelled)) {
        for slot in &mut group.slots {
            if let Some(answer) = slot.answer.take() {
                #[expect(
                    unsafe_code,
                    reason = "the group is not returning these, so this is the \
                              last owner of each"
                )]
                unsafe {
                    answer.release();
                }
            }
        }
        return match (failed, ending) {
            (Some(thrown), _) => Outcome::Threw(thrown),
            (None, Some(Ending::Cancelled)) => Outcome::Cancelled,
            (None, _) => Outcome::TimedOut,
        };
    }

    let answers = group
        .slots
        .iter_mut()
        // `None` here means a child neither returned nor threw while the group
        // was still completing, which takes a spawn the scheduler refused from
        // inside its own turn. Null keeps the vector the length the member's
        // return type promises rather than shortening it silently.
        .map(|slot| slot.answer.take().unwrap_or_else(Value::null))
        .collect();
    Outcome::Completed(answers)
}

/// Runs the jobs one after another on the caller's own context.
///
/// The answer for a host installed with no task beneath it. It is the same
/// table: the first throw stops the rest, an expired deadline stops the rest,
/// and nothing is running when it returns — trivially, because nothing ever ran
/// anywhere but here.
fn run_here(ctx: &mut Ctx, jobs: Vec<Job>, bounds: Bounds) -> Outcome {
    let expires = bounds.deadline.map(|after| Instant::now() + after);
    let mut answers: Vec<Value> = Vec::with_capacity(jobs.len());
    let mut jobs = jobs.into_iter();
    let mut ending = None;
    for job in jobs.by_ref() {
        if expires.is_some_and(|at| Instant::now() >= at) {
            ending = Some(Ending::TimedOut);
            break;
        }
        let answer = job(ctx);
        if ctx.pending().is_some() {
            ending = Some(Ending::Threw);
            answers.push(answer);
            break;
        }
        answers.push(answer);
    }
    // The rest are dropped unrun, which `Job`'s own docs make safe.
    drop(jobs);
    match ending {
        None => Outcome::Completed(answers),
        Some(ending) => {
            for answer in answers {
                #[expect(
                    unsafe_code,
                    reason = "the group is not returning these, so this is the \
                              last owner of each"
                )]
                unsafe {
                    answer.release();
                }
            }
            match ending {
                Ending::Threw => Outcome::Threw(ctx.take_thrown()),
                Ending::TimedOut => Outcome::TimedOut,
                // Unreachable here: this path has no task, so there is nothing
                // for a scheduler to have cancelled — the jobs ran on the
                // caller's own stack, one after another.
                Ending::Cancelled => Outcome::Cancelled,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactor::{Reactor, run_until_idle};
    use crate::scheduler::Scheduler;
    use nvs_runtime::host::{Bounds, with_current};
    use nvs_runtime::{DebugFlags, OutputSink, TraceKind, Value};
    use std::time::Duration;

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    /// A context that can promote a bare-message failure into an exception
    /// object, which is what every embedder installs before a request runs.
    ///
    /// Without one, `Ctx::take_thrown` answers a null [`Thrown`] and the
    /// message is gone — the same everywhere, not a property of this seam. A
    /// child inherits the class through [`Ctx::child`], so a group that
    /// propagates a real message is also the assertion that it does.
    fn ctx_with_error_class() -> Ctx {
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

    /// The shape a `Core` member has: no argument carries the host in.
    fn group(ctx: &mut Ctx, jobs: Vec<Job>, bounds: Bounds) -> Outcome {
        with_current(|host| host.run_group(ctx, jobs, bounds))
            .expect("`Scheduler::run` installs a host for the length of a turn")
    }

    fn answers(outcome: &Outcome) -> Vec<i64> {
        match outcome {
            Outcome::Completed(values) => values
                .iter()
                .map(|value| value.as_int().expect("every job answered an int"))
                .collect(),
            other => panic!("expected a completed group, got {other:?}"),
        }
    }

    /// The result is in **job** order even when the children finish in another
    /// one, which is `rule:concurrency/map-preserves-keys-and-order`'s "preserving the input's keys and order"
    /// one level below the member that promises it.
    #[test]
    fn the_answers_come_back_in_job_order_and_not_completion_order() {
        let mut sched = Scheduler::new();
        let seen: Rc<RefCell<Vec<Outcome>>> = Rc::new(RefCell::new(Vec::new()));
        let out = Rc::clone(&seen);

        sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
            // Three children, each yielding a different number of times, so the
            // completion order is 2, 1, 0 and not 0, 1, 2.
            let jobs: Vec<Job> = (0..3)
                .map(|index| {
                    Box::new(move |_: &mut Ctx| {
                        for _ in 0..(3 - index) {
                            suspend_current(Waiting::Yielded);
                        }
                        Value::int(index)
                    }) as Job
                })
                .collect();
            out.borrow_mut().push(group(ctx, jobs, Bounds::default()));
        });
        sched.run();

        assert_eq!(answers(&seen.borrow()[0]), vec![0, 1, 2]);
    }

    /// § 4's whole point, asserted from inside the call's own frame: the line
    /// after `run_group` returns already knows every child ended.
    #[test]
    fn no_child_is_still_running_when_the_call_returns() {
        let mut sched = Scheduler::new();
        let ended = Rc::new(std::cell::Cell::new(0_usize));
        let counter = Rc::clone(&ended);
        let at_return = Rc::new(std::cell::Cell::new(usize::MAX));
        let seen = Rc::clone(&at_return);

        sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
            let jobs: Vec<Job> = (0..4)
                .map(|index| {
                    let counter = Rc::clone(&counter);
                    Box::new(move |_: &mut Ctx| {
                        suspend_current(Waiting::Yielded);
                        counter.set(counter.get() + 1);
                        Value::int(index)
                    }) as Job
                })
                .collect();
            let outcome = group(ctx, jobs, Bounds::default());
            assert!(matches!(outcome, Outcome::Completed(_)));
            seen.set(counter.get());
        });
        let report = sched.run();

        assert_eq!(
            at_return.get(),
            4,
            "every child had ended before the call returned"
        );
        assert_eq!(sched.parked_count(), 0);
        assert_eq!(sched.tracked_tasks(), 0);
        assert_eq!(report.finished, 5, "the parent and its four children");
    }

    /// A child that throws cancels its siblings, the call waits for those
    /// cancellations, and the **first** throw is the one that comes back.
    #[test]
    fn a_throw_cancels_every_sibling_and_the_call_waits_for_them() {
        let mut sched = Scheduler::new();
        let ran = Rc::new(std::cell::Cell::new(0_u32));
        let counter = Rc::clone(&ran);
        let threw = Rc::new(RefCell::new(String::new()));
        let message = Rc::clone(&threw);

        sched.spawn(ctx_with_error_class(), TaskRoot::Request, move |ctx| {
            let mut jobs: Vec<Job> = Vec::new();
            jobs.push(Box::new(|ctx: &mut Ctx| {
                ctx.set_pending("the first throw");
                Value::null()
            }));
            for _ in 0..3 {
                let counter = Rc::clone(&counter);
                jobs.push(Box::new(move |_: &mut Ctx| {
                    // Parks forever: only a cancellation can end this child, so
                    // the call returning at all is the proof it waited for one.
                    suspend_current(Waiting::Parked);
                    counter.set(counter.get() + 1);
                    Value::int(0)
                }));
            }
            match group(ctx, jobs, Bounds::default()) {
                Outcome::Threw(thrown) => *message.borrow_mut() = thrown.message(),
                other => panic!("expected a throw, got {other:?}"),
            }
        });
        let report = sched.run();

        assert_eq!(&*threw.borrow(), "the first throw");
        assert_eq!(
            ran.get(),
            0,
            "a cancelled child runs no further instruction"
        );
        assert_eq!(sched.parked_count(), 0);
        assert_eq!(sched.tracked_tasks(), 0);
        assert_eq!(report.cancelled, 3);
    }

    /// § 3's `limit` shapes concurrency rather than throwing, and what proves
    /// it is the **peak** — a total would be the same either way.
    #[test]
    fn a_limit_bounds_how_many_children_run_at_once() {
        let mut sched = Scheduler::new();
        let peak = Rc::new(std::cell::Cell::new(0_usize));
        let live = Rc::new(std::cell::Cell::new(0_usize));
        let inner = Rc::clone(&peak);

        sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
            let jobs: Vec<Job> = (0..6)
                .map(|index| {
                    let peak = Rc::clone(&inner);
                    let live = Rc::clone(&live);
                    Box::new(move |_: &mut Ctx| {
                        live.set(live.get() + 1);
                        peak.set(peak.get().max(live.get()));
                        // Held across a suspend, so a limit that was ignored
                        // would show all six at once.
                        suspend_current(Waiting::Yielded);
                        live.set(live.get() - 1);
                        Value::int(index)
                    }) as Job
                })
                .collect();
            let bounds = Bounds {
                limit: Some(2),
                deadline: None,
            };
            assert_eq!(answers(&group(ctx, jobs, bounds)), vec![0, 1, 2, 3, 4, 5]);
        });
        sched.run();

        assert_eq!(peak.get(), 2, "never more than the limit at once");
    }

    /// The deadline cancels every child and the call waits, which is § 4's
    /// fourth row. It needs a reactor, because the parent's own wait is what
    /// the clock has to end.
    #[test]
    fn the_deadline_cancels_every_child_and_the_call_waits() {
        let _reactor = reactor::install(Reactor::new().expect("the OS refused a poll"));
        let mut sched = Scheduler::new();
        let timed_out = Rc::new(std::cell::Cell::new(false));
        let seen = Rc::clone(&timed_out);

        sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
            let jobs: Vec<Job> = (0..2)
                .map(|index| {
                    Box::new(move |_: &mut Ctx| {
                        crate::timer::sleep(Duration::from_secs(30));
                        Value::int(index)
                    }) as Job
                })
                .collect();
            let bounds = Bounds {
                limit: None,
                deadline: Some(Duration::from_millis(20)),
            };
            seen.set(matches!(group(ctx, jobs, bounds), Outcome::TimedOut));
        });
        run_until_idle(&mut sched).expect("the reactor refused a poll");

        assert!(
            timed_out.get(),
            "the group ends on the clock, not on a child"
        );
        assert_eq!(sched.parked_count(), 0);
        assert_eq!(sched.tracked_tasks(), 0);
    }

    /// Two jobs that can only ever wait for each other: each holds one
    /// channel's receiving end and the other's sending end, and neither reaches
    /// its `send` because neither's receive can return. Both senders stay alive
    /// inside the jobs, so nothing disconnects and nothing errors — this is a
    /// wait, which is what makes it the deadlock a program writes by accident.
    ///
    /// `reached` counts the children that entered the wait and `past` the
    /// instructions run after one, so a pair that never started and a pair that
    /// was resumed after its cancellation fail on different lines.
    fn a_deadlocked_pair(
        reached: &Rc<std::cell::Cell<u32>>,
        past: &Rc<std::cell::Cell<u32>>,
    ) -> Vec<Job> {
        let (tx_a, rx_a) = crate::channel::channel::<i64>(1);
        let (tx_b, rx_b) = crate::channel::channel::<i64>(1);
        let (in_a, in_b) = (Rc::clone(reached), Rc::clone(reached));
        let (past_a, past_b) = (Rc::clone(past), Rc::clone(past));
        vec![
            Box::new(move |_: &mut Ctx| {
                in_a.set(in_a.get() + 1);
                let carried = rx_a.recv().unwrap_or(0);
                past_a.set(past_a.get() + 1);
                let _ = tx_b.send(carried);
                Value::int(0)
            }) as Job,
            Box::new(move |_: &mut Ctx| {
                in_b.set(in_b.get() + 1);
                let carried = rx_b.recv().unwrap_or(0);
                past_b.set(past_b.get() + 1);
                let _ = tx_a.send(carried);
                Value::int(1)
            }) as Job,
        ]
    }

    /// A deliberate deadlock ends at the deadline like any other group that has
    /// not finished, and the call still does not return until both children are
    /// torn down where they park.
    ///
    /// The pair waits on a channel rather than on a bare park because that is
    /// the shape a program reaches the same state through, and because a
    /// channel is where a cancellation that left a registration behind would
    /// show: `crate::channel`'s § *A cancelled waiter takes itself out of the
    /// queue* is the claim, and a parked count of zero here is it.
    #[test]
    fn two_tasks_waiting_on_each_others_channel_are_ended_by_the_groups_deadline() {
        let _reactor = reactor::install(Reactor::new().expect("the OS refused a poll"));
        let mut sched = Scheduler::new();
        let reached = Rc::new(std::cell::Cell::new(0_u32));
        let past = Rc::new(std::cell::Cell::new(0_u32));
        let timed_out = Rc::new(std::cell::Cell::new(false));
        let seen = Rc::clone(&timed_out);
        let (entered, further) = (Rc::clone(&reached), Rc::clone(&past));

        sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
            let jobs = a_deadlocked_pair(&entered, &further);
            let bounds = Bounds {
                limit: None,
                deadline: Some(Duration::from_millis(20)),
            };
            seen.set(matches!(group(ctx, jobs, bounds), Outcome::TimedOut));
        });
        run_until_idle(&mut sched).expect("the reactor refused a poll");

        assert!(
            timed_out.get(),
            "the deadlock ended as something but the clock"
        );
        assert_eq!(
            reached.get(),
            2,
            "a child never reached the wait it deadlocks in"
        );
        assert_eq!(
            past.get(),
            0,
            "a cancelled child ran an instruction past its wait"
        );
        assert_eq!(
            sched.parked_count(),
            0,
            "a child was left parked on a channel"
        );
        assert_eq!(sched.tracked_tasks(), 0);
    }

    /// The same deadlock with no deadline at all: what ends it is the
    /// cancellation of the task awaiting it, which is § 4's last row. The
    /// children die with that wait rather than outliving it, and the call
    /// answers [`Outcome::Cancelled`] rather than unwinding through itself.
    #[test]
    fn a_deadlocked_pair_is_ended_by_cancelling_the_task_that_awaits_them() {
        let mut sched = Scheduler::new();
        let reached = Rc::new(std::cell::Cell::new(0_u32));
        let past = Rc::new(std::cell::Cell::new(0_u32));
        let cancelled = Rc::new(std::cell::Cell::new(false));
        let seen = Rc::clone(&cancelled);
        let (entered, further) = (Rc::clone(&reached), Rc::clone(&past));

        let awaiter = sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
            // What a `Core\Task::all` stands on, and the reason this answer is a
            // return value at all: a stack carrying a helper frame is one no
            // forced unwind may cross, so the scheduler resumes the awaiter with
            // its notice instead of tearing it down where it parks. Without the
            // frame this call never comes back and the group's last row could
            // not be observed from anywhere.
            let _frame = nvs_runtime::HelperFrame::enter();
            let jobs = a_deadlocked_pair(&entered, &further);
            seen.set(matches!(
                group(ctx, jobs, Bounds::default()),
                Outcome::Cancelled
            ));
        });
        // A second root, because the cancellation has to arrive from outside a
        // wait that by construction never ends. Its yields are what give the
        // group its turns to start both children and park on them.
        sched.spawn(ctx(), TaskRoot::Request, move |_| {
            for _ in 0..4 {
                suspend_current(Waiting::Yielded);
            }
            cancel_task(awaiter);
        });
        sched.run();

        assert!(
            cancelled.get(),
            "the group answered something other than its awaiter's cancellation"
        );
        assert_eq!(
            reached.get(),
            2,
            "a child never reached the wait it deadlocks in"
        );
        assert_eq!(
            past.get(),
            0,
            "a cancelled child ran an instruction past its wait"
        );
        assert_eq!(
            sched.parked_count(),
            0,
            "a child outlived the wait that owned it"
        );
        assert_eq!(sched.tracked_tasks(), 0);
    }

    /// One event per child, each closed with the child's own wall time — the
    /// split `rule:observability/spawn-is-its-own-event` asks a group for, and
    /// the form is the rule's `spawn` rather than either isolate spelling.
    #[test]
    fn a_traced_task_group_records_a_spawn_event_per_child() {
        let mut sched = Scheduler::new();
        let mut parent = ctx();
        parent.set_debug_flags(DebugFlags::TRACE);
        let events = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&events);

        sched.spawn(parent, TaskRoot::Request, move |ctx| {
            let jobs: Vec<Job> = (0..3)
                .map(|index| Box::new(move |_: &mut Ctx| Value::int(index)) as Job)
                .collect();
            assert_eq!(answers(&group(ctx, jobs, Bounds::default())), vec![0, 1, 2]);
            *seen.borrow_mut() = ctx
                .trace()
                .iter()
                .filter(|event| event.kind == TraceKind::Spawn)
                .map(|event| event.callee.clone())
                .collect();
        });
        sched.run();

        let events = events.borrow();
        assert_eq!(events.len(), 3, "one event per child, not one per group");
        for event in events.iter() {
            assert!(event.starts_with("spawn started="), "{event}");
            assert!(
                event.contains(" joined=") && event.contains(" child="),
                "a joined child carries both timestamps and the split: {event}"
            );
        }
    }

    /// The gate every probe in the tree shares: both bits off is no event, and
    /// the clock that would have made one is never read.
    #[test]
    fn an_untraced_spawn_records_no_spawn_event() {
        let mut sched = Scheduler::new();
        let untraced = Rc::new(std::cell::Cell::new(false));
        let seen = Rc::clone(&untraced);

        sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
            let jobs: Vec<Job> = (0..2)
                .map(|index| Box::new(move |_: &mut Ctx| Value::int(index)) as Job)
                .collect();
            assert_eq!(answers(&group(ctx, jobs, Bounds::default())), vec![0, 1]);
            seen.set(ctx.trace().is_empty());
        });
        sched.run();

        assert!(
            untraced.get(),
            "a request nobody is observing recorded an event"
        );
    }

    /// The children share the request's statics, which is the whole of what
    /// "share the request" means for the acceptance program's gauge.
    #[test]
    fn a_child_writes_the_request_s_statics_and_its_siblings_see_it() {
        let mut sched = Scheduler::new();
        let mut parent = ctx();
        parent.install_statics(std::sync::Arc::from(vec![None]));
        let total = Rc::new(std::cell::Cell::new(0_i64));
        let seen = Rc::clone(&total);

        sched.spawn(parent, TaskRoot::Request, move |ctx| {
            let jobs: Vec<Job> = (1..=3)
                .map(|step| {
                    Box::new(move |child: &mut Ctx| {
                        // The base pointer is the parent's storage, so this is
                        // the read-modify-write compiled code would emit.
                        let slot = child.statics_base();
                        #[expect(unsafe_code, reason = "the alias this test exists to pin")]
                        unsafe {
                            let was = (*slot).as_int().unwrap_or(0);
                            *slot = Value::int(was + step);
                        }
                        suspend_current(Waiting::Yielded);
                        Value::int(step)
                    }) as Job
                })
                .collect();
            assert_eq!(answers(&group(ctx, jobs, Bounds::default())), vec![1, 2, 3]);
            let slot = ctx.statics_base();
            #[expect(unsafe_code, reason = "the alias this test exists to pin")]
            unsafe {
                seen.set((*slot).as_int().unwrap_or(0));
            }
        });
        sched.run();

        assert_eq!(
            total.get(),
            6,
            "one store, three writers, the parent reads it"
        );
    }

    /// A child's `echo` reaches the request in **job** order, whatever order the
    /// children finished in.
    #[test]
    fn a_child_s_output_reaches_the_request_in_job_order() {
        let mut sched = Scheduler::new();
        let printed = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&printed);

        sched.spawn(Ctx::buffered(), TaskRoot::Request, move |ctx| {
            let jobs: Vec<Job> = (0..3)
                .map(|index| {
                    Box::new(move |child: &mut Ctx| {
                        for _ in 0..(3 - index) {
                            suspend_current(Waiting::Yielded);
                        }
                        let _ = child.write_output(format!("{index}").as_bytes());
                        Value::int(index)
                    }) as Job
                })
                .collect();
            let _ = group(ctx, jobs, Bounds::default());
            *seen.borrow_mut() = ctx.take_buffered_output().unwrap_or_default();
        });
        sched.run();

        assert_eq!(String::from_utf8(printed.borrow().clone()).unwrap(), "012");
    }

    /// The fallback: a host reached with no task beneath it still answers, and
    /// still answers § 4's table.
    #[test]
    fn a_group_with_no_task_beneath_it_runs_its_jobs_here() {
        let _installed = install();
        let mut ctx = ctx_with_error_class();

        let jobs: Vec<Job> = (0..3)
            .map(|n| Box::new(move |_: &mut Ctx| Value::int(n)) as Job)
            .collect();
        assert_eq!(
            answers(&group(&mut ctx, jobs, Bounds::default())),
            vec![0, 1, 2]
        );

        let mut jobs: Vec<Job> = Vec::new();
        jobs.push(Box::new(|ctx: &mut Ctx| {
            ctx.set_pending("no scheduler, same table");
            Value::null()
        }));
        let reached = Rc::new(std::cell::Cell::new(false));
        let flag = Rc::clone(&reached);
        jobs.push(Box::new(move |_: &mut Ctx| {
            flag.set(true);
            Value::null()
        }));
        match group(&mut ctx, jobs, Bounds::default()) {
            Outcome::Threw(thrown) => assert_eq!(thrown.message(), "no scheduler, same table"),
            other => panic!("expected a throw, got {other:?}"),
        }
        assert!(!reached.get(), "the first throw stops the rest");
    }
}
