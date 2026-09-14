# Handoff

## State

**Goal `m5-proofs` (M5), stage 3 is complete.** The request and session refusals landed last
session; the three isolate proofs the Verify paragraph names are now on disk, and the stage's
`cargo-named` check has all three of its tests.

A child's memory breach and its CPU breach each cross as `ok = false` with the parent running
(`crates/nvs-host/tests/limits.rs`), reading the limit out of `rule:errors/on-limit`'s report as the
depth case beside them does. The CPU case also asserts the flag is still up on the parent, because
`Ctx::isolate` shares the safepoint word: the parent keeps running in the sense
`rule:security/isolate-failure-is-a-value` means, while the tree's ceiling still stands.

The cyclic-argument proof went into `crates/nvs-host/src/isolate.rs`'s own test module rather than
into `limits.rs` as the last handoff proposed — it is a crossing and not a limit, and that module
already holds the `parent`, `run` and class-resolution fixtures it needs. Same `-p nvs-host`, which
is all the check names.

**Stage 2 is the earliest red stage and nothing of it is written.** Nothing is blocked.

The pack's `[context] rules` is missing `errors/on-limit` and `security/isolate-budget-is-the-trees`;
both are cited by the tests this stage just landed.

## Next group

**Stage 2: the deadlock** — one file set: `crates/nvs-host/src/channel.rs` and
`crates/nvs-host/src/group.rs`, both in their own `#[cfg(test)]` modules, plus one `.nvst` case.

- [ ] **Two tasks waiting on each other's channel are ended by the group's deadline** — the test name
      `two_tasks_waiting_on_each_others_channel_are_ended_by_the_groups_deadline` verbatim, in
      `crates/nvs-host/src/channel.rs:540`'s test module over the `recv` that suspends at
      `crates/nvs-host/src/channel.rs:352` and the group's own expiry at
      `crates/nvs-host/src/group.rs:350`.
      `rule:concurrency/nothing-is-still-running-when-a-call-returns`.
- [ ] **A deadlocked pair is ended by cancelling the task that awaits them** —
      `a_deadlocked_pair_is_ended_by_cancelling_the_task_that_awaits_them`, the other half of the
      same check, in `crates/nvs-host/src/group.rs:573`'s test module.
      `rule:concurrency/cancellation-runs-no-user-code`.
- [ ] **The same deadlock as a program** —
      `tests/conformance/task/a-deliberate-deadlock-is-ended-by-the-deadline-and-leaves-no-child-running.nvst`,
      the stage's second check, written beside
      `tests/conformance/task/a-task-tree-dies-with-its-parent.nvst:1`.
      `rule:concurrency/nothing-is-still-running-when-a-call-returns`.

## Backlog

- Stage 2's scale check names `a_hundred_thousand_tasks_in_flight_at_once_all_finish_on_one_core`;
  `crates/nvs-host/src/scheduler.rs:1987` holds the near miss `a_hundred_thousand_tasks_are_in_flight_on_one_core`
  — read what it asserts before writing a second one (`docs/agent/loop-goal.toml:9727`).
- Stage 4, the `spawn` trace event — `crates/nvs-runtime/src/ctx/trace.rs`, `docs/agent/loop-goal.md:107`.
- Stage 5, the one record this goal may open — `docs/agent/loop-goal.md:128`.
- Stages 6 to 9 are unstarted: the three dropped options, the speedup, TSAN, the rulebook.
