# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 is under way.** `bun nv
parity <group>` compares a Python tool with its `nv` replacement over the cases in
`tools/nv/parity/groups.json`, ignoring only the rewrites `tools/nv/parity/known.json` declares and,
per group, an `unordered` entry pattern. Thirteen groups match on every case: `brief` (7),
`decisions` (14), `disk` (10), `gaps` (10), `holes` (8), `layout` (5), `links` (6), `migration` (9),
`owners` (12), `peek` (16), `plan` (13), `records` (12) and `rules` (14). `rules` also matched with
every finding kind seeded into `docs/rules/` and reverted. The write modes were compared by hand:
Python's `rules.py --render --check` is clean after `nv rules --render`, and a real `decisions
--apply` wrote the same bytes to all three files. Stage 5 still owes `directives` and `reference`
(loop-goal.md § *Stage 5*).
**`data/` is a snapshot, not yet the authority.** The legacy homes still are. Re-run `bun nv import
--write` before a stage reads `data/` as the truth, and at the cutover. `main` is still frozen, and
the tag `pre-overhaul` is the rollback point. The driver's red Stage 6 check (`nv impact --probe`)
is an unbuilt command, not a regression.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green. A check that is quiet on a clean tree is proved by injecting a fault
with a script under `.agent-tmp/`, running both tools, and `git checkout` of the file.
`tools/nv/lib/py.ts` holds what a port needs to print exactly what Python printed: `parseArgs`
(argparse's reading and its error messages), `splitlines`, `pyRepr`, `pyInt`, `squash`, `wrap` and
`fill` (textwrap), and `comparePaths` (`sorted()` over Windows paths). Use it, never a new copy.

## Next group

**Stage 5: the read-only tools, proven by parity** — one file set: `tools/nv/cmd/**`,
`tools/nv/parity/**`, `tools/nv/lib/py.ts`, `tools/nv/main.ts`. loop-goal.md § *Stage 5* is the
spec: parity is owed for every mode the floor, a goal `.toml` or a process doc invokes. Each new
command adds its group to `tools/nv/parity/groups.json` and is registered in `COMMANDS` in
`tools/nv/main.ts:29`. A case that needs an input file uses one under `tools/nv/parity/fixtures/`.

- [ ] **`bun nv directives`** (`tools/directives.py:1011`'s `main`, 1069 lines, into a new
      `tools/nv/cmd/directives.ts`). Find the modes the floor and the process docs invoke
      (`grep -rn "directives.py" docs/agent tools`), and give each a case.
- [ ] **`bun nv reference`** (`tools/reference.py:916`'s `main`, 968 lines, into a new
      `tools/nv/cmd/reference.ts`). It renders `docs/novis.md` from the rulebook's shipped rules, so
      compare a `--check` form rather than a write, and check a write by hand as `rules` was.

## Backlog

- The files `nv rules` and `nv decisions` render still name `python tools/rules.py --render` and
  `python tools/decisions.py --render`, so both tools write the same bytes. The slice that deletes
  each Python tool changes that constant and re-renders (`RENDER_COMMAND` in
  `tools/nv/cmd/rules.ts`, `COMMAND` in `tools/nv/cmd/decisions.ts`) — loop-goal.md § *Stage 5*.
- The pack's playbook filter held back the backslash-u trap, which this session hit again; it names
  `tools/nv/lib/py.ts` now, so an item naming that file gets it.
