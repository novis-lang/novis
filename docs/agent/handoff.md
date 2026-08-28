# Handoff

## State

**M4's Stage 8, and `Core\Time\Duration` is closed**: both slices were test cases over landed
work, so the only `crates/nvs-stdlib/src` change is a doc comment, and the class went 0.95 →
**1.05** (20 cases over 19 members). The tree is at **761 conformance plus 189 differential**.
`gaps.py` still reports 0 members no case calls and ranks the differential gap 0; `Core\Math`
(0.97) is now the thinnest class and `Core\Uri`/`Core\Time\DateTime` (1.00) the next.

- **The arithmetic is bounded at both ends of the `i64` nanosecond count**, and the bottom value
  is reachable from source as `$max->negated()->minus(1ns)` — which is already the asymmetry the
  case is written around. `plus` and `minus` are each refused on *both* sides, `multipliedBy`
  stops at the product rather than the operand, and `negated` fails at exactly one value in the
  whole type while being an involution everywhere else. Eight refusals, counted, all one sentence
  differing only in the member each names. `compareTo` is the member the ends cannot reach: it has
  an answer at the very pair `minus` must refuse.
- **ADR 0070 § 5's round trip was over-broad and its body now says so.** `toString` is total and
  `minus`/`negated` produce negative durations, but the grammar has no sign, so
  `parse((90m)->negated()->toString())` throws *"`-` has no meaning in a duration"*. Widening the
  grammar would put a spelling in `nvs.toml` and the lexer that § 4 refuses in source, so the
  refusal is the decision and both halves are pinned by
  `time-duration-round-trips-through-parse-exactly-where-the-grammar-has-a-spelling.nvst`.

**The item's premise was stale**: `orient.py` named "`Duration`'s one uncased member", and
`gaps.py --coverage --json` reports `uncalled: []` for every class — depth per member is the whole
frontier now, so an item saying "the member no case calls" should say "the claim no case makes".

## Next group

**`Core\Math` is the thinnest class left** (0.97, 37 cases over 38 members) and the file set is
`crates/nvs-stdlib/src/math.rs` plus `tests/conformance/core/`. The registry block runs
`math.rs:54`–`math.rs:400`; all three slices share it.

- [ ] **`Core\Math`'s integer members at the ends of `int`** (`math.rs:54` `abs`, `math.rs:61`
      `sign`, `math.rs:117` `intDiv`, `math.rs:124` `mod`) — spec § 3. The *bound asserted on both
      sides* shape: `abs(INT_MIN)` has no answer for the same reason `Duration::negated` has none
      at its bottom, and division by zero is ADR 0007 § 4's `ArithmeticError` rather than a value.
      Check each site's `Fault::` constructor before assuming a `catch` reaches it.
- [ ] **`clamp`, `min` and `max` agree on one ordering** (`math.rs:68`, `math.rs:75`,
      `math.rs:82`) — the *agreement* shape: `clamp($x, $lo, $hi)` asked of a sweep must equal
      `max($lo, min($x, $hi))` on every row, asserted by counting, and an inverted `$lo > $hi` is
      the boundary the member is written around.
- [ ] **`toBase`/`fromBase` round-trip at the ends of their radix range** (`math.rs:299`,
      `math.rs:306`) — the same shape this session's second slice used on `parse`/`toString`: the
      last accepted radix and the first refused one, named together, and every rendering handed
      straight back.

## Backlog

- 68 unasserted `Fault` sites remain, 65 of them `fatal` — `gaps.py --errors`, judged one at a time.
- An algebra result is an unparameterized `Core\ObjectSet`, so `union`/`intersect`/`diff` cannot be
  bound or passed — a `nvs_stdlib::registry` fix, not a test one.
- `csv.rs:512`'s `thrown` is unreachable from source and is owed no case — playbook, *Divergences*.
- 54 of `loop-goal.toml`'s 156 guard-test names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `array<T> as array<U>` panics `nvs-ir` at `crates/nvs-ir/src/lower/expr.rs:877` — playbook,
  *Writing a test case*.
