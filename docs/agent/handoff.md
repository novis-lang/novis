# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `bun nv gen-attribution` has only its `--check-c-deps` mode, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

`bun nv try` replaces `tools/try.py` whole, and its parity group matches on 11 cases, fixtures under `tools/nv/parity/fixtures/try-*`. The group is `sequential`, because both programs write the same `.agent-tmp/try-<stem>` files. `bun nv gen-attribution --check-c-deps` matches on 5 cases, and the ledger `C_DEPENDENCIES` now exists twice: in the Python tool and in `tools/nv/cmd/gen-attribution.ts`. The Python one goes when the tool is deleted. `Cargo.toml:714`, `:897` and `:930` and `rule:packaging/a-c-dependency-answers-two-questions` still name the Python file as the ledger's home, so they move in the deletion slice.

The cutover's step 3b maps `try` and `gen-attribution` in `TOOLS`. The rehearsal now prints `audit: no check runs python`. `audit checks` still names 8 Python tools that say the old proofs name, and those are deleted at the cutover, not ported. It also reports 38 gap citations by position that name no gap today (`.agent-tmp/cutover-unresolved-citations.txt`). In the rehearsal tree the `try --bundle` floor check fails only because git-ignored `tests/db/ca.crt` is absent there; it passes in the main tree.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse uncommitted work: in the worktree, run `git reset -q --hard <main head>; git clean -fdq`, then `git apply` a patch from `git diff` in main, copy any new file across by hand, then run the script. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape.

`main` is frozen. Tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: the notice half of `gen-attribution`.** One file set: `tools/gen-attribution.py`, `tools/nv/cmd/gen-attribution.ts`, `tools/nv/parity/groups.json` and `known.json`, and `.github/workflows/ci.yml`.

- [ ] **Port writing and `--check` of `THIRD-PARTY-LICENSES.txt`** (`tools/gen-attribution.py:767`): SPDX parsing (`tools/gen-attribution.py:260`), the license-file reader and `render` (`tools/gen-attribution.py:544`), into `tools/nv/cmd/gen-attribution.ts:271`, which refuses those modes today. The output must be byte-identical; add `[]` and `["--check"]` cases to the group, with a `tree` if the write case needs one. `rule:packaging/the-third-party-notice-is-generated-never-written-by-hand` owns the policy.
- [ ] **Point CI and the ledger's citations at the port** (`tools/gen-attribution.py:181`): `ci.yml` line 470, the root `Cargo.toml` at lines 714, 897 and 930, and `rule:packaging/a-c-dependency-answers-two-questions` name `tools/gen-attribution.py`'s `C_DEPENDENCIES`. The CI switch needs `setup-bun`, so it may belong to the cutover's CI step instead (see Backlog).
- [ ] **Rehearse the cutover again** (`tools/nv/cmd/audit.ts:122`): only the Python tools the cutover deletes should say the old name, and every floor check that ran `try.py` or `gen-attribution.py` should pass under `cutover-sample.ts --all-of`.

## Backlog

- `.github/workflows/ci.yml` still runs `python tools/ci-changes.py`, `db-matrix.py`, `lints.py`, `gen-attribution.py`, `check-migration.py`, `reference.py`, `rules.py`, `records.py` and `check-links.py`, and needs `setup-bun`. The floor check `the CI workflow runs the five-driver matrix` wants `python tools/db-matrix.py --all` in it, so its want line and the switch have to be settled together at the cutover (goal prose stage 9, `docs/agent/goals/122-tooling-overhaul.md`).
- `bun nv ci-green`'s `BOOKKEEPING` still lists `docs/agent/` and `docs/plan/` only. Once handoffs live under `data/goals/`, it needs that prefix too (`tools/nv/cmd/ci-green.ts:23`).
- `bun nv loop` is not a driver yet (`tools/nv/cmd/loop.ts:21`), and the cutover needs one.
- The cutover's 38 unresolved gap citations by position (`.agent-tmp/cutover-unresolved-citations.txt`) need a mapping or a manual pass before `--for-real`.
