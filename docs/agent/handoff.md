# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 106 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and
both of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt
clean), `mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is
**106 passed, 0 failed** and `mwl test tests/` is **592 passed, 0 failed** — run those as well as
`verify.py`, which executes no `.mwlt` case at all (playbook, twice), and **rebuild
`target/release/mwl.exe` first** if anything under `crates/` is newer than it (playbook, *Running
things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**50 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is
the same list for the 109 unasserted `Fault::` sites on the conformance side. Do not re-derive
either.

**Over `Core\Arr`, ask the key question before writing a twin, because it decides which kind of case
you are writing.** Almost every PHP array function renumbers the integer keys of its result and keeps
the string ones; ADR 0069 § 3 refuses exactly that, so an MWL member either renumbers all of them or
none. Whether the twin *matches* therefore depends on the subject: over a list the two rules coincide
and the case is an `--ORACLE--` one; over a map or a mixed-key subject they part and the case is an
`--ORACLE-DIVERGES--` one with MWL's own output frozen in `--EXPECT--`. Both shapes now exist for the
window and the ends — `arr-slice-matches-array_slice.mwlt` and
`arr-slice-on-string-keys-diverges-from-array_slice.mwlt` are the pair to copy — and which way a
member points is in its own doc comment in `crates/mwl-stdlib/src/arr.rs`, not something to derive
from PHP.

**A second, unrelated divergence is settled and pinned**: PHP's next free integer key is a property
of an array's *history* and MWL's is a property of its *entries*, because a pure member returns a
fresh array with no counter to inherit. `mwl_core_arr_append`'s doc comment owns that fact and
`arr-append-derives-the-next-free-key-where-php-remembers-it.mwlt` pins it, including `array_pop`
lowering PHP's counter to the key it removed. MWL's own `$a[] = $v` does match PHP.

**The shape a `?T` twin takes is settled** — both sides render absence as `none` and the case
compares the answer, never the representation; `str-index-of-matches-strpos.mwlt` is the worked one,
and the `return $found as string;` ending it needs is in the playbook under *Writing MWL itself*.

The spellings a case cannot use — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, an array literal wherever an `array<T>` is expected (**including an argument
position**, not only a `foreach` or a `var`), a bare `function` at file scope — are all in the
playbook under *Writing a test case* and *Writing MWL itself*; do not re-discover them.

## Next group

All three are differential and share `tests/differential/core/` with one region of
`crates/mwl-stdlib/src/arr.rs`, so a session that loads it once can take two. Each is a family off
`python tools/gaps.py --differential`, so no session re-derives the twin. Read the members' doc
comments for the key rule first, per *State* above.

- [ ] **`Core\Arr`'s pad and reverse against `array_pad`/`array_reverse`** — `padStart`
      (`crates/mwl-stdlib/src/arr.rs:1948`), `padEnd` (`arr.rs:1965`), `reverse` (`arr.rs:1987`),
      spec § 2. `array_pad`'s one function takes a signed width for both directions where MWL has two
      members, and `array_reverse`'s `$preserve_keys` is the same option-vs-uniform question the
      window family just answered — so expect one matching case per member over a list and one
      divergence case for the string-keyed subject.
- [ ] **`Core\Arr`'s membership pair against `in_array`/`array_search`** — `contains`
      (`arr.rs:3415`), `keyOf` (`arr.rs:3434`), spec § 2. Both rest on the one strict-identity
      question `mwl_runtime::identity` answers, and `in_array`'s `$strict` flag is the whole
      divergence: PHP's default compares loosely, which ADR 0090 forbids outright. `keyOf` answers
      `?string` where `array_search` answers `false`, so it takes the `?T` twin shape above.
- [ ] **`Core\Arr`'s counting pair against `array_unique`/`array_count_values`** — `unique`
      (`arr.rs:3472`), `countBy` (`arr.rs:2525`), spec § 2. `array_unique`'s default compares by
      string cast and it keeps the *first* key of each run, so both the identity rule and the key
      rule are in play at once; `array_count_values` refuses a non-scalar entry with a warning where
      MWL's member has a declared element type.

## Backlog

- `Core\Arr::fill`/`flattenDeep`/`reduce` and the four predicate members (`find`, `findKey`, `any`,
  `all`) still have no oracle case — `python tools/gaps.py --differential`.
- The 109 unasserted `Fault::` sites are the conformance half of the same worklist —
  `python tools/gaps.py --errors`.
- `Core\Json::decodeAs<T>`'s decoder reads a scalar-fielded class only — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- ADR 0086 § 1's substitution table, so the terminal sink neutralizes a control byte —
  `crates/mwl-stdlib/src/cli.rs` gap 1.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
