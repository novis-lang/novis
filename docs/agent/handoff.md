# Handoff

## State

**Conformance is at 532 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*). The tree is clean.

**`Core\Math` is 38 members over 23 cases**, and the two added here take the family's cross-member
*identity* frontiers. `intDiv` and `mod` are one division: over 88 rows of both signs the quotient
and the remainder reconstruct the dividend, the remainder is smaller than the divisor and carries the
dividend's sign — the three conditions that *are* truncated division — and `mod` answers on every
row exactly what the `%` operator answers, which is the division of labour spec § 3 states. `gcd` and
`lcm` are one factorization: over 100 rows their product is the magnitude of the arguments' product,
neither is ever negative, the divisor divides both and the multiple is a multiple of both. Each
pair's refusals are named as the rows where the identity has no left-hand side, bounded on both
sides — `gcd(INT_MIN, 0)` refuses where `gcd(INT_MIN + 1, 0)` answers `INT_MAX`, and `lcm(INT_MAX, 2)`
refuses where `lcm(INT_MAX, 1)` answers. Parity is the second: nine members are exactly odd and two
exactly even on 7 narrow and 6 wide rows, the sign of `-0.0` is read through the reciprocal because
`==` cannot see it, NaN is the one input where no parity can be asked, and `acos`/`acosh` are
excluded as a claim rather than an omission. **`atanh` is the exception and it is now lore** — see
the playbook bullet; it is asserted to a relative rounding step instead. Both cases were run on the
WSL leg as well as the native one before being written down, which is what a float agreement case
owes.

## Next group

Three slices, all reading `crates/mwl-stdlib/src/math.rs` and adding new files under
`tests/conformance/core/`. `docs/spec/01-core-library.md` § 3 owns the rules; each is the *agreement*
shape from conventions.md. The float trap that governs all three is the playbook's `hypot` bullet
plus the new `atanh` one beside it: two spellings may only be asserted equal on rows whose
*intermediate* is exactly representable **and** where both sides are the same computation.

- [ ] **`sqrt`, `cbrt`, `exp` and `log` undo themselves only where the answer is exact** — square a
      `sqrt` and cube a `cbrt` over perfect squares, perfect cubes and dyadic fractions and count the
      round trip; then `log($x, $b)` on exact powers of the base. The rows that are *not* exact are
      the point of the case as much as the ones that are: name a couple and assert the round trip is
      merely near, not equal. `crates/mwl-stdlib/src/math.rs:145` (`sqrt`), `:152` (`cbrt`),
      `:166` (`exp`), `:173` (`log`).
- [ ] **`atan2` is `atan` plus the quadrant, and is the only member that can tell the four apart** —
      `atan2($y, $x) == atan($y / $x)` exactly on the first quadrant (same computation, one division
      that must be exact: use dyadic ratios), then the three rows `atan` cannot express at all — both
      signed zeros of each argument, the two infinities, and `atan2(0.0, 0.0)`, which has an answer
      where a quotient does not. `crates/mwl-stdlib/src/math.rs:221` (`atan2`), `:214` (`atan`).
- [ ] **Every `Core\Math` constant is what its own members answer at a landmark** — `PI` against
      `acos(-1.0)`, `E` against `exp(1.0)`, `TAU` against `PI + PI`, `INT_MAX`/`INT_MIN` against the
      values `intDiv`/`gcd` stop at, and `EPSILON` bounded on both sides as the smallest float with
      `1.0 + EPSILON != 1.0` while half of it is not. Thin neighbour to point at:
      `math-constants-are-class-constants.mwlt`. `crates/mwl-stdlib/src/math.rs:340` (the table),
      `:357` (`EPSILON`), `:361` (`INT_MAX`).

## Backlog

- `Core\Json::decodeAs<T>`'s decoder reads scalar fields only — no enum, `decimal`, `Instant`,
  `array` or nested class, and no optional key from a parameter default (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row, so
  `Core\Str::format` is not yet a sink and `Core\Hash::hmac`'s key is a plain `CoreTy::Bytes`.
- ADR 0086 § 1's substitution table, which the terminal sink and `Cli\Text::plain` need (M8,
  `crates/mwl-stdlib/src/cli.rs` gap 1).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- A closure cannot be called through the variable holding it, so a sweep is always `foreach` over a
  typed array literal (`mwl-ir` gap 1).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
