# Handoff

## State

**Goal 2, Stage 2. The parking stream is on disk, and the route it reaches its core by is decided.**
`crates/nvs-host/src/net.rs` is ADR 0115 § 3: `NvsTcp` implements nothing but `std::io::Read` and
`std::io::Write`, tries the syscall first, and only on `WouldBlock` registers, parks and loops. The
registration is kept while the task holds the stream, so a repeat park on the same interest costs no
syscall at all and a changed interest costs one `reregister`; `Drop` hands it back for the case where
the stream dies before its task. Off a core — `nvs run`, a unit test — it waits on a poll of its own
rather than returning a `WouldBlock` the caller cannot wait on, and the module doc says why that is
not the standing decision bent. 30 tests in the crate, all green.

**How a task reaches its core's reactor: a thread-local.** `reactor.rs`'s module doc § *How a task
reaches this reactor* is that decision's only home, including what it was chosen over and why (a
second opaque pointer in `Ctx`: a `Read` has no `Ctx` either, and a reactor is per core while a `Ctx`
is per request). `reactor::install` holds one for as long as its guard lives, `reactor::with_current`
borrows it, and **`run_until_idle` no longer takes a `&mut Reactor`** — that borrow, held across
`sched.run()`, is exactly the one a task suspending inside the call could never get through.
`scheduler::current_task` and `scheduler::suspend_current` are the other half: the running task's id
and its yielder with no `Ctx` to travel in, maintained by the task's own stack across a switch
(`RUNNING`'s doc owns why the scheduler cannot do it). The borrow rule that comes with it — **never
across a suspend point** — is § 2 rule 1 read as a borrow, and is in the same module doc.

**The acceptance failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still not
a regression** — loop-goal.md § *Stage 2* writes no `Core\Task` member until that stage is green, and
`Task::all` is Stage 4's. `nvs-host` is still outside `nvs-cli`'s dependency graph, so
`THIRD-PARTY-LICENSES.txt` still needs no regeneration.

## Next group

**The task's stack, and the limit armed from it.** One file set: `crates/nvs-host/src/scheduler.rs`
(`spawn` at `scheduler.rs:246`, and the module doc paragraph at `scheduler.rs:33-38` that currently
says a task takes `corosensei`'s default), plus `crates/nvs-runtime/src/ctx.rs:829` for the second
slice only. They are one group because the second cannot be written without the first: an arming call
needs real bounds, which is what the stack policy produces.

- [ ] **Stack policy: 1 MiB reserved, resident-narrow, pooled per worker.** ADR 0115 § 4. Today
      `Scheduler::spawn` (`scheduler.rs:246`) builds a `Coroutine::new` with `corosensei`'s default
      stack and `scheduler.rs:33-38` says so in as many words; the pool is per worker and therefore
      O(cores) × O(in-flight), which is the ADR 0004 sentence the module doc owes. `corosensei`'s
      `DefaultStack`/`StackPointer` surface is what a pooled stack has to satisfy.
- [ ] **Arm the recursion limit from the task's own stack.** ADR 0115 § 4's last bullet, against the
      known gap recorded in `ctx.rs`: `Ctx::arm_stack_limit(&mut self, base, ceiling)`
      (`ctx.rs:829`) is called with an estimate today, and a task on a host-allocated stack knows its
      real bounds. The call site is `Scheduler::spawn`'s coroutine body, beside `ctx.set_yielder`.
- [ ] **A parking `connect` for `NvsTcp`** (`net.rs:@from_std`) — one `WRITABLE` wait plus a
      `take_error` check. Only if the two above leave room; it is a different concern in the same
      crate, and the accept loop is what actually needs it.

## Backlog

- The blocking pool for filesystem calls, name resolution and child processes — ADR 0106 § 6,
  bounded at twice the core count.
- `Worker::spawn` installs no reactor: its body does, one `reactor::install` line. Worth folding into
  `Worker::spawn` once a second caller exists, not before.
- `THIRD-PARTY-LICENSES.txt` regeneration, owed by whichever session first puts `nvs-host` in
  `nvs-cli`'s dependency graph.
- `Core\Task::all` / `::map` — Stage 4, and the standing acceptance failure until then
  (`docs/agent/loop-goal.md` § *Stage 2*).
- ADR 0072 § 5 cancellation as a *mechanism*: `Waiting::Parked` plus `Scheduler::wake` is the seam,
  and nothing cancels yet.
