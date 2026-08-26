# Handoff

## State

**Conformance is at 528 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*).

**Another session holds work in this tree uncommitted** — `docs/adr/0101-…`, `docs/adr/README.md`,
`ground-rules.md`, `docs/plan/m4b.md` and two `tainted`/`secret` conformance cases. Two of the 528
are theirs; stage your own paths and leave those alone.

**`Core\Math` is 38 members over 19 cases.** The two added here take the family's two *agreement*
frontiers. `toRadians`/`toDegrees` invert each other on 9 quarter-turn rows and on 12 oblique ones
— the oblique dozen is the set the std spelling would fail, which is what pins PHP's
`($d / 180.0) * PI` as the implemented expression rather than `f64::to_radians` — plus 7 rows
inverted from the radian side, the four quarter turns named against `PI / 2`, `PI`, `TAU * 0.75`
and `TAU` instead of a frozen decimal, 6 mirrored rows, and the pair's one asymmetry: at `1e308`
`toRadians` divides first and stays finite while `toDegrees` multiplies by 180 and reaches `INF`,
neither throwing. `clamp` answers exactly `min(max($x, $lo), $hi)` on 90 `int`, 42 `float` and 42
`string` rows and on the swapped composition order too, agrees with it on NaN (`min` keeps a NaN
and `max` drops it, so the composition eats it and both answer `low`), and parts from it only over
the 6 empty ranges, where the composition is still total and answers `high` while `clamp` refuses —
which is the whole reason `clamp` is its own member. Both existing family cases gained a one-line
pointer at their new neighbour.

## Next group

Three slices, all reading `crates/mwl-stdlib/src/math.rs` and adding new files under
`tests/conformance/core/`. `docs/spec/01-core-library.md` § 3 owns the rules; each is the
*agreement* shape from conventions.md, which is where this family still has the most room.

- [ ] **The four rounding members agree on every integer-valued float and split only where a
      fraction is present** — sweep a table of integer-valued floats (both signs, `±0.0`, `2^52`,
      `2^53`) counting that `ceil`, `floor`, `truncate` and `round` all answer the row itself, then
      a fractional table counting that `truncate` agrees with `floor` on the positive rows and with
      `ceil` on the negative ones, which is the only thing that tells it from either.
      `crates/mwl-stdlib/src/math.rs:89` (`ceil`), `:96` (`floor`), `:103` (`truncate`),
      `:110` (`round`).
- [ ] **`abs` and `sign` agree with the ordering trio on every row** — `abs($x)` is
      `Core\Math::max($x, 0 - $x)` and `sign($x)` is what comparing `$x` against `min`/`max` says,
      swept over both signs, both zeros and both `int` extremes, counted rather than read off.
      `crates/mwl-stdlib/src/math.rs:54` (`abs`), `:61` (`sign`), `:68` (`min`), `:75` (`max`).
- [ ] **`intDiv`/`mod` satisfy the division identity, and `gcd`/`lcm` satisfy theirs** —
      `intDiv($a, $b) * $b + mod($a, $b) == $a` over a swept table of both signs, and
      `gcd($a, $b) * lcm($a, $b) == abs($a * $b)`, counted, with the rows each pair refuses named
      beside them. `crates/mwl-stdlib/src/math.rs:117` (`intDiv`), `:124` (`mod`), `:131` (`gcd`),
      `:138` (`lcm`).

## Backlog

- `Core\Math::toBase`/`fromBase` agree over a sweep, not just at the two refused bases — `math.rs:299`, `:306`.
- `Core\Math::format` against `Core\Str::format`'s numeric holes — `math.rs:313`.
- `Core\Json::decodeAs<T>` reads scalar fields only; no enum, `decimal`, `Instant`, array or nested class (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row (`mwl_stdlib::hash` module doc).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
