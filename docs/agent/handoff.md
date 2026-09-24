# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard` and `bun nv splice` are
written, and the status row and goal table are pure functions in `tools/nv/driver/status.ts`.
`chain.py` stays until the Stage 9 cutover, because its `--show`, `--retire`, `--set` and `--retitle`
have no successor yet. `main` is frozen. Tag `pre-overhaul` is the rollback.

A goal's stage record carries `summary`, which the importer reads from a `**Does:**` line under the
stage's `## Stage N — <title>` heading. This goal's thirteen stages have one, in both
`docs/agent/goals/122-tooling-overhaul.md` and its copy `docs/agent/loop-goal.md`, which must stay
byte-identical: the importer finds the installed goal by that match. `nv check` reports every stage
with no `summary` ("a stage says what it does"), so the cutover's `bun nv check` fails until each
open goal's stages have one (see Backlog).

`bun nv loop --goal` prints `goalTable`. Until the cutover, its results come from the Python driver's
memo `.loop/goal-green.json` (`memoResults`), matched by check name. The memo's key digest is a
blake2b of size 6, which Bun cannot compute, and the memo is not re-checked against the tree. So the
floor shows only the verdicts that memo has filed, and a check that a later edit broke still shows as
green.

`nv splice` (`tools/nv/cmd/splice.ts`) ports all four forms of `splice.py`. It differs in three ways:
its messages open `nv splice:`, its help names `bun nv splice`, and a CRLF target keeps CRLF, where
`splice.py` writes it back as LF. `cargo-p` still names no `nv` command. It gets `verify` in its
`commands` when `nv verify` lands. `nv import --write` refuses while `impact_probes` has no importer.

One Stage 7 bullet waits for Stage 9's cutover: deleting `dossier.py`'s `--emit-goals`,
`--check-goals` and fan-out flags, and `generated_by` in `tools/loop.py`.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `tools/nv/lib/py.ts` holds what a port needs to print exactly what
Python printed. `bun x tsc --noEmit -p .` typechecks the tools, and `bun nv selftest` runs every tools
test.

## Next group

**Stage 8: parity for the ported writers** — one file set: `tools/nv/parity/groups.json`,
`tools/nv/parity/known.json`, `tools/nv/cmd/parity.ts`. The goal's § *Stage 8* **Parity** bullet and
§ *Standing decisions* "Parity before deletion" specify it.

- [ ] **A `splice` parity group** beside `peek` at `tools/nv/parity/groups.json:168`. Run both tools
      against two copies of one scratch tree and compare the files and output. List the three
      differences in § State in `tools/nv/parity/known.json:18`, each with its reason. First check
      that `tools/nv/cmd/parity.ts` can run a tool that writes.
- [ ] **Delete `tools/splice.py`** in its own slice once that group is green. Then point every
      place that still names it at `bun nv splice`: AGENTS.md rules 1 and 2,
      `docs/agent/commands.md:38` and `docs/agent/commands.md:157` (`git grep splice\.py` for the rest).
      `nv bg` and `nv guard` owe no parity group, because no Python tool of theirs is left.

## Backlog

- The cutover's `bun nv check` needs a `summary` on every open goal's stages: a script writes a
  `**Does:**` line for each. 54 of the 78 stages in open goals have no `## Stage N` heading to put it
  under (goal § *Stage 9*).
- `nv verify` on `nv bg`, then `verify` in the `cargo-p` rule's `commands` (goal § *Stage 8*).
- `nv orient` and `nv session --wrap` (goal § *Stage 8*).
- The driver half of `nv loop`, which paints `tools/nv/driver/status.ts` and feeds `[g]` from its
  own results (goal § *Stage 8*).
