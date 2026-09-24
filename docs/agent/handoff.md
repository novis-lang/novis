# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list` and `bun nv bg` are written. `chain.py` stays until the Stage 9 cutover, because
its `--show`, `--retire`, `--set` and `--retitle` have no successor yet. `main` is frozen. Tag
`pre-overhaul` is the rollback.

`nv loop` is `--list` only (`tools/nv/cmd/loop.ts`): `--stage`, `--name` and `--feature` narrow the
plan, and the last line is `list: N check(s) match`. While `docs/agent/loop-goal.toml` exists it is
the plan, imported through `installedGoal` in `tools/nv/import/goals.ts`, because `data/goals/*.json`
is the import's snapshot and falls behind the toml the sessions edit (the stored `tooling-overhaul`
record had 1 of Stage 8's 5 checks). `nv import --write` refuses while `impact_probes` has no
importer, so the records are refreshed only at the cutover. The driver half of `nv loop` is still
to write.

`nv bg` (`tools/nv/cmd/bg.ts`) keeps each job under `.agent-tmp/bg/<id>/`, with a detached supervisor
(`bg --run <id>`) that writes the `exit` file. `nv verify --start`/`--wait` is to be rebuilt on it
when `nv verify` is written.

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

The next failing acceptance line should be `bun test tools/nv/test/guard.test.ts` in Stage 8. `nv guard`
is not written yet, so this is an artefact still to write, not a regression.

## Next group

**Stage 8: `nv guard`** — one file set: `tools/nv/cmd/guard.ts` (new), `tools/nv/test/guard.test.ts`
(new), `tools/nv/main.ts`, `.claude/settings.json`, `tools/guard-read.py`. loop-goal.md § *Stage 8*,
"`nv guard` is the one `PreToolUse` hook", is the spec.

- [ ] **`bun test tools/nv/test/guard.test.ts` passes: every guard rule denies its raw pattern and
      allows its near miss.** Port `tools/guard-read.py`'s Read rule into `tools/nv/cmd/guard.ts`, add
      the Bash and PowerShell rules the spec lists (loop-goal.md § *Stage 8*, the `nv guard` bullets),
      fail open on a payload it cannot read, and register `guard` in `COMMANDS` at
      `tools/nv/main.ts:37`.
- [ ] **`bun nv guard --check` prints `guard: wired for Read, Bash and PowerShell` and `guard: every
      rule names a registered command`**: `.claude/settings.json:4` runs `bun nv guard` for the three
      tools, and `tools/guard-read.py:1` is deleted with its entry.

## Backlog

- The driver half of `nv loop` (respawn protocol, exit `75`, `NOVIS_LOOP_RUN`, every flag) — loop-goal.md § *Stage 8*.
- `nv verify`, `nv orient`, `nv session`, `nv side`, `nv respawn`, `nv splice` — loop-goal.md § *Stage 8*'s file set.
- `nv verify --start`/`--wait` over `nv bg` — `tools/nv/cmd/bg.ts`'s module doc.
- Stage 9's argv script maps the two `dossier.py` goal-emitting floor checks — loop-goal.md § *Stage 9*.
