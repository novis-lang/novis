# Handoff

## State

**Conformance is at 534 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*). The tree is clean.

**`Core\Math` is 38 members over 25 cases**, and the two added here take the family's *round trip*
and *quadrant* frontiers. `sqrt`, `cbrt`, `exp` and `log` undo themselves exactly where the
intermediate they go through is representable and merely near everywhere else: 18 perfect squares
and 18 perfect cubes round trip on the nose, 13 exact powers of base 2 and base 10 come back as
their own integer exponent, and `exp`/`log` is held to a relative rounding step because it has no
exact row but the fixed point. The inexact rows are all square roots and that is a claim rather
than an omission — `sqrt` is the only root member IEEE 754 requires to be correctly rounded, and
`cbrt`'s irrational rows are where the two legs actually disagree (playbook, *Writing a test
case*). `atan2` is asserted against `atan` of the quotient over 32 rows in all four quadrants: it
agrees exactly on the 16 with a positive `$x` and is a half turn away on the other 16, which
partitions the table, and then answers on the five kinds of row a quotient cannot carry at all —
both signed zeros of `$y` on each x-axis, all four sign pairs at the origin, and the four corners
at infinity. Both cases were run on the WSL leg as well as the native one before being written
down, which is what a float agreement case owes.

## Next group

Three slices, all reading `crates/mwl-stdlib/src/math.rs` and adding new files under
`tests/conformance/core/`. `docs/spec/01-core-library.md` § 3 owns the rules; each is the
*agreement* shape from conventions.md. The tolerance spelling all three need is now a playbook
bullet under *Writing a test case*, beside the `cbrt` one — do not re-derive it.

- [ ] **Every `Core\Math` constant is what its own members answer at a landmark** — `PI` against
      `acos(-1.0)`, `E` against `exp(1.0)`, `TAU` against `PI + PI`, `INT_MAX`/`INT_MIN` against the
      values `intDiv`/`gcd` stop at, and `EPSILON` bounded on both sides as the smallest float with
      `1.0 + EPSILON != 1.0` while half of it is not. Thin neighbour to point at:
      `math-constants-are-class-constants.mwlt`. `crates/mwl-stdlib/src/math.rs:340` (the table),
      `:357` (`EPSILON`), `:362` (`INT_MAX`), `:387` (`NAN`), `:392` (`INFINITY`).
- [ ] **`sin`, `cos` and `tan` are one circle, and the identity is what says so** — `sin($x) *
      sin($x) + cos($x) * cos($x)` is 1 to a rounding step on every row of a swept table, `tan($x)`
      is `sin($x) / cos($x)` to the same step, and the quarter turns are where `tan` has no answer
      and hands back the enormous finite value the rounded `PI / 2.0` earns rather than an infinity.
      `crates/mwl-stdlib/src/math.rs:180` (`sin`), `:187` (`cos`), `:194` (`tan`).
- [ ] **The six hyperbolic members are their exponential definitions** — `sinh` against
      `(exp($x) - exp(0.0 - $x)) / 2.0` and `cosh` against the sum, `cosh * cosh - sinh * sinh`
      is 1, `tanh` is the ratio, and `asinh`/`acosh`/`atanh` undo the three over a table — all to a
      relative step, never to equality, since none of the six is correctly rounded.
      `crates/mwl-stdlib/src/math.rs:229` (`sinh`), `:250` (`asinh`), `:264` (`atanh`).

## Backlog

- `Core\Math::min`/`max` over the `int|float` union: where the two representations meet, and what
  ordering they take — `docs/spec/01-core-library.md` § 3.
- `Core\Json::decodeAs<T>`'s decoder reads a scalar-fielded class only — ADR 0071, `mwl_stdlib::json`
  gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- ADR 0086 § 1's substitution table, which the terminal sink needs — `crates/mwl-stdlib/src/cli.rs`
  gap 1.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1.
