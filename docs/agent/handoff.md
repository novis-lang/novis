# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 131 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is **131 passed,
0 failed** and `mwl test tests/` is **617 passed, 0 failed** — run those as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice), and **rebuild `target/release/mwl.exe` first** if
anything under `crates/` is newer than it (playbook, *Running things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**32 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is the
same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either. PHP on
this machine is 8.5.9, on both legs.

**`Core\Math` is now the largest cluster — 11 of the 32 — and nothing in it has a key rule to diverge
over.** A `Core\Math` member either agrees with its twin outright or parts over one of three things, and
the two pairs already written say which: a **tie** (PHP's two-argument `min` answers its *second*
argument and its `max` its first, where `pick` answers the first to both; `f64::total_cmp` also separates
`-0.0` from `0.0` where PHP's `<` does not), a **conversion** (`fmod` widens two `int`s where
`Core\Math::mod`'s two `float` parameters do not — the integer remainder is `%`), or a **guard** (a zero
divisor is a throw per spec § 3, where `fmod` answers `NAN`; the IEEE *domain* rows — infinite dividend,
`NAN` operand — stay at IEEE's answer and agree). `intDiv` agrees with `intdiv` on every row, both
refusals included.

**Neither leg's PHP has `gmp`**, so `Core\Math::gcd` and `::lcm` have no callable twin at all (playbook,
*Writing a test case*); that pair is in the backlog rather than the next group for that reason.
`Core\Math` oracle cases may echo a `float` directly — MWL's rendering is byte-identical to PHP's — but
never a `NAN`, which PHP 8.4+ warns about coercing to a string.

**Over `Core\Arr`, ask the key question before writing a twin, because it decides which kind of case you
are writing.** Almost every PHP array function renumbers the integer keys of its result and keeps the
string ones; ADR 0069 § 3 refuses exactly that, so an MWL member either renumbers all of them or none.
Over a list the two rules coincide and the case is an `--ORACLE--` one; over a map or a mixed-key
subject they part and it is an `--ORACLE-DIVERGES--` one with MWL's own output frozen in `--EXPECT--`.
Seven pairs exist to copy (`arr-slice-*`, `arr-pad-*`, `arr-reverse-*`, `arr-replace-range-*`,
`arr-unique-*`, `arr-sort-by-key-*`, `arr-fill-*`). Which way a member points is in its own doc comment
in `crates/mwl-stdlib/src/arr.rs`; read the comment rather than assuming.

**Three comparison facts are settled and pinned.** MWL orders by `mwl_stdlib::ordering::compare_values`,
one row per representation with nothing crossing except the two numeric ones, where PHP's `<` converts:
`min([0, "a"])` is `0` there and a throw here, and two numeral strings compare *numerically* there and
bytewise here. The third is the same fact one level up: ADR 0007 § 5 makes every stored *key* a
`string`, so `sortByKey` compares keys bytewise where `ksort` reads a numeral key as a number. That
refusal is a `Fault::thrown` a `catch (Throwable $e)` does reach, unlike the `Fault::fatal` an
argument-shape guard raises (playbook, *Writing a test case*, two adjacent bullets).

**Rendering is what makes a `?T` comparable**: absence is `none` on both sides and the guarded branch
returns `$found as string`; a `?bool` has no such shape, so keep a `bool`-valued subject out of such a
case. The spellings a case cannot use — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, an array literal where an `array<T>` is expected, a bare `function` at file scope, an
`int` literal in a `float` or `array<float>` position, `<` over two strings — are all in the playbook
under *Writing a test case* and *Writing MWL itself*; do not re-discover them.

## Next group

All three are differential, all three read a doc comment in `crates/mwl-stdlib/src/math.rs` and write
into `tests/differential/core/`, so a session that loads that file once can take two. None of these has
a `gmp` dependency; all of them echo a `float`, so `Show::real`'s `NAN` guard from
`math-int-div-and-mod-match-intdiv-and-fmod` is the shape to copy.

- [ ] **`Core\Math::sqrt`, `::exp` and `::log` against `sqrt`, `exp`, `log`/`log10`/`log2`** —
      `crates/mwl-stdlib/src/math.rs:144`, `:165` and `:974`, spec § 3. `log` covers three PHP functions
      through a `base` option, so the questions are whether each default matches and what the members do
      at the domain edges PHP answers with `NAN`/`-INF` — a `sqrt` of a negative, a `log` of zero and of
      a negative — which is where a guard would part them from IEEE.
- [ ] **`Core\Math::hypot` and `::atan2` against `hypot` and `atan2`** — `math.rs:948` and `:959`,
      spec § 3. `hypot`'s whole reason is the intermediate overflow `sqrt($a ** 2 + $b ** 2)` has, so the
      case wants a pair whose squares overflow `float` and one that is exactly representable; `atan2`'s
      is the quadrant, including both signed zeros.
- [ ] **`Core\Math::toBase` and `::fromBase` against `decbin`/`dechex`/`decoct` and
      `bindec`/`hexdec`/`octdec`** — `math.rs:1016` and `:1049`, spec § 3. Two members replacing seven
      PHP functions plus `base_convert`, so the shape is agreement across a swept table of bases; PHP's
      `bindec` silently ignores a digit outside the base and returns a `float` past `PHP_INT_MAX`, which
      is the divergence to look for.

## Backlog

- `Core\Math::gcd`/`::lcm` — no `gmp` on either leg; decide between an explicit Euclidean `--ORACLE--` and
  a conformance case (`docs/agent/handoff.md`, above).
- `Core\Math::isNan`, `::isFinite`, `::toRadians`, `::toDegrees` — the fourth `Core\Math` cluster
  (`math.rs:993`, `:1002`, `:270`, `:277`).
- `Core\Str`'s remaining twins — `compare`, `slice`, `before`, `replaceRange`, `reverse`, `wrap`,
  `fromCodePoint` (`python tools/gaps.py --differential`).
- `Core\Arr::flattenDeep` against `iterator_to_array` (`arr.rs:2135`).
- `--errors`: the unasserted `Fault::` sites, which is the conformance half of the same worklist.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
