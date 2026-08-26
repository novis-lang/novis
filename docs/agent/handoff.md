# Handoff

## State

**Conformance is the only frontier left, at 496 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **496 passed, 0 failed** and `mwl test tests/` is **647 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than
it (playbook, *Running things*).

**`python tools/gaps.py --errors` is the worklist and it now prints 69 sites, of which 59 are
`fatal` and unreachable by any case, leaving 10 — and that 10 is now the honest number, with no
correction to apply.** This session closed `Core\Time\Zone::of`'s unknown-zone refusal and
`Core\Time::parse`'s two `thrown_as` in one new case each. The one site still hidden behind a
`{member}` hole is `time.rs:1968`, and it is **not owed a case**: `datetime_built` re-derives
every stored zone id from the resolved zone, so no program can reach it — the new zone case
asserts that invariant instead, over all seventeen `DateTime` members that read the slot. The
plan's `Open now` says what both cases assert past their rows.

**`time.rs` is down to `fromIso` alone**, and `math.rs`'s two are the largest single-file block
left. `orient.py`'s `[context] modules` now names `time.rs`, `math.rs` and `regex.rs` — the three
the group below reads — with `str.rs`, `encoding.rs` and `path.rs` taken out, their sections being
closed.

## Next group

These are refusal slices and they do not cluster by file the way a section pass did: what all
three share is `tests/conformance/core/` and one **read-only** `mwl-stdlib` module each, one
`peek.py` apiece, and all three modules are in the `[context] modules` manifest already. Slice 1's
two sites are one file, which is why it goes first. Every one is the *Edges* shape over a
`Fault::thrown`, closed with a counted claim and with the bound named on both sides where there is
one.

- [ ] **`Core\Math`'s two guarded arguments** — `crates/mwl-stdlib/src/math.rs:1022` (`Core\Math::log
      was given a base that is not greater than zero`) and `crates/mwl-stdlib/src/math.rs:1150`
      (`Core\Math::format was asked for {decimals} decimals, past its cap of {MAX_DECIMALS}`). Spec
      § 3 owns the rows, and the plan's `Open now` already records that base `1.0` is `NAN` on both
      sides and that the *argument's* domain is left unguarded — so the counted claim is which of
      `Core\Math`'s 38 members guards an argument at all, against the transcendental ones that
      answer `NaN`/`INF` instead. `format`'s cap is a bound to state on both sides.
- [ ] **`Core\Time::fromIso`** — `crates/mwl-stdlib/src/time.rs:1715`, whose `{err}` is `jiff`'s own
      wording and must **not** be frozen: assert the name in front of it and the offset/reason
      shape, the way the new `parse` case handles a calendar refusal. Spec § 4. The accepted side is
      RFC 3339 with `Z`, with an offset, and with a fraction; the counted claim is over the
      spellings `Instant::toIso` itself emits, which must all read back.
- [ ] **`Core\Regex\Match::group`** — `crates/mwl-stdlib/src/regex.rs:933` (`the pattern declares no
      group `{}``). Spec § 5, ADR 0063 R11. The plan already records the `groups()` reading —
      `PREG_UNMATCHED_AS_NULL` — so the case's bound is the three-way split `group` is built on:
      not declared (throws, naming the group), declared and did not participate (`null`), and
      participated capturing nothing (`""`).

## Backlog

- `Core\Time::parse` refuses a zonal *pattern* field with a `ParseError` rather than a
  `LogicError`, because `compile` cannot refuse it — `format` accepts every zonal field and shares
  that compiler, so only the reader sees it. Pinned as-is by the new case; the fix would be a typed
  error out of `mwl_stdlib::cldr::read`. `crates/mwl-stdlib/src/time.rs:2459`'s comment owns the
  intended split.
- `Core\Time\DateTime::format`'s `VV` renders `UTC` for a fixed zone (`cldr.rs:457`'s
  `unwrap_or("UTC")`), so a `+05:30` zone prints an IANA name it is not. `mwl_stdlib::cldr` owns it.
- `json.rs`'s two `thrown_as` (`:509`, `:741`) are the only assertable sites left outside the three
  slices above and `arr.rs`/`csv.rs`/`out.rs`/`random.rs`.
- 8 differential twins left, `python tools/gaps.py --differential`; `Core\Path`'s three are the
  largest block.
- `Core\Str::wrap`'s multibyte half is still owed as an `--ORACLE-DIVERGES--` file — the last
  `Core\Str` divergence file not written (plan, `Open now`).
- `docs/spec/02-php-migration.md` is 31% classified, `python tools/check-migration.py`.
