# Handoff

## State

**Goal 2, Stage 2: every item is implemented, and what is left of the stage is the acceptance
check's own test names.** The reactor, the parking stream over TCP and Unix, the stack policy,
deadlines on every wait, the blocking pool — and now item 7. `crates/nvs-host/src/watchdog.rs` is
ADR 0106 § 7: one thread for the process, started by the first `Watchdog::register` and joined by
`Watchdog::drop` (it is process-owned, not thread-local, so unlike `BlockingPool` it may join),
holding one entry per registered core. `Timers` publishes its first entry into an `AtomicU64`
(`timer.rs:@publish`), read off the core through `DeadlineView` (`timer.rs:106`) and handed out by
`Reactor::deadline_view` (`reactor.rs:447`). Both modules' docs are the two homes: `timer.rs` §
*Reading a core's earliest deadline from another thread* for why the mechanism is free, `watchdog.rs`
for what a reading means and why detection is all that happens.

**Registration is at the reactor-install site, not in `Worker::spawn`** — decided here and recorded
in `watchdog.rs` § *Registering a core*. `Worker::spawn` never creates a reactor (its body does), so
it has no view to register; the two lines a worker adds are in that section.

**67 tests in the crate on Windows.** Nothing new is `#[cfg(unix)]`, so the WSL leg is the same set
plus the Unix-socket ones.

**The acceptance failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still not
a regression** — `Task::all` is Stage 4's, and loop-goal.md § *Stage 2* writes no `Core\Task` member
until this stage is green.

## Next group

**The Stage 2 `cargo-named` check's ten test names, none of which exists** (`loop-goal.toml:1405`
and `:1416`). This is now the only thing between the stage and green, and every behaviour is already
implemented — these are tests over landed work, so several slices fit one session. One file set:
`crates/nvs-host/src/*.rs`'s own `mod tests`. Check each claim against the module before writing:
the block is the specification, and a name is not evidence the behaviour is spelled the way it reads.

- [ ] **The stream's four**, in `net.rs:614`: `a_socket_read_that_would_block_parks_its_coroutine`,
      `a_parked_coroutine_resumes_when_its_descriptor_is_ready`,
      `a_core_serves_another_task_while_one_is_parked`, `a_stream_satisfies_std_io_read_and_write`.
      ADR 0115 §§ 2–3.
- [ ] **The scheduler's three**, in `scheduler.rs:509`:
      `one_core_runs_one_scheduler_and_a_task_never_migrates`,
      `a_refcount_is_still_non_atomic_under_the_scheduler`, `many_tasks_can_be_created_and_driven`.
- [ ] **The two one-liners**: `a_timer_and_a_deadline_are_the_same_wheel` (`timer.rs:283`, item 5)
      and `a_blocking_call_goes_to_a_pool_bounded_at_twice_the_core_count` (`blocking.rs:374`, where
      `the_bound_is_twice_the_core_count` at `blocking.rs:389` asserts the bound but not the *call*
      going to the pool). ADR 0106 § 6.
- [ ] **`a_rustls_session_streams_over_it_unmodified`** (`net.rs:614`) — the one with a cost:
      `crates/nvs-host/Cargo.toml` has no `[dev-dependencies]` at all, so this adds `rustls` as one.
      Goal 4 owns the client; this asserts only that a session composes over `NvsStream`.

## Backlog

- Admission control subscribes to `Stall` rather than growing a second detector — `watchdog.rs`
  § *Report and shed, never kill*.
- `DEFAULT_MARGIN`/`DEFAULT_INTERVAL` become `[limits]` keys in goal 3 — same section.
- Stage 3 items 8–10 (`spawn`/`await` lowering, the task tree, cancellation) — loop-goal.md.
- M4's residue is the 1000-case corpus count — docs/implementation-plan.md.
