# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `audit checks` still names 26 Python checks and 24 tool files that say the old name, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

`nv rules` now reads the topic and rule records under `data/rules/`. On main, `--list`, `--show` for all 1011 rules, `--stats` and `--citations` print the same bytes as before, and `--check` and `--render --check` are clean. After the rehearsed cutover, all 51 distinct `bun nv rules` floor argvs pass. `nv records --check` passes after the cutover too. Its only finding was three links in `docs/decisions/0107.md` to the deleted `loop-goal.md`/`.toml`. Cutover step 3b now turns those into code text. Step 3b also rewrites a rule's `guardedBy` `tools/data/dossier-policy.toml` to `data/proofs/policy.json` (the `REHOMED` map).

Run it with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse: in the worktree, `git reset -q --hard <main head>; git clean -fdq`, copy in any uncommitted tool file, then run it. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape and writes `.agent-tmp/cutover-sample[-<command>].txt`.

What the rehearsal still leaves red, besides the worktree's missing `tests/db` fixtures:
- **`nv playbook`** takes neither `--check` nor `--closes` (7 checks), and **`nv orient`** takes no `--traps` (1 check).
- **The `nv proofs` flags `--emit-goals` and `--check-goals`** were deleted by design. 4 floor checks still run them. Each needs an argv that prints the same `want` line, or a record of why it is retired. The floor is never traded.
- **No `bun nv` command yet** for `db-matrix` (10 checks), `bench` (4), `python -c` (4), `try`, `gen-attribution` and `ci-changes` (2 each), and `ci-green` and `class-cards` (1 each). These are Stage 11 ports.
- **`bun nv gaps --module <path>` does not exist.** Every gap block the script cuts points to it.

`main` is frozen. Tag `pre-overhaul` is the rollback. The guard hook refuses a shell command that names `docs/agent/loop-goal.toml` beside a `grep`.

## Next group

**Stage 9: the playbook's and orient's floor flags.** One file set: `tools/nv/cmd/playbook.ts` and `tools/nv/cmd/orient.ts`, with the rehearsal as the test.

- [ ] **`nv playbook --check` and `--closes <slug>`** (`tools/nv/cmd/playbook.ts:400`). These are `playbook.py`'s two floor flags. Their `want` lines are `none -- every carried-gaps owner is live or struck` and ``goal `decided-closures` owns no docs/agent/carried-gaps.md row``. Prove it with `.agent-tmp/cutover-sample.ts <worktree> --all-of playbook`.
- [ ] **`nv orient --traps <path>`** prints `ranked from the whole playbook` (`tools/nv/cmd/orient.ts:1`). Prove it with `--all-of orient`.

## Backlog

- **The driver.** `nv loop` runs sessions with `respawn`'s protocol, `--goal-only` and every flag loop-goal.md § *Stage 8* names (`tools/nv/cmd/loop.ts`, `tools/nv/driver/`). The cutover's gate is `--goal-only` green under this driver. Porting `tools/loop.py` takes several sessions.
- **The Stage 11 ports the 26 Python checks need**: `db-matrix`, `bench`, `try`, `gen-attribution`, `ci-changes`, `ci-green`, `class-cards` and the `python -c` checks. See loop-goal.md § *Stage 11*.
- **`nv gaps --module <path>`**, which every cut gap block points to (`tools/nv/cmd/gaps.ts`). See loop-goal.md § *Stage 9* step 2.
- **The old name of the feature proofs** in 24 tool files, listed by `bun nv audit checks`. It goes when the Python tools are deleted and `tools/nv/` is reworded.
- The 4 floor checks on deleted `nv proofs` flags, and the 38 unresolved gap citations. Both are listed by the rehearsal.
- The eight playbook records that `nv import --write` rewrites. See `docs/agent/playbook/tooling/bun-nv-import-write-rewrites-a-data-playbook-bullet-back-to.md`.
