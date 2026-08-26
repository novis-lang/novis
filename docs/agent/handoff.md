# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 139 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is **139 passed,
0 failed** and `mwl test tests/` is **625 passed, 0 failed** — run those as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice), and **rebuild `target/release/mwl.exe` first** if
anything under `crates/` is newer than it (playbook, *Running things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**19 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is the
same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either. PHP on
this machine is 8.5.9, on both legs.

**`Core\Math` is closed but for the `gmp` pair**, which has no callable twin on either leg
(playbook, *Writing a test case*) and sits in the backlog. **`Core\Str` is 9 of the remaining 19**, and
the last session's slice says what shape they take: a `Core\Str` member parts from its twin over the
**unit** before anything else, because `mwl_stdlib::granularity::DEFAULT` is `Unit::Grapheme` where
`substr` counts bytes and `mb_substr` counts code points. Inside ASCII with no carriage return the three
units coincide, so the agreeing rows are one `--ORACLE--` file sweeping the whole sign table and the
multibyte rows are a second `--ORACLE-DIVERGES--` file with a frozen `--EXPECT--` — forced, not
stylistic, because the Windows `php` has no `mbstring` and `mb_*` is not callable at all (playbook).
`str-slice-and-replace-range-*` is the worked pair to copy.

**One library change landed with it and it is the only one of its kind in `Core\Math`**:
`toRadians`/`toDegrees` now compute PHP's `($n / 180.0) * PI` and `($n / PI) * 180.0` rather than
`f64::to_radians`/`to_degrees`. The std forms are more accurate by up to an ulp; the ulp is invisible at
precision-14 rendering and visible through `==`, which is what a ported round trip compares. Both doc
comments state the trade in full — do not "fix" them back.

**The plan's `Open now` holds every worked `Core\Math` instance** and now the `Core\Str` unit rule too.
Over `Core\Arr` the key question still decides the kind of case: PHP renumbers a result's integer keys
and keeps its string ones, ADR 0069 § 3 refuses exactly that, so a member matches its twin over a *list*
and diverges over a map; which way a member points is in its own doc comment in
`crates/mwl-stdlib/src/arr.rs`.

**The spellings a case cannot use** — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, an array literal where an `array<T>` is expected, a bare `function` at file scope, an
`int` literal in a `float` position, `<` over two strings — are all in the playbook under *Writing a
test case* and *Writing MWL itself*; do not re-discover them. `Show::yn` in the `isNan` case is the
`bool` rendering to copy and `Show::bytes` in the grapheme case is the `bytes` one.

## Next group

All three are `crates/mwl-stdlib/src/str.rs`, in one 120-line region (`:1787`–`:1901`), so the file set
is that one file plus `tests/differential/core/`. Take them in this order; item 1 is the largest.

- [ ] **`Core\Str::before` and `::after` against `strstr`, `stristr` and `strrchr`** —
      `crates/mwl-stdlib/src/str.rs:1832` and `:1851`, spec § 1. The questions are what each answers
      when the needle is absent (PHP's three all answer `false`, and a `?string` is the MWL shape),
      whether the needle itself is included, which occurrence `before`/`after` take where `strstr` takes
      the first and `strrchr` the last, and whether a case-insensitive option exists or `stristr` has no
      twin at all. Expect one `--ORACLE--` file; a `false`-versus-`null` return is a rendering question,
      not a divergence, so fold it with a `Show::orNone` helper.
- [ ] **`Core\Str::compare` against `strcmp`, `strcasecmp`, `strnatcmp` and `strnatcasecmp`** —
      `crates/mwl-stdlib/src/str.rs:1787`, spec § 1. Four twins into one member, so the case is an
      *agreement* sweep over a table of pairs with the option matrix as the axis. Two traps apply:
      `<` over two strings does not lower (playbook), so render the sign through `Core\Math::sign` or an
      `if`; and PHP's `strcmp` returns the byte difference in some versions and −1/0/1 in others, so
      normalize both sides to a sign before comparing.
- [ ] **`Core\Str::reverse` against `strrev`** — `crates/mwl-stdlib/src/str.rs:1876`, spec § 1. The
      same unit split as `slice`: `strrev` reverses bytes and mangles any multibyte subject outright,
      so the ASCII rows are an `--ORACLE--` file and the cluster rows a frozen one. Check first whether
      an ASCII-only `--ORACLE--` is worth its own file or belongs folded into item 2's.

## Backlog

- `Core\Math::gcd`/`::lcm` — no `gmp` on either leg; an oracle must write a Euclidean loop in PHP
  (`docs/implementation-plan.md`, `Open now`).
- `Core\Str::chunk`, `::codePoints`, `::replaceAll`, `::wrap`, `::fromCodePoint` — the rest of the
  `Core\Str` differential gap (`python tools/gaps.py --differential`).
- `Core\Regex::quote`/`::groups`, `Core\Json::decode`/`::isValid`, `Core\Path::basename`/`::dirname`/
  `::normalize`, `Core\Arr::flattenDeep` — the non-`Str` half of the same list.
- `gaps.py --errors` — the unasserted `Fault::` sites, which is the conformance half of Stage 4's gap.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
