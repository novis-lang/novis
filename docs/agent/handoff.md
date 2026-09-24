# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `bun nv loop` runs the sweep but no session.

`bun nv loop --goal-only [--full] [--collect] [--stage|--name|--feature]` is the acceptance sweep (`tools/nv/cmd/loop.ts:293`, `tools/nv/driver/accept.ts:512`): the Python tier order, a stop at the first red unless `--collect`, `NOT GREEN:` / `GOAL REACHED` / `also red:` lines, and its own memo `.loop/accept-green.json` keyed by check id on `nv why`'s unit key. Proved on stage 9 alone: two reds as the ledger names them, the LF audit green, answered by the memo on a rerun, and run again under `--full`. What it still does not do is in `accept.ts`'s header: the verify-record reuse, the batched proofs run, the floor gate, the WSL leg and valgrind. The cost of keying all 1047 checks at once is not measured.

In the rehearsal the cutover leaves two findings: 38 gap citations by position that name no gap today (`.agent-tmp/cutover-unresolved-citations.txt`), and the loop is not yet a driver. `nv audit checks` stays red on the old proofs name in 7 Python tools' comments until Stage 12 deletes them.

`bun nv impact --probe` holds in the rehearsal: every one of the 11 probes. Of the 63 floor-check shapes `cutover-sample.ts` runs, 57 pass, and the 6 that fail are owed elsewhere (release binary, `ci-green`, `class-cards --check`, `proofs --gate`, `loop-stats --guard`). Before sampling, copy the git-ignored `tests/db/ca.crt`, `.agent-tmp/impact-reads.json`, `.loop/check-reads.json` and `.loop/proof-reads.json` into the worktree.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. To rehearse uncommitted work, run `git diff > .agent-tmp/rehearsal.patch` in main. Then in the worktree run `git reset -q --hard <main head>; git clean -fdq; git apply ../../rehearsal.patch`, and run the script. After that, `bun .agent-tmp/cutover-sample.ts <tree>`. `main` is frozen, and tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: the loop becomes a driver.** One file set: `tools/nv/cmd/loop.ts`, `tools/nv/driver/status.ts`, read against `tools/loop.py`.

- [ ] **The session turn** (`tools/loop.py:8447` `turn`, `tools/loop.py:5996` `run_session`, beside `goalOnly` at `tools/nv/cmd/loop.ts:293`): run `nv orient`'s pack plus the session prompt through `claude -p` with stream JSON, feed the stream to `tools/nv/driver/status.ts:100`'s `Session` for the status row, then `acceptance` from `tools/nv/driver/accept.ts:512` with `--collect` on the gate-open sweep, the ledger line and `respawn`'s exit `75`. Goal prose stage 8 owns the flags it keeps.
- [ ] **Run the cutover for real** (`tools/loop.py:1`, which its step 4 turns into a shim): once the driver runs a session, rehearse once more, run `cutover-sample.ts`, then `bun .agent-tmp/cutover.ts --root . --for-real` on main, read a sample of the diff, and commit it. Answer the 38 unresolved gap citations first.

## Backlog

- The sweep's missing halves, each named in `tools/nv/driver/accept.ts`'s header: reuse `nv verify`'s green test records, one batched `nv proofs --verify`, the floor gate, the WSL leg, the valgrind sweep.
- Time `checkKeys` (`tools/nv/cmd/loop.ts:268`) over the whole plan once, with a single command, never a sweep.
- `nv audit checks` goes green only when Stage 12 deletes the Python tools that still say the old proofs name.
