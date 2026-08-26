# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 90 of 150** — and
the gap is behavioural depth per member, not coverage: every registered member already has a case,
and both of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy
and fmt clean), `mwl test tests/conformance` is **486 passed, 0 failed** and `mwl test tests/` is
**576 passed, 0 failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all
(playbook, twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is
newer than it (playbook, *Running things*).

**The work is now differential, and the reason is cost.** A conformance case has to derive its own
`--EXPECT--`: the session writes a scratch program, runs it, reads the output and transcribes it.
An `--ORACLE--` case has no frozen output at all — PHP computes it and the runner compares — so
that whole phase does not exist. Measured over sessions 0014–0030 the loop wrote conformance
exclusively, at 2.9 cases a session, and touched differential not once. Differential is 60 short
and conformance 114, so the cheap 60 come first.

**`python tools/gaps.py` is the worklist; do not re-derive it.** `--differential` lists every member
whose spec **Replaces** column names a PHP built-in and which no oracle case calls — 67 of them,
each with the twin and a `file:line` anchor into its implementation. `--errors` lists the 109
`Fault::` sites in `mwl-stdlib` whose message no case asserts, which is the *edges* shape ready
made, `thrown` rows first because most `fatal` rows are argument type-guards the checker already
refuses. `--member 'Core\Arr::chunk'` says what the corpus already asks of one member. Both lists
are candidates, not a plan: judging which is a real claim is the session's job and is the part
worth its context.

**A twin is a claim, not a transliteration.** Check the spec row before assuming agreement: `Core\Str`
counts grapheme clusters where PHP counts bytes, and the corpus already carries four
`--ORACLE-DIVERGES--` cases saying so (`str-length-diverges-from-strlen.mwlt` is the worked one).
Where a member is deliberately unlike its twin, the case names the reason and stops — that is a
case, not a failure. Where the two agree only on ASCII, the existing convention is a case named
`…-on-ascii-matches-…`. File names are `<class>-<member>-matches-<php-fn>.mwlt` under
`tests/differential/core/`, and a case there **may** use `--ORACLE--`; one under
`tests/conformance/` may never (conventions).

Four spellings a case cannot use: a closure called through the variable holding it, the
first-class callable `Class::method(...)` (both `mwl-ir` gap 1 — declare a `class` with a `public
static function` and call it *directly*), `bool as int`, and an array literal written straight into
an `array<array<mixed>>` element, which reads as `array<mixed>` and then will not satisfy
`array<array<T>>`. What a case *can* do: an option bag as a brace literal argument, whose values may
be *variables*; `catch (RuntimeError $e) { … $e->message … }` at file scope; and
`Core\Arr::count($a) as int` wherever a `uint` meets an `int` (playbook has the `E0407`).

## Next group

All three are differential, share `tests/differential/core/`, and each takes one family off
`python tools/gaps.py --differential` so no session re-derives the twin. Take them in this order.

- [ ] **`Core\Str`'s search family against the `strpos` family** — `indexOf` (`str.rs:1504`),
      `lastIndexOf` (`str.rs:1531`), `countOf` (`str.rs:1563`), spec § 1: `strpos`/`stripos`,
      `strrpos`/`strripos` and `substr_count`. The claim worth pinning is the **not-found answer** —
      MWL returns `?uint`, PHP returns `false`, and `false` is not `0` — so the twin has to render
      both sides the same way before comparing them; that rendering *is* the case. Sweep the
      `{from}`/`{before}` and `{caseInsensitive}` options, and keep the subject ASCII so the
      grapheme rule is not what is being tested here.
- [ ] **`Core\Arr`'s ends against `reset`/`end`/`array_key_first`/`array_key_last`** — `first`
      (`arr.rs:3256`), `last` (`arr.rs:3275`), `firstKey` (`arr.rs:3298`), `lastKey`
      (`arr.rs:3313`), spec § 2. Same shape, and the edge is the **empty array**, where PHP's
      `reset`/`end` answer `false` and its `array_key_*` answer `null` while MWL answers `?T` for
      all four — one of these four may well want `--ORACLE-DIVERGES--` rather than a twin, and
      deciding which is the slice. PHP's internal array pointer has no MWL equivalent (spec § 2),
      so the twin must not depend on where `reset` leaves it.
- [ ] **`Core\Arr`'s window against `array_slice`/`array_splice`** — `slice` (`arr.rs:1599`),
      `replaceRange` (`arr.rs:1650`), `chunk` (`arr.rs:1704`), spec § 2. The conformance suite
      already proves these three agree with each other over all 72 sign combinations of offset and
      length; this asks the different question of whether **PHP** agrees, and the key-preservation
      flag is where it is most likely not to.

## Backlog

- Conformance is 114 short and every section has had a depth pass, so what is left is the shape a
  case takes rather than the section: `python tools/gaps.py --errors` is the readiest supply.
- The three conformance slices this group displaced, all in `crates/mwl-stdlib/src/arr.rs`:
  `flattenDeep` as the fixpoint of `flatten` (`arr.rs:2084`, `arr.rs:2125`); `groupBy`
  (`arr.rs:1139`) and `countBy` (`arr.rs:2515`) as one partition; `Core\Arr::column`
  (`arr.rs:2182`) over a row that does not have the cell.
- `Core\Json::decodeAs<T>`'s decoder is unbuilt — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row — that module's doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1.
- ADR 0086 § 1's substitution table — `crates/mwl-stdlib/src/cli.rs` gap 1.
