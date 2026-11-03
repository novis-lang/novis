//! Every way a parked task is torn down, at the seam that does it: the forced
//! unwind of the task's stack in `nvs_host::Scheduler`
//! (`rule:concurrency/cancellation-runs-no-user-code`).
//!
//! **Why these tests exist beside the scheduler's own.** The scheduler's unit
//! tests cancel tasks too, but each test here parks the way a real worker does:
//! a native body under `TaskRoot::Worker`, a few calls deep, holding a value
//! whose `Drop` is the native teardown the rule still owes. That is the shape of
//! the queue worker parked in its connect (`nvs-cli`'s `worker.rs`), and on
//! Windows the optimized profiles crashed in that unwind while debug passed. So
//! these run in the `proof` profile as well as in debug, and each asserts the
//! same three things: every held value is dropped exactly once, no code after
//! the park runs, and the scheduler runs a fresh task afterwards.
//!
//! **No task parks under an `extern "C"` frame.** A forced unwind cannot cross
//! one and the process aborts, which is a different fault from the one these
//! pin; the scheduler leaks such a task instead (`Drop for Scheduler`).

use std::cell::Cell;
use std::rc::Rc;

use nvs_host::{Scheduler, TaskId, Waiting, cancel_task, suspend};
use nvs_runtime::{Ctx, OutputSink, TaskRoot};

/// The root a queue worker runs under, so a fault here is charged the way a
/// worker's is.
const ROOT: TaskRoot = TaskRoot::Worker;

fn ctx() -> Ctx {
    Ctx::new(OutputSink::Sink)
}

/// Counts its own drop into a shared cell: a stand-in for the native teardown
/// a cancelled task still owes, such as a socket closed or an arena released.
struct Held {
    drops: Rc<Cell<usize>>,
}

impl Drop for Held {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

/// What one parked task reports back to its test.
#[derive(Default)]
struct Probe {
    /// How many times the task's [`Held`] was dropped.
    drops: Rc<Cell<usize>>,
    /// Whether the task reached its park.
    parked: Rc<Cell<bool>>,
    /// Whether any code after the park ran. A torn-down task never gets there.
    resumed: Rc<Cell<bool>>,
}

impl Probe {
    /// The body of a task that holds a [`Held`] a few calls deep and parks
    /// there, as the queue worker parks inside its connect.
    fn body(&self) -> impl FnOnce(&mut Ctx) + 'static {
        let held = Held {
            drops: Rc::clone(&self.drops),
        };
        let parked = Rc::clone(&self.parked);
        let resumed = Rc::clone(&self.resumed);
        move |task_ctx| open(task_ctx, held, &parked, &resumed)
    }

    fn assert_torn_down(&self, what: &str) {
        assert!(self.parked.get(), "{what}: the task never reached its park");
        assert_eq!(
            self.drops.get(),
            1,
            "{what}: what the task held was not dropped exactly once"
        );
        assert!(
            !self.resumed.get(),
            "{what}: code after the park ran in a cancelled task"
        );
    }
}

/// The outer frame of the park, standing in for the worker's `open`.
#[inline(never)]
fn open(task_ctx: &mut Ctx, held: Held, parked: &Cell<bool>, resumed: &Cell<bool>) {
    let local = held;
    connect(task_ctx, parked, resumed);
    std::hint::black_box(&local);
}

/// The inner frame, standing in for the driver's connect: it parks with
/// nothing arranged to wake it, so only a teardown ends it.
#[inline(never)]
fn connect(task_ctx: &Ctx, parked: &Cell<bool>, resumed: &Cell<bool>) {
    parked.set(true);
    suspend(task_ctx, Waiting::Parked);
    resumed.set(true);
}

/// Spawns a task that yields once and returns, runs the scheduler, and asserts
/// that it finished: the scheduler still works after a teardown.
fn assert_runs_on(sched: &mut Scheduler, what: &str) {
    let finished = Rc::new(Cell::new(false));
    let done = Rc::clone(&finished);
    sched.spawn(ctx(), ROOT, move |task_ctx| {
        suspend(task_ctx, Waiting::Yielded);
        done.set(true);
    });
    let report = sched.run();
    assert!(
        finished.get(),
        "{what}: a task spawned after the teardown did not run to its end"
    );
    assert_eq!(report.parked, 0, "{what}: a task is still parked");
}

#[test]
fn a_parked_task_cancelled_by_the_scheduler_unwinds_and_drops_what_it_held() {
    let probe = Probe::default();
    let mut sched = Scheduler::new();
    let id = sched.spawn(ctx(), ROOT, probe.body());
    let report = sched.run();
    assert_eq!(report.parked, 1, "the task did not park");
    assert_eq!(probe.drops.get(), 0, "a parked task dropped what it holds");

    assert_eq!(sched.cancel(id), 1, "the cancel marked no task");
    let report = sched.run();
    assert_eq!(report.cancelled, 1, "the scheduler tore down no task");
    probe.assert_torn_down("a cancel by the scheduler");
    assert_runs_on(&mut sched, "a cancel by the scheduler");
}

#[test]
fn a_parked_task_cancelled_from_inside_another_task_unwinds_and_drops_what_it_held() {
    let probe = Probe::default();
    let mut sched = Scheduler::new();
    let victim = sched.spawn(ctx(), ROOT, probe.body());
    sched.run();
    assert!(probe.parked.get(), "the task did not park");

    // A second task cancels the first and returns, as `nvs run`'s script task
    // stops a worker on its way out.
    let marked = Rc::new(Cell::new(0));
    let marked_by = Rc::clone(&marked);
    sched.spawn(ctx(), TaskRoot::Request, move |_| {
        marked_by.set(cancel_task(victim));
    });
    let report = sched.run();
    assert_eq!(marked.get(), 1, "`cancel_task` marked no task");
    assert_eq!(report.cancelled, 1, "the scheduler tore down no task");
    probe.assert_torn_down("a cancel from inside another task");
    assert_runs_on(&mut sched, "a cancel from inside another task");
}

#[test]
fn a_scheduler_dropped_with_a_parked_task_unwinds_it_and_drops_what_it_held() {
    let probe = Probe::default();
    let mut sched = Scheduler::new();
    sched.spawn(ctx(), ROOT, probe.body());
    let report = sched.run();
    assert_eq!(report.parked, 1, "the task did not park");

    // A worker retiring with a request still parked.
    drop(sched);
    probe.assert_torn_down("the scheduler's drop");
    let mut next = Scheduler::new();
    assert_runs_on(&mut next, "the scheduler's drop");
}

#[test]
fn a_task_cancelled_before_its_first_turn_drops_its_body_unrun() {
    let probe = Probe::default();
    let mut sched = Scheduler::new();
    let id = sched.spawn(ctx(), ROOT, probe.body());
    assert_eq!(sched.cancel(id), 1, "the cancel marked no task");
    sched.run();

    assert!(!probe.parked.get(), "a task cancelled before its turn ran");
    assert_eq!(
        probe.drops.get(),
        1,
        "the unrun body's captures were not dropped exactly once"
    );
    assert_runs_on(&mut sched, "a cancel before the first turn");
}

#[test]
fn a_thousand_parked_tasks_cancelled_in_one_turn_each_drop_once() {
    const TASKS: usize = 1000;
    let probes: Vec<Probe> = (0..TASKS).map(|_| Probe::default()).collect();
    let mut sched = Scheduler::new();
    let ids: Vec<TaskId> = probes
        .iter()
        .map(|probe| sched.spawn(ctx(), ROOT, probe.body()))
        .collect();
    let report = sched.run();
    assert_eq!(report.parked, TASKS, "not every task parked");

    for id in &ids {
        assert_eq!(sched.cancel(*id), 1, "a cancel marked no task");
    }
    let report = sched.run();
    assert_eq!(report.cancelled, TASKS, "not every task was torn down");
    let wrong: Vec<usize> = probes
        .iter()
        .enumerate()
        .filter(|(_, probe)| probe.drops.get() != 1 || probe.resumed.get())
        .map(|(index, _)| index)
        .collect();
    assert!(
        wrong.is_empty(),
        "these tasks were not torn down cleanly: {wrong:?}"
    );
    assert_runs_on(&mut sched, "a thousand cancels in one turn");
}
