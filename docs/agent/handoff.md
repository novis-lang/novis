# Handoff

## State

**Corpus at 941, all passing** (938 → 941 this session). The Stage 8 acceptance check wants 950 and
still fails: a growth floor, not a regression — no case fails and no check names a test that
disappeared. It needs roughly two more sessions of this shape.

**The previous handoff's next group was already landed and was not written.** `Core\Math`'s `lcm`,
`gcd`, `hypot` and `atan2` are all pinned to depth already; `gaps.py --coverage` counts cases per
member and cannot see that, which now has a playbook bullet under *Tooling*. What this session did
instead is § 9's two object collections, where the thin members were genuinely thin. **No
`nvs-stdlib` source changed** — every member the three cases reach already answered correctly.

**Three facts the new cases pin and nothing else records.** (1) A `Core\ObjectMap`'s keys *are* a
`Core\ObjectSet`: one script of writes run against both leaves `count`, `isEmpty`, `has` over the
whole universe, the `foreach` order and `keys()` agreeing at all 15 steps, a write over a held key
moves it nowhere in either, and remove-then-write moves it to the end in both. (2) Both walk a
**snapshot taken where the loop begins**, so a body may `remove`, `add` or `clear` what it is
walking and the walk still finishes whole — and a nested walk takes its own snapshot on every
entry, so the inner loop sees the outer body's removals: 3 + 2 + 1 pairs, not 3 × 3. (3) With a
nullable `V`, `get` cannot separate an absent key from a key holding `null` — that is § 9's `?V`
read giving up information, the cost of having no subscript to throw and no `getOrNull` twin (ADR
0063 R5) — while `has`, `count`, `keys` and `values` all still separate the two.

**What the three new cases pin**, all in `tests/conformance/core/`:

- `object-collections-are-one-collection-asked-two-ways.nvst` — the cross-class agreement above,
  counted over a 15-step script whose every mutator meets its degenerate case, plus the two
  insertion-order rules and the identity rule (a twin by contents is a second entry in each; an
  alias is not).
- `object-collections-walk-a-snapshot-so-a-body-may-mutate-what-it-walks.nvst` — removal, addition
  and `clear` inside the body, each asserted as the set's walk *and* the map's key walk agreeing,
  plus what the collection is left holding, plus the nested-walk count.
- `object-map-has-is-the-only-member-that-tells-an-absent-key-from-a-null-value.nvst` — the two
  indistinguishable reads side by side, then every member that does distinguish them, then that a
  `null` write is a write and a removal is a removal.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`),
a `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have
playbook bullets under *Writing a test case*.

**Orientation.** The pack was complete for the group it was given, but the group was stale, and
choosing a new one cost `gaps.py --coverage`, `gaps.py --errors` and four greps outside the
manifest. Add `crates/nvs-stdlib/src/objset.rs`, `objmap.rs` and `router.rs` to `[context] modules`;
`math.rs`, which the last handoff asked for, is no longer needed. Still owed from the last four
sessions: `hash.rs`, `test.rs`, `nvs-types/src/links.rs`, `routes.rs`, `validate.rs`,
`docs/spec/01-core-library.md`, `docs/spec/02-php-migration.md`, `tools/check-migration.py`, the
stage-7 comment header's per-goal floor table, a selector printing the failing check's own `cases`
block, and a `[context] anchors` entry for `registry.rs`'s `UNCLASSIFIED`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/router.rs` and `tests/conformance/core/`.
`Core\Router::urlAbsolute` has the lowest floor in the whole `gaps.py --coverage` table — **2**
cases against `url`'s 11 — and its two existing cases are both refusals, so nothing yet says what it
*answers*. Checked, not assumed: `ls tests/conformance/core/ | grep router` is the eight files.

- [ ] **`urlAbsolute` is `url` with the configured origin in front, on every row `url` already
      builds** (`nvs_core_router_url_absolute` at `crates/nvs-stdlib/src/router.rs:459`,
      `nvs_core_router_url` at `:446`) — the *agreement* shape over the rows the seven `router-url-*`
      cases build: a closed-set capture, a computed value, a dropped optional capture, a leftover
      `params` key that becomes a query string. Count the rows where `urlAbsolute` is exactly
      `origin . url`, rather than reading either off a line.
- [ ] **The origin is the mount's configuration and nothing a request carries** (same two anchors)
      — ADR 0102 § 6 makes this the rule; assert that a `Host` or `X-Forwarded-Host` header does not
      move the answer, beside the existing no-origin refusal.
- [ ] **The join of origin and path happens once** (`router.rs:459`) — the *bound* shape over an
      origin written with and without a trailing slash, with an explicit port, and under both
      schemes: one separator in every cell, never two and never none.

## Backlog

- `Core\Uuid` (`tryParse` 3, `v7` 4) and `Core\Debug` (`dump` 3) are the next-thinnest floors —
  `python tools/gaps.py --coverage`, checked against a grep before being believed.
- Item 12's `UNCLASSIFIED` roster, 10 members — `crates/nvs-stdlib/src/registry.rs:1526`.
- The `bytes` array key ICE in `nvs-ir` — `docs/agent/playbook.md`, *Writing a test case*.
- `catch (Core\Error $e)` panics instead of diagnosing — same section.
- The `[context]` manifest additions listed under *Orientation* above —
  `docs/agent/loop-goal.toml`.
