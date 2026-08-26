# Handoff

## State

**Stage 4's two counts are the frontier — conformance 468 of 600, differential 90 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean)
and `mwl test tests/conformance` is **468 passed, 0 failed** — run it as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice).

**`Core\Math` and `Core\Uri` are now done to depth**, at 11 and 10 cases, joining `hash`, `csv`,
`validate`, `out`, `heap`, `uuid`, `path`, `json`, `random`, `bytes`, `encoding`, § 9's two collections
and `regex`. Math is pinned by the bound each member is written around: `clamp`'s range is closed at
both ends, a one-point range is the last non-empty one and a `low` one above `high` is the first empty
one; the exact tie is the only input the six `RoundMode`s answer differently, so the two nearest floats
either side of `2.5` agree under every one of them and `precision` only moves the place that tie is
looked for; and `intDiv`'s `-1` is the one divisor an overflow is possible at, overflowing for exactly
one dividend, while `mod` keeps the dividend's sign even when the remainder is `-0`. Uri is pinned by
RFC 3986 § 5.4.2's abnormal table run verbatim against the RFC's own base — an ascent past the root
stops there, only a whole segment is a dot segment, and the identical text inside a query or a fragment
is data — by `parseQuery` keeping a name it cannot read rather than repairing it (a malformed bracket
is one literal key, a `.` or a space is never rewritten, a nameless pair is dropped), and by `with`
refusing any component that does not survive its own recomposition, with the port accepted at both ends
of `0-65535` and refused one past it and an absent component (`null`) told from an empty one (`""`).

**Three case shapes are established** and named in the plan's *Open now*: a section's edges, invariance
over a sweep, and a bound asserted on both sides. Reuse them rather than inventing a fourth.

## Next group

`Core\Time` closes the group this session started; after it, only the two largest sections have never
had a pass, and they are the whole remainder. The file set they share is
`crates/mwl-stdlib/src/{time,str,arr}.rs` and `tests/conformance/core/`.

- [ ] **`Core\Time` depth** — `crates/mwl-stdlib/src/time.rs:473` `Duration::parse` and the eight
      component readers at `:405`-`:458`; spec § 4. Two `time-` cases. `parse` shares ADR 0070's literal
      grammar, so the case is that one grammar reached from two entry points, plus each unit's boundary.
- [ ] **`Core\Str` depth** — `crates/mwl-stdlib/src/str.rs:1412` `slice`, `:901` `split`, `:1199`
      `replace`, `:1997` `padStart`; spec § 1. Three `str-` cases. The shape is a bound on both sides:
      `slice`'s offset and length at each end and one past it, including the negative spellings; `split`
      with a limit at the exact piece count and one past it; and the pad members at a width the subject
      already meets. The `mb_*` half of PHP has no oracle on the Windows leg (playbook), so a Unicode row
      cites the UCD table instead.
- [ ] **`Core\Arr` depth** — `crates/mwl-stdlib/src/arr.rs:1599` `slice`, `:1704` `chunk`, `:2765`
      `sort`, `:3462` `unique`; spec § 2. Three `arr-` cases. `slice`/`chunk` are the same bound as
      `Str`'s over a different subject, `sort` is an invariant (a permutation of its input, stable at
      equal keys), and `unique` is ADR 0090 § 3's identity table read one row per representation, as
      § 9's collections already are.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder and ADR 0071's non-scalar fields — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — `mwl_stdlib::hash` module doc.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1's remainder.
- The differential corpus is 90 of 150 — `docs/plan/M4S.md`'s Stage 4 paragraph.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- ADR 0090 § 3's string/array/object equality helpers are still owed — that ADR's own body.
