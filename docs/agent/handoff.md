# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 is under way.** `bun nv
parity <group>` compares a Python tool with its `nv` replacement over the cases in
`tools/nv/parity/groups.json`, ignoring only the rewrites `tools/nv/parity/known.json` declares and,
per group, an `unordered` entry pattern. Five groups match on every case: `brief` (7 of 7),
`owners` (12 of 12), `peek` (16 of 16), `plan` (12 of 12: `--show`, `--get`, `--past`, `--stale`)
and `records` (1 of 1: `--stats`). `tools/nv/lib/chain.ts` is the chain's reader: its order, the
installed goal (matched on `docs/agent/loop-goal.md`'s H1) and the walked set.
**`data/` is a snapshot, not yet the authority.** The legacy homes still are. Re-run `bun nv import
--write` before a stage reads `data/` as the truth, and at the cutover. `main` is still frozen, and
the tag `pre-overhaul` is the rollback point. The driver's red Stage 6 check (`nv impact --probe`)
is an unbuilt command, not a regression.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green.

## Next group

**Stage 5: the read-only tools, proven by parity** — one file set: `tools/nv/cmd/**`,
`tools/nv/parity/**`, `tools/nv/main.ts`. loop-goal.md § *Stage 5* is the spec: parity is owed for
every mode the floor, a goal `.toml` or a process doc invokes. Each new command adds its group to
`tools/nv/parity/groups.json` and is registered in `tools/nv/main.ts:21`'s `COMMANDS`.

- [ ] **`bun nv plan --check`** (`tools/plan.py:645`, into `tools/nv/cmd/plan.ts:1`): the index is
      milestone records now, so the row-parse and title-cell checks become record-vs-H1 checks, and a
      `Carried by` cell becomes the record's `state`/`backlog`. `pastState()` in
      `tools/nv/cmd/plan.ts` is already the `done` rule. A goal tagged with a label (`dossier`,
      `post-parity`) has a null milestone in its record, so python's "in none (..., tagged ...)"
      clause needs a declared rewrite in `tools/nv/parity/known.json`. The field aim is read from the
      plan's leading comment (`tools/plan.py:201`).
- [ ] **`bun nv records --check`** (`tools/records.py:1`, into `tools/nv/cmd/records.ts:1`).
- [ ] **`bun nv gaps` and `bun nv holes`** (`tools/gaps.py:1`, `tools/holes.py:1`): the next two
      read-only tools the floor invokes.

## Backlog

- `nv plan --sync`, `--set` and `--amend` are writers, and they belong to the cutover stage, not
  Stage 5 (loop-goal.md § *Stage 5*).
- `--stale`'s parity case finds no sentence on either side, so only its header and count are
  compared. A seeded case would need a scratch root, which `nv parity` does not take yet.
