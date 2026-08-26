# Handoff

## State

**Stage 4's differential count is one case from its gate — 149 of 150** — while conformance stands
at 486 of 600, and the gap on both is behavioural depth per member, not coverage: every registered
member already has a case and both of Stage 4's named guards pass. Verify is green (**1596** cargo
tests, 74 suites, clippy and fmt clean), `mwl test tests/conformance` is **486 passed, 0 failed**,
`mwl test tests/differential` is **149 passed, 0 failed** and `mwl test tests/` is **635 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than it
(playbook, *Running things*).

**`python tools/gaps.py --differential` is the worklist and names 10 members** — `Core\Str` is now
closed, so what is left is `Core\Path`'s three, `Core\Regex`'s two, `Core\Json`'s two,
`Core\Math`'s `gmp` pair (no callable twin on either leg) and `Core\Arr::flattenDeep`. Do not
re-derive it; `--errors` is the same list for the unasserted `Fault::` sites on the conformance
side. PHP on this machine is 8.5.9, on both legs, and has neither `mbstring` nor `intl` nor `gmp`.

**The shape a `Core\Str` divergence file takes is settled and worth copying** — a member's ASCII
rows are one `--ORACLE--` file and its multibyte rows a second `--ORACLE-DIVERGES--` file with a
frozen `--EXPECT--` rendered through `Core\Encoding::toHex($s as bytes)`, never as text. Worked
pairs: `str-code-points-and-from-code-point-*`, `str-slice-and-replace-range-*`, `str-reverse-*`,
`str-chunk-*`. A frozen expectation is derived *independently* — PCRE carries its own UTF-8, so
`preg_split("//u", …)` splits into code points with no `mbstring`, and a four-line PHP decoder over
`unpack("C*", …)` turns each piece into its scalar value.

**No library change landed this session** — both slices are cases only, and the whole group's third
item (`Core\Str::wrap` past ASCII) was not reached.

**The plan's `Open now` holds every worked `Core\Str` and `Core\Math` instance.** Over `Core\Arr`
the key question still decides the kind of case: PHP renumbers a result's integer keys and keeps
its string ones, ADR 0069 § 3 refuses exactly that, so a member matches its twin over a *list* and
diverges over a map; which way a member points is in its own doc comment in
`crates/mwl-stdlib/src/arr.rs`.

**The spellings a case cannot use** — a closure through the variable holding it,
`Class::method(...)`, `bool as int`, a bare `function` at file scope, an `int` literal in a `float`
position, `<` over two strings, and a `?T` returned into a `T` position without `as` — are all in
the playbook under *Writing a test case* and *Writing MWL itself*; do not re-discover them. Two
spellings this session confirmed and added there: a typed declaration inside a loop body does
*not* collide with itself, and `foreach` over a member's `array<uint>` result binds `uint`.

**`orient.py`'s `[context] modules` was one pattern short**: every `Core\Str` case renders through
`Core\Encoding::toHex` and `Core\Json::encode`, so `src/encoding.rs` and `src/json.rs` belong
beside `src/str.rs` in the manifest — and the next group needs `src/regex.rs`.

## Next group

Both slices are `crates/mwl-stdlib/src/regex.rs`, so the file set is that one file plus
`tests/differential/core/`. Either one on its own closes the differential gate at 150; take them in
this order, and stop after the first if the second would push past the ceiling.

- [ ] **`Core\Regex::quote` against `preg_quote`** — `crates/mwl-stdlib/src/regex.rs:1232`, spec
      § 5 and ADR 0056 §§ 4-5. The member is `regex::escape` and takes no `$delimiter` argument,
      because ADR 0056 § 5 removed the `/…/` syntax that one existed for. The two escape
      *different sets*, so a row-by-row comparison of the outputs is a divergence and the counted
      claim is the one that agrees: for every printable ASCII character, and for a table of
      multi-character literals, the quoted form matches its own literal and nothing else — MWL
      through `Core\Regex::matches`, PHP through `preg_match("/" . preg_quote($c, "/") . "/")`.
      Read the doc comment at the anchor before writing the rows.

- [ ] **`Core\Regex\Match::groups` against `preg_match`'s `$matches`** —
      `crates/mwl-stdlib/src/regex.rs:953`, and the sibling `::group` at `:929` for the rule this
      one has to agree with. It answers `array<?string>` in `preg_match`'s own order and shape
      (`built_match`), so an unmatched optional group is a `null` element where PHP's array has an
      empty string or no entry at all — `Core\Json::encode` renders the whole array in one piece
      and `json_encode($m)` renders PHP's, which is the only spelling that compares them. The
      neighbouring divergence is already stated at `:925`: a *named* group that does not exist
      throws where `preg_match` answers an absent entry.

## Backlog

- `Core\Str::wrap` past ASCII — `crates/mwl-stdlib/src/str.rs:1901`, algorithm at `:1934`; the last
  `Core\Str` `--ORACLE-DIVERGES--` file (plan, `Open now`).
- `Core\Path::basename`/`::dirname`/`::normalize` — `path.rs:447`/`:478`/`:664`; mind
  `Path::SEPARATOR` differing between the two legs (loop-goal, *Standing decisions*).
- `Core\Json::decode` and `::isValid` — `json.rs:673`/`:977`, twins `json_decode`/`json_validate`.
- `Core\Arr::flattenDeep` against `iterator_to_array` — `arr.rs:2135`.
- `Core\Math::gcd`/`::lcm` — no callable twin on either leg; the expectation needs an explicit
  Euclidean loop in the PHP body (playbook, *Writing a test case*).
- Conformance 486 → 600 is the larger gap; a new case picks one of conventions.md's four shapes.
