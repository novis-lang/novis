# Handoff

## State

**ADR 0009's cached grapheme count is on disk, which closes the stage 6 acceptance check.**
`nvs_runtime::StrHeader` has a fourth word — the count, `COUNT_UNKNOWN` until something asks —
filled lazily by `NvsStr::grapheme_count` and corrected at every seam by `nvs_str_concat`,
`nvs_str_concat_n` and `nvs_str_append`. The UAX #29 kernel moved to the new
`crates/nvs-runtime/src/graphemes.rs`, because only the crate owning the header can correct a
join; `nvs_stdlib::granularity` still states *which* unit is the default and now calls that
kernel, with `Unit::length_of` as the seam that reads the header and `Core\Str::length` as its
first caller. What each half spends is in `string.rs`'s § *The cached grapheme count* and in
`graphemes.rs`'s § *Why one seam is not always one correction*.

**A propagation happens only when every operand already has a count**, which is also what keeps a
`bytes` payload out of the text paths — nothing ever asks a `bytes` for clusters, so no `bytes`
allocation has a cached count and no seam of one is read as text. An immortal literal's count is
written by `immortal_header_bytes` at compile time (its signature now takes the payload, not the
length), so the one header two requests share is still never written at run time.

**Item 12's roster is still untouched** and still opens at `Core\Bytes`; the acceptance check
outranked it a third time. The roster is `crates/nvs-stdlib/src/registry.rs:1526`'s `UNCLASSIFIED`
list — a ratchet that only shrinks, 62 members across ten classes: `Bytes` 12, `Path` 9, `Time` 6,
`Uri` 9, `Test` 8, `Validate` 6, `Hash` 4, `Uuid` 2, `Router` 2, `Csv` 2.

**Untouched:** `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale (the route
table is built now), a `bytes` array key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics —
the last two have playbook bullets under *Writing a test case*.

**Orientation gaps.** New and the expensive one: `0009` is in `[context] rules` but no `[context]
adrs` entry names its sections, so the item's own specification — § 2 and *Consequences* — had to
be sliced by hand; add `0009 § 2` and `0009 Consequences`. Still owed from before: a selector that
prints the *failing* check's own `cases` block, `[context] modules` naming an `nvs-stdlib` class
module, a `[context] anchors` entry for `registry.rs`'s `UNCLASSIFIED` list, and ADR 0088 § 1,
0063 R11, `0085 §§ 1-4`, `0071 §§ 2, 7`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/registry.rs` — every slice deletes its class's lines
from `UNCLASSIFIED` at `registry.rs:1526` and reads the rule at `registry.rs:74` — plus one class
module each. Item 12 in roster order, largest class first.

- [ ] **`Core\Bytes`' twelve rows** (ADR 0088 § 2) — `crates/nvs-stdlib/src/bytes.rs:161`. Ten
      follow the two ordinary bullets; `pack` at `bytes.rs:246` and `unpack` at `bytes.rs:253` are
      ADR 0063 R11's fourth grammar, which `Qual`'s third bullet makes `Sink`. Land the class as
      one commit — a member classified *and* still named in `UNCLASSIFIED` fails the gate just as
      an unclassified one missing from it does.
- [ ] **`Core\Path`'s nine rows** (ADR 0088 § 2) — `crates/nvs-stdlib/src/path.rs:72`. Every
      parameter is a filesystem path, so the question each row answers is whether it reaches a
      syscall.
- [ ] **`Core\Time`'s six rows** (ADR 0088 § 2) — `Time::fromIso` at
      `crates/nvs-stdlib/src/time.rs:1130` and `Time::parse` beside it are the two that read a
      caller's text.

## Backlog

- `Core\Str`'s other unit-counting members still call `Unit::length` on an unwrapped `&str`;
  `granularity::Unit::length_of` is what reaches the cache (that module's own docs).
- No `.nvst` case observes the cached count from source — it is a runtime property, and
  `benches/abi-probe`'s perf guard is where a *repeat* index would be measured.
- Item 14's migration rows: `python tools/check-migration.py --report` (loop-goal.md § 14).
- `router.rs:39`'s *Known gaps* 1 and 2 are stale now the route table is built.
- A `bytes` array key ICEs in `nvs-ir`; `catch (Core\Error $e)` panics (playbook, *Writing a test
  case*).
