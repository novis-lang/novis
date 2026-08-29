# Handoff

## State

**Goal 2, Stage 2. Items 4 and 5 are on disk: the reactor, the parking stream, the task's stack
policy, the parking `connect`, and deadlines.** `crates/nvs-host/src/timer.rs` is the one home for
the timer design — one deadline per task (a task is one stack, so arming twice replaces), kept exact
so `Reactor::retire` can drop a finished task's deadline the way it drops its registrations, and
enforced as the timeout of the poll `Reactor::turn` was about to make rather than by a clock of its
own. `sleep` and `park_until` are the two views of it; off a core the thread sleeps, exactly as
`net.rs`'s reads block there. `NvsTcp::connect` (`net.rs:121`) is ADR 0115 § 3's order applied to a
handshake, and `finish_connecting` (`net.rs:155`) is the one home for *why* its two questions are
`SO_ERROR` and a zero-length write and not `mio`'s own `peer_addr` — see the playbook bullet. 44
tests in the crate, green on Windows and under WSL.

`crates/nvs-host/src/stack.rs` is unchanged and still ADR 0115 § 4's only home. **The acceptance
failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still not a regression** —
loop-goal.md § *Stage 2* writes no `Core\Task` member until this stage is green, and `Task::all` is
Stage 4's. `nvs-host` is still outside `nvs-cli`'s dependency graph, so `THIRD-PARTY-LICENSES.txt`
still needs no regeneration.

## Next group

**Joining the two mechanisms this session landed, then the last of § 3's surface.** One file set:
`crates/nvs-host/src/net.rs`, `crates/nvs-host/src/timer.rs` and `crates/nvs-host/src/reactor.rs`.
They are one group because the first slice is what makes the second and third worth having — every
wait in `net.rs` currently has no upper bound at all, which is ADR 0074 § 5's "no spelling for an
unbounded wait" read from the other end.

- [ ] **A wait that a deadline can end.** `net.rs:197`'s `wait_until_ready` gains a deadline: arm
      `Timers` (`timer.rs:77`) for the running task beside the reactor registration, and on the way
      back out of `suspend_current` decide between "retry the syscall" and `io::ErrorKind::TimedOut`
      by re-reading the clock — a wake is still a hint, so the clock and not the wake is what
      decides. `block_until_ready` (`net.rs:259`) takes the same bound as its poll timeout. ADR 0074
      § 5 and ADR 0072 § 3 both resolve to this one mechanism; nothing here needs a `Core` spelling
      yet.
- [ ] **`NvsTcp::connect` under a connect timeout.** ADR 0074 § 5 names `connect_timeout`
      separately from `deadline`, and `finish_connecting` (`net.rs:155`) is one loop over the call
      above, so this is the argument threaded through and a test that a connect to a black hole
      (`10.255.255.1:80` is the usual unroutable stand-in) returns `TimedOut` in the time asked for
      rather than in the platform's own minutes.
- [ ] **The Unix-domain sibling of `NvsTcp`.** loop-goal.md § *Stage 2* item 4 names it in the same
      breath as the TCP stream. **Decide the platform question first and record it in `net.rs`'s
      module doc**: `mio::net::UnixStream` is `#[cfg(unix)]`, so this is either a cfg-gated type or
      a named non-goal — the parking half is `NvsTcp`'s verbatim and only the constructor differs.

## Backlog

- **The blocking pool**, bounded at twice the core count — loop-goal.md § *Stage 2* item 6, ADR 0106
  § 6. New module; no overlap with the file set above.
- **The watchdog** reading each worker's in-flight deadline — item 7, ADR 0106 § 7. Wants the
  deadline mechanism above to exist first.
- **`Reactor::turn` retries a bounded wait**, which spins if readiness keeps arriving for a task
  that no longer parks; bounded by the deadline and documented at the site. Revisit if a benchmark
  ever sees it.
- Stage 3 (`spawn`/`await`, the task tree) is what this stage unblocks — `docs/agent/loop-goal.md`.
