# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 is under way.** `bun nv
parity <group>` compares a Python tool with its `nv` replacement over the cases in
`tools/nv/parity/groups.json`, ignoring only the rewrites `tools/nv/parity/known.json` declares and,
per group, an `unordered` entry pattern. Eleven groups match on every case: `brief` (7), `disk` (10),
`gaps` (10), `holes` (8), `layout` (5), `links` (6), `migration` (9), `owners` (12), `peek` (16),
`plan` (13) and `records` (12). `migration` also matched with every finding kind seeded into
`docs/spec/02-php-migration.md` and reverted. `disk --clean` refuses while a driver holds
`.loop/running`, so parity compares only that refusal; its dry-run sweep was compared by calling both
`clean` functions directly, and `--deep` by hand, and both matched. Stage 5 still owes `rules`,
`decisions`, `directives` and `reference` (loop-goal.md § *Stage 5*).
**`data/` is a snapshot, not yet the authority.** The legacy homes still are. Re-run `bun nv import
--write` before a stage reads `data/` as the truth, and at the cutover. `main` is still frozen, and
the tag `pre-overhaul` is the rollback point. The driver's red Stage 6 check (`nv impact --probe`)
is an unbuilt command, not a regression.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green. A check that is quiet on a clean tree is proved by injecting a fault
with a script under `.agent-tmp/`, running both tools, and `git checkout` of the file.

## Next group

**Stage 5: the read-only tools, proven by parity** — one file set: `tools/nv/cmd/**`,
`tools/nv/parity/**`, `tools/nv/main.ts`. loop-goal.md § *Stage 5* is the spec: parity is owed for
every mode the floor, a goal `.toml` or a process doc invokes. Each new command adds its group to
`tools/nv/parity/groups.json` and is registered in `COMMANDS` in `tools/nv/main.ts:28`.

- [ ] **`bun nv rules`** (`tools/rules.py:433`'s `main`, 539 lines, into a new
      `tools/nv/cmd/rules.ts`): the rulebook's load, validate, render and citation resolve. Find
      the modes the floor and the process docs invoke (`grep -rn "rules.py" docs/agent tools`), and
      give each a case. `--render` writes files, so compare its `--check` form.
- [ ] **`bun nv decisions`** (`tools/decisions.py:579`'s `main`, 653 lines, into a new
      `tools/nv/cmd/decisions.ts`): the same shape over `docs/decisions/`; reuse what `rules.ts`
      and `records.ts` already parse.

## Backlog

- `bun nv directives` (`tools/directives.py:1011`, 1069 lines) and `bun nv reference`
  (`tools/reference.py:916`, 968 lines) close Stage 5 — loop-goal.md § *Stage 5*.
- `splitlines`, Python's `repr` and its `int` parser are copied into several `tools/nv/cmd/*.ts`;
  one `tools/nv/lib/pytext.ts` would hold them — the playbook's backslash-u bullet waits on it.
