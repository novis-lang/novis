# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard`, `bun nv splice` and
`bun nv verify` are written. `tools/verify.py` is deleted, its parity group is retired, and every
citation outside the frozen records names `bun nv verify` or `nv verify`; the two floor checks run
`bun nv verify --list`. Its module doc's *Why* sections are cited as
`git show pre-overhaul:tools/verify.py`. `tools/verify_keys.py` stays while `tools/loop.py` and
`tools/impact.py` import it. `chain.py` stays until the Stage 9 cutover, because its `--show`,
`--retire`, `--set` and `--retitle` have no successor yet. `main` is frozen. Tag `pre-overhaul` is
the rollback.

The playbook still has two homes: `docs/agent/playbook/` (read by `playbook.py` and `orient.py`) and
`data/playbook/` (what `nv import` made). An edit to a bullet goes into both until Stage 9.

**The red acceptance check is `every record is valid and every reference resolves`.** `nv check`'s
80 findings are all "a stage says what it does", because `data/goals/*.json` was imported before
`summary` existed, and `nv import --write` refuses while `impact_probes` has no importer.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and
`bun nv selftest` runs every tools test.

## Next group

**Stage 4: the records are valid** — one file set: `tools/nv/cmd/import.ts`, `tools/nv/import/`,
`tools/nv/schema/impact-probes.ts` and `data/goals/*.json`. The goal's Stage 4 section and the
check at the head of the pack specify it.

- [ ] **Close `nv check`'s 80 findings.** Either give `nv import` an `impact_probes` importer, so
      `--write` stops refusing (`tools/nv/cmd/import.ts:149`) and rewrites `data/goals/*.json` with
      the `summary` that `tools/nv/import/goals.ts:137` already reads, or add `summary` to the goal
      records by a script. The check is `tools/nv/schema/goal.ts:103`; the record type is
      `tools/nv/schema/impact-probes.ts`. Read `bun nv import --check` first: it names what is unread.
      Done when `bun nv check` prints `nv check: 0 findings`.

## Backlog

- Stage 9 cutover: delete `dossier.py`'s `--emit-goals`, `--check-goals` and fan-out flags, and
  `generated_by` in `tools/loop.py`.
- `chain.py`'s `--show`, `--retire`, `--set` and `--retitle` have no `nv chain` successor yet.
- `docs/agent/playbook/` has no renderer from `data/playbook/`.
- `lints.py` has no `nv` successor; `nv verify`'s `lints` step runs it.
- `tools/verify_keys.py` goes when `tools/loop.py:70` and `tools/impact.py` read the keys from
  `tools/nv/keys/` instead.
