# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk, git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. `main` is frozen, and tag `pre-overhaul` is the rollback.

`bun nv loop` with no mode is one turn of the driver (`tools/nv/cmd/loop.ts`), and `NOVIS_LOOP_CLAUDE` names a stand-in for `claude` (`.agent-tmp/stand-in.ts`, git-ignored, which plays script N on its Nth call, counted in `.loop/stand-in-count`). One turn has been rehearsed on the cutover tree, wall, wip sweep and all.

**The whole plan has been swept under the new driver on the cutover tree** (`bun nv loop --goal-only --collect`, the same `sweepOver` a turn runs; log in `.agent-tmp/floor-sweep.log`). No red came from the driver. Every floor red was a cutover defect or the rehearsal tree, and `cutover.ts` now fixes each of them:
- the four checks that `git grep` a rule's `"id"` line in `docs/rules/<topic>.json` now print `data/rules/<id>.json` with its id in front (step 3b);
- `spec_registry_coverage.rs`'s owner gate reads `data/chain.json`, not `N-<slug>.md` names (a step 2b patch);
- a `want` line naming a ported Python tool names its `bun nv` command (step 3b);
- step 4b runs `bun nv playbook --retire`, which deletes 20 bullets whose own `until: gone` names a file the cutover deleted.

Each of those checks was re-run green with `bun nv loop --run --name`. One floor red is left: `the playbook's own checks survive the file that is gone`. **21 bullets name `docs/agent/loop-goal.toml`, `loop-goal.md`, `handoff.md` or `tools/dossier.py` and declare no trailer that retires them**, listed in `.agent-tmp/cutover-stale-bullets.txt`.

Rehearsal setup: copy the git-ignored `tests/db/ca.crt`, `.agent-tmp/impact-reads.json`, `.loop/check-reads.json` and `.loop/proof-reads.json` into the worktree. The vscode checks also need `editors/vscode/node_modules`, a junction to main's (`cmd //c "mklink /J editors\vscode\node_modules D:\mwl\editors\vscode\node_modules"`), and a `git clean -fdq` leaves it standing. Rehearse with `git reset -q --hard <main head>; git clean -fdq` in the worktree, `bun .agent-tmp/cutover.ts --root <tree>`, commit there, then `bun .agent-tmp/cutover-sample.ts <tree>`. `nv audit checks` stays red on the old proofs name in 7 Python tools' comments until Stage 12 deletes them.

## Next group

**Stage 9: the cutover leaves the floor green.** One file set: `.agent-tmp/cutover.ts`, `docs/agent/playbook/`, `tools/nv/cmd/playbook.ts`.

- [ ] **Triage the 21 bullets the cutover leaves naming a deleted file** (`.agent-tmp/cutover-stale-bullets.txt`; the check is `tools/nv/cmd/playbook.ts:482`). Apply the goal's playbook rubric to each bullet: delete it, move its fact to a module doc, or rewrite its path to the file that replaced it (the goal record `data/goals/<slug>.json` for `loop-goal.toml`, `tools/nv/cmd/proofs.ts` for `dossier.py`). Write the decisions as a table in `cutover.ts` beside step 4b, so the cutover applies them, not a hand edit on main. Then rehearse and run `bun nv loop --run --name "playbook's own checks survive"` green.
- [ ] **Sweep once more after that**, with `bun nv loop --goal-only --collect` in the rehearsal tree, and expect reds only from Stage 9's `audit checks` and from Stages 10 to 12. Then **run the cutover for real** (`tools/loop.py:1`, which step 4 turns into a shim for `bun nv loop`). Run it on main, and have `bun nv loop --goal-only` green on every floor check before the commit.

## Backlog

- `examples/http.nvs`'s comment and `tools/origin.py` name the Python origin. When Stage 12 deletes `origin.py`, the comment names `bun tools/nv/driver/origin.ts` — `tools/nv/driver/origin.ts` header.
- Overload retries that cost no attempt, and rejoining a dropped stream with `claude --resume` (`tools/loop.py:7165`, `tools/loop.py:7207`). Today both count as a CLI failure — `tools/nv/cmd/loop.ts` header.
- The chain switch after a reached goal (`tools/loop.py:7383`), needed before any goal after `tooling-overhaul` runs under the new driver — `tools/nv/cmd/loop.ts` header.
- The goal-end doc and owner gates in the turn (`tools/loop.py:7370`) — `tools/nv/cmd/loop.ts` header.
- The sweep's missing legs: verify-record reuse, batched proofs, WSL (with its own origin), valgrind — `tools/nv/driver/accept.ts` header.
- The 38 unresolved gap citations the cutover leaves — `.agent-tmp/cutover-unresolved-citations.txt`, goal prose stage 9.
