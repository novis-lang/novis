# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 123 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and
both of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt
clean), `mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is
**123 passed, 0 failed** and `mwl test tests/` is **609 passed, 0 failed** — run those as well as
`verify.py`, which executes no `.mwlt` case at all (playbook, twice), and **rebuild
`target/release/mwl.exe` first** if anything under `crates/` is newer than it (playbook, *Running
things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**38 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is
the same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either.

**Over `Core\Arr`, ask the key question before writing a twin, because it decides which kind of case
you are writing.** Almost every PHP array function renumbers the integer keys of its result and keeps
the string ones; ADR 0069 § 3 refuses exactly that, so an MWL member either renumbers all of them or
none. Over a list the two rules coincide and the case is an `--ORACLE--` one; over a map or a
mixed-key subject they part and the case is an `--ORACLE-DIVERGES--` one with MWL's own output frozen
in `--EXPECT--`. Five pairs now exist to copy — the window (`arr-slice-*`), the padding pair, the
`reverse` pair, `replaceRange` and `arr-unique-*`. Which way a member points is in its own doc
comment in `crates/mwl-stdlib/src/arr.rs`; read the comment rather than assuming the key rule bites.
**A member answering a value rather than an array escapes the rule entirely** — `min`/`max` and
`reduce` all match PHP over a map — so for those the divergence, if there is one, is in the
comparison or the callback protocol instead.

**Two comparison facts are settled and pinned.** MWL orders by
`mwl_stdlib::ordering::compare_values`, one row per representation with nothing crossing except the
two numeric ones, where PHP's `<` converts: `min([0, "a"])` is `0` there (the `int` cast to a string)
and a throw here, and two numeral strings compare *numerically* there and bytewise here, so
`min(["1e2", "50"])` is `"50"` in PHP and `"1e2"` in MWL. That refusal is a `Fault::thrown` and a
`catch (Throwable $e)` does reach it, unlike the `Fault::fatal` an argument-shape guard raises
(playbook, *Writing a test case*, two adjacent bullets).

**Rendering is what makes a `?T` comparable**: absence is `none` on both sides and the guarded branch
returns `$found as string` (`arr-min-and-max-match-*`, `arr-find-and-find-key-match-*`). A `?bool`
has no such shape — the truthy condition panics `mwl-ir` — so keep a `bool`-valued subject out of a
case about a `?T`-answering member (playbook, same section).

The spellings a case cannot use — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, an array literal wherever an `array<T>` is expected, a bare `function` at file scope,
an `int` literal in an `array<float>`'s element position — are all in the playbook under *Writing a
test case* and *Writing MWL itself*; do not re-discover them.

## Next group

All three are differential, all three read a doc comment in `crates/mwl-stdlib/src/arr.rs` and write
into `tests/differential/core/`, so a session that loads that file once can take two. Each is a
family off `python tools/gaps.py --differential`, so no session re-derives the twin. Read the
members' doc comments for the key rule first, per *State* above. PHP on this machine is 8.5.9.

- [ ] **`Core\Arr::sortByKey` against `ksort`/`krsort`/`uksort`** — `crates/mwl-stdlib/src/arr.rs:2940`,
      spec § 2. MWL stores every key as a `string` (ADR 0007 § 5) where `ksort` compares an int key
      numerically, so a mixed-key or numeral-key subject is where the diverging half lives; the
      comparison itself is `ordering.rs:33`, the same one `min`/`max` just diverged over.
- [ ] **`Core\Arr::fill` against `array_fill`** — `crates/mwl-stdlib/src/arr.rs:2287`, spec § 2.
      `array_fill`'s start index may be negative, which in PHP 8 builds a list from that index; MWL's
      keys are strings, so ask what a negative start means here before writing the rows.
- [ ] **`Core\Arr::flattenDeep` against `iterator_to_array`** — `crates/mwl-stdlib/src/arr.rs:2135`,
      spec § 2. The twin is only half a twin — PHP has no array flatten — so expect the matching half
      to be narrow and the case to carry most of its weight in `--ORACLE-DIVERGES--` prose.

## Backlog

- `Core\Str` has eleven differential gaps left (`gaps.py --differential`); `compare`, `slice` and
  `chunk` are the largest.
- `Core\Math` has eleven, all numeric and cheap to write once one of them settles float rendering.
- `--errors`: the unasserted `Fault::` sites on the conformance side, which is the other half of
  Stage 4's gap (`docs/implementation-plan.md`, *Open now*).
- ADR 0088's qualifier classification is unbuilt across every `mwl-stdlib` member row
  (`mwl_stdlib::hash`'s module doc).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
