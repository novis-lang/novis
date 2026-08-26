# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 147 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is **147 passed,
0 failed** and `mwl test tests/` is **633 passed, 0 failed** — run those as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice), and **rebuild `target/release/mwl.exe` first** if
anything under `crates/` is newer than it (playbook, *Running things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**12 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is the
same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either. PHP on
this machine is 8.5.9, on both legs.

**`Core\Str` is down to the code point pair**, and every remaining member is decided by the **unit**
first: `mwl_stdlib::granularity::DEFAULT` is `Unit::Grapheme` where the byte-wise twins count bytes and
the `mb_*` family counts code points — but **`mb_*` is not callable on the Windows leg at all**, so a
member's ASCII rows are one `--ORACLE--` file and its multibyte rows a second `--ORACLE-DIVERGES--` file
with a frozen `--EXPECT--`. One way round that is new this session and is in the playbook: PCRE carries
its own UTF-8, so `preg_split("//u", …)` splits a PHP string into *code points* with no `mbstring`, and
there is no `intl` either, so nothing on either leg counts graphemes. Worked pairs to copy:
`str-slice-and-replace-range-*`, `str-reverse-*`, `str-chunk-*`.

**A fold of several twins into one member is checked as a fold, and the two twins may disagree with each
other rather than with MWL.** `replaceAll` is that shape at its sharpest — `strtr`'s one-pass
longest-match reading against `str_replace`'s cascade, parting on 19 of a 48-cell table — and what
decides between them is a *counted* property, not a row: the same table written back to front answers
the same thing on 48 of 48 cells one pass and on 29 in sequence. `wrap` is the milder one, both of
`wordwrap`'s trailing arguments folded into options, with 352 of 352 lines within their width when long
words may be cut against 122 when they may not.

**No library change landed this session** — both slices are cases only. `Core\Math` is closed but for
the `gmp` pair, which has no callable twin on either leg (playbook, *Writing a test case*) and sits in
the backlog.

**The plan's `Open now` holds every worked `Core\Str` and `Core\Math` instance.** Over `Core\Arr` the key
question still decides the kind of case: PHP renumbers a result's integer keys and keeps its string ones,
ADR 0069 § 3 refuses exactly that, so a member matches its twin over a *list* and diverges over a map;
which way a member points is in its own doc comment in `crates/mwl-stdlib/src/arr.rs`.

**The spellings a case cannot use** — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, a bare `function` at file scope, an `int` literal in a `float` position, `<` over two
strings, and a `?T` returned into a `T` position without `as` — are all in the playbook under *Writing
a test case* and *Writing MWL itself*; do not re-discover them. Four spellings that **do** work and were
re-confirmed this session: a keyed nested literal (`array<array<string>> $t = [["a" => "b"], …]`),
indexing one of those by the outer `foreach`'s own `string $k` into a declared `array<string>`, a
`foreach ($table as string $needle => string $replacement)` over a member's own argument, and the
`"\u{301}"` escape in a `.mwlt` string literal — which is how a decomposed cluster is written in source
without an invisible byte, on the PHP side too.

## Next group

All three are `crates/mwl-stdlib/src/str.rs`, so the file set is that one file plus
`tests/differential/core/`. Take them in this order; the first two are one member pair split by the same
unit rule, and the third finishes a member this session left half-covered.

- [ ] **`Core\Str::codePoints` and `::fromCodePoint` over ASCII** — `crates/mwl-stdlib/src/str.rs:1079`
      and `:2311`, spec § 1. The twins are `mb_str_split`/`mb_ord` and `chr`/`mb_chr`, and inside ASCII
      the callable halves are enough: `str_split` plus `array_map("ord", …)` is `codePoints`, and `chr`
      is `fromCodePoint`. Round-trip both directions as a counted claim over the whole 0–127 table
      rather than row by row, and read the doc comments at both anchors for what each refuses (a
      surrogate, a code point past U+10FFFF).
- [ ] **The same pair past ASCII** — same anchors. `chr(233)` is one *byte* where
      `Core\Str::fromCodePoint(233)` is a character, so that half is a genuine divergence with a
      callable twin; `codePoints` past ASCII has `preg_split("//u", …)` for the pieces but nothing for
      their numbers, so either decode UTF-8 by hand in the `--ORACLE--` block or freeze the rows and
      cite the UCD. Render every cell through `Core\Encoding::toHex($s as bytes)` (playbook).
- [ ] **`Core\Str::wrap` past ASCII** — `crates/mwl-stdlib/src/str.rs:1901`, the algorithm at `:1934`.
      The width counts clusters where `wordwrap` counts bytes, so an accented word wraps one column
      early there and not here; `--ORACLE-DIVERGES--` with a frozen `--EXPECT--`, and the break-resets-
      the-line rule is worth a multibyte cell of its own since a break is only accepted on a unit
      boundary.

## Backlog

- `Core\Arr::flattenDeep` against `iterator_to_array` — `crates/mwl-stdlib/src/arr.rs:2135`.
- `Core\Regex::quote` and `::groups` against `preg_quote`/`preg_match` — `regex.rs:1232`, `:953`.
- `Core\Json::decode` and `::isValid` against `json_decode`/`json_validate` — `json.rs:673`, `:977`.
- `Core\Path::basename`/`dirname`/`normalize` — `path.rs:447`, `:478`, `:664`; a case asserting a built
  path must normalize the separator (loop-goal, *Standing decisions*).
- `Core\Math::gcd`/`::lcm` have no callable twin on either leg — an oracle must write Euclid out in PHP.
- `python tools/gaps.py --errors` is the conformance-side worklist: `Fault::` sites no case asserts.
