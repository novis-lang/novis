# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 is under way.** `bun nv
parity <group>` compares a Python tool with its `nv` replacement over the cases in
`tools/nv/parity/groups.json`, ignoring only the rewrites `tools/nv/parity/known.json` declares and,
per group, an `unordered` entry pattern. Six groups match on every case: `brief` (7 of 7), `holes`
(8 of 8), `owners` (12 of 12), `peek` (16 of 16), `plan` (13 of 13) and `records` (8 of 8: the full
audit, `--check`, `--only`, `--residue`, `--stats`, `--orphans`). `records --graph` is still
Python's. `tools/nv/lib/chain.ts` is the chain's reader.
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
`tools/nv/parity/groups.json` and is registered in `tools/nv/main.ts:21`'s `COMMANDS`.

- [ ] **`bun nv gaps`** (`tools/gaps.py:521`'s `main`, 595 lines, into a new
      `tools/nv/cmd/gaps.ts`): the process docs invoke the bare summary and `--errors`
      (`docs/agent/conventions.md:128`); port every mode `tools/gaps.py:525` declares
      (`--differential`, `--errors`, `--coverage`, `--member`, `--limit`, `--json`) and give each a
      parity case. `--json` is Python's `json.dumps`, which escapes every non-ASCII character:
      `tools/nv/cmd/holes.ts`'s `asciiJson` already writes that.
- [ ] **`bun nv records --graph NNNN`** (`tools/records.py:511`, into `tools/nv/cmd/records.ts:1`):
      `docs/agent/doc-cleanup.md:28` invokes it. The record has no `changes`, so `creates` is every
      rule whose `because` puts the record first and `modifies` every other rule naming it. Python
      lists them in front-matter order, which nothing else holds, so nv lists them by rule id and
      the group declares that. `depends on:` prints the bullet as written, which is
      `dependsOnText` when the record has one; check what the prose writes when it does not.

## Backlog

- `nv plan --sync`, `--set` and `--amend` are writers, and they belong to the cutover stage, not
  Stage 5 (loop-goal.md § *Stage 5*).
- `--stale`'s and `--check`'s parity cases, and the `records` and `holes` ones, run on a clean tree,
  so only the passing output is compared. A seeded case needs a scratch root, and `nv parity` does
  not take one yet.
- `crates/nvs-ir/tests/refusals.rs` runs `python tools/holes.py --guarded`; it moves to `bun nv holes
  --guarded` at the cutover, with the floor's two `holes.py` checks.
