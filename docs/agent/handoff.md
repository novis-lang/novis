# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 6 have landed, and Stage 7 is in progress.** `bun nv proofs` is
`dossier.py`'s audit (parity group `proofs`, 20 of 20) plus `--run` and `--verify`, which take
`--group` more than once, `--id`, `--only`, `--valgrind`, `--quiet`, `--no-cache`, `--strict` and
`--show`. `tools/nv/proofs/run.ts` runs and judges the programs on the proof binary
(`target/proof/`), rebuilt when the Stage 6 key of `nvs-cli` at the `shipped` tier moves; the key it
was built at is `target/proof/nvs.key`, and green verdicts live in `.loop/proofs-green.json`. Checks
1 and 3 of Stage 7 print every `want` line. `--run` takes no suite name: it runs examples and attacks
both, and a `--run` output cannot be a parity case because its counts carry times and cache hits.
`--bless`, `--comments` and the perf ledger are still `dossier.py`'s. `dossier.py --group X` alone
crashes (`KeyError: 'help'`), so that listing has no parity case. `main` is frozen. Tag
`pre-overhaul` is the rollback.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`. Never prove a cut with a sweep. A Python tool is deleted only after its
replacement's parity is green. `tools/nv/lib/py.ts` holds what a port needs to print exactly what
Python printed, and `bun x tsc --noEmit -p .` typechecks the tools. `bun nv parity proofs` takes
about a minute.

What the probes do not assert yet:
- A kept unit keyed on everything counts as `wide`. The 14 binaries in
  `tools/data/impact-wide.txt` have no recorded reads.
- `new-example`'s keep list matches only wide units. Stage 7's proof groups make it a real cell.
- clippy, doc-tests and the builds are `verify` steps, not units, until the verify cutover.

## Next group

**Stage 7: proofs, the dossier becomes ordinary feature-proof machinery** — one file set:
`tools/nv/keys/checks.ts`, `tools/nv/cmd/why.ts`, `tools/nv/cmd/proofs.ts` and `tools/nv/proofs/**`.
loop-goal.md § *Stage 7* is the spec.

- [ ] **A proofs check is a unit keyed on what it read**: `bun nv why "proofs: lang:types"` must
      print `source: observed`, so each group a `--verify` names is a unit `proofs: <group>` in
      `tools/nv/keys/checks.ts:260` (`units`), keyed on the proof binary's key
      (`tools/nv/proofs/run.ts:69`, `proofBinary`) and the group's example and attack files.
      `rule:testing/feature-proofs`.
- [ ] **`--comments` and `--bless`**, from `tools/dossier.py:730` and `tools/dossier.py:3354`, into
      `tools/nv/cmd/proofs.ts:308` (`run`); `--bless` runs on `tools/nv/proofs/run.ts:69`'s binary.
- [ ] **The directive rename**: `// dossier: exit N` to `// proof: exit N` and `// dossier:
      known-gap` to `// proof: gap <gap-id>`, by one script under `.agent-tmp/`, with the regexes in
      `tools/nv/proofs/run.ts:50` and `tools/nv/proofs/collect.ts:166`.

## Backlog

- `--record-perf` and the perf report, and the perf-currency decision record — loop-goal.md § *Stage 7*.
- The driver runs every proof check a sweep owes in one `nv proofs` call — loop-goal.md § *Stage 7*.
- `nv disk` counts `target/proof/` — loop-goal.md § *Stage 7*.
- Deleting `dossier.py`'s fan-out and goal emission — loop-goal.md § *Stage 7*.
