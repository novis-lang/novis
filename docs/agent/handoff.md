# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list`, `bun nv bg`, `bun nv guard` and `bun nv splice` are written, and the status row
and goal table are pure functions in `tools/nv/driver/status.ts`. All six Stage 8 acceptance checks
pass by hand, `status.test.ts` included. `chain.py` stays until the Stage 9 cutover, because its
`--show`, `--retire`, `--set` and `--retitle` have no successor yet. `main` is frozen. Tag
`pre-overhaul` is the rollback.

`nv splice` (`tools/nv/cmd/splice.ts`) ports all four forms of `splice.py`. Its differences, for the
parity group that must precede deleting `splice.py`: messages open `nv splice:` rather than `splice.py:`,
the help names `bun nv splice`, and a CRLF target keeps CRLF. `splice.py` reads in text mode and so
writes such a file back as LF. The guard's `inline-write` rule names `nv splice`. `cargo-p` still names
no `nv` command, and gets `verify` in its `commands` when `nv verify` lands.

`tools/nv/driver/status.ts` has `Session` (fed stream-json events), `statusRow`, `keyRow` and
`goalTable`. Nothing paints them yet, because the driver half of `nv loop` is unwritten. A stage has no
`summary` field yet, so the table shows a stage's title in `what it does`. The floor is the stage whose
title is `floor` (`FLOOR`), which is how the importer names stage 1.

`nv loop` is `--list` only (`tools/nv/cmd/loop.ts`). While `docs/agent/loop-goal.toml` exists it is
the plan, imported through `installedGoal` in `tools/nv/import/goals.ts`. `nv import --write` refuses
while `impact_probes` has no importer, so the records are refreshed only at the cutover.

One Stage 7 bullet waits for Stage 9's cutover: deleting `dossier.py`'s `--emit-goals`,
`--check-goals` and fan-out flags, and `generated_by` in `tools/loop.py`. Two floor checks still run
`dossier.py --emit-goals --dry-run` and `--check-goals` (`docs/agent/loop-goal.toml:12293`, `:12317`).

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `tools/nv/lib/py.ts` holds what a port needs to print exactly what
Python printed. `bun x tsc --noEmit -p .` typechecks the tools, and `bun nv selftest` runs every tools
test.

## Next group

**Stage 8: the goal table from the records** — one file set: `tools/nv/schema/goal.ts`,
`tools/nv/import/goals.ts`, `tools/nv/cmd/loop.ts`, `tools/nv/driver/status.ts`,
`docs/agent/goals/122-tooling-overhaul.md`. The goal's § *Stage 8* `[g]` bullet specifies all three.

- [ ] **A stage record carries `summary`, read from a `**Does:** <sentence>` line under its heading.**
      Add `summary: s.optional(s.string())` to the stage at `tools/nv/schema/goal.ts:21`, have
      `installedGoal` at `tools/nv/import/goals.ts:146` read it, and write a `**Does:**` line under
      each of this goal's stage headings in `docs/agent/goals/122-tooling-overhaul.md:416`.
- [ ] **`bun nv loop --goal` prints `goalTable`** from `tools/nv/driver/status.ts:190`, beside
      `--list` in `tools/nv/cmd/loop.ts:175`. Its results come from the Python driver's last sweep
      while it runs the plan; where that sweep's per-check results are kept is not checked.
- [ ] **`nv check` names a stage with no `summary`**, as a query beside the goal's others at
      `tools/nv/schema/goal.ts:82`.

## Backlog

- `nv verify` on `nv bg`, then `verify` in the `cargo-p` rule's `commands` (goal § *Stage 8*).
- `nv orient` and `nv session --wrap` (goal § *Stage 8*).
- The driver half of `nv loop`, painting `tools/nv/driver/status.ts` (goal § *Stage 8*).
- Parity groups for `splice`, `bg` and `guard`, with the differences listed in § State (goal § *Stage 8*).
