# Handoff

## State

Goal `m8-stdlib-depth`. **Stage 12 is closed.** `Core\Metrics` is registered with three verbs and no
reader, a literal name outside `[a-z][a-z0-9_]*` and a `tainted` label value are compile-time
diagnostics on all three verbs, and the three Rust tests the stage's acceptance names all exist and
pass under `-p nvs-stdlib`. Its `.nvst` cases are on disk.

Stages 0–12 are closed; stage 13 (`Core\Process::spawn`) is the next one and is unstarted — the class
today has `run` alone (`crates/nvs-stdlib/src/process.rs:95`) and none of its five acceptance tests
exists. Nothing is blocked.

## Next group

**Stage 13: `Core\Process::spawn`** — one file set: `crates/nvs-stdlib/src/process.rs` (the registry
row at `:95`, the bounded drain at `:420`, the scheduler test at `:734`, the tests module at `:516`),
plus the handle class's registration in `crates/nvs-stdlib/src/registry.rs`.

- [ ] **`spawn` and its handle class are registered** — a second `CoreMethod` beside `run`'s at
      `crates/nvs-stdlib/src/process.rs:95`, answering the handle as a `CoreTy::Instance`, with the
      handle's own instance members and reference cards. `rule:core-classes/process-spawn` is the
      rule (`python tools/rules.py --show core-classes/process-spawn`) and
      `rule:core-classes/process-is-argv-only` keeps the argv-only shape —
      `there_is_no_shell_string_form_of_run_or_spawn` at
      `crates/nvs-stdlib/src/process.rs:547` already asserts it over the whole roster, so a `spawn`
      with a second text parameter fails there. The goal's § *Standing decisions* fixes the spelling
      under the verb lexicon. Closes `process_spawn_is_a_registered_member`.
- [ ] **A handle's reads suspend and the child never outlives its task** — the reads reuse the
      bounded pipe reader at `crates/nvs-stdlib/src/process.rs:420` and suspend the way
      `a_process_wait_suspends_its_coroutine_through_the_blocking_pool`
      (`crates/nvs-stdlib/src/process.rs:734`) already shows, with the kill on task end that the
      goal's standing decisions require so memory stays O(in-flight). Closes
      `a_spawned_childs_reads_suspend_the_coroutine_and_free_the_core`,
      `killing_a_spawned_child_ends_it_and_wait_answers_its_status` and
      `a_spawned_child_is_killed_when_its_task_ends` under `-p nvs-stdlib`.
- [ ] **One span per spawn, and the child's output reaches the program** —
      `a_spawn_produces_exactly_one_span` joins the tests module at
      `crates/nvs-stdlib/src/process.rs:516`. The shape to follow is `Core\Http`'s, which files one
      trace event per call whatever its attempts through `ctx.record_http` at
      `crates/nvs-stdlib/src/http.rs:3806`, behind the `TRACE` debug flag —
      `rule:observability/trace-events-carry-a-kind` is the rule. Then the conformance case
      `tests/conformance/core/process-spawn-streams-a-childs-output-into-the-program.nvst` the
      stage's `nvs-suite` check names.

## Backlog

- The release guard `concurrent_spawns_leave_the_scheduler_serving_other_tasks` —
  `benches/abi-probe/tests/perf_guards.rs:436`, run under `--release -p nvs-abi-probe`, is stage 13's
  second check and lands after the three slices above.
- Stages 14–15, the verification remainder and the `shipped` flips — the `[[check]]` blocks tagged
  `14 verification remainder` and `15 the rulebook` in `docs/agent/loop-goal.toml`.
- `rule:observability/metrics-three-members` spells the bag `array<string, string>` while the row
  declares `array<string>` (`crates/nvs-stdlib/src/metrics.rs:116`). One of the two is wrong; stage
  15's rulebook pass is where it is settled.
