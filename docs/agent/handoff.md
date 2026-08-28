# Handoff

## State

**M4's Stage 8, and `Core\Math` is closed at 1.00** (38 cases over 38 members). The tree is
at **762 conformance plus 189 differential**. `gaps.py` still reports 0 members no case
calls and ranks the differential gap 0; `Core\Uri` and `Core\Time\DateTime` (1.00) are now
the thinnest classes.

- **The whole group's premise was stale, not just the item's.** All three named slices were
  already on disk verbatim — see the playbook bullet this session added, which is the
  cheap check that finds this in one call. What was left for `Core\Math` was a claim of a
  different shape, and that is what landed:
  `math-the-int-members-reach-both-ends-of-int-and-the-float-members-stop-at-2-53.nvst`.
- **`Core\Math` splits into two families and they stop in different places.** An
  `int`-parameter member (`sign`, `abs`, `intDiv`, `gcd`, `lcm`) takes the whole range and
  refuses only where its own *answer* leaves `int` — `abs($min)`, `gcd($min, 0)` and
  `gcd($min, $min)` (both answering `$min`'s magnitude), `lcm($min, 1)`. A `float`-parameter
  member (`floor`, `ceil`, `truncate`, `mod`, `hypot`) refuses at ADR 0007 § 2's 2^53,
  1024× short of either end, because the `int` argument reaches it only through the one
  implicit widening. Counted over five members × six magnitudes so a member that grew a
  conversion of its own fails.
- **The same widening rule raises two different classes at its two sites, and the case says
  so rather than freezing it.** At a `Core` member's declared `float` parameter it is
  `nvs_runtime::helpers::does_not_fit`'s `Fault::thrown`, promoted to spec § 10's
  `RuntimeError`; at a `callable`'s parameter it is `closure.rs:488`'s
  `Fault::thrown_as(Arithmetic, …)`, which
  `arr-a-callback-float-parameter-widens-an-int-and-stops-at-2-53.nvst` catches as
  `ArithmeticError`. `does_not_fit`'s own doc comment (`helpers.rs:1243`) already owns the
  gap; the new case asserts the message and a `Throwable`, not the class.

## Next group

**`Core\Test`'s own boundaries** — the file set is `crates/nvs-stdlib/src/test.rs` plus
`tests/conformance/core/`. Verified new this session: nothing under `tests/conformance/`
names `assertEqualsDeep`'s depth cap, and `gaps.py --errors` still lists it as one of only
two unasserted `thrown` sites (the other, `csv.rs:512`, the playbook says is owed no case).

- [ ] **`assertEqualsDeep` refuses past 64 levels of containment rather than comparing half
      a graph** (`crates/nvs-stdlib/src/test.rs:685` `difference`, `:691` the throw,
      `:706` `MAX_DEPTH = 64`) — the *bound asserted on both sides* shape: a graph 64 deep
      compares and a graph 65 deep throws with the message naming `$actual` plus the path.
      An array cannot contain itself (ADR 0090 § 3), so the subject is an object graph built
      by a loop; check the throw is `Fault::thrown` at the site before assuming a `catch`
      reaches it (it is).
- [ ] **A failing `assertEqualsDeep` names the path to the first difference, not the whole
      subject** (`test.rs:685`–`:705`, `array_difference`/`object_difference`) — the same
      `$actual{path}` rendering the depth refusal quotes, asserted where the difference is
      nested rather than at the top level.
- [ ] **`Core\Test::assertSame` and `assertEqualsDeep` agree on every row where identity and
      deep equality coincide and part exactly at a copy** (`test.rs` registry block) — the
      *agreement* shape, counted over a swept table.

## Backlog

- `Core\Uri` and `Core\Time\DateTime` are the next thinnest at 1.00 — check for the existing
  case first (this session's playbook bullet).
- `crates/nvs-stdlib/src/csv.rs:512`'s `thrown` is unreachable from source; `gaps.py --errors`
  will keep listing it (playbook, *Writing a test case*).
- 65 `Fault::fatal` sites remain unasserted and most are internal invariants no program
  reaches — `docs/agent/loop-goal.md` § *Standing decisions*.
- 54 of 156 guard tests named in `loop-goal.toml` match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
