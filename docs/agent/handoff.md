# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 has begun.** `bun nv
parity <group>` runs a Python tool and its `nv` replacement over the cases in
`tools/nv/parity/groups.json` and ignores only the rewrites `tools/nv/parity/known.json` declares.
`bun nv brief --where` reads the rulebook from `data/rules/` and matches `python tools/brief.py
--where` on all seven cases, so Stage 5's one acceptance check now passes. `nv brief` has no other
mode yet, and its owners home names `bun nv owners --check` instead of counting, a declared
difference that `nv owners` removes.
**`data/` is a snapshot, not yet the authority.** The legacy homes still are. Re-run `bun nv import
--write` before a stage reads `data/` as the truth, and at the cutover. `main` is still frozen, and
the tag `pre-overhaul` is the rollback point.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green.

## Next group

**Stage 5: the read-only tools, proven by parity** — one file set: `tools/nv/cmd/**`,
`tools/nv/parity/**`, `tools/nv/main.ts`. loop-goal.md § *Stage 5* is the spec. Each new command
adds its group to `tools/nv/parity/groups.json` with every mode the floor, a goal `.toml` or a
process doc invokes, and is registered in `tools/nv/main.ts:17`'s `COMMANDS`.

- [ ] **`bun nv owners`** (`tools/owners.py:512`): classifies the gap records under `data/gaps/`
      by owner, with `--check` and `--registers`. Then `nv brief --where owners` counts from it,
      and the `brief` entry about the owners line leaves `tools/nv/parity/known.json`.
- [ ] **`bun nv peek` with `--locate` and `--outline`** (`tools/peek.py:45`): the read tool
      every session uses. A `rule:` target reads `docs/rules/<topic>/<slug>.md`.
- [ ] **`bun nv plan --show` and `bun nv records --stats`** (`tools/plan.py:917`,
      `tools/records.py:566`): read-only modes over the milestone and decision records.

## Backlog

- `nv gaps`, `nv holes`, `nv links`, `nv layout`, `nv disk`, `nv directives` — loop-goal.md § *Stage 5*.
- `nv migration` and `nv reference` (`tools/check-migration.py:230`); `reference.py` runs the
  binary and every example, so it is the largest of them — loop-goal.md § *Stage 5*.
- The rest of `nv brief` (status, milestones, map) — `tools/brief.py`.
- Each replaced check becomes a query in `nv check`, its old flag an alias — loop-goal.md § *Stage 5*.
