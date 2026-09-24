# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, Stage 4's records are valid, and Stage 8 is in
progress.** `bun nv check` prints `nv check: 0 findings`, `bun nv render --check` is current and
`data/chain.json` is tracked, so all three Stage 4 checks pass. `nv import` covers every record type:
`impact_probes` is read back from `data/impact-probes.json`, which has no legacy home. Every goal
stage that carries a check has a `**Does:**` line under its `## Stage N — <title>` heading in the
goal's `.md`, and `data/goals/*.json` was re-imported with those summaries.

`bun nv chain`, `bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard`,
`bun nv splice` and `bun nv verify` are written, and `tools/verify.py` is deleted.
`tools/verify_keys.py` stays while `tools/loop.py` and `tools/impact.py` import it. `chain.py` stays
until the Stage 9 cutover. `main` is frozen. Tag `pre-overhaul` is the rollback.

The playbook still has two homes: `docs/agent/playbook/` (read by `playbook.py` and `orient.py`) and
`data/playbook/`. An edit to a bullet goes into both until Stage 9. Where they differ, the
`data/playbook/` record is the newer one (the playbook bullet on `nv import --write` says what to do).

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and
`bun nv selftest` runs every tools test.

## Next group

**Stage 8: the writers and the driver** — one file set: `tools/nv/cmd/orient.ts` (new),
`tools/orient.py`, `tools/nv/main.ts`, `tools/nv/parity/groups.json`. loop-goal.md § *Stage 8*
specifies it.

- [ ] **`bun nv orient` builds the pack.** Port `tools/orient.py:1847`'s `main` to
      `tools/nv/cmd/orient.ts`, register it in `tools/nv/main.ts:40`'s `COMMANDS`, and add a parity
      group to `tools/nv/parity/groups.json`. `[context]` resolves by id from the goal record, and
      the goal's position prints as `N of M` from `data/chain.json`. `tools/orient.py:1466` counts
      the live goal's `## Stage` headings; the goal record's `stages` replace that read.
- [ ] **Its parity is green.** `bun nv parity orient` matches `python tools/orient.py` on the live
      goal, with every difference declared in `tools/nv/parity/known.json` and its reason. The
      runner is `tools/nv/cmd/parity.ts:1`.

## Backlog

- `nv session --wrap`, `nv side`, `nv respawn` and the `nv loop` driver are the rest of Stage 8 —
  loop-goal.md § *Stage 8*.
- Stage 9 cutover: delete `dossier.py`'s `--emit-goals`, `--check-goals` and fan-out flags, and
  `generated_by` in `tools/loop.py`.
- `chain.py`'s `--show`, `--retire`, `--set` and `--retitle` have no `nv chain` successor yet.
- `docs/agent/playbook/` has no renderer from `data/playbook/`, and some legacy bullets are older than
  their records (one says `python bun nv splice`).
- `lints.py` has no `nv` successor; `nv verify`'s `lints` step runs it.
- `tools/verify_keys.py` goes when `tools/loop.py:70` and `tools/impact.py` read the keys from
  `tools/nv/keys/` instead.
