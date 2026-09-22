# Handoff

## State

Goal `core-decimal` is three of its eight members in: `Core\Decimal::allocate`, `::ceil` and
`::divExact` each own a description, three examples, an attack, a bench figure and a `covers:`
marker on a Rust case. `python tools/dossier.py --id 'Core\Decimal::<member>'` reports each
`complete.`

Writing the `allocate` attack found a compiler defect and it is fixed on the way past: a negated
number literal took no placement, so `decimal $owed = -19.99;` was `E0401: expected decimal, found
float` while the positive spelling needed nothing, and a negative `decimal` constant was writable
only as `-19.99 as decimal`. `negated_literal_expectation` now passes an exactly-`decimal` target through to
the operand (`crates/nvs-types/src/expr/literals.rs:209`); `uint`, the literal types and a union's
arms are deliberately still out, so `-e` is placed exactly where `e` is.

Nothing is blocked.

## Next group

The remaining five members of goal `core-decimal`, in the goal's own order — one file set:
`crates/nvs-stdlib/src/decimal.rs` for the reference card and the Rust case that gets the marker,
plus a new directory each under `docs/examples/core/Decimal/`, `tests/hostile/core/Decimal/` and one
file under `benches/members/core/Decimal/`. One slice is one member with all its feature proofs
(`rule:testing/feature-proofs`), and neighbours share the implementing file, so the second and third
cost a fraction of the first.

- [ ] **`Core\Decimal::divRound`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/decimal.rs:97`
- [ ] **`Core\Decimal::floor`** — owes examples, hostile, perf, tests. Its Rust case is
      `decimal_floor_ceil_truncate_and_round_answer_decimal_at_the_scale_asked`, which already
      carries `ceil`'s marker and takes a second line.
      `crates/nvs-stdlib/src/decimal.rs:129`
- [ ] **`Core\Decimal::pow`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/decimal.rs:120`

## Backlog

- `::round` and `::truncate` are the goal's last two items, same file set — `docs/agent/loop-goal.md`.
- A scalar-returning `Core\Decimal` member costs 2 allocations and 34 bytes per call, because a
  `decimal` does not fit a `Value`'s inline payload; not a defect, recorded so a representation
  change has a baseline — `docs/perf/members.md`.
- The `.nvst` cases for these three are credited by a plain call rather than a `covers:` marker,
  which `rule:testing/proof-attribution` allows for a `Core` member and nothing else.
