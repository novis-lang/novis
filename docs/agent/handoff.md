# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, Stage 4's records are valid, and Stage 8's
checks are green. Stage 8's prose still owes the `nv session --wrap` apply half and the copy-free
goal switch.** `bun nv check` prints `nv check: 0 findings`, and `bun nv render --check` is current.
The driver's red check is Stage 9's `bun nv audit goals`, which is the cutover and is not started.

`bun nv session` exists. `parseWrap` and `parseEdits` in `tools/nv/cmd/session.ts` are the wrap
file's reader, and `validate` there is `tools/session.py`'s `validate`, ported: every section kind,
the no-commit refusal, body links, `rule:` citations, goal numbers, the link gate, the three tree
gates and `manifestProblems`. `--wrap F` runs it and refuses with every problem at once. Two parts
are not ported yet: the playbook lead-in collision check (`playbook_collisions`), and the playbook
fragment file names (`writtenPaths` names a bullet by its section's directory). A valid wrap is
still handed to `python tools/session.py --wrap`, which validates again and applies.
`bun nv parity writers` is 7 of 7, with its differences in `tools/nv/parity/known.json`. Two of
those entries are reported `unused`: they declare message differences in branches no fixture
reaches (a goal number, a broken link), because a fixture carrying either would turn a floor check red.

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

- [ ] **`nv session --wrap` applies.** Port the `apply_*` functions (`tools/session.py:1238`) and
      `playbook_targets` (`tools/session.py:1342`) into `tools/nv/cmd/session.ts`, and replace the
      pass-through at `tools/nv/cmd/session.ts:1215`. Make `writtenPaths`
      (`tools/nv/cmd/session.ts:759`) name each bullet's own fragment file once `playbook_targets`
      is ported. A playbook bullet is written to `data/playbook/` too while the playbook has two homes.
      Parity: a valid wrap under `--dry-run` prints the same `would apply` lines on both sides.
- [ ] **The collision check.** Port `playbook_collisions` (`tools/session.py:1315`) into
      `validatePlaybook` (`tools/nv/cmd/session.ts:683`), reading `sliceBullets` in
      `tools/nv/cmd/orient.ts:1129`.

## Backlog

- The copy-free goal switch: Stage 8's prose in `docs/agent/loop-goal.md`.
- Stage 9, the cutover: `bun nv audit goals`, `audit checks`, `audit eol` (loop-goal.toml Stage 9).
