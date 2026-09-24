# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk, git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. `main` is frozen, and tag `pre-overhaul` is the rollback.

`bun nv loop` with no mode is now one turn of the driver (`tools/nv/cmd/loop.ts:438`, the session in `tools/nv/driver/launch.ts`): the session prompt plus `nv orient`'s pack down `claude -p` as stream-json, the status row repainted per event, the scoped sweep with the floor gate (`.loop/accept-floor.json`), a whole-plan `--collect` sweep when the scoped one is green with checks held, the ledger lines, and exit 75. It continues `.loop/run.json` in `loop.py`'s shape, so the running run keeps its numbering, and it refuses a tree whose `.loop/running` names another run. With no `NOVIS_LOOP_RUN` it is a run of one session. **It has not run a real session yet**: only `bun test tools/nv/test/launch.test.ts`, whose stand-in `claude` proves the stream, the log and the stdin close, and the refusal against the live marker. What it still does not do is listed in `loop.ts`'s header: the chain switch, the usage wall and retries, the wip sweep, the hold, repair and DONE-claim sessions, the goal-end doc and owner gates, the checkpoint.

`bun nv loop --goal-only` is the sweep by hand (`tools/nv/driver/accept.ts:512`); what it lacks is in `accept.ts`'s header (verify-record reuse, the batched proofs run, the WSL leg, valgrind).

In the rehearsal the cutover leaves 38 gap citations by position that name no gap today (`.agent-tmp/cutover-unresolved-citations.txt`). `nv audit checks` stays red on the old proofs name in 7 Python tools' comments until Stage 12 deletes them. `bun nv impact --probe` holds there; of the 63 floor-check shapes `cutover-sample.ts` runs, the 6 that fail are owed elsewhere. Before sampling, copy the git-ignored `tests/db/ca.crt`, `.agent-tmp/impact-reads.json`, `.loop/check-reads.json` and `.loop/proof-reads.json` into the worktree. Rehearse with `git diff > .agent-tmp/rehearsal.patch` in main, then in the worktree `git reset -q --hard <main head>; git clean -fdq; git apply ../../rehearsal.patch`, `bun .agent-tmp/cutover.ts --root <tree>` and `bun .agent-tmp/cutover-sample.ts <tree>`.

## Next group

**Stage 9: the driver is safe to leave unattended.** One file set: `tools/nv/cmd/loop.ts`, `tools/nv/driver/launch.ts`, read against `tools/loop.py`.

- [ ] **The wip sweep and the CLI retries** (`tools/nv/cmd/loop.ts:489` ends the run on any non-zero exit today, `tools/nv/cmd/loop.ts:500` only counts dirty paths): port `mark_interrupted` (`tools/loop.py:5784`, subject `wip(loop): the unfinished slice of`, which `nv orient` and `holes` match) over the paths `.loop/written.txt` names, and the usage-wall wait and non-zero-exit backoff (`tools/loop.py:7147`, `tools/loop.py:7228`, `limit.json`). Goal prose stage 8 owns the flags it keeps.
- [ ] **Rehearse one turn** in the cutover worktree: after the cutover script, `bun nv loop --max-sessions 1` there with a stand-in `claude` (add an env override beside `Bun.which` at `tools/nv/cmd/loop.ts:483`), and read the ledger and log it leaves.
- [ ] **Run the cutover for real** (`tools/loop.py:1`, which its step 4 turns into a shim that runs `bun nv loop` with the same arguments): rehearse once more, run it on main, and have `bun nv loop --goal-only` green before the commit.

## Backlog

- The chain switch after a reached goal (`tools/loop.py:7383`), before any goal after `tooling-overhaul` runs under the new driver — `tools/nv/cmd/loop.ts` header.
- The goal-end doc and owner gates in the turn (`tools/loop.py:7370`) — `tools/nv/cmd/loop.ts` header.
- The sweep's missing legs: verify-record reuse, batched proofs, WSL, valgrind — `tools/nv/driver/accept.ts` header.
- The 38 unresolved gap citations the cutover leaves — `.agent-tmp/cutover-unresolved-citations.txt`, goal prose stage 9.
