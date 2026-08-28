# Handoff

## State

**M4's Stage 8 is where the work is, and the differential gap is closed**: `python
tools/gaps.py --differential` ranks **0** members with a PHP twin and no oracle case, the
three `Core\Time` clocks having landed this session. The tree is at **743 conformance plus
189 differential**, so the frontier is the conformance floor of 750 and, past coverage,
depth — `python tools/gaps.py` is what ranks it and `python tools/loop.py --list` still
reports no named `.nvst` case owed by any stage.

- **None of the three clock members has a value a case can pin**, so each case asserts a
  *property* both implementations hold, computed on each side from that side's own clock:
  `now` is a bracket (a fixed past instant, a fixed future one, the three scales of one
  reading agreeing) plus the ISO round trip; `monotonic` is an ordering plus "this is not
  a wall clock", the two implementations not even sharing an origin; `sleep` is the *cost*
  measured on the monotonic clock, at least what was asked for and under a ceiling.
- **One divergence was found and is not decided**: `Core\Time::sleep` returns at once for
  a negative `Duration` where all three of PHP's functions raise a `ValueError`. The sleep
  case pins the property both sides do hold — a negative request costs no time — and names
  the difference in its own prose rather than pinning it; it is in `## Backlog`.
- The thinnest class by depth is now **`Core\Time\DateTime` at 0.71** (12 cases over 17
  members), which is what the next group takes.

## Next group

**`Core\Time\DateTime`'s calendar arithmetic**, the thinnest class `python tools/gaps.py`
ranks. The file set is `crates/nvs-stdlib/src/time.rs` for the signatures and
`tests/conformance/core/` for the cases — the same signatures this session read, so the
anchors below are already resolved. § 4's opening rule is what every slice is about: a
`DateTime` moves by a count of a `Unit`, a calendar step a DST boundary or a short month
can lengthen, where an `Instant` moves by an exact `Duration`.

- [ ] **`plus`/`minus` are not `Duration` arithmetic** (`crates/nvs-stdlib/src/time.rs:918`,
      `:925`) — the *agreement* shape: the same step asked of a `DateTime` and of its
      `toInstant()` must **differ** across a DST boundary and across a short month
      (2024-01-31 plus one month, 2024-03-10 in `America/New_York` plus one day), and
      agree everywhere else, counted over a sweep rather than read off a line.
- [ ] **`startOf`/`endOf` over every `Core\Unit`** (`time.rs:960`, `:967`) — the *bound
      asserted on both sides*: the last instant each keeps beside the first it does not,
      for each unit, plus the invariant that `startOf` is never after its subject and
      `endOf` never before it.
- [ ] **`next`/`previous` over every `Core\Weekday`** (`time.rs:932`, `:939`) — the edge
      the member is written around is a subject that is *already* that weekday: seven
      rows each way from one fixed date, asserting each answer is strictly after (before)
      the subject and lands on the named weekday, counted.

## Backlog

- A `Core` enum case is not assignable to a binding of its own type — `Core\Charset $c =
  Core\Charset::Ascii;` is `E0401`, and so is `array<Core\Charset> $s = [...]` and
  `Core\Unit $u = Core\Unit::Month;`, while the same shape over a user `enum` compiles.
  Every `Core` enum argument must therefore be written inline at its call site. Owner:
  `nvs_types::expr::is_assignable` against ADR 0047 § 5's literal/case types.
- `Core\Time::sleep` returns at once for a negative `Duration` where `sleep`, `usleep` and
  `time_nanosleep` all raise a `ValueError`. Priority 2 says PHP-compatible observable
  behaviour, so the likely answer is a throw; owner is `docs/spec/01-core-library.md` § 4,
  and the divergence is named in the new sleep case's prose.
- A `require` whose path is not a string literal runs nothing at all, silently, in both
  forms — `nvs_hir::requires`' own known gap.
- ADR 0033's container axis: a `secret` value behind an `array<T>` element or an ADR 0036
  shape field carries no bit — `nvs_stdlib::debug`'s known gap 1.
- `nvs_stdlib::test`'s known gap 1: a non-`Comparable` object under `assertEquals` throws
  where ADR 0079 § 4 writes a compile error.
- `signatures.rs`' known gap: a class constant's declared type is unmodelled, so
  `Class::TOKEN` infers `mixed` at every expression site.
