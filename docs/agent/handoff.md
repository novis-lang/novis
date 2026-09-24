# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 is under way.** `bun nv
parity <group>` compares a Python tool with its `nv` replacement over the cases in
`tools/nv/parity/groups.json`, ignoring only the rewrites `tools/nv/parity/known.json` declares and,
per group, an `unordered` entry pattern. Five groups match on every case: `brief` (7 of 7),
`owners` (12 of 12), `peek` (16 of 16), `plan` (10 of 10: `--show` whole, `:lead`, `:verify`, a bad
part, a bad id, and `--get`) and `records` (1 of 1: `--stats`). `nv plan` reads `data/plan/`, and
`nv records` takes its set from `data/decisions/` and its sizes from the prose.
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
`tools/nv/parity/groups.json` and is registered in `tools/nv/main.ts:20`'s `COMMANDS`.

- [ ] **`bun nv plan --past` and `--stale`, the floor's other plan modes** (`tools/plan.py:391`,
      `tools/plan.py:867`, into `tools/nv/cmd/plan.ts:1`): `--past` reads the chain, the live goal and
      `nv owners --json`'s `registers[*].owners` (`tools/nv/cmd/owners.ts:418`); a goal record with no
      checks is a retired one, as `tools/nv/cmd/owners.ts:144` reads it.
- [ ] **`bun nv plan --check` and `bun nv records --check`** (`tools/plan.py:645`,
      `tools/records.py:478`): the floor runs both; each becomes a query in `nv check` with the old
      flag kept as an alias that prints the same verdict lines.
- [ ] **`bun nv gaps` and `bun nv holes`** (`tools/gaps.py:1`, `tools/holes.py:1`): the next two
      read-only tools of the stage.

## Backlog

- `nv links`, `nv layout`, `nv disk`, `nv directives`, `nv migration`, `nv reference` — loop-goal.md § *Stage 5*.
- Stage 6's `nv impact --probe`, the driver's red check — loop-goal.md § *Stage 6*.
