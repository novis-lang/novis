# Handoff

## State

**M4's Stage 8, depth.** The tree is at **788 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Regex\Match` left the
frontier this session (median 2.5 → 4.5, floor 1 → 2). The thinnest classes are now
`Core\Uri` and `Core\Regex`, both at median 3.0 with a floor of 1. Nothing is blocked.

- **`regex-match-offset-is-a-grapheme-index-from-zero-to-the-subject-length.nvst`** settles
  the question one case could not ask: the unit. A ZWJ family sequence is one grapheme, five
  code points and eighteen octets, so `x1` after one reports `1` and neither `5` nor
  `PREG_OFFSET_CAPTURE`'s `18`; a combining accent and a `\r\n` pair are the same fact in
  everyday shapes. The ceiling is named on both sides — the last non-empty match of `"abc"`
  is at `2`, the zero-width `$` reaches `3` which is the subject's own length, a search from
  `3` finds nothing and the empty pattern still matches exactly there, so `matchAll` over the
  empty pattern walks `0 1 2 3`. A seven-match sweep then counts the invariant that only
  holds under a grapheme index: `Core\Str::at($subject, $m->offset())` is the match's own
  first character.
- **`regex-match-groups-is-every-declared-group-counted.nvst`** makes the count a function of
  the pattern alone — `1` plus one per numbered group plus one per name — and asserts the
  tally over six patterns rather than any row, so PHP's default trimming of trailing
  unmatched groups would fail here while every printed line still looked right. A
  non-capturing group moves no number, an unreached optional group is a present `null`, and a
  match with no groups is one entry rather than an empty array.
- **`regex-match-group-agrees-with-groups-over-both-spellings-of-a-key.nvst`** asks
  `group()` every key `groups()` listed and counts the agreement, then pins `group_key`'s own
  rule: `2` and `"2"` are one key on the rows that throw as much as on the rows that answer,
  while `"02"`, `"KIND"`, `""` and `-1` are undeclared keys. The bound is named on both sides
  — `2` answers and `3` throws, quoting "declares no group `3`".

## Next group

**`Core\Regex`'s own floor: `quote`, `replaceWith` and `compile`** — the file set is
`crates/nvs-stdlib/src/regex.rs` plus `tests/conformance/core/`, the same file this session
worked in. The three registry rows are at `regex.rs:171`, `:148` and `:104`.

- [ ] **`quote`'s edges** (`regex.rs:171` the row, `:1258` the implementation) — one case on
      disk. The boundary is which characters it escapes and which it leaves: the round trip
      `Core\Regex::matches($s, Core\Regex::quote($s))` over a table of metacharacter-bearing
      subjects is the invariant, counted, and an empty subject plus a subject that is nothing
      but metacharacters are the edges.
- [ ] **`replaceWith`'s callback contract** (`regex.rs:148`, `:1116`) — one case. What the
      callback is handed is a `Match`, so the three members above are readable from inside
      it; no match at all, a callback returning the empty string, and a `{limit:}` at `0` and
      at one below the match count are the edges.
- [ ] **`compile` answers a `Pattern` the other members take** (`regex.rs:104`, `:693`) —
      three cases. The agreement shape: every member taking `Pattern|string` answers the same
      thing for a compiled pattern as for its source string, counted over a sweep.

## Backlog

- `Core\Uri`'s floor — `buildQuery` 1, `compareTo` 1, `decodeComponent` 2
  (`crates/nvs-stdlib/src/uri.rs`); `compareTo` is content equality, and ADR 0090 § 4 forbids
  `==` on two objects, so the spelling is `$a->compareTo($b) == 0`.
- `Core\Time\Zone`'s floor — `system` 1, `fixed` 2, `offsetAt` 4 (`docs/agent/loop-goal.md`).
- `Core\Str`'s floor — `fold` 1, `graphemes` 1, `indexOf` 1; the Windows `php` has no
  `mbstring`, so the Unicode rows cite the UCD table rather than an oracle.
- `matchAll` converts each offset over the whole prefix, so k matches in an n-byte subject is
  O(n·k) — `crates/nvs-stdlib/src/regex.rs`'s module doc gap 3 owns the cursor that fixes it.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
