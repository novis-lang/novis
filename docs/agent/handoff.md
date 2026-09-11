# Handoff

## State

**Goal `finish-response` — stage 4's member is on disk and runs end to end under `nvs run`.**
`Core\Script::finish()` is a registered row (`crates/nvs-stdlib/src/script.rs:164`) whose symbol no
call site ever calls: `Lowering::lower_finish` (`crates/nvs-ir/src/lower/exception.rs:68`) seals the
block with a `Terminator::Throw` of a `Core\Script\Finished` instance, recognised at
`crates/nvs-ir/src/lower/expr.rs:3505` by `nvs_types::CORE_SCRIPT_FINISH`. Every `finally` between
the call and the root runs, no `catch` arm admits it, the report names a fourth
`ExitReason::Finish` with status `0` and no error, `afterResponse` work runs and the process exits
`0`. `examples/finish.nvs` prints the acceptance's five lines exactly.

**Two spellings of the marker's name, held together by a test.** `nvs-stdlib` depends on
`nvs-runtime` and on no part of the compiler, so it carries `FINISH_MARKER_NAME`
(`crates/nvs-stdlib/src/script.rs:392`) and `nvs_types`'s
`the_marker_the_runtime_classifies_by_is_the_one_the_compiler_declares` pins it to
`nvs_hir::errors::FINISH_MARKER`. `script::is_finish` (`:560`) takes the *name*, not the object, so
a host can ask while the exception is still pending and leave the real one for the ladder.

**Only `nvs run` classifies the ending.** `crates/nvs-cli/src/main.rs` asks before it takes anything
and reports `Ok(())`; the served path still reads a finish as a failure, which is the next group.

## Next group

**Stage 4: the served path and the two refusals** — one file set:
`crates/nvs-host/src/isolate.rs`, `crates/nvs-runtime/src/ctx/hooks.rs`,
`crates/nvs-stdlib/src/script.rs`. Tag it stage 4 so `[context.stage.4]`'s overlay applies.

- [ ] **Make a finished request an ordinary end on the served path.**
      `crates/nvs-host/src/isolate.rs:899` sets `failed` from `pending().is_some()`, so the marker
      currently gives `Completion::ok == false` and the deferred gate at `:884` refuses. Ask
      `nvs_stdlib::script::is_finish` off `Ctx::pending_class` before that line, take the marker so
      nothing downstream sees a pending failure, and leave both gates unedited.
      `rule:concurrency/after-response-outlives-the-connection`.
- [ ] **Refuse `finish` inside an exit hook, beside the `EXITED` arm.**
      `abandon_exit_hook` (`crates/nvs-runtime/src/ctx/hooks.rs:281`) already turns an `exit` into a
      `RuntimeError`; a finish owes the same and for the same reason. **The wrinkle to decide first:**
      `nvs-runtime` cannot name `nvs-stdlib`, so either the marker's name moves down into
      `nvs-runtime` with `script::FINISH_MARKER_NAME` delegating to it, or the arm asks
      `Ctx::pending_class` against a constant declared there. Prefer the move — one spelling below
      both readers. `rule:observability/a-hook-observes-and-never-steers`.
- [ ] **Write stage 4's three `-p nvs-stdlib` acceptance tests**, whose claims are now all
      reachable: `the_exit_report_names_the_finish_ending_with_a_zero_status_and_no_error` drives
      `run_exit_hooks` (`crates/nvs-stdlib/src/script.rs:485`) with a marker `Thrown`, built the way
      `crates/nvs-runtime/src/ctx/error.rs:687`'s test builds one;
      `finish_inside_an_exit_hook_throws_and_the_drain_continues` and
      `finish_in_a_task_child_ends_that_child_and_not_the_request` follow the two items above.

## Backlog

- The `catch`-arm refusal naming the marker class is unwritten — `tests/conformance/reject/a-catch-arm-naming-the-finish-marker-is-refused.nvst`, stage 3's `E0780` sibling (`docs/agent/loop-goal.md` § stage 3).
- Stage 5's record and rulebook edits, including the fourth row on `rule:observability/three-endings-fire-the-exit-queue` (`docs/agent/loop-goal.md` § stage 5).
- A `match` over `Core\Script\ExitReason` with three arms still compiles after a fourth case landed; whether enum `match` owes exhaustiveness is its own question (`docs/rules/types/`).
