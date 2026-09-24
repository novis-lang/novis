# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `audit checks` still names 26 Python checks, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

In `tools/nv/`, only the files the cutover's step 2b patches still say the old name: `lib/chain.ts`, `lib/state.ts`, `cmd/chain.ts`, `cmd/loop.ts`, `keys/checks.ts`, `proofs/collect.ts` and `proofs/perf.ts`. The rest spell it through `OLD` in `tools/nv/import/lib.ts`, and `nv audit` imports it from there. A JSON file spells it `dos{2}ier` in a regex and `dossier` in a literal. The proofs' no-perf switch is `NVS_PROOFS_NO_PERF`, and no tool sets it.

The cutover's step 2b holds exact `from`/`to` replacements, and a `from` that is missing or matches twice is a finding. In the last rehearsal, all 20 patches applied. They make `nv chain`, `nv loop --feature`, the check keys and `nv proofs` read the flat goals directory and `data/proofs/`.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse: in the worktree, run `git reset -q --hard <main head>; git clean -fdq`, then the script. `bun .agent-tmp/policy-dump.ts <tree root>`, run from inside that tree, prints its loaded proof policy. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape.

`main` is frozen. Tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: what still breaks after the cutover.** One file set: `tools/nv/test/`, `tools/nv/schema/proofs.ts`, `tools/nv/proofs/collect.ts` and the cutover script's step 2b. The rehearsal is the test.

- [ ] **The guard test after the cutover** (`tools/nv/test/guard.test.ts:18`, `tools/nv/test/guard.test.ts:127`): its whole-read fixture names `docs/agent/loop-goal.toml`, which step 2 deletes, so 2 tests fail in the rehearsed tree and nowhere else. Point the fixture at a file that survives, or add it to step 2b.
- [ ] **The proof policy has no per-kind override** (`tools/nv/schema/proofs.ts:9`, `data/goals/the-description-is-owed.json:8`): `data/proofs/policy.json` holds `report` and `skip` alone. The handoffs of goals `plain-comments` and `the-description-is-owed` switch a proof on by adding `about = true` / `comments = true` under `[all]` in the deleted TOML. Give that switch a home (a schema field, or `POLICY` in `tools/nv/proofs/collect.ts:40`), and rewrite those goals' context and handoff lines to name it.

## Backlog

- The pack itself still ranks only the manifest's bullets. Stage 10's *Served by file* has `nv orient` rank the whole playbook by the item's paths, and `runTraps` in `tools/nv/cmd/orient.ts` is that ranking. Goal prose stage 10.
- `bun nv import --write` rewrites 9 playbook records whose `files` an earlier rewrite edited by hand (`tools/nv/cmd/verify.ts`, `tools/nv/cmd/splice.ts`). Decide which side is right before running it on a whole tree. Goal prose stage 10.
- Stage 11 ports: `db-matrix`, `bench`, `try`, `gen-attribution`, `ci-changes`, `ci-green`, `class-cards`, and the `python -c` checks. Goal prose stage 11.
- The Python tools that say the old name (`tools/dossier.py`, `tools/chain.py`, `tools/goals.py`, `tools/goal-switch.py`, `tools/loop.py`, `tools/owners.py`, `tools/plan.py`, `tools/relink.py`, `tools/session.py`) keep stage 9's `audit checks` red until they are deleted, and each is deleted only after its parity is green. Goal prose *Parity before deletion*.
- Two crate doc comments name the deleted policy TOML (`crates/nvs-stdlib/src/db/row.rs:18`, `crates/nvs-stdlib/src/db/row.rs:1972`). Step 3b rewrites no Rust comment.
- One `audit checks` run on the rehearsed tree once listed 866 Python checks, and a later run listed 26 with nothing written in between. Re-run it once before trusting a count. `tools/nv/cmd/audit.ts:92`.
