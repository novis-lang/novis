# Handoff

## State

**Stage 0 items 1, 2, 3 and 9 are done, and item 4 is down to its § 5 remainder.** ADR 0090 § 2's numeric
domain now runs: a cross-representation pair — `$n == $f`, `$u == $n`, and any pairing with a `decimal` —
compares mathematically rather than by representation. The pairing is settled in `mwl-ir`'s `lower_binary`
beside the `decimal` and `Tagged` arms, as a new `Helper::NumericEq` over the new
`mwl_runtime::numeric_identical`, so `mwl-codegen`'s "a `BinOp` has one representation" invariant is intact
and its `ty != rty` refusal is still a genuine internal error. A *call* rather than a widening conversion
because none of the three widenings is exact — `Helper::NumericEq`'s own doc comment owns that reasoning,
and `integer_eq_float` reads a float at its own binary value rather than at the decimal it prints as.

`the_numeric_types_are_one_domain` is gone from `crates/mwl-types/tests/equality.rs`; those rows now run in
`tests/conformance/lang/equality-across-overlapping-types-still-compiles.mwlt`, which is the stronger
assertion and the one home for the fact.

`python tools/verify.py` is green (1331 tests). No valgrind run: this slice adds no refcount edge — every
representation in the new arm is a scalar, so neither operand is released.

**The one thing left in item 4, and it is the next group:** `==` now has two answers for the same numeric
pair. Two statically typed operands take § 3's table (`1 == 1.0` is `true`); one `mixed` operand takes
`value_identical`, which keeps PHP's `1 === 1.0` being `false`. `crates/mwl-runtime/src/identity.rs`'s
module doc *Known gap* states it in full, including why closing it is not one line: `value_hash` has to
canonicalize a numeric too, or `Arr::unique` indexes a set that disagrees with its own comparison.

## Next group — ADR 0090 § 5's numeric row (Stage 0 item 4, the last of it)

**Shared file set:** `crates/mwl-runtime/src/identity.rs` for both slices, then `crates/mwl-stdlib/src/arr.rs`
and one conformance case. [2] cannot land without [1] or the hash contradicts the comparison.

- [ ] **Item 4d — `value_identical`'s numeric rows become one domain** (ADR 0090 § 3's numeric row and
      § 5's "applies *3*'s table"). `shallow_identical` at
      [identity.rs:179](../../crates/mwl-runtime/src/identity.rs#L179) has three separate numeric arms
      (`Int|Uint`, `Float`, `Decimal`); they become one, delegating to `numeric_identical` at
      [identity.rs:336](../../crates/mwl-runtime/src/identity.rs#L336) extended with the `decimal` row that
      `crate::Decimal::compare_f64` and `Decimal::compare` already answer. **This decides `Core\Arr` too**
      and the decision is pre-authorized (loop-goal.md § *Standing decisions*, the strict-identity bullet):
      `Arr::contains([1.0], 1)` becomes `true`, because one comparison with two answers is the worse trap.
      Rewrite the module doc's `float`/`decimal` bullets and delete the *Known gap* section it replaces.
- [ ] **Item 4e — `value_hash` canonicalizes a numeric.** `hash_to_depth` at
      [identity.rs:256](../../crates/mwl-runtime/src/identity.rs#L256) writes a different discriminant for
      `Int|Uint`, `Float` and `Decimal`; identical values must hash equally, so an integral float and an
      integral decimal have to feed the same bytes as the `i128` does. A non-integral float and a
      non-integral decimal that are equal need the same treatment — hashing both as their `Decimal::reduced`
      form is the candidate, with `f64_parts` the existing reader. `arr.rs`'s `ObjectSet`-style wrapper at
      [arr.rs:2504](../../crates/mwl-stdlib/src/arr.rs#L2504) is the caller that makes this load-bearing;
      pin the new answer with a case under `tests/conformance/core/`.

## Backlog

- ADR 0047 § 4's literal and enum-case type atoms are refused by name — Stage 0 item 5, `mwl-types`.
- `$n + $f` and `$n < $f` type-check and still fail in `mwl-codegen` — ADR 0007 § 4's promotion table,
  `mwl-ir`'s gap 19.
- `private`/`protected` are enforced nowhere at an access site — Stage 0 item 6, `mwl-types`' gap list.
- `Comparable`/`Stringable` carry no member signatures, so `instanceof Stringable` panics `mwl-ir` — item 7.
- ADR 0061's `autoload` grammar and its name-to-file fixpoint — Stage 0 item 8, `mwl-hir`.
- An abandoned generator never runs the `finally` it is suspended inside — `mwl-ir`'s gap 18.

`loop-goal.toml`'s `[context] modules` gained `mwl-codegen/src/{emit,lib}.rs`, `mwl-runtime/src/helpers.rs`
and `decimal.rs`, `mwl-ir/src/ir.rs` and `mwl-stdlib/src/arr.rs`; `adrs` gained `0054 §5`. Nothing else
this session needed was missing from the pack.
