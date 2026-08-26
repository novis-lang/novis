# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 136 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is **136 passed,
0 failed** and `mwl test tests/` is **622 passed, 0 failed** — run those as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice), and **rebuild `target/release/mwl.exe` first** if
anything under `crates/` is newer than it (playbook, *Running things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**25 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is the
same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either. PHP on
this machine is 8.5.9, on both legs.

**`Core\Math` is down to 4 of the 25 and `Core\Str` is the largest cluster at 11.** What a `Core\Math`
member parts from its twin over is a **tie**, a **conversion**, a **guard** or a **repair**, and the
plan's `Open now` holds every worked instance; the transcendentals, the base pair and the two
predicates are all closed, leaving the two angle conversions and the `gmp` pair. **Neither leg's PHP
has `gmp`**, so `::gcd`/`::lcm` have no callable twin at all (playbook, *Writing a test case*) and sit
in the backlog rather than the group.

**The split this session used is the shape to copy for any member with a repairing twin.** PHP's base
conversions have no failure mode — an invalid digit, an empty string and an overflow are each turned
into some number — so the agreeing rows are one `--ORACLE--` file and the repairs are one
`--ORACLE-DIVERGES--` file with a frozen `--EXPECT--`. The new playbook bullet says why that is forced
rather than stylistic: PHP's `Deprecated:` notice goes to stdout and would break the diff.

**A `bool`-answering member now has a worked rendering.** `bool as string` prints `false` as nothing and
`bool as int` does not lower, so `Show::yn` in
`math-is-nan-and-is-finite-match-is_nan-is_finite-and-is_infinite.mwlt` is the two-character helper to
copy, beside `Show::real` in the `sqrt`/`exp`/`log` case for floats. `Core\Math` oracle cases may echo a
`float` directly — MWL's rendering is byte-identical to PHP's — but never a `NAN`.

**Over `Core\Arr` and `Core\Str`, ask the key question before writing a twin**, because it decides which
kind of case it is: PHP renumbers a result's integer keys and keeps its string ones, ADR 0069 § 3
refuses exactly that, so a member matches its twin over a *list* and diverges over a map. Seven `arr-*`
pairs exist to copy, and which way a member points is in its own doc comment in
`crates/mwl-stdlib/src/arr.rs`. **The `php` on the Windows `PATH` has no `mbstring`** (playbook), which
is what the `Core\Str` cluster meets first: `Core\Str::slice`'s twin is `substr` *and* `mb_substr`, so
only the byte-wise half is callable here and the code-point rows have to cite the member's own doc.

**The spellings a case cannot use** — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, an array literal where an `array<T>` is expected, a bare `function` at file scope, an
`int` literal in a `float` position, `<` over two strings — are all in the playbook under *Writing a
test case* and *Writing MWL itself*; do not re-discover them. The case-changing members are
`Core\Str::lower`/`::upper`, not `toLower`.

## Next group

Item 1 is `crates/mwl-stdlib/src/math.rs` and items 2–3 are `crates/mwl-stdlib/src/str.rs`, so the
usual "second slice only if it shares the files" rule points at 2+3. Item 1 is first anyway because it
is two doc comments and one echo row wide — closing `Core\Math` bar the `gmp` pair — and leaves a
session room for item 2 despite the file change.

- [ ] **`Core\Math::toRadians` and `::toDegrees` against `deg2rad` and `rad2deg`** —
      `crates/mwl-stdlib/src/math.rs:270` and `:277`, spec § 3. The questions are whether either side
      uses a stored constant or `$n * PI / 180.0`, what the round trip loses over a table of angles,
      and what the infinite and `NaN` arguments do. No guard is expected on either side, so this is
      most likely one `--ORACLE--` file with a counted round trip.
- [ ] **`Core\Str::slice` and `::replaceRange` against `substr` and `substr_replace`** —
      `crates/mwl-stdlib/src/str.rs:1412` and `:1439`, spec § 1. Ask the unit question first: PHP's
      `substr` is bytes and `mb_substr` code points, and the Windows `php` has no `mbstring`, so the
      byte-wise rows are the callable ones. Then the negative offset, the offset past the end and the
      length past the end — three places `substr` repairs and MWL may refuse.
- [ ] **`Core\Str::before` and `::after` against `strstr`, `stristr` and `strrchr`** —
      `crates/mwl-stdlib/src/str.rs:1830` and `:1849`, spec § 1. The questions are what each answers
      when the needle is absent (`strstr` is `false`, which has no MWL spelling), whether the needle
      itself is included, and which end `strrchr` searches from.

## Backlog

- `Core\Math::gcd`/`::lcm` — no `gmp` on either leg, so an oracle has to compute its own Euclidean
  loop in PHP or the pair stays out of the count (`crates/mwl-stdlib/src/math.rs:925`, `:940`).
- The remaining eight `Core\Str` differential members after the group above —
  `python tools/gaps.py --differential`.
- `Core\Regex::quote`/`::groups`, `Core\Json::decode`/`::isValid`, `Core\Path::basename`/`::dirname`/
  `::normalize`, `Core\Arr::flattenDeep` — same worklist.
- `gaps.py --errors`, the conformance half: `Fault::` sites in `mwl-stdlib` no case asserts.
- `docs/plan/` M4B's index row and the file's own H1 have drifted apart (`python tools/plan.py
  --check` reports it); a one-line fix nobody owns.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
