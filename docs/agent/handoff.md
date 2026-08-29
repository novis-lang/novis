# Handoff

## State

**`Core\Time\TimeOfDay` is finished to depth, and the non-modular step the last session found is
fixed.** `clock_stepped` reduces a count to one turn of the dial before `jiff` sees it
(`crates/nvs-stdlib/src/time.rs:2895`, `clock_cycle` beside it), because a span is totalled in `i64`
nanoseconds and every count past roughly 2,562,047 hours used to wrap *there* and answer something
plausible. The span built from the whole count is still built and thrown away: it is what refuses a
count no span can carry and what names the range, so the refusal at 175,307,617 hours is unchanged.
The step is now modular across the whole accepted range, pinned at the bound itself (175,307,616
hours) and at `i64::MAX` nanoseconds.

**The corpus is at 931**, all passing. The Stage 8 acceptance check wants 950 and still fails — a
growth floor, not a regression: no case fails and no check names a test that disappeared.

**What the two new cases pin**, both in `tests/conformance/core/`:

- `time-of-day-names-both-ends-of-the-dial-and-what-is-past-them.nvst` — the dial is closed and its
  two ends touch. Both ends named by rendering and in nanoseconds since midnight (0 and
  86,399,999,999,999); no reading lies outside them, over a 100-row sweep of the four fields'
  extremes, with the tie falling exactly once at each end; the far side of each end is the other,
  asserted as an identity between the two for all six units rather than as a rendering each; a whole
  turn is the identity from either end in either direction (12 each way); and the one place on the
  dial where the step wraps but the order does not — at the top a step forward answers *smaller*,
  counted beside noon, where it does not.
- `time-of-day-at-bounds-each-field-on-both-sides.nvst` — hour 23/24, minute 59/60, second 59/60 and
  nanoseconds 999,999,999/10⁹, each ceiling with its accepted neighbour, plus the fact that the four
  fields are refused *together*: `at(24, 60, {second: 60, nanos: 1000000000})` is one sentence naming
  the whole reading, and a field far past its ceiling is refused rather than taken modulo anything.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`),
a `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have
playbook bullets under *Writing a test case*.

**Orientation.** `[context] modules` gained `crates/nvs-stdlib/src/cldr.rs` and
`crates/nvs-stdlib/src/ordering.rs` this session, which the last one asked for and nothing had
applied; the rest of its list is still owed — `hash.rs`, `test.rs`, `nvs-types/src/links.rs`,
`routes.rs`, `validate.rs`, `docs/spec/01-core-library.md`, `docs/spec/02-php-migration.md`,
`tools/check-migration.py`, the stage-7 comment header's per-goal floor table, a selector printing
the failing check's own `cases` block, and a `[context] anchors` entry for `registry.rs`'s
`UNCLASSIFIED`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/time.rs` and `tests/conformance/core/`. `Core\Time\Date`
is the thinnest § 4 class left — `gaps.py --coverage` reads floor 3 over 6 members, `compareTo 3,
at 4, minus 5` — and the three slices below are the same three shapes that just finished
`TimeOfDay`, which is why they share a file set and a reading.

- [ ] **`Core\Time\Date::compareTo` is one total order, and `Core\Arr::sort` agrees with it**
      (`crates/nvs-stdlib/src/time.rs:2820`, `date_of` at `:2619`) — the laws counted over a table
      rather than read off a row, and the agreement half handed a comparator that reads `format`
      alone, never `compareTo`. `ordering.rs` is why: `Core\Arr::sort` has no natural order for an
      object. The `TimeOfDay` twin is the worked shape.
- [ ] **`Core\Time\Date::at` bounds each field on both sides** (`nvs_core_time_date_at` at
      `crates/nvs-stdlib/src/time.rs:2727`, `date_of` at `:2619`) — year, month 1/12 against 0 and
      13, and the day whose ceiling *moves*: 28/29 February by leap year, 30 against 31 by month.
      That last one is what makes this case different from the `TimeOfDay` twin, whose four ceilings
      are constants.
- [ ] **The calendar's two ends refuse where the clock's wrap** (`date_stepped` at
      `crates/nvs-stdlib/src/time.rs:2689`) — the first and last representable `Date`, and what
      `plus`/`minus` do one step past each. `a-date-at-either-end-of-its-year-range-still-formats`
      already covers rendering there; the step is not covered, and the contrast with
      `clock_stepped`'s wrap is the point — a date has somewhere for a carry to go until it does not.

## Backlog

- `Core\Time\DateTime` is the next thinnest after `Date` — `isLeapYear 3, dayOfYear 4, timeOfDay 4`
  (`python tools/gaps.py --coverage`).
- Item 12's 10 `UNCLASSIFIED` members, `crates/nvs-stdlib/src/registry.rs:1526` — ADR 0088 § 2.
- `Core\Router::urlAbsolute` is the whole of `BELOW_THE_FLOOR` and stays unreachable until the class
  splits, per the goal's standing decision.
- A `bytes` array key ICEs in `nvs-ir`; `catch (Core\Error $e)` panics — both have playbook bullets.
- The `[context]` fields still owed are listed in `## State`.
