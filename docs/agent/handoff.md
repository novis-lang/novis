# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 113 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and
both of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt
clean), `mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is
**113 passed, 0 failed** and `mwl test tests/` is **599 passed, 0 failed** — run those as well as
`verify.py`, which executes no `.mwlt` case at all (playbook, twice), and **rebuild
`target/release/mwl.exe` first** if anything under `crates/` is newer than it (playbook, *Running
things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**47 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is
the same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either.

**Over `Core\Arr`, ask the key question before writing a twin, because it decides which kind of case
you are writing.** Almost every PHP array function renumbers the integer keys of its result and keeps
the string ones; ADR 0069 § 3 refuses exactly that, so an MWL member either renumbers all of them or
none. Over a list the two rules coincide and the case is an `--ORACLE--` one; over a map or a
mixed-key subject they part and the case is an `--ORACLE-DIVERGES--` one with MWL's own output frozen
in `--EXPECT--`. Four pairs now exist to copy — the window (`arr-slice-*`), the padding pair
(`arr-pad-end-matches-array_pad`, `arr-pad-on-string-keys-diverges-from-array_pad`) and `reverse`
(`arr-reverse-matches-array_reverse`, `arr-reverse-on-string-keys-diverges-from-array_reverse`).
Which way a member points is in its own doc comment in `crates/mwl-stdlib/src/arr.rs`.

**A second, unrelated divergence is settled and pinned**: PHP's next free integer key is a property
of an array's *history* and MWL's is a property of its *entries*, because a pure member returns a
fresh array with no counter to inherit. `mwl_core_arr_append`'s doc comment owns that fact and
`arr-append-derives-the-next-free-key-where-php-remembers-it.mwlt` pins it. MWL's own `$a[] = $v`
does match PHP.

**The shape a `?T` twin takes is settled** — both sides render absence as `none` and the case
compares the answer, never the representation; `str-index-of-matches-strpos.mwlt` and
`arr-contains-and-key-of-match-in_array-and-array_search-strictly.mwlt` are the worked ones, and the
`return $found as string;` ending a `?T` renderer needs is in the playbook under *Writing MWL
itself*.

**Another agent is writing `benches/userland/`, `tools/bench.py` and `docs/perf/` in this tree right
now** — those paths were untracked or modified throughout this session and are none of this loop's
work. Stage your own paths explicitly (playbook, *Tooling*); never `git commit -a`.

The spellings a case cannot use — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, an array literal wherever an `array<T>` is expected, a bare `function` at file scope,
an `int` literal in an `array<float>`'s element position — are all in the playbook under *Writing a
test case* and *Writing MWL itself*; do not re-discover them.

## Next group

All three are differential and share `tests/differential/core/` with `crates/mwl-stdlib/src/arr.rs`,
so a session that loads it once can take two. Each is a family off `python tools/gaps.py
--differential`, so no session re-derives the twin. Read the members' doc comments for the key rule
first, per *State* above. PHP on this machine is 8.5.9, so PHP 8.4's `array_find` family is available
to an oracle.

- [ ] **`Core\Arr`'s counting pair against `array_unique`/`array_count_values`** — `unique`
      (`crates/mwl-stdlib/src/arr.rs:3472`), `countBy` (`arr.rs:2525`), spec § 2. `array_unique`
      defaults to `SORT_STRING`, which collapses `0`, `"0"`, `false` and `null` into one entry where
      `unique` compares by strict identity — so expect a matching case over a single-typed list and a
      divergence case over a mixed one. `array_count_values` refuses anything but `int`/`string`
      values with a warning; `countBy` takes a callback.
- [ ] **`Core\Arr`'s predicate family against `array_find`/`array_find_key`/`array_any`/`array_all`**
      — `find` (`arr.rs:3347`), `findKey` (`arr.rs:3363`), `any` (`arr.rs:3377`), `all`
      (`arr.rs:3392`), spec § 2. All four take a callback receiving `($value, $key)`; PHP's take
      `($value, $key)` too, so the twin is close — the questions are the empty-subject answers and
      `find`'s `?T` versus PHP's `null`.
- [ ] **`Core\Arr::min`/`max` against PHP's `min`/`max`** — `arr.rs:4031`, `arr.rs:4043`, spec § 2.
      PHP's compare loosely across types and answer the *first* extreme on a tie; MWL's answer `?T`
      over an empty subject where PHP throws. `Core\Math::min`/`max` (`math.rs:816`, `math.rs:825`)
      are the same twin from the other side and can ride along.

## Backlog

- `Core\Arr::fill` against `array_fill` (`arr.rs:2287`) and `flattenDeep` against
  `iterator_to_array` (`arr.rs:2135`) — `gaps.py --differential`.
- `Core\Arr::reduce` against `array_reduce` (`arr.rs:2616`) and `sortByKey` against
  `ksort`/`krsort`/`uksort` (`arr.rs:2940`) — `gaps.py --differential`.
- The 11 `Core\Str` twins still without an oracle case — `gaps.py --differential`, `str.rs`.
- `gaps.py --errors`: the unasserted `Fault::` sites, which is the conformance half of the gap.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- ADR 0100 § 3's shebang slice: one `mwl-syntax` branch at offset 0, `E0009` already reserved.
