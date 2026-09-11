# Handoff

## State

**Goal `finish-response` — stage 4's member now ends a request ordinarily on both hosts.**
`crates/nvs-host/src/isolate.rs:905` asks `nvs_runtime::is_finish` off `Ctx::pending_class` before
anything is taken, takes the marker into a binding of its own so every gate below still reads
`thrown` to mean *this isolate failed*, and hands it to the exit-queue seam
(`crates/nvs-host/src/isolate.rs:973`) so the report names `Finish`.

**The marker's name and `is_finish` moved to `nvs-runtime`** (`crates/nvs-runtime/src/throwable.rs`,
beside the slot constants it is restated for the same reason as). `nvs-host` cannot name
`nvs-stdlib` — that crate depends on `nvs-host` — and `nvs_stdlib::script` now re-exports both, so
`nvs-cli` and `nvs_types`'s pinning test are unchanged.

**A pre-existing defect fell out of the control case and is fixed.** The deferred gate read
`Completion::ok` alone, and an `exit` is not a failure, so an exited request was running its
after-response work against `rule:concurrency/after-response-outlives-the-connection`. Both gates
(`crates/nvs-host/src/isolate.rs:824` and `:890`) now read `Ctx::ending` beside the flag.

**Refusing the member inside an exit hook landed** in `Ctx::abandon_exit_hook`
(`crates/nvs-runtime/src/ctx/hooks.rs`), beside the `EXITED` arm and with its reasoning.

Stage 4's `-p nvs-host` check is green (4 of 4). Its `-p nvs-stdlib` check is 2 of 3 —
`finish_in_a_task_child_ends_that_child_and_not_the_request` is unwritten. Stage 2's own checks
are still entirely unwritten, and they are the earliest red stage.

## Next group

**Stage 2: the queue on the served path** — one file set: `crates/nvs-host/tests/deferred.rs`, whose
helpers this session extended (`raise_the_finish_marker`, `records_the_ending`, `DRAINED`,
`closure_of`) and which is where all five of stage 2's tests belong.

- [ ] **Pin that a served request's exit hooks run, in the rule's order, on the host that serves it.**
      Three tests — `a_served_requests_exit_hooks_run_before_its_after_response_work`,
      `a_served_requests_exit_hooks_run_after_the_ladder_has_reported_a_throw`,
      `a_served_request_that_registered_no_hook_pays_no_drain` — against the drain call at
      `crates/nvs-host/src/isolate.rs:973`, which already runs before both callers' deferred work.
      `records_the_ending` at `crates/nvs-host/tests/deferred.rs:162` is the seam stand-in; the
      ordering half needs a static that both it and a deferred closure append to.
      `rule:observability/three-endings-fire-the-exit-queue`.
- [ ] **Pin the two endings that run no hook on the served path.**
      `a_fatal_on_the_served_path_runs_no_exit_hook` and `a_cancelled_request_runs_no_exit_hook`,
      against the `if !cancelled` guard at `crates/nvs-host/src/isolate.rs:968` — a cancellation is
      refused there and a `FATAL` at the seam itself, so the two fail apart and the cases must too.
      `rule:observability/a-fatal-and-a-cancellation-run-no-exit-hook`.
- [ ] **Write stage 4's last `-p nvs-stdlib` case**,
      `finish_in_a_task_child_ends_that_child_and_not_the_request`, beside
      `crates/nvs-stdlib/src/script.rs:1395`'s `finish_marker` helper. Different file set, so take it
      only if the two above left room. What a finish inside a `Core\Task` child *means* is not
      settled by any rule this pack printed — read `docs/agent/loop-goal.md`'s stage 4 prose before
      writing it. `rule:observability/three-endings-fire-the-exit-queue`.

## Backlog

- `rule:observability/a-hook-observes-and-never-steers` names only `exit` as refused inside a hook;
  the finish refusal that landed needs its sentence — stage 5's rulebook work, `docs/rules/`.
- `rule:concurrency/after-response-outlives-the-connection` says a finished request runs its deferred
  work only by implication; stage 5 should say the fourth ending by name.
- `Ctx::set_pending` releasing nothing it displaces is a sharp edge with one caller now depending on
  the workaround — `crates/nvs-runtime/src/ctx/error.rs:209`, playbook bullet filed.
