# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 is under way.** `bun nv
parity <group>` compares a Python tool with its `nv` replacement over the cases in
`tools/nv/parity/groups.json`, ignoring only the rewrites `tools/nv/parity/known.json` declares and,
per group, an `unordered` entry pattern. Fourteen groups match on every case: `brief` (7),
`decisions` (14), `directives` (16), `disk` (10), `gaps` (10), `holes` (8), `layout` (5), `links`
(6), `migration` (9), `owners` (12), `peek` (16), `plan` (13), `records` (12) and `rules` (14).
`rules` also matched with every finding kind seeded into `docs/rules/` and reverted, and
`directives` with a dropped, a wrong and a malformed `[unread:]` trailer seeded into
`crates/nvs-config/src/tree.rs` and reverted; its template case reads
`tools/nv/parity/fixtures/directives-template.toml`, which carries every `--check-template` problem
kind. Stage 5 still owes `reference` (loop-goal.md § *Stage 5*).
**`data/` is a snapshot, not yet the authority.** The legacy homes still are. Re-run `bun nv import
--write` before a stage reads `data/` as the truth, and at the cutover. `main` is still frozen, and
the tag `pre-overhaul` is the rollback point. The driver's red Stage 6 check (`nv impact --probe`)
is an unbuilt command, not a regression.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green. A check that is quiet on a clean tree is proved by injecting a fault
with a script under `.agent-tmp/`, running both tools, and `git checkout` of the file.
`tools/nv/lib/py.ts` holds what a port needs to print exactly what Python printed: `parseArgs`
(argparse's reading and its error messages, with `optional` for a `nargs="?"` option), `splitlines`,
`pyRepr`, `pyInt`, `squash`, `wrap` and `fill` (textwrap), and `comparePaths` (`sorted()` over
Windows paths). Use it, never a new copy. `bun x tsc --noEmit -p .` typechecks the tools.

## Next group

**Stage 5: the read-only tools, proven by parity** — one file set: `tools/nv/cmd/**`,
`tools/nv/parity/**`, `tools/nv/lib/py.ts`, `tools/nv/main.ts`. loop-goal.md § *Stage 5* is the
spec: parity is owed for every mode the floor, a goal `.toml` or a process doc invokes. Each new
command adds its group to `tools/nv/parity/groups.json` and is registered in `COMMANDS` in
`tools/nv/main.ts:30`. A case that needs an input file uses one under `tools/nv/parity/fixtures/`.

- [ ] **`bun nv reference`** (`tools/reference.py:916`'s `main`, 968 lines, into a new
      `tools/nv/cmd/reference.ts`). Find the modes the floor and the process docs invoke
      (`grep -rn "reference.py" docs/agent tools`), and give each a case. A Python `json.dumps`
      escapes every non-ASCII character; `pyJson` in `tools/nv/cmd/directives.ts` is the port of
      it, and moves into `tools/nv/lib/py.ts` the moment a second command needs it.
- [ ] **Stage 5's close-out** — with `reference` green, every Python tool the stage names has a
      matching group; read loop-goal.md § *Stage 5* for what the stage owes beyond parity and take
      the next stage's first item from `tools/nv/main.ts:30`'s list of what is still missing.

## Backlog

- `tools/nv/cmd/directives.ts` prints `--help` as the other ports do (usage and summary), not
  argparse's full help; no floor check or doc reads it (loop-goal.md § *Stage 5*).
- The playbook bullets under `docs/agent/playbook/tooling/` that name `tools/directives.py` are
  renamed at the cutover's `argv` rewrite, not before (loop-goal.md § *Standing decisions*).
