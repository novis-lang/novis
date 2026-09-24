# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 is under way.** `bun nv
parity <group>` compares a Python tool with its `nv` replacement over the cases in
`tools/nv/parity/groups.json`, ignoring only the rewrites `tools/nv/parity/known.json` declares and,
per group, an `unordered` entry pattern. Two groups match on every case: `brief` (`--where`, 7 of 7)
and `owners` (12 of 12: the roster, `--check` with each no-op flag, `--closes`, `--deferrals`,
`--registers`, `--untagged`, `--unowned`, `--json`). `nv owners` classifies the gap records under
`data/gaps/`, and `nv brief --where owners` counts from it.
**`data/` is a snapshot, not yet the authority.** The legacy homes still are. Re-run `bun nv import
--write` before a stage reads `data/` as the truth, and at the cutover. `main` is still frozen, and
the tag `pre-overhaul` is the rollback point. The driver's red Stage 6 check (`nv impact --probe`)
is an unbuilt command, not a regression.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green.

## Next group

**Stage 5: the read-only tools, proven by parity** — one file set: `tools/nv/cmd/**`,
`tools/nv/parity/**`, `tools/nv/main.ts`. loop-goal.md § *Stage 5* is the spec. Each new command
adds its group to `tools/nv/parity/groups.json` with every mode the floor, a goal `.toml` or a
process doc invokes, and is registered in `tools/nv/main.ts:18`'s `COMMANDS`. A record with no
position (a gap, a bullet) lists in slug order, and the group's `unordered` pattern declares it.

- [ ] **`bun nv peek` with `--locate` and `--outline`** (`tools/peek.py:45`): the read tool
      every session uses. A `rule:` target reads `docs/rules/<topic>/<slug>.md`.
- [ ] **`bun nv plan --show` and `bun nv records --stats`** (`tools/plan.py:917`,
      `tools/records.py:566`): read-only modes over the milestone and decision records.
      `plan.py --past` reads `owners.py --json`'s `registers[*].owners`, which `nv owners --json`
      prints in the same shape (`tools/nv/cmd/owners.ts:418`).

## Backlog

- `nv gaps`, `nv holes`, `nv links`, `nv layout`, `nv disk`, `nv directives` — loop-goal.md § *Stage 5*.
- `nv migration` and `nv reference` (`tools/check-migration.py:230`); `reference.py` runs the
  binary and every example, so it is the largest of them — loop-goal.md § *Stage 5*.
- The rest of `nv brief` (status, milestones, map) — `tools/brief.py`.
- Each replaced check becomes a query in `nv check`, its old flag an alias; `owners --check` is the
  first with a parity-proven replacement — loop-goal.md § *Stage 5*.
- `owners.py` is deleted only in a slice of its own, after its callers (`verify.py`, `loop.py`
  `owner_gate`, `plan.py`, the floor argvs) move to `nv owners` — loop-goal.md § *Stage 5*.
