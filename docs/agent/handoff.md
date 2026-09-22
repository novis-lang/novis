# Handoff

## State

Goal `core-decimal` is met: all eight members — `Core\Decimal::allocate`, `::ceil`, `::divExact`,
`::divRound`, `::floor`, `::pow`, `::truncate` and `::round` — own a description, three examples, an
attack, a measured figure and a `covers:` marker on a Rust case. `python tools/dossier.py --group
'Core\Decimal' --owed` reports `0 features`.

`::truncate` and `::round` are attributed by the same Rust case as `::ceil` and `::floor`,
`decimal_floor_ceil_truncate_and_round_answer_decimal_at_the_scale_asked`, which already asserted
all four: its `covers:` line now names all four members.

The floor also turned up a flake outside this goal and it is fixed: `crate::stop::ONE_STOP_AT_A_TIME`
moved out of `serve.rs`'s `mod tests` so the third case that stops the process takes it too.

Nothing is blocked.

## Next group

The first members of goal `core-encoding`, which the chain reaches next — one file set:
`crates/nvs-stdlib/src/encoding.rs` for the `covers:` markers, plus a new directory each under
`docs/examples/core/Encoding/` and `tests/hostile/core/Encoding/`, and one file under
`benches/members/core/Encoding/`. One slice is one member with all its feature proofs
(`rule:testing/feature-proofs`). That goal ships its own generated handoff, so this section points
at its first two items rather than replacing them.

- [ ] **`Core\Encoding::encodeText`** — owes examples, hostile, perf, tests. It takes the text and
      the encoding name, so what parts it from a plain write is the encoding it names.
      `crates/nvs-stdlib/src/encoding.rs:341`
- [ ] **`Core\Encoding::decodeText`** — owes examples, hostile, perf, tests. Its refusal on bytes
      no encoding can read is the attack. `crates/nvs-stdlib/src/encoding.rs:350`

## Backlog

- `Core\Decimal` has no instance members, so nothing in the class is owed beyond the eight —
  `crates/nvs-stdlib/src/decimal.rs`'s registry `instance: &[]`.
- The figures for `::truncate` and `::round` are 93 ns/op each, 3 statements, 2 allocations —
  `docs/perf/members.ndjson`.
