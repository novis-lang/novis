# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: two floor checks still run Python, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

The stage-9 check is now named "no check runs Python, and no check, directive or tool says the old proofs name", in `data/goals/tooling-overhaul.json`, the goal's toml and `docs/agent/loop-goal.toml`. `bun nv goal` writes only `context --add`, so a check name is edited by hand in those three.

`bun nv ci-changes`, `bun nv ci-green` and `bun nv class-cards` replace their Python tools, each with a green parity group. `ci-changes` reads the package graph off the manifests with `smol-toml`, because CI's `changes` job has no cargo; `tools/nv/test/ci-changes.test.ts` holds that graph to `cargo metadata`. Its lane table names `tools/nv/cmd/db-matrix.ts` and `tools/nv/cmd/reference.ts` where Python named the `.py` files. `--lanes` is new, and prints the table the lane-table floor check reads.

The cutover's step 3b maps all three in `TOOLS`. `RETARGETED` now maps an argv to `{ argv, name? }`, and carries both `python -c` checks: the `ci.yml` marker check becomes a `bun -e` that prints the markers in the same order, and the lane-table check becomes `bun nv ci-changes --lanes`. Want lines are matched in order (`tools/loop.py:3739`), which is why the `ci.yml` check is not a `git grep`. The rehearsal printed "has no `bun nv` command yet" for `try` and `gen-attribution` alone, and `audit checks` named 4 checks: those two, each under two goals. It also named 8 Python tools that say the old name. Those are deleted at the cutover, not ported.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse uncommitted work: in the worktree, run `git reset -q --hard <main head>; git clean -fdq`, then `git apply` a patch from `git diff` in main, copy any new file across by hand, then run the script. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape.

`main` is frozen. Tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: the last two checks that run Python.** One file set: the two Python tools, their ports under `tools/nv/cmd/`, `tools/nv/main.ts`, `tools/nv/parity/groups.json` and `known.json`, and the cutover script's step 3b `TOOLS` table. `tools/nv/cmd/class-cards.ts` is the smallest shape of a port, and `tools/nv/cmd/ci-changes.ts` the one with an argparse-compatible `parseArgs` call.

- [ ] **Port `tools/try.py --bundle`** (`tools/try.py:42`): the floor check `nvs build --compile produces a runnable binary` wants `Hello, World!`. Port the whole tool or only its `--bundle` mode, with a parity group over both, and add `try` to `TOOLS`.
- [ ] **Port `tools/gen-attribution.py --check-c-deps`** (`tools/gen-attribution.py:696`): the floor check wants `C dependencies`. CI also runs `--check` (`.github/workflows/ci.yml:470`), so the port needs that mode too before the Python tool can go.
- [ ] **Rehearse the cutover** (`tools/nv/cmd/audit.ts:122`) after the ports: `audit checks` in the rehearsed tree should name no check that runs Python, and only the Python tools the cutover deletes should say the old name.

## Backlog

- `.github/workflows/ci.yml` still runs `python tools/ci-changes.py`, `db-matrix.py`, `lints.py`, `gen-attribution.py`, `check-migration.py`, `reference.py`, `rules.py`, `records.py` and `check-links.py`, and needs `setup-bun`. The floor check `the CI workflow runs the five-driver matrix` wants `python tools/db-matrix.py --all` in it, so its want line and the switch have to be settled together at the cutover (goal prose stage 9, `docs/agent/goals/122-tooling-overhaul.md`).
- `bun nv ci-green`'s `BOOKKEEPING` still lists `docs/agent/` and `docs/plan/` only. Once handoffs live under `data/goals/`, it needs that prefix too (`tools/nv/cmd/ci-green.ts:23`).
- `bun nv loop` is not a driver yet (`tools/nv/cmd/loop.ts:21`), and the cutover needs one.
