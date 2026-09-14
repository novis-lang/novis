//! One core, one scheduler, one run queue of stackful coroutines.
//!
//! This is `rule:http-server/a-core-is-never-blocked-on-a-syscall`
//! 's "one single-threaded scheduler of our own pinned per core", and the
//! shape `benches/abi-probe` models — its `Ctx` doc calls
//! itself "deliberately shaped like the real `Ctx` will be", and this module is
//! the real one.
//!
//! # Why a task never leaves the thread it started on
//!
//! `docs/plan/design.md` § *Thread-per-core, shared-nothing runtime* is the one
//! home for that decision; what matters here is that this module is where it is
//! either held or lost. A [`Scheduler`] is `!Send` and `!Sync` — structurally,
//! not by convention — so there is no safe way to hand a run queue, or anything
//! sitting in it, to a second thread. That is what buys the non-atomic
//! refcount in `nvs-runtime`: a `Value`'s refcount is incremented without a
//! `lock` prefix because no other thread can ever hold a reference to it, and
//! `Ctx` is itself `!Send` for the same reason. **A future stealing scheduler
//! is not a tuning change here; it is a change to every refcount in the
//! runtime**, which is why the property is written into the type rather than
//! into a comment.
//!
//! # What suspending costs, and what it does not
//!
//! A task is a stackful coroutine, so suspending it is a stack switch: the
//! registers are saved, the stack pointer moves, and the frames underneath —
//! including JIT-compiled ones — stay exactly where they are. **Nothing in the
//! call chain is marked `async`.** A helper that has to wait reaches its
//! yielder through the [`Ctx`] it was already handed ([`suspend`]), so
//! `Core\IO::read` has the same signature whether or not there is a scheduler
//! beneath it, and Novis's surface never grows a colour.
//!
//! What it spends, as `rule:programs/memory-priority`
//! requires: **one stack per in-flight task**, held for as long as that task is
//! suspended and handed back to the worker's pool when it completes. That is
//! O(in-flight) and not O(requests served), which is the test that section
//! applies. How wide that stack is reserved, how little of it is ever resident,
//! and what the pool itself costs is `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled` — spelled in
//! [`crate::stack`], which is that policy's only home. This module takes a
//! stack, arms the task's recursion limit from it, and gives it back.
//!
//! # The task tree, and what cancelling one costs
//!
//! Every task has a parent and a list of children, and the parent is **taken
//! from the task that spawned it** rather than passed in: [`spawn_child`] reads
//! [`current_task`], so
//! `rule:concurrency/a-child-belongs-to-the-calling-task`'s
//! "each is a child of the calling task" is a property of the call rather than
//! of a caller's diligence. A task spawned from the worker itself
//! ([`Scheduler::spawn`], with nothing running) is a root. No ADR slot is free
//! for the rest of it, so the decisions below are recorded here, which is the
//! module they are held or lost in.
//!
//! **A running task cannot reach the `&mut Scheduler` that is resuming it**, so
//! [`spawn_child`] issues the child's [`TaskId`], links it into the tree, and
//! leaves the coroutine itself unbuilt until the scheduler's own turn. The tree
//! is therefore the half of a scheduler a task can reach — an `Rc<RefCell<..>>`
//! published in a thread-local for the length of [`Scheduler::run`], the shape
//! [`crate::reactor`] already uses for the reactor — while the run queue and the
//! stack pool stay behind `&mut self`. The parent still has the child's id the
//! moment it asks for one, which is what a `Task::all` needs in order to wait on
//! what it spawned.
//!
//! **Cancellation marks; the scheduler tears down.** [`Scheduler::cancel`] and
//! [`cancel_task`] set a flag on a task and every descendant of it, and do
//! nothing else. The teardown is a forced unwind of the coroutine's stack and it
//! runs on the *scheduler's* stack — before a marked task is resumed, or over
//! the parked set once the run queue drains — because unwinding a coroutine from
//! a frame standing on that same coroutine's stack is not something to be clever
//! about. A marked task therefore dies at its next safepoint ([`suspend`],
//! [`suspend_current`]) when it is running and immediately when it is already
//! suspended, which is `rule:concurrency/cancellation-runs-no-user-code`'s "torn down by the runtime at its next
//! safepoint" from both directions. Only one task can be marked and still be
//! mid-instruction — the one that cancelled itself or an ancestor — because a
//! core runs one task at a time.
//!
//! What that unwind runs is native `Drop` and nothing else: no `catch`, no
//! cleanup block, no user handler, which is § 5's rule and the same mechanism
//! `Drop for Scheduler` already relies on for a worker retiring with requests
//! still parked. A cancelled task hands back **no** [`Finished`] — its `Ctx` is
//! dropped on its own stack rather than returned, so the output and exit code of
//! a cancelled task are not readable afterwards. [`Scheduler::take_ended`]
//! hands back the ids instead — a cancelled task's beside every other task
//! that ended — and whoever owns the request boundary decides what that means.
//!
//! **A stack carrying script frames is told instead of unwound**, and this is
//! the half of the rule that is not free. A forced unwind is a panic, and an
//! Novis stack is not one it may cross: a `Core` member's entry point is
//! `extern "C"`, which aborts the process rather than letting one out, and the
//! compiled frames beneath it carry no unwind tables at all
//! (`rule:errors/propagation`). So a task is only
//! ever force-unwound when [`nvs_runtime::HelperFrame`] says its stack is
//! clear of both, which [`yield_on`] reads at each suspension and [`Task`]
//! remembers. A task standing on a helper frame is **resumed** with
//! [`Resume::Cancelled`] in place of what it was waiting for; the member it is
//! inside stops the request with `Ctx::cancel`, and it dies by `rule:errors/propagation`'s
//! return status at the safepoint compiled code was going to poll anyway. § 5
//! is untouched by the difference — the status is a `FATAL` no `catch` sees,
//! and what runs between the notice and the death is native `Drop` and a
//! `return`, not user code.
//!
//! The notice is delivered **once**. A park site that ignores it parks again
//! and is then left parked, rather than being collected, resumed and re-parked
//! on every turn for as long as the scheduler lives; the sites that a script
//! can reach — `Core\Time::sleep` and a `Core\Task` group — do not ignore it.
//! What that costs is at the very end of a worker's life, in `Drop for
//! Scheduler`, which has no turn left to give and leaks such a task rather than
//! aborting on it; that `Drop`'s own doc is the home of what it spends.
//!
//! **A task's death cancels whatever it left running**, whether it died by
//! returning, by cancellation or by teardown. That is `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s "control
//! does not leave the call with work still running", enforced one level below
//! the member that promises it: a parent that forgets to wait leaves no orphan,
//! it only loses the child's result. The one shape that outlives its spawning
//! *call* is § 6's `afterResponse`, and it is not an exception — that closure is
//! a child of the request tree rather than of the task that registered it, and
//! [`detach_current`] is what takes that tree out from under the connection
//! before it runs. Cutting the link is the tree's **own** act, taken with its
//! answer already filed and never by the parent: nothing here re-parents a task
//! onto a stranger, so a detached tree still spends the budget it was born
//! under, which is the objection [`Scheduler::orphan`] records against
//! re-parenting downward.
//!
//! **A task wakes a peer through that same tree.** [`Wake`] is a permission to
//! wake one task, taken while that task is running and fired from anywhere; the
//! ids it queues are drained into the run queue by [`Scheduler::run`], after the
//! resume that filled the queue or at the start of the next turn. A bounded
//! channel between two tasks on one core is the first consumer
//! ([`mod@crate::channel`]), and what the route avoids is handing every such
//! primitive a `&mut Scheduler` that a running task provably cannot have.
//!
//! What the tree spends, as `rule:programs/memory-priority` requires: one node per live task — a parent
//! id, a child vector and a flag — removed when that task ends, so it is
//! O(in-flight) and not O(tasks ever spawned), plus one queued id per wake in
//! flight, which every turn drains.
//!
//! **A `Core` member reaches none of this directly**, and what it reaches
//! instead is [`crate::group`] — the implementor of `nvs-runtime`'s host seam,
//! installed by [`Scheduler::run`] beside the tree and holding the two
//! decisions this module does not: what a child gets for a [`Ctx`], and the
//! order of steps that makes `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s "nothing still running" a property
//! of the call. `nvs-stdlib` does not
//! depend on this crate and may not start to — the signature registry lives
//! there, so `nvs-types` and `nvs-codegen` would link a reactor to answer a
//! question about a signature. `Core\Task::all` therefore arrives through
//! [`nvs_runtime::host`], a trait declared in the crate both sides already
//! depend on and published in a thread-local the way [`crate::reactor`] and
//! this module's own `TREE` already are. That module's own docs are the one home for the
//! decisions behind it, including why what crosses is a whole group rather than
//! a `spawn`/`wait`/`cancel` for the caller to sequence: § 4's "nothing still
//! running" is a property of the sequence, so the seam owns the sequence.
//!
//! # What this module deliberately does not know
//!
//! Nothing about I/O. [`Waiting::Parked`] hands the core back and the task sits
//! in the parked set until someone calls [`Scheduler::wake`] with its
//! [`TaskId`]; *who* calls it, on what readiness mechanism, is
//! [`crate::reactor`]'s, and [`Scheduler::run`] returning with tasks still
//! parked is exactly the state in which that reactor blocks. The two are joined
//! by [`crate::reactor::run_until_idle`] and by nothing else, so a run queue
//! stays testable with no I/O in it at all.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::rc::Rc;

use corosensei::{Coroutine, CoroutineResult, Yielder};
use nvs_runtime::{Ctx, TaskPanic, TaskRoot};

use crate::stack::StackPool;
use crate::tsan::Fiber;

/// What a suspended task is waiting for.
///
/// A variant per reason a task is not running, because each needs a different
/// answer from the scheduler. `rule:concurrency/the-reactor-reports-readiness` adds the I/O
/// registration that turns [`Waiting::Parked`] into something a reactor can
/// wait on; the scheduler side of it is already here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Waiting {
    /// The task gave the core back but is still runnable — put it at the back
    /// of the run queue.
    ///
    /// This is what a long-running task does at a safepoint so that its
    /// neighbours are not starved, and it is the only fair-scheduling
    /// mechanism a cooperative scheduler has.
    Yielded,
    /// The task cannot make progress and must not be resumed until something
    /// wakes it by [`TaskId`].
    ///
    /// A task that parks with nothing arranged to wake it is stuck for as long
    /// as the scheduler lives — which is a bug in the caller, not something
    /// the run queue can detect, since "arranged to wake" is a fact about the
    /// reactor and not about the queue.
    Parked,
}

/// What a suspension told the scheduler, which is [`Waiting`] plus the one
/// thing only the suspending stack can know.
///
/// Private, and built in [`yield_on`] rather than passed in: a park site says
/// *why* it is waiting and nothing about what its stack looks like, which is
/// the whole point of a seam that removes async colouring.
#[derive(Clone, Copy, Debug)]
struct Suspended {
    waiting: Waiting,
    /// Whether this stack is one [`Scheduler::tear_down`] may force-unwind —
    /// `false` the moment a helper frame is on it, per
    /// [`nvs_runtime::HelperFrame`].
    unwindable: bool,
}

/// What a resume means to the task being resumed.
///
/// The scheduler's half of the answer [`nvs_runtime::HelperFrame`] asks for: a
/// cancelled task whose stack carries script frames is resumed with
/// [`Resume::Cancelled`] instead of being unwound, and dies by
/// `rule:errors/propagation`'s return status at
/// its next safepoint. The module doc's *task tree* section is the whole
/// decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Resume {
    /// Carry on: whatever the task was waiting for is ready, or the scheduler
    /// is simply giving it its next slice.
    Run,
    /// The task has been cancelled and this resume is the notice. It is
    /// delivered **once**, so a park site that ignores it is parked again with
    /// nothing further coming.
    Cancelled,
}

/// What a suspend answered its caller — the return of [`suspend`] and
/// [`suspend_current`].
///
/// [`Resume`]'s answers plus one, because "nothing suspended" is a refusal a
/// caller has to handle and is not a way of being resumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Resumed {
    /// There was no task beneath the call, so nothing was suspended and the
    /// core was never handed back. The caller must block or fail rather than
    /// run on as if it had waited.
    NotSuspended,
    /// The task suspended and something woke it.
    Woken,
    /// The task suspended and what brought it back was its own cancellation.
    /// The caller must stop waiting: this is delivered once, and the task is
    /// expected to reach a safepoint rather than park again.
    Cancelled,
}

impl Resumed {
    /// Whether the core was actually handed back.
    #[must_use]
    pub fn suspended(self) -> bool {
        !matches!(self, Self::NotSuspended)
    }

    /// Whether the wait ended in a cancellation.
    #[must_use]
    pub fn cancelled(self) -> bool {
        matches!(self, Self::Cancelled)
    }
}

/// The yielder a task suspends through — private to this crate, because the
/// only pointer to one that escapes is the opaque `*const ()` in [`Ctx`].
type TaskYielder = Yielder<Resume, Suspended>;

/// How large the closure a coroutine starts in may be, in bytes —
/// `corosensei`'s number and not ours.
///
/// It copies that closure onto the new stack before the first resume, and its
/// `allocate_obj_on_stack` refuses anything larger with a bare
/// `type is too big to transfer` — a panic raised from whichever coroutine
/// happens to run first, naming neither the closure nor the field that grew it.
///
/// **This is the reason a task's context crosses in a `Box`.**
/// [`nvs_runtime::Ctx`] is most of a kilobyte by itself, because it is where
/// every per-request fact accumulates and there is one per request rather than
/// one per call; captured by value it leaves this budget with nothing in it,
/// and the next field added anywhere in the workspace takes the whole
/// scheduler down. [`Scheduler::start`] boxes it for the crossing and moves it
/// back onto the coroutine's own stack in its first statement, which costs one
/// allocation and one move per **task** — not per helper call, and not on any
/// path a request takes more than once. What it buys is that this budget is
/// spent by a handful of pointers rather than by a context, so a new
/// per-request field is a question about memory (priority 5) rather than about
/// whether tasks start at all.
const CORO_TRANSFER_LIMIT: usize = 1024;

/// A task's identity within one scheduler, and the handle something wakes it
/// by.
///
/// Unique for the life of the scheduler that issued it, never reused: an id
/// handed to a reactor registration that outlives its task must fail to wake
/// anything rather than wake a stranger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(u64);

impl TaskId {
    /// The raw number, for a log line or a reactor's own table.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Rebuilds an id from the number [`TaskId::raw`] handed out.
    ///
    /// Crate-private on purpose: [`crate::reactor`] needs it because a `mio`
    /// token *is* the raw id — that is what saves the reactor a second table —
    /// and nothing outside this crate has any business forging one. An id that
    /// names no live task wakes nothing, which is the reactor's rule 2, so
    /// this is not a hole even inside the crate.
    pub(crate) const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

/// One task's place in the tree: who spawned it, what it spawned, and whether
/// it is under a cancellation.
///
/// Held for exactly as long as the task is, which is the module doc's
/// O(in-flight) claim.
#[derive(Debug)]
struct TaskNode {
    parent: Option<TaskId>,
    children: Vec<TaskId>,
    cancelled: bool,
}

/// A child a running task asked for, waiting for the scheduler's own turn to
/// become a coroutine.
///
/// The body is boxed because this is the one place a task's body is stored
/// rather than immediately consumed; the id in it was issued at the call, so the
/// parent is not waiting on this to name its child.
struct Pending {
    id: TaskId,
    ctx: Ctx,
    root: TaskRoot,
    body: Box<dyn FnOnce(&mut Ctx)>,
}

impl std::fmt::Debug for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pending").field("id", &self.id).finish()
    }
}

/// The half of a scheduler a running task can reach — the module doc's
/// *The task tree* section owns why the split is here and not elsewhere.
#[derive(Debug, Default)]
struct TaskTree {
    next_id: u64,
    nodes: HashMap<TaskId, TaskNode>,
    pending: Vec<Pending>,
    /// Tasks something has asked to wake, drained into the run queue by
    /// [`Scheduler::run`]. A running task cannot reach the parked set either,
    /// so this is the queue a [`Wake`] pushes onto — that type's docs own why.
    wakes: Vec<TaskId>,
}

impl TaskTree {
    /// Issues the next id and puts it in the tree under `parent`.
    ///
    /// A child of a task that is already cancelled is born cancelled: the
    /// window between marking a running task and reaching its next safepoint is
    /// wide enough for it to spawn, and a child born into it would otherwise be
    /// the one thing the sweep never looks at.
    fn issue(&mut self, parent: Option<TaskId>) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;
        let cancelled = parent.is_some_and(|parent| self.is_cancelled(parent));
        self.nodes.insert(
            id,
            TaskNode {
                parent,
                children: Vec::new(),
                cancelled,
            },
        );
        if let Some(node) = parent.and_then(|parent| self.nodes.get_mut(&parent)) {
            node.children.push(id);
        }
        id
    }

    fn is_cancelled(&self, id: TaskId) -> bool {
        self.nodes.get(&id).is_some_and(|node| node.cancelled)
    }

    /// Marks `id` and everything beneath it, answering how many were newly
    /// marked. An id naming no live task marks nothing, exactly as a wake for
    /// one wakes nothing.
    fn cancel(&mut self, id: TaskId) -> usize {
        let mut stack = vec![id];
        let mut marked = 0;
        while let Some(id) = stack.pop() {
            let Some(node) = self.nodes.get_mut(&id) else {
                continue;
            };
            stack.extend_from_slice(&node.children);
            if !node.cancelled {
                node.cancelled = true;
                marked += 1;
            }
        }
        marked
    }

    /// Takes a task that has ended out of the tree, answering with the children
    /// it left behind for the caller to cancel.
    fn retire(&mut self, id: TaskId) -> Vec<TaskId> {
        let Some(node) = self.nodes.remove(&id) else {
            return Vec::new();
        };
        if let Some(parent) = node.parent.and_then(|parent| self.nodes.get_mut(&parent)) {
            parent.children.retain(|child| *child != id);
        }
        node.children
    }

    /// Cuts `id` loose from its parent, in both directions, answering whether
    /// there was one to cut.
    ///
    /// The tree is the only thing [`Scheduler::orphan`] walks, so a task with
    /// no parent is a task no ending parent takes with it. Everything beneath
    /// it comes along — the children keep pointing at `id`, and `id` is now a
    /// root — which is what makes this whole *tree* rather than one task.
    fn detach(&mut self, id: TaskId) -> bool {
        let Some(node) = self.nodes.get_mut(&id) else {
            return false;
        };
        let Some(parent) = node.parent.take() else {
            return false;
        };
        if let Some(parent) = self.nodes.get_mut(&parent) {
            parent.children.retain(|child| *child != id);
        }
        true
    }
}

thread_local! {
    /// The tree of the scheduler turning on this thread right now.
    ///
    /// Installed for the length of [`Scheduler::run`] and restored to whatever
    /// was there before, so a scheduler driven from inside another one's task —
    /// which the tests do — nests rather than clobbers. `None` between turns is
    /// the refusal [`spawn_child`] and [`cancel_task`] answer with.
    static TREE: RefCell<Option<Rc<RefCell<TaskTree>>>> = const { RefCell::new(None) };
}

/// Restores the previously installed tree when dropped.
struct TreeGuard {
    previous: Option<Rc<RefCell<TaskTree>>>,
}

impl Drop for TreeGuard {
    fn drop(&mut self) {
        TREE.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

fn install_tree(tree: &Rc<RefCell<TaskTree>>) -> TreeGuard {
    let previous = TREE.with(|slot| slot.borrow_mut().replace(Rc::clone(tree)));
    TreeGuard { previous }
}

fn current_tree() -> Option<Rc<RefCell<TaskTree>>> {
    TREE.with(|slot| slot.borrow().clone())
}

/// A permission to wake one task on this core, taken while that task is
/// running.
///
/// A running task cannot reach the `&mut Scheduler` that is resuming it, so a
/// primitive wanting to wake a *peer* — a channel whose queue has just stopped
/// being empty — cannot call [`Scheduler::wake`] either. This is the route it
/// takes instead: the id is queued on the task tree, which is the half of a
/// scheduler a task can reach, and [`Scheduler::run`] moves that task from
/// parked to ready.
///
/// It holds the tree rather than reading the thread-local when it fires, and
/// that is not an optimization. It is what lets a handle taken inside a turn
/// fire outside one — an end of a channel held by an accept loop, or by a test
/// driving a scheduler by hand — and it is what keeps two schedulers on one
/// thread from crossing wakes, since a [`TaskId`] is unique only within its own
/// tree.
///
/// [`crate::RemoteWake`] is the sibling for a thread that is not this core:
/// same purpose, a poke of `mio`'s waker rather than a push, and `Send` where
/// this is deliberately not.
pub struct Wake {
    tree: Rc<RefCell<TaskTree>>,
    id: TaskId,
}

impl Wake {
    /// Takes a wake for the task running right now.
    ///
    /// `None` outside a turn, which is the same refusal [`current_task`] and
    /// [`spawn_child`] answer with and means the same thing: there is no task
    /// here to be woken later.
    #[must_use]
    pub fn current() -> Option<Self> {
        let id = current_task()?;
        let tree = current_tree()?;
        Some(Self { tree, id })
    }

    /// Which task this wakes.
    #[must_use]
    pub const fn id(&self) -> TaskId {
        self.id
    }

    /// Queues the wake, to be delivered by the scheduler that owns the task.
    ///
    /// Immediately after the current resume when this is called from a task,
    /// and at the start of the next [`Scheduler::run`] when it is called from
    /// outside one. Waking a task that is not parked does nothing, exactly as
    /// [`Scheduler::wake`] does nothing for one: a wake is a hint that may have
    /// raced with the task ending, and being late is not an error.
    pub fn wake(&self) {
        self.tree.borrow_mut().wakes.push(self.id);
    }
}

impl std::fmt::Debug for Wake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hand-written: deriving would print the whole tree behind the `Rc`.
        f.debug_struct("Wake").field("id", &self.id).finish()
    }
}

/// A **root** task that reached its end, handed back with the context it ran
/// under.
///
/// **Only a root files one.** A child's answer is delivered through whatever
/// spawned it — [`crate::Isolate`]'s own slot — and never through this list, so
/// a child's context bought nothing by being kept and everything by being
/// dropped: under a server every request, every connection and every scheduled
/// fire is a child, so filing all of them held one arena and one carrier per
/// request **served**, which is
/// `rule:programs/memory-priority`'s own definition of a
/// leak. Their ids are not lost with them — [`Scheduler::take_ended`] carries
/// every task that ended, whichever way it ended, which is what `rule:concurrency/the-parking-contract`
/// rule 3's deregistration reads.
///
/// What it spends, as `rule:programs/memory-priority` asks: one `Ctx` — the arena at its peak, the
/// output buffer and the exit code — per **root** that has ended and has not
/// been taken. A worker has one or two of those for the length of the process
/// (`nvs serve`'s accept loop and its ticker), and `nvs run` takes its one back
/// the moment the run is over, so this is O(roots) rather than O(tasks).
#[derive(Debug)]
pub struct Finished {
    /// Which task this was.
    pub id: TaskId,
    /// The context the task ran under, with its output, its exit code and its
    /// pending failure all still readable — this is what a request boundary
    /// reads back.
    pub ctx: Ctx,
    /// `Err` if a panic reached the task's root and was contained there by
    /// [`nvs_runtime::run_task`], which is
    /// `rule:http-server/containment-does-not-end-at-the-helper`
    /// 's outer boundary. [`TaskPanic::retires_worker`] is the only
    /// decision a caller has to make from one.
    pub outcome: Result<(), TaskPanic>,
}

/// What one call to [`Scheduler::run`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunReport {
    /// How many times a task was resumed. Every suspension costs one of these
    /// on the way back in, so this is the stack-switch count.
    pub resumes: usize,
    /// How many tasks reached their end during this call.
    pub finished: usize,
    /// How many tasks are parked now that the run queue is empty. Non-zero is
    /// the state `rule:concurrency/the-reactor-reports-readiness`'s reactor waits in.
    pub parked: usize,
    /// How many tasks were torn down for a cancellation during this call. Their
    /// ids are in [`Scheduler::take_ended`], beside every task that returned;
    /// they are not in
    /// [`RunReport::finished`], because a cancelled task never returns one.
    pub cancelled: usize,
}

struct Task {
    id: TaskId,
    coro: Coroutine<Resume, Suspended, Finished>,
    /// Whether this task's stack may be force-unwound, as of its last
    /// suspension. `true` for a task that has never run: an unstarted
    /// coroutine's stack holds nothing to unwind.
    unwindable: bool,
    /// Whether [`Resume::Cancelled`] has already been delivered to it. A
    /// cancellation is delivered once — a task that parks again after being
    /// told is left parked rather than told a second time, which is what keeps
    /// [`Scheduler::run`]'s parked sweep from spinning against a park site that
    /// ignores the answer.
    told: bool,
    /// This task's ThreadSanitizer fiber — a zero-sized nothing in every build
    /// but the sanitizer leg's, where it is the access history that follows
    /// this stack rather than the thread currently under it. [`crate::tsan`] is
    /// the home of what it is for.
    fiber: Fiber,
}

impl Task {
    /// Resumes this task, telling the sanitizer that the stack changed.
    ///
    /// The resume and nothing else goes inside [`Fiber::around`], which is that
    /// method's whole contract.
    fn resume(&mut self, resume: Resume) -> CoroutineResult<Suspended, Finished> {
        let Self { coro, fiber, .. } = self;
        fiber.around(|| coro.resume(resume))
    }

    /// Unwinds this task's stack where it stands, telling the sanitizer that
    /// the stack changed.
    ///
    /// A forced unwind is a `longjmp` onto the task's stack and back out of it,
    /// so it is the same switch a resume makes and is announced the same way —
    /// but only when there is a stack to switch to. `corosensei` performs no
    /// switch at all for a coroutine that has not started or has already
    /// finished, and announcing one would leave the sanitizer attributing the
    /// scheduler's own frames to the task.
    fn force_unwind(&mut self) {
        let Self { coro, fiber, .. } = self;
        if coro.started() && !coro.done() {
            fiber.around(|| coro.force_unwind());
        } else {
            coro.force_unwind();
        }
    }
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The coroutine's stack is not something a debug line can say anything
        // useful about, and printing an address would make the output differ
        // between runs.
        f.debug_struct("Task").field("id", &self.id).finish()
    }
}

/// The run queue of one core.
///
/// Created on the thread that runs it — see [`crate::Worker`] — and never
/// moved off it, which the `!Send` bound in this type makes structural rather
/// than advisory.
pub struct Scheduler {
    /// The parent links, the child lists, the cancel flags and the id counter —
    /// shared with the running task rather than owned outright, because a task
    /// cannot reach the `&mut Scheduler` resuming it. The module doc's
    /// *The task tree* section is that decision's home.
    tree: Rc<RefCell<TaskTree>>,
    ready: VecDeque<Task>,
    parked: HashMap<TaskId, Task>,
    /// The **roots** that have ended and have not been taken. [`Finished`]'s
    /// doc owns why a child is never one of them.
    finished: Vec<Finished>,
    /// Every task that ended, however it ended, until whoever drops their I/O
    /// registrations takes them — [`Scheduler::take_ended`].
    ended: Vec<TaskId>,
    /// This worker's supply of task stacks — `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`, and
    /// [`crate::stack`]'s module doc for the whole policy. It lives here rather
    /// than in [`crate::Worker`] because this is the type that knows when a
    /// task starts and when it ends, which is the only pair of moments a pool
    /// cares about.
    stacks: StackPool,
    /// Makes this type `!Send` and `!Sync`, which is the module doc's whole
    /// first section. A raw pointer is the standard spelling and costs no
    /// bytes.
    _pinned_to_one_thread: PhantomData<*const ()>,
}

impl std::fmt::Debug for Scheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scheduler")
            .field("ready", &self.ready.len())
            .field("parked", &self.parked.len())
            .field("finished", &self.finished.len())
            .field("tracked", &self.tree.borrow().nodes.len())
            .field("stacks", &self.stacks)
            .finish()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Scheduler {
    /// Tears down every task this scheduler still holds, suspended ones
    /// included.
    ///
    /// A worker that retires with a request still parked is the ordinary
    /// shutdown, not an exceptional one, and dropping a suspended coroutine
    /// unwinds its stack. That unwind has to pass through
    /// [`nvs_runtime::run_task`]'s containment boundary, which would otherwise
    /// swallow it and leave `corosensei` unable to tell a torn-down stack from
    /// a corrupted one — it aborts the process at that point.
    /// [`nvs_runtime::Teardown`] is the narrow window that lets it through, and
    /// its doc owns the reasoning.
    ///
    /// No script code runs here, which is
    /// `rule:concurrency/cancellation-runs-no-user-code`
    /// 's rule: what the unwind runs is native `Drop`, so an arena is
    /// released and a handle is closed, and no `catch` or `finally` is
    /// consulted.
    ///
    /// **A task standing on a helper frame is leaked instead** — see
    /// [`Task::unwindable`] and [`nvs_runtime::HelperFrame`]. There is no
    /// unwind that can cross that frame and no safepoint to reach without
    /// resuming the task, and this is a `Drop`: nobody is left to run it. What
    /// it spends is that task's stack mapping and whatever the stack held,
    /// once per task, at the death of the worker that owned it —
    /// `rule:programs/memory-priority`'s
    /// O(in-flight) rather than O(requests served) — and what it buys is
    /// `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`'s
    /// rule that no path reaches `abort()`, which is the one this would
    /// otherwise be. The ordinary end of a request never arrives here:
    /// [`Scheduler::run`] returns when its tasks are done, and one still parked
    /// in a `Core` member is a worker retiring under a request that never
    /// finished.
    fn drop(&mut self) {
        if self.ready.is_empty() && self.parked.is_empty() {
            return;
        }
        let _teardown = nvs_runtime::Teardown::enter();
        let ready = std::mem::take(&mut self.ready);
        let parked = std::mem::take(&mut self.parked);
        for mut task in ready.into_iter().chain(parked.into_values()) {
            if task.unwindable {
                // Unwound here rather than left to `Coroutine`'s own `Drop`,
                // which would make the same switch with nothing to announce it
                // — [`Task::force_unwind`]. What the drop below is left to do
                // is release a coroutine that is already finished, which
                // switches no stack. A task that is *forgotten* keeps its
                // fiber for the same reason it keeps its stack: neither is
                // handed back, so neither is retired.
                task.force_unwind();
                drop(task);
            } else {
                std::mem::forget(task);
            }
        }
    }
}

impl Scheduler {
    /// An empty run queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tree: Rc::new(RefCell::new(TaskTree::default())),
            ready: VecDeque::new(),
            parked: HashMap::new(),
            finished: Vec::new(),
            ended: Vec::new(),
            stacks: StackPool::new(),
            _pinned_to_one_thread: PhantomData,
        }
    }

    /// Puts a task on the run queue and returns its id.
    ///
    /// `ctx` is moved onto the task's own stack and handed back through
    /// [`Finished`] when it ends, rather than borrowed, because the coroutine
    /// body has to be `'static` — the same reason `benches/abi-probe`'s
    /// `in_coroutine` moves it. That also keeps every caller free of raw
    /// pointers.
    ///
    /// `root` decides what a panic beneath this task costs: `rule:http-server/containment-does-not-end-at-the-helper` fails
    /// a [`TaskRoot::Request`] as one request and retires the worker for a
    /// [`TaskRoot::Worker`].
    ///
    /// The stack comes from this worker's pool ([`crate::stack`]) and the
    /// task's recursion limit is armed **from that stack** before the context
    /// moves onto it. `nvs-runtime`'s `Ctx` module doc records the gap this
    /// closes: [`Ctx::new`] can only assert a ceiling from the stack pointer it
    /// happens to be constructed on, which for a task is the *worker's* stack
    /// and therefore describes memory the task will never run on. This crate
    /// allocated the stack and knows its base and its width exactly, which is
    /// what that doc means by "an embedder that knows its bounds" — `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`
    /// 's last bullet.
    ///
    /// # Panics
    ///
    /// If the OS refuses a stack reservation and the pool is empty;
    /// [`crate::stack::StackPool::take`] owns why nothing softer is available
    /// or wanted here.
    ///
    /// Nothing runs until [`Scheduler::run`] is called.
    pub fn spawn<F>(&mut self, ctx: Ctx, root: TaskRoot, body: F) -> TaskId
    where
        F: FnOnce(&mut Ctx) + 'static,
    {
        // Never passed in: the tree is a fact about who called, and a parameter
        // would be a fact about who remembered. Reaching this with a task
        // running takes a scheduler driven from inside another one's task, so
        // it is almost always `None` — the root case.
        let parent = current_task();
        let id = self.tree.borrow_mut().issue(parent);
        self.start(id, ctx, root, Box::new(body));
        id
    }

    /// Builds the coroutine for an id the tree has already issued and puts it on
    /// the run queue.
    ///
    /// Its callers — [`Scheduler::spawn`] and the drain of [`spawn_child`]'s
    /// pending list — both have to arm the stack limit the same way, and the
    /// way it is armed is the whole reason this is not two copies.
    fn start(&mut self, id: TaskId, ctx: Ctx, root: TaskRoot, body: Box<dyn FnOnce(&mut Ctx)>) {
        let stack = self.stacks.take();
        let mut ctx = ctx;
        let (base, ceiling) = crate::stack::bounds(&stack);
        ctx.arm_stack_limit(base, ceiling);
        // Boxed for the crossing and unboxed the instant the coroutine starts,
        // so what this closure *captures* is a pointer rather than a whole
        // context: `CORO_TRANSFER_LIMIT` is why, and the assertion below is what
        // says so when a future field pushes it back over.
        let carried = Box::new(ctx);
        // The first resume's argument is ignored: a task that has not started
        // has an empty stack, so a cancelled one is torn down rather than told.
        let entry = move |yielder: &TaskYielder, _first: Resume| {
            // Back onto this coroutine's own stack, where the context lives for
            // the length of the task and where every helper reaches it by
            // pointer; the box is freed here and is not a per-helper cost.
            let mut ctx = *carried;
            // Taking a pointer to the yielder is safe; only turning it back
            // into a reference is not, and `suspend` below owns that `unsafe`.
            // It is erased to `*const ()` so that `nvs-runtime` — the crate
            // every compiled unit links — needs no coroutine dependency.
            let raw = std::ptr::from_ref(yielder).cast::<()>();
            ctx.set_yielder(raw);
            // The same pointer, published a second time for code that was handed
            // no `Ctx` at all — a `std::io::Read` on a socket. `RUNNING`'s docs
            // own why the task maintains this rather than the scheduler.
            RUNNING.set(Some(Running { id, yielder: raw }));
            let outcome = nvs_runtime::run_task(root, || body(&mut ctx));
            // Cleared on both paths, the contained-panic one included, so no
            // pointer to a stack that is about to go away can escape in the
            // context.
            ctx.set_yielder(std::ptr::null());
            RUNNING.set(None);
            Finished { id, ctx, outcome }
        };
        assert!(
            std::mem::size_of_val(&entry) <= CORO_TRANSFER_LIMIT,
            "a task's entry closure is over `corosensei`'s transfer limit, so no task can \
             start — put the new per-task state behind a pointer rather than raising the \
             limit, which is not ours"
        );
        let coro = Coroutine::with_stack(stack, entry);

        self.ready.push_back(Task {
            id,
            coro,
            unwindable: true,
            told: false,
            fiber: Fiber::new(),
        });
    }

    /// Moves a parked task back to the run queue, reporting whether there was
    /// one to move.
    ///
    /// `false` for an id that is running, has already finished, or was never
    /// issued — a wake is a hint from something that may have raced with the
    /// task ending, so it is not an error to be late.
    pub fn wake(&mut self, id: TaskId) -> bool {
        match self.parked.remove(&id) {
            Some(task) => {
                self.ready.push_back(task);
                true
            }
            None => false,
        }
    }

    /// Runs until the run queue is empty, then returns.
    ///
    /// Empty does not mean done: [`RunReport::parked`] is how many tasks are
    /// waiting for a wake that has to come from outside. A non-zero count is
    /// precisely the point at which [`crate::reactor::run_until_idle`] blocks
    /// on readiness instead; a caller running without a reactor treats it as
    /// "there is nothing more I can do".
    pub fn run(&mut self) -> RunReport {
        let mut report = RunReport::default();
        // For the length of the turn and no longer: this is what `spawn_child`
        // and `cancel_task` reach, and outside a turn there is nothing running
        // for either of them to be a child of or to cancel from.
        let _installed = install_tree(&self.tree);
        // Beside the tree and for the same length, so a `Core` member can reach
        // a host exactly when it can reach a tree to put children in.
        // [`crate::group`] is the implementor and the home of what a child gets
        // for a `Ctx`.
        let _host = crate::group::install();
        loop {
            // Before the run queue is read rather than only after a resume: a
            // wake issued between turns, by an end of a channel held outside
            // the scheduler, is as real as one issued from a task and this is
            // the only place it can land.
            self.drain_wakes();
            while let Some(mut task) = self.ready.pop_front() {
                // Before the resume, not after: a task marked while it was on
                // the queue must not get another instruction — unless its stack
                // is one no unwind may cross, in which case the notice *is* the
                // instruction and the module doc's *task tree* section owns why.
                let resume = if self.tree.borrow().is_cancelled(task.id) {
                    if task.unwindable {
                        self.tear_down(task, &mut report);
                        continue;
                    }
                    if task.told {
                        // Already told, and back here anyway: it owes a
                        // safepoint, and telling it twice would be a second
                        // notice with no new fact in it.
                        Resume::Run
                    } else {
                        task.told = true;
                        Resume::Cancelled
                    }
                } else {
                    Resume::Run
                };
                report.resumes += 1;
                match task.resume(resume) {
                    CoroutineResult::Yield(suspended) => {
                        task.unwindable = suspended.unwindable;
                        match suspended.waiting {
                            Waiting::Yielded => self.ready.push_back(task),
                            Waiting::Parked => {
                                self.parked.insert(task.id, task);
                            }
                        }
                    }
                    CoroutineResult::Return(finished) => {
                        report.finished += 1;
                        // Asked before `orphan` retires the node, which is what
                        // makes the answer "this task had no parent" rather
                        // than "the tree has forgotten this id".
                        let is_root = self.parent_of(finished.id).is_none();
                        self.ended.push(finished.id);
                        self.orphan(finished.id);
                        if is_root {
                            self.finished.push(finished);
                        }
                        // A child's `Ctx` is dropped right here, on the
                        // scheduler's own stack, because nothing was ever going
                        // to read it back — [`Finished`] is the home of why.
                        // The coroutine is done, so its stack holds nothing and
                        // `into_stack` can take it — this is the one moment a
                        // stack is recyclable, and letting `task` drop here
                        // instead would hand every request's mapping back to the
                        // OS.
                        self.stacks.give(task.coro.into_stack());
                    }
                }
                self.drain_pending();
                self.drain_wakes();
            }
            // A parked task has no next safepoint to reach — it is already
            // standing on one — so this is where a cancellation catches it.
            // Tearing one down can cancel its children, which is why this is a
            // loop and not a tail.
            //
            // A task that has already been told and parked again is not doomed
            // here: it has had its one notice, and collecting it again would
            // put it back on the run queue every turn for as long as it parks.
            let doomed: Vec<TaskId> = self
                .parked
                .iter()
                .filter(|(id, task)| {
                    self.tree.borrow().is_cancelled(**id) && (task.unwindable || !task.told)
                })
                .map(|(id, _)| *id)
                .collect();
            // A teardown drops the handles the dead task was holding, and
            // dropping the last sender of a channel wakes everyone still
            // reading it — so the sweep below can refill the run queue even
            // when it finds nothing else to do.
            if doomed.is_empty() && self.tree.borrow().wakes.is_empty() {
                break;
            }
            for id in doomed {
                if let Some(task) = self.parked.remove(&id) {
                    if task.unwindable {
                        self.tear_down(task, &mut report);
                    } else {
                        // Its stack cannot be unwound where it stands, so it
                        // goes back on the run queue and the branch above hands
                        // it `Resume::Cancelled` on the way in.
                        self.ready.push_back(task);
                    }
                }
            }
        }
        report.parked = self.parked.len();
        report
    }

    /// Turns the children a task left behind into cancellations, and takes the
    /// task itself out of the tree.
    ///
    /// The module doc's *task tree* section owns why a death cancels downward
    /// rather than re-parenting: the alternative is an orphan whose budget is
    /// charged to a request that is over.
    fn orphan(&mut self, id: TaskId) {
        let mut tree = self.tree.borrow_mut();
        let children = tree.retire(id);
        for child in children {
            tree.cancel(child);
        }
    }

    /// Unwinds a cancelled task's stack and recycles it.
    ///
    /// **Only for a task whose stack may be unwound** — [`Task::unwindable`],
    /// which is [`nvs_runtime::HelperFrame`] read at the task's last
    /// suspension. A task standing on a helper frame is resumed with
    /// [`Resume::Cancelled`] instead; both callers make that test, and calling
    /// this without it is a process abort rather than a wrong answer.
    ///
    /// Called only from [`Scheduler::run`], which is to say only from the
    /// scheduler's own stack: `force_unwind` is a `longjmp` into the coroutine
    /// and back out again, and issuing one from a frame standing on the stack
    /// being unwound is not a thing to arrange. The unwind runs native `Drop`
    /// and no script code (`rule:concurrency/cancellation-runs-no-user-code`), and it has to pass through
    /// [`nvs_runtime::run_task`]'s containment boundary, which is what
    /// [`nvs_runtime::Teardown`] is for — the same window `Drop for Scheduler`
    /// opens for the same reason.
    fn tear_down(&mut self, mut task: Task, report: &mut RunReport) {
        {
            let _teardown = nvs_runtime::Teardown::enter();
            task.force_unwind();
        }
        // Unwound is finished as far as the stack is concerned, so the mapping
        // is recyclable here exactly as it is for a task that returned.
        self.stacks.give(task.coro.into_stack());
        self.orphan(task.id);
        self.ended.push(task.id);
        report.cancelled += 1;
    }

    /// Turns the children asked for during the last resume into real tasks.
    ///
    /// After every resume rather than once per turn, so a child is on the run
    /// queue before its parent's next slice: a parent that spawns and then
    /// parks is waiting on work that is already runnable.
    fn drain_pending(&mut self) {
        let pending = std::mem::take(&mut self.tree.borrow_mut().pending);
        for Pending {
            id,
            ctx,
            root,
            body,
        } in pending
        {
            self.start(id, ctx, root, body);
        }
    }

    /// Delivers the wakes queued since the last drain.
    ///
    /// Beside [`Scheduler::drain_pending`] and for the same reason: a wake a
    /// task issued during its slice takes effect before the next task runs, so
    /// a peer it unblocked is on the run queue rather than waiting for the core
    /// to go idle first. A wake naming a task that is not parked — one already
    /// runnable, or one that has been torn down — does nothing.
    fn drain_wakes(&mut self) {
        let wakes = std::mem::take(&mut self.tree.borrow_mut().wakes);
        for id in wakes {
            self.wake(id);
        }
    }

    /// Takes the **roots** that have ended since this was last called.
    ///
    /// [`Finished`] owns why a child is never in here and what that saves.
    pub fn take_finished(&mut self) -> Vec<Finished> {
        std::mem::take(&mut self.finished)
    }

    /// Takes the ids of every task that ended since this was last called —
    /// returned, or torn down for a cancellation.
    ///
    /// Draining rather than reading, and one list rather than two, because
    /// there is one consumer and one rule: a reactor registration must not
    /// outlive the task that made it (`rule:concurrency/the-parking-contract` rule 3,
    /// [`crate::reactor::run_until_idle`]), and how the task ended does not
    /// change that. A list read without draining is walked again on every turn
    /// and grows with the number of tasks a worker has *served*, which under a
    /// server is O(requests served) in both memory and per-turn work.
    pub fn take_ended(&mut self) -> Vec<TaskId> {
        std::mem::take(&mut self.ended)
    }

    /// Marks a task and everything beneath it for teardown, answering how many
    /// were newly marked.
    ///
    /// Nothing is unwound here — the module doc's *task tree* section owns why
    /// the teardown waits for the scheduler's own turn. A task already marked,
    /// and an id naming no live task, both mark nothing; `0` is therefore not an
    /// error, in the same way a late [`Scheduler::wake`] is not one.
    pub fn cancel(&mut self, id: TaskId) -> usize {
        self.tree.borrow_mut().cancel(id)
    }

    /// Whether this task is marked for teardown but has not reached it yet.
    #[must_use]
    pub fn is_cancelled(&self, id: TaskId) -> bool {
        self.tree.borrow().is_cancelled(id)
    }

    /// The task that spawned this one, or `None` for a root and for an id that
    /// names no live task.
    #[must_use]
    pub fn parent_of(&self, id: TaskId) -> Option<TaskId> {
        self.tree
            .borrow()
            .nodes
            .get(&id)
            .and_then(|node| node.parent)
    }

    /// The live children of a task, in the order they were spawned.
    ///
    /// A copy rather than a borrow, because the tree is shared with whatever is
    /// running and a borrow across a resume would be a borrow across arbitrary
    /// script code.
    #[must_use]
    pub fn children_of(&self, id: TaskId) -> Vec<TaskId> {
        self.tree
            .borrow()
            .nodes
            .get(&id)
            .map_or_else(Vec::new, |node| node.children.clone())
    }

    /// How many tasks the tree is holding a node for — every task that has been
    /// spawned and has not yet ended, and no others.
    ///
    /// The O(in-flight) property `rule:programs/memory-priority` asks of anything the runtime keeps,
    /// made checkable rather than asserted.
    #[must_use]
    pub fn tracked_tasks(&self) -> usize {
        self.tree.borrow().nodes.len()
    }

    /// How many tasks are parked waiting for a wake.
    #[must_use]
    pub fn parked_count(&self) -> usize {
        self.parked.len()
    }

    /// How many tasks are on the run queue right now.
    #[must_use]
    pub fn ready_count(&self) -> usize {
        self.ready.len()
    }

    /// How many task stacks this worker is holding idle for the next spawn.
    ///
    /// Bounded by [`crate::stack::MAX_POOLED_STACKS`] and, below that, by the
    /// number of tasks this scheduler has ever had in flight at once — which is
    /// the O(in-flight) property `rule:programs/memory-priority` asks of anything the runtime keeps.
    #[must_use]
    pub fn pooled_stacks(&self) -> usize {
        self.stacks.pooled()
    }
}

/// Spawns a child of the task that is running, answering with its id.
///
/// **This is how a task gets a child at all**, and the parent is the caller
/// rather than an argument — `rule:concurrency/a-child-belongs-to-the-calling-task`'s "each is a child of the calling
/// task". The child's id is issued here, so a parent can wait on what it
/// spawned before the scheduler has built anything; the coroutine itself is
/// built on the scheduler's next turn, which the module doc's *task tree*
/// section explains.
///
/// `None` means there is no task running on this thread, or no scheduler
/// turning — the same refusal [`suspend`] answers with `false`, and it obliges
/// the caller the same way: use [`Scheduler::spawn`] and be a root, or fail.
/// Nothing is spawned in that case.
///
/// A child spawned by a task that is already cancelled is born cancelled and
/// never runs a single instruction, which is the only ordering in which `rule:concurrency/nothing-is-still-running-when-a-call-returns`
/// 's "nothing still running" survives a parent racing its own teardown.
pub fn spawn_child<F>(ctx: Ctx, root: TaskRoot, body: F) -> Option<TaskId>
where
    F: FnOnce(&mut Ctx) + 'static,
{
    let parent = current_task()?;
    let tree = current_tree()?;
    let mut tree = tree.borrow_mut();
    let id = tree.issue(Some(parent));
    tree.pending.push(Pending {
        id,
        ctx,
        root,
        body: Box::new(body),
    });
    Some(id)
}

/// Marks a task and everything beneath it for teardown from inside a task,
/// answering how many were newly marked.
///
/// [`Scheduler::cancel`] is the same operation for a caller holding the
/// scheduler; this is the one a `Core\Task::all` will reach when a sibling
/// throws, since that code is running on a task's own stack. It marks and
/// returns — no unwinding happens on this stack, including when the id is the
/// caller's own, which then dies at its next safepoint.
///
/// `0` for an id that names no live task, for one already marked, and for a call
/// with no scheduler turning beneath it.
pub fn cancel_task(id: TaskId) -> usize {
    current_tree().map_or(0, |tree| tree.borrow_mut().cancel(id))
}

/// Makes the running task a root, so that the task which spawned it no longer
/// cancels it by ending — the module doc's *task tree* section owns why this is
/// the one shape that gets to do it.
///
/// `rule:concurrency/after-response-outlives-the-connection` is
/// the whole caller list: a request tree that has filed its answer and still has
/// after-response work to run has to outlive the connection that was waiting for
/// that answer, and a connection ending is a task returning. It is called from
/// **inside** the tree being detached and with its answer already published,
/// which is what makes it safe to lose the link: there is nothing left for the
/// parent to wait for, so `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s "control does not leave the call with
/// work still running" has already been kept by the time this runs.
///
/// `false` when there is no task running here, when no scheduler is turning, and
/// for a task that is already a root — refusals a caller treats the same
/// way, since each of them means the link this would have cut is not there.
pub fn detach_current() -> bool {
    let Some(id) = current_task() else {
        return false;
    };
    let Some(tree) = current_tree() else {
        return false;
    };
    tree.borrow_mut().detach(id)
}

/// Suspends the task `ctx` is running inside, reporting whether there was one.
///
/// **This is the seam that removes async colouring.** The caller is an
/// ordinary function taking an ordinary `&Ctx`: it does not know whether it is
/// on a coroutine stack, its own callers are not marked, and nothing about
/// Novis's calling convention changes because it might wait.
///
/// [`Resumed::NotSuspended`] means there is no scheduler beneath this context —
/// a `Core` member reached from `nvs run`, or a unit test — and **the caller
/// must then do something else**: block, or fail. It must not treat a refusal
/// as a successful wait, because nothing suspended and control is about to run
/// on as if it had.
///
/// **A cancelled task's suspend either does not return, or returns
/// [`Resumed::Cancelled`].** Which of the two depends on the stack, not on the
/// caller: a stack the scheduler may unwind is unwound where it stands, so no
/// frame between here and the task's root runs anything but its own `Drop`; a
/// stack carrying a helper frame cannot be unwound at all, so it is resumed
/// with the notice instead and the caller must stop waiting. Either way no
/// script code runs, which is `rule:concurrency/cancellation-runs-no-user-code`, and the module doc's *task tree*
/// section owns the whole decision.
///
/// # Panics
///
/// Never by itself. The coroutine machinery underneath aborts only on a stack
/// overflow of the task's own stack, which is
/// `rule:errors/on-limit`'s
/// territory and is guarded by `Ctx`'s stack limit long before it is reached.
pub fn suspend(ctx: &Ctx, waiting: Waiting) -> Resumed {
    let raw = ctx.yielder();
    if raw.is_null() {
        return Resumed::NotSuspended;
    }
    yield_on(raw, waiting)
}

thread_local! {
    /// The task running on this thread right now, and `None` between tasks.
    ///
    /// **Maintained by the tasks themselves, never by [`Scheduler::run`].** The
    /// coroutine body publishes on entry and clears on exit, and [`yield_on`]
    /// takes the entry off the thread on the way out of a switch and puts it
    /// back when control returns to that stack. The scheduler could not do it:
    /// the yielder half lives on the task's own stack, and the only code that
    /// can reach it is the code standing on that stack.
    ///
    /// Clearing rather than leaving the last runner in place is deliberate. A
    /// read from outside a task then answers "there is no task here", which is
    /// a refusal the caller already has to handle, instead of a stranger's
    /// [`TaskId`] that would register one task's interest against another's.
    static RUNNING: Cell<Option<Running>> = const { Cell::new(None) };
}

/// What a running task publishes for code that was handed no [`Ctx`].
#[derive(Clone, Copy)]
struct Running {
    id: TaskId,
    /// The same erased `&TaskYielder` [`Ctx::yielder`] carries, written from the
    /// same place in the same statement, so there is one source and two readers
    /// rather than two sources.
    yielder: *const (),
}

/// The task running on this core right now, if there is one.
///
/// This is the id a registration is keyed by, so it is what
/// [`crate::reactor::Reactor::register`]'s caller needs and cannot otherwise
/// obtain: a `std::io::Read` is handed a buffer and nothing else.
///
/// `None` means there is no task beneath this call — a `Core` member reached
/// from `nvs run`, or a unit test — and is the same refusal [`suspend`] answers
/// with `false`. `crate::reactor`'s module docs own the whole route.
#[must_use]
pub fn current_task() -> Option<TaskId> {
    RUNNING.get().map(|running| running.id)
}

/// How many children the running task still has: spawned and not yet ended,
/// whether they are on the run queue, parked under a wait, or still pending a
/// coroutine.
///
/// `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s "control does not leave the call with work still running" is
/// the promise `Core\Task::all` and `::map` keep by construction. This is how
/// a caller that is *not* one of those members reads the same fact off the
/// tree — `rule:testing/task-tree-and-virtual-clock`'s runner, which fails a test whose task tree outlived it rather than
/// letting the teardown cancel it silently. A child that has ended is already
/// out of the tree ([`Scheduler::orphan`], run the moment its coroutine
/// returns and before whatever was awaiting it is resumed), so a task joined
/// by an `await` is not counted here.
///
/// `0` with no task running and `0` with no scheduler turning: neither is a
/// child still going, and a caller with nowhere to spawn one had no tree to
/// leave dirty.
#[must_use]
pub fn children_still_running() -> usize {
    let Some(id) = current_task() else {
        return 0;
    };
    current_tree().map_or(0, |tree| {
        tree.borrow()
            .nodes
            .get(&id)
            .map_or(0, |node| node.children.len())
    })
}

/// Suspends the running task without a [`Ctx`] to reach the yielder through,
/// reporting whether there was one.
///
/// [`suspend`] is the seam for a helper that *has* a context, and it stays the
/// one every `Core` member uses. This is for the code that has no argument to
/// carry one in — the parking `Read` and `Write` of a socket, whose signatures
/// are `std::io`'s and not ours.
///
/// Every answer means the same thing it means there, and obliges the caller the
/// same way: [`Resumed::NotSuspended`] is "nothing suspended, so block or fail
/// rather than run on", and [`Resumed::Cancelled`] is "stop waiting".
pub fn suspend_current(waiting: Waiting) -> Resumed {
    match RUNNING.get() {
        Some(running) => yield_on(running.yielder, waiting),
        None => Resumed::NotSuspended,
    }
}

/// Hands the core back through an erased `&TaskYielder`, and restores this
/// task's identity when control comes back to this stack.
///
/// This is also where the scheduler learns what kind of stack it is holding.
/// [`nvs_runtime::HelperFrame`]'s count is a fact about the stack that is
/// running, so it is read here — the last moment this stack is the running one
/// — and travels out in the suspension rather than being asked for afterwards,
/// when the scheduler is standing on its own stack and the count is somebody
/// else's.
///
/// **Every per-stack thread-local this runtime keeps is taken and put back
/// here.** [`RUNNING`], the helper-frame count, and
/// [`nvs_runtime::CurrentStack`]'s pair — that last one because it is the only
/// one whose stale value is a *dangling pointer* rather than a wrong number,
/// which is what its own doc records.
fn yield_on(raw: *const (), waiting: Waiting) -> Resumed {
    // SAFETY: a non-null erased yielder is written in exactly one place —
    // `Scheduler::spawn`'s coroutine body, from `&Yielder` — into the task's
    // `Ctx` and into `RUNNING` in the same statement, and cleared from both
    // before the body returns. Reaching this line therefore means the yielder is
    // alive on the stack of the coroutine that is running right now, which is
    // the coroutine this call is standing on, and a `&Yielder` is all a suspend
    // needs. `Ctx` and `RUNNING` are both per-thread, so no other thread can
    // observe the pointer against a different stack.
    #[allow(
        unsafe_code,
        reason = "the one deref of the opaque yielder; the crate exists to own it"
    )]
    let yielder: &TaskYielder = unsafe { &*raw.cast::<TaskYielder>() };
    // Off the thread before the switch and back on after it. An unwind through
    // the switch — a suspended coroutine dropped during teardown — therefore
    // leaves `RUNNING` empty rather than naming a task whose stack is gone.
    let running = RUNNING.replace(None);
    // Off the thread beside `RUNNING` and for the same reason: the count
    // belongs to this stack, which is about to stop being the one the thread is
    // running. A task that never comes back leaves it at zero, and its own
    // guards saturate against that as they drop.
    let frames = nvs_runtime::HelperFrame::take();
    // The context this stack is running a compiled frame of, off the thread
    // beside the two above and for the reason
    // [`nvs_runtime::CurrentStack`] owns: the guard that installed it is on
    // *this* stack, so a task that installs while this one is parked would
    // otherwise save these words and write them back after this context has
    // been dropped.
    let current = nvs_runtime::CurrentStack::take();
    let resume = yielder.suspend(Suspended {
        waiting,
        unwindable: frames == 0,
    });
    current.restore();
    nvs_runtime::HelperFrame::restore(frames);
    RUNNING.set(running);
    match resume {
        Resume::Run => Resumed::Woken,
        Resume::Cancelled => Resumed::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::{MAX_POOLED_STACKS, TASK_STACK_SIZE};
    use crate::{Worker, cpus};
    use nvs_runtime::{NvsStr, OutputSink};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    /// A task's entry closure stays far inside [`CORO_TRANSFER_LIMIT`], which
    /// it does by carrying a pointer to the context rather than the context.
    ///
    /// The assertion in [`Scheduler::start`] is the one that fires on a real
    /// breach; this is the *headroom*, and it is the number worth watching.
    /// Spending it back down to nothing is how the limit gets reached: a
    /// context captured by value leaves a handful of bytes, and the next field
    /// added anywhere in the workspace turns every coroutine in the suite into
    /// `type is too big to transfer`.
    #[test]
    fn a_tasks_entry_closure_leaves_the_stack_switch_room_to_spare() {
        let carried = Box::new(ctx());
        let body: Box<dyn FnOnce(&mut Ctx)> = Box::new(|_| {});
        let id = TaskId(1);
        let root = TaskRoot::Request;
        // The capture set `Scheduler::start`'s closure has, and nothing else:
        // what is being pinned is that none of them is a whole `Ctx`.
        let entry = move |_yielder: &TaskYielder, _first: Resume| {
            let _ = (*carried, body, id, root);
        };
        let size = std::mem::size_of_val(&entry);
        assert!(
            size <= CORO_TRANSFER_LIMIT / 8,
            "a task's entry closure is {size} bytes against a limit of {CORO_TRANSFER_LIMIT} \
             — near enough to it that the next per-task field is a breach rather than a \
             cost. Put the state behind a pointer, as the context already is."
        );
    }

    /// A compiled frame that hands the core back once and returns, which is
    /// the only shape that arms [`nvs_runtime::CurrentStack`] from a test:
    /// `nvs_runtime::call` is what installs the pair, and it installs it
    /// around a call into compiled code.
    ///
    /// Nothing is written to `out` because `nvs_runtime::call` initialises the
    /// slot to null and null is this frame's result.
    #[expect(
        unsafe_code,
        reason = "an `NvsFn` is the compiled ABI's own signature; this body \
                  dereferences none of its three pointers"
    )]
    unsafe extern "C" fn parks_once(
        _ctx: *mut Ctx,
        _args: *const nvs_runtime::Value,
        _out: *mut nvs_runtime::Value,
    ) -> i32 {
        suspend_current(Waiting::Yielded);
        nvs_runtime::OK
    }

    #[test]
    fn a_task_that_parks_leaves_no_context_on_the_thread_for_the_next_one() {
        // The use-after-free `nvs_runtime::CurrentStack` exists to stop,
        // written as the interleaving that produces it. Two tasks each park
        // inside a compiled frame, so both hold a `CurrentCtx` guard across a
        // switch: A installs and parks, B installs and parks, A resumes and
        // *ends* — dropping its context — and only then does B's guard drop.
        //
        // Without `yield_on` carrying the pair, B's guard would save A's two
        // words on the way in and write them back here, putting a freed `Ctx`
        // and a freed live list on the thread, and the next object allocated on
        // this core would link itself onto a list that is gone. The assertion is
        // on the pointer being absent rather than on it being stale, because
        // reading through the stale one is the very thing that is undefined.
        let mut sched = Scheduler::new();
        for _ in 0..2 {
            sched.spawn(ctx(), TaskRoot::Request, |ctx| {
                nvs_runtime::call(parks_once, ctx, &[]).expect("the frame returns");
            });
        }
        sched.run();

        let after = nvs_runtime::CurrentStack::take();
        let armed = after.is_armed();
        after.restore();
        assert!(
            !armed,
            "a core with no task running carries no context; this one is holding the \
             one the first task installed, whose allocation is already gone"
        );
    }

    #[test]
    fn a_finished_task_hands_its_stack_back_for_the_next_one() {
        // `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`'s pool, from the only side that can observe it: a stack
        // is recycled at the end of a task and not at the end of the worker, so
        // a run of N sequential requests costs one reservation and not N.
        let mut sched = Scheduler::new();
        assert_eq!(sched.pooled_stacks(), 0, "a fresh pool holds nothing");

        sched.spawn(ctx(), TaskRoot::Request, |_| {});
        assert_eq!(sched.pooled_stacks(), 0, "a stack in use is not idle");
        sched.run();
        assert_eq!(sched.pooled_stacks(), 1, "a finished task's stack is kept");

        sched.spawn(ctx(), TaskRoot::Request, |_| {});
        assert_eq!(
            sched.pooled_stacks(),
            0,
            "the next spawn must take the idle stack rather than reserve another"
        );
        sched.run();
        assert_eq!(sched.pooled_stacks(), 1);
    }

    #[test]
    fn the_pool_never_holds_more_than_the_tasks_that_were_in_flight() {
        // The O(in-flight) claim, asserted as the bound rather than as a
        // number: two tasks alive at once leave two stacks, and running two
        // more afterwards leaves two still.
        let mut sched = Scheduler::new();
        for _ in 0..2 {
            sched.spawn(ctx(), TaskRoot::Request, |ctx| {
                suspend(ctx, Waiting::Yielded);
            });
        }
        sched.run();
        assert_eq!(sched.pooled_stacks(), 2);

        for _ in 0..2 {
            sched.spawn(ctx(), TaskRoot::Request, |_| {});
            sched.run();
        }
        assert_eq!(sched.pooled_stacks(), 2, "sequential tasks reuse, not grow");
    }

    #[test]
    fn a_task_s_recursion_limit_is_armed_from_its_own_stack() {
        // `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`'s last bullet. `Ctx::new` arms from the *worker's*
        // stack pointer and an asserted ceiling, which describes memory this
        // task never runs on; what the task must see is the pair computed from
        // the stack this crate handed it. Read on the task's own stack and
        // asserted outside it, so a failure is a failed test rather than a
        // contained panic.
        let seen: Rc<Cell<(usize, usize, usize)>> = Rc::new(Cell::new((0, 0, 0)));
        let report_to = Rc::clone(&seen);

        let mut sched = Scheduler::new();
        sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
            let frame = 0_u8;
            let (soft, hard) = ctx.stack_bounds();
            report_to.set((std::ptr::from_ref(&frame) as usize, soft, hard));
        });
        let report = sched.run();
        assert_eq!(report.finished, 1);

        let (frame, soft, hard) = seen.get();
        assert!(hard < soft, "the hard floor is below the soft limit");
        assert!(
            frame > soft,
            "the task's first frame must sit above its own soft limit"
        );
        assert!(
            frame - hard <= TASK_STACK_SIZE,
            "the floor must be on the task's stack, not on the worker's"
        );
    }

    #[test]
    fn a_spawned_task_runs_to_completion_on_the_run_queue() {
        let mut sched = Scheduler::new();
        let id = sched.spawn(ctx(), TaskRoot::Request, |ctx| {
            ctx.write_output(b"ran")
                .expect("a discarding sink cannot fail");
        });

        let report = sched.run();

        assert_eq!(report.finished, 1);
        assert_eq!(report.parked, 0);
        let finished = sched.take_finished();
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].id, id);
        assert!(finished[0].outcome.is_ok());
    }

    #[test]
    fn a_helper_reaches_the_yielder_through_the_context_alone() {
        // The whole async-colouring claim in one test: `deep` takes a `&Ctx`
        // and nothing else, is not marked in any way, and suspends. If this
        // ever needs a second argument, Novis has grown a colour.
        fn deep(ctx: &Ctx) -> bool {
            suspend(ctx, Waiting::Yielded).suspended()
        }

        let mut sched = Scheduler::new();
        sched.spawn(ctx(), TaskRoot::Request, |ctx| {
            assert!(deep(ctx), "a task's context must carry its yielder");
            assert!(deep(ctx), "and must still carry it after a suspension");
        });

        let report = sched.run();

        // Three resumes: the initial entry plus one per suspension.
        assert_eq!(report.resumes, 3);
        assert_eq!(report.finished, 1);
    }

    #[test]
    fn a_context_with_no_scheduler_beneath_it_refuses_to_suspend() {
        // The other half of the seam: the same call on the main stack reports
        // that nothing suspended, rather than pretending it waited.
        let ctx = ctx();
        assert!(!suspend(&ctx, Waiting::Parked).suspended());
        assert!(ctx.yielder().is_null());
    }

    #[test]
    fn a_parked_task_does_not_run_again_until_it_is_woken() {
        let mut sched = Scheduler::new();
        let id = sched.spawn(ctx(), TaskRoot::Request, |ctx| {
            suspend(ctx, Waiting::Parked);
            ctx.write_output(b"woken")
                .expect("a discarding sink cannot fail");
        });

        let first = sched.run();
        assert_eq!(first.finished, 0, "a parked task must not be resumed");
        assert_eq!(first.parked, 1);
        assert!(sched.take_finished().is_empty());

        assert!(sched.wake(id));
        let second = sched.run();
        assert_eq!(second.finished, 1);
        assert_eq!(second.parked, 0);
    }

    #[test]
    fn waking_a_task_that_is_not_parked_is_not_an_error() {
        let mut sched = Scheduler::new();
        let id = sched.spawn(ctx(), TaskRoot::Request, |_| {});
        // Issued but running, then finished — neither is parked, and a reactor
        // that raced with the task ending must not be told it failed.
        assert!(!sched.wake(id));
        sched.run();
        assert!(!sched.wake(id));
    }

    #[test]
    fn two_tasks_interleave_at_their_suspension_points() {
        // A cooperative scheduler is fair only at the points tasks give the
        // core back, so this asserts the *interleaving* rather than that both
        // finished: a run queue that drained one task fully before starting
        // the other would pass a "both finished" test and starve a neighbour.
        use std::cell::RefCell;
        use std::rc::Rc;

        let order = Rc::new(RefCell::new(Vec::new()));
        let mut sched = Scheduler::new();
        for name in ['a', 'b'] {
            let order = Rc::clone(&order);
            sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
                for step in 0..2 {
                    order.borrow_mut().push((name, step));
                    suspend(ctx, Waiting::Yielded);
                }
            });
        }

        sched.run();

        assert_eq!(
            *order.borrow(),
            vec![('a', 0), ('b', 0), ('a', 1), ('b', 1)]
        );
    }

    #[test]
    fn a_panic_in_one_task_is_contained_and_its_neighbours_still_run() {
        // `rule:http-server/containment-does-not-end-at-the-helper`'s outer boundary, applied by the scheduler rather than
        // written per task. The neighbour is what the test is really about:
        // containment that killed the core would still make the panicking
        // task's own assertion pass.
        let mut sched = Scheduler::new();
        let bad = sched.spawn(ctx(), TaskRoot::Request, |_| panic!("from a task"));
        let good = sched.spawn(ctx(), TaskRoot::Request, |ctx| {
            suspend(ctx, Waiting::Yielded);
            ctx.write_output(b"still here")
                .expect("a discarding sink cannot fail");
        });

        let report = sched.run();

        assert_eq!(report.finished, 2);
        let finished = sched.take_finished();
        let failed = finished.iter().find(|f| f.id == bad).expect("the panicker");
        let panic = failed
            .outcome
            .as_ref()
            .expect_err("the panic was contained");
        assert!(panic.message().contains("from a task"));
        assert!(
            !panic.retires_worker(),
            "a request fault is not a worker fault"
        );
        assert!(
            finished
                .iter()
                .find(|f| f.id == good)
                .expect("the neighbour")
                .outcome
                .is_ok()
        );
    }

    #[test]
    fn a_contained_panic_still_clears_the_yielder_from_the_context() {
        // The invariant that makes the opaque pointer sound: no context leaves
        // a coroutine holding a pointer into that coroutine's stack, and the
        // panic path is the one that would forget.
        let mut sched = Scheduler::new();
        sched.spawn(ctx(), TaskRoot::Request, |ctx| {
            assert!(suspend(ctx, Waiting::Yielded).suspended());
            panic!("after suspending once");
        });

        sched.run();

        let finished = sched.take_finished();
        assert!(finished[0].outcome.is_err());
        assert!(
            finished[0].ctx.yielder().is_null(),
            "a context must not carry a pointer into a stack that is gone"
        );
    }

    #[test]
    fn a_scheduler_dropped_with_a_parked_task_tears_it_down_instead_of_aborting() {
        // A worker retiring with a request still parked. Without
        // `nvs_runtime::Teardown`'s window this kills the whole process, so
        // the strongest half of this test is that it returns at all; the
        // assertion below is the other half — the stack really unwound, rather
        // than being leaked to dodge the abort.
        struct Marker(Rc<Cell<bool>>);
        impl Drop for Marker {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }

        let unwound = Rc::new(Cell::new(false));
        let marker = Marker(Rc::clone(&unwound));

        let mut sched = Scheduler::new();
        sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
            suspend(ctx, Waiting::Parked);
            // Keeps the marker alive across the park, so it is dropped by the
            // unwind rather than before it.
            drop(marker);
        });
        assert_eq!(sched.run().parked, 1);

        drop(sched);
        assert!(
            unwound.get(),
            "the parked task's stack was abandoned rather than unwound"
        );
    }

    #[test]
    fn there_is_no_current_task_off_a_scheduler() {
        assert!(current_task().is_none());
        assert!(
            !suspend_current(Waiting::Parked).suspended(),
            "a suspend with no task beneath it claimed to have parked"
        );
    }

    /// Two tasks taking turns: each has to see its **own** id every time it
    /// runs, which is what `RUNNING` being saved on the suspending task's own
    /// stack buys. A single publish at body entry would leave the second task's
    /// id standing when the first one resumed.
    #[test]
    fn a_task_sees_its_own_id_after_another_one_has_run() {
        let mut sched = Scheduler::new();

        let seen = Rc::new(RefCell::new(Vec::new()));
        let mut ids = Vec::new();
        for _ in 0..2 {
            let log = Rc::clone(&seen);
            ids.push(sched.spawn(ctx(), TaskRoot::Worker, move |ctx| {
                for _ in 0..3 {
                    log.borrow_mut()
                        .push(current_task().expect("a running task had no id"));
                    suspend(ctx, Waiting::Yielded);
                }
            }));
        }

        sched.run();
        assert!(current_task().is_none(), "an id outlived its task's turn");

        // The queue is FIFO, so the two alternate and no entry may name the
        // task that ran before it.
        let expected: Vec<_> = (0..3).flat_map(|_| ids.iter().copied()).collect();
        assert_eq!(*seen.borrow(), expected);
    }

    /// This crate's headline invariant, from the only two sides that can
    /// observe it: each worker builds and runs a queue of its *own*, and a task
    /// that suspends twice comes back on the thread it left. No I/O is
    /// involved — a scheduler with no reactor under it is still the thing that
    /// must not move a task, which is why the assertion lives here and not in
    /// `net`.
    #[test]
    fn one_core_runs_one_scheduler_and_a_task_never_migrates() {
        let listed = cpus();
        let Some(&first) = listed.first() else {
            // A platform that lists no CPU is supported; see `affinity`.
            return;
        };
        // One core is a legitimate machine: the two workers are still two
        // threads, which is what the migration half is asserted against.
        let second = listed.get(1).copied().unwrap_or(first);

        let start = |cpu| {
            Worker::spawn(cpu, move |sched| {
                let seen = Rc::new(RefCell::new(Vec::new()));
                for _ in 0..4 {
                    let log = Rc::clone(&seen);
                    sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
                        for _ in 0..3 {
                            log.borrow_mut().push(std::thread::current().id());
                            suspend(ctx, Waiting::Yielded);
                        }
                    });
                }
                let report = sched.run();
                // Cloned out rather than unwrapped: what crosses the join is
                // `Send`, and an `Rc` deliberately is not.
                let ran_on: Vec<_> = seen.borrow().clone();
                (std::thread::current().id(), report.finished, ran_on)
            })
            .expect("the OS refused a thread")
        };

        // Both started before either is joined, so the two schedulers are alive
        // at the same time rather than in sequence.
        let one = start(first);
        let two = start(second);
        let (one_thread, one_finished, one_seen) = one.join().expect("a worker panicked");
        let (two_thread, two_finished, two_seen) = two.join().expect("a worker panicked");

        assert_ne!(one_thread, two_thread, "two workers shared one thread");
        assert_ne!(
            one_thread,
            std::thread::current().id(),
            "a worker's scheduler ran on the thread that spawned it"
        );
        assert_eq!(one_finished, 4);
        assert_eq!(two_finished, 4);
        assert_eq!(
            one_seen.len(),
            12,
            "a task did not run every one of its turns"
        );
        assert_eq!(two_seen.len(), 12);
        assert!(
            one_seen.iter().all(|&id| id == one_thread),
            "a task resumed on a thread that is not its core's"
        );
        assert!(
            two_seen.iter().all(|&id| id == two_thread),
            "a task resumed on a thread that is not its core's"
        );
    }

    /// What the rule above *buys*, asserted where it is spent. An `NvsStr`'s
    /// count is a plain `Cell` in its header (`nvs_runtime::StrHeader`), and it
    /// is sound only because the value never leaves the core that made it — the
    /// type holds a raw pointer, so the compiler refuses to move one to another
    /// thread at all. What is left to check is that coroutines did not break it
    /// from *inside* a core: two tasks hold the same allocation, take and give
    /// back references either side of a suspension, and the count must be exact
    /// at every step and back to one at the end.
    #[test]
    fn a_refcount_is_still_non_atomic_under_the_scheduler() {
        let mut sched = Scheduler::new();
        let text = NvsStr::new(b"shared");
        assert_eq!(text.refcount(), 1);

        let counts = Rc::new(RefCell::new(Vec::new()));
        for _ in 0..2 {
            // One reference per task, taken here and given back when the task's
            // closure is dropped with it.
            let mine = text.clone();
            let log = Rc::clone(&counts);
            sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
                for _ in 0..3 {
                    let held = mine.clone();
                    log.borrow_mut().push(mine.refcount());
                    suspend(ctx, Waiting::Yielded);
                    log.borrow_mut().push(held.refcount());
                    drop(held);
                    suspend(ctx, Waiting::Yielded);
                }
            });
        }
        assert_eq!(text.refcount(), 3, "each task took one reference");

        sched.run();
        assert_eq!(
            text.refcount(),
            1,
            "a finished task's own reference outlived it"
        );

        // The queue is FIFO, so the two tasks alternate and the sequence is a
        // fact rather than a race: the first task clones (4), the second clones
        // (5), then each reads 5 and 4 as they give theirs back.
        let expected: Vec<usize> = (0..3).flat_map(|_| [4, 5, 5, 4]).collect();
        assert_eq!(
            *counts.borrow(),
            expected,
            "the count read differently than a single-threaded sequence gives"
        );
    }

    /// The run queue at the width a request load has, rather than at the width
    /// the tests above need: a thousand tasks created up front, each suspending
    /// twice, all driven to completion by one core. A stack is a *reservation*
    /// (`stack`'s module doc), so what this costs is address space and the
    /// pages the bodies touched.
    #[test]
    fn many_tasks_can_be_created_and_driven() {
        const TASKS: usize = 1_000;

        let mut sched = Scheduler::new();
        let ran = Rc::new(Cell::new(0_usize));
        for _ in 0..TASKS {
            let counted = Rc::clone(&ran);
            sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
                suspend(ctx, Waiting::Yielded);
                counted.set(counted.get() + 1);
                suspend(ctx, Waiting::Yielded);
            });
        }

        let report = sched.run();
        assert_eq!(report.finished, TASKS, "a task never reached its end");
        assert_eq!(report.parked, 0, "a yielding task was filed as parked");
        assert_eq!(
            report.resumes,
            TASKS * 3,
            "a task cost more or fewer stack switches than its two suspensions"
        );
        assert_eq!(
            ran.get(),
            TASKS,
            "a task's body did not run past its first turn"
        );
        assert_eq!(
            sched.pooled_stacks(),
            TASKS.min(MAX_POOLED_STACKS),
            "the pool kept a different number of stacks than the tasks in flight"
        );
    }

    /// The width `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`
    /// states its claim at, which is the only width that proves it: a hundred
    /// thousand tasks in flight on one core, each holding its own
    /// [`TASK_STACK_SIZE`] reservation before the first one is driven. That is
    /// 100 GiB of address space in a 64-bit process against a resident cost
    /// measured in the pages these shallow bodies wrote, and what is asserted
    /// is that one core drives every one of them to its end.
    ///
    /// **Skipped in the debug profile**, for the reason `nvs-cli`'s throughput
    /// guard is: `tools/verify.py` runs this binary beside every other test
    /// binary in the workspace, and a hundred thousand stack reservations and
    /// the switches they cost through an unoptimized scheduler is not a thing to
    /// do on that path. The driver runs it under `--release` once its sweep has
    /// finished — `tools/loop.py`'s release checks.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "100k stack reservations belong in the driver's release slot"
    )]
    fn a_hundred_thousand_tasks_in_flight_at_once_all_finish_on_one_core() {
        const TASKS: usize = 100_000;

        demand_the_mappings_this_many_stacks_needs(TASKS);

        let mut sched = Scheduler::new();
        let ran = Rc::new(Cell::new(0_usize));
        for _ in 0..TASKS {
            let counted = Rc::clone(&ran);
            sched.spawn(ctx(), TaskRoot::Request, move |ctx| {
                // Suspending is what puts every task in flight at the same
                // moment: `Scheduler::start` takes the stack at spawn, so the
                // hundred thousandth reservation is live while the first task
                // still has both of its turns left to run.
                suspend(ctx, Waiting::Yielded);
                counted.set(counted.get() + 1);
                suspend(ctx, Waiting::Yielded);
            });
        }

        let report = sched.run();
        assert_eq!(report.finished, TASKS, "a task never reached its end");
        assert_eq!(report.parked, 0, "a yielding task was filed as parked");
        assert_eq!(
            report.resumes,
            TASKS * 3,
            "a task cost more or fewer stack switches than its two suspensions"
        );
        assert_eq!(ran.get(), TASKS, "a task's body did not run past its turn");
        assert_eq!(
            sched.pooled_stacks(),
            MAX_POOLED_STACKS,
            "a hundred thousand finished tasks left the pool at something other \
             than its cap"
        );
    }

    /// Refuses to run the test above on a Linux kernel that cannot hold its
    /// mappings, naming the sysctl that fixes it.
    ///
    /// A stack is an `mmap` plus an `mprotect` (`stack`'s module doc), so a task
    /// in flight costs more than one VMA and Linux counts VMAs against
    /// `vm.max_map_count`, whose stock setting is far short of what this many
    /// stacks needs. The demand is on the kernel rather than on the test: a run
    /// that shrank until it fit would prove the claim at a width nobody doubts,
    /// on the one platform servers are actually deployed to.
    #[cfg(target_os = "linux")]
    fn demand_the_mappings_this_many_stacks_needs(tasks: usize) {
        /// Two mappings per stack and the process's own on top, at the value
        /// distributions ship as the raised setting.
        const MAPPINGS: usize = 262_144;
        const SYSCTL: &str = "/proc/sys/vm/max_map_count";

        let read = std::fs::read_to_string(SYSCTL)
            .unwrap_or_else(|err| panic!("{SYSCTL} could not be read: {err}"));
        let allowed: usize = read
            .trim()
            .parse()
            .unwrap_or_else(|err| panic!("{SYSCTL} did not hold a number: {err}"));
        assert!(
            allowed >= MAPPINGS,
            "{tasks} task stacks need about {MAPPINGS} mappings and this kernel allows \
             {allowed}: raise it with `sysctl -w vm.max_map_count={MAPPINGS}`"
        );
    }

    /// Windows and macOS cap no equivalent count, so there is nothing to demand.
    #[cfg(not(target_os = "linux"))]
    fn demand_the_mappings_this_many_stacks_needs(_tasks: usize) {}

    /// Sets its flag when it is dropped — a stand-in for the native teardown a
    /// cancelled task still owes: an arena released, a transaction rolled back,
    /// a file closed. `rule:concurrency/cancellation-runs-no-user-code` is the list.
    struct NativeDrop(Rc<Cell<bool>>);

    impl Drop for NativeDrop {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    /// Yields until `probe` reads true, or gives up after a bounded number of
    /// turns so a broken scheduler fails the test rather than hanging it.
    fn yield_until(task_ctx: &Ctx, probe: &Rc<Cell<bool>>) {
        for _ in 0..8 {
            if probe.get() {
                return;
            }
            suspend(task_ctx, Waiting::Yielded);
        }
    }

    #[test]
    fn a_child_takes_its_parent_from_the_task_that_spawned_it() {
        // `rule:concurrency/a-child-belongs-to-the-calling-task`'s "each is a child of the calling task", asserted as a
        // property of the *call*: nothing here passes a parent in, and the only
        // way `spawn_child` could get this wrong is by reading the wrong task.
        let seen: Rc<Cell<Option<TaskId>>> = Rc::new(Cell::new(None));
        let report_to = Rc::clone(&seen);

        let mut sched = Scheduler::new();
        let parent = sched.spawn(ctx(), TaskRoot::Request, move |task_ctx| {
            let child = spawn_child(ctx(), TaskRoot::Request, |child_ctx| {
                suspend(child_ctx, Waiting::Parked);
            })
            .expect("a task running under a scheduler can spawn a child");
            report_to.set(Some(child));
            // Parked rather than returned, so both are still live to look at.
            suspend(task_ctx, Waiting::Parked);
        });
        sched.run();

        let child = seen.get().expect("the parent never published a child id");
        assert_eq!(
            sched.parent_of(child),
            Some(parent),
            "the child's parent is not the task that spawned it"
        );
        assert_eq!(
            sched.children_of(parent),
            vec![child],
            "the parent does not list the child it spawned"
        );
        assert_eq!(
            sched.parent_of(parent),
            None,
            "a task spawned from the worker itself is a root"
        );
        assert_eq!(sched.tracked_tasks(), 2);
    }

    #[test]
    fn the_tree_holds_a_node_for_every_live_task_and_no_others() {
        // `rule:programs/memory-priority`'s O(in-flight) test applied to the tree itself: a node is
        // held for a task that has not ended, and a run of sequential requests
        // costs one node rather than one per request served.
        let mut sched = Scheduler::new();
        assert_eq!(sched.tracked_tasks(), 0, "a fresh scheduler tracks nothing");

        sched.spawn(ctx(), TaskRoot::Request, |task_ctx| {
            suspend(task_ctx, Waiting::Parked);
        });
        assert_eq!(
            sched.tracked_tasks(),
            1,
            "a spawned task is in the tree before it is ever resumed"
        );

        for _ in 0..4 {
            sched.spawn(ctx(), TaskRoot::Request, |_| {});
            sched.run();
        }
        assert_eq!(sched.take_finished().len(), 4);
        assert_eq!(
            sched.tracked_tasks(),
            1,
            "the tree grew with tasks served rather than with tasks in flight"
        );
    }

    #[test]
    fn no_call_returns_with_a_child_still_running() {
        // `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s whole promise. The member that makes it is Stage 4's;
        // what the scheduler owes underneath it is that a parent returning with
        // a child parked mid-work leaves nothing that can run another
        // instruction — the child holding a row lock in that section's example.
        let parked = Rc::new(Cell::new(false));
        let child_parked = Rc::clone(&parked);
        let watched_by_parent = Rc::clone(&parked);
        let past_safepoint = Rc::new(Cell::new(false));
        let child_past_safepoint = Rc::clone(&past_safepoint);
        let seen: Rc<Cell<Option<TaskId>>> = Rc::new(Cell::new(None));
        let report_to = Rc::clone(&seen);

        let mut sched = Scheduler::new();
        sched.spawn(ctx(), TaskRoot::Request, move |task_ctx| {
            let child = spawn_child(ctx(), TaskRoot::Request, move |child_ctx| {
                child_parked.set(true);
                suspend(child_ctx, Waiting::Parked);
                child_past_safepoint.set(true);
            })
            .expect("a task running under a scheduler can spawn a child");
            report_to.set(Some(child));
            yield_until(task_ctx, &watched_by_parent);
            // And returns without waiting for it, which is the case the
            // guarantee has to survive.
        });

        let report = sched.run();
        let child = seen.get().expect("the parent never published a child id");
        assert!(parked.get(), "the child never reached its safepoint");
        assert_eq!(report.finished, 1, "only the parent reached its own end");
        assert_eq!(report.cancelled, 1, "the child was left running");
        assert_eq!(sched.ready_count(), 0);
        assert_eq!(sched.parked_count(), 0);
        assert!(
            sched.take_ended().contains(&child),
            "the torn-down child's registrations have nothing left to retire them"
        );
        assert!(
            !past_safepoint.get(),
            "the child ran an instruction past the safepoint it was torn down at"
        );
        assert_eq!(sched.tracked_tasks(), 0, "the tree kept a dead task");
    }

    #[test]
    fn a_task_tree_dies_with_its_parent_and_leaves_no_orphan() {
        // Three generations, all parked on something that never arrives, and one
        // cancellation at the root. The grandchild is the one that matters: a
        // sweep that only walks the cancelled task's own children leaves it
        // alive, holding a stack and a budget charged to a request that is over.
        let torn: Rc<RefCell<Vec<TaskId>>> = Rc::new(RefCell::new(Vec::new()));
        let ids: Rc<RefCell<Vec<TaskId>>> = Rc::new(RefCell::new(Vec::new()));

        let mut sched = Scheduler::new();
        let root = {
            let ids = Rc::clone(&ids);
            let torn = Rc::clone(&torn);
            sched.spawn(ctx(), TaskRoot::Request, move |task_ctx| {
                let grandchild_ids = Rc::clone(&ids);
                let grandchild_torn = Rc::clone(&torn);
                let child = spawn_child(ctx(), TaskRoot::Request, move |child_ctx| {
                    let grandchild = spawn_child(ctx(), TaskRoot::Request, move |g_ctx| {
                        let id = current_task().expect("a running task has an id");
                        let _teardown = NativeDrop(Rc::new(Cell::new(false)));
                        grandchild_torn.borrow_mut().push(id);
                        suspend(g_ctx, Waiting::Parked);
                    })
                    .expect("a child can spawn a child");
                    grandchild_ids.borrow_mut().push(grandchild);
                    suspend(child_ctx, Waiting::Parked);
                })
                .expect("a task running under a scheduler can spawn a child");
                ids.borrow_mut().push(child);
                suspend(task_ctx, Waiting::Parked);
            })
        };
        sched.run();
        assert_eq!(sched.tracked_tasks(), 3, "three generations are live");
        assert_eq!(sched.parked_count(), 3);
        assert_eq!(torn.borrow().len(), 1, "the grandchild never ran");

        assert_eq!(
            sched.cancel(root),
            3,
            "the mark did not reach the grandchild"
        );
        let report = sched.run();

        assert_eq!(report.cancelled, 3, "a generation survived its ancestor");
        assert_eq!(sched.parked_count(), 0, "an orphan is still parked");
        assert_eq!(sched.ready_count(), 0);
        assert_eq!(
            sched.tracked_tasks(),
            0,
            "the tree outlived every task in it"
        );
        let mut ended = sched.take_ended();
        ended.sort_unstable();
        let mut expected = ids.borrow().clone();
        expected.push(root);
        expected.sort_unstable();
        assert_eq!(ended, expected);
        assert_eq!(
            sched.pooled_stacks(),
            3,
            "a torn-down task's stack was handed back to the OS instead of the pool"
        );
    }

    #[test]
    fn a_cancelled_task_runs_no_catch_and_no_cleanup_block() {
        // `rule:concurrency/cancellation-runs-no-user-code`, and the intuitive implementation is the wrong one: what
        // a cancelled task still owes is *native* teardown, and everything a
        // program wrote for its own way out — a catch clause, a cleanup block, a
        // registered handler — does not run. Here the native half is a `Drop`
        // and the script half is every statement past the safepoint.
        let dropped = Rc::new(Cell::new(false));
        let native = Rc::clone(&dropped);
        let caught = Rc::new(Cell::new(false));
        let ran_catch = Rc::clone(&caught);
        let cleaned = Rc::new(Cell::new(false));
        let ran_cleanup = Rc::clone(&cleaned);

        let mut sched = Scheduler::new();
        let id = sched.spawn(ctx(), TaskRoot::Request, move |task_ctx| {
            let _native = NativeDrop(native);
            suspend(task_ctx, Waiting::Parked);
            ran_catch.set(true);
            ran_cleanup.set(true);
        });
        sched.run();
        assert!(
            !dropped.get(),
            "the task was torn down before it was cancelled"
        );

        sched.cancel(id);
        let report = sched.run();

        assert_eq!(report.cancelled, 1);
        assert!(dropped.get(), "native teardown did not run");
        assert!(!caught.get(), "a catch clause ran on the way out");
        assert!(!cleaned.get(), "a cleanup block ran on the way out");
        assert!(
            sched.take_finished().is_empty(),
            "a cancelled task handed back a Finished, so the unwind was caught \
             by the containment boundary instead of passing through it"
        );
    }

    #[test]
    fn a_cancelled_task_standing_on_a_helper_frame_is_resumed_and_not_unwound() {
        // The abort this closes, and the reason `Core\Time::sleep` can park at
        // all: a forced unwind may not cross an `extern "C"` frame, so a task
        // parked under one is *told* it is cancelled and dies by `rule:errors/propagation`'s
        // return status instead. `HelperFrame` is what the park reads, and it
        // is the same guard `nvs_runtime::run_helper` holds around every
        // `Core` member.
        let told = Rc::new(Cell::new(false));
        let saw = Rc::clone(&told);
        let dropped = Rc::new(Cell::new(false));
        let native = Rc::clone(&dropped);

        let mut sched = Scheduler::new();
        let id = sched.spawn(ctx(), TaskRoot::Request, move |task_ctx| {
            let _native = NativeDrop(native);
            let _frame = nvs_runtime::HelperFrame::enter();
            saw.set(suspend(task_ctx, Waiting::Parked).cancelled());
        });
        sched.run();

        sched.cancel(id);
        let report = sched.run();

        assert!(told.get(), "the park was never told about the cancellation");
        assert_eq!(
            report.cancelled, 0,
            "the stack was force-unwound after all, which is the abort"
        );
        assert!(dropped.get(), "the task did not reach its own end");
        assert_eq!(
            sched.take_finished().len(),
            1,
            "a task that returns hands back a Finished, cancelled or not"
        );
    }

    #[test]
    fn a_cancellation_is_delivered_once_and_a_park_after_it_is_left_alone() {
        // Delivery-once is what keeps `run`'s parked sweep from spinning: a
        // park site that ignores the notice would otherwise be collected,
        // resumed and re-parked on every turn, for as long as the scheduler
        // lived.
        let notices = Rc::new(Cell::new(0_usize));
        let counted = Rc::clone(&notices);

        let mut sched = Scheduler::new();
        let id = sched.spawn(ctx(), TaskRoot::Request, move |task_ctx| {
            let _frame = nvs_runtime::HelperFrame::enter();
            for _ in 0..2 {
                if suspend(task_ctx, Waiting::Parked).cancelled() {
                    counted.set(counted.get() + 1);
                }
            }
        });
        sched.run();

        sched.cancel(id);
        let report = sched.run();
        assert_eq!(notices.get(), 1, "the notice did not arrive");
        assert_eq!(
            report.parked, 1,
            "a task that parked again after its notice was collected again"
        );

        // And an ordinary wake still resumes it, with no second notice: the
        // fact was delivered, not the flag re-read.
        assert!(sched.wake(id));
        let report = sched.run();
        assert_eq!(notices.get(), 1, "the notice was delivered twice");
        assert_eq!(report.finished, 1);
    }

    #[test]
    fn a_worker_retiring_over_a_helper_frame_leaks_rather_than_aborts() {
        // `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`: no path reaches `abort()`. `Drop for Scheduler` unwinds
        // what it can and leaks what it cannot, and the leak is deliberate —
        // the doc on that `Drop` says what it spends. Reaching the assert at
        // all is the whole test; the process not surviving is the failure.
        let mut sched = Scheduler::new();
        sched.spawn(ctx(), TaskRoot::Request, move |task_ctx| {
            let _frame = nvs_runtime::HelperFrame::enter();
            suspend(task_ctx, Waiting::Parked);
        });
        let report = sched.run();
        assert_eq!(report.parked, 1);
        drop(sched);
    }

    #[test]
    fn a_cancelled_tasks_arena_is_released() {
        // The same teardown seen from the memory side, which is the half ADR
        // 0004 cares about: what a cancelled task was holding is released at the
        // moment it dies, not at the end of the worker. An `Rc` stands in for
        // the arena because it is the one holder a test can count.
        let arena = Rc::new(vec![0_u8; 64]);
        let held = Rc::clone(&arena);

        let mut sched = Scheduler::new();
        let id = sched.spawn(ctx(), TaskRoot::Request, move |task_ctx| {
            let _arena = held;
            suspend(task_ctx, Waiting::Parked);
        });
        sched.run();
        assert_eq!(
            Rc::strong_count(&arena),
            2,
            "a parked task is not holding what it allocated"
        );

        sched.cancel(id);
        sched.run();
        assert_eq!(
            Rc::strong_count(&arena),
            1,
            "the cancelled task's memory outlived the task"
        );
    }
}
