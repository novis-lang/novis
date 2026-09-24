# Handoff

## State

**Goal `tooling-overhaul`, Stage 9.** The cutover script `.agent-tmp/cutover.ts` is on disk. It is git-ignored and never committed. It is rehearsed in the scratch worktree `.agent-tmp/worktrees/cutover-rehearsal` (branch `cutover-rehearsal`, never merged), reset to main's head before each run. **The cutover commit cannot land yet**: `audit checks` still names the checks that run Python, and `bun nv loop` is not a driver. It takes `--list` and `--goal` alone (`tools/nv/cmd/loop.ts:21`).

`bun nv bench` replaces `tools/bench.py` (`tools/nv/cmd/bench.ts`), all three modes: the suite, `--warm-start` and `--serve-vs-fpm`. `bun nv parity bench` matches on 19 of 19 cases, and one of them runs `--check` over two real cases with nvs and PHP. `--warm-start --max-work-ms 6` and `--serve-vs-fpm --record` both ran here and printed every `want` line of their two checks. The Bun generator spends less per request than Python's did, so `nvs serve` reads higher than under `bench.py`; the module doc says so. `parseArgs` (`tools/nv/lib/py.ts`) now takes `positionals`, argparse's one `nargs="*"` positional.

Step 3b's `TOOLS` table maps `bench` and `db-matrix`, and the rehearsal confirmed it: neither is on the unported list any more. `audit goals` printed both of its lines. `audit checks` named 12 checks that still run Python, which are 6 checks with each floor check listed under two goals. It also named 8 Python tools that still say the old name, and **the stage-9 check itself**: its name, "no check runs Python, and no check, directive or tool says dossier", contains the old word, so the audit flags it and the check cannot pass under that name.

Run the cutover with `bun .agent-tmp/cutover.ts --root <tree>`. It refuses the main tree unless you pass `--for-real`. To rehearse uncommitted work: in the worktree, run `git reset -q --hard <main head>; git clean -fdq`, then `git apply` a patch from `git diff` in main, copy any new file across by hand, then run the script. `bun .agent-tmp/cutover-sample.ts <tree> [--all-of <nv command>]` runs one floor check per argv shape.

`main` is frozen. Tag `pre-overhaul` is the rollback.

## Next group

**Stage 9: the checks that still run Python.** One file set: the live goal's checks in `docs/agent/loop-goal.toml` (and their records under `data/goals/`), the cutover script's step 3b `TOOLS` and `RETARGETED` tables, `tools/nv/parity/groups.json`, and the Python tools they run. The rehearsal is the test, and `bun nv audit checks` in the rehearsed tree lists every one. `tools/nv/cmd/bench.ts` is the shape of a port: an argparse-compatible `parseArgs` call, a parity group, and one `TOOLS` entry.

- [ ] **The stage-9 check names the old word** (`docs/agent/loop-goal.toml:13762`): rename it so its own name does not say the old proofs name, in its record under `data/goals/` and in the toml, with `bun nv goal` where it can write it. `tools/nv/cmd/audit.ts:122` reads check names, and the `want` lines stay.
- [ ] **Six one-off Python checks** (`docs/agent/loop-goal.toml:2362`, `docs/agent/loop-goal.toml:2879`, `docs/agent/loop-goal.toml:10174`): `tools/try.py --bundle` (`nvs build --compile`), `tools/gen-attribution.py --check-c-deps` (the C-dependency ledger), `ci.yml`'s suites and the two lane-table checks (`python -c` and `ci-changes` between them), `ci-green` and `class-cards`. The rehearsal's "has no `bun nv` command yet" lines count them by tool. Port each to a `bun nv` command or a `RETARGETED` entry. The floor's `want` lines stay.
- [ ] **Rehearse the cutover** (`tools/nv/cmd/audit.ts:122`) after the ports: `audit checks` in the rehearsed tree should name no check that runs Python.

## Backlog

- `bun nv loop` as the driver (`tools/nv/cmd/loop.ts:21`) — the cutover's shim needs it; goal prose stage 8.
- 8 Python tools say the old name (`tools/chain.py`, `tools/goals.py`, `tools/plan.py` and five more); each is deleted once its replacement's parity group is green, per the goal's § *Standing decisions*.
- `tools/bench.py` is deleted in a slice after the cutover lands, now that `bun nv parity bench` is green.
