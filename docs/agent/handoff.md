# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3 and 4 are done.** Stage 4's two release-only guards are in
`benches/abi-probe/tests/perf_guards.rs`: `a_typed_decimal_arithmetic_loop_stays_in_its_cost_class`
(measured 39-40x a checked-return frame, ceiling 120) and
`the_linear_regex_tier_keeps_pace_with_the_backtracking_tier_on_one_corpus` (a floor of 1.0, measured
~2000x). Nothing is blocked.

The decimal guard compiles the **same program** `benches/members/` measures as
`lang:types/numbers-bool-int-uint-float-decimal`'s figure, so the ledger row and the gate cannot
drift; `docs/perf/members.ndjson` carries 61.2 ns/op for it and the guard prints 61 ns/iteration for
the same loop. The regex guard writes both patterns from Rust consts into one fixture and asserts
each tier with `nvs_stdlib::regex::validate` before timing, so a routing change fails loudly rather
than quietly comparing one engine with itself. The ceiling reasoning for each lives in the test's own
comment, not in a record.

## Next group

**Stage 5: the JSON wire** — one file set: `crates/nvs-stdlib/src/json.rs`,
`crates/nvs-types/src/derive.rs` and the one `.nvst` case under `tests/conformance/core/`.
`rule:core-classes/derive-field-list`, amended to the `Decided:` sentence where they disagree
(goal § *Standing decisions*).

- [ ] **`decimal`, `Instant` and an inline shape stop erasing to `CodecTy::Opaque`** —
      `crates/nvs-types/src/derive.rs:44` gap 1 and `crates/nvs-stdlib/src/json.rs:128` gap 1 are one
      knot: each type gets its wire form, a `CodecTy` arm and a `decode_field` case, and an
      `array<T>` of one of them follows for free. The check names
      `a_decimal_instant_or_shape_field_erases_to_a_codec_it_can_decode` in `nvs-types`.
- [ ] **`decodeAs` fills a field of each of the three kinds** — `crates/nvs-stdlib/src/json.rs:128`.
      Three named `#[test]`s: `decode_as_fills_a_decimal_field_from_the_numbers_own_digits`,
      `decode_as_fills_an_instant_field_from_an_rfc_3339_string`,
      `decode_as_fills_an_inline_shape_field`. A `decimal` is never read through `f64`.
- [ ] **A 25-digit decimal round-trips through `Core\Json` exactly** —
      `tests/conformance/core/json-decode-as-round-trips-a-25-digit-decimal-exactly.nvst`, which is
      ADR 0054's own M8 bullet at `docs/decisions/0054.md:251-252`.

## Backlog

- A leftover unit can land on a zero ratio that comes first; pinned by
  `tests/conformance/core/decimal-allocate-keeps-the-ratios-keys-and-the-amounts-sign.nvst`, and a
  decision to reopen only if skipping zero ratios is wanted.
- `Core\Math::ceil`/`floor`/`truncate`/`round` still answer `float` alone where spec § 3 writes
  `int|float|decimal`; `crates/nvs-stdlib/src/math.rs`'s module doc owns why that is the answer and
  not a gap.
- `lang:types/numbers-bool-int-uint-float-decimal` now has its perf proof and still owes tests,
  examples and hostile cases; those are the `dossier` goals' rows, not this one's.
