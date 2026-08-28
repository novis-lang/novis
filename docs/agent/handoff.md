# Handoff

## State

**M4's Stage 8, depth.** The tree is at **806 conformance plus 189 differential**. Nothing
is blocked.

One **availability bug** was found and closed this session, and it is worth knowing about
because the shape recurs: `Core\Bytes::repeat` and `Core\Str::repeat` bounded their loop by
the caller's count rather than by the result's size, so an empty subject walked past both
size checks and spun — 74 seconds for a count of 2e9, unbounded at `uint`'s maximum. Both
short-circuit now (`crates/nvs-stdlib/src/bytes.rs:@nvs_core_bytes_repeat` and
`str.rs:@nvs_core_str_repeat` own the reasoning in their doc comments), and the playbook
bullet under *Writing Novis itself* names the shape. The remaining count-shaped producers
have **not** been audited for it — that is the next group's first item.

The `Core\Bytes` floor the last handoff named turned out to be mostly already asserted:
`at`'s whole item was in `bytes-reads-name-the-octet-they-stop-at.nvst` and the
`startsWith`/`endsWith` agreement item was in
`bytes-the-three-predicates-are-what-compare-and-index-of-already-say.nvst`, which sweeps
all 64 ordered pairs of a table. Neither was rewritten. What landed is `repeat`'s half,
which was the one genuinely open item in that group.

The gap two handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two
`Uri`s is `E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**The rest of the count-shaped producers, and the pad members' own degenerate arguments** —
one file set: `crates/nvs-stdlib/src/str.rs`, `crates/nvs-stdlib/src/arr.rs` and
`tests/conformance/core/`. `count-shaped-producers-refuse-alike.nvst` already names the
eight members and pins their two refusal sentences; what none of them is asked is whether
the *work* is bounded by the result rather than by the count.

- [ ] **Audit the other six count-shaped producers for the loop-bound hole this session
      closed** (`str.rs:2031` `padStart`, `str.rs:2047` `padEnd`, `str.rs:2068`
      `padding_run`, `str.rs:2106` `write_run`, `arr.rs:1974` `padStart`, `arr.rs:1991`
      `padEnd`) — one slice. The question at each is whether any loop's iteration count
      comes from the caller's `uint` rather than from the size already reserved. `write_run`
      divides by the padding's piece count, so an empty padding is the row to check first.
      Fix what is holed and say in the case comment which ones were already clean.
- [ ] **`Core\Str::padStart`/`padEnd`'s degenerate padding** (`str.rs:2068`) — one case. An
      empty padding, a padding wider than the gap it fills, and a target at or below the
      subject's own length, named together, with whichever of those is a throw pinned by its
      message.
- [ ] **`Core\Arr::padStart`/`padEnd` agree with their `Core\Str` twins on the degenerate
      rows** (`arr.rs:1974`) — one case, the *agreement* shape: the same three degenerate
      targets asked of both pairs, asserting that the four members answer alike rather than
      what each answered.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `Core\Uri::compareTo` exists
  while `$a < $b` over two `Uri`s is `E0411` — ADR 0013 owns the rule.
- `gaps.py --coverage`'s ranking is by case count, which is a proxy for depth; the classes
  it names at 3.0 (`Core\Encoding`, `Core\Test`, `Core\Random`, `Core\Debug`) each have
  several multi-claim cases already, so read the bodies before believing the rank.
- `docs/agent/guard-name-debt.md` — 54 of 156 guard names match nothing `cargo test` runs.
- ADR 0007 § 2's `array<T> as array<U>` conversion does not lower, which is what keeps
  `Core\Csv::format`'s non-`string` cell refusal unreachable from source.
