# Handoff

## State

**M4's Stage 8, depth.** The tree is at **785 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Test` left the frontier
this session (median 2.0 → 3.0), and its floor of 2 is now `assertThrows` alone. The thinnest
class is now `Core\Regex\Match` (median 2.5, floor 1). Nothing is blocked.

- **`test-assert-null-refuses-every-value-that-is-not-null.nvst`** separates ADR 0035 § 2's
  falsy table from "is null" by running both sweeps and counting: nine falsy-but-not-null
  subjects (both integer zeroes, both float zeroes, `""`, `"0"`, `false`, `[]`, an enum case
  backed by `0`) all *fail*, six truthy ones fail identically, three spellings of `null` hold,
  and all eighteen agree with `assertSame($x, null)`. The last block pins what the report cannot
  say: `int` 0, `uint` 0, `0.0` and the enum case all render as `0`.
- **`test-assert-count-agrees-with-arr-count-over-every-length.nvst`** asks a five-length by
  six-expectation grid, so `held` falls on the diagonal and nowhere else — which is the bound
  asserted on both sides once per length. Duplicates, a string-keyed map and a nested
  `array<array<string>>` all count entries; an `unset` moves the expectation that holds; and
  `18446744073709551615` is an ordinary failure with both sides named.
- **`test-assert-does-not-throw-quotes-the-throw-it-caught.nvst`** quotes four throw shapes (a
  throw three frames down, an empty message, `/ 0`'s `ArithmeticError`, and a failed assertion's
  own report), then pins the half a report cannot show: the propagated class is
  `Core\Test\Failure` and not the body's — the two `catch` clauses are ordered so the body's
  `RuntimeError` would bind first if it were being passed through — and the next two assertions
  hold rather than inheriting the pending throw. A six-body table then agrees with a hand-written
  `try` around the same `callable`.

## Next group

**`Core\Regex\Match`'s floor: `offset`, `groups` and `group`** — the file set is
`crates/nvs-stdlib/src/regex.rs` plus `tests/conformance/core/`. The three registry rows are
together at `regex.rs:310`, `:317` and `:324`, and the class's own slot list is at `:338`, so
read those five lines once before taking the first slice: all three members read the same two
slots off the same instance, which is why one file set covers the group.

- [ ] **`offset`'s edges** (`regex.rs:324` the row, `:985` the implementation) — it sits at one
      case, the fewest of any member on disk. A match at offset 0, a match at the very end of the
      subject, an empty match, and a multi-byte subject are the boundaries: whether the number is
      octets or characters is the question a single case never asks.
- [ ] **`groups` is the whole capture list, counted** (`regex.rs:317`, `:966`) — a pattern with
      no groups, one with an unmatched optional group, and one with more groups than the subject
      filled; sweep a table and count rather than reading one line.
- [ ] **`group` agrees with `groups` over every key** (`regex.rs:310`, `:929`, and `group_key`
      just above it) — an agreement case: every index and every name `groups` answers, asked of
      `group`, plus the key that names nothing.

## Backlog

- **An enum case erased to `mixed` reads falsy in a condition when it is backed by `0`**, which
  ADR 0035 § 2's own table row and § 4 say is truthy — the tag is all the runtime helper has, and
  ADR 0047 § 5 is why. Unpinned either way; ADR 0035's M3 verification bullet names this exact
  row. Owner: `docs/adr/0035-truthy-boolean-context.md` § 4.
- `Core\Test::assertThrows` is the class's remaining floor at two cases — `python tools/gaps.py --coverage`.
- `Core\Uri::buildQuery`, `::compareTo` and `Core\Regex::quote`/`::replaceWith` each sit at one case — same tool.
- 54 of the 156 guard tests `loop-goal.toml` names still match nothing `cargo test` runs — `docs/agent/guard-name-debt.md`.
- ADR 0028 § 2's abandoned-generator `finally` is pre-authorized and unlanded — `docs/agent/loop-goal.md`.
- `orient.py`'s `[context] playbook` names two selectors under *Writing a test case* that match no bullet; it warned twice this session — `docs/agent/loop-goal.toml`.
