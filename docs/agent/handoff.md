# Handoff

## State

**Goal `m5-proofs` (M5). Stage 2 is closed, and stage 4 is half landed.**

Stage 2 needed no new assertions. Both deadlock tests and the `.nvst` case were already on disk
from an earlier session under names the check does not spell; the two `cargo-named` tests now carry
the check's own spelling in `crates/nvs-host/src/group.rs` and their assertions are untouched, and
`tests/conformance/task/a-deliberate-deadlock-is-ended-by-the-deadline-and-leaves-no-child-running.nvst`
passes as it stood.

Stage 4's runtime half is built: `Ctx::open_spawn` and `Ctx::close_spawn`
(`crates/nvs-runtime/src/ctx/trace.rs`) file one `TraceKind::Spawn` event per spawn under `TRACE` or
`PROFILE`, carrying the form's spelling, both timestamps on a process-wide monotonic epoch, the
parent-observed wall and the split against the child's own. `run_as_children` emits one per
`Core\Task` child and closes them all where the call returns. Two of that stage's three `-p
nvs-host` tests are green; the third is the isolate's, and `Completion` has nowhere yet to carry a
child's own wall time, which is what the next group builds.

Nothing is blocked.

## Next group

**Stage 4: the `spawn` trace event, the isolate half** — one file set:
`crates/nvs-host/src/isolate.rs` and `crates/nvs-runtime/src/host.rs`.

- [ ] **A child isolate reports its own wall time** — `Completion` at
      `crates/nvs-runtime/src/host.rs:268` gains the field the split is computed against, filled
      where the child's body ends, at `crates/nvs-host/src/isolate.rs:630`'s join and the placed
      form's at `crates/nvs-host/src/isolate.rs:674`. Timed only where the parent opened an event,
      as `crates/nvs-host/src/group.rs`'s `Child::timed` does it.
      `rule:observability/spawn-is-its-own-event`.
- [ ] **A `spawn script` opens its event where the child starts and closes it at the join** — the
      test name `a_traced_spawn_script_records_one_spawn_event_closed_at_its_join` verbatim, in
      `crates/nvs-host/src/isolate.rs`'s own test module, over `Isolate::start` at
      `crates/nvs-host/src/isolate.rs:423` and the joins at `crates/nvs-host/src/isolate.rs:630`.
      `rule:observability/spawn-is-its-own-event`.
- [ ] **The untraced gate covers an isolate as well as a task child** — the same isolate test
      asserts that with both bits off `crates/nvs-host/src/isolate.rs:423` records nothing and reads
      no clock; `crates/nvs-host/src/group.rs:1020` is that assertion for a `Core\Task` child.
      `rule:testing/debug-probes`.

## Backlog

- Stage 5: the record, and the concurrency rule its check names by id — the ruling it writes down is
  already settled in `docs/agent/loop-goal.md` § *Standing decisions*.
- Stage 6: `on:`, `limits:` and `grants:` — two `-p nvs-types` tests, three `-p nvs-host` tests and
  three `.nvst` cases, listed in `docs/agent/loop-goal.toml`'s `stage = "6 the options"`.
- A group with no task beneath it (`run_here`) records no spawn event, because nothing is started
  there — `crates/nvs-host/src/group.rs`'s module doc if it ever needs saying.
