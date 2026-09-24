# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 is under way.** `bun nv
parity <group>` compares a Python tool with its `nv` replacement over the cases in
`tools/nv/parity/groups.json`, ignoring only the rewrites `tools/nv/parity/known.json` declares and,
per group, an `unordered` entry pattern. Five groups match on every case: `brief` (7 of 7),
`owners` (12 of 12), `peek` (16 of 16), `plan` (13 of 13: `--show`, `--get`, `--past`, `--stale`,
`--check`) and `records` (1 of 1: `--stats`). `tools/nv/lib/chain.ts` is the chain's reader.
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

- [ ] **`bun nv records --check`** (`tools/records.py:464`'s `CHECKS`, into
      `tools/nv/cmd/records.ts:88`): `--check` is quiet on a clean tree, so parity compares an
      empty output and exit 0, and the port must still run every check for real. The decision
      record (`tools/nv/schema/decision.ts:11`) has no `changes` field and its `status` enum is
      `accepted`/`retired`, so `metadata` and `changes` become checks over the records and the
      rules' `because` lists: a `because` entry is a decision record, and the prose has its
      `> **In short:**` block. `structure`, `links`, `section refs`, `changelog residue` and
      `stale counters` read `docs/decisions/NNNN.md` as prose, the same as Python. Add a
      `["--only", "links"]` case too, since that path prints `ok` lines.
- [ ] **`bun nv gaps` and `bun nv holes`** (`tools/gaps.py:1`, `tools/holes.py:1`): the next two
      read-only tools the floor invokes.

## Backlog

- `nv plan --sync`, `--set` and `--amend` are writers, and they belong to the cutover stage, not
  Stage 5 (loop-goal.md § *Stage 5*).
- `--stale`'s and `--check`'s parity cases run on a clean tree, so only the passing output is
  compared. A seeded case needs a scratch root, and `nv parity` does not take one yet.
