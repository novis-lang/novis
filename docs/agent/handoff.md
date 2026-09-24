# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `audit checks` still names the checks that run Python, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

`bun nv db-matrix` replaces `tools/db-matrix.py` (`tools/nv/cmd/db-matrix.ts`). `bun nv parity db-matrix` matches on 9 of 9 cases, and one of them runs the SQLite leg for real. `bun nv db-matrix --all` printed all 8 legs `ok`, the three socket legs over WSL included, so every `want` line of the five checks holds under the new `argv`. Step 3b's `TOOLS` table in the cutover script now maps `db-matrix` to `db-matrix`, so the five matrix checks are rewritten at the cutover. That mapping has not been rehearsed yet: the next rehearsal's `audit checks` should no longer list a `db-matrix.py` check. `parseArgs` (`tools/nv/lib/py.ts`) now takes `repeated`, argparse's `action="append"`, and `order`, the order an ambiguous prefix lists its matches in.

In the last rehearsal, all 25 of step 2b's patches applied, every test under `tools/nv/test/` passed, and `bun nv audit goals` printed both of its lines. `audit checks` named no Rust file. It named the Python checks, and 8 Python tools that still say the old name.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse uncommitted work: in the worktree, run `git reset -q --hard <main head>; git clean -fdq`, then `git apply` a patch from `git diff` in main, then the script. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape.

`main` is frozen. Tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: the checks that still run Python.** One file set: the live goal's checks in `docs/agent/loop-goal.toml` (and their records under `data/goals/`), the cutover script's step 3b `TOOLS` table, `tools/nv/parity/groups.json`, and the Python tools they run. The rehearsal is the test, and `bun nv audit checks` in the rehearsed tree lists every one. `tools/nv/cmd/db-matrix.ts` is the shape of a port: an argparse-compatible `parseArgs` call, a parity group, and one `TOOLS` entry.

- [ ] **`tools/bench.py` runs in two checks** (`docs/agent/loop-goal.toml:2309`, `docs/agent/loop-goal.toml:4191`): port it to `bun nv bench` with a parity group, or say in step 3b which command its `argv` becomes. The checks use `--warm-start --max-work-ms` and `--serve-vs-fpm --record`. The tool is 1336 lines, so a port of the whole of it is a session of its own. The floor's `want` lines stay.
- [ ] **Five one-off Python checks** (`docs/agent/loop-goal.toml:2362`, `docs/agent/loop-goal.toml:2879`, `docs/agent/loop-goal.toml:10174`, `docs/agent/loop-goal.toml:10858`): `try.py`, `gen-attribution.py`, two inline `python -c` checks and `ci-changes.py`. Each needs an `nv` form before the cutover can say no check runs Python.
- [ ] **Rehearse the cutover** (`tools/nv/cmd/audit.ts:122`) after the ports: `audit checks` in the rehearsed tree should name no check that runs `db-matrix.py`, `bench.py` or the one-off tools.

## Backlog

- `bun nv loop` is not a driver yet (`tools/nv/cmd/loop.ts:21`). The cutover cannot land until it is one. Goal prose stage 8.
- The Python tools with a green parity group are deleted only in a later slice (the goal's § *Standing decisions*, "Parity before deletion"). Deleting `tools/db-matrix.py` also means rewriting the places that name it: `tests/db/compose.yaml`'s comments and `crates/nvs-db`'s module doc.
