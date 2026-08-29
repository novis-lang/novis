# Handoff

## State

**Item 12's roster: 10 members across five classes remain** on `UNCLASSIFIED` at
`crates/nvs-stdlib/src/registry.rs:1526` — `Core\Uuid` (2), `Core\Hash` (3), `Core\Hash\Stream` (1),
`Core\Router` (2), `Core\Csv` (2). `Core\Uri`, `Core\Test` and `Core\Validate` joined `Core\Bytes`,
`Core\Path` and `Core\Time` off the ratchet this session, 23 members in all.

**`Core\Uri` is the roster's first class to claim `Qual::Launder`**, and the only judgement in it
worth re-reading is the one against `Core\Time`: a parse here is `Contagious`, not `Neutral`,
because `host()`/`path()`/`query()` hand the caller's own bytes back component for component. The
two encoders launder and their doc comments name the sink (the URI grammar; an
`application/x-www-form-urlencoded` body); the two decoders stay `Contagious`, because a pair that
shared a mark would launder every string surviving a round trip. `uri.rs`'s module doc
§ *What these members do with a qualifier* owns all of it.

**`Core\Test` and `Core\Validate` are the flat case** — every member answers `void` or `bool`, so
every parameter is `Neutral` by the `Qual` enum's own first bullet at `registry.rs:74`, with no
judgement to make. The two things that look like more are written down in `test.rs`'s new
§ *What these members do with a qualifier*: `message` is not a sink (the terminal makes that refusal
once, where the bytes are written — ADR 0086), and `assertThrows`'s `$expected` is not one either,
because `Sink` is only ever ADR 0063 R11's four grammars.

**Nothing consumes `Qual` yet** — it is declarative in `nvs-stdlib` and no `nvs-types` code reads
`classification()`, so a classification slice cannot change what a program does today.

**The driver's failing check is `conformance [5 depth]: 904 passing, wanted 950`, and it is not a
regression** — it is item 10's corpus floor, 46 cases short, and no classification slice can move it
because `Qual` is unobservable from a `.nvst` case. It closes by depth cases, which is what the next
group starts; the goal's own standing decision is that a class's `floor` column moving is what
counts, not the total.

**Untouched:** `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale, a `bytes`
array key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — the last two have playbook bullets
under *Writing a test case*.

**Orientation gaps.** Still owed, none added yet: `docs/spec/02-php-migration.md` and
`tools/check-migration.py` in `[context] docs`, the stage-7 comment header's per-goal floor table, a
selector that prints the *failing* check's own `cases` block, `[context] modules` naming an
`nvs-stdlib` class module and `nvs-ir/src/lower/*`, a `[context] anchors` entry for `registry.rs`'s
`UNCLASSIFIED`, and ADR 0088 § 1, 0063 R11, `0085 §§ 1-4`, `0071 §§ 2, 7`. New this session:
`[context] shapes` needs conventions.md's *four shapes a depth case takes* whenever the group is
conformance depth rather than a member — it is printed today only because the `Core` member shape
selector happens to sit beside it.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/random.rs` and `tests/conformance/core/`. `gaps.py`
ranks `Core\Random` the second-thinnest class (depth 4.0, floor 3, 13 cases over 7 members) and its
three thinnest members are one file's worth of reading. Conventions.md's *four shapes a depth case
takes* is the shape list; a bound asserted on both sides is the one these three want.

- [ ] **`Core\Random::float`'s bounds** (`crates/nvs-stdlib/src/random.rs:285`, 3 cases) — the
      half-open interval asserted on both ends, and the sweep that says no draw leaves it.
- [ ] **`Core\Random::int`'s inclusive pair** (`crates/nvs-stdlib/src/random.rs:263`, 4 cases) — the
      degenerate `int(n, n)`, the inverted pair's refusal, and a sweep counting that every draw is
      inside.
- [ ] **`Core\Random::pick` over an empty and a one-entry array**
      (`crates/nvs-stdlib/src/random.rs:428`, 4 cases) — the boundary where it stops accepting, with
      `gaps.py --errors` naming the refusal.

## Backlog

- The ratchet's last 10 members: `Core\Uuid`, `Core\Hash`, `Core\Hash\Stream`, `Core\Router`,
  `Core\Csv` — `registry.rs:1526`, ADR 0088 § 2.
- `Core\Hash`'s three are the roster's remaining judgement: a digest of a `secret` is the `Qual`
  enum's own "a hash of a secret" example of `Neutral`.
- Conformance depth for `Core\Router`, `Core\Hash\Stream` and `Core\Csv` — `gaps.py`'s class table.
- `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale.
- A `bytes` array key ICEs in `nvs-ir` — playbook, *Writing a test case*.
- `catch (Core\Error $e)` panics rather than diagnosing — playbook, *Writing a test case*.
