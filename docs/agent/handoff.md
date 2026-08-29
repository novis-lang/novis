# Handoff

## State

**Goal 2, Stage 2. Item 4 is closed.** The reactor, the parking stream, the task's stack policy, the
parking `connect`, deadlines on every wait — and now the Unix-domain sibling, which is the *same*
type: `crates/nvs-host/src/net.rs` holds `NvsStream<S>`, generic over whatever the reactor can
register, with `NvsTcp` and `NvsUnix` as type aliases over it. That module's doc § *One type over
the source, not one type per socket family* is the one home for why a generic and not a second
concrete type, and for what it costs; § *Every wait is bounded by a clock, not by a wake* is still
the one home for the deadline. What stays per family is only the address — `connect` is the socket
constructor plus a call to the shared `connected`, `peer_addr` returns two different types, and
`finish_connecting` is shared through the private `Connecting` trait.

**53 tests in the crate under WSL, 49 on Windows, which compiles none of the `#[cfg(unix)]` half** —
that gap is now a playbook bullet under *Running things*, and it is the reason a Unix-only slice is
not finished when `verify.py` is green here.

**What is left of Stage 2 is items 6 and 7**, the blocking pool and the watchdog, and they are
independent of each other and of everything above.

**The acceptance failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still
not a regression** — loop-goal.md § *Stage 2* writes no `Core\Task` member until this stage is
green, and `Task::all` is Stage 4's. `nvs-host` is still outside `nvs-cli`'s dependency graph, so
`THIRD-PARTY-LICENSES.txt` still needs no regeneration.

## Next group

**The blocking pool, loop-goal.md § *Stage 2* item 6 and ADR 0106 § 6.** One file set:
`crates/nvs-host/src/reactor.rs`, `crates/nvs-host/src/scheduler.rs`, a new
`crates/nvs-host/src/blocking.rs` and `crates/nvs-host/src/lib.rs:62-80` (the module list and the
re-exports). Read `reactor.rs:332` (`turn`) first — it is the poll the pool has to be able to
interrupt.

- [ ] **A cross-thread wake on the reactor, because nothing here has one yet.** `Reactor::turn`
      (`reactor.rs:332`) blocks in `Poll::poll`; a pool thread finishing a call off the core has no
      way to say so. `mio::Waker` is the mechanism and it is one more token that is not a `TaskId`.
      Anchors: `reactor.rs:332`, `reactor.rs:454` (`with_current`), `scheduler.rs:314`
      (`Scheduler::wake`), `scheduler.rs:69` (`Waiting`).
- [ ] **The pool itself, bounded at twice the core count** — ADR 0106 § 6 states that bound and why
      it is stated, and `crates/nvs-host/src/affinity.rs`'s `cpus` is where the count comes from. A
      submitted call parks the task exactly as a socket does and the wake above is what ends it.
- [ ] **Verify it under WSL as well as here**, per the new playbook bullet: the pool is not
      `#[cfg(unix)]`, but everything it will carry first (filesystem calls, name resolution) is
      where the two platforms diverge.

## Backlog

- Item 7, the watchdog — loop-goal.md § *Stage 2*; independent of item 6.
- `Core\Task::all` and the rest of the member surface — Stage 4, ADR 0072.
- `[context] playbook` in `docs/agent/loop-goal.toml` has no selector for the new *Running things*
  WSL bullet; a Unix-touching slice will want it printed.
- The 1000-case corpus count is M4's residue — `docs/implementation-plan.md`.
