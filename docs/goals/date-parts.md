---
looper: 1
title: "a Date, a DateTime and a TimeOfDay return their parts, and a Date becomes a DateTime"
summary: "Reached when Core\\Time\\Date has weekday, dayOfYear, isLeapYear, year, month, day and atStartOfDay, DateTime and TimeOfDay return each of their fields as a number, no Novis code reads a field by parsing format, the CLDR pattern table on the reference page is the set the compiler accepts and a test keeps it so, and every new member has its feature proofs."
context: [crates/nvs-stdlib/src/time.rs, docs/reference/core/Time-Date.md, docs/reference/core/Time-DateTime.md, docs/reference/core/Time-TimeOfDay.md]
stages:
  - id: 1
    title: "a Date returns its parts and becomes a DateTime, and the reference pages say what the compiler accepts"
    status: done
    context: [crates/nvs-stdlib/src/cldr.rs]
  - id: 2
    title: "a DateTime and a TimeOfDay return each field as a number, and the intl extension reads them"
    status: done
    context: [extensions/intl/nvs/DateFormat.nvs, tests/conformance/core/time-datetime-parts-agree-with-the-value-they-came-from.nvst]
  - id: 3
    title: "the seven Date members have their feature proofs"
    status: done
    context: [docs/agent/prose.md, docs/examples/core/Time-DateTime/weekday/about.md]
  - id: 4
    title: "the eleven DateTime and TimeOfDay readers have their feature proofs"
    status: done
    context: [docs/agent/prose.md, docs/examples/core/Time-DateTime/dayOfYear/about.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: rust-date
    stage: 1
    name: "a Date's parts are its three fields and its calendar, and atStartOfDay is the first instant of that day in the zone"
    run: [cargo, test, --lib, --, date_parts_]
    timeout: 1800
    tests:
      - date_parts_are_the_three_fields_and_the_calendar_of_that_day
      - date_parts_at_start_of_day_is_the_first_instant_of_that_day_in_the_zone
  - id: table
    stage: 1
    name: "the pattern table on the DateTime reference page lists exactly the letters the pattern compiler accepts"
    run: [cargo, test, --lib, --, the_reference_table_lists_every_letter_the_compiler_accepts]
    timeout: 1800
    tests:
      - the_reference_table_lists_every_letter_the_compiler_accepts
  - id: case-date
    stage: 1
    name: "a Date's parts agree with format, with DateTime's, and atStartOfDay round-trips through date()"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/time-date-parts-agree-with-the-date-they-came-from.nvst]
    timeout: 1800
  - id: rust-fields
    stage: 2
    name: "a DateTime's fields are read in its own zone, and a TimeOfDay's are its four slots"
    run: [cargo, test, --lib, --, fields_are_]
    timeout: 1800
    tests:
      - datetime_fields_are_read_in_the_values_own_zone
      - time_of_day_fields_are_its_four_slots
  - id: case-datetime
    stage: 2
    name: "a DateTime's field readers agree with format over the whole table, in every zone it uses"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/time-datetime-parts-agree-with-the-value-they-came-from.nvst]
    timeout: 1800
  - id: case-time-of-day
    stage: 2
    name: "a TimeOfDay's field readers agree with format and with the DateTime it came from"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/time-of-day-parts-agree-with-the-reading-they-came-from.nvst]
    timeout: 1800
  - id: proofs-date
    stage: 3
    name: "the seven new Date members have every feature proof"
    run: [bun, nv, proofs, --only, "Core\\Time\\Date::weekday", "Core\\Time\\Date::dayOfYear", "Core\\Time\\Date::isLeapYear", "Core\\Time\\Date::year", "Core\\Time\\Date::month", "Core\\Time\\Date::day", "Core\\Time\\Date::atStartOfDay"]
    timeout: 1800
  - id: proofs-fields
    stage: 4
    name: "the eleven new DateTime and TimeOfDay readers have every feature proof"
    run: [bun, nv, proofs, --only, "Core\\Time\\DateTime::year", "Core\\Time\\DateTime::month", "Core\\Time\\DateTime::day", "Core\\Time\\DateTime::hour", "Core\\Time\\DateTime::minute", "Core\\Time\\DateTime::second", "Core\\Time\\DateTime::nanos", "Core\\Time\\TimeOfDay::hour", "Core\\Time\\TimeOfDay::minute", "Core\\Time\\TimeOfDay::second", "Core\\Time\\TimeOfDay::nanos"]
    timeout: 1800
---

# a Date, a DateTime and a TimeOfDay return their parts, and a Date becomes a DateTime

## Why

A user's program needed `isWorkingDay(Date $d)`. `Core\Time\Date` has no `weekday()`, and there is no
documented way to turn a `Date` into a `DateTime`, which has one. So the program reads the weekday with
`format('e')`. That works: `e` is the ISO weekday, Monday `1`. But `e` is on no page a user reads.

The gap is wider than the report. No `Core\Time` value returns a year, a month, a day or an hour as a
number, because a `Core` instance has no property a program can reach and no class has a reader. Every
program parses `format` instead, and so does this repository: `extensions/intl/nvs/DateFormat.nvs:16`
builds its record from `format('M') as int` and five more like it.

The reference pages are behind the code too. `docs/reference/core/Time-DateTime.md:33` says `c`, `w`,
`W`, `L`, `F`, `Q` and `G` are refused, but the compiler accepts them and about fifteen more letters
(`crates/nvs-stdlib/src/cldr.rs:40`). `Time-Date.md:12` and `Time-TimeOfDay.md:11` say `<` does not
compile, but `Core\Time\Date::at(2024, 1, 6) < Core\Time\Date::at(2024, 1, 8)` is `true` on the release
binary of 2026-11-19.

## Standing decisions

**The members (agreed with the person).** `Date` gains `weekday()`, `dayOfYear()`, `isLeapYear()`,
`year()`, `month()`, `day()` and `atStartOfDay(Zone $zone)`. `DateTime` gains `year()`, `month()`,
`day()`, `hour()`, `minute()`, `second()` and `nanos()`. `TimeOfDay` gains `hour()`, `minute()`,
`second()` and `nanos()`. The reference pages list what the compiler accepts, and a test keeps them so.

**Names and types are `with`'s (my call).** Each reader has the name and type of the `with` option for
the same field (`crates/nvs-stdlib/src/time.rs:1568`): `year(): int`, every other field `uint`. So
`$d->with({ month: $d->month() + 1 })` reads as one vocabulary. A `DateTime`'s readers read the civil
time in the value's own zone, as `dayOfYear` and `weekday` already do.

**`atStartOfDay`, not an instance `at` (my call).** `Date::at` is already the static constructor, so an
instance member of that name would be two meanings for one name. `atStartOfDay(Zone $zone): DateTime`
is the first instant of the day in the zone. A zone that skips midnight resolves forward, which is the
compatible disambiguation `Core\Time::at` uses (`crates/nvs-stdlib/src/time.rs:4669`). A time of day is
added with the existing `DateTime::withTime`, so no second member takes a `TimeOfDay`.

**`Date::weekday()` returns `Core\Time\DateTime::Weekday` (my call).** The enum keeps its name. A
`Core\Time\Weekday` would break every caller and buy nothing a reader needs.

**The pattern table is the compiler's (my call).** `Time-DateTime.md` lists every letter `Field::of`
accepts (`crates/nvs-stdlib/src/cldr.rs:349`), with `e` described as the weekday number, Monday `1`.
Its "refused" sentence names only the letters `Field::of` refuses. The Rust test
`the_reference_table_lists_every_letter_the_compiler_accepts`, in `cldr.rs`'s tests (`:3108`), reads the
page with `include_str!` and checks the table's letters against `Field::of` both ways. The page is text a
user reads, so it follows `AGENTS.md` § *Text an end user reads*.

**Which `format` reads switch (my call).** `extensions/intl/nvs/DateFormat.nvs` switches to the readers,
because it is real code on a request path and each `format` call compiles a pattern at run time. The
conformance cases that compute an expected value with `format` keep it: there `format` is the
independent oracle the case checks against.

**`Core\Time` is no longer "whole".** The module doc's claim (`crates/nvs-stdlib/src/time.rs:54`) and its
"days in a month" paragraph (`:140`) are rewritten to say what the classes return now.

**What it costs.** No slot and no memory per value. A reader allocates nothing: it reads a slot, or one
civil field of the zoned time a `DateTime` already derives. Each bench declares `allocations 0`.
`atStartOfDay` builds one `DateTime`, the same as `Core\Time::at`. The surface grows by 18 members, each
with its feature proofs. Pattern parsing goes out of the intl extension.

**Lanes.** Stages 1 and 2 both edit `crates/nvs-stdlib/src/time.rs`, so they run in order. Stage 3
writes under `docs/examples/core/Time-Date/`, `tests/hostile/core/Time-Date/` and
`benches/members/core/Time-Date/` alone. Stage 4 writes under the `Time-DateTime` and `Time-TimeOfDay`
directories of the same three trees. Stages 3 and 4 have disjoint files and may run as side goals once
stage 2 is done.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. a Date returns its parts and becomes a DateTime, and the reference pages say what the compiler accepts

**Does:** the seven `Date` members and the three reference pages.

- [x] `crates/nvs-stdlib/src/time.rs:2570`: after `compareTo` in `DATE`'s `instance` list, `weekday`,
  `dayOfYear`, `isLeapYear`, `year`, `month`, `day` and `atStartOfDay`, each with its reference card held
  to `AGENTS.md` § *Text an end user reads*. The bodies sit beside `nvs_core_time_date_compare_to`
  (`:4939`) and read the date with `date_of` (`:4736`). `weekday` returns the `WEEKDAY` index as
  `nvs_core_time_datetime_weekday` does (`:4545`). Each symbol joins `date_address` (`:2743`).
- [x] `atStartOfDay` builds its value as `nvs_core_time_at` does (`:4669`): the date at midnight, then
  `zone.to_zoned`, with the same `PAST_THE_RANGE` error. Its card says a zone that skips midnight gives
  the first time that exists that day.
- [x] The Rust tests `date_parts_are_the_three_fields_and_the_calendar_of_that_day` and
  `date_parts_at_start_of_day_is_the_first_instant_of_that_day_in_the_zone` in the tests module
  (`:5183`), each with its `// covers:` line. The second uses a zone whose clocks skip midnight on one
  day. The `rust-date` check runs once.
- [x] `tests/conformance/core/time-date-parts-agree-with-the-date-they-came-from.nvst:1`: over a table of
  dates, including both ends of `-9999..=9999` and 29 February, each reader agrees with `format` and
  with the same reader on `atStartOfDay($zone)`, and `atStartOfDay($zone)->date()` is the date again.
  Its `// covers:` line names the seven members. The `case-date` check runs once.
- [x] `docs/reference/core/Time-DateTime.md:16`: the table lists every letter `Field::of` accepts, and
  the sentence at `:32` names only the letters it refuses. Then the Rust test
  `the_reference_table_lists_every_letter_the_compiler_accepts` in `crates/nvs-stdlib/src/cldr.rs:3108`.
  The `table` check runs once.
- [x] `docs/reference/core/Time-Date.md:6` names the new members and `atStartOfDay` as the way to a
  `DateTime`, and `:12` says `<` orders two dates. `docs/reference/core/Time-TimeOfDay.md:11` gets the
  same `<` fix, after a one-line case shows `<` compiles on two `TimeOfDay` values.
- [x] `crates/nvs-stdlib/src/time.rs:54` and `:140`: the module doc says what the three classes return
  now, and drops "`Core\Time` is whole".

### 2. a DateTime and a TimeOfDay return each field as a number, and the intl extension reads them

**Does:** the eleven readers, and the one Novis file that parses `format` for a number.

- [x] `crates/nvs-stdlib/src/time.rs:1779`: after `isLeapYear` in `DATETIME`'s `instance` list, `year`,
  `month`, `day`, `hour`, `minute`, `second` and `nanos`, each with its card. The bodies sit beside
  `nvs_core_time_datetime_day_of_year` (`:4556`) and read `zoned_of(...)` (`:4063`). Each symbol joins
  `DATETIME`'s address function.
- [x] `crates/nvs-stdlib/src/time.rs:2874`: after `compareTo` in `TIME_OF_DAY`'s `instance` list, `hour`,
  `minute`, `second` and `nanos`, each with its card, reading `clock_of` (`:4972`). Each symbol joins
  `time_of_day_address` (`:3064`).
- [x] The Rust tests `datetime_fields_are_read_in_the_values_own_zone` and
  `time_of_day_fields_are_its_four_slots`, each with its `// covers:` line. The first reads one instant
  in two zones on either side of the date line. The `rust-fields` check runs once.
- [x] `tests/conformance/core/time-datetime-parts-agree-with-the-value-they-came-from.nvst:5`: the
  `// covers:` line gains the seven readers, and the table checks each against `format`. The
  `case-datetime` check runs once.
- [x] `tests/conformance/core/time-of-day-parts-agree-with-the-reading-they-came-from.nvst:1`: each of
  the four readers agrees with `format` and with the `DateTime` the reading came from, at `00:00`,
  `23:59:59.999999999` and a reading with nanoseconds. The `case-time-of-day` check runs once.
- [x] `extensions/intl/nvs/DateFormat.nvs:16`: the record is built from the readers, not from `format`.
  `cargo test --test intl` runs once.
- [x] `docs/reference/core/Time-DateTime.md` and `Time-TimeOfDay.md` name the readers in their intro.

### 3. the seven Date members have their feature proofs

**Does:** the feature proofs. Every line here is text an end user reads. Run `bun nv proofs --only` with
the `proofs-date` check's seven ids first and last.

**Split:** three writing subagents, one per tree: the examples and `about.md` files, the attacks, the
benches. Their files are disjoint, and none builds until all end. Each brief names
`docs/agent/prose.md` and the `Time-DateTime/weekday` or `Time-DateTime/dayOfYear` proof of the same
kind as its model.

- [x] `docs/examples/core/Time-Date/<member>/about.md:1` and three examples with their `.out` for each
  of `weekday`, `dayOfYear`, `isLeapYear`, `year`, `month`, `day` and `atStartOfDay`. The `weekday`
  examples include a working-day test. The `atStartOfDay` examples include `withTime` for a time of day.
- [x] `tests/hostile/core/Time-Date/<member>/01-….nvs:1` for each of the seven, modelled on
  `tests/hostile/core/Time-DateTime/dayOfYear/01-the-ends-of-the-calendar-and-a-skipped-day.nvs`.
  `atStartOfDay`'s attack uses a zone that skips midnight and both ends of the range.
- [x] `benches/members/core/Time-Date/<member>.nvs:1` for each of the seven, modelled on
  `benches/members/core/Time-DateTime/dayOfYear.nvs`. The six readers declare `allocations 0`.
- [x] The `proofs-date` check is green, and `bun nv proofs --comments` passes on the new files.

### 4. the eleven DateTime and TimeOfDay readers have their feature proofs

**Does:** the feature proofs for `DateTime`'s `year`, `month`, `day`, `hour`, `minute`, `second` and
`nanos`, and `TimeOfDay`'s `hour`, `minute`, `second` and `nanos`. Run `bun nv proofs --only` with the
`proofs-fields` check's eleven ids first and last.

**Split:** as stage 3, three writing subagents, one per tree.

- [x] `docs/examples/core/Time-DateTime/<member>/` and `docs/examples/core/Time-TimeOfDay/<member>/`:
  `about.md` and three examples with their `.out` for each of the eleven. The `DateTime` `about.md`
  files say the field is read in the value's own zone.
- [x] `tests/hostile/core/Time-DateTime/<member>/01-….nvs:1` and
  `tests/hostile/core/Time-TimeOfDay/<member>/01-….nvs:1` for each of the eleven.
- [x] `benches/members/core/Time-DateTime/<member>.nvs:1` and `benches/members/core/Time-TimeOfDay/<member>.nvs:1`
  for each of the eleven, each declaring `allocations 0`.
- [x] The `proofs-fields` check is green, and `bun nv proofs --comments` passes on the new files.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and this goal's own tests at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 4 are done. Stage 4 landed in one commit, written by three subagents:

- `docs/examples/core/Time-DateTime/<member>/` (seven) and `docs/examples/core/Time-TimeOfDay/<member>/`
  (four): `about.md` and three examples with `.out` each. Every `DateTime` `about.md` says the field
  is read in the value's own zone, and each shows the matching `with` call.
- `tests/hostile/core/Time-DateTime/<member>/01-….nvs` and `Time-TimeOfDay/<member>/01-….nvs`: one
  attack each (range ends, Samoa's skipped day, Berlin's skipped and repeated hours, LMT offsets with
  odd minutes and seconds, the 2016 leap second, the wrap at midnight). All eleven pass under
  `bun nv proofs --run`.
- `benches/members/core/Time-DateTime/<member>.nvs` and `Time-TimeOfDay/<member>.nvs`: each declares
  `calls 1` and `allocations 0`, measured with `nvs run --count`; all eleven flat under
  `bun nv scaling --growth` and `--iterations --sized`. `Time-DateTime/month.nvs` runs 256 iterations.
- `proofs-fields` is green (`looper check --name proofs-fields`), and `bun nv proofs --comments`
  passes on the new files. The only proof still owed is perf, which no new member has recorded yet.
- The sweep's reds are stale: `env-secret-proofs` names no check, and nightly run 44510687786 is at
  `adc7487822`, before the fix commits `4c82a2409a`, `eb5c02e983` and `656f5b2f2d`.

### Next

Stage 5, a close: this goal is not the chain's full gate. Run `cargo build`, `cargo clippy
--all-targets -- -D warnings`, and this goal's checks by name (`looper check --name rust-date`,
`table`, `case-date`, `rust-fields`, `case-datetime`, `case-time-of-day`, `proofs-date`,
`proofs-fields`). Then `bun nv owners --closes date-parts`, move any decision only this file holds
into `crates/nvs-stdlib/src/time.rs`'s module doc or a rule, and claim `DONE`. Expected red until
the next full gate: `bun nv proofs --gate` for the perf of all eighteen new members, the stale perf
records of every `Core\Time` member from the `time.rs` edits, and `website/public/llms-full.txt`,
which may need `bun nv render --website` for the new cards.

### Backlog

- Two subagents found `==` false for two `TimeOfDay` values (and two `DateTime` values) with the
  same fields, which looks like identity comparison on a value class; and `DateTime` has no
  `compareTo`, so `<` does not compile on it. Both are outside this goal; a later goal decides them.
