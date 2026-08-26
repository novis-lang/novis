# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 141 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is **141 passed,
0 failed** and `mwl test tests/` is **627 passed, 0 failed** — run those as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice), and **rebuild `target/release/mwl.exe` first** if
anything under `crates/` is newer than it (playbook, *Running things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**17 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is the
same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either. PHP on
this machine is 8.5.9, on both legs.

**`Core\Str` is 7 of the remaining 17**, and two rules decide what shape each takes. The first is the
**unit**: `mwl_stdlib::granularity::DEFAULT` is `Unit::Grapheme` where `substr`/`strrev`/`str_split`
count bytes and the `mb_*` family counts code points, so a member's ASCII rows are one `--ORACLE--`
file and its multibyte rows a second `--ORACLE-DIVERGES--` file with a frozen `--EXPECT--` — forced,
not stylistic, because the Windows `php` has no `mbstring` and `mb_*` is not callable at all
(playbook). `str-slice-and-replace-range-*` is the worked pair to copy. The second is that a fold of
several twins into one member with options is checked as a **fold**: `compare`'s four corners are one
sweep over every ordered pair of a table, normalized to a sign first, and
`str-compare-matches-strcmp-strcasecmp-strnatcmp-and-strnatcasecmp` is that shape.

**No library change landed this session** — both slices are cases only. `Core\Math` is closed but for
the `gmp` pair, which has no callable twin on either leg (playbook, *Writing a test case*) and sits in
the backlog.

**The plan's `Open now` holds every worked `Core\Str` and `Core\Math` instance**, including what
`strrchr` and `strcasecmp` actually do rather than what the spec's *Replaces* column implies they do.
Over `Core\Arr` the key question still decides the kind of case: PHP renumbers a result's integer keys
and keeps its string ones, ADR 0069 § 3 refuses exactly that, so a member matches its twin over a *list*
and diverges over a map; which way a member points is in its own doc comment in
`crates/mwl-stdlib/src/arr.rs`.

**The spellings a case cannot use** — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, a bare `function` at file scope, an `int` literal in a `float` position, `<` over two
strings, and a `?T` returned into a `T` position without `as` — are all in the playbook under *Writing
a test case* and *Writing MWL itself*; do not re-discover them. An array literal **is** accepted where a
member's parameter is `array<T>` (`Core\Str::join([$a, $b], $glue)` checks); it is `foreach` over a bare
literal and `var` inference that refuse it. `Show::yn` in the `isNan` case is the `bool` rendering to
copy and `Show::bytes` in the grapheme case is the `bytes` one.

## Next group

All three are `crates/mwl-stdlib/src/str.rs`, so the file set is that one file plus
`tests/differential/core/`. Take them in this order; each is the same two-file unit split as `slice`.

- [ ] **`Core\Str::reverse` against `strrev`** — `crates/mwl-stdlib/src/str.rs:1876`, spec § 1. The
      member is grapheme-aware and `strrev` reverses bytes, so a multibyte subject comes back as
      invalid UTF-8 there and as a cluster-wise reversal here. The ASCII rows are an `--ORACLE--`
      file — worth their own, since `reverse` is its own involution and that is a counted claim
      (`reverse(reverse($s)) == $s` over a table, which `strrev` also holds byte-wise) — and the
      cluster rows are a frozen `--ORACLE-DIVERGES--` file asserting through
      `Core\Encoding::toHex($s as bytes)`, because a decomposed cluster is invisible in an editor.
- [ ] **`Core\Str::chunk` against `str_split` and `chunk_split`** — `crates/mwl-stdlib/src/str.rs:951`,
      spec § 1. Both PHP twins are callable on this leg (`mb_str_split` is not), so the ASCII rows are
      an `--ORACLE--`: sweep the size, including a size larger than the subject and the empty subject,
      and note that `chunk_split` appends its separator after the *last* chunk where an `array<string>`
      has no separator at all. `str_split("", 3)` answering `[""]` in PHP 8.2+ is the boundary to ask
      about. The multibyte rows are the same frozen second file.
- [ ] **`Core\Str::replaceAll` against `str_replace` and `strtr`** —
      `crates/mwl-stdlib/src/str.rs:1270`, spec § 1. The question is the order the pairs are applied
      in: `str_replace` with array arguments applies each pair to the *result* of the last, so a
      replacement can be replaced again, where `strtr` with a map takes the longest key at each
      position and never revisits. Which one `replaceAll` is decides whether the twin is one function
      or the other, and the case is the table where the two PHP functions disagree.

## Backlog

- `Core\Str::codePoints` and `::fromCodePoint` — twins are `mb_str_split`/`mb_ord`/`mb_chr`, none
  callable on the Windows leg; check `php -m` inside WSL first (`docs/agent/playbook.md`).
- `Core\Str::wrap` against `wordwrap` — `crates/mwl-stdlib/src/str.rs:1901`; two arguments are refused
  here that PHP accepts, so the case is half agreement and half refusal.
- `Core\Math::gcd`/`::lcm` — no `gmp` on either leg; the expectation has to be a Euclidean loop written
  in the oracle itself (`docs/implementation-plan.md`, `Open now`).
- `Core\Arr::flattenDeep`, `Core\Regex::quote`/`::groups`, `Core\Json::decode`/`::isValid`,
  `Core\Path::basename`/`::dirname`/`::normalize` — the eight non-`Str` members left on
  `python tools/gaps.py --differential`.
- `python tools/gaps.py --errors` — the unasserted `Fault::` sites, which is the conformance half of
  the same worklist (`docs/implementation-plan.md`, `Open now`).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
