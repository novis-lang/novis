# Handoff

## State

**Goal 55 — the scheduler's claims are proven at the scale M5 promised them — has just started; nothing of it has landed yet.** Goal `m4-refusals`'s whole list is this goal's Stage 1 floor.

Settled before the first session, in the goal's § *Standing decisions*: `on: worker`'s meaning is ADR
0006's and only its mechanism is new (stage 5's one record); `Core\Task::map` stays on its core and the
speedup is proven through worker-placed children; `Core\Server::isDraining` answers in a child; the
100k test is release-only and demands the Linux map count rather than shrinking; `race` is not built.
None of these is re-decided.

## Next group

**Stage 2: the scale and the deadlock** — one file set: `crates/nvs-host/src/scheduler.rs`,
`crates/nvs-host/src/group.rs`, `crates/nvs-host/src/channel.rs`, `crates/nvs-host/src/stack.rs`.

- [ ] **A hundred thousand tasks in flight on one core** — beside
      `crates/nvs-host/src/scheduler.rs:1934`, marked release-only as
      `crates/nvs-cli/src/serve.rs:1812-1817` is; on Linux it reads `/proc/sys/vm/max_map_count` and
      fails naming the sysctl. `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`.
- [ ] **The map-count fact** — `crates/nvs-host/src/stack.rs:15-16`'s module doc, and the operator's
      line in `docs/setup.md`.
- [ ] **A deliberate deadlock ended by the deadline, and by cancelling its awaiter** — beside
      `crates/nvs-host/src/group.rs:771`, over `crates/nvs-host/src/channel.rs`'s channel, plus
      `tests/conformance/task/a-deliberate-deadlock-is-ended-by-the-deadline-and-leaves-no-child-running.nvst`.
      `rule:concurrency/nothing-is-still-running-when-a-call-returns`.

## Backlog

- Stage 3, the isolate proofs — `crates/nvs-host/src/isolate.rs`, `crates/nvs-host/tests/limits.rs:769`,
  `crates/nvs-stdlib/src/session.rs:743-771`; its own file set.
- Stage 4, the `spawn` event — `crates/nvs-runtime/src/ctx/trace.rs:155`,
  `crates/nvs-host/src/isolate.rs:401`, `crates/nvs-host/src/group.rs:376`; shares `isolate.rs` with
  stage 3.
- Stages 5–6, the record and the three options — `crates/nvs-types/src/expr/isolate.rs:143`,
  `crates/nvs-ir/src/lower/expr.rs:3188`, `crates/nvs-stdlib/src/script.rs:619`,
  `crates/nvs-host/src/group.rs:197`.
- Stage 7, the speedup guard — `benches/abi-probe/tests/perf_guards.rs:486`; needs stage 6.
- Stage 8, TSAN — `.github/workflows/ci.yml:460-574`, `tools/tsan.sh`; the worker cores of stage 6 are
  in its scope, so it runs after them.
- When this goal's last check goes green the driver takes goal `m4b-editor`.
