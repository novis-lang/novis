# Handoff

## State

**M4's Stage 8, and `Core\Time\DateTime` is closed**: both of the session's slices were
pure test cases over landed work, so no `crates/nvs-stdlib/src` line changed, and the
class went 0.88 → **1.00** (17 cases over 17 members). The tree is at **757 conformance
plus 189 differential**. `python tools/loop.py --list` still reports no named `.nvst`
case owed by any stage, and `gaps.py --differential` still ranks 0.

- **`with` bounds each of its seven components separately, and every refusal names the
  one it refused.** `month` is 1..=12 at both ends, `day` has a static 1..=31 *and* a
  bound from the month the same call is building (February 29th refused against this
  receiver's 2023, accepted one option wider at 2024), the clock fields are zero-based so
  their first refused value is their count, `nanos` stops at 999999999, `year` at ±9999,
  and a value too wide for the field's own storage takes `time.rs:2238`'s earlier refusal
  — which still names the component. `withTime` is the half that cannot be partial: a
  `TimeOfDay` carries all four clock fields, so the receiver's nanos are replaced even
  where nothing named them, and its bounds are `TimeOfDay::at`'s, checked a member earlier
  and naming *that* member. Berlin's spring-forward gap is not a bound in either
  spelling — both shift forward by it, which is the agreement
  `nvs_core_time_datetime_with_time`'s doc states.
- **`startOf`/`endOf` are one agreement over all eleven `Core\Unit` granularities**, and
  `next`/`previous` one over all seven `Core\Weekday` cases, both asserted by counting.
  `startOf(g)->plus(1, g)` is `endOf(g)` plus one nanosecond and `startOf(g) <= $d <=
  endOf(g)`, over two receivers — an ordinary Berlin day and its 23-hour spring-forward
  one, which is where an extent computed on a civil field rather than taken from the zone
  would fall out. Both receivers agree 11/11.

**`granularity.rs` is not the time-unit table** — it is `Core\Str`'s grapheme/code-point
unit. `Core\Unit`'s cases are `time.rs:807` (`UNIT`), and the previous handoff pointed at
the wrong file.

## Next group

**`Core\ObjectSet` is the thinnest class left** (0.89, 8 cases over 9 members) and the
file set is `crates/nvs-stdlib/src/objset.rs` plus `tests/conformance/core/`. Its rows are
all in one block, `objset.rs:30-110`, so the first two slices share everything; the third
is `Core\Time\Duration` (0.95) and a different file.

- [ ] **`Core\ObjectSet`'s algebra at its degenerate ends** (`objset.rs:86` `union`,
      `objset.rs:93` `intersect`, `objset.rs:100` `diff`) — spec § 13. The identities and
      the annihilators, counted rather than read off a line: union with the empty set and
      with itself, intersect with itself and with a disjoint set, `diff` from itself and
      from the empty set. Identity is what membership means here (ADR 0090 § 4), so two
      equal-looking instances are two elements — assert that too.
- [ ] **`add`/`remove`/`has`/`count`/`clear` are one invariant, not five members**
      (`objset.rs:51` `add`, `objset.rs:58` `has`, `objset.rs:65` `remove`,
      `objset.rs:72` `count`, `objset.rs:79` `isEmpty`, `objset.rs:107` `clear`). Adding a
      member twice leaves the count alone, removing one that is absent is not an error,
      `isEmpty` and `count() == 0` never disagree, and `clear` is `remove` over every
      element. One sweep, one set of counters.
- [ ] **`Core\Time\Duration`'s one uncased member** (`crates/nvs-stdlib/src/time.rs`,
      `DURATION`'s block) — 0.95, 18 cases over 19 members. `python tools/gaps.py --class
      'Core\Time\Duration'` names it; a different file set from the two above.

## Backlog

- A `Core` enum cannot be swept from an `array<Core\Unit>` or passed through a
  user-declared `Core\Unit` parameter — `E0401`, rendered *"expected `Core\Unit`, found
  `Core\Unit`"*. Two bugs in one: the restriction itself, and a diagnostic that prints
  both sides identically. `docs/agent/playbook.md` § *Writing a test case* has the
  workaround; the fix belongs to `nvs_types`.
- 54 of the 156 guard tests `loop-goal.toml` names still match nothing `cargo test` would
  run — `docs/agent/guard-name-debt.md`.
- `Core\Math` (0.97, 37/38) and `Core\Uri` are the classes just above the frontier.
