# Handoff

## State

**Conformance is at 536 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*). The tree is clean.

**`Core\Math` is 38 members and 11 constants over 27 cases**, and the two added here take the
family's *constant* and *identity* frontiers. Every constant is now pinned to the member that
produces it rather than only to its own literal: `PI` is what four inverse and circular members
answer at their landmarks, `TAU` is `PI` doubled in all three directions, `E` is `exp` at one and
`log`'s own default base, `EPSILON` and `FLOAT_MIN` are read from both sides of the same relative
step (the step survives at `FLOAT_MIN` and is lost one halving below it, which is what makes
`FLOAT_MIN` the smallest *normal*), `FLOAT_MAX`/`INFINITY` are the two sides of `isFinite`, `NAN` is
the one constant `==` cannot confirm, `INT_MAX`/`INT_MIN` are exactly where `intDiv`, `gcd` and
`abs` stop — the last argument each answers and the first each refuses — and `UINT_MAX` is the 64th
bit `fromBase` has no `int` for. Eleven of eleven, counted rather than read off the lines.
`sin`/`cos`/`tan` are then one circle over a 20-row table: the Pythagorean identity, the quotient
and the `[-1, 1]` bound each hold on all 20, held to a rounding step because none of the three is
correctly rounded, and the quarter turns are asserted as *properties* — `tan(PI / 2.0)` is enormous
and finite rather than an infinity, `cos(PI / 2.0)` and `sin(PI)` are tiny and positive rather than
zero — so no platform's last digits are frozen. Both cases were measured on the WSL leg as well as
the native one, through `php` rather than a cross-build; the new playbook bullet names the rows that
came back exact so the next float session does not re-measure them.

## Next group

Three slices, all reading `crates/mwl-stdlib/src/math.rs` and adding new files under
`tests/conformance/core/`. `docs/spec/01-core-library.md` § 3 owns the rules; each is the
*agreement* shape from conventions.md. The tolerance spelling and the exact-on-both-legs rows are
both playbook bullets under *Writing a test case* — do not re-derive either.

- [ ] **The six hyperbolic members are their exponential definitions** — `sinh` against
      `(exp($x) - exp(0.0 - $x)) / 2.0` and `cosh` against the sum, `cosh * cosh - sinh * sinh`
      is 1, `tanh` is the ratio, and `asinh`/`acosh`/`atanh` undo the three over a table — all to a
      relative step, never to equality, since none of the six is correctly rounded.
      `crates/mwl-stdlib/src/math.rs:229` (`sinh`), `:236` (`cosh`), `:243` (`tanh`),
      `:250` (`asinh`), `:257` (`acosh`), `:264` (`atanh`).
- [ ] **The three inverse circular members undo the three circular ones only inside their own
      principal branch** — `asin(sin($x))` is `$x` on a swept table inside `[-PI/2, PI/2]` and is
      the *reflection* of it outside, `acos(cos($x))` folds the negative half onto the positive,
      and `atan(tan($x))` is the one that is periodic rather than reflected. Counted, to a relative
      step. `crates/mwl-stdlib/src/math.rs:180` (`sin`), `:187` (`cos`), `:194` (`tan`),
      `:201` (`asin`), `:208` (`acos`), `:215` (`atan`).
- [ ] **`format` and `round` agree wherever both name the same precision** — `Core\Math::format($n,
      {decimals: $d})` renders what `Core\Math::round($n, {precision: $d})` answers, on every row of
      a table, and parts from it only in the options `round` has no opinion about (the separators,
      whose defaults are MWL's and not `number_format`'s). `crates/mwl-stdlib/src/math.rs:110`
      (`round`), `:313` (`format`), `:449` (`FORMAT_OPTIONS`, where the empty group separator is
      decided).

## Backlog

- `Core\Json::decodeAs<T>`'s decoder reads scalar fields only — ADR 0071, `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row — `mwl_stdlib::hash`'s module doc.
- ADR 0086 § 1's substitution table, which `Cli\Text::plain` needs — `crates/mwl-stdlib/src/cli.rs` gap 1.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1.
- A closure cannot be called through the variable holding it — `mwl-ir` gap 1, playbook.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
