# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `audit checks` still names 26 Python checks and 17 tool files that say the old name, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

The cutover's step 2b patches the tools that can only change in the cutover commit. It holds exact `from`/`to` replacements, and a `from` that is missing or matches twice is a finding. In the rehearsal, all 20 patches apply:
- `tools/nv/lib/chain.ts`, `tools/nv/lib/state.ts` and `tools/nv/cmd/chain.ts` read the flat goals directory. `nv chain --check` exempts a feature-proof goal (its `## The target` cites `rule:testing/feature-proofs`) from the `## Why here` and README-row notes. That predicate matches the 103 generated goals and no hand-written one. After the cutover, `chain --check` passes with no notes.
- `tools/nv/cmd/loop.ts --feature` matches the `proofs:` prefix alone. `tools/nv/keys/checks.ts` keys proofs on `data/proofs`.
- `tools/nv/proofs/collect.ts` and `tools/nv/proofs/perf.ts` read `data/proofs/policy.json` and `help-backlog.json`. Without this, step 2's deletion of `tools/data/*.toml` would leave `nv proofs` silently reading an empty policy. `loadPolicy` gives the same 306 skips before and after the cutover. The only difference is the backlog path inside the reason text. `docs/perf/members.md`'s policy path moves with the generator.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse: in the worktree, run `git reset -q --hard <main head>; git clean -fdq`, then the script. `bun .agent-tmp/policy-dump.ts <tree root>`, run from inside that tree, prints its loaded proof policy. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape.

`main` is frozen. Tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: what still breaks or says the old name after the cutover.** One file set: `tools/nv/` and the cutover script's step 2b. The rehearsal is the test.

- [ ] **The rest of `tools/nv/`** (`tools/nv/import/goals.ts:1`, `tools/nv/import/proofs.ts:1`, `tools/nv/proofs/collect.ts:90`, `tools/nv/cmd/proofs.ts:36`, `tools/nv/parity/known.json:180`, `tools/nv/test/import.test.ts:1`): the importer is committed and must still read the legacy paths. Spell the old name the way `tools/nv/cmd/audit.ts:26` does. Reword each parity entry, test line and comment so it does not say the old name. The `NVS_*_NO_PERF` variable in `collect.ts` is renamed together with every tool that sets it.
- [ ] **The guard test after the cutover** (`tools/nv/test/guard.test.ts:18`, `tools/nv/test/guard.test.ts:127`): its whole-read fixture names `docs/agent/loop-goal.toml`, which step 2 deletes, so 2 tests fail in the rehearsed tree and nowhere else. Point the fixture at a file that survives, or add it to step 2b.
- [ ] **The proof policy has no per-kind override** (`tools/nv/schema/proofs.ts:9`, `data/goals/the-description-is-owed.json:8`): `data/proofs/policy.json` holds `report` and `skip` alone. The handoffs of goals `plain-comments` and `the-description-is-owed` switch a proof on by adding `about = true` / `comments = true` under `[all]` in the deleted TOML. Give that switch a home (a schema field, or `POLICY` in `tools/nv/proofs/collect.ts:40`), and rewrite those goals' context and handoff lines to name it.

## Backlog

- The pack itself still ranks only the manifest's bullets. Stage 10's *Served by file* has `nv orient` rank the whole playbook by the item's paths, and `runTraps` in `tools/nv/cmd/orient.ts` is that ranking. Goal prose stage 10.
- `bun nv import --write` rewrites 9 playbook records whose `files` an earlier rewrite edited by hand (`tools/nv/cmd/verify.ts`, `tools/nv/cmd/splice.ts`). Decide which side is right before running it on a whole tree. Goal prose stage 10.
- Stage 11 ports: `db-matrix`, `bench`, `try`, `gen-attribution`, `ci-changes`, `ci-green`, `class-cards`, and the `python -c` checks. Goal prose stage 11.
- The Python tools that still say the old name (`tools/chain.py`, `tools/goals.py`, `tools/goal-switch.py`, `tools/owners.py`, `tools/plan.py`, `tools/relink.py`, `tools/session.py`) are deleted only after their parity is green. Goal prose *Parity before deletion*.
- Two crate doc comments name the deleted policy TOML (`crates/nvs-stdlib/src/db/row.rs:18`, `crates/nvs-stdlib/src/db/row.rs:1972`). Step 3b rewrites no Rust comment.
- One `audit checks` run on the rehearsed tree once listed 866 Python checks, and a later run listed 26 with nothing written in between. Re-run it once before trusting a count. `tools/nv/cmd/audit.ts:96`.
