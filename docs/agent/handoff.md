# Handoff

## State

Milestone `dossier`, goal `core-debug`, **reached**. Both of its features —
`Core\Debug::dump` and `Core\Debug::render` — are `complete.` under
`python tools/dossier.py --id`: a description, three examples, an attack, a measured figure and a
Rust test carrying the `covers:` marker each. `python tools/verify.py` is 14 of 14 green,
`--doc` resolves every link, and `owners.py --closes core-debug` and `playbook.py --closes
core-debug` both name nothing. Nothing is blocked.

## Next group

**One slice is one feature, all its proofs together** — one file set:
`crates/nvs-stdlib/src/decimal.rs`, `docs/examples/core/Decimal/`, `tests/hostile/core/Decimal/`,
`benches/members/core/Decimal/`. Each item is `rule:testing/feature-proofs`, and the driver installs
goal `core-decimal` before the next session reads this.

- [ ] **`Core\Decimal::divExact`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/decimal.rs:88`
- [ ] **`Core\Decimal::divRound`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/decimal.rs:97`
- [ ] **`Core\Decimal::allocate`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/decimal.rs:111`

## Backlog

- `Core\Debug::render` counts 31 allocations and 1.5 kB for one rendering of a three-element array,
  which is an opportunity rather than a fault — `docs/perf/members.md`.
- `Core\Debug::dump` costs 4.5 µs against `render`'s 0.72 µs on the same host, the difference being
  one unbuffered write per call — `docs/perf/members.md`.
- The `about.md` a feature owes is still not counted by the sweep; goal `the-description-is-owed`
  switches it on — `docs/agent/goals/`.
