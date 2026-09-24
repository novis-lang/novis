# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 6 have landed, and Stage 7 is in progress. All three Stage 7
checks print their `want` lines.** `bun nv proofs` is `dossier.py`'s audit, `--run`, `--verify`,
`--bless`, `--comments`, `--record-perf` and `--perf-report` (parity group `proofs`, 22 of 22). The
perf ledger's code is `tools/nv/proofs/perf.ts`. `--record-perf` runs on the release binary, which
`releaseBinary` in `tools/nv/proofs/run.ts` builds by the Stage 6 key, the same rule `proofBinary`
follows. A `--perf-report` from `nv` matches `dossier.py`'s report except for the two header lines
that name the tool. Nothing in `dossier.py` is deleted yet. `main` is frozen. Tag `pre-overhaul` is
the rollback.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, and that script is written with Write, never a heredoc. Never prove a cut with a sweep.
A Python tool is deleted only after its replacement's parity is green. `tools/nv/lib/py.ts` holds what
a port needs to print exactly what Python printed (`comparePaths` is Python's `Path` order, `fixed` is
`format(x, ".Nf")`), and `bun x tsc --noEmit -p .` typechecks the tools. `bun nv parity proofs` takes
about a minute. To try `--record-perf` by hand, copy `docs/perf/members.ndjson` under `.agent-tmp/`
first and copy it back afterwards, and pass `--reps 3` or more: one rep is refused as noise.

What the probes do not assert yet:
- A kept unit keyed on everything counts as `wide`. The 14 binaries in
  `tools/data/impact-wide.txt` have no recorded reads.
- `new-example`'s keep list matches only wide units. No probe in `tools/data/impact-probes.json`
  names a `proofs: <group>` unit yet.
- clippy, doc-tests and the builds are `verify` steps, not units, until the verify cutover.

The acceptance line `bun nv chain --check` is Stage 8's, and `nv chain` is not written yet.

## Next group

**Stage 7: proofs, the dossier becomes ordinary feature-proof machinery** — one file set:
`tools/nv/proofs/**`, `docs/perf/members.ndjson`, `docs/rules/testing/member-perf-ledger.md` and
one new decision record. loop-goal.md § *Stage 7* is the spec.

- [ ] **Perf currency ignores comments, layout and cards**: `tools/nv/proofs/collect.ts:302`
      (`implHash`) hashes the implementing file at the card tier, from `analyse` at
      `tools/nv/keys/scan.ts:221`. `tools/nv/proofs/perf.ts:253` writes that hash into each new
      record. The same commit rewrites every `impl_hash` in `docs/perf/members.ndjson` whose old-form
      hash matches the file on disk, with a script under `.agent-tmp/`. The currency paragraph of
      `rule:testing/member-perf-ledger` ("A figure is re-measured only when") is rewritten whole,
      with one new decision record (check the next free number just before writing it). Then
      `python tools/rules.py --render`.
- [ ] **`dossier.py` reads the new hash too, or is cut first**: `tools/dossier.py:1593`
      (`impl_hash`) must agree with `implHash` while `dossier.py` still gates anything, or its audit
      and `nv proofs`'s disagree on what perf is owed. Parity group `proofs` shows it.

## Backlog

- The driver runs every proof check a sweep owes in one `nv proofs` call, memoised per `proofs: <group>` unit — loop-goal.md § *Stage 7*.
- `nv disk` counts `target/proof/` — loop-goal.md § *Stage 7*.
- Deleting `dossier.py`'s fan-out, goal emission and perf ledger — loop-goal.md § *Stage 7*.
- `docs/perf/members.md` is written by `nv proofs --perf-report`, not by `nv render`, which the entity table lists it under — loop-goal.md § *The data*.
- An impact probe that edits one group's example and keeps every other `proofs:` unit — `tools/data/impact-probes.json`.
- The `known-gap` word in the run's count lines (`0 known-gap`) stays until `dossier.py` is deleted, because parity compares it — `tools/nv/proofs/run.ts`.
