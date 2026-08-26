# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 97 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and
both of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt
clean), `mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is
**97 passed, 0 failed** and `mwl test tests/` is **583 passed, 0 failed** — run those as well as
`verify.py`, which executes no `.mwlt` case at all (playbook, twice), and **rebuild
`target/release/mwl.exe` first** if anything under `crates/` is newer than it (playbook, *Running
things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**59 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and
`--errors` is the same list for the 109 unasserted `Fault::` sites on the conformance side. Do not
re-derive either.

**The shape a `?T` twin takes is settled, and the seven cases below share it.** A member answering
`?T` has no PHP counterpart that can be compared directly — `strpos` answers `false`, `reset` answers
`false`, `array_key_first` answers `null` — so both sides render absence as `none` first and the case
compares the *answer*, never the representation. In MWL that is a `class Show` with a
`public static function render(?T $found): string`, and its body must end `return $found as string;`
because a `== null` guard does not narrow the binding (playbook, *Writing MWL itself*). In the oracle
it is a plain PHP function that must not be named after a built-in (playbook, *Writing a test case*).
`str-index-of-matches-strpos.mwlt` is the worked one.

**A twin is a claim, not a transliteration.** Check the spec row before assuming agreement: `Core\Str`
counts grapheme clusters where PHP counts bytes, and the corpus carries five `--ORACLE-DIVERGES--`
cases saying where the two part — the newest,
`str-index-of-out-of-range-from-diverges-from-strpos.mwlt`, is the shape to copy when a member
saturates a position PHP raises a `ValueError` for. Where the two agree only on ASCII, the convention
is a case named `…-on-ascii-matches-…`. File names are `<class>-<member>-matches-<php-fn>.mwlt` under
`tests/differential/core/`, and a case there **may** use `--ORACLE--`; one under `tests/conformance/`
may never (conventions).

The spellings a case cannot use — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, an array literal in an `array<array<mixed>>` element, a bare `function` at file
scope — are all in the playbook under *Writing a test case* and *Writing MWL itself*; do not
re-discover them.

## Next group

All three are differential, and they share `tests/differential/core/` and one region of
`crates/mwl-stdlib/src/arr.rs` (1599-1980), so a session that loads it once can take two. Each is a
family off `python tools/gaps.py --differential`, so no session re-derives the twin.

- [ ] **`Core\Arr`'s window against `array_slice`/`array_splice`/`array_chunk`** — `slice`
      (`crates/mwl-stdlib/src/arr.rs:1599`), `replaceRange` (`arr.rs:1650`), `chunk` (`arr.rs:1704`),
      spec § 2. The conformance suite already proves these three agree with each other over all 72
      sign combinations of offset and length; this asks whether **PHP** agrees, and
      `array_slice`'s/`array_chunk`'s key-preservation flag is where it is least likely to. Render an
      array with the `Render::of` helper `arr-values-after-unset-matches-array_values.mwlt` already
      carries, so keys are compared and not just values.
- [ ] **`Core\Arr`'s ends-as-values against `array_push`/`array_unshift`/`array_shift`/`array_pop`** —
      `append` (`arr.rs:1757`), `prepend` (`arr.rs:1792`), `withoutFirst` (`arr.rs:1826`),
      `withoutLast` (`arr.rs:1862`), spec § 2. MWL's four are pure and answer the new array where
      PHP's mutate in place and answer a count or the removed entry, so the twin compares the PHP
      array *after* the call against MWL's result — and `withoutFirst`'s renumbering of a list is the
      row `array_shift` is most likely to differ on.
- [ ] **`Core\Arr`'s pad and reverse against `array_pad`/`array_reverse`** — `padStart`
      (`arr.rs:1938`), `padEnd` (`arr.rs:1955`), `reverse` (`arr.rs:1977`), spec § 2. One PHP
      function covers both pad members by the sign of its length, which makes the pair an *agreement*
      case; `array_reverse`'s `$preserve_keys` is the flag to sweep on both settings.

## Backlog

- `Core\Str`'s remaining twins — `compare`, `slice`, `before`, `after`, `chunk`, `replaceAll`,
  `replaceRange`, `reverse`, `wrap` (`gaps.py --differential`, `str.rs:1412`-`1899`).
- Conformance is the larger gap at 486 of 600, and `gaps.py --errors` is its ready-made worklist
  (`docs/implementation-plan.md`, *Open now*).
- `Core\Json::decodeAs<T>` reads a scalar-fielded class only (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row
  (`mwl_stdlib::hash`'s module doc).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
