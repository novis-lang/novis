# Handoff

## State

**`Core\Time\DateTime` is finished to depth and is no longer the thinnest § 4 class.** Three cases
landed against it and `python tools/gaps.py --coverage` now reads it at depth 6.0 / floor 4
(`weekday 4, isLeapYear 5, next 5`); a fourth case took `Core\Time::fromEpoch` and
`Core\Time\Instant`'s three epoch readings in the same file. **No `nvs-stdlib` source changed** —
every member the four cases reach already answered correctly, which is what the sweeps say.

**The corpus is at 938**, all passing. The Stage 8 acceptance check wants 950 and still fails — a
growth floor, not a regression: no case fails and no check names a test that disappeared. It has
moved 934 → 938 this session and needs roughly three more sessions of the same shape.

**Three facts the new cases pin and nothing else records.** (1) A leap year has three independent
readings — the `isLeapYear` flag, whether `Core\Time\Date::at($y, 2, 29)` is accepted, and whether the
ordinal count shifts — and they agree on all fourteen rows of a table carrying both halves of the
century rule in **both signs** (year 0 and -400 are leap where -100 is not). (2) The interval a `Date`
spans is wider at both ends than a `DateTime`'s, which now has a playbook bullet. (3) The three epoch
readings are one number at three resolutions and every one of them truncates **toward zero**, so below
the epoch the coarser reading is the *later* point and the seconds scale covers two whole seconds
around 0 — `Core\Time::fromEpoch(-1, {nanos: 500000000})->toEpochSeconds()` is `0`.

**What the four new cases pin**, all in `tests/conformance/core/`:

- `time-datetime-leap-year-is-februarys-last-day-and-the-years-length.nvst` — the three-way agreement
  above, counted, with the leap/common split beside it so an all-`false` agreement is not vacuous, plus
  both ends of the two ranges and the flag read on eight days of two years.
- `time-datetime-day-of-year-is-one-increasing-run-per-year.nvst` — four years walked a day at a time
  (366/365/366/365 steps each): the ordinal gains exactly one per step, is its own position in the
  walk, agrees with the `D` pattern letter and with `date()->format("D")` on every day, and restarts at
  1 on the next January. Plus the month table, where two months share an ordinal and ten are one apart.
- `time-datetime-date-and-time-of-day-are-the-two-halves-that-reassemble.nvst` — over eight moments in
  four zones the two halves partition the rendering, each is invariant under every change to the other,
  and both reassembly spellings (through `Core\Time::parse`, and through
  `startOf(Day)->withTime(...)`) answer the moment they came from; one instant in two zones has two
  pairs of halves.
- `time-epoch-is-one-number-at-three-resolutions-truncated-toward-zero.nvst` — the three scales checked
  against each other rather than against written figures, the half-second walk across the epoch, and
  `fromEpoch`'s two bounds on both sides (`nanos` below 1e9, seconds within the representable range).

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`),
a `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have
playbook bullets under *Writing a test case*.

**Orientation.** The pack was complete for this group; nothing outside it was read. The manifest list
the last three sessions asked for is still owed — `hash.rs`, `test.rs`, `nvs-types/src/links.rs`,
`routes.rs`, `validate.rs`, `docs/spec/01-core-library.md`, `docs/spec/02-php-migration.md`,
`tools/check-migration.py`, the stage-7 comment header's per-goal floor table, a selector printing
the failing check's own `cases` block, and a `[context] anchors` entry for `registry.rs`'s
`UNCLASSIFIED`. Add `crates/nvs-stdlib/src/math.rs` to `[context] modules` for the next group.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/math.rs` and `tests/conformance/core/`. `Core\Math` is now
one of the thinnest classes `gaps.py --coverage` names (depth 5.0 / floor 3 over 38 members,
`atan2 3, hypot 3, lcm 3`), and its three thinnest members are all *agreement* or *bound* shaped.

- [ ] **`lcm` and `gcd` are one pair, and the pair is a law rather than a table**
      (`nvs_core_math_lcm` at `crates/nvs-stdlib/src/math.rs:972`, `nvs_core_math_gcd` at `:957`) —
      over a table of pairs, `gcd(a,b) * lcm(a,b) == |a*b|`, both divide/are divided as they claim, the
      sign convention is asserted on negatives in both slots, and the zero row is named on both sides.
- [ ] **`hypot` is the distance no intermediate square overflows**
      (`nvs_core_math_hypot` at `crates/nvs-stdlib/src/math.rs:988`) — agreement with
      `Core\Math::sqrt(a*a + b*b)` where both spellings are representable, and the magnitudes where
      only `hypot` still answers, which is the whole reason the member exists.
- [ ] **`atan2` names the quadrant that `atan` cannot**
      (`nvs_core_math_atan2` at `crates/nvs-stdlib/src/math.rs:999`) — the four quadrants and the four
      axes counted as one sweep, agreement with `atan` where the quotient is defined, and both signed
      zeros.

## Backlog

- The corpus growth floor: 938 of the 950 the Stage 8 acceptance check wants — `docs/agent/loop-goal.toml`.
- `Core\Arr`'s floor-3 members (`column`, `fillKeys`, `firstKey`) — `python tools/gaps.py --coverage`.
- Item 12's 10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526` — ADR 0088 § 2.
- A `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — `docs/agent/playbook.md`.
- The `[context]` manifest additions listed in § State — `docs/agent/loop-goal.toml`.
