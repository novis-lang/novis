# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 145 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is **145 passed,
0 failed** and `mwl test tests/` is **631 passed, 0 failed** — run those as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice), and **rebuild `target/release/mwl.exe` first** if
anything under `crates/` is newer than it (playbook, *Running things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**14 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is the
same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either. PHP on
this machine is 8.5.9, on both legs.

**`Core\Str` is 4 of the remaining 14**, and every one of them is decided by the **unit** first:
`mwl_stdlib::granularity::DEFAULT` is `Unit::Grapheme` where the byte-wise twins count bytes and the
`mb_*` family counts code points, so a member's ASCII rows are one `--ORACLE--` file and its multibyte
rows a second `--ORACLE-DIVERGES--` file with a frozen `--EXPECT--` — forced, not stylistic, because the
Windows `php` has no `mbstring` and `mb_*` is not callable at all (playbook). Four worked pairs to copy
now: `str-slice-and-replace-range-*`, `str-reverse-*` and `str-chunk-*`. The second rule is that a fold
of several twins into one member with options is checked as a **fold** —
`str-compare-matches-strcmp-strcasecmp-strnatcmp-and-strnatcasecmp` is that shape, and
`str-chunk-matches-str_split-and-chunk_split` is the other: PHP's third function is not a fourth member
but `Core\Str::join(Core\Str::chunk($s, $n), $end) . $end`, pinned on all 30 cells.

**An agreeing file pins properties, not rows.** What `reverse` and `chunk` gained is counted claims a
plausible-looking member still fails — the involution, the palindromes, the width, the 100 ordered pairs
of `reverse($a . $b) == reverse($b) . reverse($a)`; the rejoin and the piece count. **The claim that
survives a unit change is the one that cannot discriminate**: the rejoin holds under bytes, code points
and clusters alike, so what parts `chunk` from `str_split` is the piece *count* at size 1.

**No library change landed this session** — all four slices are cases only. `Core\Math` is closed but for
the `gmp` pair, which has no callable twin on either leg (playbook, *Writing a test case*) and sits in
the backlog.

**The plan's `Open now` holds every worked `Core\Str` and `Core\Math` instance.** Over `Core\Arr` the key
question still decides the kind of case: PHP renumbers a result's integer keys and keeps its string ones,
ADR 0069 § 3 refuses exactly that, so a member matches its twin over a *list* and diverges over a map;
which way a member points is in its own doc comment in `crates/mwl-stdlib/src/arr.rs`.

**The spellings a case cannot use** — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, a bare `function` at file scope, an `int` literal in a `float` position, `<` over two
strings, and a `?T` returned into a `T` position without `as` — are all in the playbook under *Writing
a test case* and *Writing MWL itself*; do not re-discover them. Three spellings that **do** work and were
re-confirmed this session: an `array<T>` literal in a member's `array<T>` parameter, a typed declaration
inside a loop body (it is only a *sibling* block that gets `E0406`), and `$size as uint` at a `uint`
parameter fed from an `array<int>` sweep. `Show::yn` in the `isNan` case is the `bool` rendering to copy
and `Show::bytes` in the grapheme cases is the `bytes` one.

## Next group

All three are `crates/mwl-stdlib/src/str.rs`, so the file set is that one file plus
`tests/differential/core/`. Take them in this order; the first two are the same two-file unit split as
`reverse` and `chunk`, and the third is one pair of members in one region.

- [ ] **`Core\Str::replaceAll` against `str_replace` and `strtr`** — `crates/mwl-stdlib/src/str.rs:1270`,
      spec § 1. Two twins folded into one member, so it is checked as a *fold*: `str_replace` applies its
      pairs in sequence and can therefore replace what an earlier pair produced, while `strtr` with an
      array applies the longest matching key at each position and never re-scans its own output. Which of
      those `replaceAll` is, is what the ASCII `--ORACLE--` file has to pin — over a table where the two
      twins *disagree* (`["a" => "b", "b" => "c"]` over `"ab"`), not only where they agree. The
      multibyte half is a `--ORACLE-DIVERGES--` file only if the member's matching is unit-sensitive;
      read the doc comment at the anchor before assuming it is.
- [ ] **`Core\Str::wrap` against `wordwrap`** — `crates/mwl-stdlib/src/str.rs:1901`, spec § 1. The
      algorithm is PHP's own (`wrapped`, `str.rs:1934`) so the ASCII table should agree outright across
      the width, `breakWith` and `cutLongWords` grid; the two arguments this member refuses and
      `wordwrap` does not — an empty `breakWith`, and a zero width with cutting — are the boundary
      asserted on both sides, PHP raising `ValueError` for the same pair. The width counts in clusters
      (`str.rs:1924`), which is the frozen half.
- [ ] **`Core\Str::codePoints` and `::fromCodePoint`** — `crates/mwl-stdlib/src/str.rs:1079` and
      `str.rs:2311`, spec § 1. `ord`/`chr` are callable on the Windows leg and `mb_ord`/`mb_chr` are not,
      so the ASCII `--ORACLE--` half runs against `ord`/`chr` and the whole non-ASCII half is frozen. The
      round trip is the counted claim: `fromCodePoint` over `codePoints` reproduces the subject.

## Backlog

- `Core\Math::gcd`/`::lcm` — no `gmp` on either leg, so the oracle must be an explicit Euclidean loop in
  PHP or the pair stays uncounted (`docs/implementation-plan.md`, `Open now`).
- `Core\Arr::flattenDeep` against `iterator_to_array` — `crates/mwl-stdlib/src/arr.rs:2135`.
- `Core\Regex::quote` and `::groups` — `regex.rs:1232`, `regex.rs:953`; twins `preg_quote`, `preg_match`.
- `Core\Json::decode` and `::isValid` — `json.rs:673`, `json.rs:977`; `json_decode`, `json_validate`.
- `Core\Path::basename`/`::dirname`/`::normalize` — `path.rs:447`, `:478`, `:664`; mind the two legs'
  separators (loop-goal, *Standing decisions*).
- Conformance depth, 486 of 600: `python tools/gaps.py --errors` is the worklist for the unasserted
  `Fault::` sites (`docs/implementation-plan.md`, `Open now`).
