# Handoff

## State

**Goal `tooling-overhaul`: Stages 3, 4, 5 and 6 are landed.** `bun nv impact --probe` holds every
key to `data/impact-probes.json`, and all ten probes hold. `bun nv why "<name>"` prints a unit's
`source:` and its inputs grouped by partition. `impact --show <probe>` lists every unit an edit
moves, and it replaces `.agent-tmp/checks-smoke.ts`. `tools/nv/keys/checks.ts` gives each unit a
`role`, and a probe pattern matches `<role>: <name>`. An integration test binary now keys its
package's library at `shipped` (`testBuild` in `tools/nv/keys/key.ts`). The database matrix builds
each `cargo test` list its script writes (`suitesIn`). Nothing calls `checks.ts` from `verify`,
`impact.py`, `loop` or `proofs` yet: those are still Python and switch at the cutover. `data/` is a
snapshot, not yet the authority. `main` is frozen. Tag `pre-overhaul` is the rollback.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green. `tools/nv/lib/py.ts` holds what a port needs to print exactly what
Python printed. `bun x tsc --noEmit -p .` typechecks the tools.

What the probes do not assert yet, each named in `data/impact-probes.json`'s `what` or below:
- A kept unit keyed on everything counts as `wide`, not as a failure. The 14 binaries in
  `tools/data/impact-wide.txt` have no recorded reads, so every `cargo test -p` over one is wide.
- `new-example`'s keep list matches only wide units: every dossier group gate is wide. Stage 7's
  proof groups turn that cell into a real one.
- clippy, doc-tests and the builds are `verify` steps, not units. Their cells wait for the verify
  cutover.

## Next group

**Stage 7: proofs, the dossier becomes ordinary feature-proof machinery** — one file set:
`tools/nv/cmd/proofs.ts`, `tools/nv/proofs/**` and `tools/dossier.py`. loop-goal.md § *Stage 7* is
the spec.

- [ ] **`bun nv proofs`: the roster, and `--id`, `--owed` and `--gaps`**, ported from
      `tools/dossier.py:947` (`roster`) and the arguments at `tools/dossier.py:3381`, with a parity
      group in `tools/nv/parity/groups.json`. `rule:testing/feature-proofs` says what a feature owes.
- [ ] **`--verify` with several `--group` values over one roster**, from `tools/dossier.py:3381`.
      The driver's checks at `docs/agent/loop-goal.toml:13684` are the acceptance.
- [ ] **`--run --id <feature> --show`**: each program's `== <path>` line, its output, then
      `exit N · T ms`, as loop-goal.md § *Stage 7* words it. Anchor: `tools/dossier.py:3399`.

## Backlog

- Narrow the wide binaries: record their reads in `verify` and drop them from `tools/data/impact-wide.txt` — owner: loop-goal.md § *Stage 6*.
- Three binaries read every file under `crates/` as text (`directives`, `stdout_policy`, `tier_boundary`), so any `crates/` edit re-runs them — owner: `tools/nv/keys/checks.ts`.
- Probe cells for clippy, doc-tests and builds, once `verify` is ported — owner: loop-goal.md § *Stage 6*.
- Point the `proofs` cells of `test-module`, `stdlib-code` and `driver-tool` at Stage 7's proof units — owner: `data/impact-probes.json`.
