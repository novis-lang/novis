# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk, git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. `main` is frozen, and tag `pre-overhaul` is the rollback.

`bun nv loop` with no mode is one turn of the driver (`tools/nv/cmd/loop.ts`). `NOVIS_LOOP_CLAUDE` names a command to run in place of `claude`. The stand-in `.agent-tmp/stand-in.ts` (git-ignored) plays script N on its Nth call, counted in `.loop/stand-in-count`: 1 is the usage wall a minute ahead with exit 1, 2 writes `tools/nv/stand-in-probe.txt` and exits 0 unwrapped, 3 and later write `CONTINUE stand-in` and exit 0. **One turn was rehearsed on the cutover tree** with `NOVIS_LOOP_CLAUDE="bun D:/mwl/.agent-tmp/stand-in.ts" bun nv loop --max-sessions 1`. The wall was waited out, the next session ran in the same turn, and the unwrapped write was swept into a `wip(loop)` commit. The ledger lines read correctly.

That turn's sweep stopped at the floor's `examples/http.nvs`: the new driver never started the 8099 origin `loop.py` holds up. `tools/nv/driver/origin.ts` is that origin now, held around `--run` and every sweep in `tools/nv/cmd/loop.ts`, and `bun nv loop --run --name examples/http.nvs` is green on main. **No sweep under `nv loop` has run past that fixture yet**, so the next red of the floor under the new driver is not known.

What the turn still does not do is listed in `loop.ts`'s header, and what the sweep lacks is in `tools/nv/driver/accept.ts`'s header. In the rehearsal the cutover leaves 38 gap citations by position that name no gap today (`.agent-tmp/cutover-unresolved-citations.txt`). `nv audit checks` stays red on the old proofs name in 7 Python tools' comments until Stage 12 deletes them. Before sampling, copy the git-ignored `tests/db/ca.crt`, `.agent-tmp/impact-reads.json`, `.loop/check-reads.json` and `.loop/proof-reads.json` into the worktree. Rehearse with `git diff > .agent-tmp/rehearsal.patch` in main, then in the worktree `git reset -q --hard <main head>; git clean -fdq; git apply ../../rehearsal.patch`, `bun .agent-tmp/cutover.ts --root <tree>`, commit the result there, and `bun .agent-tmp/cutover-sample.ts <tree>`.

## Next group

**Stage 9: the driver is safe to leave unattended.** One file set: `tools/nv/cmd/loop.ts`, `tools/nv/driver/accept.ts`, `tools/nv/driver/origin.ts`, read against `tools/loop.py`.

- [ ] **Sweep the floor once under the new driver** in the cutover worktree: reset it, run the cutover, commit, delete `.loop/stand-in-count`, write `3` into it so the stand-in wraps at once, and run one turn with `NOVIS_LOOP_CLAUDE` as above. Fix each red that `loop.py`'s sweep does not show, the way the origin was fixed: find what `loop.py` holds around its sweep (`tools/loop.py:4065`) that `sweepOver` at `tools/nv/cmd/loop.ts:340` does not.
- [ ] **Run the cutover for real** (`tools/loop.py:1`, which its step 4 turns into a shim that runs `bun nv loop` with the same arguments): rehearse once more, run it on main, and have `bun nv loop --goal-only` green before the commit.

## Backlog

- `examples/http.nvs`'s comment and `tools/origin.py` name the Python origin; when Stage 12 deletes `origin.py`, the comment names `bun tools/nv/driver/origin.ts` — `tools/nv/driver/origin.ts` header.
- Overload retries that cost no attempt, and rejoining a dropped stream with `claude --resume` (`tools/loop.py:7165`, `tools/loop.py:7207`); today both count as a CLI failure — `tools/nv/cmd/loop.ts` header.
- The chain switch after a reached goal (`tools/loop.py:7383`), before any goal after `tooling-overhaul` runs under the new driver — `tools/nv/cmd/loop.ts` header.
- The goal-end doc and owner gates in the turn (`tools/loop.py:7370`) — `tools/nv/cmd/loop.ts` header.
- The sweep's missing legs: verify-record reuse, batched proofs, WSL (with its own origin), valgrind — `tools/nv/driver/accept.ts` header.
- The 38 unresolved gap citations the cutover leaves — `.agent-tmp/cutover-unresolved-citations.txt`, goal prose stage 9.
