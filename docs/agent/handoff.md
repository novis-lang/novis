# Handoff

## State

**`Core\Time\Date` is finished to depth.** Three cases landed this session and `python tools/gaps.py
--coverage` now reads the class at depth 6.0 / floor 5 (`compareTo 5, minus 6, plus 6`), so it is no
longer the thinnest § 4 class — `Core\Time\DateTime` is (floor 3). No `nvs-stdlib` source changed:
every member the three cases reach already answered correctly, which is what the sweeps say.

**The corpus is at 934**, all passing. The Stage 8 acceptance check wants 950 and still fails — a
growth floor, not a regression: no case fails and no check names a test that disappeared.

**Two facts about `Date::at` the cases pin and nothing else records.** Its fields have *two* bounds
apiece, one entry apart, with a different sentence each: the calendar's (`parameter 'month' is not in
the required range of 1..=12`) and `parts`'s narrowing to `i16`/`i8` before `civil::Date::new` sees
anything (`a year is within -9999..=9999, and a month and a day are numbers a calendar writes`) —
the join is at 32,767/32,768 for the year and 127/128 for the month and the day. And where
`TimeOfDay::at` refuses its four fields in one sentence naming the whole reading, `Date::at` names
the *outermost* field that is wrong, year before month before day: two constructors, two message
shapes, both now frozen. `date_stepped` has the same two-sentence split on its count, at
7,304,484/7,304,485 days.

**What the three new cases pin**, all in `tests/conformance/core/`:

- `time-date-compare-to-is-one-total-order-and-arr-sort-agrees-with-it.nvst` — the `TimeOfDay` twin's
  sweep over a 12-row table (144 pairs, 1,728 triples), plus the two halves a calendar has and a
  clock does not: the order is year-then-month-then-day, parting from a day-first cascade on 44 of
  the 144 pairs, and it is *not* the order its own rendering sorts under — 4 pairs disagree, all
  across the minus sign two BCE rows put in the text.
- `time-date-at-bounds-each-field-on-both-sides.nvst` — each field's ends named with their refused
  neighbours, the day's ceiling read from the other two fields over six (year, month) rows including
  both halves of the century rule (2000 leap, 1900 not), and the second bound above.
- `time-date-ends-refuse-where-the-clock-wraps.nvst` — the calendar is an interval and the dial is
  closed: a step off either end refuses in all five units a date takes and one step inside is
  answered, against twelve moves off the dial's two ends that all wrap and none refuse; and the step
  is modular for the clock and not for the calendar, counted both ways.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`),
a `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have
playbook bullets under *Writing a test case*.

**Orientation.** The pack was complete for this group; nothing outside it was read. The manifest list
the last two sessions asked for is still owed — `hash.rs`, `test.rs`, `nvs-types/src/links.rs`,
`routes.rs`, `validate.rs`, `docs/spec/01-core-library.md`, `docs/spec/02-php-migration.md`,
`tools/check-migration.py`, the stage-7 comment header's per-goal floor table, a selector printing
the failing check's own `cases` block, and a `[context] anchors` entry for `registry.rs`'s
`UNCLASSIFIED`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/time.rs` and `tests/conformance/core/`. `Core\Time\DateTime`
is now the thinnest § 4 class — `gaps.py --coverage` reads floor 3 over 17 members, `isLeapYear 3,
dayOfYear 4, timeOfDay 4` — and its three thinnest members are all *agreement* shaped, which is the
shape with the most room left and the one the three `Date` cases just worked.

- [ ] **`isLeapYear` is February's last day and the year's length, said three ways**
      (`nvs_core_time_datetime_is_leap_year` at `crates/nvs-stdlib/src/time.rs:2470`, `date_of` at
      `:2619`) — counted over a table of years spanning both halves of the century rule and both ends
      of the range, asserting the three agree rather than what each answered: the flag, whether
      `Core\Time\Date::at($y, 2, 29)` is accepted, and whether `dayOfYear` reaches 366.
- [ ] **`dayOfYear` is one increasing run with no gap and no repeat**
      (`nvs_core_time_datetime_day_of_year` at `crates/nvs-stdlib/src/time.rs:2460`) — over a whole
      year stepped a day at a time, the count is `1..=365`/`366` each exactly once and the step of one
      day is the step of one in the count; plus the two ends, 12-31 and 01-01 either side of a
      new year.
- [ ] **`date()` and `timeOfDay()` are the two halves that reassemble the value**
      (`nvs_core_time_datetime_date` at `crates/nvs-stdlib/src/time.rs:2419`,
      `nvs_core_time_datetime_time_of_day` at `:2429`) — over a table, the two components rendered
      together are the whole value's own rendering, and each half compares to its sibling's the way
      the wholes do.

## Backlog

- Item 12's 10 `UNCLASSIFIED` members — `crates/nvs-stdlib/src/registry.rs:1526`, ADR 0088 § 2.
- A `bytes` array key ICEs in `nvs-ir` — playbook, *Writing a test case*.
- `catch (Core\Error $e)` panics rather than diagnosing — playbook, *Writing a test case*.
- `Date::at` and `TimeOfDay::at` refuse in two different message shapes; both are pinned, neither is
  argued anywhere. ADR 0063 R4 fixes that failure throws, not what the sentence names.
- The `[context]` manifest gaps listed under *Orientation* above — `docs/agent/loop-goal.toml`.
- The Stage 8 conformance floor is 950 against 934 on disk — `docs/agent/loop-goal.toml`.
