# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`
is written: `--check` is a query over `data/chain.json`, the goal and handoff records and the prose,
and `--new`, `--move` and `--remove` edit `data/chain.json` and nothing else. Parity group `chain`
matches `chain.py --check` (1 of 1). `chain.py` stays until the Stage 9 cutover, because its
`--show`, `--retire`, `--set` and `--retitle` have no successor yet. `main` is frozen. Tag
`pre-overhaul` is the rollback.

`nv chain --check` does not resolve a goal's `[context]` selectors, which `chain.py` did through
`orient.py`'s `manifest_findings`. That half arrives with `nv orient`, which resolves `[context]` by id.

One Stage 7 bullet waits for Stage 9's cutover: deleting `dossier.py`'s `--emit-goals`,
`--check-goals` and fan-out flags, and `generated_by` in `tools/loop.py`. Two floor checks still run
`dossier.py --emit-goals --dry-run` and `--check-goals` (`docs/agent/loop-goal.toml:12293`, `:12317`).
Stage 9's argv script has to map those two checks while keeping their `want` lines.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `tools/nv/lib/py.ts` holds what a port needs to print exactly what
Python printed. `bun x tsc --noEmit -p .` typechecks the tools. `loop.py` has no test suite. A driver
change is proved by a probe script under `.agent-tmp/` that builds a `Goal` from
`docs/agent/loop-goal.toml` and stubs `capture`.

The next failing acceptance line is `bun nv loop --list --name ...` in Stage 8. `nv loop` is not
written yet, so this is an artefact still to write, not a regression.

## Next group

**Stage 8: `nv loop --list` and `nv bg`** — one file set: `tools/nv/cmd/loop.ts` (new),
`tools/nv/cmd/bg.ts` (new), `tools/nv/main.ts`, `tools/nv/test/bg.test.ts` (new). loop-goal.md
§ *Stage 8*, "`nv loop` is the driver" and "`nv bg`", is the spec.

- [ ] **`bun nv loop --list --name "one proofs run verifies several groups"` prints that check and
      `list: 1 check matches`**: the acceptance plan read from the live goal's record under
      `data/goals/`, narrowed by `--stage <label>`, `--name <text>` and `--feature <id>`. Port the
      `--list` branch at `tools/loop.py:6522`, and register `loop` in `COMMANDS` at
      `tools/nv/main.ts:35`. Only `--list` is this item; the driver itself is a later group.
- [ ] **`nv bg <command…>` starts a job detached and prints its id; `--wait <id>` gives back the
      output's tail and the exit status; `--list` names running jobs**: jobs and logs under
      `.agent-tmp/bg/`. `bun test tools/nv/test/bg.test.ts` covers a job exiting 0 and one exiting
      non-zero. Register it at `tools/nv/main.ts:35`.

## Backlog

- `nv guard` and `tools/nv/test/guard.test.ts`, then `.claude/settings.json` and deleting
  `guard-read.py` — loop-goal.md § *Stage 8*, "`nv guard`".
- `nv verify`, `nv orient`, `nv session --wrap` and parity groups `writers` and `orient` —
  loop-goal.md § *Stage 8*.
- `nv orient` owes the `[context]` resolution half of `nv chain --check`.
- The Stage 7 cutover bullet above — loop-goal.md § *Stage 9*.
