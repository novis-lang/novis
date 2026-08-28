# Handoff

## State

**M4's Stage 8, and `Core\Uri` is closed**: its last unasked edge landed, so the class
is at depth 1.00 with every member cased and no boundary left that `gaps.py` names. The
tree is at **755 conformance plus 189 differential**. Both slices this session were
*pure test cases* — unlike the two before them, neither member disagreed with its own doc
comment, so no `crates/nvs-stdlib/src` line changed:

- **`with` already draws each component's own grammar, at both ends.** A scheme is
  RFC 3986 § 3.1's `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )` because `recompose`
  concatenates and `read` (`uri.rs:747`) decides, so the first byte is the half a
  hand-written rule would get wrong: `1x`, `+a`, `-a`, `.a`, `a_b` and the empty scheme
  are all refused while `a.`, `a+`, `a-` and `HTTP` are read.
  `uri-with-draws-the-scheme-grammar-and-tells-an-empty-host-from-none.nvst` counts 8
  accepted against 9 refused rather than reading a row, and pins the other half the item
  named: an **empty** host is a different authority from an **absent** one (`http:///p`
  against `http:/p`), each preserved under a `with` that does not mention the host, since
  `with` replaces and never removes.
- **`Core\Random::int`'s closed bound now reaches the ends of `int` itself.** The
  standing sweep asks for a die and for ±2^62; what none asked is the singleton at each
  extreme, the whole 2^64-value range, and the two-value ranges hard against each end —
  the places an implementation computing `$max - $min + 1` overflows.
  `random-int-s-closed-bound-reaches-the-ends-of-int-itself.nvst` names the first refused
  range at both extremes too, so the empty-range throw is pinned where the numbers in its
  message are the type's own.

`python tools/loop.py --list` still reports no named `.nvst` case owed by any stage, and
`gaps.py --differential` still ranks 0.

## Next group

**`Core\Time\DateTime` is the thinnest class left** (0.88, 15 cases over 17 members) and
the file set is `crates/nvs-stdlib/src/time.rs` plus `tests/conformance/core/`. Its rows
are all in one block, `time.rs:911-1023`, so the first two slices share everything. The
third is a different file and is only worth taking if the first two leave real room.

- [ ] **`with` and `withTime` have no *component* boundary case** (`time.rs:946` `with`,
      `time.rs:953` `withTime`) — spec § 13. This is the same shape this session just
      wrote for `Core\Uri::with`: each option has its own bound and none of them is
      asked about. A month at `0` and at `13`, a day at `29` against February in a
      non-leap year and in a leap one, an hour at `24`, a nanosecond at `1000000000` —
      the last accepted value and the first refused one named together, and the refusal
      saying which component rather than that the result is not a time.
- [ ] **`startOf`/`endOf` are an agreement, not two members** (`time.rs:960` `startOf`,
      `time.rs:967` `endOf`, with `next`/`previous` at `time.rs:932`/`939`) — spec § 13.
      One question asked of every granularity: `endOf(g)` is the last instant strictly
      inside the unit and `startOf(g)->plus(one g)` is the instant after it, so the two
      must **agree** for every granularity rather than each answering plausibly on its
      own row. `crates/nvs-stdlib/src/granularity.rs` is the table to sweep.
- [ ] **`Core\ObjectSet`'s algebra at its degenerate ends** (`objset.rs:86` `union`,
      `objset.rs:93` `intersect`, `objset.rs:100` `diff`) — 0.89, second-thinnest, and a
      *different* file set. Union with the empty set and with itself, intersect with
      itself and with a disjoint set, `diff` from itself — the identities and the
      annihilators, counted.

## Backlog

- `Core\Csv::format`'s "column N is not a `string`" (`csv.rs:512`) is the one unasserted
  `thrown` left and no program can reach it — it waits on ADR 0007 § 2's
  `array<T> as array<U>` conversion row, which panics `nvs-ir` today.
- 54 of the 156 guard names in `loop-goal.toml` match nothing `cargo test` runs;
  `docs/agent/guard-name-debt.md` is the list and the three causes.
- `Core\Time\Duration` (0.95) and `Core\Math` (0.97) are the next two after DateTime.
