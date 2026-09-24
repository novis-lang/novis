# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `bun nv loop` is not a driver. It runs no session and no sweep (`tools/nv/cmd/loop.ts:27`).

`bun nv loop --run --stage|--name|--feature` runs the checks those filters select, once each, through `tools/nv/driver/accept.ts`'s `Sweep`, which shares one process between checks that ask for the same one and judges every check kind. That module's header names what the Python sweep still does that it does not: the memo, the verify-record reuse, the batched proofs run, the tiers, the WSL leg and valgrind. The plan is 1047 checks, 1017 of them the floor, so a sweep with no memo is hours and never a proof.

In the rehearsal the cutover leaves two findings: 38 gap citations by position that name no gap today (`.agent-tmp/cutover-unresolved-citations.txt`), and the loop is not yet a driver. `nv audit checks` stays red on the old proofs name in 7 Python tools' comments until Stage 12 deletes them.

`bun nv impact --probe` holds in the rehearsal: every one of the 11 probes. Of the 63 floor-check shapes `cutover-sample.ts` runs, 57 pass, and the 6 that fail are owed elsewhere (release binary, `ci-green`, `class-cards --check`, `proofs --gate`, `loop-stats --guard`). Before sampling, copy the git-ignored `tests/db/ca.crt`, `.agent-tmp/impact-reads.json`, `.loop/check-reads.json` and `.loop/proof-reads.json` into the worktree.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. To rehearse uncommitted work, run `git diff > .agent-tmp/rehearsal.patch` in main. Then in the worktree run `git reset -q --hard <main head>; git clean -fdq; git apply ../../rehearsal.patch`, and run the script. After that, `bun .agent-tmp/cutover-sample.ts <tree>`. `main` is frozen, and tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: the loop becomes a driver.** One file set: `tools/nv/cmd/loop.ts`, `tools/nv/driver/accept.ts`, `tools/nv/test/accept.test.ts`, read against `tools/loop.py`.

- [ ] **`bun nv loop --goal-only [--full]`, the whole sweep** (`tools/nv/driver/accept.ts:18`, beside `runChecks` at `tools/nv/cmd/loop.ts:224`): port `Goal._check`'s tier order from `tools/loop.py:4079` (setup, floor fixtures, cargo and command checks, goal fixtures, overlap, release), stop at the first red unless collecting, and print `NOT GREEN:` or `GOAL REACHED` with `also red:` lines as `all_reds` does at `tools/loop.py:4320`. Memoize each green check under its unit key from `tools/nv/keys/checks.ts:298` (`units`, then `keyOf` over `parts(tree)`) in a memo of the new driver's own, and let `--full` ignore it. Stage 9 needs this green under the new driver before the cutover commit. Prove it on one stage with `--stage`, never the whole plan.
- [ ] **The session turn** (`tools/loop.py:8447` `turn`, `tools/loop.py:5996` `run_session`): run `nv orient`'s pack plus the session prompt through `claude -p` with stream JSON, feed the stream to `tools/nv/driver/status.ts:100`'s `Session` for the status row, then the sweep, the ledger line and `respawn`'s exit `75`. Goal prose stage 8 owns the flags it keeps.
- [ ] **Run the cutover for real** (`tools/loop.py:1`, which its step 4 turns into a shim): once the driver runs a session, rehearse once more, run `cutover-sample.ts`, then `bun .agent-tmp/cutover.ts --root . --for-real` on main, read a sample of the diff, and commit it. Answer the 38 unresolved gap citations first.

## Backlog

- A ported gate keys on everything, since `.loop/check-reads.json` is keyed by the Python argv and nothing observes a `bun nv` run. An `nv` observation would narrow it again (goal prose stage 6).
- The old proofs name in 7 Python tools' comments goes when Stage 12 deletes them (goal prose stage 12).
- The notice's "Regenerate with" line changes with the Stage 12 slice that deletes `tools/gen-attribution.py`.
- The Python driver's floor gate, audit cadence, WSL leg, valgrind sweep and fuzz campaign are not in `driver/accept.ts` yet (`tools/nv/driver/accept.ts:18`), and goal prose stage 8 says they stay unchanged.
