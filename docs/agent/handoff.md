# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `audit checks` still names 26 Python checks and 24 tool files that say the old name, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

`nv playbook --check` and `--closes <slug>` are ported, and `nv orient --traps <path>...` prints the traps section ranked from the whole playbook. After the rehearsed cutover, all 7 `playbook` floor checks and the `--traps` check pass. `--check` prints what `playbook.py --check` printed through the selector list; its module doc in `tools/nv/cmd/playbook.ts` names the three differences. The playbook importer now ends a lead at the first `**` outside a code span, which fixed four bullet records.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse: in the worktree, `git reset -q --hard <main head>; git clean -fdq`, copy in any uncommitted tool file, then run it. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape and writes `.agent-tmp/cutover-sample[-<command>].txt`.

What the rehearsal still leaves red, besides the worktree's missing `tests/db` fixtures:
- **The `nv proofs` flags `--emit-goals` and `--check-goals`** were deleted by design. 4 floor checks still run them. Each needs an argv that prints the same `want` line, or a record of why it is retired. The floor is never traded.
- **No `bun nv` command yet** for `db-matrix` (10 checks), `bench` (4), `python -c` (4), `try`, `gen-attribution` and `ci-changes` (2 each), and `ci-green` and `class-cards` (1 each). These are Stage 11 ports.
- **`bun nv gaps --module <path>` does not exist.** Every gap block the script cuts points to it.

`main` is frozen. Tag `pre-overhaul` is the rollback. The guard hook refuses a shell command that names `docs/agent/loop-goal.toml` beside a `grep`.

## Next group

**Stage 9: what the cutover's cut points at.** One file set: `tools/nv/cmd/gaps.ts` and the cutover script, with the rehearsal as the test.

- [ ] **`bun nv gaps --module <path>`** (`tools/nv/cmd/gaps.ts:472`): the gap records a module owes, which every gap block the cutover cuts from a module doc points to. Prove it on a module the cutover cut, in the rehearsal worktree.
- [ ] **The four floor checks that ran `dossier.py --emit-goals`/`--check-goals`** (`docs/agent/loop-goal.toml:12293`, `docs/agent/loop-goal.toml:12317`): give each an argv in `.agent-tmp/cutover.ts` that prints its `want` line, or record why it is retired. The floor is never traded. Prove it with `.agent-tmp/cutover-sample.ts <worktree> --all-of proofs`.

## Backlog

- The pack itself still ranks only the manifest's bullets. Stage 10's *Served by file* has `nv orient` rank the whole playbook by the item's paths, and `runTraps` in `tools/nv/cmd/orient.ts` is that ranking. Goal prose stage 10.
- `bun nv import --write` rewrites 9 playbook records whose `files` an earlier rewrite edited by hand (`tools/nv/cmd/verify.ts`, `tools/nv/cmd/splice.ts`). Decide which side is right before running it on a whole tree. Goal prose stage 10.
- Stage 11 ports: `db-matrix`, `bench`, `try`, `gen-attribution`, `ci-changes`, `ci-green`, `class-cards`, and the `python -c` checks. Goal prose stage 11.
