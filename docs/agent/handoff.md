# Handoff

## State

**Corpus at 948, all passing** (944 → 948). The Stage 8 acceptance check wants 950 and still fails:
a growth floor, not a regression — no case fails and no check names a test that disappeared. **Two
more cases close it**, and the next group below is three.

**The whole next group landed, and a fourth slice with it.** `Core\Arr`'s three thinnest members —
`column`, `fillKeys`, `firstKey`/`lastKey` — each gained the shape its existing cases were missing,
and `flattenDeep` gained the one no case had asked for. **No `nvs-stdlib` source changed**: every
member the four cases reach already answered correctly.

**What the four new cases pin**, all in `tests/conformance/core/`. (1) `column`'s answer is the
cells of exactly the rows `Core\Arr::hasKey` admits, in row order, rebuilt from a seven-row table
and compared over five columns — so a `null` cell counts and an absent key does not, which is the
one distinction the member turns on and no line-by-line case separates; under `{indexBy}` the
length becomes the number of *distinct* index cells, counted by hand over four more columns.
(2) `fillKeys` and `fromKeysAndValues` answer identically for seven key lists paired with
`Core\Arr::fill(count($keys), $v)` — the raw count, not the distinct one, which the length-mismatch
throw asserts from the other side — with the distinct count and the first-occurrence order each
rebuilt independently. (3) `firstKey`/`lastKey` agree with `keys`, with what `foreach` binds and
with `first`/`last` after every step of an eight-step script of writes and removals: a write over a
held key moves it nowhere, remove-then-write moves it to the end, and an emptied array has no ends.
(4) `flattenDeep` answers the same two leaves at depths 0 through 5,000 — the explicit stack its
doc comment claims, at a depth no literal could spell — and an array written into itself flattens
to the snapshot it stored, which is why the walk needs no visited set.

**Files:** `arr-column-answers-one-cell-per-row-that-has-one-over-a-whole-table.nvst`,
`arr-fill-keys-is-from-keys-and-values-over-one-value-repeated.nvst`,
`arr-first-key-and-last-key-name-the-ends-of-insertion-order-after-every-write-and-removal.nvst`,
`arr-flatten-deep-walks-a-nesting-no-recursion-could-and-a-self-storing-array-is-a-copy.nvst`.

**One trap cost a run and has a playbook bullet**: a `string|int` array subscript ICEs in `nvs-ir`
while the `Core` member taking the same union accepts it. Same family as the `bytes` array key ICE.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`)
and `catch (Core\Error $e)` panicking — both have playbook bullets.

**Orientation.** The pack was complete for the item it was given, and `[context] modules` already
carried `arr.rs`. Still owed: `hash.rs`, `test.rs`, `nvs-types/src/links.rs`, `routes.rs`,
`validate.rs`, `docs/spec/01-core-library.md`, `docs/spec/02-php-migration.md`,
`tools/check-migration.py`, the stage-7 comment header's per-goal floor table, and a selector
printing the failing check's own `cases` block.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/arr.rs` and `tests/conformance/core/`. `Core\Arr` is
still the class `python tools/gaps.py --coverage` ranks thinnest — floor **3** over 55 members and
234 cases — and these are the three members left at that floor. **Two of them close the 950
acceptance check.**

- [ ] **`from` drains a sequence it does not own, and the drive is where it stops**
      (`nvs_core_arr_from` at `crates/nvs-stdlib/src/arr.rs:2527`) — existing:
      `arr-from-drains-a-sequence.nvst`, `arr-from-limit-bounds-the-drive-on-both-sides.nvst` and
      `arr-readers-agree-across-both-array-shapes.nvst`. Left is the *agreement*: one sequence
      drained by `from` and walked by `foreach`, asserting the two see the same entries in the same
      order at every prefix, rather than comparing two finished arrays.
- [ ] **`overlayDeep` recurses exactly where both sides hold a map and nowhere else**
      (`nvs_core_arr_overlay_deep` at `:3836`) — existing:
      `arr-overlay-deep-differs-from-overlay-only-where-both-sides-hold-a-map.nvst`,
      `arr-combination-members-treat-every-key-alike.nvst` and
      `arr-the-empty-array-is-what-every-reshaping-member-answers-it-with.nvst`. Left is
      *invariance over a sweep*: over a table of key/value shape pairs — list against map, map
      against scalar, absent against present, empty map against non-empty — count the cells where
      `overlayDeep` and `overlay` agree and assert it is exactly the cells where the recursion does
      not apply.
- [ ] **`product` is the fold `sum` is, over the other operator** (`nvs_core_arr_product` at
      `:4186`) — existing: `arr-sum-product-and-average-over-numbers.nvst`,
      `arr-numeric-folds-agree-on-one-entry-and-part-on-none.nvst` and
      `arr-readers-agree-across-both-array-shapes.nvst`. Left is the *agreement* with
      `Core\Arr::reduce` over an explicit multiply, across a sweep that crosses the int/float
      boundary — including the empty array's 1, a single zero, and a value large enough to leave
      `int`.

## Backlog

- `Core\Math`'s floor-3 trio — `atan2`, `hypot`, `lcm` — is the next class after `Core\Arr`, and
  shares `crates/nvs-stdlib/src/math.rs` (`python tools/gaps.py --coverage`).
- A `string|int` array subscript ICEs in `nvs-ir` (`crates/nvs-ir/src/lower/expr.rs:1972` has no
  tagged arm); same family as the `bytes` array key ICE, and neither is fixed.
- `catch (Core\Error $e)` panics rather than diagnosing — playbook, *Writing a test case*.
- Item 12's roster: 10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`.
- `[context]` still owes `hash.rs`, `test.rs`, `links.rs`, `routes.rs`, `validate.rs`, the two spec
  files, `tools/check-migration.py` and a selector for the failing check's own `cases` block.
