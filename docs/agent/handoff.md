# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard`, `bun nv splice` and
`bun nv verify` are written. `chain.py` stays until the Stage 9 cutover, because its `--show`,
`--retire`, `--set` and `--retitle` have no successor yet. `main` is frozen. Tag `pre-overhaul` is the
rollback.

**`bun nv parity verify` matches 12 of 12.** The cases are `--list` in four shapes, the flag conflicts
and the argument errors. `nv verify` now parses through `parseArgs` in `tools/nv/lib/py.ts`, so its
usage errors are argparse's. The run itself is not compared, because it prints timings and each
program keeps its own cache. The commit that added the group lists the differences in the run, each
with its reason: the green cache shape, the TS binary keys in `verify-test-green.json`, no `compiled`
field in `impact-reads.json`, and the case trees keyed at `card`. `tools/verify.py` still exists, and
nothing calls `bun nv verify` yet.

The playbook still has two homes: `docs/agent/playbook/` (read by `playbook.py` and `orient.py`) and
`data/playbook/` (what `nv import` made). An edit to a bullet goes into both until Stage 9.
`nv check`'s 80 findings are all "a stage says what it does", because `data/goals/*.json` was imported
before `summary` existed and `nv import --write` refuses while `impact_probes` has no importer. So the
acceptance check `every record is valid and every reference resolves` stays red until that lands (see
Backlog).

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and
`bun nv selftest` runs every tools test.

## Next group

**Stage 8: `nv verify` replaces `verify.py`** — one file set: `tools/loop.py`, `tools/session.py`,
`tools/orient.py`. The goal's Stage 8 bullet "`nv verify` takes over `verify.py`'s steps unchanged,
over the Stage 6 keys" specifies it. `tools/verify.py` is cited from 345 files, so this is three slices.

- [ ] **Point the driver at `bun nv verify`.** `VerifyWatch` at `tools/loop.py:1045` reads
      `verify.py`'s progress file, the rustdoc gate calls `verify.py --doc`, and `crate_tests` at
      `tools/loop.py:3036` reads `verify-test-green.json`, whose keys are now the TS binary keys. Port
      all three together, or the driver reruns every binary verify already ran.
- [ ] **Point `tools/session.py` and `tools/orient.py` at `bun nv verify`.** Anchors:
      `tools/session.py:463` and `tools/session.py:470` (the wrap's verify-ran check), and the pack's
      "WHEN YOU ARE DONE" block in `tools/orient.py`.
- [ ] **Rewrite the remaining citations with a script under `.agent-tmp/`, and delete
      `tools/verify.py`.** `tools/verify_keys.py` stays while `tools/loop.py:70` and `tools/impact.py`
      import it.

## Backlog

- `nv import` needs an `impact_probes` importer. Or rewrite `data/goals/*.json` with `summary`, which
  closes `nv check`'s 80 findings (`tools/nv/cmd/import.ts`).
- Stage 9 cutover: delete `dossier.py`'s `--emit-goals`, `--check-goals` and fan-out flags, and
  `generated_by` in `tools/loop.py`.
- `chain.py`'s `--show`, `--retire`, `--set` and `--retitle` have no `nv chain` successor yet.
- `docs/agent/playbook/` has no renderer from `data/playbook/`.
- `lints.py` has no `nv` successor. `verify`'s parity group declares its step command on the `nv` side
  until it has one (`tools/nv/parity/known.json`).
