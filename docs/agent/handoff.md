# Handoff

## State

**Corpus at 951, all passing** (948 → 951), which **closes the Stage 8 acceptance check** that wanted
950 and failed after session 0009. No case fails and no check names a test that disappeared.

**The whole next group landed — all three slices.** `Core\Arr`'s three floor-3 members named by the
last handoff each gained the shape their existing cases were missing. **No `nvs-stdlib` source
changed**: every member the three cases reach already answered correctly, on the first run in each
case.

**What the three new cases pin**, all in `tests/conformance/core/`. (1) `from` and a `foreach` drive
one source identically: a single `foreach` records the entries seen and the advances made at every
prefix, and a fresh cursor per limit `0..6` is asked to match both — with the same comparison against
the *next* prefix required to agree nowhere, so seven identical cells could not pass. The unbounded
drive, the `Iterable` shape and the array shape (where the agreement is over the values, the keys
being what `from` drops) are each asserted beside it. (2) `product` is `sum`'s fold over the other
operator: each agrees with the fold written out, both split at every one of 28 split points, each
seed is inert in its own fold and only its own (`1` leaves the product alone and moves the sum, `0`
the reverse), both promote alike, and both refuse the same two subjects naming themselves — while
`sum` answers the `int` row whose product overflows, which is where the pair parts. (3) `overlayDeep`
at a key holding maps on both sides is *itself* applied to those two maps — the identity `merged`'s
re-entry into `overlay_into` means — bottoming out in `::overlay` at one level, with the nested key
order rebuilt from both sides (base's order, then the layer's new keys) rather than read off either
answer.

**Files:** `arr-from-and-a-foreach-drive-one-sequence-alike-at-every-prefix.nvst`,
`arr-product-is-the-fold-arr-sum-is-over-the-other-operator.nvst`,
`arr-overlay-deep-at-a-key-is-itself-one-level-down-and-keeps-the-bases-order.nvst`.

**One trap cost a run and has a playbook bullet**: a loop echoing its separator after each row leaves
a trailing space, and `--EXPECT--` is byte-exact.

**`Core\Arr` is still the thinnest class** — `python tools/gaps.py --coverage` ranks it floor **3**
over 55 members and 237 cases — and the three members left at that floor are the next group below.
`Core\Math` is beside it at floor 3 (`atan2`, `hypot`, `lcm`) if `Core\Arr` clears.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`)
and `catch (Core\Error $e)` panicking — both have playbook bullets. `Core\Math` has no `pow`; `**` is
the operator, and `E0405` says so at once.

**Orientation.** The pack was complete for the item it was given. Still owed by `[context]`:
`hash.rs`, `test.rs`, `nvs-types/src/links.rs`, `routes.rs`, `validate.rs`,
`docs/spec/01-core-library.md`, `docs/spec/02-php-migration.md`, `tools/check-migration.py`, the
stage-7 comment header's per-goal floor table, and a selector printing the failing check's own
`cases` block.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/arr.rs` and `tests/conformance/core/`. These are the
three members `gaps.py --coverage` now names at `Core\Arr`'s floor of 3, and the first two share the
offset/length reading the `Core\Str` twins already have a worked pair for.

- [ ] **`replaceRange` takes the window `slice` names, and putting the window back is the subject**
      (`nvs_core_arr_replace_range` at `crates/nvs-stdlib/src/arr.rs:1717`) — spec § 2. The `Core\Str`
      twin's shape, over an array: the offset/length sign table on both sides, and the identity
      `replaceRange($a, $k, $n, Core\Arr::slice($a, $k, $n)) == $a` over every cell of it. The
      playbook's *`Core\Str`'s twins part from PHP over a unit* bullet owns why the string pair needed
      an oracle and this one does not — an array indexes entries, not clusters.
- [ ] **`withoutFirst` is `slice($a, 1)`, and the question is what happens to the keys**
      (`nvs_core_arr_without_first` at `crates/nvs-stdlib/src/arr.rs:1910`) — spec § 2. Agreement with
      `slice`, with `first`/`firstKey` before and after, and over a map as well as a list, so whether
      the remaining entries keep their keys is asserted rather than shown on one line.
- [ ] **`underlay` is not `overlay` with its arguments flipped, and the difference is key order**
      (`nvs_core_arr_underlay` at `crates/nvs-stdlib/src/arr.rs:3858`) — ADR 0069 § 1; the doc comment
      at that anchor states the claim outright. Over a table: `underlay($a, $b)` and `overlay($b, $a)`
      hold the same entries for every pair (compare after `Core\Arr::sort` of the keys) and part on
      `Core\Arr::keys` wherever `$b` holds a key `$a` does not — counted both ways, so a member that
      was implemented as the flip fails.

## Backlog

- Item 12's 10 `UNCLASSIFIED` members — `crates/nvs-stdlib/src/registry.rs:1526`, goal item 12.
- `catch (Core\Error $e)` ICEs in `nvs-ir` — playbook, *Divergences and refusals already pinned*.
- A `string|int` array subscript ICEs where the `Core` member taking the union accepts it — playbook.
- `Core\Math` at floor 3 (`atan2`, `hypot`, `lcm`) once `Core\Arr` clears — `gaps.py --coverage`.
- The `[context]` selectors listed under *Orientation* above — `docs/agent/loop-goal.toml`.
