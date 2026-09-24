# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, Stage 4's records are valid, and Stage 8's
checks are green. Stage 8's prose still owes `nv session --wrap`, `nv parity writers` and the
copy-free goal switch.** `bun nv check` prints `nv check: 0 findings`, and `bun nv render --check` is
current.

`bun nv chain`, `bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard`,
`bun nv splice`, `bun nv verify` and `bun nv orient` are written. `bun nv parity orient` is green:
its group compares only the pack's section heads, warnings, current item and the rest-of-group
numbers (`select` in `tools/nv/parity/groups.json`). `manifestFindings` in `tools/nv/cmd/orient.ts`
is the nv home of `orient.py`'s manifest audit, and `nv chain --check` runs it over every goal
record and side goal record. `nv orient` still gets the wrap skeleton from
`python tools/session.py --template` until `nv session` exists.

`tools/orient.py` stays until the Stage 9 cutover, because `chain.py`, `session.py` and the
driver import or run it. So do `chain.py` and `tools/verify_keys.py`. `main` is frozen. Tag
`pre-overhaul` is the rollback.

The playbook still has two homes: `docs/agent/playbook/` (read by `playbook.py`, `orient.py` and
`nv orient` through the importer) and `data/playbook/`. An edit to a bullet goes into both until
Stage 9.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and
`bun nv selftest` runs every tools test.

## Next group

**Stage 8: the writers and the driver** — one file set: `tools/nv/cmd/session.ts` (new), `tools/nv/parity/groups.json`, `tools/nv/cmd/orient.ts`. loop-goal.md § *Stage 8* specifies it: "`nv session --wrap` keeps the wrap file's shape … one parser reads it, and it applies every write as records, all or nothing".

- [ ] **`nv session --wrap` parses the wrap file.** Port `tools/session.py:225`'s `parse_wrap` into
      `tools/nv/cmd/session.ts`, keeping the `## <kind>:` sections, and refuse the same malformed
      input. Start with `--template` and `--check`, so `nv orient`'s closing block can drop its call
      to the Python tool.
- [ ] **The wrap's manifest gate uses `manifestFindings`.** Port `tools/session.py:1062` to call
      `manifestFindings` from `tools/nv/cmd/orient.ts` over the live goal's record.
- [ ] **`bun nv parity writers` exists.** Add a `writers` group to `tools/nv/parity/groups.json`
      with a `tree` (`tools/nv/cmd/parity.ts:44`), so one wrap applied by both tools to two copies gives the same files.

## Backlog

- Stage 9 cutover: delete `dossier.py`'s `--emit-goals`, `--check-goals` and fan-out flags, and `generated_by` in `tools/loop.py`.
- `chain.py`'s `--show`, `--retire`, `--set` and `--retitle` have no `nv chain` successor yet.
- `docs/agent/playbook/` has no renderer from `data/playbook/`.
- `lints.py` has no `nv` successor; `nv verify`'s `lints` step runs it.
- `tools/verify_keys.py` goes when `tools/loop.py:70` and `tools/impact.py` read the keys from `tools/nv/keys/` instead.
- `nv orient` drops `[context] spec`, which the goal record has no field for; no live goal names it.
