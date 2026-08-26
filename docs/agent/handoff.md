# Handoff

## State

**Conformance is at 530 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*). The tree is clean: the
`tainted`/`secret` work the previous handoff reported in flight has landed.

**`Core\Math` is 38 members over 21 cases.** The two added here take the family's remaining
*agreement* frontiers among the members that share a rule. The four rounding members are one rule
wherever a float has no fraction — 14 integer-valued rows including `±0.0`, `±2^52` and `±2^53` are
fixed points of all four, the sign of `-0.0` survives all four (asserted through the reciprocal,
because `==` cannot see it), and the bound is named on both sides: `2^52 - 0.5` is the last float
that straddles and `2^52` the first that cannot. On 12 fractional rows they split every time, and
`truncate` follows `floor` on the 6 positive and `ceil` on the 6 negative, which is the only thing
that tells it from either. `abs` and `sign` are the ordering trio written another way: `abs($x)` is
both `max($x, -$x)` and `-min($x, -$x)`, and `sign($x)` is what `max` and `min` independently say
about `$x` against zero, over 9 `int` and 10 `float` rows — and each parts from its derivation at
exactly the one value it refuses, `abs` at `int`'s smallest (where `0 - $x` wraps silently, playbook)
and `sign` at NaN (where the derivation lands on -1 because `max` drops a NaN). Both thin family
cases gained a pointer at their new neighbour.

## Next group

Three slices, all reading `crates/mwl-stdlib/src/math.rs` and adding new files under
`tests/conformance/core/`. `docs/spec/01-core-library.md` § 3 owns the rules; each is the
*agreement* shape from conventions.md. Note the float trap that governs all three: two spellings may
only be asserted equal on rows whose *intermediate* is exactly representable, or the case pins one
host's libm rather than a property (playbook, *Writing a test case*, the `hypot` bullet).

- [ ] **`intDiv`/`mod` satisfy the division identity, and `gcd`/`lcm` satisfy theirs** —
      `intDiv($a, $b) * $b + mod($a, $b) == $a` over a swept table of both signs, and
      `gcd($a, $b) * lcm($a, $b) == abs($a * $b)`, counted, with the rows each pair refuses named
      beside them. All-integer, so no exactness caveat applies.
      `crates/mwl-stdlib/src/math.rs:117` (`intDiv`), `:124` (`mod`), `:131` (`gcd`), `:138` (`lcm`).
- [ ] **Parity is exact on every host, so every odd and even member can be swept for it** — `sin`,
      `tan`, `sinh`, `tanh`, `asin`, `atan`, `asinh`, `atanh`, `cbrt` and `sign` are odd
      (`f(-$x) == -f($x)`), `cos` and `cosh` are even (`f(-$x) == f($x)`), and none of those
      equalities depends on how the host rounds, because both sides are the *same* libm call. Sweep
      one table of magnitudes across all twelve and count, with the domain-limited members
      (`asin`, `atanh`) kept inside their own interval. `crates/mwl-stdlib/src/math.rs:152` (`cbrt`),
      `:180`-`:194` (`sin`/`cos`/`tan`), `:201`-`:215` (`asin`/`acos`/`atan`),
      `:229`-`:264` (the six hyperbolics), `:61` (`sign`).
- [ ] **`sqrt`, `cbrt`, `exp` and `log` undo themselves only where the answer is exact** — square a
      `sqrt` and cube a `cbrt` over perfect squares, perfect cubes and dyadic fractions and count the
      round trip; then `log($x, $b)` on exact powers of the base. The rows that are *not* exact are
      the point of the case as much as the ones that are: name a couple and assert the round trip is
      merely near, not equal. `crates/mwl-stdlib/src/math.rs:145` (`sqrt`), `:152` (`cbrt`),
      `:166` (`exp`), `:173` (`log`).

## Backlog

- `Core\Math::toBase`/`fromBase` agree over a sweep, not just at the two refused bases — `math.rs:299`, `:306`.
- `Core\Math::format` against `Core\Str::format`'s numeric holes — `math.rs:313`.
- `Core\Json::decodeAs<T>` reads scalar fields only; no enum, `decimal`, `Instant`, array or nested class (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row (`mwl_stdlib::hash` module doc).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
