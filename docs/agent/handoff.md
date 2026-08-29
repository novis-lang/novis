# Handoff

## State

**Goal 2, Stage 2: nine of the acceptance check's ten test names are on disk, and every behaviour
under them was already implemented.** New this session: `a_socket_read_that_would_block_parks_its_coroutine`,
`a_parked_coroutine_resumes_when_its_descriptor_is_ready`, `a_core_serves_another_task_while_one_is_parked`
and `a_stream_satisfies_std_io_read_and_write` (`net.rs`); `one_core_runs_one_scheduler_and_a_task_never_migrates`,
`a_refcount_is_still_non_atomic_under_the_scheduler` and `many_tasks_can_be_created_and_driven`
(`scheduler.rs`); `a_timer_and_a_deadline_are_the_same_wheel` (`timer.rs`);
`a_blocking_call_goes_to_a_pool_bounded_at_twice_the_core_count` (`blocking.rs`).

**One of them found a bug and it is fixed.** `Reactor::turn` (`reactor.rs:583`) returned `0` after any
poll that woke nothing, and `run_until_idle` reads `0` with an empty run queue as *idle* — so with the
blocking pool saturated, a surplus waker poke over an already-drained queue abandoned the still-parked
tasks (11 of 34 finished). The retry condition is now the same condition the blocking state is entered
on: `!timers.is_empty() || remote_waits() > 0`. That function's own comment is the home of why.

**76 tests in the crate on Windows.** Nothing new is `#[cfg(unix)]`, so the WSL leg is the same set plus
the Unix-socket ones. One existing case was flaky and is fixed rather than worked around:
`a_second_wedge_on_a_later_deadline_reports_again` (`watchdog.rs:467`) armed a deadline at an instant
that could equal its `Timers`' publishing base, which `Timers::publish` moves a nanosecond later to keep
it out of the `NOTHING` sentinel's way — so the sweep found it a nanosecond short of overdue. The
playbook bullet is the recognition test.

**The acceptance failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still not a
regression** — `Task::all` is Stage 4's, and loop-goal.md § *Stage 2* writes no `Core\Task` member until
this stage is green.

## Next group

**The tenth name, then Stage 3's task tree.** File set: `crates/nvs-host/src/net.rs`,
`crates/nvs-host/Cargo.toml` and `crates/nvs-host/src/scheduler.rs`.

- [ ] **`a_rustls_session_streams_over_it_unmodified`**, in `net.rs:614`'s `mod tests` — the last name
      in the Stage 2 `cargo-named` check (`loop-goal.toml:1416`) and the only one with a cost: add
      `rustls` and a self-signed certificate source as **dev**-dependencies of `nvs-host`
      (`crates/nvs-host/Cargo.toml:12`), pre-authorized under ADR 0051 § 4 — pure-Rust crypto, so no
      `aws-lc` backend. Shape: the server half on a plain `std::net::TcpStream` thread, the client half
      a `rustls::ClientConnection` driven through `rustls::Stream` over an `NvsTcp` on a core, with the
      peer answering late so the handshake parks mid-record. ADR 0115 § 3.
- [ ] **`no_call_returns_with_a_child_still_running`** and
      **`a_task_tree_dies_with_its_parent_and_leaves_no_orphan`** — Stage 3 item 9, ADR 0072 § 4. The
      parent/child edge has nowhere to live yet: `Scheduler` (`scheduler.rs:168`) holds `ready`,
      `parked` and `finished` and no tree, and `Scheduler::spawn` (`scheduler.rs:273`) takes a
      `TaskRoot` and no parent. Decide where the edge lives before writing either test.
- [ ] **`a_cancelled_task_runs_no_catch_and_no_cleanup_block`** and
      **`a_cancelled_tasks_arena_is_released`** — same file, ADR 0072 § 5, once the tree exists.
      Cancellation runs no user code; the standing decisions forbid softening that for a fixture that
      looks like it wants a `finally`.

## Backlog

- Stage 3 item 8: `spawn`/`await` lowering, `-p nvs-ir` (`loop-goal.toml:1453`) — the two names there
  are `nvs-ir`'s, not this crate's.
- Stage 3 item 11: `a_bounded_channel_send_suspends_rather_than_growing` (`loop-goal.md` § *Stage 3*).
- M4 residue: the 1000-case conformance corpus, met as the suite grows (`docs/implementation-plan.md`).
- `orient.py`'s `[context]` manifest was complete for this session; nothing needed outside it.
