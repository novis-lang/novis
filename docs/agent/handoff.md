# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `audit checks` still names 26 checks that run Python, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

In the last rehearsal, all 25 of step 2b's patches applied. Every test under `tools/nv/test/` passed in the rehearsed tree, and `bun nv audit goals` printed both of its lines there. `audit checks` named no Rust file there. It named the 26 Python checks, and 8 Python tools that still say the old name. The cutover deletes those tools.

The guard rule `goal-grep` (`tools/nv/cmd/guard.ts`, `GOAL_CHECKS`) denies a searcher over one goal's record `data/goals/<slug>.json`, and over `docs/agent/loop-goal.toml` until the cutover. A step 2b patch drops the TOML from the rule and from its test. Two more step 2b patches point `crates/nvs-stdlib/src/db/row.rs`'s two doc comments at `data/proofs/policy.json`.

The proof policy's per-kind switch is `owes` in `data/proofs/policy.json` (`tools/nv/schema/proofs.ts`). `bun nv check` validates a record against its schema. The proofs' no-perf switch is `NVS_PROOFS_NO_PERF`, and no tool sets it.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse uncommitted work: in the worktree, run `git reset -q --hard <main head>; git clean -fdq`, then `git apply` a patch from `git diff` in main, then the script. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape.

`main` is frozen. Tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: the checks that still run Python.** One file set: the live goal's checks in `docs/agent/loop-goal.toml` (and their records under `data/goals/`), the cutover script's step 3b `argv` table, and the Python tools they run. The rehearsal is the test, and `bun nv audit checks` in the rehearsed tree lists every one.

- [ ] **`tools/db-matrix.py` runs in five checks** (`docs/agent/loop-goal.toml:3207`, `docs/agent/loop-goal.toml:7253`, `docs/agent/loop-goal.toml:10687`): port it to an `nv` subcommand with a parity check, or say in step 3b which command its `argv` becomes. `rule:testing/feature-proofs` does not cover it. The floor's `want` lines stay.
- [ ] **`tools/bench.py` runs in two checks** (`docs/agent/loop-goal.toml:2309`, `docs/agent/loop-goal.toml:4191`): the same choice, for `--warm-start --max-work-ms` and `--serve-vs-fpm --record`.
- [ ] **Five one-off Python checks** (`docs/agent/loop-goal.toml:2362`, `docs/agent/loop-goal.toml:2879`, `docs/agent/loop-goal.toml:10174`, `docs/agent/loop-goal.toml:10858`): `try.py`, `gen-attribution.py`, two inline `python -c` checks and `ci-changes.py`. Each needs an `nv` form before the cutover can say no check runs Python.

## Backlog

- `bun nv loop` is not a driver yet (`tools/nv/cmd/loop.ts:21`). The cutover cannot land until it is one. Goal prose stage 8.
- The 8 Python tools that still say the old name are deleted by the cutover only after their parity checks are green (the goal's § *Standing decisions*, "Parity before deletion").
