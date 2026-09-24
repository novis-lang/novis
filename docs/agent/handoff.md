# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, Stage 4's records are valid, and Stage 8 is in
progress.** `bun nv check` prints `nv check: 0 findings`, and `bun nv render --check` is current.

`bun nv chain`, `bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard`,
`bun nv splice`, `bun nv verify` and now `bun nv orient` are written. `nv orient` prints the same
sections, items and warnings as `python tools/orient.py`, plus a `Goal <slug>, N of M on the chain`
line. It reads the goal, handoff, plan status and playbook through their importers from the legacy
files the Python writers still keep current (its module doc says which), and it gets the wrap
skeleton from `python tools/session.py --template` until `nv session` exists. Its parity group is
registered but red: `nv parity` compares line by line, and the packs differ on purpose in tool names,
unwrapped checklist items and the position line.

`tools/verify_keys.py` stays while `tools/loop.py` and `tools/impact.py` import it. `chain.py` and
`orient.py` stay until the Stage 9 cutover. `main` is frozen. Tag `pre-overhaul` is the rollback.

The playbook still has two homes: `docs/agent/playbook/` (read by `playbook.py`, `orient.py` and
`nv orient` through the importer) and `data/playbook/`. An edit to a bullet goes into both until
Stage 9.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and
`bun nv selftest` runs every tools test.

## Next group

**Stage 8: the writers and the driver** — one file set: `tools/nv/cmd/parity.ts`, `tools/nv/parity/groups.json`, `tools/nv/parity/known.json`, `tools/nv/cmd/orient.ts`. loop-goal.md § *Stage 8* specifies it: "the packs for the live goal carry the same sections and items".

- [ ] **`bun nv parity orient` is green.** Give a parity group a way to compare only selected lines,
      for example a `select` pattern next to `unordered` in `tools/nv/cmd/parity.ts:31`'s `Group`.
      Set it on the `orient` group in `tools/nv/parity/groups.json` to the section heads, warnings,
      `-- YOUR ITEM` and the rest-of-group numbers (`^(== |!! |-- YOUR|   \[\d+\])`). Declare in
      `tools/nv/parity/known.json` what still differs, with its reason. That covers the
      `orient.py` -> `nv orient` warning prefix and the `[2]` line, which nv prints whole from the
      unwrapped record text. Then `tools/orient.py` may be deleted in the slice after.
- [ ] **`manifest_findings` has an nv home.** Port `tools/orient.py:1337` to an export of
      `tools/nv/cmd/orient.ts` that `nv chain --check` and a later `nv session` can call. It counts
      stages from the goal record's `stages`, replacing the `## Stage` heading count at
      `tools/orient.py:1466`.

## Backlog

- Stage 9 cutover: delete `dossier.py`'s `--emit-goals`, `--check-goals` and fan-out flags, and `generated_by` in `tools/loop.py`.
- `chain.py`'s `--show`, `--retire`, `--set` and `--retitle` have no `nv chain` successor yet.
- `docs/agent/playbook/` has no renderer from `data/playbook/`.
- `lints.py` has no `nv` successor; `nv verify`'s `lints` step runs it.
- `tools/verify_keys.py` goes when `tools/loop.py:70` and `tools/impact.py` read the keys from `tools/nv/keys/` instead.
- `nv orient` drops `[context] spec`, which the goal record has no field for; no live goal names it.
