# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 6 have landed, and Stage 7 has started.** Every Stage 6 check
is green. `bun nv proofs` is `dossier.py`'s audit: the roster, the per-group status, `--group`,
`--id`, `--owed`, `--gaps`, `--gate`, `--json`, `--only` and `--no-perf`. Parity group `proofs`
matches in 20 of 20 cases. The one declared difference: `nv proofs` prints a feature's PHP twins,
because `dossier.py`'s `php_twins` reads the wrong spec cell. `tools/nv/proofs/roster.ts` is the
roster and `tools/nv/proofs/collect.ts` is the policy, the markers, the ledger and `owed`.
`dossier.py --group X` with no other flag crashes (`KeyError: 'help'` in `print_group`). `nv proofs`
prints that listing, so it has no parity case. Running the proofs, `--bless`, `--comments` and the
perf ledger are still `dossier.py`'s. `main` is frozen. Tag `pre-overhaul` is the rollback.

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
`tools/nv/cmd/proofs.ts`, `tools/nv/proofs/**` and `tools/dossier.py`. loop-goal.md § *Stage 7* is
the spec.

- [ ] **`--run` and `--verify` with several `--group` values over one roster**, ported from
      `tools/dossier.py:1449` (`run_suite`), `tools/dossier.py:1273` (`run_one_example`) and
      `tools/dossier.py:1317` (`run_one_hostile`), into `tools/nv/cmd/proofs.ts:241`
      (`run`). The check wants `lang:types`, `types:exception`, `nothing owed` and `0 failed`.
      `rule:testing/feature-proofs`.
- [ ] **`--run --id <feature> --show`**: each program's `== <path>` line, its output, then
      `exit N · T ms`, and an attack's `T ms of L ms`. Examples first, then attacks, then the
      bench. Written in `tools/nv/cmd/proofs.ts:241`.
- [ ] **`--comments` and `--bless`**, from `tools/dossier.py:730` and `tools/dossier.py:3354`.
      `commentProblems` is already at `tools/nv/proofs/collect.ts:129`.

## Backlog
- `nv why "proofs: lang:types"` must answer `source: observed` (Stage 7's third check). `tools/nv/keys/checks.ts` owns it.
- The proof profile and binary currency by the Stage 6 key, not by mtime. loop-goal.md § *Stage 7*.
- `impl_hash` at the card tier, the ledger rewrite and its decision record. `rule:testing/member-perf-ledger`.
- The `// dossier:` directives become `// proof:` by script. loop-goal.md § *Stage 7*.
- The policy is read from `tools/data/dossier-policy.toml` until the cutover makes `data/proofs/` the authority. `tools/nv/proofs/collect.ts`.
