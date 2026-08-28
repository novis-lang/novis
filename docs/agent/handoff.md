# Handoff

## State

**M4's Stage 8, depth.** The tree is at **850 conformance plus 189 differential**, all green.
Nothing is blocked.

Two cases landed, in two different classes, because the group the previous handoff named
turned out to be **already answered**: `gaps.py` ranks `Core\Bytes::contains`, `endsWith` and
`indexOf` as its thinnest three, but the *agreement* shape they were to be given is on disk
twice already — `bytes-contains-agrees-with-index-of-over-every-needle.nvst` and
`bytes-the-three-predicates-are-what-compare-and-index-of-already-say.nvst` — and the
needle-length bound, the empty needle and the binary octets (NUL and high-bit alike) are held
by `bytes-both-predicates-hold-the-same-bound-on-the-needles-length.nvst` and the sniffing
case. Reading the eight `--TEST--` lines first is what found this; it is the cheapest move
against a class `gaps.py` ranks by count.

What was genuinely unasked in that class was `indexOf`'s **`from` option**, which the whole
corpus reached with two spot values (`{from: 5}` and `{from: -5}`, one subject, one needle):

- **`from` resolves where `slice`'s offset resolves.** Both read ADR 0063 R8's sign rule
  through the same `offset` helper (`crates/nvs-stdlib/src/bytes.rs:367`), so the case sweeps
  from below `-length` to past `length` and asserts, by counting, that the **empty needle**'s
  answer — which is the resolved position itself, `find` answering `Some(0)` — equals
  `length($s) - length(slice($s, $f))`, and that every needle's answer is the tail's answer
  shifted by that same position. Nothing in the case re-implements the clamp it checks.

`Core\ObjectSet`'s algebra was the second: its existing cases ask the degenerate ends (a set
against itself, against a disjoint one) and that a result is new rather than a mutated operand,
so a *partial* overlap — the case the three members exist for — was untouched. 25 pairs from
one pool of eight identities, 16 laws each (inclusion-exclusion, both operands' two halves, the
three-piece tiling, the two symmetries and `diff`'s asymmetry) plus membership decided over the
whole domain rather than over the members present. A `union` that appended both walks fails the
first law and nothing in the degenerate cases.

The playbook's new bullet is the trap that cost the second slice a rewrite: a set returned by
`union`/`intersect`/`diff` has no writable type, so every derived set is a chain. That is a
real hole and it is in the backlog.

`orient.py`'s pack was complete for this group; nothing outside it was read.

## Next group

**`Core\Validate`** — 8 cases over 6 members, the smallest class in `gaps.py`'s table, floor 3
at depth 4.0. One shared file set: `crates/nvs-stdlib/src/validate.rs` plus
`tests/conformance/core/`. Read the eight existing `--TEST--` lines first (the group above is
why): the length limits, the LDH label rule, the `atext` local part and the `{version}` option
are each already held by a case of their own.

- [ ] **`Core\Validate::isIp`** (3 cases) — row `validate.rs:177`, helper `validate.rs:420`.
      *A bound on both sides*: what the existing case asks is the `{version}` option refusing
      the other family's spelling, not the numbers — so the last dotted-quad octet accepted
      (`255`) beside the first refused (`256`), the leading-zero form, the group count and the
      one `::` an IPv6 address may hold beside the second, each named with its neighbour.
- [ ] **`Core\Validate::isAscii`** (4 cases) — row `validate.rs:191`, helper `validate.rs:450`.
      *Agreement*: it and `isPrintable` cross twice already; what no case asks is whether it
      agrees with `Core\Encoding::isValidText` over a byte sweep, which is the other member
      that decides what a `string` may hold.
- [ ] **`Core\Validate::isDomain`** (4 cases) — row `validate.rs:170`, helper `validate.rs:402`.
      *Edges*: the trailing root dot, a single label with no dot at all, and the interaction
      between the label-length limit and the total-length one — the two limits' case pins each
      alone.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `Core\Uri::compareTo` exists
  while `$a < $b` over two `Uri`s is `E0411` — docs/agent/loop-goal.md, deserves its own session.
- `Core\ObjectSet::union`/`intersect`/`diff` return an un-parameterized `Core\ObjectSet`, which
  no declared type accepts (`crates/nvs-stdlib/src/objset.rs:89`) — a derived set cannot be
  bound or passed to a user function. Same shape for `Core\ObjectMap` if its rows match.
- `Core\Csv::format`'s "column N is not a `string`" refusal is unreachable from source until
  ADR 0007 § 2's `array<T> as array<U>` lowers — playbook, § *Writing a test case*.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  docs/agent/guard-name-debt.md.
- `Core\Uri` is now the thinnest class in `gaps.py` (compareTo 3, decodeFormValue 3, path 3).
