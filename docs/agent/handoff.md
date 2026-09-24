# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

In the rehearsal the cutover leaves two findings: 38 gap citations by position that name no gap today (`.agent-tmp/cutover-unresolved-citations.txt`), and the loop is not yet a driver. `nv audit` reports only the old proofs name in 7 Python tools' comments (`chain`, `goal-switch`, `goals`, `owners`, `plan`, `relink`, `session`). Stage 12 deletes them (goal prose stage 12), so that check stays red until then. The cutover now deletes `tools/dossier.py` with its `proofs` parity group (step 2). Its new step 3c unlinks every markdown link to a home step 2 deleted and marks each such Python line `check-links:retired`, so `nv links` holds.

Of the 63 floor-check shapes `cutover-sample.ts` runs, 56 pass. The 7 that fail: `impact --probe` is a regression, covered by the first item below. `bench` ×2 need a release binary the worktree lacks. `ci-green`, `class-cards --check`, `proofs --gate` and `loop-stats --guard` are red on main too, owed by later goals or stages. Before sampling, copy the git-ignored `tests/db/ca.crt`, `.agent-tmp/impact-reads.json`, `.loop/check-reads.json` and `.loop/proof-reads.json` into the worktree. Without them, `try --bundle` and three impact probes fail for no real reason.

`bun nv impact` and `bun nv why` read the live goal's record once `docs/agent/loop-goal.toml` is gone (`tools/nv/keys/checks.ts`, `goalChecks`). The notice's "Regenerate with" line keeps naming the Python tool, because the parity group compares both notices byte for byte. The Stage 12 slice that deletes `tools/gen-attribution.py` changes the line and regenerates the notice. The comment at `tools/nv/cmd/gen-attribution.ts:629` says so.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. To rehearse uncommitted work, run `git diff > .agent-tmp/rehearsal.patch` in main. Then in the worktree run `git reset -q --hard <main head>; git clean -fdq; git apply ../../rehearsal.patch`, and run the script. After that, `bun .agent-tmp/cutover-sample.ts <tree>`. `main` is frozen, and tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: the cutover rehearsal.** One file set: `tools/nv/keys/checks.ts`, `.agent-tmp/cutover.ts`, `tools/nv/cmd/loop.ts`.

- [ ] **Key a ported check by its `bun nv` argv** (`tools/nv/keys/checks.ts:424`, `tools/nv/keys/checks.ts:492`): `runs(argv, DB_MATRIX)` knows only `tools/db-matrix.py`, so after step 3b rewrites the argv to `bun nv db-matrix` it is keyed as a plain `nv` unit. Every probe with a `^db-matrix: ` keep or rerun pattern then fails in the rehearsal. Match both argvs and read the suites from whichever program exists. Check the same for each Python-gate role the argv rewrite reaches (`observed` is keyed by argv too, at `tools/nv/keys/checks.ts:77`). `rule:testing/feature-proofs` is not involved; the goal prose stage 6 owns the key.
- [ ] **Rehearse the cutover again** (`tools/nv/keys/checks.ts:424`): `impact --probe` must hold in the worktree. The copy of the ignored read records could move into `cutover-sample.ts` itself.
- [ ] **Make `bun nv loop` a driver** (`tools/nv/cmd/loop.ts:21`): the last thing that keeps the cutover commit from landing. The goal prose stage 8 owns the spec.

## Backlog

- The committed notice went stale against `Cargo.lock`, and no gate noticed. CI is blocked, and no floor check runs `gen-attribution --check`. Worth a floor check at the cutover (goal prose stage 9).
- `--check-c-deps` runs in no CI job, though its rule says it fails CI (`rule:packaging/a-c-dependency-answers-two-questions`).
- Three Rust comments name `tools/dossier.py`, which the cutover deletes: `crates/nvs-cli/src/main.rs:318`, `crates/nvs-cli/src/main.rs:2737`, `crates/nvs-stdlib/tests/spec_registry_coverage.rs:411`. Stage 12's prose rewrite (goal prose stage 12).
- Step 3c leaves link text as plain words, or as a file name in backticks, in `CONTRIBUTING.md`, `docs/setup.md` and the agent docs. Stage 12 rewrites those docs whole.
