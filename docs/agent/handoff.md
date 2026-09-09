# Handoff

## State

**Goal 23 — stage 4 is closed.** All three tests the `4 isolation` check names pass, and stages 1–3 stay
green. Stage 5 is next and neither of its two tests exists yet under the name the check gives it.

`nvs serve` now watches its fleet: the process holds one `nvs_host::Watchdog` in the fan-out frame
(`crates/nvs-cli/src/serve.rs:383`), each `Core` carries the CPU it will be pinned to and a clone of it,
and a worker registers itself from its own thread on the line after it installs its reactor — which is
where a `DeadlineView` comes from. The handle deregisters on drop, so the watched set is the cores
running rather than the cores started. The no-CPU fallback path is unwatched by construction: a `CpuId`
only ever comes from `nvs_host::cpus()`, so an unpinned thread has no name a report could carry.

**The registration call site itself is held by compilation alone.** The acceptance check is filed
`-p nvs-server`, and that crate cannot reach `nvs serve`; the fixture registers its own two cores, so
what the test holds is the mechanism — one entry per running core, a report that names the wedged core
and not its turning neighbour, and a fleet that keeps serving through both.

## Next group

**Stage 5: the number** — one file set: `crates/nvs-cli/src/serve.rs` with `crates/nvs-cli/src/script.rs`,
which are the two files the `5 measured` check's tests are filed against.

- [ ] **`ten_thousand_concurrent_cold_requests_for_one_file_compile_it_exactly_once`** —
      `crates/nvs-cli/src/script.rs:790` already holds
      `ten_thousand_concurrent_cold_requests_compile_the_file_exactly_once`, which is one scheduler on
      one core; the acceptance name is the fleet's version of it, so the slice is the same assertion
      made across workers sharing one `Arc<Compiler>`. `rule:security/isolate-shares-nothing`'s "shares
      immutable compiled code" is the specification, and the goal's § *Standing decisions* settles that
      the cache is shared rather than per-core.
- [ ] **`serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names`** — new, beside
      the fan-out tests at `crates/nvs-cli/src/serve.rs:1285`
      (`one_worker_is_spawned_per_core_and_each_takes_its_own_listener_handle` is the fixture shape).
      `rule:http-server/the-accept-fan-out-is-one-worker-per-core` is what it measures; the margin is
      the test's own to name, and a host with fewer than four CPUs is the early return the neighbouring
      fleet tests already take.

**The pack has no `[context.stage.5]`.** `docs/agent/loop-goal.toml` defines overlays for stages 2, 3 and
4 only, so a group naming stage 5 gets the goal's base manifest — wider, not broken. Add one naming
`crates/nvs-cli/src/script.rs` when the group above is taken.

## Backlog

- Stage 5 and after are `docs/agent/goals/chain.toml`'s order, not this file's.
- The third copy of the worker skeleton now exists — `one_bleeding_core`, `one_admitting_core` and
  `one_watched_core` (`crates/nvs-server/src/serve.rs:4495`, `:6612`, `:6812`) — so the extraction that
  backlog item was waiting for is due.
- `rule:http-server/a-wedged-core-is-shed-never-killed` stays `designed`: nothing subscribes to a stall
  report, and `crates/nvs-host/src/watchdog.rs` § *Report and shed, never kill* owns why.
- The watchdog's margin and interval are compiled-in until the `[limits]` block lands
  (`crates/nvs-host/src/watchdog.rs:53`).
- Anything that must survive a goal switch goes in `docs/agent/carried-gaps.md`.
