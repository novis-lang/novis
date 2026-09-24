# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and Stage 5 is under way.** `bun nv
parity <group>` compares a Python tool with its `nv` replacement over the cases in
`tools/nv/parity/groups.json`, ignoring only the rewrites `tools/nv/parity/known.json` declares and,
per group, an `unordered` entry pattern. It runs Python with `PYTHONIOENCODING=utf-8`, so a `…` or a
`—` compares as itself. Nine groups match on every case: `brief` (7), `gaps` (10), `holes` (8),
`layout` (5), `links` (6), `owners` (12), `peek` (16), `plan` (13) and `records` (12). `links` and
`layout` also matched with every finding kind seeded by a script under `.agent-tmp/` and reverted.
Stage 5 still owes `rules`, `decisions`, `disk`, `directives`, `migration` and `reference`
(loop-goal.md § *Stage 5*).
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
`tools/nv/parity/groups.json` and is registered in `COMMANDS` in `tools/nv/main.ts:25`.

- [ ] **`bun nv migration`** (`tools/check-migration.py:230`'s `main`, 320 lines, into a new
      `tools/nv/cmd/migration.ts`): CI's `docs` job runs it bare and the floor runs it with
      `--min N` (`docs/agent/loop-goal.toml:1909`); `--report` is its other mode. Give each a case,
      and one `--min` above the current count so the failing path is compared.
- [ ] **`bun nv disk`** (`tools/disk.py:445`'s `main`, 492 lines, into a new `tools/nv/cmd/disk.ts`):
      the report and `--deep` are read-only and owe parity now. `--clean` deletes, so it is a
      writer: port it, but compare only `--clean -n`. Sizes move between two runs of a live tree,
      so a size column may need a declared rewrite in `tools/nv/parity/known.json`.

## Backlog

- `nv plan --sync`, `--set` and `--amend` are writers, and they belong to the cutover stage, not
  Stage 5 (loop-goal.md § *Stage 5*).
- `--stale`'s and `--check`'s parity cases, and the `records` and `holes` ones, run on a clean tree,
  so only the passing output is compared. A seeded case needs a scratch root, and `nv parity` does
  not take one yet.
- `crates/nvs-ir/tests/refusals.rs` runs `python tools/holes.py --guarded`; it moves to `bun nv holes
  --guarded` at the cutover, with the floor's two `holes.py` checks.
- `tools/session.py --wrap` calls `check-links.py`'s `findings_in` in-process; `nv links` exports
  `findingsIn` with the same resolver parameter for the wrap's port (loop-goal.md § *Stage 5*).
- The rest of Stage 5 after the next group: `rules`, `decisions`, `directives`, `reference`
  (loop-goal.md § *Stage 5*).
