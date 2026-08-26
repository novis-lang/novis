# Handoff

## State

**Conformance is at 524 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*).

**`Core\Math` is 38 members over 17 cases.** The two added here take the family's two float
frontiers. The first asserts `hypot` answers exactly what `sqrt($x * $x + $y * $y)` answers on
nineteen rows whose square sum is *exactly representable* — triples, zeros, signs, dyadic fractions
and `3 * 2^52` — which is what makes the agreement a property rather than this host's libm
(playbook, *Writing a test case*), and then names the two rows where the naive form loses: `3e200`
squared is an infinity while `hypot` still answers `5.0E+200`, and `3e-200` squared underflows to a
finite, plausible, wrong `0`. The second sweeps one nine-row table straddling `|x| > 1`, `|x| = 1`
and `|x| < 1` through six inverse members at once, counting rather than reading off lines: `asin`
and `acos` agree about their shared domain on 9 of 9 rows, `acosh` is NaN on the six the other two
accept five of, `atanh` partitions all nine into finite / infinite-at-the-pole / NaN with exactly
one holding per row, `atan` and `asinh` are finite on 18 of 18, and `threw=0` asserts § 3's
"IEEE's value, not a throw" rather than inferring it from the case having got that far. The two
existing family cases keep what they claim and each gained a one-line pointer at its new neighbour.

## Next group

Three slices, all reading `crates/mwl-stdlib/src/math.rs` and adding new files under
`tests/conformance/core/`. `docs/spec/01-core-library.md` § 3 owns the rules; each slice is the
*agreement* shape from conventions.md, which is where this family has the most room left.

- [ ] **`toRadians` and `toDegrees` are inverses over a sweep, and agree with `PI` and `TAU` at the
      quarter turns** — count the round-trip rows that return the degree they started at over a
      table of 0/90/180/270/360 and both signs, then name the four quarter turns against
      `PI / 2`-style constant expressions rather than against a frozen decimal.
      `crates/mwl-stdlib/src/math.rs:271` (`toRadians`), `:278` (`toDegrees`), `:342` (`PI`),
      `:347` (`TAU`).
- [ ] **`clamp` agrees with `min(max($x, $lo), $hi)` on every row** — the composition is the
      definition, so counting the rows where the two spellings answer the same value is what a
      `clamp` that grew its own bound check fails, while `math-clamp-is-closed-at-both-ends.mwlt`
      keeps the endpoints. `crates/mwl-stdlib/src/math.rs:82` (`clamp`), `:68` (`min`), `:75`
      (`max`).
- [ ] **The four rounding members agree on every integer-valued float and split only where the
      sign says they must** — `ceil`/`floor`/`truncate`/`round` over a table straddling zero,
      counting the rows where all four agree (the integer-valued ones) and asserting `truncate`
      follows `ceil` below zero and `floor` above it, which is the one rule that separates it from
      `floor` at all. `crates/mwl-stdlib/src/math.rs:89` (`ceil`), `:96` (`floor`), `:103`
      (`truncate`), `:110` (`round`).

## Backlog

- `Core\Json::decodeAs<T>` reads scalar fields only — no enum, `decimal`, `Instant`, nested class or
  optional key (`mwl_stdlib::json` gap 2, ADR 0071).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row, so
  `Core\Hash::hmac`'s key is a plain `CoreTy::Bytes` (`mwl_stdlib::hash`'s module doc).
- ADR 0086 § 1's substitution table is unbuilt, so no terminal sink neutralizes a control byte
  (`crates/mwl-stdlib/src/cli.rs` gap 1, M8).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- A `?bool` cannot be tested for truth, so a member answering one has no `yn` rendering
  (implementation-plan, *Open now*).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
