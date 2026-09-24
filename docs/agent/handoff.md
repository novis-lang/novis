# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `audit checks` still names 26 Python checks, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

In the last rehearsal, all 20 of step 2b's patches applied, and every test under `tools/nv/test/` passed in the rehearsed tree. The guard test no longer reads `docs/agent/loop-goal.toml` as a file. Its `loop-goal-grep` cases name that path only as text.

The proof policy's per-kind switch is `owes` in `data/proofs/policy.json` (`tools/nv/schema/proofs.ts`), keyed by `all` or a kind: `all` applies first, then a kind's own entry. The importer carries the TOML's `[all]` and `[<kind>]` sections into it. Step 2b's `loadPolicy` patch applies it after the cutover. Goals `the-description-is-owed` and `plain-comments` name `owes.all` as their switch. `bun nv check` is the command that validates a record against its schema. `bun nv records --check` checks decision records only.

In `tools/nv/`, only the files the cutover's step 2b patches still say the old name. The rest spell it through `OLD` in `tools/nv/import/lib.ts`. The proofs' no-perf switch is `NVS_PROOFS_NO_PERF`, and no tool sets it.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse uncommitted work: in the worktree, run `git reset -q --hard <main head>; git clean -fdq`, then `git apply` a patch from `git diff` in main, then the script. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape.

`main` is frozen. Tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: what the cutover leaves pointing at a deleted file.** One file set: `tools/nv/cmd/guard.ts`, `tools/nv/test/guard.test.ts`, the cutover script's step 2b, and one Rust file. The rehearsal is the test.

- [ ] **`loop-goal-grep` guards a file the cutover deletes** (`tools/nv/cmd/guard.ts:367`, `tools/nv/test/guard.test.ts:41`): after step 2, no `docs/agent/loop-goal.toml` exists, so the rule denies nothing. Either retarget it to the live goal's record under `data/goals/` with a step 2b patch, or delete the rule and its cases. `bun nv loop --list` is what its reason recommends.
- [ ] **Two crate doc comments name the deleted policy TOML** (`crates/nvs-stdlib/src/db/row.rs:18`, `crates/nvs-stdlib/src/db/row.rs:1972`): step 3b rewrites no Rust comment. Add a step 2b patch that points them at `data/proofs/policy.json`. The comment is not in the `card` tier, so no perf figure goes stale (`rule:testing/member-perf-ledger`).

## Backlog

- The pack itself still ranks only the manifest's bullets. Stage 10's *Served by file* has `nv orient` rank the whole playbook by the item's paths, and `runTraps` in `tools/nv/cmd/orient.ts` is that ranking. Goal prose stage 10.
- `bun nv import --write` rewrites 8 playbook records whose `files` an earlier rewrite edited by hand, plus the live handoff's record. Decide which side is right before running it on a whole tree. Until then, run it and `git checkout` what you did not change. Goal prose stage 10.
- Stage 11 ports: `db-matrix`, `bench`, `try`, `gen-attribution`, `ci-changes`, `ci-green`, `class-cards`, and the `python -c` checks. Goal prose stage 11.
- The Python tools that say the old name (`tools/dossier.py`, `tools/chain.py`, `tools/goals.py`, `tools/goal-switch.py`, `tools/loop.py`, `tools/owners.py`, `tools/plan.py`, `tools/relink.py`, `tools/session.py`) keep stage 9's `audit checks` red until they are deleted, and each is deleted only after its parity is green. Goal prose *Parity before deletion*.
- The prose of goals `plain-comments` and `the-description-is-owed` still runs `python tools/dossier.py` in its steps and checks. That is stage 3b's argv rewrite and a prose pass. Goal prose stage 9.
- One `audit checks` run on the rehearsed tree once listed 866 Python checks, and a later run listed 26 with nothing written in between. Re-run it once before trusting a count. `tools/nv/cmd/audit.ts:92`.
