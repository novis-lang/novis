# Handoff

## State

**Goal `m5-proofs` (M5). Stage 8 `tsan` is complete: all four of its acceptance checks are green.**
`wsl.exe -- bash -lc "bash tools/tsan.sh"` ends on `tsan: clean` over `nvs-host` and `nvs-runtime`,
and `.github/workflows/ci.yml:598` is the `tsan:` job that runs that same script.

`crates/nvs-host/src/tsan.rs` is the annotation half — one fiber per task, switched inside
`Fiber::around`, which `Task::resume` and `Task::force_unwind`
(`crates/nvs-host/src/scheduler.rs:614`) wrap the stack switch in. That module's doc owns why an
annotation may not straddle a switch; the playbook carries the failure it produces.

**The gate is this crate's `tsan` cargo feature, not the `--cfg nvs_tsan` the goal's stage 8 names.**
`cfg(sanitize = "thread")` — the predicate that would read as what it means — is unstable and is an
`E0658` on the pinned stable compiler whether or not the branch is taken, and a bare `--cfg` trips
`unexpected_cfgs` in every ordinary build because the only place `check-cfg` could declare it is the
`[lints]` table `tools/lints.py` generates. Recorded in the module doc's § *What is compiled*.

**Stage 9 `the rulebook` is the last stage and none of it is done.**

## Next group

**Stage 9: the rulebook** — one file set: `docs/rules/concurrency.json`,
`docs/rules/observability.json` and the two chapters `python tools/rules.py --render` writes from them.

- [ ] **`concurrency/on-worker-runs-the-child-on-another-core` flips to `shipped`, with `guardedBy`
      filled from this goal's cases and tests** — `docs/rules/concurrency.json:45` is the `status`
      field, `:43` the rule's `id`. The rule is `rule:concurrency/on-worker-runs-the-child-on-another-core`
      itself, and stage 5's record `docs/decisions/0184.md` is its `because`.
- [ ] **`observability/spawn-is-its-own-event` flips to `shipped` the same way** —
      `docs/rules/observability.json:322` is its `id`, and `rule:observability/spawn-is-its-own-event`
      is what it has to be true of.
- [ ] **`python tools/rules.py --render` rewrites the chapters** — `docs/rules/concurrency.md:53` and
      `docs/rules/observability.md` are what it overwrites, and `--check` is what gates them.

## Backlog

- `docs/agent/loop-goal.md:200` still says `--cfg nvs_tsan`; the goal file is the user's, so the
  deviation is recorded in `crates/nvs-host/src/tsan.rs`'s module doc instead.
- No `nvs serve` end-to-end runaway test for the CPU/memory ceilings — `docs/agent/carried-gaps.md`.
