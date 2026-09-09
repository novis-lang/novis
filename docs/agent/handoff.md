# Handoff

## State

**Goal 24 — stage 6's first two items are landed.** `Core\Budget` is registered with `memoryHeld`,
`memoryPeak` and `memoryLimit`, all three `uint` byte counts over `Ctx`; `§16 Core\Budget` left
`spec-classes-part-two-outstanding.txt` and its three keys left `migration-members-outstanding.txt`.
`rule:observability/memory-is-three-numbers-on-core-budget` and
`rule:observability/a-memory-peak-is-recorded-not-asked-for` are both `shipped` now, with the cases
and the module as their `guardedBy`.

**The peak is recorded, not sampled.** `budget::add` moves a `PEAK` thread-local inside the branch
that already tests for a positive delta; `Ctx::new` rebases it and carries what it displaced, and
`Drop for Ctx` republishes `max(enclosing, reached)` before any teardown allocates, so a nested
isolate cannot erase its parent's mark. `crates/nvs-runtime/src/budget.rs`'s module doc is the home
of what that spends, and `crates/nvs-stdlib/src/budget.rs`'s owns why the record's `int` is a `uint`
here and why `Core\Os::residentBytes` stays where it is.

**The stage-5 acceptance check was naming a test that does not exist** — a shorthand for
`every_migration_member_row_names_a_registered_member` — and now names the real one. Stage 5 is
green; the playbook bullet above is the trap.

**Stage 6 item 3 is the whole of what is left in the goal**: the three readings an operator gets
without asking. Nothing else in `loop-goal.toml` is open.

## Next group

**Stage 6: the three unasked readings** — one file set: `crates/nvs-stdlib/src/script.rs`,
`crates/nvs-server/src/metrics.rs`, `crates/nvs-config/src/tree.rs`.

- [ ] **`Core\Script\ExitReport::memoryPeak`.** A fourth reading on the report a `spawn script`
      handler is handed, off `Ctx::memory_peak` exactly as `Core\Budget::memoryPeak` reads it
      ([0148](../decisions/0148.md) § 14, and `rule:observability/a-memory-peak-is-recorded-not-asked-for`
      for what the mark is). The class rows are `crates/nvs-stdlib/src/script.rs:256`, its slot layout
      `crates/nvs-stdlib/src/script.rs:343` and its symbol table `crates/nvs-stdlib/src/script.rs:338`
      — a slot, not a member body, so this is not the five-edit shape.
- [ ] **The `nvs_request_memory_peak_bytes` histogram**, beside the four default series at
      `crates/nvs-server/src/metrics.rs:154` and with `Kind::Histogram`'s buckets
      (`crates/nvs-server/src/metrics.rs:119`), per
      `rule:observability/the-runtime-exports-what-it-already-measures` — the runtime already keeps
      the figure, which is the whole admission test.
- [ ] **`[limits] memory_high_water`, a fraction whose crossing writes one `Warn`.**
      `rule:observability/memory-high-water-writes-a-warn` and 0148 § 14: unwritten is off, off is
      silent, and a value outside `0.0..=1.0` is refused at boot by the typed-value path
      `crates/nvs-config/src/tree.rs:171`'s `Limits` block already has beside `max_output`.

## Backlog

- A `.nvst` case for the nested mark — a parent's peak surviving a child that allocated less. It is
  pinned today as a `-p nvs-runtime` unit test, `crates/nvs-runtime/src/budget.rs`'s
  `a_nested_context_measures_its_own_allocation_and_restores_what_it_displaced`; the program-level
  spelling needs `spawn`, which `docs/agent/loop-goal.md` § Stage 6 item 2 asks for.
- Nothing installs an operating-system shutdown handler, so nothing raises `SafepointFlags::SHUTDOWN`
  outside tests — `crates/nvs-cli/src/serve.rs:730`, with the control socket beside it.
- `Core\Process::spawn` is still `unowned` in `crates/nvs-stdlib/tests/migration-members-outstanding.txt`
  — `docs/agent/carried-gaps.md` § Unowned.
- `spec-classes-part-two-outstanding.txt` still owes `Core\Signature` (29), `Core\Metrics` (unowned)
  and §17's four; none is this goal's.
