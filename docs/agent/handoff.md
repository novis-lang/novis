# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk, git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. `main` is frozen, and tag `pre-overhaul` is the rollback.

`bun nv loop` with no mode is one turn of the driver (`tools/nv/cmd/loop.ts`, the session in `tools/nv/driver/launch.ts`, the wip sweep and the usage wall in `tools/nv/driver/sweep.ts`). A session that ends without wrapping has its own paths (write tools on the stream, plus `.loop/written.txt`) swept into a `wip(loop)` commit with `.loop/interrupted.json`; a wall-refused session waits out `.loop/limit.json` and runs again in the same turn; a non-zero exit backs off and retries up to `--max-retries`. **It has not run a real session yet**: only `bun test tools/nv/test/launch.test.ts tools/nv/test/sweep.test.ts`. What it still does not do is listed in `loop.ts`'s header: the chain switch, overload retries, rejoining a dropped stream, the hold and keys, repair and DONE-claim sessions, the goal-end doc and owner gates, the checkpoint.

`bun nv loop --goal-only` is the sweep by hand (`tools/nv/driver/accept.ts`); what it lacks is in `accept.ts`'s header (verify-record reuse, the batched proofs run, the WSL leg, valgrind).

In the rehearsal the cutover leaves 38 gap citations by position that name no gap today (`.agent-tmp/cutover-unresolved-citations.txt`). `nv audit checks` stays red on the old proofs name in 7 Python tools' comments until Stage 12 deletes them. Before sampling, copy the git-ignored `tests/db/ca.crt`, `.agent-tmp/impact-reads.json`, `.loop/check-reads.json` and `.loop/proof-reads.json` into the worktree. Rehearse with `git diff > .agent-tmp/rehearsal.patch` in main, then in the worktree `git reset -q --hard <main head>; git clean -fdq; git apply ../../rehearsal.patch`, `bun .agent-tmp/cutover.ts --root <tree>` and `bun .agent-tmp/cutover-sample.ts <tree>`.

## Next group

**Stage 9: the driver is safe to leave unattended.** One file set: `tools/nv/cmd/loop.ts`, `tools/nv/driver/launch.ts`, `tools/nv/driver/sweep.ts`, read against `tools/loop.py`.

- [ ] **Rehearse one turn** in the cutover worktree: after the cutover script, `bun nv loop --max-sessions 1` there with a stand-in `claude` (add an env override beside `Bun.which` at `tools/nv/cmd/loop.ts:479`), and read the ledger and log it leaves. Have the stand-in once exit 1 with a `rate_limit_event` a minute ahead and once write a file and exit 0 unwrapped, so the wall branch (`tools/nv/cmd/loop.ts:539`) and the wip commit are seen on a real tree.
- [ ] **Run the cutover for real** (`tools/loop.py:1`, which its step 4 turns into a shim that runs `bun nv loop` with the same arguments): rehearse once more, run it on main, and have `bun nv loop --goal-only` green before the commit.

## Backlog

- Overload retries that cost no attempt, and rejoining a dropped stream with `claude --resume` (`tools/loop.py:7165`, `tools/loop.py:7207`); today both count as a CLI failure — `tools/nv/cmd/loop.ts` header.
- The chain switch after a reached goal (`tools/loop.py:7383`), before any goal after `tooling-overhaul` runs under the new driver — `tools/nv/cmd/loop.ts` header.
- The goal-end doc and owner gates in the turn (`tools/loop.py:7370`) — `tools/nv/cmd/loop.ts` header.
- The sweep's missing legs: verify-record reuse, batched proofs, WSL, valgrind — `tools/nv/driver/accept.ts` header.
- The 38 unresolved gap citations the cutover leaves — `.agent-tmp/cutover-unresolved-citations.txt`, goal prose stage 9.
