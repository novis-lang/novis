# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

In the rehearsal the cutover leaves two findings: 38 gap citations by position that name no gap today (`.agent-tmp/cutover-unresolved-citations.txt`), and the loop is not yet a driver. `nv audit` reports only the old proofs name in 7 Python tools' comments (`chain`, `goal-switch`, `goals`, `owners`, `plan`, `relink`, `session`). Stage 12 deletes them (goal prose stage 12), so that check stays red until then.

`bun nv impact --probe` holds in the rehearsal: every one of the 11 probes. A `bun nv db-matrix` check is keyed like the Python script it replaces (`tools/nv/keys/checks.ts`, `dbMatrixOf`), and step 3b rewrites the probe patterns that named a ported gate (`^gate: ` to `^nv: `, the old prefix to `proofs:`, and the emitter's walkable check to its `chain:` name). A ported gate keys on everything. `.loop/check-reads.json` is keyed by the Python argv, so its narrowing does not carry over. That is wider, never narrower, and it is in the backlog.

Of the 63 floor-check shapes `cutover-sample.ts` runs, 57 pass. The 6 that fail are not regressions. `bench` ×2 need a release binary the worktree lacks. `ci-green`, `class-cards --check`, `proofs --gate` and `loop-stats --guard` are red on main too, owed by later goals or stages. Before sampling, copy the git-ignored `tests/db/ca.crt`, `.agent-tmp/impact-reads.json`, `.loop/check-reads.json` and `.loop/proof-reads.json` into the worktree.

The notice's "Regenerate with" line keeps naming the Python tool until the Stage 12 slice that deletes `tools/gen-attribution.py` (comment at `tools/nv/cmd/gen-attribution.ts:629`).

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. To rehearse uncommitted work, run `git diff > .agent-tmp/rehearsal.patch` in main. Then in the worktree run `git reset -q --hard <main head>; git clean -fdq; git apply ../../rehearsal.patch`, and run the script. After that, `bun .agent-tmp/cutover-sample.ts <tree>`. `main` is frozen, and tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: the loop becomes a driver.** One file set: `tools/nv/cmd/loop.ts`, `tools/loop.py`, `.agent-tmp/cutover.ts`.

- [ ] **Make `bun nv loop` a driver** (`tools/nv/cmd/loop.ts:21`): it takes `--list` and `--goal` alone, so the shim step 4 writes starts nothing that runs a session. It is the last thing that keeps the cutover commit from landing. The goal prose stage 8 owns the driver's shape, and the status line is the user's spec in that stage. Port what `tools/loop.py` does per turn, and prove it with single commands, never a sweep.
- [ ] **Run the cutover for real** (`tools/loop.py:1`, which its step 4 turns into a shim): once the driver runs a session, rehearse once more, run `cutover-sample.ts`, then run `bun .agent-tmp/cutover.ts --root . --for-real` on main, read a sample of the diff, and commit it. The two findings the script prints must be answered first: the 38 unresolved gap citations, and the driver.

## Backlog

- A ported gate keys on everything, since `.loop/check-reads.json` is keyed by the Python argv and nothing observes a `bun nv` run. An `nv` observation would narrow it again (goal prose stage 6).
- The old proofs name in 7 Python tools' comments goes when Stage 12 deletes them (goal prose stage 12).
- The notice's "Regenerate with" line changes with the Stage 12 slice that deletes `tools/gen-attribution.py`.
