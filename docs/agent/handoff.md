# Handoff

## State

**Stage 4's differential gate is met — `mwl test tests/differential` reads 151 against the 150
it requires — so conformance is the only frontier left, at 486 of 600.** Verify is green
(**1596** cargo tests, 74 suites, clippy and fmt clean), `mwl test tests/conformance` is **486
passed, 0 failed** and `mwl test tests/` is **637 passed, 0 failed** — run those as well as
`verify.py`, which executes no `.mwlt` case at all (playbook, twice), and **rebuild
`target/release/mwl.exe` first** if anything under `crates/` is newer than it (playbook,
*Running things*).

**`python tools/gaps.py --errors` is now the worklist, not `--differential`.** It names 109
unasserted `Fault::` sites — 63 `fatal`, 42 `thrown`, 4 `thrown_as` — and a `thrown` is the kind
a case can catch and echo, so those are the *Edges* shape conventions.md names.
`--differential` still lists 8 members with an uncalled PHP twin; they are in the backlog rather
than the next group because the gate they feed is already met.

**The shape a `Core\Regex` divergence takes is settled and worth copying.** Where two twins
answer *differently shaped* things — `quote` against `preg_quote`, whose escape sets differ in
8 of 95 printable-ASCII cells — the case never compares the outputs; it counts the property both
sides exist for, over a grid. Both regex members' doc comments in
`crates/mwl-stdlib/src/regex.rs` are the home of what parts them, and each names the `.mwlt`
that counts it.

**PHP on this machine is 8.5.9 with PCRE 10.44, on both legs, and has neither `mbstring` nor
`intl` nor `gmp`.** `preg_quote($c, "/")` escapes 22 printable-ASCII characters where
`regex::escape` escapes 18, and `PREG_UNMATCHED_AS_NULL` is what makes `$matches` agree with
`Core\Regex\Match::groups`.

**The spellings a case cannot use** — a closure through the variable holding it,
`Class::method(...)`, `bool as int`, a bare `function` at file scope, an `int` literal in a
`float` position, `<` over two strings, and a `?T` returned into a `T` position without `as` —
are all in the playbook under *Writing a test case* and *Writing MWL itself*; do not
re-discover them. One added this session: a `?Instance` narrows fine at file scope, so a `?Match`
needs no helper class.

**`orient.py`'s `[context]` manifest was short in two fields.** `modules` printed only
`src/arr.rs`, `src/registry.rs` and `src/str.rs`, so `src/regex.rs` — the file both slices
edited — was absent, and the next group needs `src/bytes.rs`. `adrs` printed 0009 §§ 1 and 3,
0051 § 4 and 0007 § 3; the item itself named ADR 0056 §§ 4-5 and the work needed ADR 0063 R3, R5
and R11, none of which were printed.

## Next group

All three slices are `crates/mwl-stdlib/src/bytes.rs` plus `tests/conformance/core/`, and each
is the *Edges* shape: `gaps.py --errors` already holds the anchors, so do not re-derive them.
Spec § 7 owns the rows. Take them in this order, and stop after the first if the second would
push past the ceiling.

- [ ] **`Core\Bytes::pack`'s argument refusals** — `bytes.rs:818` (a code wants an argument the
      call did not write), `:945` (`x*` has nothing to repeat), `:963`, `:990`, `:1003` (a
      buffer, integer or float field given the wrong kind of argument) and `:1015` (the format
      writes fewer fields than the call passed). Spec § 7, and ADR 0009 § 3's
      checked-never-repaired rule is the reason each is a throw rather than a truncation.
- [ ] **`Core\Bytes::pack`'s range refusals** — `bytes.rs:855` (a value past a code's bit
      width), `:891` (outside a 32-bit float's range, where rounding is not silent) and `:918`
      (a field width the argument does not fill). The *bound asserted on both sides* shape: name
      the last accepted value beside the first refused one.
- [ ] **`Core\Bytes::unpack`'s buffer bounds and the two single-value ones** —
      `bytes.rs:1136` (a code reading past the buffer), `:1244` (a format whose offset is past
      the end), `:459` (`Core\Bytes::at` outside the buffer) and `:618` (`Core\Bytes::fill`
      given something that is not one octet).

## Backlog

- `Core\Path::basename`/`dirname`/`normalize` against `basename`/`dirname`/`realpath` —
  `path.rs:447`, `:478`, `:664`; watch the separator rule in `loop-goal.md` § *Standing
  decisions*.
- `Core\Json::decode` and `::isValid` against `json_decode`/`json_validate` — `json.rs:673`,
  `:977`.
- `Core\Arr::flattenDeep` against `iterator_to_array` — `arr.rs:2135`.
- `Core\Math::gcd`/`lcm` have no callable twin (`no gmp`); an oracle must compute a Euclidean
  loop in PHP or the pair stays out of the count — `math.rs:945`, `:960`.
- `Core\Str::wrap` past ASCII, the last `Core\Str` divergence file — an `--ORACLE-DIVERGES--`
  with a frozen `--EXPECT--` rendered through `Core\Encoding::toHex`.
- `Core\Encoding::fromBase64`/`fromBase64Url`/`fromBase32`/`fromHex` refusals —
  `encoding.rs:840`, `:872`, `:915`, `:960`.
