# Handoff

## State

**M4's Stage 8, depth.** The tree is at **782 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Bytes` left the frontier
this session (median 2 → 3, floor 2 → 3 across `at`, `compare` and `contains`). The thinnest
class is now `Core\Test` (median 2.0, floor 2), then `Core\Regex\Match` (2.5, floor 1).
Nothing is blocked.

- **`Core\Bytes::at`'s bound was already pinned on both sides** when this session opened it —
  see the playbook bullet added under *Writing a test case*. What it was genuinely missing was
  the index *type*'s two ends, so `bytes-reads-name-the-octet-they-stop-at.nvst` gained two
  rows: `i64::MAX` addresses nothing, and `i64::MIN` cannot be added to the length at all
  (`bytes.rs:386`'s `checked_add`), and both refuse rather than wrapping back into range. The
  low one is computed as `0 - $most - 1` because a literal past `int` is a different refusal
  that would hide this one.
- **`bytes-compare-is-str-compare-over-octets-and-a-total-order.nvst`** asks an eight-entry
  table, written in ascending byte order, four questions by counting: every pair agrees with
  `Core\Str::compare` over the same octets (64), every pair answers the sign of its two indices
  with equality only on the diagonal (64, 8 equal), swapping the arguments negates the answer
  (64), and `<=` is transitive over every triple (512). The table carries the empty buffer, a
  proper-prefix chain, and `"ß"` — 0xc3 0x9f, which sorts *after* `"z"` only if the octets are
  read unsigned. A fifth block orders 0x00 and 0xff, which no `string` can name.
- **`bytes-contains-agrees-with-index-of-over-every-needle.nvst`** sweeps all 78 substrings of
  `"abracadabra"` (both degenerate lengths included) and the same 78 with an octet the subject
  does not hold appended: `contains` and `indexOf` agree about presence on all 156, every
  substring is located at or before the offset it was cut from (the repeated `"abra"` run is
  what makes that a question), and a needle longer than the subject is never contained. 0x00
  is an ordinary needle, where a C-string search would stop at it.

## Next group

**`Core\Test`'s floor: `assertNull`, `assertCount` and `assertDoesNotThrow`** — the file set is
`crates/nvs-stdlib/src/test.rs` plus `tests/conformance/core/`. All three sit at two cases and
the class is the thinnest on disk. Read the three registry rows together at `test.rs:165`,
`:172` and `:190` before taking the first: they share the pass/fail reporting path, so one case
per member can reuse the same rendering helper, and `held`/`failed` at `test.rs:341` and `:523`
are where the two outcomes are actually produced.

- [ ] **`assertNull`'s edges, on both sides of "is null"** (`crates/nvs-stdlib/src/test.rs:165`
      the row, `:339` the implementation) — a `mixed` parameter, so the interesting rows are the
      values that are *not* null but are falsy or empty (`0`, `""`, `false`, `[]`, an enum case
      backed by `0` — ADR 0035 § 4), each of which must fail rather than pass, counted.
- [ ] **`assertCount` agrees with `Core\Arr::count` over a swept table** (`test.rs:172`, `:365`)
      — every array shape the class can build, asserted by counting agreements rather than read
      off a line, with the empty array and an off-by-one expectation each way included.
- [ ] **`assertDoesNotThrow` reports the throw it caught** (`test.rs:190`, `:505`) — a callback
      that returns, one that throws, and what the failure detail names; `Fault::` at the site
      decides whether the case can render the outcome inline.

## Backlog

- `Core\Regex\Match` is the next thinnest at median 2.5 with a floor of 1 (`offset` 1) — `python tools/gaps.py --coverage`.
- `Core\Uri::buildQuery`, `::compareTo` and `Core\Regex::quote`/`::replaceWith` each sit at one case — same tool.
- 54 of the 156 guard tests `loop-goal.toml` names still match nothing `cargo test` runs — `docs/agent/guard-name-debt.md`.
- ADR 0028 § 2's abandoned-generator `finally` is pre-authorized and unlanded — `docs/agent/loop-goal.md`.
- `orient.py`'s `[context] playbook` names two selectors under *Writing a test case* that match no bullet; it warned twice this session — `docs/agent/loop-goal.toml`.
