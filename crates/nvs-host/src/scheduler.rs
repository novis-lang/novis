//! One core, one scheduler, one run queue of stackful coroutines.
//!
//! This is [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//! § 6's "one single-threaded scheduler of our own pinned per core", and the
//! shape `benches/abi-probe` has been modelling since M0 — its `Ctx` doc calls
//! itself "deliberately shaped like the real `Ctx` will be", and this module is
//! what it was shaped like.
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
//! `Core\File::read` has the same signature whether or not there is a scheduler
//! beneath it, and Novis's surface never grows a colour.
//!
//! What it spends, as [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md)
//! requires: **one stack per in-flight task**, held for as long as that task is
//! suspended and freed when it completes. That is O(in-flight) and not
//! O(requests served), which is the test that section applies. How large a
//! stack is, and whether it is pooled, is ADR 0115's to decide along with the
//! reactor; until then a task takes `corosensei`'s default.
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

use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;

use corosensei::{Coroutine, CoroutineResult, Yielder};
use nvs_runtime::{Ctx, TaskPanic, TaskRoot};

/// What a suspended task is waiting for.
///
/// Two variants, because there are two reasons a task is not running and they
/// need different answers from the scheduler. ADR 0115 adds the I/O
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

/// The yielder a task suspends through — private to this crate, because the
/// only pointer to one that escapes is the opaque `*const ()` in [`Ctx`].
type TaskYielder = Yielder<(), Waiting>;

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

/// A task that reached its end, handed back with the context it ran under.
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
    /// [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
    /// § 2's outer boundary. [`TaskPanic::retires_worker`] is the only
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
    /// the state ADR 0115's reactor waits in.
    pub parked: usize,
}

struct Task {
    id: TaskId,
    coro: Coroutine<(), Waiting, Finished>,
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
    next_id: u64,
    ready: VecDeque<Task>,
    parked: HashMap<TaskId, Task>,
    finished: Vec<Finished>,
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
    /// [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
    /// § 5's rule: what the unwind runs is native `Drop`, so an arena is
    /// released and a handle is closed, and no `catch` or `finally` is
    /// consulted.
    fn drop(&mut self) {
        if self.ready.is_empty() && self.parked.is_empty() {
            return;
        }
        let _teardown = nvs_runtime::Teardown::enter();
        self.ready.clear();
        self.parked.clear();
    }
}

impl Scheduler {
    /// An empty run queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_id: 0,
            ready: VecDeque::new(),
            parked: HashMap::new(),
            finished: Vec::new(),
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
    /// `root` decides what a panic beneath this task costs: ADR 0106 § 2 fails
    /// a [`TaskRoot::Request`] as one request and retires the worker for a
    /// [`TaskRoot::Worker`].
    ///
    /// Nothing runs until [`Scheduler::run`] is called.
    pub fn spawn<F>(&mut self, ctx: Ctx, root: TaskRoot, body: F) -> TaskId
    where
        F: FnOnce(&mut Ctx) + 'static,
    {
        let id = TaskId(self.next_id);
        self.next_id += 1;

        let mut ctx = ctx;
        let coro = Coroutine::new(move |yielder: &TaskYielder, ()| {
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
        });

        self.ready.push_back(Task { id, coro });
        id
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
        while let Some(mut task) = self.ready.pop_front() {
            report.resumes += 1;
            match task.coro.resume(()) {
                CoroutineResult::Yield(Waiting::Yielded) => self.ready.push_back(task),
                CoroutineResult::Yield(Waiting::Parked) => {
                    self.parked.insert(task.id, task);
                }
                CoroutineResult::Return(finished) => {
                    report.finished += 1;
                    self.finished.push(finished);
                }
            }
        }
        report.parked = self.parked.len();
        report
    }

    /// Takes the tasks that have ended since this was last called.
    pub fn take_finished(&mut self) -> Vec<Finished> {
        std::mem::take(&mut self.finished)
    }

    /// The tasks that have ended and have not been taken yet.
    ///
    /// Reading without draining, because two consumers want different things
    /// from the same list: whoever owns the request boundary takes it, and
    /// [`crate::reactor::run_until_idle`] only needs the ids in order to drop
    /// their reactor registrations (ADR 0115 § 2 rule 3) and must not consume
    /// what it did not ask for.
    #[must_use]
    pub fn finished(&self) -> &[Finished] {
        &self.finished
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
}

/// Suspends the task `ctx` is running inside, reporting whether there was one.
///
/// **This is the seam that removes async colouring.** The caller is an
/// ordinary function taking an ordinary `&Ctx`: it does not know whether it is
/// on a coroutine stack, its own callers are not marked, and nothing about
/// Novis's calling convention changes because it might wait.
///
/// `false` means there is no scheduler beneath this context — a `Core` member
/// reached from `nvs run`, or a unit test — and **the caller must then do
/// something else**: block, or fail. It must not treat a refusal as a
/// successful wait, because nothing suspended and control is about to run on
/// as if it had.
///
/// # Panics
///
/// Never by itself. The coroutine machinery underneath aborts only on a stack
/// overflow of the task's own stack, which is
/// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's
/// territory and is guarded by `Ctx`'s stack limit long before it is reached.
pub fn suspend(ctx: &Ctx, waiting: Waiting) -> bool {
    let raw = ctx.yielder();
    if raw.is_null() {
        return false;
    }
    yield_on(raw, waiting);
    true
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

/// Suspends the running task without a [`Ctx`] to reach the yielder through,
/// reporting whether there was one.
///
/// [`suspend`] is the seam for a helper that *has* a context, and it stays the
/// one every `Core` member uses. This is for the code that has no argument to
/// carry one in — the parking `Read` and `Write` of a socket, whose signatures
/// are `std::io`'s and not ours.
///
/// `false` means the same thing it means there, and obliges the caller the same
/// way: nothing suspended, so block or fail rather than run on.
pub fn suspend_current(waiting: Waiting) -> bool {
    match RUNNING.get() {
        Some(running) => {
            yield_on(running.yielder, waiting);
            true
        }
        None => false,
    }
}

/// Hands the core back through an erased `&TaskYielder`, and restores this
/// task's identity when control comes back to this stack.
fn yield_on(raw: *const (), waiting: Waiting) {
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
    yielder.suspend(waiting);
    RUNNING.set(running);
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::OutputSink;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
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
            suspend(ctx, Waiting::Yielded)
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
        assert!(!suspend(&ctx, Waiting::Parked));
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
        // ADR 0106 § 2's outer boundary, applied by the scheduler rather than
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
            assert!(suspend(ctx, Waiting::Yielded));
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
        // A worker retiring with a request still parked. Before
        // `nvs_runtime::Teardown` existed this killed the whole process, so
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
            !suspend_current(Waiting::Parked),
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
}
