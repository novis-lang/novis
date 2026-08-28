# Handoff

## State

**M4's Stage 8, depth.** The tree is at **823 conformance plus 189 differential**. Nothing is
blocked.

`Core\Uuid` is off the thin list — `python tools/gaps.py` now ranks it at depth 5.0, floor 3, where
it opened the session at floor 2. Two cases landed, both over `crates/nvs-stdlib/src/uuid.rs`'s own
doc comments:

- **`tryParse` and `parse` are one reader**, the *agreement* shape. The existing parse case asks
  each member about its own subjects, so a `tryParse` that grew its own idea of the shape answered
  plausibly on every line it appeared on. The new case asks all 18 subjects of both, renders a
  throw and a `null` as the same empty string (no successful parse can produce one, `toString`
  being 36 wide), and asserts the per-row equality plus six fixed counts. Three of the subjects are
  one value in three case spellings, counted separately, so the agreement is on the *value* and not
  merely on acceptance.
- **`v7` carries the clock `Core\Time::now()` reads**, which is the claim `uuid.rs:309`'s doc makes
  and no case could see: the two sibling cases assert that the leading field *increases* and opens
  with `01`, which any monotonic counter satisfies. The new case reads the 48-bit field as the
  number it is — `Core\Encoding::fromHex` over the first twelve hex digits, folded octet by octet
  through `Core\Bytes::at` — and brackets one draw between two `toEpochMillis` reads. `v4`'s leading
  field is asked the same question and lands elsewhere, which is what says the window can answer no.
  A 32-draw sweep then bounds every stamp inside one window and counts repeats at 0, the second
  being the 74-random-bits half of the layout.

`orient.py` printed two `!!` lines this session: `[context] playbook` in `docs/agent/loop-goal.toml`
names `"Writing a test case > nvs-codegen has"` and `"Writing a test case > emitbinop's ordering"`,
and no bullet under that heading leads with either — the two selectors are stale and should be
re-pointed or dropped.

The gap eleven handoffs back still stands: **no `Core` class reaches `nvs_hir::implements_interface`**,
so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s is `E0411`. It is in the backlog and
still deserves a session of its own.

## Next group

**`Core\Arr`'s three floor-1 members** — `gaps.py`'s thinnest members anywhere in the corpus, one
file set: `crates/nvs-stdlib/src/arr.rs` and `tests/conformance/core/`. Each has exactly one case,
and over `Core\Random`, `Core\Debug`, `Core\Bytes` and `Core\Uuid` alike the gap was never a missing
row — read the member's doc comment first and look for the rule stated there that no case observes.

- [ ] **`flattenDeep` against `flatten`, as a fixed point** (`crates/nvs-stdlib/src/arr.rs:2161`) —
      the *invariance* shape: `flattenDeep` is `flatten` applied until it stops changing, so
      asserting that over a sweep of nestings is one rule rather than one row per depth.
      `arr-flatten-unwraps-one-level-and-flatten-deep-all-of-them.nvst` is the case that exists.
- [ ] **`column`'s missing key and its index argument** (`crates/nvs-stdlib/src/arr.rs:2218`) — the
      *edges*: a key absent from some rows but not others, and the three-argument form's re-keying
      when the index key collides. `arr-column-takes-one-cell-out-of-every-row.nvst` exists.
- [ ] **`overlayDeep` against `overlay`** (`crates/nvs-stdlib/src/arr.rs:3748`) — no case names it
      at all beyond the one; the *agreement* shape at depth 1, where the two must not differ.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `Core\Uri::compareTo` exists while
  `$a < $b` over two `Uri`s is `E0411` — `docs/agent/loop-goal.md`, its own session.
- `[context] playbook` in `docs/agent/loop-goal.toml` has two stale bullet selectors under *Writing
  a test case* (above).
- `Core\Debug` is `gaps.py`'s thinnest class by median (3.5) with `dump` at 2 — `docs/agent/loop-goal.md`.
- `Core\Str::fold` / `graphemes` / `indexOf` are floor-1 too, but the Windows `php` has no
  `mbstring` for the Unicode rows — `docs/agent/playbook.md`.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `csv.rs:512`'s `Fault::thrown` is unreachable until ADR 0007 § 2's `array<T> as array<U>` lowers —
  `docs/agent/playbook.md`.
