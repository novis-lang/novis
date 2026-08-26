# Handoff

## State

**Conformance is at 545 of 600, and it is the only frontier left.** Verify is green (1597 cargo tests,
74 suites, 545 conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt` trees itself,
so after a green `verify.py` there is nothing else to run (playbook, *Running things*).

This session added no library code. It took the first two slices of the `Core\Time` group, both new
files under `tests/conformance/core/` and both agreement sweeps over `crates/mwl-stdlib/src/time.rs`:

- `time-instant-order-and-difference-agree-and-a-zone-is-only-a-reading.mwlt` — 100 pairs of a
  ten-row table asserting that `compareTo`'s sign is `since`'s sign, that `since` is antisymmetric,
  and that `minus` and `plus` each rebuild the other end; then 50 readings asserting `in`→`toInstant`
  is the identity in five zones, 100 cross-zone pairs asserting order and difference survive being
  read in two *different* calendars, and the five renderings that show `in` is not decorative.
- `time-datetime-parts-agree-with-the-value-they-came-from.mwlt` — 24 values (six instants × four
  zones) asserting `date`, `timeOfDay`, `zone`, `dayOfYear` and `isLeapYear` each agree with what the
  value `format`s as, and 36 pairs asserting `difference` in an absolute unit is `Instant::since` with
  the operands swapped. Its tail pins the one place the zone decides the answer: 23 absolute hours is
  no whole day in UTC and exactly one in Europe/Berlin, because the Berlin day between them is 23
  hours long.

`gaps.py --coverage` named `date`, `dayOfYear` and `difference` as the three `DateTime` members no case
called, and all three are called now. `orient.py`'s pack was complete for this work; nothing was fetched
outside it beyond `time.rs`'s member tables and the part-member bodies.

**A by-hand pass over `docs/adr/` is still in flight and is not loop work.** 103 modified ADRs plus
`ground-rules.md` have been uncommitted for three sessions now — a `Scope:` field added, `Supersedes:`
renamed to `Amends:`, heading levels moved. That is [doc-cleanup.md](doc-cleanup.md)'s pass, which
AGENTS.md says the user fires and the loop never does. **Do not stage it and do not `git commit -a`**:
stage your own paths, exactly as `session.py --wrap` already does.

## Next group

Three slices, **all on `crates/mwl-stdlib/src/time.rs`** and all writing a new file under
`tests/conformance/core/`, so a session taking two pays for the file set once.
`docs/spec/01-core-library.md` § 4 owns the `Time` rules. `Core\Time\Date` is the thinnest class left
at 0.33 cases per member; `TimeOfDay` and `Duration` are next at 0.67 and 0.37.

- [ ] **`Core\Time\Date` is six members over one civil date, and they undo each other** — `at` and
      `format` are inverses over a table of dates, `plus`/`minus` in the same unit return the date
      they started from (except where a month step clamps, which is the boundary to name on both
      sides), `with` is the identity when handed the parts `format` just read, and `compareTo` agrees
      with the calendar order of the same table. `crates/mwl-stdlib/src/time.rs:2664` (`at`), `:2706`
      (`plus`), `:2713` (`minus`), `:2725` (`with`), `:2756` (`compareTo`).
- [ ] **`Core\Time\TimeOfDay` is a clock that wraps, and says so at both ends** — `plus`/`minus` wrap
      around midnight rather than carrying a date, so a sweep of offsets from a fixed time is
      `(minute-of-day + n) mod 1440` on every row, and `compareTo` orders by that same number.
      `crates/mwl-stdlib/src/time.rs:2860` (`at`), `:2905` (`plus`), `:2920` (`with`).
- [ ] **`Core\Time\Duration`'s constructors and its `to…` readers are one scale** — every unit
      constructor times its own factor is the same nanosecond count, `multipliedBy` and `negated`
      compose, and `parse` reads back what `toString` wrote for a table spanning both signs.
      `crates/mwl-stdlib/src/time.rs:473` (`parse`), `:545` (`multipliedBy`).

## Backlog

- `Core\Uri` is 0.53 cases per member over 19 — the thinnest class outside `Core\Time`
  (`python tools/gaps.py --coverage`).
- `gaps.py --differential` still names 9 members with a PHP twin and no oracle case, all `Core\Time`
  and `Core\Encoding`; those belong in `tests/differential/`, not here.
- `Core\Json::decodeAs<T>` still decodes a scalar-fielded class only (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row
  (`mwl_stdlib::hash`'s module doc).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
