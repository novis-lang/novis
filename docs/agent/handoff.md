# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard` and `bun nv splice` are
written, and the status row and goal table are pure functions in `tools/nv/driver/status.ts`.
`chain.py` stays until the Stage 9 cutover, because its `--show`, `--retire`, `--set` and `--retitle`
have no successor yet. `main` is frozen. Tag `pre-overhaul` is the rollback.

**`tools/splice.py` is deleted.** Every citation names `bun nv splice`, and the `splice` parity group
is gone with it. `tools/loop-stats.py` counts a Bash call as an edit when it names `nv splice` or
`splice.py` (`SPLICE_MARKERS`), because older logs carry the Python name. The one citation left is a
measured figure in the goal's own prose, `docs/agent/goals/122-tooling-overhaul.md:528`, which is
history and stays.

The playbook still has two homes: the pages under `docs/agent/playbook/` are what `playbook.py` and
`orient.py` read, and `data/playbook/` is what `nv import` made from them. No renderer writes the
pages yet, so `bun nv render` does not touch them, and an edit to a bullet goes into both by hand or by
script until Stage 9.

A goal's stage record carries `summary`, from a `**Does:**` line under the stage's `## Stage N` heading.
`nv check`'s 80 findings are all "a stage says what it does": `data/goals/*.json` was imported before
`summary` existed, and `nv import --write` refuses while `impact_probes` has no importer. So the
acceptance check `every record is valid and every reference resolves` stays red until that importer
lands or the records are rewritten (see Backlog).

`bun nv loop --goal` reads its results from the Python driver's memo `.loop/goal-green.json`
(`memoResults`), matched by check name, and the memo is not re-checked against the tree.

One Stage 7 bullet waits for Stage 9's cutover: deleting `dossier.py`'s `--emit-goals`,
`--check-goals` and fan-out flags, and `generated_by` in `tools/loop.py`.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `tools/nv/lib/py.ts` holds what a port needs to print exactly what
Python printed. `bun x tsc --noEmit -p .` typechecks the tools, and `bun nv selftest` runs every tools
test.

## Next group

**Stage 8: `nv verify`** — one file set: `tools/verify.py`, `tools/verify_keys.py`,
`tools/nv/cmd/bg.ts`, `tools/nv/cmd/guard.ts`. The goal's Stage 8 bullet "`nv verify` takes over
`verify.py`'s steps unchanged, over the Stage 6 keys" specifies it.

- [ ] **Port `verify.py`'s steps to `tools/nv/cmd/verify.ts`**, run detached through `nv bg`, from
      `tools/verify.py:1266` (`main`) and `tools/nv/cmd/bg.ts:161`. `--start`, `--wait`, `--fast`,
      `-p` and `--doc` keep their meaning, and each step's key comes from the Stage 6 key function.
- [ ] **Add a `verify` parity group** after `reference` at `tools/nv/parity/groups.json:279`, and delete `tools/verify.py`
      only in a later slice after it is green (§ *Standing decisions* "Parity before deletion").
- [ ] **Name `verify` in the `cargo-p` rule's `commands`** at `tools/nv/cmd/guard.ts:451`.

## Backlog

- The acceptance check `bun nv check` needs a `summary` on every open goal's stages. Either the
  `impact_probes` importer lands so `nv import --write` re-reads the `**Does:**` lines, or a script
  writes `summary` into `data/goals/*.json`. 54 of the 78 open stages have no `## Stage N` heading to
  put a line under (goal § *Stage 9*).
- `nv orient` and `nv session --wrap` (goal § *Stage 8*).
- The driver half of `nv loop`, which paints `tools/nv/driver/status.ts` and feeds `[g]` from its
  own results (goal § *Stage 8*).
- A renderer for the playbook pages, so `data/playbook/` becomes their one home (goal § *Stage 9*).
