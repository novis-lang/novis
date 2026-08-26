# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 119 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and
both of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt
clean), `mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is
**119 passed, 0 failed** and `mwl test tests/` is **605 passed, 0 failed** — run those as well as
`verify.py`, which executes no `.mwlt` case at all (playbook, twice), and **rebuild
`target/release/mwl.exe` first** if anything under `crates/` is newer than it (playbook, *Running
things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**41 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is
the same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either.

**Over `Core\Arr`, ask the key question before writing a twin, because it decides which kind of case
you are writing.** Almost every PHP array function renumbers the integer keys of its result and keeps
the string ones; ADR 0069 § 3 refuses exactly that, so an MWL member either renumbers all of them or
none. Over a list the two rules coincide and the case is an `--ORACLE--` one; over a map or a
mixed-key subject they part and the case is an `--ORACLE-DIVERGES--` one with MWL's own output frozen
in `--EXPECT--`. Five pairs now exist to copy — the window (`arr-slice-*`), the padding pair, the
`reverse` pair, `replaceRange` and `arr-unique-*`. Which way a member points is in its own doc
comment in `crates/mwl-stdlib/src/arr.rs`; a twin that renumbers *nothing* (`array_unique`) or that
normalizes a name the way MWL does (`array_count_values`) diverges for some other reason or not at
all, so read the comment rather than assuming the key rule bites.

**Rendering is what makes a `?T` or a `bool` comparable**, and both shapes are worked out now: a
`?T` renders absence as `none` on both sides and returns `$found as string` in the guarded branch
(`arr-find-and-find-key-match-array_find-and-array_find_key`, `str-index-of-matches-strpos`), and a
`bool` goes through a two-branch `yn` helper, because `echo` writes `false` as nothing at all in
*both* languages and an empty row cannot be told from a missing one
(`arr-any-and-all-match-array_any-and-array_all`).

**A second, unrelated divergence is settled and pinned**: PHP's next free integer key is a property
of an array's *history* and MWL's is a property of its *entries*, because a pure member returns a
fresh array with no counter to inherit. `mwl_core_arr_append`'s doc comment owns that fact and
`arr-append-derives-the-next-free-key-where-php-remembers-it.mwlt` pins it. MWL's own `$a[] = $v`
does match PHP.

**Another agent is writing `benches/userland/`, `tools/bench.py`, `docs/perf/` and
`crates/mwl-runtime/src/lib.rs` in this tree right now** — those paths were modified or untracked
throughout this session and are none of this loop's work. Stage your own paths explicitly (playbook,
*Tooling*); never `git commit -a`.

The spellings a case cannot use — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, an array literal wherever an `array<T>` is expected, a bare `function` at file scope,
an `int` literal in an `array<float>`'s element position, `try`/`catch` around a `Core` member's
`FATAL:` — are all in the playbook under *Writing a test case* and *Writing MWL itself*; do not
re-discover them.

## Next group

All three are differential, all three read a doc comment in `crates/mwl-stdlib/src/arr.rs` and write
into `tests/differential/core/`, so a session that loads that file once can take two. Each is a
family off `python tools/gaps.py --differential`, so no session re-derives the twin. Read the
members' doc comments for the key rule first, per *State* above. PHP on this machine is 8.5.9.

- [ ] **`Core\Arr::min`/`max` against PHP's `min`/`max`** — `arr.rs:4031`, `arr.rs:4043`, spec § 2.
      PHP's `min`/`max` compare with `<`, which is the loose comparison, and answer the *first*
      extreme over ties; an empty array is a `ValueError` there. Expect a matching case over a
      single-typed list and a divergence over a mixed-type one, on the same reasoning
      `arr-unique-on-mixed-types-diverges-from-array_unique` records.
- [ ] **`Core\Arr::reduce` against `array_reduce`** — `arr.rs:2616`, spec § 2. The callback's
      argument order and whether the key is offered is the question; `array_reduce`'s callback takes
      `($carry, $item)` and never a key.
- [ ] **`Core\Arr::sortByKey` against `ksort`/`krsort`/`uksort`** — `arr.rs:2940`, spec § 2. MWL
      stores every key as a `string` (ADR 0007 § 5) where `ksort` compares an int key numerically,
      so the mixed-key subject is where this one is likely to need the diverging half.

## Backlog

- `Core\Arr::fill` (`arr.rs:2287`) and `flattenDeep` (`arr.rs:2135`) — the same worklist.
- `Core\Math`'s eleven twins, `math.rs:144` onward — a different file, so a different group.
- `Core\Str`'s eleven twins, `str.rs:951` onward — `mb_*` is absent from the Windows `php` (playbook).
- `gaps.py --errors`: the unasserted `Fault::` sites, now that `--EXPECT-ERROR--` is a worked shape.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
- `mwl_stdlib::json` gap 2 — `decodeAs<T>` reads a scalar-fielded class only.
