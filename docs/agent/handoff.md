# Handoff

## State

**Goal 23 — stage 4's first two checks are green.** `the_state_bleed_suite_passes_across_a_core_boundary`
and `the_in_flight_ceiling_is_fleet_wide_so_a_hot_core_cannot_refuse_while_neighbours_idle` are on disk
beside the stage 3 drain checks, and stages 1–3 stay green. The watchdog is the only stage 4 item left.

The bleed suite is one suite with three arms: the rows are untouched and `nothing_bled`
(`crates/nvs-server/src/serve.rs:4695`) is the assertion every arm is held to. The core arm **joins the
planting worker** before the probing request is written, so the first run has ended by construction
rather than by timing, and the two workers take two listeners rather than one for the tally fixture's
reason — which core the OS hands a connection to is the OS's choice.

A place in the in-flight count is held by a body three bytes short, which parks the run in
`crate::body::Pull` with the place still taken. That is what lets one core hold **both** places of a
fleet ceiling of two while its neighbour, having served nothing, answers `503`.

**Nothing registers `nvs_host::Watchdog` anywhere yet** — not `crates/nvs-server/` and not
`crates/nvs-cli/src/serve.rs` — so the last item is code plus a test, and the two live in different
crates: the check is filed `-p nvs-server` while the registration belongs where a worker is built.

## Next group

**Stage 4: nothing leaks across a core** — one file set: `crates/nvs-cli/src/serve.rs` with
`crates/nvs-host/src/watchdog.rs`, and `crates/nvs-server/src/serve.rs` for the test, which is where
the two-worker fixtures already are.

- [ ] **Register one watchdog per worker**, where the worker is built —
      `crates/nvs-cli/src/serve.rs:454` (`serve_on_worker`) — through
      `crates/nvs-host/src/watchdog.rs:243`'s `register(cpu, view)`, over the deadline each accept loop
      already keeps. `rule:http-server/a-wedged-core-is-detected-by-its-deadline` is the specification
      and `rule:http-server/a-wedged-core-is-shed-never-killed` bounds what firing may do.
- [ ] **`the_watchdog_fires_per_worker_and_a_stalled_core_does_not_stall_the_fleet`** — the check is
      `-p nvs-server`, so the test goes in `crates/nvs-server/src/serve.rs` beside `one_admitting_core`
      (`crates/nvs-server/src/serve.rs:6612`), which is the two-worker fixture to copy: one core's
      registration fires on its deadline while the neighbour keeps answering.
      `rule:http-server/a-wedged-core-is-detected-by-its-deadline` is what a firing means.

## Backlog

- Stage 5 and after are `docs/agent/goals/chain.toml`'s order, not this file's.
- `one_bleeding_core` and `one_admitting_core` (`crates/nvs-server/src/serve.rs:4495`, `:6612`) are the
  same worker skeleton twice; a third would be the point to extract one helper.
- Anything that must survive a goal switch goes in `docs/agent/carried-gaps.md`.
