# Handoff

## State

**M4's Stage 8, depth.** The tree is at **808 conformance plus 189 differential**. Nothing
is blocked.

The audit the last handoff asked for is **done, and the six named sites were clean** of the
loop-bound shape: every loop in `Core\Str::padStart`/`padEnd` (through `padding_run` and
`write_run`) and in `Core\Arr::padStart`/`padEnd` (through `append_copies`) writes at least
one byte or one entry per turn, over a size `nvs_runtime::affordable` already bounded, so
none is paced by the caller's `uint`. An empty padding is refused before the run is
computed, and `write_run`'s division by the piece count cannot see a zero, a non-empty
string being at least one grapheme.

What the audit *did* find is one crate down, and its playbook bullet under *Writing Novis
itself* owns the shape: `affordable` accepts up to `isize::MAX` and the string header is
prepended after it, so `Core\Str::repeat`/`padStart`/`padEnd` at exactly that count reached
`str_layout` and took the process with a FATAL.
`crates/nvs-runtime/src/string.rs:@try_str_layout` closes it for every `built_fallibly`
caller, and `count-shaped-producers-answer-the-last-count-the-seam-accepts.nvst` pins all
eight producers at that boundary and at one count past it.

Two things about the pad members were **already asserted** and were not rewritten: the
empty-padding pair is in `str-replace-and-pad-are-the-identity-at-their-own-bound.nvst`,
and uneven truncation is in `str-pad-members.nvst`. What was not asserted, and now is, is
that the run is a function of the shortfall alone — which is also how
`nvs_core_str_pad_start`'s doc comment was found wrong: it said the run is cut "at the end
nearest the subject" for *both* members, which is true of `padStart` only.

The gap three handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two
`Uri`s is `E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**The `Core\Arr` half of the same questions** — one file set: `crates/nvs-stdlib/src/arr.rs`
and `tests/conformance/core/`. Nothing here needs `str.rs` open.

- [ ] **`Core\Arr::padStart`/`padEnd` agree with their `Core\Str` twins on the degenerate
      rows** (`arr.rs:1974`, `arr.rs:1991`, shared helper `arr.rs:@padding`) — one case.
      The three the string pair now has pinned and the array pair does not: a `$size` at or
      below the subject's count is the identity *on the values* though not on the keys, the
      padded entries are all the one `$value` and never a copy of it (`append_borrowed`
      retains, `arr.rs:@append_borrowed`), and the shortfall is counted from the subject's
      own count so a map's keys never enter it.
- [ ] **`Core\Arr::fill`'s own degenerate count** (`arr.rs:@nvs_core_arr_fill`) — one case,
      or one block folded into the above. `fill(0, $v)` is the empty array, and it is the
      one count-shaped `Core\Arr` producer with no subject, so it is where the seam's bound
      is the count itself rather than the count plus what is already held.
- [ ] **Whether any other `built_fallibly` caller checks a size it does not then allocate**
      (`crates/nvs-stdlib/src/str.rs:@built_fallibly` and its call sites) — one slice. The
      hole closed this session was the *seam* being untotal; the second half is a member
      whose `affordable` argument and whose `built_fallibly` argument are different
      expressions, which is a catchable throw now but may still carry the wrong sentence.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `Core\Uri::compareTo` exists
  while `$a < $b` over two `Uri`s is `E0411` — its own session, `docs/agent/loop-goal.md`.
- `affordable` is not yet a budget; ADR 0004 settles that the `[limits.hard]` per-request
  ceiling attaches at that seam at M6 — `crates/nvs-runtime/src/abi.rs:@affordable`.
- `array<T> as array<U>` does not lower, which is what makes `Core\Csv::format`'s
  non-`string` cell unreachable — `docs/agent/playbook.md`, *Writing a test case*.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` would run —
  `docs/agent/guard-name-debt.md`.
