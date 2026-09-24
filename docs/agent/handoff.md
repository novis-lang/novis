# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, Stage 4's records are valid, and Stage 8's
checks are green. Stage 8's prose still owes the `nv session --wrap` validate and apply halves and
the copy-free goal switch.** `bun nv check` prints `nv check: 0 findings`, and `bun nv render
--check` is current. `python tools/check-links.py` is green again.

`bun nv session` exists. `parseWrap` and `parseEdits` in `tools/nv/cmd/session.ts` are the wrap
file's reader. `--template`, `--counts`, `--check` and a malformed `--wrap F --dry-run` match
Python: `bun nv parity writers` is 4 of 4, with its differences declared in
`tools/nv/parity/known.json`. `--wrap F` refuses a malformed file and a live manifest that
`manifestProblems` refuses (both copies: `data/goals/<slug>.json` and `docs/agent/loop-goal.toml`),
exiting 1 the way Python does. Everything else it still hands to `python tools/session.py --wrap`.

`bun nv chain`, `loop --list`, `loop --goal`, `bg`, `guard`, `splice`, `verify`, `orient` and
`session` are written. `bun nv parity orient` and `bun nv parity writers` are green.

`tools/orient.py`, `chain.py`, `session.py` and `tools/verify_keys.py` stay until the Stage 9
cutover. `main` is frozen. Tag `pre-overhaul` is the rollback.

The playbook still has two homes: `docs/agent/playbook/` (read by `playbook.py`, `orient.py` and
`nv orient` through the importer) and `data/playbook/`. An edit to a bullet goes into both until
Stage 9.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and
`bun nv selftest` runs every tools test.

## Next group

**Stage 8: the writers and the driver** — one file set: `tools/nv/cmd/session.ts`, `tools/session.py`, `tools/nv/parity/groups.json`. loop-goal.md § *Stage 8* specifies it: "`nv session --wrap` keeps the wrap file's shape … one parser reads it, and it applies every write as records, all or nothing".

- [ ] **`nv session --wrap` validates.** Port `validate` (`tools/session.py:517`) into
      `tools/nv/cmd/session.ts`, folding `manifestProblems` in where Python calls
      `manifest_findings`, and run it before the pass-through at `tools/nv/cmd/session.ts:757`.
      Add a `writers` parity case with a well-formed wrap that `validate` refuses (an unknown plan
      field, a subject over 120 characters) under `tools/nv/parity/fixtures/`.
- [ ] **`nv session --wrap` applies.** Port the `apply_*` functions (`tools/session.py:1238`) and
      `wrap` (`tools/session.py:1471`), then drop the pass-through at `tools/nv/cmd/session.ts:757`.
      The plan fields are written as the `plan_status` record and re-rendered, all or nothing.

## Backlog

- The copy-free goal switch, loop-goal.md § *Stage 8*.
- Stage 9, the cutover: `bun nv audit goals`, `audit checks` and `audit eol` do not exist yet; the
  driver's acceptance check fails on the first of them.
- Retire `docs/agent/playbook/` after Stage 9, so a bullet has one home.
