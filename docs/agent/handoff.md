# Handoff

## State

**`Core\Math`'s conformance floor is closed.** Three depth cases landed over
`crates/nvs-stdlib/src/math.rs`'s circular, rounding and base members; `python tools/gaps.py`
now puts the class at depth **5.0**, floor **3** (it was 5.0 / 2, and was the ranked-first
floor-2 class). The corpus is at **924**, all passing.

**The Stage 8 acceptance check still fails at 924 of 950 wanted.** That is a growth floor,
not a regression — no case fails and no check names a test that disappeared. It closes as
the suite grows; the fastest route is the next group below, then the rest of `gaps.py`'s
floor-3 head.

**What the three cases pin**, all in `tests/conformance/core/`:

- `...-recovers-the-angle-in-all-four-quadrants-...nvst` — `atan2` is the inverse of the
  pair (`sin`, `cos`) taken together, over two full turns at eighth-turn steps: 33 rows, 17
  recovered as themselves and 16 a whole turn away, all 33 inside `[-PI, PI]`, and the four
  quadrants counted so the recovery is known to be earned in all of them. Its second half is
  the boundary the sweep never reaches — a sweep crosses an axis with a *tiny signed* leg
  (`zeroLeg=1`), so the eight exactly-signed-zero axis arguments are a separate table, and
  three of their 28 pairs agree. The case beside it owns `atan2` versus `atan`.
- `...-rounding-members-are-one-ordering-...nvst` — `floor <= x <= ceil` with the two ends
  never more than one apart, `truncate` the end nearer zero in both signs, `round` always an
  end and never further than half. Its point is the partition 7+7+14 over 28 rows: the seven
  exact halves are the only rows the ordering leaves undetermined, which is what hands the
  question to the mode case beside it.
- `...-round-trip-over-every-base-the-pair-accepts.nvst` — 11 values × 35 bases = 385 cells,
  each round-tripping, reading back upper-cased, carrying its sign in the writing and never
  padded; plus 374 consecutive-base pairs where a wider base is never a longer writing.
  `Core\Math::INT_MIN` is the row that earns the table: its magnitude does not fit a positive
  `int`, so a member that negates before writing fails at every base.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at
`crates/nvs-stdlib/src/registry.rs:1526`), a `bytes` array key ICE in `nvs-ir`, and
`catch (Core\Error $e)` panicking — the last two have playbook bullets under
*Writing a test case*.

**Orientation gaps.** `[context] modules` owes `crates/nvs-stdlib/src/math.rs` (this
session's file, still absent from the map) and now owes nothing new for the next group —
`src/regex.rs` is already in it. Still owed: `crates/nvs-stdlib/src/hash.rs`, `src/test.rs`,
`crates/nvs-types/src/links.rs`, `src/routes.rs`, `crates/nvs-stdlib/src/validate.rs`,
`src/str.rs`, `docs/spec/01-core-library.md`, `docs/spec/02-php-migration.md`,
`tools/check-migration.py`, the stage-7 comment header's per-goal floor table, a selector
printing the *failing* check's own `cases` block, and a `[context] anchors` entry for
`registry.rs`'s `UNCLASSIFIED`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/regex.rs` and `tests/conformance/core/`.
`gaps.py` ranks `Core\Regex` first — depth **5.0**, floor **2**, 27 cases over 8 members —
and its three thinnest are one file and one domain. The differential gap is empty, so these
are conformance-depth cases, never `--ORACLE--` ones (conventions.md). `quote` is on
`conformance_coverage.rs`'s `BELOW_THE_FLOOR` worklist, whose whole remaining content is
`Core\Regex::quote`, `Core\Router::urlAbsolute` and `Core\Time\TimeOfDay::compareTo` —
**delete the line in the same commit as the third case**, per the new playbook bullet.

- [ ] **`Core\Regex::quote` makes any string match itself and nothing else**
      (`crates/nvs-stdlib/src/regex.rs:1305`) — a table of subjects containing every
      metacharacter, each quoted into a pattern, counting the rows that match themselves
      exactly once and the rows where the unquoted spelling would have matched something
      else or failed to compile at all.
- [ ] **`Core\Regex::replaceWith`'s callback sees every match and only the matches**
      (`crates/nvs-stdlib/src/regex.rs:1163`) — the callback's answers reassembled against
      the subject, counting that the untouched spans plus the replacements are the subject
      back, and that a `limit` stops the callback rather than discarding its work.
- [ ] **`Core\Regex::split` and `Core\Str::join` are inverses wherever the pattern matches
      no empty span** (`crates/nvs-stdlib/src/regex.rs:1253`) — a table of subjects and
      patterns, counting the rows the join rebuilds, and the `limit` bound named on both
      sides.

## Backlog

- `Core\Arr` floor 3 — `column`, `fillKeys`, `firstKey` (`gaps.py`, 224 cases / 55 members).
- `Core\Time\DateTime` floor 3 — `isLeapYear`, `dayOfYear`, `timeOfDay` (`gaps.py`).
- `Core\Bytes` floor 3 — `unpack`, `at`, `repeat` (`gaps.py`).
- Item 12's 10 `UNCLASSIFIED` members, `crates/nvs-stdlib/src/registry.rs:1526` (ADR 0088 § 2).
- 67 unasserted error paths, 65 of them `Fault::fatal` (`python tools/gaps.py --errors`).
- `catch (Core\Error $e)` is an ICE, not a diagnostic (`docs/agent/playbook.md`).
