# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list`, `bun nv bg` and `bun nv guard` are written, and Stage 8's five acceptance checks
should all pass. `chain.py` stays until the Stage 9 cutover, because its `--show`, `--retire`, `--set`
and `--retitle` have no successor yet. `main` is frozen. Tag `pre-overhaul` is the rollback.

`nv guard` (`tools/nv/cmd/guard.ts`) is the one `PreToolUse` hook, wired in `.claude/settings.json` for
`Read|Bash|PowerShell`, and `tools/guard-read.py` is deleted. It has six rules in `RULES`, each with a
denied case and a near miss in `tools/nv/test/guard.test.ts`. Two rules name no `nv` command yet:
`inline-write` points at `python tools/splice.py`, and `cargo-p` at `python tools/verify.py -p`. When
`nv splice` and `nv verify` land, each adds its name to the rule's `commands`, which `nv guard --check`
holds to the registry. The hook costs about 100 ms of Bun start-up per shell call.

`nv loop` is `--list` only (`tools/nv/cmd/loop.ts`). While `docs/agent/loop-goal.toml` exists it is
the plan, imported through `installedGoal` in `tools/nv/import/goals.ts`. `nv import --write` refuses
while `impact_probes` has no importer, so the records are refreshed only at the cutover. The driver half
of `nv loop` is still to write. `nv bg` keeps each job under `.agent-tmp/bg/<id>/`, and `nv verify
--start`/`--wait` is to be rebuilt on it.

`nv chain --check` does not resolve a goal's `[context]` selectors. That half arrives with `nv orient`.

One Stage 7 bullet waits for Stage 9's cutover: deleting `dossier.py`'s `--emit-goals`,
`--check-goals` and fan-out flags, and `generated_by` in `tools/loop.py`. Two floor checks still run
`dossier.py --emit-goals --dry-run` and `--check-goals` (`docs/agent/loop-goal.toml:12293`, `:12317`).

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `tools/nv/lib/py.ts` holds what a port needs to print exactly what
Python printed. `bun x tsc --noEmit -p .` typechecks the tools. A driver change is proved by a probe
script under `.agent-tmp/` that builds a `Goal` from `docs/agent/loop-goal.toml` and stubs `capture`.

With Stage 8's checks green, the next failing acceptance line is expected in Stage 9 or later. Stage 8's
remaining prose items (`nv splice`, `nv verify`, `nv orient`, `nv session --wrap`, the driver, the
parity groups) carry no acceptance check of their own, so they are what Stage 9's cutover needs first.

## Next group

**Stage 8: `nv splice`** — one file set: `tools/nv/cmd/splice.ts` (new), `tools/nv/test/splice.test.ts`
(new), `tools/nv/main.ts`, `tools/nv/cmd/guard.ts`. loop-goal.md § *Stage 8* lists `splice.ts` in its
file set.

- [ ] **`bun nv splice --patch <file>` applies a conflict-marker patch across files, all or nothing.**
      Port `tools/splice.py`'s `--patch`, `<target> --patch`, `<target> <old> <new>` and `--dry-run`
      forms with the same messages, and register `splice` in `COMMANDS` at `tools/nv/main.ts:38`.
      Its test applies a two-file patch to a temporary tree and shows a stale anchor in the second file
      leaves the first untouched.
- [ ] **The guard's `inline-write` rule names `nv splice`.** Set its `commands` to `["splice"]` at
      `tools/nv/cmd/guard.ts:402` and change its reason to `bun nv splice --patch <file>`; `bun nv
      guard --check` must still print `guard: every rule names a registered command`.

## Backlog

- `nv verify` over the Stage 6 keys, rebuilt on `nv bg` for `--start`/`--wait`; then `cargo-p` names it
  at `tools/nv/cmd/guard.ts:451` — loop-goal.md § *Stage 8*.
- `nv orient` from records, resolving `[context]` by id, which `nv chain --check` then uses — loop-goal.md § *Stage 8*.
- `nv session --wrap` writing records, and `bun nv parity writers` / `parity orient` — loop-goal.md § *Stage 8*.
- The driver half of `nv loop`, keeping `respawn`'s exit-75 protocol — loop-goal.md § *Stage 8*.
- `nv loop-stats --guard` counts the guard's denials per rule — loop-goal.md § *Stage 11*.
