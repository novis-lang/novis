# Handoff

## State

**Goal `m5-proofs` (M5), stage 2 landed whole; stage 1's floor is goal `m4-refusals`'s list, carried.**

On disk from this session: the hundred-thousand-tasks-in-flight test, release-only and passing under
`--release` in about a second; the Linux map-count fact in `crates/nvs-host/src/stack.rs`'s module doc
and the sysctl an operator runs in `docs/setup.md` § *Linux and macOS*; and the deliberate deadlock
proven three ways — ended by the deadline and by cancelling its awaiter in `crates/nvs-host/src/group.rs`,
and as a `.nvst` case a program can read. Stage 0's `stack.rs:15-16` catch-up line is discharged by that
same fact.

A channel has no script-visible waiter count, so *no waiter registered* is proven by
`Scheduler::parked_count` in Rust and by a post-group `send` that wakes nobody in the `.nvst` case.
Nothing is blocked.

## Next group

**Stage 3: the isolate proofs the Verify paragraph names** — one file set:
`crates/nvs-host/src/isolate.rs`, `crates/nvs-host/tests/limits.rs`, `crates/nvs-stdlib/src/session.rs`,
and `.nvst` cases under `tests/conformance/isolate/`.

- [ ] **Request state throws in a child, and `isDraining` does not** — one `.nvst` case spawning a child
      that reads `Core\Request::method()` and starts a session, against the refusal
      `crates/nvs-stdlib/src/request.rs:1690` already builds for a context
      `crates/nvs-host/src/isolate.rs:239` never made answering; plus the one sentence naming
      `isDraining` as process state, per the goal's § *Standing decisions*.
      `rule:security/request-state-throws-in-an-isolate`.
- [ ] **`Core\Session::start` throws where no request arrived** — `crates/nvs-stdlib/src/session.rs:771`
      opens a fresh session instead; it raises `crates/nvs-stdlib/src/request.rs:1712`'s `no_request` **after** the
      configuration questions at `crates/nvs-stdlib/src/session.rs:743`, so the floor cases that call
      `start` with no store configured keep their `RuntimeError`.
      `rule:security/request-state-throws-in-an-isolate`.
- [ ] **A child's memory breach and CPU breach are each `ok = false` with the parent still running** —
      one test each beside the depth breach at `crates/nvs-host/tests/limits.rs:769`.
      `rule:security/isolate-failure-is-a-value`.
- [ ] **A cyclic argument crosses a real spawn** — driven through `Isolate::run` rather than over the
      walk alone (`crates/nvs-runtime/src/graph.rs:1108`), reading the cycle back out of the child's
      answer, beside `crates/nvs-host/tests/limits.rs:769`.
      `rule:security/isolate-values-cross-by-copy`.

## Backlog

- Stage 4, the `spawn` event — `crates/nvs-runtime/src/ctx/trace.rs:155`,
  `crates/nvs-host/src/isolate.rs:401`, `crates/nvs-host/src/group.rs:376`; shares `isolate.rs` with
  stage 3, and rewrites that file's stage-0 sentence at `trace.rs:59-64`.
- Stages 5–6, the record and the three options — `crates/nvs-types/src/expr/isolate.rs:143`,
  `crates/nvs-ir/src/lower/expr.rs:3188`, `crates/nvs-stdlib/src/script.rs:619`,
  `crates/nvs-host/src/group.rs:197`.
- Stage 7, the speedup guard — `benches/abi-probe/tests/perf_guards.rs:486`; needs stage 6.
- Stage 8, TSAN — `.github/workflows/ci.yml:460-574`, `tools/tsan.sh`; the worker cores of stage 6 are
  in its scope, so it runs after them.
- Stage 9, the rulebook — flip stage 5's rule and `observability/spawn-is-its-own-event` to `shipped`.
- When this goal's last check goes green the driver takes goal `m4b-editor`.
