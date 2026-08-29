# Handoff

## State

**`Core\Regex`'s conformance floor is closed.** Three depth cases landed over
`crates/nvs-stdlib/src/regex.rs`'s `quote`, `replaceWith` and `split`, and `Core\Regex::quote` is
deleted from `crates/nvs-stdlib/tests/conformance_coverage.rs`'s `BELOW_THE_FLOOR` — whose whole
remaining content is now `Core\Router::urlAbsolute` and `Core\Time\TimeOfDay::compareTo`. The corpus
is at **927**, all passing.

**The Stage 8 acceptance check still fails at 927 of 950 wanted.** That is a growth floor, not a
regression — no case fails and no check names a test that disappeared. It closes as the suite grows.

**What the three cases pin**, all in `tests/conformance/core/`:

- `...-quote-makes-a-literal-match-itself-exactly-once-...nvst` — the member's own escaped set (a
  character is taken iff quoting changes it) placed at the start, the middle and the end of an `ab`
  carrier: 54 subjects, every one of them exactly one match at offset 0 spanning the whole subject, and
  exactly one at offset 2 inside `XX..YY`. The two siblings ask `matches`, a yes-or-no question; this
  asks *how many* and *where*. Its second half is a 54×54 sweep — 54 of 2,916 cells match, all 54 on
  the diagonal — beside the same sweep run unquoted, where 41 patterns compile at all and reach 546
  cells belonging to someone else. The unquoted partition (13 refused + 24 coincide + 17 read
  differently) is what keeps the first count from being a restatement of the engine.
- `...-replace-with-sees-exactly-the-matches-match-all-reports.nvst` — agreement, not behaviour: the
  callback's `(offset, text)` sequence equals `matchAll`'s over ten rows chosen for how a match
  sequence goes wrong (adjacency, zero width, zero-width beside wide, alternation order, anchors,
  non-ASCII offsets, the empty sequence), 25 calls against 25 matches. An identity callback returning
  `$m->text()` rebuilds all ten subjects, which pins the copying between matches on the same call.
- `...-split-loses-exactly-what-it-matched-and-str-join-puts-it-back.nvst` — `split` throws away the
  matched text and nothing else. 12 literal-separator rows restored by `Core\Str::join`; six general
  patterns woven back from pieces interleaved with `matchAll`'s texts, each with one more piece than
  match. `keepEmpty: false` costs the inverse on 4 of the 5 rows holding an empty piece — the
  exception is the empty subject, whose dropped partition joins back to the empty string anyway.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`),
a `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have playbook
bullets under *Writing a test case*.

**Orientation gaps.** `[context] modules` owed `crates/nvs-stdlib/src/str.rs` this session (the
`Core\Str::join` row, one grep) and owes `crates/nvs-stdlib/src/time.rs` for the next group. Still
owed: `crates/nvs-stdlib/src/hash.rs`, `src/test.rs`, `crates/nvs-types/src/links.rs`, `src/routes.rs`,
`crates/nvs-stdlib/src/validate.rs`, `docs/spec/01-core-library.md`, `docs/spec/02-php-migration.md`,
`tools/check-migration.py`, the stage-7 comment header's per-goal floor table, a selector printing the
*failing* check's own `cases` block, and a `[context] anchors` entry for `registry.rs`'s
`UNCLASSIFIED`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/time.rs` and `tests/conformance/core/`. `Core\Time\TimeOfDay`
is depth **6.0**, floor **2**, 9 cases over 6 members, and `compareTo` is the last `BELOW_THE_FLOOR`
line reachable without touching `Core\Router` — the goal's standing decision says that class splits and
`::match` is out of scope, so leave `urlAbsolute` until the split lands.

- [ ] **`Core\Time\TimeOfDay::compareTo` is one total order, asserted by counting**
      (`crates/nvs-stdlib/src/time.rs:3012`) — over a table of times, antisymmetry and transitivity
      counted rather than read off a row, and the sign agreeing with the ordering `Core\Arr::sort`
      produces over the same table. **Delete the `Core\Time\TimeOfDay::compareTo` line from
      `crates/nvs-stdlib/tests/conformance_coverage.rs:265` in the same commit** — the floor test fails
      on an entry that has reached three cases, not only on one that has not.
- [ ] **`plus` and `minus` are inverses over a table of durations**
      (`crates/nvs-stdlib/src/time.rs:2966` and `:2973`) — every `$t->plus($d)->minus($d)` back to `$t`,
      counted, and whether the pair wraps at midnight or refuses is the boundary the case names on both
      sides. Check the member bodies before writing: this handoff does not know which it is.
- [ ] **The day's two ends, named together** — the first and last representable `TimeOfDay`, what
      `plus`/`minus` do at each, and what `compareTo` answers between them.

## Backlog

- `Core\Router::urlAbsolute` is the last `BELOW_THE_FLOOR` line after the group above (ADR 0102 § 6).
- Item 12's `UNCLASSIFIED` roster — `crates/nvs-stdlib/src/registry.rs:1526`, ADR 0088 § 2.
- `Core\Debug` (7 cases over 2 members) and `Core\Csv` / `Core\Hash\Stream` (5 over 2) are the
  thinnest classes by total; `python tools/gaps.py` ranks them.
- The `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — both playbook bullets.
- `python tools/check-migration.py` at 34%; the plan's *Open now* names it as the program's measure.
