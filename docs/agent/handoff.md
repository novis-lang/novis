# Handoff

## State

**M4's Stage 8, depth.** The tree is at **829 conformance plus 189 differential**, all green.
Nothing is blocked.

`Core\Str` gained three cases over `crates/nvs-stdlib/src/str.rs`'s own doc comments, one per member
of the previous handoff's group. All three take the *agreement* or *invariance-over-a-sweep* shape:
one question asked of a whole table and **counted**, so a member that answered plausibly row by row
still fails.

- **`fold` and `lower` are asked "is your answer invariant under uppercasing" of eight rows.**
  `fold` is on all eight, `lower` on five — `ß`, `ſ` and `ﬁ` are the three where a `lower`-keyed
  table loses a case pairing. The two members answer the same string on only three of the eight, so
  the counts are not read off a table they agree on, and the asymmetry is counted both ways:
  `fold(lower($s)) == fold($s)` on all eight, `lower(fold($s)) == lower($s)` on three.
- **`graphemes` is the partition `length` counts and `codePoints` refines.** Over nine subjects, all
  four invariants hold on every row (count == `length`, `join(…, "")` is the subject, every piece is
  one unit long, and re-splitting the pieces into code points reaches the subject's own total);
  the unit is strictly coarser than a code point on five of the nine, which is what a `graphemes`
  that had quietly become `codePoints` would fail.
- **`indexOf` agrees with `contains`, `countOf` and `lastIndexOf` about occurrence** on all nine
  rows, and its position is a *grapheme* index on all six that occur — `slice($h, $at, length($n))`
  is the needle, `{from: $at}` answers `$at` again, and `{from: $at + 1}` is null or strictly later.
  A byte offset passes the ASCII rows and fails `café au lait` and `👍🏽 hello 👍🏽`.

`orient.py`'s two `!!` lines are **closed**: `[context] playbook` in `docs/agent/loop-goal.toml` had
`"Writing a test case > nvs-codegen has"` (a duplicate of the `> a row` entry beside it, dropped) and
`"Writing a test case > emitbinop's ordering"` (the bullet now leads with `emit_binop`'s *`integral`
set*, re-pointed).

The gap fourteen handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s is
`E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**`Core\Str`'s next three floor-1 members** — `gaps.py` still ranks the class at floor 1, with three
different members at the bottom now. Same file set as this session: `crates/nvs-stdlib/src/str.rs`
and `tests/conformance/core/`. Read the member's doc comment first and look for the rule stated there
that no case observes.

- [ ] **`lines` against the boundary rule it shares with `graphemes`**
      (`crates/nvs-stdlib/src/str.rs:1018`, helper at `:1046`) — `line_pieces`'s own doc makes three
      claims no case counts: a trailing break does not add an empty line, an interior empty line is
      one, and `\r\n` is consumed whole under GB3 while a lone `\r` is its own cluster. The sweep
      shape is `join(lines($s), "\n")` against the subject over a table of break spellings.
- [ ] **`lowerFirst` against `upperFirst` and `lower`** (`crates/nvs-stdlib/src/str.rs:2190`,
      `upperFirst` at `:2183`) — `map_first` maps *one grapheme* and leaves the rest, so the
      agreement to count is `lowerFirst($s) == lower(at($s, 0)) . slice($s, 1)` over a table whose
      first cluster is a titlecase letter (`ǅ`), a combining pair and a non-cased character.
- [ ] **`normalize`'s four forms over two independent axes**
      (`crates/nvs-stdlib/src/str.rs:2245`) — the doc says one member with an enum rather than four
      members *because* the axes are independent; the invariants are idempotence of each form, that
      NFC and NFD round-trip, and that `length` is unchanged by normalization while
      `Core\Arr::count(codePoints(…))` is not.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `Core\Uri::compareTo` exists while
  `$a < $b` over two `Uri`s is `E0411` — `docs/agent/loop-goal.md`, a session of its own.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `Core\Debug` is `gaps.py`'s thinnest class (depth 3.5, floor 2) but only two members wide —
  `crates/nvs-stdlib/src/debug.rs`, one slice not three.
- 68 unasserted error paths, most `Fault::fatal` and unreachable from source — `gaps.py --errors`,
  judged one at a time.
- `array<T> as array<U>` does not lower (`crates/nvs-ir/src/lower/expr.rs:877`), which is what keeps
  `Core\Csv::format`'s column refusal unreachable — ADR 0007 § 2.
