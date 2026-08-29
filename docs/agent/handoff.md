# Handoff

## State

**The conformance corpus is at 908 and the driver's floor is 950**, so the failing acceptance check is
42 cases short and is not a regression — it is item 10's corpus floor, closing by depth cases at about
two a session. Item 12's roster is untouched: 10 members across five classes remain `UNCLASSIFIED` at
`crates/nvs-stdlib/src/registry.rs:1526` — `Core\Uuid` (2), `Core\Hash` (3), `Core\Hash\Stream` (1),
`Core\Router` (2), `Core\Csv` (2).

**`Core\Hash\Stream`'s floor moved 4 → 5 and `Core\Hash::hmac`'s 4 → 5.** What the two cases add over
the eight already there: the stream is an *identity* — three names for one stream (a second binding, a
helper's `Core\Hash\Stream` parameter, an `array<Core\Hash\Stream>` entry) feed one digest, `finish`
closes the object rather than the name, and two streams of one digest do not mix, counted over all six
`Core\Digest` cases — and `hmac`'s key rule is now asserted **at** its bound rather than either side of
it: block-1 zero-extends, block does not, block is not hashed, block+1 is, for both block widths.

**Two of the previous group's three items were already on disk**, which is the third consecutive time.
Item 1's terminal `finish` is the tail of `hash-streams-a-digest-in-chunks.nvst`; item 2's empty
`update` and never-updated stream are in that case and in `hash-stream-chunking-is-not-observable.nvst`;
item 3's block-wide key is `Rfc2104::agreeing`'s third row. The playbook bullets at *"read the bodies,
not only the `--TEST--` lines"* already own this — no new bullet was added.

**Nothing consumes `Qual` yet** — it is declarative in `nvs-stdlib`, no `nvs-types` code reads
`classification()`, so a classification slice still cannot change what a program does today.

**Untouched:** `crates/nvs-stdlib/src/router.rs:38`'s *Known gaps* 1 and 2 are stale, a `bytes` array
key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — the last two have playbook bullets under
*Writing a test case*.

**Orientation gaps.** Still owed, none added yet: `docs/spec/02-php-migration.md` and
`tools/check-migration.py` in `[context] docs`, the stage-7 comment header's per-goal floor table, a
selector that prints the *failing* check's own `cases` block, `[context] modules` naming an
`nvs-stdlib` class module and `nvs-ir/src/lower/*`, a `[context] anchors` entry for `registry.rs`'s
`UNCLASSIFIED`, ADR 0088 § 1, 0063 R11, `0085 §§ 1-4`, `0071 §§ 2, 7`, and conventions.md's *four
shapes a depth case takes* whenever the group is conformance depth rather than a member.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/router.rs` and `tests/conformance/core/router-url-*.nvst`.
`gaps.py` ranks `Core\Router` second-thinnest (depth 4.5, floor 2 — `urlAbsolute` 2, `url` 7) and only
four cases exist; read all four bodies first, not their `--TEST--` lines, because this group's premise
has now been stale three sessions running. `::match` is out of scope (loop-goal § *Standing
decisions*). Rows at `router.rs:139`/`:146`, bodies at `router.rs:417`/`:430`.

- [ ] **A capture's value is percent-encoded into its own segment** (`router.rs:417`) — a `{slug}`
      whose value carries `/`, a space or a non-ASCII cluster. The leftover-key case pins the *query*
      spelling against `Core\Uri::buildQuery`; the path segment's own encoding is a different rule and
      no case states it. ADR 0102 § 6. *Edges*.
- [ ] **`urlAbsolute` is `url` with the mount's origin in front, and nothing else** (`router.rs:430`) —
      one question asked of every route the fixture declares, asserting the two **agree** on the
      path-and-query tail by counting, rather than what either answered. ADR 0102 § 6. *Agreement*.
- [ ] **A typed capture refuses a value outside its closed set** (`router.rs:139`) — the last accepted
      value and the first refused one, named together, for the capture types ADR 0102 § 5 admits.
      *A bound asserted on both sides*.

## Backlog

- Item 12's 10 `UNCLASSIFIED` members — `crates/nvs-stdlib/src/registry.rs:1526`, ADR 0088 § 2.
- `router.rs:38`'s *Known gaps* 1 and 2 are stale — that crate's module doc owns the fix.
- A `bytes` array key ICEs in `nvs-ir` — playbook, *Writing a test case*.
- `catch (Core\Error $e)` panics rather than diagnosing — playbook, *Writing a test case*.
- `Core\Csv` (floor 5, 2 members) and `Core\Debug` (floor 2, `render` 7 / `dump` 3) are the next two
  thin classes after `Core\Router` — `docs/spec/01-core-library.md` §§ 13 and 21.
