# Handoff

## State

**`Core\Time\TimeOfDay` is off the conformance floor.** Two depth cases landed over
`crates/nvs-stdlib/src/time.rs`, and `Core\Time\TimeOfDay::compareTo` is deleted from
`crates/nvs-stdlib/tests/conformance_coverage.rs`'s `BELOW_THE_FLOOR`, whose whole remaining content
is now `Core\Router::urlAbsolute` — unreachable until that class splits, per the goal's standing
decision. The corpus is at **929**, all passing.

**The Stage 8 acceptance check still fails at 929 of 950 wanted.** A growth floor, not a regression:
no case fails and no check names a test that disappeared. It closes as the suite grows.

**What the two cases pin**, both in `tests/conformance/core/`:

- `time-of-day-compare-to-is-one-total-order-and-arr-sort-agrees-with-it.nvst` — the order's laws
  counted over a 12-row table rather than read off a row: trichotomy and antisymmetry over 144 pairs,
  `0` exactly where two readings render alike (14 pairs — the diagonal plus the one duplicated
  reading, so a member comparing only human-readable fields fails on the three nanosecond-only pairs),
  and transitivity over all 1,728 triples with its antecedents counted beside it (378 weak chains, 210
  strict) so the implication is not satisfied vacuously. The agreement half: `Core\Arr::sort` has no
  natural order for an object, so it is handed a comparator reading `format` alone — never `compareTo`
  — and `compareTo`'s sign is then the sign of the two values' positions in the sequence that sort
  produced, on all 144 pairs. Sorting by `compareTo` instead, from the table and from its reverse,
  reproduces that same sequence.
- `time-of-day-plus-and-minus-undo-each-other-and-refuse-at-two-boundaries.nvst` — 8 times × 6 units ×
  10 counts undone in each direction (480 each way), the unit ladder (one hour reached six ways, 40
  agreements each direction), and the pair's two boundaries: `Unit::Hour` moves where `Unit::Day` and
  the four larger cases throw a `LogicError` with one sentence (10 refusals, `plus` and `minus`
  alike), and a count past a span's ±175,307,616 hours is a `RuntimeError` naming the range.

**Found, not fixed: `plus`/`minus` stop being modular long before they refuse.**
`Core\Time\TimeOfDay::at(12, 0)->plus(24000000, Core\Unit::Hour)` is a whole number of days and
answers `14:07:11`; 2,400,000 hours still answers noon, so the break is between them and roughly at
the point where the count's nanoseconds pass `i64::MAX` (2,562,047 hours). `time.rs:2917`'s
`wrapping_add` is the site. The case asserts the modular identity at 2,400,000 and the refusal at
175,307,617 and deliberately freezes nothing in between — a pinned wrong answer is a permanent one.
Spec § 4 promises a total modular step with no such gap, so this is a correctness defect (priority 2),
not a documented bound.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`),
a `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have playbook
bullets under *Writing a test case*.

**Orientation gaps.** `[context] modules` still owes `crates/nvs-stdlib/src/hash.rs`, `src/test.rs`,
`crates/nvs-types/src/links.rs`, `src/routes.rs`, `crates/nvs-stdlib/src/validate.rs`,
`docs/spec/01-core-library.md`, `docs/spec/02-php-migration.md`, `tools/check-migration.py`, the
stage-7 comment header's per-goal floor table, a selector printing the *failing* check's own `cases`
block, and a `[context] anchors` entry for `registry.rs`'s `UNCLASSIFIED`. This session needed
`crates/nvs-stdlib/src/ordering.rs` (one peek, to learn that `Core\Arr::sort` refuses an object
outright) — add it, since every agreement case over a `Comparable` `Core` class will want it.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/time.rs` and `tests/conformance/core/`. `Core\Time\Date`
and `Core\Time\Instant` are the two remaining § 4 classes whose depth is carried by one wide case
each; `gaps.py --coverage` ranks them.

- [ ] **The dial's two ends, named together** (`crates/nvs-stdlib/src/time.rs:2895` and `:2836`) — the
      first and last representable `TimeOfDay`, what `plus`/`minus` do at each, what `compareTo`
      answers between them, and that `at` refuses the reading one past either end. The two new cases
      above use both ends as table rows but never ask what is on the far side of them.
- [ ] **`TimeOfDay::at` refuses each field on both sides** (`crates/nvs-stdlib/src/time.rs:2924`,
      `clock_from_parts` at `:2874`) — hour 23/24, minute 59/60, second 59/60, nanos 999999999/10^9,
      each bound with its accepted neighbour, and the `out_of_range` sentence naming the reading it
      refused. `field_at` is where a non-`uint` stops, which is a signature question, not a case.
- [ ] **Fix the non-modular step, or record it as a bound** (`crates/nvs-stdlib/src/time.rs:2917`) —
      the defect in `## State`. If jiff's `wrapping_add` cannot carry the count, reducing it modulo a
      day before building the span makes the step total *and* modular at every count the span accepts;
      that is a `time.rs` change with a case, not an ADR.

## Backlog

- `Core\Router::urlAbsolute` is the last `BELOW_THE_FLOOR` line — blocked on the class split
  (`docs/agent/loop-goal.md` § *Standing decisions*).
- 10 `UNCLASSIFIED` members owe ADR 0088's classification (`crates/nvs-stdlib/src/registry.rs:1526`).
- A `bytes` array key ICEs in `nvs-ir` (`docs/agent/playbook.md`, *Writing a test case*).
- `catch (Core\Error $e)` panics rather than diagnosing (`docs/agent/playbook.md`, same section).
- The corpus needs 950 cases for Stage 8; it stands at 929.
