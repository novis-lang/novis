# Handoff

## State

**Goal `finish-response` — stage 2 is green.** Both its checks are pinned in
`crates/nvs-host/tests/deferred.rs`: the exit queue runs before the request's after-response work,
runs after the ladder's tier-2 handler has reported an uncaught throw, and runs once per request that
registered — a second request on one connection drains nothing, which is the seam being the request's
and not the host's.

**The two endings that run no hook are guarded apart, because the host owns only one of them.** A
cancellation is the host's: `finish`'s `if !cancelled` (`crates/nvs-host/src/isolate.rs:968`) means
the seam is never reached, and the case's log carries the request's own entry so an isolate that never
started cannot pass it. A `FATAL` is not: the host hands `Err(FATAL)` over and
`nvs_stdlib::script::run_exit_hooks` returns from it with no report, so that case asserts the ending
handed over. `Ctx::drain_exit_hooks` (`crates/nvs-runtime/src/ctx/wiring.rs:318`) is the home of why
the host decides neither.

**`raise_the_finish_marker` is now a wrapper** over `raise_an_instance_of(ctx, class)`
(`crates/nvs-host/tests/deferred.rs:230`), so a case needing a plain `Throwable` raises one through
the table the marker already needed rather than a second copy of it.

**Stage 4 still owes the two artefacts below.** Which stage is earliest-red is the driver's
acceptance check to say — stage 3's own checks were not looked at this session.

## Next group

**Stage 4: the member, closing** — the two artefacts stage 4's checks still name, each in its own
file, both against behaviour that is already landed.

- [ ] **Write `finish_in_a_task_child_ends_that_child_and_not_the_request`**, the last of stage 4's
      `-p nvs-stdlib` check, beside its two siblings at `crates/nvs-stdlib/src/script.rs:1342`. The
      claim is that `Core\Script::finish()` inside a `Core\Task` child ends *that child* ordinarily
      and leaves the request running — the marker is a throw and reaches the child's own root, so
      what to assert is the child's completion and the request still producing its answer after it.
      `rule:observability/three-endings-fire-the-exit-queue` and
      `rule:concurrency/after-response-outlives-the-connection`.
- [ ] **Write `tests/conformance/reject/a-catch-arm-naming-the-finish-marker-is-refused.nvst`**, the
      one case in stage 4's `nvs-suite` check with no file on disk
      (`docs/agent/loop-goal.toml:8643`). Its runtime twin
      `tests/conformance/core/finish-is-caught-by-no-catch-arm.nvst:1` is the shape to follow, and
      the refusal it pins is the compile-time one stage 3 decided.

## Backlog

- Stage 5 is untouched: the rulebook fragments, the spec rows and the sweep for documents that still
  say three endings — `docs/agent/loop-goal.toml` stage 5 lists them.
- `[context] modules` did not name `crates/nvs-runtime/src/ctx/safepoint.rs` (`Ctx::cancel`),
  `crates/nvs-runtime/src/closure.rs` (the arity trim a stand-in closure relies on) or
  `crates/nvs-host/src/ladder.rs`; each cost a peek, and the driver's sweep will not add them since
  no commit here touches them.
- Carried gaps and the goal's own record are `docs/agent/carried-gaps.md`'s and the goal's.
