# Handoff

## State

**M4's Stage 8, depth.** The tree is at **840 conformance plus 189 differential**, all green.
Nothing is blocked.

`Core\Uri` was `gaps.py`'s next floor-2 class. Two of its three named members gained depth:

- **`tryParse` is total, and partitions the byte.** The existing agreement case sweeps 21
  hand-picked subjects; the new one sweeps **all 128 ASCII code points** in a path segment and
  counts — `parse` refuses 46 of them, `tryParse` throws for none, and the two agree at 128/128.
  The admitted set is printed as a string, which pins RFC 3986's `pchar` plus the two
  delimiters that *end* a path rather than sit in it (`?` and `#`), with `%`, `[` and `]`
  absent. A second case asserts the closure property `tryParse` is the predicate for: every
  URI the class can build — 11 bases × (`with` on each of six components + § 5's nine
  reference shapes), 97 survivors of 176 attempts — renders text `tryParse` reads back to the
  *same* text.
- **`buildQuery` renders a value by its own rule, not by `as string`.** Swept over 12 values
  and counted: 9 agree, 1 drops (a `null` pair has no wire spelling) and 2 diverge —
  `false` is `0` where `false as string` is `""`, and a nested array has no `as string` at
  all. Plus `scalar_text`'s throw, which is a `Fault::thrown` and so catchable: an object
  value is refused rather than written.

**The item's premise was stale, and the next session should expect this**: the handoff named
the *agreement* shape as unspent for `tryParse`, but two cases already spent it
(`uri-parse-and-try-parse-are-one-reader-asked-two-ways.nvst` and the § 12 reporting case).
`gaps.py`'s per-member count is a count, not a shape inventory — check what the existing cases
already assert before taking the shape a handoff names.

`orient.py`'s pack was complete for this group; nothing outside it was read.

The gap seventeen handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two
`Uri`s is `E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**`Core\Str`**, now `gaps.py`'s thinnest floor-2 class (depth 4.0, floor 2, 212 cases over 39
members). One shared file set: `crates/nvs-stdlib/src/str.rs` plus `tests/conformance/core/`.
Re-run `python tools/gaps.py` first, and read the existing cases' `--TEST--` lines before
picking a shape — that is what went wrong above.

- [ ] **`Core\Str::chunk`** (2 cases) — row `crates/nvs-stdlib/src/str.rs:235`, helper
      `crates/nvs-stdlib/src/str.rs:983`. A *bound asserted on both sides* is the likely gap:
      the last size it accepts and the first it refuses, and whether a chunk splits a
      multi-byte code point.
- [ ] **`Core\Str::fold`** (2 cases) — row `str.rs:382`, helper `str.rs:2214`. Read its doc
      comment for what folding is defined over; the *invariance over a sweep* shape
      (idempotence: `fold(fold($s)) == fold($s)` counted over a table) is unspent.
- [ ] **`Core\Str::fromCodePoints`** (2 cases) — row `str.rs:403`, helper `str.rs:2380`.
      `str-from-code-points-inverts-code-points.nvst` already holds the inverse; what is left
      is the two refused ranges its doc comment names at `str.rs:2304`.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `$a < $b` over two `Uri`s is
  `E0411` — docs/agent/loop-goal.md's item list owns it.
- `Core\Uri::compareTo` (3 cases) — row `uri.rs:506`, helper `uri.rs:1689`; the total-order
  half is already pinned, so what is left is agreement with `resolve`/`with`.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  docs/agent/guard-name-debt.md.
- 68 unasserted error paths, 65 of them `Fault::fatal` — `python tools/gaps.py --errors`.
- ADR 0028 § 2's abandoned-generator `finally` — docs/agent/loop-goal.md § *Standing decisions*.
