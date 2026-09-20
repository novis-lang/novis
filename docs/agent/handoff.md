# Handoff

## State

Goal `core-bigint-2-2` is met: all twenty-four `Core\BigInt` features carry every feature proof, and
both halves of the goal's `dossier: Core\BigInt` acceptance checks are green. This session wrote the
last three — `toInt`, `toUint` and `toDecimal` — and `python tools/dossier.py --record-perf --group
'Core\BigInt'` measured the whole class afresh.

`Core\BigInt::sign`'s attack found a real cost and it is fixed rather than recorded: `sign` answered
through `operand`, which copies a receiver's whole magnitude, so half a million calls on a
1048576-bit number took 25s against the case's own 20s bound. It reads the sign slot alone now
(`crates/nvs-stdlib/src/bigint.rs:768`), the case runs in 0.06s, and the member's record fell from
79.0 to 47.9 ns/op. Nothing is blocked.

## Next group

**Goal `core-budget` — `Core\Budget`, three features** — one file set:
`crates/nvs-stdlib/src/budget.rs` for the member and the Rust test, plus
`docs/examples/core/Budget/<member>/`, `tests/hostile/core/Budget/<member>/` and
`benches/members/core/Budget/<member>.nvs`. One slice is one feature with all of
`rule:testing/feature-proofs`'s proofs, and `python tools/dossier.py --id '<feature>'` prints the
path each belongs at. Every figure these three report is a live number rather than a stored one, so
each example prints what the program itself holds and each attack asks for more than the ceiling
allows.

- [ ] **`Core\Budget::memoryHeld`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/budget.rs:59`
- [ ] **`Core\Budget::memoryLimit`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/budget.rs:77`
- [ ] **`Core\Budget::memoryPeak`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/budget.rs:68`

## Backlog

- Every `Core\BigInt` member but `sign` still materializes its receiver through `operand`, so a
  member answering a scalar allocates once per call — `crates/nvs-stdlib/src/bigint.rs:768` is where
  a borrowed read would go, and `docs/perf/members.ndjson` holds the figures it would move.
