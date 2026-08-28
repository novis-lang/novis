# Handoff

## State

**M4's Stage 8, depth.** The tree is at **848 conformance plus 189 differential**, all green.
Nothing is blocked.

`Core\Random` was `gaps.py`'s thinnest class and is now floor 3 at depth 4.0 (13 cases over 7
members); it is no longer the one to take. Three members gained a case each, all in
`tests/conformance/core/`:

- **`float`'s `[0, 1)` is asserted at both ends.** The member takes no argument, so its whole
  contract is the interval, and membership alone — what the sibling sweep already counted — is
  held by a generator drawing from any sub-interval. What is counted instead is *reach*: ten
  tenths of the unit interval, 400 draws each, every one of them non-empty, so both end tenths
  are named together with the strict `< 1.0` and the inclusive `>= 0.0`. A member clamped to
  `[0, 1]` fails the ceiling line alone and nothing else.
- **`token` is `bytes` in the same hex, and the two renderers are asked one question.** `token`
  writes its own hex loop rather than calling `Core\Encoding::toHex`, which is where two
  renderings of one rule drift; the case counts *agreement* over seven counts — same width, the
  entropy count doubled rather than read as characters, the same lower-case alphabet — plus the
  round trip back through `Core\Encoding::fromHex` and that the default and the explicit `32`
  agree on width.
- **`bytes` draws over the whole octet range**, which no existing case asks. Length, distinctness
  and the oversized refusal are all held by a generator restricted to printable ASCII — the shape
  a `string`-backed implementation drifts into — while answering with a quarter of the promised
  entropy. 6000 *single-octet* draws reach all 256 values (nothing straddles a hex boundary when
  the draw is one byte), NUL among them, with the high bit set about half the time.

`orient.py`'s pack was complete for this group; nothing outside it was read.

The gap twenty handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s
is `E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**`Core\Bytes`**, now tied for `gaps.py`'s thinnest — floor 3, depth 4.0, 26 cases over 13
members, and its three thinnest members share one rule, which is what makes them one group. One
shared file set: `crates/nvs-stdlib/src/bytes.rs` plus `tests/conformance/core/`. Read the
existing cases' `--TEST--` lines first: a member of this family is pinned by what it answers at
the *empty needle* and at the ends, and those are the rows most likely already taken.

- [ ] **`Core\Bytes::contains`** (3 cases) — row `bytes.rs:204`, helper `bytes.rs:608`.
      *Agreement*: `contains`, `indexOf`, `startsWith` and `endsWith` all answer one question
      about one substring, so a table of (haystack, needle) pairs asserting they **agree** —
      `contains` iff `indexOf` is non-null, `startsWith` iff `indexOf` is 0, `endsWith` iff the
      match sits at `length - needle` — catches a member that grew its own search while every
      one of them still looks right on its own line.
- [ ] **`Core\Bytes::endsWith`** (3 cases) — row `bytes.rs:218`, helper `bytes.rs:629`. *A bound
      asserted on both sides*: the last accepted needle is the whole haystack itself, the first
      refused one is a needle one octet longer, and the empty needle is the degenerate end — all
      three named together rather than the middle row alone.
- [ ] **`Core\Bytes::indexOf`** (3 cases) — row `bytes.rs:186`, helper `bytes.rs:559`. It takes a
      third argument the other two do not (an offset), so its own gap is *edges*: an offset past
      the end, an offset landing exactly on a match, and whether a match straddling the offset is
      found — the row a `contains`-plus-`slice` implementation gets wrong.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `Comparable` over two `Core\Uri`s
  is `E0411` — `docs/agent/loop-goal.md`, and it wants a session of its own.
- `Core\Uri` is the other floor-3 class (19 members): `compareTo`, `decodeFormValue`, `path`.
- `Core\ObjectSet` (`clear`, `diff`, `intersect`) and `Core\Test` (`assertEquals`, `assertNull`,
  `assertThrows`) are the next two floor-3 groups after that — `python tools/gaps.py`.
- 68 unasserted error paths remain, 65 of them `Fault::fatal` — `python tools/gaps.py --errors`,
  and the playbook's rule about judging a site's parameter types before taking it.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` would run —
  `docs/agent/guard-name-debt.md`.
