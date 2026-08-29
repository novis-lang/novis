# Handoff

## State

**Goal 2, Stage 2. The reactor, the parking stream and the task's stack policy are all on disk.**
`crates/nvs-host/src/stack.rs` is ADR 0115 § 4's only home: `TASK_STACK_SIZE` is 1 MiB of *reserved*
address space per task (the module doc has the per-platform reason a reserved page is never a
resident one), `MAX_POOLED_STACKS` is a compiled-in stand-in for the worker's in-flight cap that
goal 3's `[limits]` replaces, and `StackPool` lives on the `Scheduler` because that is the only type
that sees both ends of a task. `Scheduler::spawn` takes a stack and `Scheduler::run` gives it back
through `Coroutine::into_stack`, so N sequential requests cost one reservation rather than N;
`Scheduler::pooled_stacks` is what a test observes that by. `stack::bounds` is the one place the
`Ctx::arm_stack_limit` pair is computed, and its doc owns why the ceiling is `TASK_STACK_SIZE` and
never `base - limit()` — `corosensei`'s `limit()` includes the guard pages, so arming from it puts
the hard floor *below* the guard and the overflow faults before the limit ever compares. 36 tests in
the crate, all green.

**The acceptance failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still not
a regression** — loop-goal.md § *Stage 2* writes no `Core\Task` member until that stage is green, and
`Task::all` is Stage 4's. `nvs-host` is still outside `nvs-cli`'s dependency graph, so
`THIRD-PARTY-LICENSES.txt` still needs no regeneration.

## Next group

**Everything left on the reactor's own surface.** One file set: `crates/nvs-host/src/net.rs` and
`crates/nvs-host/src/reactor.rs`, plus `crates/nvs-host/src/lib.rs`'s § *What is here, and what is
not*, whose "Still outstanding" paragraph names exactly what these three close. They are one group
because each is a `Reactor` method plus its consumer, and the second and third both need the timeout
argument the first one leaves alone.

- [ ] **A parking `connect` for `NvsTcp`.** ADR 0115 § 3. `net.rs:89`'s `from_std` is the
      constructor to grow it beside: a non-blocking `connect` returns `WouldBlock`/`InProgress`, so
      the shape is one `Interest::WRITABLE` registration through `reactor.rs:173`, one park, then a
      `SO_ERROR` read to turn readiness into the `io::Result` the caller expects — readiness alone
      does not mean the connection succeeded. Off a core it waits on its own poll, exactly as
      `net.rs`'s reads already do; that refusal path is written and just needs the same treatment.
- [ ] **Timers on the same reactor.** loop-goal.md § *Stage 2* item 5, and ADR 0072 § 3's
      `{limit, deadline}` is why. `reactor.rs:268`'s `poll` already takes an
      `Option<Duration>` and `reactor.rs:306`'s `turn` already computes one, so the wheel goes
      between them: the timeout handed to `poll` becomes "the earliest deadline, or none", and an
      expired entry wakes its `TaskId` through the same `Scheduler::wake` readiness uses. A sleep and
      a deadline are one mechanism seen twice — do not write two.
- [ ] **The Unix-socket sibling of `NvsTcp`.** ADR 0115 § 3 names it in the same breath as the TCP
      stream. `mio::net::UnixStream` is `cfg(unix)`, so this is a `#[cfg(unix)]` module beside
      `net.rs:62`'s `NvsTcp` with the identical try-then-park order; the point of doing it here is
      that it proves the parking shape is the socket's and not TCP's.

## Backlog

- The blocking pool, bounded at twice the core count — loop-goal.md § *Stage 2* item 6, ADR 0106 § 6.
- The watchdog reading each worker's in-flight deadline — loop-goal.md § *Stage 2* item 7.
- Admission arithmetic counting `TASK_STACK_SIZE` per admitted request — ADR 0106 § 7, goal 3.
- `MAX_POOLED_STACKS` and the stack width become `[limits]` configuration — goal 3, ADR 0115 § 4's
  closing line.
- M4's residue is the 1000-case corpus count — docs/implementation-plan.md, `Open now`.
