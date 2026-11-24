---
looper: 1
title: "The Core\\Time values compare by value, and Date, Duration and DateFormat fill their gaps"
summary: "Reached when == and ObjectMap/ObjectSet compare Date, TimeOfDay, Instant, Duration, Zone and DateTime by value (a DateTime by its moment, which also orders it), Date, TimeOfDay and DateTime write ISO text with toIso, a DateTime's text carries its offset and zone name and DateTime::fromIso reads them back, a Y pattern without w warns, Date has startOf, endOf, difference, next, previous, parse and range, Date and DateTime read daysInMonth, quarter, isoWeek and isoWeekYear, Duration reads and writes clock text, Novis\\Intl\\DateFormat takes a fields option, and the rules, the reference, the feature proofs and the website say so."
context: [crates/nvs-stdlib/src/time.rs, docs/rules/core-classes/a-time-class-builds-its-own-values.md]
stages:
  - id: 1
    title: "== compares the Core\\Time values by value, a DateTime orders by its moment, and ObjectMap and ObjectSet key them by value"
    status: done
    context: [crates/nvs-runtime/src/identity.rs, crates/nvs-runtime/src/object.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/objmap.rs, crates/nvs-stdlib/src/identity_store.rs, crates/nvs-ir/src/lower/operator.rs, crates/nvs-types/src/expr/operators.rs, docs/rules/expressions/object-identity-equality.md, docs/rules/expressions/equality-semantics.md]
  - id: 2
    title: "Date, TimeOfDay and DateTime write ISO text, a DateTime's with its offset and zone, and a Y pattern without w warns"
    status: done
    context: [crates/nvs-stdlib/src/cldr.rs, crates/nvs-types/src/intrinsics.rs, crates/nvs-diagnostics/src/code/warnings.rs, docs/rules/classes/stringable.md]
  - id: 3
    title: "Date moves to the start and end of a unit, counts a difference and finds a weekday, and Date and DateTime read four more calendar parts"
    status: done
  - id: 4
    title: "Date::parse, Date::range, Duration::toClock and Duration::fromClock"
    status: done
    context: [crates/nvs-syntax/src/duration.rs, docs/rules/types/duration.md, docs/rules/core-api/no-mode-strings.md]
  - id: 5
    title: "Novis\\Intl\\DateFormat takes a fields option, and every enum case of the component is documented"
    status: open
    context: [extensions/intl/src/dates.rs, extensions/intl/nvs/DateFormat.nvs, extensions/intl/manifest.json, wit/intl.wit, docs/rules/core-classes/intl-roster.md]
  - id: 6
    title: "the rules hold every decision and every decline, and stages 1 to 3 have their feature proofs"
    status: open
  - id: 7
    title: "stages 4 and 5 have their feature proofs, and the website says what changed"
    status: open
    context: [website/src/content/docs/syntax/operators/comparison.mdx, website/src/data/changes.json]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: value-equality
    stage: 1
    name: "== and != compare the six Core\\Time value classes by value on a typed, a nullable, a mixed and an array operand, a DateTime by its moment, and every other class still by identity"
    run: [cargo, test, --lib, time_value_]
    tests: [time_value_equal_dates_are_equal, time_value_equal_durations_are_equal, time_value_written_durations_are_equal, time_value_equal_times_and_instants_are_equal, time_value_zones_with_one_id_are_equal, time_value_datetimes_at_one_moment_in_two_zones_are_equal, time_value_mixed_operands_compare_by_value, time_value_arrays_of_dates_compare_by_value, time_value_a_user_class_still_compares_by_identity]
    timeout: 1800
  - id: datetime-order
    stage: 1
    name: "two DateTime values order by their moment, to the nanosecond, whatever their zones"
    run: [cargo, test, --lib, datetime_orders_]
    tests: [datetime_orders_by_its_moment_across_zones, datetime_orders_to_the_nanosecond]
    timeout: 1800
  - id: map-by-value
    stage: 1
    name: "ObjectMap and ObjectSet find an equal Core\\Time value built elsewhere, and still find every other object by identity"
    run: [cargo, test, --lib, a_map_keyed_by_]
    tests: [a_map_keyed_by_a_date_finds_an_equal_date, a_map_keyed_by_a_datetime_finds_the_same_moment_in_another_zone, a_map_keyed_by_a_user_object_finds_only_that_object, a_map_keyed_by_a_set_finds_an_equal_duration]
    timeout: 1800
  - id: iso-text
    stage: 2
    name: "toIso writes the text fromIso reads, for Date, TimeOfDay and DateTime, at both ends of the range"
    run: [cargo, test, --lib, to_iso_]
    tests: [to_iso_date_round_trips_through_from_iso, to_iso_time_of_day_round_trips_through_from_iso, to_iso_datetime_writes_its_offset_and_zone, to_iso_round_trips_at_both_ends_of_the_range]
    timeout: 1800
  - id: iso-zone
    stage: 2
    name: "DateTime::fromIso reads a zone written in the text, picks a repeated hour by its offset, refuses an offset the zone does not have at that time, and needs $zone only for a text without a zone"
    run: [cargo, test, --lib, from_iso_datetime_]
    tests: [from_iso_datetime_reads_the_zone_in_the_text, from_iso_datetime_picks_a_repeated_hour_by_its_offset, from_iso_datetime_refuses_an_offset_the_zone_does_not_have, from_iso_datetime_without_a_zone_needs_the_argument, from_iso_datetime_refuses_two_different_zones, from_iso_datetime_round_trips_to_iso_across_a_change_of_offset]
    timeout: 1800
  - id: week-year-warning
    stage: 2
    name: "a written pattern with Y and no w warns and names y, and one with w does not"
    run: [cargo, test, --lib, a_week_based_year_]
    tests: [a_week_based_year_without_a_week_warns, a_week_based_year_with_a_week_does_not_warn]
    timeout: 1800
  - id: date-calendar
    stage: 3
    name: "Date's startOf, endOf, difference, next and previous match DateTime's on the same day, and the four new readers match the CLDR week rules"
    run: [cargo, test, --lib, date_calendar_]
    tests: [date_calendar_start_and_end_match_datetime, date_calendar_difference_matches_datetime, date_calendar_next_and_previous_match_datetime, date_calendar_days_in_month_covers_leap_years, date_calendar_quarter_of_every_month, date_calendar_iso_week_and_year_around_new_year]
    timeout: 1800
  - id: parse-range-clock
    stage: 4
    name: "Date::parse reads date letters only, Date::range lists both ends in either direction, and toClock and fromClock round-trip negative and long durations"
    run: [cargo, test, --lib, time_gaps_]
    tests: [time_gaps_date_parse_reads_a_written_pattern, time_gaps_date_parse_refuses_a_time_letter, time_gaps_date_range_includes_both_ends, time_gaps_date_range_counts_down, time_gaps_date_range_steps_by_month, time_gaps_to_clock_writes_hours_past_a_day, time_gaps_clock_text_keeps_its_sign, time_gaps_from_clock_refuses_a_minute_past_59]
    timeout: 1800
  - id: intl-fields
    stage: 5
    name: "every DateFormat::Fields case formats in every locale of the built-in data, on dates and on dateTimes"
    run: [cargo, test, --lib, date_format_fields_]
    tests: [date_format_fields_every_case_formats_in_every_locale, date_format_fields_year_month_in_de, date_format_fields_weekday_in_de, date_format_fields_on_date_times]
    timeout: 1800
  - id: proofs
    stage: 7
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, time-values]
    timeout: 1800
  - id: close
    stage: 8
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, time-values]
    timeout: 900
---

# The Core\Time values compare by value, and Date, Duration and DateFormat fill their gaps

## Why

`==` on two `Core\Time` values compares identity. `Date::at(2026, 1, 5) == Date::at(2026, 1, 5)` is
`false`, and so is `5m == 5m`, while `<=` and `>=` both hold. The trap is silent, and it follows the
values into `Core\ObjectMap` and `Core\ObjectSet`, which key by identity
(`crates/nvs-stdlib/src/objmap.rs:639`), so a map from a day to its hours misses every day built a
second time. Today the decision is recorded at `crates/nvs-stdlib/src/time.rs:131` and in
`rule:expressions/object-identity-equality`. That rule's reasons are an unbounded walk over the object
graph and a meaning chosen per class. Neither holds for a closed list of `Core` values of one to three
fixed slots, which the language fixes and no class author can join.

The same review of the time classes found gaps that every program fills by hand:

- `Date`, `TimeOfDay` and `DateTime` read ISO text with `fromIso` and write none. Every program writes
  `format('yyyy-MM-dd')`, and `YYYY`, the week-based year, compiles: `Date::at(2024, 12, 30)` formats
  as `2025`.
- `DateTime` has `startOf`, `endOf`, `difference`, `next` and `previous`, and `Date` has none of them.
- No class reads the days in a month, the quarter or the ISO week.
- `DateTime::parse` exists, and `Date` has no way to read a written pattern without a zone.
- `Core\Arr::range` lists integers only, and a calendar or a report lists days or months.
- `Duration::toString` writes `1d1h30m`, and a time sheet writes `25:30`.
- `Novis\Intl\DateFormat` formats year, month and day only. A month heading, a weekday name or a long
  date with its weekday has no locale-aware form, while the pinned ICU4X 2.4 supports ten field sets
  and the guest uses one (`extensions/intl/src/dates.rs:181`).

Many programs do each of these many times. The user agreed to the design below on 2026-11-24.

## Standing decisions

**The user's calls (2026-11-24).**

- **`==` and `!=` compare six `Core\Time` classes by value**: `Date`, `TimeOfDay`, `Instant`,
  `Duration`, `Zone` and `DateTime`. Every other class, user or `Core`, stays identity.
- **A `DateTime` compares and orders by its moment**, to the nanosecond, and its zone does not count.
  10:00 in `Europe/Berlin` and 09:00 in `Europe/London` on one day are `==`. `DateTime` gains
  `compareTo`, so `<`, `<=`, `>` and `>=` compile on it. Two `==` values may print different text,
  because their zones differ. This reverses the decision at `time.rs:131`.
- **`ObjectMap` and `ObjectSet` find a key with `==`**, so an equal `Core\Time` value is the same key,
  and the same moment in two zones is one key.
- **`DateFormat::Fields`** has ten cases, each named by its parts from the largest to the smallest:
  `Day`, `MonthDay`, `YearMonthDay` (the default), `Weekday`, `DayWeekday`, `MonthDayWeekday`,
  `YearMonthDayWeekday`, `Month`, `YearMonth` and `Year`. The name says which parts appear, never their
  order. Each case's card says so: "Shows the month and the year. The locale decides their order, such
  as `Januar 2026` in `de`."
- **A `DateTime`'s ISO text carries its offset and its zone name**, as RFC 9557 writes it:
  `2026-01-05T08:30:00+01:00[Europe/Vienna]`, the local time, the offset and the zone. The offset
  picks between the two copies of a repeated hour, so the text names one moment. `DateTime::toIso()`
  writes this full form. `DateTime::fromIso(string $text, ?Zone $zone = null)` reads both forms: a
  text with `[Zone]` needs no second argument, and a text without one still needs `$zone`. An offset
  that does not match the zone at that local time throws `ParseError`. The text without a zone stays
  available through `format(...)`. This replaces the earlier plan of a `toIso()` without an offset,
  and the goal `route-values` builds on it: a `DateTime` crosses a query string and JSON as this text.
  User's call of 2026-11-24.
- **Everything else in the list below**, which the user agreed to as one plan.

**Calls the stage 1 session made (2026-11-24).**

- The value slots live in one roster, `crates/nvs-stdlib/src/time.rs` `VALUE_SLOTS`, and not as a
  field on each of the 198 `CoreClass` literals. `crate::instance` copies each list onto the runtime
  descriptor (`ClassDesc::value_slots`), as it already does for `HOST_HANDLE_CLASSES` and
  `SLOT_TYPES`, and `registry::compares_by_value` answers the checker. Simplicity: one table, beside
  the slot constants it names.
- A typed `==` takes the helper when either operand's static type *may* hold a value class: the class
  itself, `object`, `Comparable`, `Stringable` or `Parses`, or a union or intersection with one of
  them. Two `Comparable` values that are both `Date`s would otherwise compare by address on the typed
  path and by value on the `mixed` path, which is two answers to one `==`. The checker records the
  site in `ExprTypeTable::compares_by_value`, a span set beside the entries, because a `match` arm's
  span already holds its expression's entry. A map or set keyed by such a type takes the chain keying
  for the same reason. **What it spends:** one helper call in place of one pointer compare on those
  pairs only. Two user objects typed `Comparable` pay it too.
- The goal's tests are lib tests at two levels, because no lib test in the tree runs a whole program:
  `nvs-stdlib`'s `time.rs` tests compare built instances through `value_identical`, `value_hash`,
  `compareTo` and the collection helpers, and `nvs-ir`'s `lower/tests.rs` checks what a typed `==` and
  a `match` arm lower to. The conformance case of the whole program is stage 6's.

**Calls the session made and the user did not change.**

- The reader is `daysInMonth(): uint`, not `lengthOfMonth`: it is the name most libraries use.
- `Date::range(Date $from, Date $to, ?{ step?: Unit } $options = null): array<Date>` copies
  `Core\Arr::range`'s option name and its direction rule: both ends are included, and a `$from` after
  `$to` counts down. The step is one of `Day`, `Week`, `Month`, `Quarter` or `Year`, as a closed subset
  in the signature (`rule:types/single-value-types`). A month step from the 31st lands on each month's
  last day, as `Date::plus` does.
- How the zoned text above is read and written, in detail. `toIso()` always writes seconds and
  always writes the offset as `+HH:MM` or `-HH:MM`, so UTC is `+00:00[UTC]` and never `Z`. A fraction
  of a second is written only when it is not zero, with as few digits as keep it exact. `fromIso`
  takes the offset as optional before `[Zone]`, and without it the local time resolves in the zone as
  a text without an offset resolves today. A `Z` still throws `ParseError` naming
  `Instant::fromIso`. A text with `[Zone]` and a `$zone` naming another zone throws `ParseError`
  (`rule:errors/ambiguous-input-refused`); the same zone in both is accepted. Only one bracket is
  read: a critical flag `[!...]` and a tag such as `[u-ca=...]` throw `ParseError`. A zone name is
  looked up as `Zone` looks it up, ignoring case. A written literal is checked while compiling by the
  `DateTime` row of `crates/nvs-types/src/intrinsics.rs:397`, its zone name and its offset included,
  and a literal with no `[Zone]` and no second argument does not compile.
- `TimeOfDay::toIso()` always writes seconds, `08:30:00`.
- `Date` does not become `Stringable`. E0710's help names `toIso()` and `format(...)` for a `Date`,
  `TimeOfDay` or `DateTime`.
- The week-year warning fires on a written pattern with a `Y` and no `w`, and names `y`. A pattern
  assembled at run time is not judged. It is a new warning code in
  `crates/nvs-diagnostics/src/code/warnings.rs`, the next free number after `W1037`.
- `Duration::toClock(?{ seconds?: bool } $options = null): string` writes hours and minutes, with
  hours past 24 and at least one hour digit: `25:30`, `0:05`, `-1:15`. With `seconds: true` it writes
  `25:30:00`. A part below a second is dropped. `Duration::fromClock(string $text): Duration` reads
  `H:MM` or `H:MM:SS` with an optional `-` and any number of hour digits, and throws `ParseError` on a
  minute or second past 59.
- `Date::parse(string $text, string $pattern): Date` reads date letters only. A literal pattern is a
  prepared `CldrPattern` row checked while compiling, as `DateTime::parse`'s is
  (`crates/nvs-types/src/intrinsics.rs:236`), and a time or zone letter in it does not compile.
- `isoWeek(): uint` and `isoWeekYear(): int` follow ISO 8601: Monday first, and week 1 holds the
  year's first Thursday. `quarter(): uint` is 1 to 4. All four readers are on `Date` and `DateTime`.
- `Date::startOf` and `endOf` take `Week`, `Month`, `Quarter` and `Year`, and a week starts on Monday
  as `DateTime::startOf(Unit::Week)` does. `Date::difference(Date, Unit)` takes `Day`, `Week`, `Month`,
  `Quarter` and `Year` and counts whole units as `DateTime::difference` does. A unit below a day does
  not compile on a `Date`.

**Calls the stage 2 session made (2026-11-24).**

- An offset with seconds, which only a zone's local mean time before about 1900 has, is written
  `+HH:MM:SS` in `DateTime::toIso`'s text, as `render_offset` already writes it for a fixed zone's
  id. `+HH:MM` would name another moment, and `fromIso` would then throw on the text `toIso` wrote.
  So `fromIso` reads `+HH:MM` and `+HH:MM:SS` before `[Zone]`.
- A fixed-offset zone's name in the brackets is its id, `+05:30`, as `DateTime::zone()` returns it:
  `2026-01-05T14:00:00+05:30[+05:30]`. RFC 9557 allows an offset in the brackets.

**Declined, each written into a rule's Declined list in stage 6.**

| Proposal | Reason | Rule |
|---|---|---|
| `Date::today()`, `DateTime::now()` and a configured default zone | `now` stays on `Core\Time`, which reads the world and is faked in tests. A second way to one value was already declined for `atStartOfDay`. A hidden default zone gives a server a different day than its user's. `Core\Time::now()->in('Europe/Berlin')->date()` is the short form | `rule:core-classes/a-time-class-builds-its-own-values` |
| `Date` as `Stringable` | It is a second way to the ISO text, and a page that shows a date usually wants the locale's text. E0710 names both | `rule:classes/stringable` |
| A date as an array key | An array key stays `int` or `string`. `ObjectMap` keys by value now | `rule:core-classes/a-time-class-builds-its-own-values` |
| A `DateRange` value | `$from <= $d && $d <= $to` already says "contains", and two such tests say "overlaps" | `rule:core-classes/a-time-class-builds-its-own-values` |
| CLDR skeleton strings such as `'yMMMM'` | A skeleton is a fifth grammar, and the four are a closed list. The `Fields` enum is checked and completed | `rule:core-api/no-mode-strings` |
| A `Full` length | ICU4X 2.4 has `Long`, `Medium` and `Short` only. `YearMonthDayWeekday` with `Long` is the long form with the weekday | `rule:core-classes/intl-roster` |

**How `==` reaches the slots (stage 1).** One table says which slots a value class compares, and every
reader of the equality table reads it: the typed path, the `mixed` path and the hash.

- `Date` compares year, month and day. `TimeOfDay` compares hour, minute, second and nanos. `Instant`
  compares seconds and nanos. `Duration` compares its nanoseconds. `Zone` compares its id text.
  `DateTime` compares seconds and nanos, which are its moment, and skips its zone slot.
- The mark lives on the class's `Core` descriptor (`crates/nvs-stdlib/src/registry.rs:1952`) as its
  list of value slots, and the runtime's equality rows read it from the instance's header. The session
  follows the road `crate::instance`'s descriptor renderer takes to reach a `Core` class from a header.
  Fallback, if `nvs-runtime` cannot reach the `Core` descriptor: a hook `nvs-stdlib` installs at start,
  as other stdlib hooks are installed, with a comment naming the twin.
- A typed pair whose static type names one of the six classes lowers to one helper call in place of the
  inline pointer compare (`crates/nvs-ir/src/lower/operator.rs:1115` shows the shape of a helper arm).
  A `mixed` or union pair reaches the same comparison through `shallow_identical`
  (`crates/nvs-runtime/src/identity.rs:210`), and `value_hash` (`identity.rs:298`) feeds the same slots.
  An array that contains them compares them by value, because it reaches the same row.
- Every other reader of the equality table follows without its own change, and the session lists them
  first with an `Explore` subagent: `Core\Arr` members that test membership or uniqueness, `match`, and
  `ObjectSet`.

**What it spends.** `==` on one of these classes reads one to four slots of each operand in place of one
pointer compare, a few nanoseconds per comparison. A `mixed` pair also reads the class mark. A map or
set keyed by them hashes those slots in place of the address. Nothing changes for any other class.
`Date::range` builds one `Date` object per day or step in the list, charged to the request, and a range
longer than the memory cap allows stops at the cap like any other array. Every other new member spends
one result object or one string per call. Each module doc that records a cost says so.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is recorded in the handoff with what failed, and the stage goes on. A count, never a timing,
decides whether a cost changed.

**Lanes.** Stage 1 is the keystone for the rules of stage 6 and changes `crates/nvs-runtime/`,
`crates/nvs-ir/` and the registry. Stages 2, 3 and 4 all add members to `time.rs` and run in order.
Stage 5 touches only `extensions/intl/`, `wit/intl.wit` and its own tests, and may run beside stages 2
to 4. Stages 6 and 7 run after the code stages they prove.

## Stages

### 1. == compares the Core\Time values by value, a DateTime orders by its moment, and ObjectMap and ObjectSet key them by value

**Split:** one `Explore` subagent lists every reader of the equality table and of `value_hash` before
the first edit, as `file.rs:NN` and one line each.

- [x] `crates/nvs-stdlib/src/registry.rs:1952`: `CoreClass` gains its list of value slots, empty for
  every class but the six; the rows set in `crates/nvs-stdlib/src/time.rs:716` (`Duration`), `:1206`
  (`Zone`), `:1494` (`Instant`), `:2442` (`DateTime`), `:3123` (`Date`) and `:3556` (`TimeOfDay`).
- [x] `crates/nvs-runtime/src/identity.rs:210` and `:298`: the object arm of `shallow_identical` and
  `hash_shallow` compare and hash a value class by its slots; the module doc at `identity.rs:8` says so.
- [x] `crates/nvs-ir/src/lower/operator.rs:1115` and `crates/nvs-types/src/expr/operators.rs:926`: a
  typed `==` or `!=` whose operands name one value class lowers to the helper; a nullable operand keeps
  its null test first.
- [x] `crates/nvs-stdlib/src/time.rs:2078`: `DateTime::compareTo(DateTime $other): int` by seconds then
  nanos, registered beside the other `compareTo` rows (`time.rs:413`); the decision at `time.rs:131` is
  rewritten to say what is true now. `report_comparable_diagnostic`
  (`crates/nvs-types/src/expr/operators.rs:1014`) loses its `DateTime` branch naming `toInstant()` or
  `date()`, and `tests/conformance/reject/ordering-two-built-in-values-with-no-order-names-what-to-compare.nvst:1`
  moves to a built-in class that still has no order.
- [x] `crates/nvs-stdlib/src/objmap.rs:639` and `crates/nvs-stdlib/src/identity_store.rs:184`: `same`
  and the hash read the equality row; `ObjectMap`'s and `ObjectSet`'s cards and module docs say "a key
  is found with `==`" (`crates/nvs-stdlib/src/instance.rs:247`'s `BY_IDENTITY` reason rewritten).
  - [x] A map or set whose key type may hold a value class takes the chain keying, which already
    reads `value_identical` and `value_hash` (`crates/nvs-types/src/expr/calls.rs` `keying_of`).
  - [x] `crates/nvs-stdlib/src/identity_store.rs:1`'s *An object key is its identity* section and its
    keying table say a value-class key takes the chain.
  - [x] `ObjectMap`'s and `ObjectSet`'s cards and module docs (`objmap.rs:1`, `objset.rs:1`) say a key
    is found with `==`, and `instance.rs:247`'s `BY_IDENTITY` reason is rewritten.
- [x] The tests the checks `value-equality`, `datetime-order` and `map-by-value` name, beside the code
  they test, each run once by name.
- [x] `docs/rules/expressions/object-identity-equality.md:1` and
  `docs/rules/expressions/equality-semantics.md:12`: the class-instance row gains the six value classes,
  with the reason the old decline does not reach them and the cost.
  `docs/rules/core-classes/a-time-class-builds-its-own-values.md:1`: the written-zone paragraph's "so
  `==` stays identity" and the Declined item "One shared instance per zone name" rewritten, because two
  written zones with one id are now `==`.
  - [x] `equality-semantics.md`'s class-instance row and its closing paragraph.
  - [x] `object-identity-equality.md`: the six classes, why the decline of a property walk does not
    reach them, and the cost.
  - [x] `a-time-class-builds-its-own-values.md`: the written-zone paragraph and the Declined item.
  - [x] `bun nv brief --where equality` and `--where compareTo`: every other rule that says `==` on an
    object is identity, or that a `DateTime` has no order, is rewritten.

### 2. Date, TimeOfDay and DateTime write ISO text, a DateTime's with its offset and zone, and a Y pattern without w warns

**Split:** none, one session.

- [x] `crates/nvs-stdlib/src/time.rs:2898` (`Date`), `:3335` (`TimeOfDay`) and `:2078` (`DateTime`):
  `toIso(): string`, each writing the text its `fromIso` reads, beside `Instant::toIso`
  (`time.rs:4852`). `DateTime`'s writes the offset and `[Zone]`, as § *Standing decisions* says.
  - [x] `Date::toIso` and `TimeOfDay::toIso` (`date_iso_text`, `clock_iso_text`), with
    `to_iso_date_round_trips_through_from_iso` and `to_iso_time_of_day_round_trips_through_from_iso`.
  - [x] `DateTime::toIso` (`zoned_iso_text`), with `to_iso_datetime_writes_its_offset_and_zone`.
  - [x] `to_iso_round_trips_at_both_ends_of_the_range`, once `DateTime::fromIso` reads the zoned
    text (the next item).
- [x] `crates/nvs-stdlib/src/time.rs:4019` (`iso_datetime`) and `:3831` (`validate_iso`): the
  `DateTime` grammar reads an optional offset and one `[Zone]`, and checks the offset against the
  zone at that local time. `crates/nvs-stdlib/src/time.rs:1807`: `DateTime::fromIso`'s `$zone`
  becomes `?Zone $zone = null`. `crates/nvs-types/src/intrinsics.rs:397`: a literal's zone name and
  offset are checked while compiling, and a literal with no `[Zone]` and no second argument does not
  compile. The cards at `time.rs:2528` (`DateTime::fromIso`) and `time.rs:1524` (`Instant::fromIso`,
  whose text names `DateTime::fromIso` with a zone) say what is true now.
- [x] The tests the check `iso-zone` names, run once by name. A failing test is written first.
- [x] `crates/nvs-types/src/expr/operators.rs:3172`: E0710's help for the three classes names `toIso()`
  and `format(...)`.
- [x] `crates/nvs-types/src/intrinsics.rs:670` and `crates/nvs-stdlib/src/cldr.rs:392`: a written
  pattern with `Y` and no `w` reports the new warning; its card in
  `crates/nvs-diagnostics/src/code/warnings.rs:396`'s neighbourhood.
- [x] The tests the checks `iso-text` and `week-year-warning` name, each run once by name.
- [x] `docs/rules/core-classes/a-time-class-builds-its-own-values.md:1`: § *One text format per class*
  says each class writes the text it reads, and that a `DateTime`'s text carries its offset and zone
  (RFC 9557), with the reason: the offset names one moment in a repeated hour, and the zone lets the
  text cross a query string or a JSON document with no second argument.

### 3. Date moves to the start and end of a unit, counts a difference and finds a weekday, and Date and DateTime read four more calendar parts

**Split:** none, one session.

- [x] `crates/nvs-stdlib/src/time.rs:5103`: a calendar-only twin of `floored` over year, month and day,
  with a comment naming the zoned one; `Date::startOf` and `Date::endOf` over it.
- [x] `crates/nvs-stdlib/src/time.rs:5385`: `Date::difference(Date $other, Unit $unit): int`, counting
  as `DateTime::difference` counts.
- [x] `crates/nvs-stdlib/src/time.rs:5225`: `Date::next(Weekday)` and `Date::previous(Weekday)`.
- [x] `crates/nvs-stdlib/src/time.rs:5889`'s neighbourhood: `daysInMonth`, `quarter`, `isoWeek` and
  `isoWeekYear` on `Date`, and the same four on `DateTime` (`time.rs:2078`).
- [x] `Core\Unit`'s card says which units a `Date` takes.
- [x] The tests the check `date-calendar` names, run once by name.

### 4. Date::parse, Date::range, Duration::toClock and Duration::fromClock

**Split:** none, one session.

- [x] `crates/nvs-types/src/intrinsics.rs:236`: the prepared-pattern row for `Date::parse`, refusing a
  time or zone letter; `crates/nvs-stdlib/src/time.rs:2898`: the member, through `cldr.rs`'s parser
  (`crates/nvs-stdlib/src/cldr.rs:1389`).
- [x] `crates/nvs-stdlib/src/time.rs:5742`: `Date::range` over `date_stepped`, with the step subset in
  its signature.
- [x] `crates/nvs-stdlib/src/time.rs:1047`: `Duration::toClock` and `Duration::fromClock`, the reader
  in `crates/nvs-syntax/src/duration.rs:1` beside the duration grammar so one module owns duration
  text. `rule:types/duration` gains one paragraph on the clock form.
- [x] `docs/rules/core-classes/a-time-class-builds-its-own-values.md:1`: the constructor table gains
  `Date::parse` and `Date::range`.
- [x] The tests the check `parse-range-clock` names, run once by name.

### 5. Novis\Intl\DateFormat takes a fields option, and every enum case of the component is documented

**Split:** none, one session.

- [ ] `wit/intl.wit:87`, `:116` and `:204`: a `fields` enum and option on dates and date-times.
- [ ] `extensions/intl/src/dates.rs:78`, `:137` and `:181`: map each case to ICU4X 2.4's `DateFields`
  (`D`, `MD`, `YMD`, `E`, `DE`, `MDE`, `YMDE`, `M`, `YM`, `Y`) with the given length.
- [ ] `extensions/intl/manifest.json:144` and `extensions/intl/nvs/DateFormat.nvs:10` and `:31`: the
  `Fields` enum with one help text per case, and the option on `dates` and `dateTimes`. The `Length`
  help lists its three cases and says there is no longer one. The component is rebuilt by
  `crates/nvs-ext/build.rs:130`.
- [ ] The tests the check `intl-fields` names: the coverage test walks every case over every locale the
  built-in data has and fails naming a case and a locale that formats nothing. Run once by name.

### 6. the rules hold every decision and every decline, and stages 1 to 3 have their feature proofs

**Split:** after the session writes the rules, the proofs go to writing subagents in disjoint folders:
one for `docs/examples/core/Time-Date/`, one for `Time-DateTime/`, `Time-TimeOfDay/`, `Time-Instant/`,
`Time-Duration/` and `Time-Zone/`, and one for `docs/examples/core/ObjectMap/`,
`docs/examples/lang/expressions/comparison-and-equality/`, the attacks and the benches.

- [ ] The six declines of § *Standing decisions*, each in the Declined list of the rule the table names:
  `docs/rules/core-classes/a-time-class-builds-its-own-values.md:1`,
  `docs/rules/classes/stringable.md:1`, `docs/rules/core-api/no-mode-strings.md:14` and
  `docs/rules/core-classes/intl-roster.md:51`.
- [ ] `docs/rules/types/ordering.md:1` and `docs/rules/classes/comparable.md:1`: `DateTime` orders by
  its moment.
- [ ] `docs/rules/core-api/collections-drop-revoked-objects.md:1`: a key is found with `==`.
- [ ] The proofs `bun nv proofs --owed --goal time-values` names for stage 1 to 3's members, at the
  paths it prints: `about.md`, three examples, a bench and an attack per new member, and the changed
  examples of `==` and `ObjectMap`. Every `.nvs` and `about.md` follows AGENTS.md § *Text an end user
  reads*.

### 7. stages 4 and 5 have their feature proofs, and the website says what changed

**Split:** the examples of `Date::parse` and `Date::range` go to one writing subagent, those of
`Duration` to another, and those of `Novis\Intl\DateFormat` to a third; the session writes the website.

- [ ] The proofs `bun nv proofs --owed --goal time-values` names for stage 4 and 5's members, at the
  paths it prints. `docs/examples/novis/Intl-DateFormat/dates/` and `dateTimes/` gain a `fields`
  example each.
- [ ] `website/src/content/docs/syntax/operators/comparison.mdx:1`: `==` on the time values, and
  `<` on a `DateTime`. Reread and stamped with `bun nv site --stamp`.
- [ ] `website/src/data/changes.json:1`: a `changed` entry for `==` on the time values, for `ObjectMap`
  and `ObjectSet`, for `Novis\Intl\DateFormat` and for `Core\Time\DateTime::fromIso`, whose `$zone` is
  optional for a text with `[Zone]`; a `new` entry for each new member.
- [ ] `bun nv proofs --owed --goal time-values` names nothing.

### 8. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of docs/agent/prose.md § *A newcomer reads it* over the pages stages 6 and 7 touched.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 4 are done. Stage 4 is one commit, `1925501b9d`.

- `Date::parse(text, format)` is a prepared CLDR row: `Restriction::DateFields` in
  `crates/nvs-types/src/intrinsics.rs`, `cldr::validate_date` and `cldr::date_parse_fields_only`,
  and `(DATE_NAME, "parse")` on `PREPARED_MEMBERS`. A time or zone letter in a written pattern is
  `E0769`, and a computed one throws `LogicError`. `cldr::read` now splits into `read_fields`,
  shared with the new `cldr::read_date`, which builds a `civil::Date` with no zone.
- `Date::range(from, to, {step?})` takes `DATE_STEP_UNITS` with the default `Unit::Day`. Entry `k`
  is `from` plus `k` steps, so a month step from the 31st comes back to the 31st. The list is asked
  of the memory ceiling once, with `date_whole_units` (taken out of `Date::difference`) as the bound.
- `Duration::toClock({seconds?})` and `Duration::fromClock(text)` call
  `nvs_syntax::duration::render_clock` and `parse_clock`. A sign is written only when what is left
  is not zero, so `-30s` is `0:00` and never `-0:00`: a call this session made, recorded in
  `rule:types/duration`'s new clock paragraph.
- Both rules are updated: the constructor table and a `Date::parse`/`Date::range` paragraph, and
  the `W1038` sentence now names `Date::parse` too.
- The check `parse-range-clock` ran once by name and is green: eight tests. Build, clippy and
  `cargo fmt --check` are green. A smoke run of the debug binary printed the expected range, parse
  and clock text, and `nvs check` refused `HH:mm` at `Date::parse` and `Unit::Hour` as `step`.

### Next

Stage 5, one session, which touches only `extensions/intl/`, `wit/intl.wit` and its own tests. Its
items in the stage list carry the anchors: `wit/intl.wit:87`, `:116` and `:204`,
`extensions/intl/src/dates.rs:78`, `:137` and `:181`, `extensions/intl/manifest.json:144` and
`extensions/intl/nvs/DateFormat.nvs:10` and `:31`. The ten `Fields` cases and their card wording are
in § *Standing decisions*. Then the tests of check `intl-fields`.

Stage 6 also owes this: `rule:types/duration` still says "`90m == 1h30m` is `false`" in its literal
paragraph. Stage 1 made that `true`, so the sentence is rewritten there.

Stage 7 owes the proofs of the four stage 4 members: `Date::parse`, `Date::range`,
`Duration::toClock` and `Duration::fromClock`.

Tests that may go red in the gate: the `floored` refactor touched every `DateTime::startOf` and
`endOf` call for a day and larger, and those tests did not run in stage 3
(`datetime_start_of_*`, `datetime_end_of_*` in `time.rs`). A conformance case or example that prints
`==` on two equal `Core\Time` values and expected `false`. Grep `tests/conformance` and
`docs/examples` for `compareTo($` with `== 0` and for `Duration`/`Date` pairs under `==` when the gate
names one. A `.nvs` source with `format('Y…')` and no `w` now prints `W1038`. The `ObjectMap`,
`ObjectSet`, `DateTime::fromIso`, `Instant::fromIso` and `Core\Unit` card texts changed, so
`site --check stale` and `questions` will name their pages in stage 7. `nvs meta`'s snapshot, if a test
pins it, gains three `toIso` rows, `fromIso`'s `?Zone $zone = null`, eleven `Date` rows, four
`DateTime` rows and two `Duration` rows. A test that counts `PREPARED_MEMBERS` or the intrinsic
rows sees one more of each. Stage 6 owes `DateTime::fromIso` and `toIso` examples showing the zoned text, and
`docs/examples/core/Time-DateTime/fromIso/02-texts-with-an-offset.nvs` could show `[Europe/Berlin]`
as the fix. Stage 6 also owes the proofs of the thirteen new stage 3 members.

### Backlog
