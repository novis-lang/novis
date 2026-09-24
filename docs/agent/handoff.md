# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, Stage 4's records are valid, and Stage 8's
checks are green. Stage 8's prose still owes the `nv session --wrap` apply half, `nv parity writers`
and the copy-free goal switch.** `bun nv check` prints `nv check: 0 findings`, and `bun nv render
--check` is current.

`bun nv session` exists. `parseWrap` and `parseEdits` in `tools/nv/cmd/session.ts` are the wrap
file's reader and refuse what `session.py`'s parser refuses. `--template` is byte-identical to
`python tools/session.py --template`, and `nv orient` now takes its skeleton from it. `--check`
matches Python's except that it names nv tools and keeps the first `git status --short` line's
leading space. `--counts` and `--scrub` are ported. `--wrap F` parses the file with `parseWrap`,
then still hands it to `python tools/session.py --wrap`, which validates and applies.

`bun nv chain`, `loop --list`, `loop --goal`, `bg`, `guard`, `splice`, `verify`, `orient` and
`session` are written. `bun nv parity orient` is green. `manifestFindings` in
`tools/nv/cmd/orient.ts` is the nv home of `orient.py`'s manifest audit.

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

**Stage 8: the writers and the driver** — one file set: `tools/nv/cmd/session.ts`, `tools/nv/parity/groups.json`, `tools/nv/cmd/orient.ts`. loop-goal.md § *Stage 8* specifies it: "`nv session --wrap` keeps the wrap file's shape … one parser reads it, and it applies every write as records, all or nothing".

- [ ] **`bun nv parity writers` exists.** Add a `writers` group to `tools/nv/parity/groups.json:1`
      comparing `python tools/session.py` with `bun tools/nv/main.ts session` on `--template`,
      `--counts`, `--check` and a malformed `--wrap F --dry-run` fixture. Declare the differences:
      `--check` names nv tools (`tools/nv/cmd/session.ts:488`), and the malformed-wrap refusal is
      worded by nv.
- [ ] **The wrap's manifest gate uses `manifestFindings`.** Port `tools/session.py:1062`'s
      `manifest_findings` into `tools/nv/cmd/session.ts`, calling `manifestFindings` at
      `tools/nv/cmd/orient.ts:1512` rather than running `orient.py --audit`.
- [ ] **`nv session --wrap` validates and applies.** Port `validate` (`tools/session.py:517`),
      the `apply_*` functions (`tools/session.py:1238`) and `wrap` (`tools/session.py:1471`), then
      drop the pass-through to Python at `tools/nv/cmd/session.ts:701`. The plan fields are written
      as the `plan_status` record and re-rendered, all or nothing.

## Backlog

- The copy-free goal switch, loop-goal.md § *Stage 8*.
- Stage 9, the cutover: `bun nv audit goals`, `audit checks` and `audit eol` do not exist yet; the
  driver's acceptance check fails on the first of them.
- Retire `docs/agent/playbook/` after Stage 9, so a bullet has one home.
