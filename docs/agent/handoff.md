# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `audit checks` still names 26 Python checks and 24 tool files that say the old name, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

`bun nv gaps --module <path>` lists the gap records a file owes, or every file's under a directory, and `--json` prints them as a list. The cutover now checks every block it cuts against the records. In the rehearsal, 24 blocks are cut in 24 modules, and those 24 modules own all 46 records.

`bun nv chain --check` now also refuses a goal record that `data/chain.json` does not name. On success it prints both `want` lines of the checks the deleted goal emitter's flags ran. The cutover points those four floor checks at `chain --check` and renames them `chain: ...`. After the rehearsed cutover, all 13 checks that run `chain --check` pass.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse: in the worktree, `git reset -q --hard <main head>; git clean -fdq`, copy in any uncommitted tool file, then run it. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape and writes `.agent-tmp/cutover-sample[-<command>].txt`. With `--all-of`, it judges the `want` lines of every check that shares the argv.

What the rehearsal still leaves red, besides the worktree's missing `tests/db` fixtures:
- **No `bun nv` command yet** for `db-matrix` (10 checks), `bench` (4), `python -c` (4), `try`, `gen-attribution` and `ci-changes` (2 each), and `ci-green` and `class-cards` (1 each). These are Stage 11 ports.
- **24 tool files say the old name.** Seven are Python tools. The rest are under `tools/nv/`, and most of those read the pre-cutover layout.

`main` is frozen. Tag `pre-overhaul` is the rollback. The guard hook refuses a shell command that names `docs/agent/loop-goal.toml` beside a `grep`.

## Next group

**Stage 9: what still says the old name after the cutover.** One file set: the `tools/nv/` files `bun nv audit checks` names in the rehearsed tree, and the cutover script. The rehearsal is the test.

- [ ] **The pre-cutover goal layout reads** (`tools/nv/lib/chain.ts:41`, `tools/nv/lib/state.ts:60`, `tools/nv/cmd/chain.ts:150`): each reads or exempts the goals directory's generated subdirectory, which the cutover empties. These reads can only go in the cutover commit, so have the cutover script remove them, or apply a patch it carries. Prove in the rehearsal that `audit checks` no longer names these files and `chain --check` still passes.
- [ ] **The old check-name prefix and policy path** (`tools/nv/cmd/loop.ts:99`, `tools/nv/keys/checks.ts:309`): the cutover renames every such check to `proofs:` and rehomes the policy to `data/proofs/policy.json`, so after the cutover these names match nothing. Remove them in the same way as the first item.
- [ ] **The rest of `tools/nv/`** (`tools/nv/import/goals.ts:1`, `tools/nv/proofs/collect.ts:1`, `tools/nv/parity/known.json:180`, `tools/nv/test/import.test.ts:1`): the importer is committed and must still read legacy paths. Spell the old name the way `tools/nv/cmd/audit.ts:26` does, and reword each parity entry and test line so it does not say the old name.

## Backlog

- The pack itself still ranks only the manifest's bullets. Stage 10's *Served by file* has `nv orient` rank the whole playbook by the item's paths, and `runTraps` in `tools/nv/cmd/orient.ts` is that ranking. Goal prose stage 10.
- `bun nv import --write` rewrites 9 playbook records whose `files` an earlier rewrite edited by hand (`tools/nv/cmd/verify.ts`, `tools/nv/cmd/splice.ts`). Decide which side is right before running it on a whole tree. Goal prose stage 10.
- Stage 11 ports: `db-matrix`, `bench`, `try`, `gen-attribution`, `ci-changes`, `ci-green`, `class-cards`, and the `python -c` checks. Goal prose stage 11.
- The Python tools that still say the old name (`tools/chain.py`, `tools/goals.py`, `tools/goal-switch.py`, `tools/owners.py`, `tools/plan.py`, `tools/relink.py`, `tools/session.py`) are deleted only after their parity is green. Goal prose *Parity before deletion*.
- One `audit checks` run on the rehearsed tree listed 866 Python checks from records still holding old argvs. A second run a few minutes later listed 26, and nothing ran in between that writes. This is not explained. Re-run `audit checks` once before trusting a count. `tools/nv/cmd/audit.ts:96`.
