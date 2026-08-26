# Handoff

## State

**Conformance is at 522 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*).

**`Core\Math` is 38 members over 15 cases**, and the two added here are the ones the family cases
left. The first sweeps eleven floats — the four constants, two derived infinities, a derived `NaN`,
and `FLOAT_MAX` beside `FLOAT_MAX * 10.0` — through both `isNan` and `isFinite`, labelling each row
and counting that exactly one of nan / finite / infinite holds, so a predicate that both accepted
would leave `partitioned` short while all three tallies still looked plausible. The second names
both ends of the base argument's range for **both** `toBase` and `fromBase` — 2 and 36 accepted, 1
and 37 refused with their messages — then sweeps bases 0 to 40 through both members, counting 35
accepted, 6 refused, and 41 of 41 rows where the two agreed, which is what makes the four named
lines a bound rather than four samples. `math-base-conversion-round-trips.mwlt` keeps the inverse
property and gains a one-line pointer at the new file.

## Next group

Three slices, all reading `crates/mwl-stdlib/src/math.rs` and adding new files under
`tests/conformance/core/`; `math-trigonometry-and-angle-conversion.mwlt` and
`math-roots-exponentials-and-logarithms.mwlt` hold what is already claimed and are worth one read
before writing. `docs/spec/01-core-library.md` § 3 owns the rules, and `mwl_stdlib::math`'s own
module doc owns the domain-error rule slices 1 and 2 both rest on.

- [ ] **`hypot` agrees with `sqrt($x * $x + $y * $y)` everywhere the naive form is representable,
      and answers where it is not** — the *agreement* shape over a table of pairs, counting the
      rows where the two forms are equal, plus the overflow row (`FLOAT_MAX`-scale operands) that
      is the whole reason the member exists: the naive square is an infinity there and `hypot` is
      still finite. `crates/mwl-stdlib/src/math.rs:159` (`hypot`),
      `crates/mwl-stdlib/src/math.rs:145` (`sqrt`).
- [ ] **The inverse members answer IEEE's `NaN` outside their domain rather than throwing** — the
      module doc's second bullet is the rule and no case asserts it: `asin(2.0)`, `acos(2.0)`,
      `acosh(0.5)` and `atanh(1.0)` classified through `Core\Math::isNan`, beside the in-domain row
      that is finite, so the refusal-shaped behaviour is pinned as a *value* and not as a throw.
      `crates/mwl-stdlib/src/math.rs:201` (`asin`), `:208` (`acos`), `:257` (`acosh`), `:264`
      (`atanh`).
- [ ] **`toRadians` and `toDegrees` are inverses, and agree with the constants at the quarter
      turns** — a sweep over 0, 90, 180, 270, 360 counting the round trips that come back equal,
      with `toRadians(180.0)` against `Core\Math::PI` and `toRadians(360.0)` against
      `Core\Math::TAU` as the two rows a member with its own conversion factor fails.
      `crates/mwl-stdlib/src/math.rs:271` (`toRadians`), `:278` (`toDegrees`).

## Backlog

- `Core\Csv` and `Core\Out` are the next-thinnest after `Core\Math` — 5 cases for 2 members and 3
  for 1 — `docs/spec/01-core-library.md` § 12.
- `Core\Json::decodeAs<T>`'s decoder reads a scalar-fielded class only (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is on no `mwl-stdlib` member row (`mwl_stdlib::hash` doc).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
