# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 133 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is **133 passed,
0 failed** and `mwl test tests/` is **619 passed, 0 failed** — run those as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice), and **rebuild `target/release/mwl.exe` first** if
anything under `crates/` is newer than it (playbook, *Running things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**27 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is the
same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either. PHP on
this machine is 8.5.9, on both legs.

**`Core\Math` is down to 8 of the 27 and `Core\Str` is now the largest cluster at 11.** What a
`Core\Math` member parts from its twin over is one of three things — a **tie**, a **conversion** or a
**guard** — and the plan's `Open now` holds the worked instances; the transcendental pairs are closed
and every one of them agreed with IEEE outright, so the remaining eight are the base conversions, the
two predicates, the two angle conversions and the `gmp` pair.

**The base guard on `Core\Math::log` is new and it is the only behaviour this session changed.** PHP
guards `log`'s *base* and not its argument: a base not greater than zero is a `ValueError` there and is
now a `Fault::thrown` here, and base `1.0` is `NAN` on both sides rather than the infinity
`ln($n) / ln(1.0)` answers. The argument's domain stays at IEEE on both sides. The rule and its reasoning
are the helper's own doc comment at `crates/mwl-stdlib/src/math.rs:974`.

**Neither leg's PHP has `gmp`**, so `Core\Math::gcd` and `::lcm` have no callable twin at all (playbook,
*Writing a test case*); that pair is in the backlog rather than the next group for that reason.
`Core\Math` oracle cases may echo a `float` directly — MWL's rendering is byte-identical to PHP's, at
both extremes — but never a `NAN`, which PHP 8.4+ warns about coercing to a string; `Show::real` in
`math-sqrt-exp-and-log-match-sqrt-exp-and-the-log-family.mwlt` is the shape to copy.

**A `bool`-answering member has no rendering, so the `isNan`/`isFinite` slice needs its own.**
`bool as string` prints `false` as nothing at all and `bool as int` does not lower, so a case pinning a
predicate wants an `if ($b) { return "y"; } return "n";` helper beside `Show::real`, in the same
`final class`.

**Over `Core\Arr` and `Core\Str`, ask the key question before writing a twin**, because it decides which
kind of case it is: PHP renumbers a result's integer keys and keeps its string ones, ADR 0069 § 3
refuses exactly that, so a member matches its twin over a *list* and diverges over a map. Seven pairs
exist to copy (`arr-slice-*`, `arr-pad-*`, `arr-reverse-*`, `arr-replace-range-*`, `arr-unique-*`,
`arr-sort-by-key-*`, `arr-fill-*`), and which way a member points is in its own doc comment in
`crates/mwl-stdlib/src/arr.rs`. The three settled comparison facts — `compare_values` orders one row per
representation, PHP's `<` converts, and ADR 0007 § 5 makes every stored key a bytewise `string` — are in
the plan's `Open now`. **The `php` on the Windows `PATH` has no `mbstring`** (playbook), which is what
the `Core\Str` cluster will meet first.

**The spellings a case cannot use** — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, an array literal where an `array<T>` is expected, a bare `function` at file scope, an
`int` literal in a `float` or `array<float>` position, `<` over two strings — are all in the playbook
under *Writing a test case* and *Writing MWL itself*; do not re-discover them.

## Next group

All three are differential, all three read a doc comment in `crates/mwl-stdlib/src/math.rs` and write
into `tests/differential/core/`, so a session that loads that file once can take two. None has a `gmp`
dependency. The first is the only one of the three with a refusal to assert.

- [ ] **`Core\Math::toBase` and `::fromBase` against `decbin`/`dechex`/`decoct`/`base_convert`** —
      `crates/mwl-stdlib/src/math.rs:1040` and `:1073`, spec § 3. The questions are the base bounds
      (2 and 36, and what each side does outside them), the alphabet's case, a negative `$n`, and
      whether `fromBase` refuses a digit the base cannot hold where `base_convert` skips it.
- [ ] **`Core\Math::isNan` and `::isFinite` against `is_nan`, `is_finite` and `is_infinite`** —
      `math.rs:1017` and `:1026`, spec § 3. One member replaces two PHP functions, so the claim is
      that `isFinite` negated together with `isNan` is exactly `is_infinite`, swept over a table of
      both infinities, both zeros, a `NAN` and the subnormal ends. Needs the `y`/`n` helper above.
- [ ] **`Core\Math::toRadians` and `::toDegrees` against `deg2rad` and `rad2deg`** — `math.rs:738`
      and `:744`, spec § 3. Both are one multiplication, so the whole question is the constant and
      the round trip, asserted by counting over a table rather than read off a line.

## Backlog

- `Core\Math::gcd`/`::lcm` have no `gmp` twin on either leg — the oracle must spell out Euclid in PHP
  or the pair stays out of the count (`docs/agent/handoff.md`, above).
- `Core\Str` × 11 is the largest remaining differential cluster, and `mbstring` is absent on the
  Windows leg (`python tools/gaps.py --differential`).
- `Core\Json::decode`, `::isValid`, `Core\Regex::quote`, `::groups`, `Core\Path::basename` — five
  singles left after the `Math`/`Str` clusters (same worklist).
- Conformance is 486 of 600 and moves only by new case files; `python tools/gaps.py --errors` is its
  worklist (`docs/implementation-plan.md`, `Open now`).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row
  (`crates/mwl-stdlib/src/hash.rs` module doc).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
