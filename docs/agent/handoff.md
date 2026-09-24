# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

`bun nv gen-attribution` now does all three modes: writing `THIRD-PARTY-LICENSES.txt`, `--check`, and `--check-c-deps`. Its parity group matches on 7 cases, including `[]` and `["--check"]`, and is `sequential` because both programs write the same notice at the root. The write is byte-identical to the Python tool's. CI's attribution job runs `bun nv gen-attribution --check` behind `oven-sh/setup-bun` pinned to v2.2.0, with `bun-version-file: package.json` (whether that reads `engines.bun` is *not checked*; CI cannot run while billing blocks it). Every prose citation of the ledger names `tools/nv/cmd/gen-attribution.ts`.

Two things still name the Python tool on purpose, because parity needs them identical until it is deleted: the notice's own "Regenerate with" line (`tools/nv/cmd/gen-attribution.ts:631`), and the ledger copy inside `tools/gen-attribution.py`. `rule:packaging/a-c-dependency-answers-two-questions` says `--check-c-deps` fails CI, and no CI job runs it (*not checked* whether a floor check does).

The cutover's step 3b maps `try` and `gen-attribution` in `TOOLS`. `audit checks` still names 8 Python tools that say the old proofs name, and those are deleted at the cutover, not ported. It also reports 38 gap citations by position that name no gap today (`.agent-tmp/cutover-unresolved-citations.txt`). In the rehearsal tree the `try --bundle` floor check fails only because git-ignored `tests/db/ca.crt` is absent there.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse uncommitted work: in the worktree, run `git reset -q --hard <main head>; git clean -fdq`, then `git apply` a patch from `git diff` in main, copy any new file across by hand, then run the script. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape.

`main` is frozen. Tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: the cutover rehearsal.** One file set: `.agent-tmp/cutover.ts`, `tools/nv/cmd/audit.ts`, `tools/nv/cmd/gen-attribution.ts`.

- [ ] **Teach the cutover the notice's regenerate line** (`tools/nv/cmd/gen-attribution.ts:631`): when step 2 deletes `tools/gen-attribution.py`, it rewrites that line and `choose`'s message to name `bun nv gen-attribution`, then runs `bun nv gen-attribution` so the committed notice matches. `rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`.
- [ ] **Rehearse the cutover again** (`tools/nv/cmd/audit.ts:122`): only the Python tools the cutover deletes should say the old name, and every floor check sampled by `cutover-sample.ts` should pass in the rehearsal tree.
- [ ] **Make `bun nv loop` a driver** (`tools/nv/cmd/loop.ts:21`): the last thing that keeps the cutover commit from landing. `rule:testing/feature-proofs` is not involved; the goal prose stage 8 owns the spec.

## Backlog

- The committed notice had gone stale against `Cargo.lock` with no gate noticing: CI is blocked, and no floor check runs `--check`. Worth a floor check at the cutover (goal prose stage 9).
- `--check-c-deps` runs in no CI job, though its rule says it fails CI (`rule:packaging/a-c-dependency-answers-two-questions`).
