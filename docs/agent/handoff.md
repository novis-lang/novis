# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2 and 3 are done.** `Core\Decimal`'s roster is complete at
`crates/nvs-stdlib/src/decimal.rs:84` — `divExact`, `divRound`, `allocate`, `pow`, and now the four
cuts `floor`, `ceil`, `truncate` and `round`, each with its named `#[test]` and three `.nvst` cases.
Both of stage 3's checks are green. Nothing is blocked.

The calls this session settled are in the module doc, not in a record — the goal's § *Standing
decisions* pre-authorized each. The four cuts are **one division against `1`**
(`Decimal::checked_div_at_scale`), so `round` and `divRound` reach the same `rounds_away` and agree
on every mode by construction rather than by two rules kept in step. `round` is spelled
`round($value, $mode, $scale = 0)`: a `defaults:` entry aligns to the *end* of the parameter list, so
a required mode and an optional scale can sit in no other order, which is the one place it parts from
`divRound`'s `$scale, $mode`. `floor` and `ceil` are the two answers no `Core\RoundMode` names —
`Up`/`Down` are directions about zero, these are directions along the line.

`crates/nvs-stdlib/src/math.rs`'s "Known gap: `decimal` at the four rounding members" is gone:
`Core\Math`'s four stay `float`'s, and its module doc now says why and where the `decimal` ones are.

## Next group

**Stage 4: the two release-only cost guards** — one file:
`benches/abi-probe/tests/perf_guards.rs` (the shape at `:1-15` is a release-only test with a loose
threshold that prints its own figure), plus wherever `python tools/dossier.py --id` puts a feature's
bench program (`benches/members/README.md` § *Where a bench goes*). Both items are measured claims a
record makes, so the record section is what specifies them.

- [ ] **A typed `decimal` arithmetic loop as a figure with a guard, beside the `int` one** —
      `benches/abi-probe/tests/perf_guards.rs:676`. `docs/decisions/0054.md:246-250` is the claim.
      The check names the test `a_typed_decimal_arithmetic_loop_stays_in_its_cost_class`, and the
      whole stage is release-only: `cargo test --release -p nvs-abi-probe --test perf_guards`.
- [ ] **The linear regex tier's throughput against the backtracking tier's, on one shared corpus** —
      `benches/abi-probe/tests/perf_guards.rs:1-15` for the shape. `docs/decisions/0056.md:146-148`
      is the claim being measured, "the default tier is not a performance concession". The check
      names `the_linear_regex_tier_keeps_pace_with_the_backtracking_tier_on_one_corpus`.

## Backlog

- A leftover unit can land on a zero ratio that comes first; pinned by
  `tests/conformance/core/decimal-allocate-keeps-the-ratios-keys-and-the-amounts-sign.nvst`, and a
  decision to reopen only if skipping zero ratios is wanted.
- `Core\Math::ceil`/`floor`/`truncate`/`round` still answer `float` alone where spec § 3 writes
  `int|float|decimal`; `crates/nvs-stdlib/src/math.rs`'s module doc owns why that is the answer and
  not a gap.
