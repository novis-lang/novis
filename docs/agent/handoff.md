# Handoff

## State

**Goal 2, Stage 2 is complete — implementation *and* acceptance.** Every `nvs-host` test the Stage 2
`cargo-named` checks name (`loop-goal.toml:1400` and `:1411`) is on disk and green: 77 tests in the
crate on Windows, the same set plus the `#[cfg(unix)]` socket ones under WSL.

**New this session, and it is the whole session:** `a_rustls_session_streams_over_it_unmodified`
(`net.rs:1215`). A real TLS 1.3 session — handshake, a self-signed `rcgen` certificate the client
verifies against a root store, and an application record — runs over `NvsTcp` through
`rustls::Stream`, with nothing in the client half naming this crate except the line that constructs
the socket. The peer answers late twice: before its handshake flight, and again with the answer's
last byte held back, so one park lands strictly *inside* an application record. That second one is
what the test is for — `rustls` treats a socket `WouldBlock` as an error to propagate rather than a
park to retry, and a short read corrupts a length-prefixed frame instead of merely delaying it.

**`rustls` and `rcgen` are `[dev-dependencies]` of `nvs-host` alone**, on the `ring` provider rather
than the C `aws-lc-rs` default. The workspace `Cargo.toml`'s comment above them is the one home of
why this is where ground-rules.md's pure-Rust default is spent rather than met: `ring` is not pure
Rust, nothing it compiles reaches a request or a release artifact, and `cargo tree -e normal` never
reaches it. No attribution is owed — see the new playbook bullet for which of the two supply-chain
gates cares and which does not.

**Stage 3 has no scaffolding yet**, and that is why this session stopped at one slice rather than
taking the group's second: `scheduler.rs` contains no parent link, no child list and no cancellation
of any kind, so `no_call_returns_with_a_child_still_running` is a from-scratch design slice over a
file this session never had to open, not a test over landed work.

**The acceptance failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still not
a regression** — `Task::all` is Stage 4's, and loop-goal.md § *Stage 2* writes no `Core\Task` member.

**The orientation pack was missing nothing this item needed.** For the next group it will be:
`[context] adrs` carries ADR 0072 §§ 4 and 5 only, and the task tree below needs § 1 (what a task
*is*) and § 6 (`afterResponse`, which decides whether a child can outlive its parent) as well.

## Next group

**Stage 3's task tree, in `nvs-host`.** File set: `crates/nvs-host/src/scheduler.rs` and
`crates/nvs-host/src/lib.rs`. Anchors, all in `scheduler.rs`: `Waiting:69`, `TaskId:98`,
`Finished:121`, `RunReport:138`, `Scheduler:168`, `spawn:273`, `wake:314`, `run:331`,
`take_finished:356`, `suspend:414`, `current_task:460`, `suspend_current:474`.

- [ ] **The tree itself** — a parent `TaskId` on each task and the children under it, taken from the
      spawning task rather than passed in, plus a cancel flag a child observes at its next safepoint
      (`suspend:414` is the safepoint that exists). ADR 0072 § 4. Decide-and-record in
      `scheduler.rs`'s module doc; no ADR slot is free for it.
- [ ] **`no_call_returns_with_a_child_still_running`** (`loop-goal.toml:1465`) — ADR 0072 § 4's
      table, at the scheduler level: a join that returns with a child still in the run queue is the
      failure, so assert over `RunReport`/`take_finished` rather than over a sleep.
- [ ] **`a_task_tree_dies_with_its_parent_and_leaves_no_orphan`** (`loop-goal.toml:1466`) — the
      parent's death path; `parked_count:374` and `ready_count:380` are what make "no orphan"
      countable.
- [ ] **`a_cancelled_task_runs_no_catch_and_no_cleanup_block`** (`loop-goal.toml:1468`) — ADR 0072
      § 5. Native teardown runs, user code does not; the intuitive implementation is the wrong one.

## Backlog

- Stage 3's lowering half: `a_spawn_lowers_to_a_task_on_the_current_core` and
  `an_await_suspends_until_its_task_completes`, in `nvs-ir` (`loop-goal.toml:1456`).
- `Core\Task::all` for `examples/tasks.nvs` — Stage 4's, and the standing acceptance failure until
  then (`docs/agent/loop-goal.md` § *Stage 4*).
- ADR 0106 § 7's shed half: the watchdog fires ADR 0020 § 4's floor and nothing else; shedding is
  admission control's and is in no crate yet (`crates/nvs-host/src/watchdog.rs` module doc).
- M4's residue: the 1000-case corpus count, met as the suite grows (`docs/plan/m4.md`).
