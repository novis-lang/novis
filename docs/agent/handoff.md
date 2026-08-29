# Handoff

## State

**Stage 3 is complete: all five `nvs-host` names its `cargo-named` check asks for are green**
(`loop-goal.toml:1465`). 93 tests in the crate on Windows, up from 83.

Item 11's bounded channel is `crates/nvs-host/src/channel.rs` — a `!Send`, per-core queue whose
`send` suspends at the bound rather than growing and whose `recv` suspends on empty. Its module
doc is that design's only home (no ADR slot was free, per the goal's standing decisions), and the
three things to know before touching it are there: the bound is backpressure and not a tuning
knob; a wake goes to exactly one waiter because one `send` frees exactly one slot; and a waiter
registration is an RAII guard on the waiting task's own stack, so a cancelled task deregisters
itself under the forced unwind rather than leaving a wake that can never be delivered.

**The scheduler grew one public type for it: `nvs_host::Wake`** — the route by which a running task
wakes a peer, since it cannot reach the `&mut Scheduler` that is resuming it. It queues a `TaskId`
on the task tree and `Scheduler::run` drains it, at the top of a turn as well as after each resume.
`scheduler.rs`'s module doc § *The task tree* owns it; the new playbook bullet owns the two halves
of it that are easy to get wrong.

**What is left of item 11 is its language surface.** `Core\Task\Channel` has no row in
`nvs_stdlib::registry` — nothing under `crates/nvs-stdlib` or `crates/nvs-types` mentions `Task` at
all yet — so the whole `Core\Task` class is Stage 4's, starting with item 12 below.

**The acceptance failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still
not a regression**, and the next group is what closes it.

**The orientation pack was missing nothing this item needed.** The gap the last handoff named is
still open: `[context] adrs` carries ADR 0072 §§ 4 and 5 only, and §§ 1 and 2 are what item 12 and
item 13 are specified by — add both.

## Next group

**Stage 4 item 12's `Core\Task::all`, the compile-time half first.** File set:
`crates/nvs-stdlib/src/registry.rs` (`CLASSES:798`, `CoreTy:136`), a new
`crates/nvs-stdlib/src/task.rs` for the class, `crates/nvs-types/src/core_lib.rs:28` (the one place
that reads the registry into the type table, `install:35`), and
`crates/nvs-types/tests/core_members.rs`. Nothing named `Task` exists in either crate, so the first
slice is a new class and not an added row.

- [ ] **`a_task_all_binds_each_fields_own_type`** (`loop-goal.toml:1477`) — `Core\Task::all` over a
      shape literal of `fn` literals, each field keeping its own type rather than collapsing to
      `mixed`. ADR 0072 § 1. Needs the `Core\Task` class to exist first
      (`crates/nvs-stdlib/src/registry.rs:798`).
- [ ] **`a_task_all_field_holding_a_callable_variable_is_a_compile_error`**
      (`loop-goal.toml:1476`) — the refusal that makes the heterogeneous typing possible at all, and
      the item's own note says it is the easiest thing to leave out. It is a `nvs-types` checking
      diagnostic, so it needs a code from the `E07xx` band (next free `E0773`).
- [ ] **`a_limit_and_deadline_options_shape_is_the_only_spelling`** (`loop-goal.toml:1480`) — ADR
      0072 § 3, and the standing decisions already settle what it must *not* accept: there is no
      `race` and no `timeout` wrapper.

## Backlog

- `Core\Task\Channel`, the language surface over `crates/nvs-host/src/channel.rs` — loop-goal.md
  item 11's remaining half.
- `Core\Task::map`, subject-first — loop-goal.md item 13, ADR 0072 § 2.
- `Core\Task::afterResponse` and the re-parenting onto the request tree — loop-goal.md item 15, ADR
  0072 § 6; `scheduler.rs`'s module doc names it as the one shape that outlives its spawning call.
- `[context] adrs` in `docs/agent/loop-goal.toml` needs ADR 0072 §§ 1 and 2 added.
- The `nvs-ir` half of Stage 3, `a_spawn_lowers_to_a_task_on_the_current_core` and
  `an_await_suspends_until_its_task_completes` — `loop-goal.toml:1456`.
- M4's 1000-case corpus count, met as the conformance suite grows — docs/implementation-plan.md.
