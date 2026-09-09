# Handoff

## State

**Goal 23 — stage 3 is complete. All four `-p nvs-server` drain checks are green beside the five
`nvs-cli` fan-out checks, and stages 1 and 2 stay green.** The next open stage is 4.

The drain is the **process's** bit and every worker takes its own handle on it, so a probe answers
the same on whichever core the proxy reached; the in-flight tally is each accept loop's own, so the
process ends when the last core's reaches zero rather than the first's. Both are asserted in
`crates/nvs-server/src/serve.rs`, the second over two `nvs_host::Worker`s on two listeners — two
handles on one socket would have made the case depend on which core the OS picked, which is the
fan-out's own property and not the tally's.

The rustdoc gate is green again: `one_mount`'s `# Errors` linked `address`, which became `addresses`
when the boot learned to bind every configured entry.

**Nothing registers `nvs_host::Watchdog` anywhere yet** — not `crates/nvs-server/` and not
`crates/nvs-cli/src/serve.rs` — so stage 4's third item is code plus a test, unlike the other two.

## Next group

**Stage 4: nothing leaks across a core** — one file set: `crates/nvs-server/src/serve.rs` with
`crates/nvs-server/src/admit.rs`. The third item also reaches `nvs-cli` and `nvs-host`, so take it
last.

- [ ] **`the_state_bleed_suite_passes_across_a_core_boundary`** — the rows are already written and
      run one core (`crates/nvs-server/src/serve.rs:4216`, one row's shape at
      `crates/nvs-server/src/serve.rs:4187`); what is missing is the parameterisation, a second
      worker serving the second request so the state a run could leave behind has a *different*
      core to be found on. `rule:security/isolate-shares-nothing` is what a row asserts and
      `rule:http-server/the-accept-fan-out-is-one-worker-per-core`, whose "what the cores share is
      compiled program text and nothing else" is why the boundary is worth crossing. `the_process_exits_when_the_last_cores_in_flight_count_reaches_zero`
      in the same file is the two-worker fixture to copy.
- [ ] **`the_in_flight_ceiling_is_fleet_wide_so_a_hot_core_cannot_refuse_while_neighbours_idle`** —
      the counter is one `Arc<Admission>` every core clones (`crates/nvs-server/src/admit.rs:177`,
      handed over at `crates/nvs-server/src/serve.rs:693`), so the assertion is that a request
      admitted on one core is counted against the same ceiling on another.
      `rule:http-server/admission-is-arithmetic-not-a-number` is the specification.
- [ ] **`the_watchdog_fires_per_worker_and_a_stalled_core_does_not_stall_the_fleet`** — a
      registration per worker (`crates/nvs-host/src/watchdog.rs:243`, over the deadline each loop
      already keeps) taken where the worker is built, `crates/nvs-cli/src/serve.rs:454`.
      `rule:http-server/a-wedged-core-is-detected-by-its-deadline` is the specification, and nothing
      in the tree registers one today.

## Backlog

- Stage 5's number: `serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names`
  and `ten_thousand_concurrent_cold_requests_for_one_file_compile_it_exactly_once`, `-p nvs-cli`
  (`docs/agent/loop-goal.toml`, stage `5 measured`).
- Stage 5's suites: the conformance and differential trees under the fan-out
  (`docs/agent/loop-goal.toml`, stage `5 suites`).
- Which core takes a connection is still asserted nowhere — the handles are, the race is not, and
  stage 5's measurement is where that lands.
