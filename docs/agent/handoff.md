# Handoff

## State

**M4's Stage 8, depth.** The tree is at **843 conformance plus 189 differential**, all green.
Nothing is blocked.

`Core\Str` was `gaps.py`'s thinnest floor-2 class and is now at depth 5.0 (216 cases over 39
members); its floor-2 members are `lines`, `lowerFirst` and `normalize`. Three members gained a
case each, all in `tests/conformance/core/`:

- **`chunk`'s size is a bound named on both sides**, and four properties are counted over every
  size from 1 to two past the subject's length: the chunks re-join to the subject, there are
  `ceil(n/size)` of them, only the last is short, and their `Core\Str::length`s sum to the
  subject's — which is the cluster rule itself, since splitting the decomposed `é` in the subject
  would count six units against five while the re-join still held. At size 1 `chunk` is exactly
  `Core\Str::graphemes`, asserted elementwise.
- **`fold` then `compare` is a caseless equality that strictly contains `compare`'s own
  `{caseInsensitive: true}`.** Over all 81 ordered pairs of a 9-row table: the option calls 13
  equal, folding 21, and **nothing the option calls equal is different under folding** — the
  containment the doc comment claims, which no single row can distinguish from a coincidence. The
  8 extra pairs are `ß`/`ss` (4), the `ﬁ` ligature (2) and the two Greek sigmas (2); the option
  keeps the final sigma because its mapping is `lower`'s.
- **`fromCodePoint`/`fromCodePoints` admit exactly the Unicode scalar values**, shown as a mark
  per code point across four windows that each reach equally far either side of a boundary:
  U+0000 (not a boundary), U+D800 and U+DFFF (a *hole*, not a ceiling — a member refusing
  everything above U+D7FF still prints the first window right) and U+10FFFF. The same window
  asserts the two members agree at every point.

`orient.py`'s pack was complete for this group; nothing outside it was read.

The gap eighteen handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two
`Uri`s is `E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**`Core\Json`**, now a floor-2 class at depth 4.0 (55 cases over 13 members). One shared file
set: `crates/nvs-stdlib/src/json.rs` plus `tests/conformance/core/`. Read the existing cases'
`--TEST--` lines before picking a shape — a `gaps.py` count is not a shape inventory.

- [ ] **`Core\Json::decodeAs`** (2 cases) — row `crates/nvs-stdlib/src/json.rs:145`, helper
      `crates/nvs-stdlib/src/json.rs:713`. The likely gap is *a bound asserted on both sides*: the
      shapes it accepts into a declared type against the first one it refuses, and whether a
      refusal names the member it stopped at.
- [ ] **`Core\Json::isValid`** (3 cases) — row `json.rs:152`, helper `json.rs:989`. *Agreement* is
      the shape with room: `isValid($t)` must be true exactly where `decode($t)` does not throw,
      counted over one table of well- and ill-formed texts, so a predicate that grew its own
      parser fails while looking right on every line.
- [ ] **`Core\Json::decode`** (5 cases) — row `json.rs:138`, helper `json.rs:685`. Take it only if
      the two above leave context; `Core\Json::encode` round-trips are the invariance shape.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `Comparable` over two `Core`
  instances is `E0411` — docs/agent/handoff.md's own note, and it wants a session of its own.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  docs/agent/guard-name-debt.md.
- `Core\Random` (depth 4.0, floor 2: `float` 2, `token` 3) is the next thinnest after `Core\Json`,
  and its cases must be properties rather than frozen values.
- `crates/nvs-stdlib/src/csv.rs:512`'s `Fault::thrown` is unreachable from source until ADR 0007
  § 2's `array<T> as array<U>` lowers — playbook, *Divergences and refusals already pinned*.
- ADR 0007 § 2's `array<T> as array<U>` conversion does not lower, which is what blocks a case
  from indexing into an `array<mixed>` — playbook, *Writing a test case*.
