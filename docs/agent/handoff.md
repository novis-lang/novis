# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 6 have landed, and Stage 7 is in progress. All three Stage 7
checks print their `want` lines.** `bun nv proofs` is `dossier.py`'s audit, `--run`, `--verify`,
`--bless`, `--comments`, `--record-perf` and `--perf-report` (parity group `proofs`, 22 of 22). The
perf ledger's code is `tools/nv/proofs/perf.ts`. `--record-perf` runs on the release binary, which
`releaseBinary` in `tools/nv/proofs/run.ts` builds by the Stage 6 key. `bun nv disk` prints
`target/proof/` on a line of its own, and counts its incremental caches with debug's and release's.
`main` is frozen. Tag `pre-overhaul` is the rollback.

Perf currency is the implementing file's `card` tier (ADR 0220, `rule:testing/member-perf-ledger`).
`implHash` in `tools/nv/proofs/collect.ts` is the only hash. `dossier.py` reads it from
`bun nv proofs --impl-hash FILE...`, in one call per audit. Nothing in `dossier.py` is deleted yet.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, and that script is written with Write, never a heredoc. Never prove a cut with a sweep.
A Python tool is deleted only after its replacement's parity is green. `tools/nv/lib/py.ts` holds what
a port needs to print exactly what Python printed, and `bun x tsc --noEmit -p .` typechecks the tools.
`bun nv parity proofs` takes about a minute. To try `--record-perf` by hand, copy
`docs/perf/members.ndjson` under `.agent-tmp/` first and copy it back afterwards, and pass `--reps 3`
or more. `nv disk` has no parity group, so a change to its report is checked by running it.

What the probes do not assert yet:
- A kept unit keyed on everything counts as `wide`. The 14 binaries in
  `tools/data/impact-wide.txt` have no recorded reads.
- `new-example`'s keep list matches only wide units. No probe in `tools/data/impact-probes.json`
  names a `proofs: <group>` unit yet.
- clippy, doc-tests and the builds are `verify` steps, not units, until the verify cutover.

The acceptance line `bun nv chain --check` is Stage 8's, and `nv chain` is not written yet. It is an
artefact not written yet, not a regression.

## Next group

**Stage 7: proofs, the dossier becomes ordinary feature-proof machinery** — one file set:
`tools/loop.py`'s proof-check handling, `tools/nv/proofs/**`, `tools/data/impact-probes.json`.
loop-goal.md § *Stage 7* is the spec.

- [ ] **The driver runs every proof check a sweep owes in one `nv proofs` call**, and hands each check
      its own group's verdict lines, memoised per `proofs: <group>` unit: `tools/loop.py:2276`
      (`runs_release_cli`, which still matches `dossier.py` argvs). loop-goal.md § *Stage 7*, "One pass".
- [ ] **An impact probe edits one group's example and keeps every other `proofs:` unit**:
      `tools/data/impact-probes.json:1`. `rule:testing/feature-proofs`; loop-goal.md § *Stage 7*.

## Backlog

- Deleting `dossier.py`'s fan-out, goal emission and perf ledger. Floor checks at `docs/agent/loop-goal.toml:12293` and `:12317` still run `--emit-goals --dry-run` and `--check-goals` — loop-goal.md § *Stage 7*.
- `docs/perf/members.md` is written by `nv proofs --perf-report`, not by `nv render`, which the entity table lists it under — loop-goal.md § *The data*.
- The `known-gap` word in the run's count lines (`0 known-gap`) stays until `dossier.py` is deleted, because parity compares it — `tools/nv/proofs/run.ts`.
