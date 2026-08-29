# Handoff

## State

**Goal 2, Stage 2. The reactor is on disk and a parked task is woken by real readiness.**
`crates/nvs-host/src/reactor.rs` is ADR 0115 §§ 1–2: `mio` (`os-poll` + `net`) keyed by `TaskId` —
the token *is* the raw id, so there is no second table — `register` before `suspend`, a wake that is
a hint, `retire` on `Finished`, and `run_until_idle` (`reactor.rs:306`) polling with a zero timeout
while the run queue has work and blocking only when something parked can still be woken. 21 tests in
the crate, all green. The module doc owns the reasoning, including the half of a registration the
kernel owns versus the half `retire` frees.

**A latent path to `abort()` was closed on the way, and it was not the reactor's.** Dropping a
suspended `corosensei` coroutine unwinds its stack, and `nvs_runtime::run_task`'s containment
boundary swallowed the unwind marker, which corosensei answers by aborting the process. That is an
ordinary worker shutdown with a request still parked, so it is ADR 0106's "no path reaches
`abort()`". `nvs_runtime::Teardown` (`abi.rs`, below `run_task`) is a thread-local window in which
`run_task` re-raises rather than contains; `Drop for Scheduler` holds it across the drop of the
ready and parked sets. No script code runs in the window — what unwinds is native `Drop`, which
ADR 0072 § 5 already requires — and the playbook bullet has the recognition test.

**`mio` is a new dependency** (+`windows-sys` on Windows, both MIT/Apache-2.0); the workspace
`Cargo.toml` comment above it answers ADR 0051 § 4's two questions and points at ADR 0115 § 1 for
why a poller is not the `tokio` the standing decisions forbid. `THIRD-PARTY-LICENSES.txt` still
needs no regeneration — `nvs-host` is not in `nvs-cli`'s graph yet, so the session that wires it in
runs `gen-attribution.py`.

**The acceptance check on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still not
a regression** — loop-goal.md § *Stage 2* writes no `Core\Task` member until that stage is green,
and `Task::all` is Stage 4's.

## Next group

**The stream, and the design question in front of it.** One file set: `crates/nvs-host/src/` (a new
`net.rs`, plus `lib.rs:45` where the modules are declared and `lib.rs:49` where they are
re-exported), and `crates/nvs-runtime/src/ctx.rs:829` for the third slice only.

- [ ] **Decide how a task reaches its core's reactor, and record it.** `NvsTcp::read` is a plain
      `std::io::Read` with no `Ctx` and no reactor argument, so the stream has to reach both from
      inside its own `&mut self`. The yielder already travels in `Ctx` as an opaque `*const ()`
      (`ctx.rs:@yielder`, `scheduler.rs:361`) and the reactor is `!Send` and one-per-worker, so the
      two candidates are a second opaque pointer in `Ctx` set by `Scheduler::run`, or a thread-local
      installed by `Worker::spawn`. Whichever wins, `run_until_idle` (`reactor.rs:306`) currently
      takes `&mut Reactor` across `sched.run()`, which no task can borrow through — that signature
      changes with this decision. Pre-authorized under the goal's standing decisions: decide it and
      record it in `reactor.rs`'s module doc, do not open an ADR.
- [ ] **`net.rs` — `NvsTcp: std::io::Read + std::io::Write` over the reactor.** ADR 0115 § 3, in
      that order: issue the syscall, return on success, and only on `WouldBlock` register
      (`reactor.rs:133`), `suspend(ctx, Waiting::Parked)` (`scheduler.rs:361`) and loop. The
      registration is kept while the task holds the stream, so a repeat park pays `reregister`
      (`reactor.rs:153`) and not `register`. The cost table in § 3 is why the order is that way
      round; a test that a first read finding buffered bytes never touches the reactor is the one
      that pins it.
- [ ] **Arm the recursion limit from the task's own stack.** ADR 0115 § 4's last bullet closes the
      known gap recorded in `ctx.rs`: a task on a host-allocated stack knows its real bounds, so
      `Ctx::arm_stack_limit` (`ctx.rs:829`) can be called with them instead of an estimate. Needs
      the stack decision (1 MiB reserved, resident-narrow, pooled per worker) to land first, which
      is why it is last.

## Backlog

- The blocking pool for filesystem calls, name resolution and child processes — ADR 0106 § 6,
  bounded at twice the core count.
- Stack policy: 1 MiB reserved, resident-narrow, pooled per worker — ADR 0115 § 4. Today a task
  takes `corosensei`'s default, as `scheduler.rs`'s module doc says.
- `THIRD-PARTY-LICENSES.txt` regeneration, owed by whichever session first puts `nvs-host` in
  `nvs-cli`'s dependency graph.
- `Core\Task::all` / `::map` — Stage 4, and the standing acceptance failure until then
  (`docs/agent/loop-goal.md` § *Stage 2*).
- ADR 0072 § 5 cancellation as a *mechanism*: `Waiting::Parked` plus `Scheduler::wake` is the seam,
  and nothing cancels yet.
