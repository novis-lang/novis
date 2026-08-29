# Handoff

## State

**The driver's failing acceptance check is closed.**
`tests/conformance/core/a-repeated-grapheme-count-agrees-with-the-first.nvst` is written and passes
against the tree as it stands, before goal 1 item 13's header cache exists — which is the point of
the shape it takes. It pins the invariant a cache must not break rather than the cache: every one of
nine subjects is asked its length three times, the third answer is checked against a fresh
`Core\Str::graphemes` partition, and the joined subject's count is compared to the sum of its two
halves. Five of the nine seams merge, so the corrected total is 22 against an additive 27 — the line
that fails if `length` ever becomes additive across `.`.

**Item 12's roster is down to `Core\Bytes`.** Twenty-two rows across five classes carry ADR 0088
§ 2's classification now — `Core\Arr`'s two key parameters, `Core\Attributes`' member name,
`Core\Math`'s `fromBase` and `format`, `Core\Json`'s three, and all eleven of `Core\Encoding`.
`Qual`'s doc at `crates/nvs-stdlib/src/registry.rs:74` stayed the only home of the rule; every
judgement here is one of its first two bullets.

**Two things the pass turned up, both recorded at their sites.** `Core\Arr`'s `int|string` key union
could not carry the mark, because `ARRAY_KEY` is shared between parameter positions that need
opposite answers and element positions the gate does not walk at all — so it split into
`ARRAY_KEY_NEUTRAL` and `ARRAY_KEY_CONTAGIOUS` beside the original, whose doc now says why the
original keeps an unclassified `Str` arm. And **the spec's Q column was wrong once**:
`Core\Math::format` was marked *neutral*, which is right for every other member of that class and
false for the one that renders its two separator options into the answer verbatim. The registry is
the home (ADR 0088 § 2) and the spec renders it, so the cell was corrected;
`FORMAT_OPTIONS`' doc in `math.rs` records the reasoning.

**The classification is still declaration-only.** `nvs_types::core_lib.rs:286` lowers
`CoreTy::Text(_)`/`Blob(_)` to a plain interned `string`/`bytes`, so nothing a program can do
changed and the playbook bullet about no `Core` member accepting a `tainted` argument holds for all
twenty-two. Enforcement is a separate item and no session has touched it.

**Untouched:** `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale (the route
table is built now), a `bytes` array key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — the
last two have playbook bullets under *Writing a test case*.

**Orientation gaps.** `[context] adrs` printed no section for ADR 0088 § 1, whose table decides
`Core\Json::decode` outright; the handoff had carried the verdict forward, so it cost nothing this
time, but the field still owes it. `0063`'s *In short* printed but not R11, which is where the four
sink grammars are named and is exactly what the next slice needs for `Core\Bytes::pack`. Still owed
from before: `0085 §§ 1-4` and `0071 §§ 2, 7`.

## Next group

**Item 12 continued — ADR 0088 § 2. Shared file set:** `crates/nvs-stdlib/src/registry.rs` (the
`UNCLASSIFIED` roster at `:1526`, which now opens at `Core\Bytes`) plus one
`crates/nvs-stdlib/src/<class>.rs` per slice. `Qual`'s doc at `registry.rs:74` is the rule and the
spec's Q column is the second opinion. **Land the whole group as one commit**: a member that is
classified *and* still on the roster fails the gate, so a per-slice commit leaves a red tree behind
it.

- [ ] **`Core\Bytes`' twelve rows** — `crates/nvs-stdlib/src/bytes.rs:161`. Ten follow the two
      ordinary bullets (`length`, `compare`, `contains`, `startsWith`, `endsWith`, `indexOf` answer
      no byte of an argument; `at`, `slice`, `repeat`, `join` do). **`pack` at `:246` and `unpack`
      at `:253` are the interesting pair**: their format string is the fourth of ADR 0063 R11's
      grammars, which `Qual`'s third bullet makes `Sink` — the first non-`Core\Str::format` sink in
      the registry, so read R11 rather than assuming.
- [ ] **`Core\Path`'s nine rows** — `crates/nvs-stdlib/src/path.rs:72`. Eight answer a path built
      out of their arguments and are contagious; `isAbsolute` at `:125` is the class's one `bool`
      and is neutral.
- [ ] **`Core\Validate`'s six rows** — `crates/nvs-stdlib/src/validate.rs:159`. Every one answers a
      `bool`, so all six are neutral by the first bullet with nothing to weigh. Cheapest slice on
      the roster; take it as the third if the first two left room.

## Backlog

- The conformance suite's `min_passing = 950` floor is unmet at 904 — `docs/agent/goals/1-core-depth.toml`.
- Item 13's own half is unwritten: the cached count in `NvsStr`'s header and its three
  `nvs-runtime` tests — `docs/agent/goals/1-core-depth.md` § *Stage 6*.
- `Core\Attributes` has no per-member Q column in `docs/spec/01-core-library.md` § 7 — only a class
  summary at `:916`, so its two marks live in `attributes.rs` alone.
- `router.rs:39`'s *Known gaps* 1 and 2 are stale and should be rewritten.
- Item 10's floor: `python tools/gaps.py` ranks the classes still under three cases.
