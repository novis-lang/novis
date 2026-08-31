# Handoff

## State

**The failing acceptance check is closed.** `nvs-stdlib (Reflect, Ast, Decimal) [8 introspection]`
named `div_exact_throws_where_div_round_rounds`, which did not exist because the class it is about
did not: ADR 0054's *Still owed* line puts `Core\Decimal::divExact`/`divRound`/`allocate` at M8 and
none of the three had landed.

**`crates/nvs-stdlib/src/decimal.rs` is on disk**, `Core\Decimal` with the first two of those:
`divExact(decimal $value, decimal $divisor): decimal` and
`divRound(decimal $value, decimal $divisor, uint $scale, Core\RoundMode $mode): decimal`. The mode is
`Core\Math`'s enum, not a second one — `crate::math::RoundMode` and `round_mode` are `pub(crate)` now
and `round_mode` takes the caller's own name for its `FATAL`.

**Neither member re-implements division.** `nvs_runtime::decimal`'s new private `long_divide` is the
one long division: `Decimal::checked_div` is it plus half-even, `checked_div_exact` is it refusing a
remainder, and `checked_div_at_scale` is it stopped at a named scale handing back a public `Discard`.
`divRound` is deliberately **not** `checked_div` rounded a second time — that would carry a tie into
the second decision that the exact quotient never had. Both those module docs own the reasoning.

**`allocate`, `pow`, `floor`, `ceil` and `round` are the rest of ADR 0054's roster and are not in
this slice**; they are gap 1 of `crates/nvs-stdlib/src/decimal.rs`'s module doc, which is their home,
and `crate::math`'s own gap note is why the four rounding members land there rather than widening
`Core\Math`'s `float` ones.

`Core\Cli::displayWidth` is untouched for the second session running — the acceptance failure
outranked it both times. `orient.py` printed no section of ADR 0054 although the item turned out to
live entirely inside § 3: `[context] adrs` needs `0054:3`, and still needs `0019:1`, `0019:2`,
`0014:3`, `0086:1`, `0086:3` and `0033:4`; `[context] spec` still misses
`docs/spec/01-core-library.md` § 15.

## Next group

**`Core\Cli`'s last member and the case that pins the other half, over `crates/nvs-stdlib/src/cli.rs`
and `tests/conformance/core/`** — unchanged from the last two handoffs, and now with nothing ahead of
it.

- [ ] **`Core\Cli::displayWidth`** — ADR 0086 § 3's `displayWidth(string $value): uint`, UAX #11
      columns rather than `Core\Str::length`'s graphemes. The five edits of a `Core` member, at
      `crates/nvs-stdlib/src/cli.rs:213` (the rows — it goes after `colorDepth`, spec order),
      `crates/nvs-stdlib/src/cli.rs:362` (the cards, in row order beside `WIDTH_DOC`),
      `crates/nvs-stdlib/src/cli.rs:1020` (the `address()` arm) and
      `crates/nvs-stdlib/src/cli.rs:1286` (`nvs_core_cli_width`, the body to write beside). The
      decision to make first is where the width table comes from: a dependency is pre-authorized
      under ADR 0051 § 4 and owes the `[workspace.dependencies]` comment, `cargo deny check` and
      `python tools/gen-attribution.py`.
- [ ] **A `.nvst` case for `E0797`** — the refusal at
      `crates/nvs-types/src/expr/quals.rs:658`, reached from
      `crates/nvs-types/src/expr/calls.rs:343`. `--EXPECTF-ERROR--`, which has to reproduce the
      diagnostic's own indentation.

## Backlog

- `Core\Decimal::allocate($amount, $ratios)` — ADR 0054 § 3's penny split, the member no mainstream
  language ships; `crates/nvs-stdlib/src/decimal.rs`'s gap 1.
- `Core\Decimal::pow`, `floor`, `ceil` and `round` — ADR 0054 §§ 3-4; same gap, and
  `crates/nvs-stdlib/src/math.rs`'s own gap note is why they land there.
- `nvs_object_slot_get` and `Core\Reflect\ClassInfo::get` run no `onPropertyGet` — gap 3 of
  `crates/nvs-stdlib/src/reflect.rs`; changes a hot path, so it wants its own verify.
- A reflective write reaches storage rather than a hooked property's hook — same gap 3.
- `Core\IO::truncate` and `lock` — spec § 14, the two members of the handle half still owed.
- Reading `[log] target` — ADR 0020 § 6, the last of stage 7.
