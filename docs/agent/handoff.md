# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6 `the options` is complete, gap included: a spawn site's `limits:`
and `grants:` now cross to a worker core instead of pinning the child to the parent's.**

`nvs_runtime::PlacedIsolate` carries a `Narrowing`, attached at the spawn by
`PlacedIsolate::narrowed_by` and applied on the far core by `PlacedIsolate::build`. `Ctx::narrow`
split to make that one mechanism rather than two: `Ctx::remains` resolves the three ceilings where
the parent can still be read, and `Ctx::narrow_under(narrowing, Remains)` applies a narrowing to
*this* context against them — which is the whole of what the same-core path ever read a parent for.
`crates/nvs-host/src/group.rs:224` no longer declines the crossing, and `placed.rs`'s known gap for
it is deleted.

**Stage 7 `the speedup` is what the driver's acceptance check is failing on**, and it is the first
kind of failure: neither artefact is written yet. `benches/abi-probe/benches/isolation.rs` has no
`worker_fan_out` row and `benches/abi-probe/tests/perf_guards.rs` has no near-linear guard.

**A stage label to fix as you go**: the handoff this one replaced called the narrowing-crossing group
"Stage 7", but `docs/agent/loop-goal.toml`'s stage 7 is `the speedup` — the group was stage 6's gap.
The pack therefore opened with the stage-7 overlay over stage-6 work. The group below is stage 7 for
real.

## Next group

**Stage 7: the speedup** — one file set: `benches/abi-probe/`.

- [ ] **The fan-out has its bench row beside spawn-to-result** —
      `benches/abi-probe/benches/isolation.rs:68` is the `isolate/spawn_to_result` row and a
      `worker_fan_out` row goes beside it, iterating a batch that belongs next to
      `spawn_to_result_batch` in `benches/abi-probe/shared/isolate.rs` (`peek.py --locate
      spawn_to_result_batch`) — the same source the guard compiles, which is why that module is
      outside `src/`. `rule:testing/perf-two-mechanisms`.
- [ ] **A CPU-bound fan-out across four worker cores is near-linear** — the guard named
      `a_cpu_bound_fan_out_across_four_worker_cores_is_near_linear_by_the_margin_this_test_names`,
      beside `benches/abi-probe/tests/perf_guards.rs:486`'s spawn-to-result guard and self-relative
      per `rule:testing/perf-two-mechanisms`: the bound is the same total work on one core, measured
      in the same run on this machine, never a figure quoted from another. A worker placement that
      cannot get a core falls back to this one *silently*, so the case has to fail rather than pass
      when every child ran here. `rule:concurrency/on-worker-runs-the-child-on-another-core`.

## Backlog

- Only a method entry is placed; a path entry needs a resolver a worker core can reach —
  `crates/nvs-host/src/placed.rs`'s `# Known gaps`, owner `m5-proofs`.
- Stage 8 `tsan` and stage 9 `the rulebook` are untouched — `docs/agent/loop-goal.toml:9920` on.
