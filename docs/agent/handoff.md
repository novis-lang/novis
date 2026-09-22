# Handoff

## State

Goal `core-decimal` is six of its eight members in: `Core\Decimal::allocate`, `::ceil`, `::divExact`,
`::divRound`, `::floor` and `::pow` each own a description, three examples, an attack, a bench figure
and a `covers:` marker on a Rust case. `python tools/dossier.py --id 'Core\Decimal::<member>'`
reports each `complete.` `::truncate` and `::round` are what is left.

`divRound` and `floor` are attributed by widening a `covers:` line that already named a neighbour —
`div_exact_throws_where_div_round_rounds` and
`decimal_floor_ceil_truncate_and_round_answer_decimal_at_the_scale_asked` each assert both members
of their pair — so `::truncate` and `::round` are the same one-line edit to the second of those.

Nothing is blocked.

## Next group

The last two members of goal `core-decimal`, in the goal's own order — one file set:
`crates/nvs-stdlib/src/decimal.rs` for the `covers:` marker, plus a new directory each under
`docs/examples/core/Decimal/` and `tests/hostile/core/Decimal/`, and one file under
`benches/members/core/Decimal/`. One slice is one member with all its feature proofs
(`rule:testing/feature-proofs`), and both share the implementing file with the six that landed.

- [ ] **`Core\Decimal::truncate`** — owes examples, hostile, perf, tests. Its cut towards zero is
      what parts it from `::floor` and `::ceil` on a negative value, which is the example that
      earns its place. `crates/nvs-stdlib/src/decimal.rs:147`
- [ ] **`Core\Decimal::round`** — owes examples, hostile, perf, tests. It takes the mode before the
      scale, and is `::divRound` against `1`. `crates/nvs-stdlib/src/decimal.rs:156`

## Backlog

- A literal with a point is not placed from the other operand of an arithmetic expression, so
  `$price + 0.01` on a `decimal` is `E0455` while `$price + 1` compiles —
  `crates/nvs-types/src/expr/literals.rs` owns the placement, `rule:types/arithmetic` row 9 owns the
  refusal. Deciding whether the operand is a placement position is bigger than a proof slice.
- `Core\Decimal::pow`, `::floor` and `::divRound` each allocate twice per call at 64–97 ns
  (`docs/perf/members.ndjson`); nothing says whether a `decimal` answer owes two.
