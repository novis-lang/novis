# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 6 have landed, and Stage 7 is in progress. All three Stage 7
checks print their `want` lines.** `bun nv proofs` is `dossier.py`'s audit, `--run`, `--verify`,
`--bless`, `--comments`, `--record-perf` and `--perf-report` (parity group `proofs`, 22 of 22). The
perf ledger's code is `tools/nv/proofs/perf.ts`. `--record-perf` runs on the release binary, which
`releaseBinary` in `tools/nv/proofs/run.ts` builds by the Stage 6 key. `main` is frozen. Tag
`pre-overhaul` is the rollback.

The driver batches its proof checks. `proof_groups` in `tools/loop.py` names a check that is
`bun nv proofs --verify` with only `--group` flags. `Goal.proofs_verify` runs every such check the
cargo tier owes in one `nv proofs` call, and `split_proofs` hands each check its own groups' sections.
With several groups, `nv proofs` closes each section on `-- <group>: passed` or `-- <group>: failed`.
`Goal.proof_inputs` keys each check on its groups' `proofs:` units, and mirrors `proofParts` in
`tools/nv/keys/checks.ts`. The floor's `dossier.py` checks are unchanged. They join the batch when
the cutover rewrites their `argv`s. The probe `one-group-example` in `data/impact-probes.json` holds.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, and that script is written with Write, never a heredoc. Never prove a cut with a sweep.
A Python tool is deleted only after its replacement's parity is green. `tools/nv/lib/py.ts` holds what
a port needs to print exactly what Python printed, and `bun x tsc --noEmit -p .` typechecks the tools.
`bun nv parity proofs` takes about a minute. Impact probes live in `data/impact-probes.json`, not under
`tools/data/`. `loop.py` has no test suite. A driver change is proved by a probe script under
`.agent-tmp/` that builds a `Goal` from `docs/agent/loop-goal.toml` and stubs `capture`.

What the probes do not assert yet:
- A kept unit keyed on everything counts as `wide`. The 14 binaries in
  `tools/data/impact-wide.txt` have no recorded reads.
- `new-example`'s keep list matches only wide units.
- clippy, doc-tests and the builds are `verify` steps, not units, until the verify cutover.

The acceptance line `bun nv chain --check` is Stage 8's, and `nv chain` is not written yet. It is an
artefact not written yet, not a regression.

## Next group

**Stage 7: proofs, the two binaries in the driver** — one file set: `tools/loop.py`'s proof-check
handling, `tools/nv/cmd/proofs.ts`. loop-goal.md § *Stage 7*, "Two binaries", is the spec.

- [ ] **A gate-open sweep runs its proof checks on the release binary**: `Goal.proofs_verify` at
      `tools/loop.py:2739` passes `--nvs` with the release CLI when `floor_gate` is open. It joins the
      prebuild first and holds `RELEASE_CLI_LOCK`, as `runs_release_cli` at `tools/loop.py:2280` does
      for `dossier.py`. `Goal.release_cli` at `tools/loop.py:3103` is the build. A shut-gate sweep
      stays on the proof binary. loop-goal.md § *Stage 7*, "Two binaries".
- [ ] **`runs_release_cli` also matches `bun nv proofs --record-perf`**, which runs the release
      binary: `tools/loop.py:2280`. Its docstring is rewritten whole.
- [ ] **Stage 7 is closed out** against loop-goal.md § *Stage 7*, point by point, and whatever it
      still names is listed here as the next group: `tools/loop.py:2280`.

## Backlog

- Stage 8, `nv chain` and the rest of the writers: loop-goal.md § *Stage 8*.
- `new-example`'s keep list has no narrow `proofs:` unit to name until the floor is cut over:
  `data/impact-probes.json`.
