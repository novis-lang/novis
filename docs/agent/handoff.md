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

`nv parity` runs a tool that writes: a group with a `tree` lays it out twice under
`.agent-tmp/parity/<group>/`, runs each side in its own copy, and compares the files each run changed
as well as the output. **`bun nv parity splice` is green, 20 of 20**, and `known.json` declares its six
differences with their reasons: the message prefix, the help's pointer to a docstring, the OS error
wording, CRLF kept by nv and turned into LF by `splice.py`, and two help paragraphs nv rewrote.
`cargo-p` still names no `nv` command. It gets `verify` in its `commands` when `nv verify` lands.
`nv import --write` refuses while `impact_probes` has no importer.

One Stage 7 bullet waits for Stage 9's cutover: deleting `dossier.py`'s `--emit-goals`,
`--check-goals` and fan-out flags, and `generated_by` in `tools/loop.py`.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `tools/nv/lib/py.ts` holds what a port needs to print exactly what
Python printed. `bun x tsc --noEmit -p .` typechecks the tools, and `bun nv selftest` runs every tools
test.

## Next group

**Stage 8: retire `splice.py`** — one file set: `tools/splice.py` and every file that names it, which
`git grep -l "splice\.py"` lists (about 180, most of them playbook records under `data/playbook/` and
their rendered pages, and generated goals under `docs/agent/goals/dossier/`). The goal's § *Standing
decisions* "Parity before deletion" and "A mechanical change … is made by a script" specify it.

- [ ] **Delete `tools/splice.py`, and point every citation at `bun nv splice`** with one script under
      `.agent-tmp/`. Edit a playbook record in `data/playbook/` and re-render with `bun nv render`,
      never the page. Hand-read these, because they are code or prose rather than a citation:
      AGENTS.md rules 1 and 2, `docs/agent/commands.md:38`, `tools/brief.py:468` together with
      `tools/nv/cmd/brief.ts:21` (keep the `brief` parity group green), and `tools/loop-stats.py:326`,
      which counts a session's edits by the text `splice.py` and must also count `nv splice`. Drop the
      `splice` group from `tools/nv/parity/groups.json:190` and `tools/nv/parity/known.json:26` in the
      same slice, since no Python side is left to run.

## Backlog

- The cutover's `bun nv check` needs a `summary` on every open goal's stages: a script writes a
  `**Does:**` line for each. 54 of the 78 stages in open goals have no `## Stage N` heading to put it
  under (goal § *Stage 9*).
- `nv verify` on `nv bg`, then `verify` in the `cargo-p` rule's `commands` (goal § *Stage 8*).
- `nv orient` and `nv session --wrap` (goal § *Stage 8*).
- The driver half of `nv loop`, which paints `tools/nv/driver/status.ts` and feeds `[g]` from its
  own results (goal § *Stage 8*).
