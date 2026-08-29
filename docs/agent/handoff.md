# Handoff

## State

**Goal 2, Stage 3's task tree is on disk** — the first item of Stage 3, and four of the five
`nvs-host` names its `cargo-named` check asks for are green over it (`loop-goal.toml:1465`). 83
tests in the crate on Windows, up from 77.

Every task now has a parent and a list of children, and the parent is taken from the spawning task
rather than passed in. The design and every decision inside it is recorded in `scheduler.rs`'s
module doc § *The task tree, and what cancelling one costs*, which is its only home — no ADR slot
was free, per the goal's standing decisions. The three things worth knowing before touching it: a
running task reaches the tree through a thread-local `Rc<RefCell<TaskTree>>` and not through the
scheduler (see the new playbook bullet for why); cancellation *marks* a subtree and the scheduler
unwinds it on its own stack at the marked task's next safepoint, or at once if it is already
parked; and a task's death cancels what it left running, which is where ADR 0072 § 4's "nothing
still running" actually lives.

**A cancelled task hands back no `Finished`.** Its `Ctx` is dropped on its own stack by the forced
unwind, so its output and exit code are not readable afterwards — `Scheduler::take_cancelled` gives
the ids, and `run_until_idle` uses the same list to drop reactor registrations that would otherwise
outlive their task.

**Stage 3's remaining name is `a_bounded_channel_send_suspends_rather_than_growing`**, which is
item 11 and needs a channel module that does not exist. That is the next group, and it is why this
session stopped here: it shares nothing with the file this one had open.

**The acceptance failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still
not a regression** — `Task::all` is Stage 4's item 12, and no `Core\Task` member exists yet.

**The orientation pack was missing nothing this item needed**, and the gap the previous handoff
predicted was real: `[context] adrs` carries ADR 0072 §§ 4 and 5 only, and §§ 1 and 6 both decided
something here (§ 1 that the parent comes from the caller, § 6 that `afterResponse` is a child of
the request tree rather than of the registering task). Add both to that field.

## Next group

**Item 11's bounded channel, in `nvs-host`.** File set: a new `crates/nvs-host/src/channel.rs`,
`crates/nvs-host/src/lib.rs`, and `crates/nvs-host/src/scheduler.rs` for the park/wake pair it is
built on. Anchors, all in `scheduler.rs`: `wake:527`, `run:544`, `spawn_child:782`,
`cancel_task:810`, `suspend_current:899`, `TaskTree:211`.

- [ ] **The channel itself** — a bounded queue whose `send` suspends when it is full and whose
      `recv` suspends when it is empty, waking the other side by `TaskId` through
      `Scheduler::wake` (`scheduler.rs:527`). It is `!Send` and per-core like everything else here.
      Decide-and-record in the new module's doc; loop-goal.md item 11, ADR 0072's roster.
- [ ] **`a_bounded_channel_send_suspends_rather_than_growing`** (`loop-goal.toml:1470`) — the
      backpressure claim asserted as a bound on both sides: the last send that is accepted without
      suspending, and the first one that suspends.
- [ ] **A cancelled task blocked on a channel is torn down like any other** — it is parked, so the
      sweep at `run:544` already reaches it; what needs asserting is that the peer's queue does not
      keep its entry alive.

## Backlog

- Stage 3's `nvs-ir` half: `a_spawn_lowers_to_a_task_on_the_current_core` and
  `an_await_suspends_until_its_task_completes` (`loop-goal.toml:1456`) — a different crate and a
  different file set from everything above.
- Stage 4's `Core\Task` roster, which is what the standing acceptance failure on
  `examples/tasks.nvs` is waiting for — loop-goal.md items 12-16.
- `[context] adrs` in `docs/agent/loop-goal.toml` needs ADR 0072 §§ 1 and 6 (see § State).
- M4's residue: the 1000-case conformance corpus count, docs/plan/m4.md.
