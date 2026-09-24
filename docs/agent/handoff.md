# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 is under way.** `bun nv
parity <group>` compares a Python tool with its `nv` replacement over the cases in
`tools/nv/parity/groups.json`, ignoring only the rewrites `tools/nv/parity/known.json` declares and,
per group, an `unordered` entry pattern. Seven groups match on every case: `brief` (7 of 7), `gaps`
(10 of 10: every mode, `--json` included), `holes` (8 of 8), `owners` (12 of 12), `peek` (16 of 16),
`plan` (13 of 13) and `records` (12 of 12: the full audit, `--check`, `--only`, `--residue`,
`--stats`, `--orphans` and `--graph`, which also matched on all 215 records by a one-off script).
Stage 5 still owes `rules`, `decisions`, `links`, `layout`, `disk`, `directives`, `migration` and
`reference` (loop-goal.md § *Stage 5*).
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
`tools/nv/parity/groups.json` and is registered in `COMMANDS` in `tools/nv/main.ts:23`.

- [ ] **`bun nv links`** (`tools/check-links.py:320`'s `main`, 361 lines, into a new
      `tools/nv/cmd/links.ts`): `docs/agent/doc-cleanup.md:30` and CI's `docs` job run it bare, and
      it takes paths as arguments. A case that finds a broken link needs a seeded fault: write it
      with a script under `.agent-tmp/`, run both, then `git checkout` the file.
- [ ] **`bun nv layout`** (`tools/layout.py:213`'s `main`, 254 lines, into a new
      `tools/nv/cmd/layout.ts`): `--check` and `--rows`, which `docs/agent/commands.md:345`
      describes. Give each mode a parity case.

## Backlog

- `nv plan --sync`, `--set` and `--amend` are writers, and they belong to the cutover stage, not
  Stage 5 (loop-goal.md § *Stage 5*).
- `--stale`'s and `--check`'s parity cases, and the `records` and `holes` ones, run on a clean tree,
  so only the passing output is compared. A seeded case needs a scratch root, and `nv parity` does
  not take one yet.
- `crates/nvs-ir/tests/refusals.rs` runs `python tools/holes.py --guarded`; it moves to `bun nv holes
  --guarded` at the cutover, with the floor's two `holes.py` checks.
- The rest of Stage 5 after the next group: `rules`, `decisions`, `disk`, `directives`, `migration`,
  `reference` (loop-goal.md § *Stage 5*).
