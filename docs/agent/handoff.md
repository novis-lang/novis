# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk, git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. `main` is frozen, and tag `pre-overhaul` is the rollback.

`bun nv loop` with no mode is one turn of the driver (`tools/nv/cmd/loop.ts`), and `NOVIS_LOOP_CLAUDE` names a stand-in for `claude` (`.agent-tmp/stand-in.ts`, git-ignored). One turn has been rehearsed on the cutover tree, wall, wip sweep and all.

**Every floor check is green under the new driver on the rehearsed cutover tree.** `bun nv loop --goal-only --collect` over all 1047 checks (log `.agent-tmp/floor-sweep-2.log`, 935s with the memo) is red only on Stage 9's `audit checks`, Stage 10's trailer check, Stage 11's `audit ci` and `loop-stats --guard`, and Stage 12's `audit python`, all work still to come. The 21 bullets that named a deleted file are triaged in `cutover.ts` beside step 4b (the `TRIAGE` table: 13 deleted, 8 rewritten with a mechanical `until`), and `nv playbook --check` there reports no stale path and no bad trailer.

Rehearsal setup: copy the git-ignored `tests/db/ca.crt`, `.agent-tmp/impact-reads.json`, `.loop/check-reads.json` and `.loop/proof-reads.json` into the worktree. The vscode checks also need `editors/vscode/node_modules`, a junction to main's (`cmd //c "mklink /J editors\vscode\node_modules D:\mwl\editors\vscode\node_modules"`), and a `git clean -fdq` leaves it standing. Rehearse with `git reset -q --hard <main head>; git clean -fdq` in the worktree, `bun .agent-tmp/cutover.ts --root <tree>`, commit there, then `bun .agent-tmp/cutover-sample.ts <tree>`. `nv audit checks` stays red on the old proofs name in 7 Python tools' comments until Stage 12 deletes them.

**The handover to the new driver, as read:** the Python turn that serves the cutover session sees `tools/loop.py` changed and takes its `rejudge` exit (`tools/loop.py:7291`) before anything reads a goal file, so `respawn.py` starts the shim, which is `bun nv loop`. The new driver keeps the carried `judge` field in `.loop/run.json` (`tools/nv/test/launch.test.ts:38`) and does not act on it, so the cutover session gets no ledger verdict of its own; its own green `--goal-only` before the commit is that verdict.

## Next group

**Stage 9: the cutover, for real, on `main`.** One file set: the whole tree, written by `.agent-tmp/cutover.ts`.

- [ ] **Run the cutover on main** — `bun .agent-tmp/cutover.ts --root . --for-real` (`.agent-tmp/cutover.ts:4`), read a sample with `bun .agent-tmp/cutover-sample.ts .`, then `bun nv loop --goal-only --collect` and expect exactly the five reds named in `## State`; any other red is a cutover defect, fixed in `cutover.ts` and re-rehearsed rather than patched on main. Goal prose § *Stage 9* (`docs/agent/loop-goal.md:553`) is the spec: one slice, one commit.
- [ ] **Wrap with `bun nv session --wrap`, not `python tools/session.py`** (`tools/nv/cmd/session.ts:61`): the cutover deletes `docs/agent/handoff.md`, and the handoff is the goal record's from then on. The status line is `CONTINUE`; the next turn is the new driver (`tools/loop.py:7291` hands it over).

## Backlog

- `examples/http.nvs`'s comment and `tools/origin.py` name the Python origin. When Stage 12 deletes `origin.py`, the comment names `bun tools/nv/driver/origin.ts` — `tools/nv/driver/origin.ts` header.
- Overload retries that cost no attempt, and rejoining a dropped stream with `claude --resume` (`tools/loop.py:7165`, `tools/loop.py:7207`). Today both count as a CLI failure — `tools/nv/cmd/loop.ts` header.
- The chain switch after a reached goal (`tools/loop.py:7383`), needed before any goal after `tooling-overhaul` runs under the new driver — `tools/nv/cmd/loop.ts` header.
- The goal-end doc and owner gates in the turn (`tools/loop.py:7370`), and judging a `judge` carried in from a Python rejudge — `tools/nv/cmd/loop.ts` header.
- The sweep's missing legs: verify-record reuse, batched proofs, WSL (with its own origin), valgrind — `tools/nv/driver/accept.ts` header.
- The 38 unresolved gap citations the cutover leaves — `.agent-tmp/cutover-unresolved-citations.txt`, goal prose stage 9.
