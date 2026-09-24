# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 6 have landed, and Stage 7 is in progress. All three Stage 7
checks print their `want` lines.** `bun nv proofs` is `dossier.py`'s audit, `--run`, `--verify`,
`--bless` and `--comments` (parity group `proofs`, 22 of 22). The directives are now `// proof: exit N`
and `// proof: gap <gap id>`. A gap marker names a record under `data/gaps/`, and both `nv proofs` and
`dossier.py` print that record's title (`gapTitle` in `tools/nv/proofs/collect.ts`, `gap_title` in
`dossier.py`). Each `--group` that a check names is a unit `proofs: <group>` (`tools/nv/keys/checks.ts`).
The perf ledger (`--record-perf`, the report) is still `dossier.py`'s. `main` is frozen. Tag
`pre-overhaul` is the rollback.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, and that script is written with Write, never a heredoc. Never prove a cut with a sweep.
A Python tool is deleted only after its replacement's parity is green. `tools/nv/lib/py.ts` holds what
a port needs to print exactly what Python printed (`comparePaths` is Python's `Path` order), and
`bun x tsc --noEmit -p .` typechecks the tools. `bun nv parity proofs` takes about a minute.

What the probes do not assert yet:
- A kept unit keyed on everything counts as `wide`. The 14 binaries in
  `tools/data/impact-wide.txt` have no recorded reads.
- `new-example`'s keep list matches only wide units. No probe in `tools/data/impact-probes.json`
  names a `proofs: <group>` unit yet.
- clippy, doc-tests and the builds are `verify` steps, not units, until the verify cutover.

The acceptance line `bun nv chain --check` is Stage 8's, and `nv chain` is not written yet.

## Next group

**Stage 7: proofs, the dossier becomes ordinary feature-proof machinery** — one file set:
`tools/nv/cmd/proofs.ts`, `tools/nv/proofs/**` and `docs/perf/members.ndjson`.
loop-goal.md § *Stage 7* is the spec.

- [ ] **`--record-perf` and the perf report**, from `tools/dossier.py:1686` and
      `tools/dossier.py:1787`, into `tools/nv/cmd/proofs.ts`. They run on the release binary.
      `rule:testing/member-perf-ledger`.
- [ ] **Perf currency ignores comments**: `tools/nv/proofs/collect.ts:301` (`implHash`) hashes at
      the card tier, the same commit rewrites the matching `impl_hash`es in
      `docs/perf/members.ndjson`, and the rule's currency paragraph is rewritten whole with one new
      decision record. `rule:testing/member-perf-ledger`.

## Backlog

- The driver runs every proof check a sweep owes in one `nv proofs` call, memoised per `proofs: <group>` unit — loop-goal.md § *Stage 7*.
- `nv disk` counts `target/proof/` — loop-goal.md § *Stage 7*.
- Deleting `dossier.py`'s fan-out and goal emission — loop-goal.md § *Stage 7*.
- An impact probe that edits one group's example and keeps every other `proofs:` unit — `tools/data/impact-probes.json`.
- The `known-gap` word in the run's count lines (`0 known-gap`) stays until `dossier.py` is deleted, because parity compares it — `tools/nv/proofs/run.ts`.
