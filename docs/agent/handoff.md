# Handoff

## State

**M4's Stage 8, depth.** The tree is at **826 conformance plus 189 differential**, all green.
Nothing is blocked.

`Core\Arr` is off the floor — `python tools/gaps.py` ranks it at depth 5.0, **floor 2**, where it
opened the session at floor 1. Three cases landed over `crates/nvs-stdlib/src/arr.rs`'s own doc
comments, one per member of the previous handoff's group:

- **`flattenDeep` is `flatten` iterated to a fixed point.** Over a sweep of eight nestings, the
  deep answer is unchanged by one further `flatten` and by a second deep walk, and holds no
  nested array (`Core\Str::countOf($json, "[") == 1`); the fourth count says only three of the
  eight rows would have been answered by a single `flatten`, which is what keeps the other three
  counts from being vacuous. Where the iteration *stops* is a type, not a fault — see the new
  playbook bullet.
- **`column` appends under the next *free* integer**, not the next position: a row keyed `"5"`
  moves the counter past 5 and a later `"2"` does not move it back, so the appended keys are 6 and
  7. That exposed an overstatement in the member's own doc — an appended `0` collapses under a
  later row whose `indexBy` cell is `0`, so "one entry per matching row either way" is not
  unconditional. `arr.rs:2199`'s bullet now says both, and the case pins the collision.
- **`overlayDeep` differs from `overlay` only where both sides of a key hold a non-list array**,
  counted over the ten ways that condition can fail or hold — a list on either side, an empty
  array on either side, a scalar, an absent key, three levels down. Exactly two of ten differ.

The gap thirteen handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s
is `E0411`. It is in the backlog and still deserves a session of its own.

`orient.py`'s two `!!` lines are unchanged and still owed: `[context] playbook` in
`docs/agent/loop-goal.toml` names `"Writing a test case > nvs-codegen has"` and `"Writing a test
case > emitbinop's ordering"`, and no bullet under that heading leads with either — the two
selectors are stale and should be re-pointed or dropped.

## Next group

**`Core\Str`'s three floor-1 members** — `gaps.py`'s thinnest members anywhere in the corpus now,
one file set: `crates/nvs-stdlib/src/str.rs` and `tests/conformance/core/`. Each has exactly one
case, and over `Core\Arr`, `Core\Uuid`, `Core\Bytes` and `Core\Debug` alike the gap was never a
missing row — read the member's doc comment first and look for the rule stated there that no case
observes.

- [ ] **`fold` against `lower`, as the agreement they are not** (`crates/nvs-stdlib/src/str.rs:2214`)
      — case-folding is not lowercasing, and the *agreement* shape asks both of one table and
      counts where they part company (`ß`, `İ`, `ſ`) rather than reading one row off each.
- [ ] **`graphemes` against `codePoints` and `length`** (`crates/nvs-stdlib/src/str.rs:1084`) — the
      three ways to count a string, swept over one table so a cluster that a member splits
      differently fails the count rather than one line.
- [ ] **`indexOf`'s options and its `?uint` answer** (`crates/nvs-stdlib/src/str.rs:1538`) — four
      arguments, and the boundary shape: the last offset that finds and the first that does not,
      with `lastIndexOf` asked the same question.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `Core\Uri::compareTo` exists while
  `$a < $b` over two `Uri`s is `E0411` — `docs/agent/loop-goal.md`, a session of its own.
- `[context] playbook` in `docs/agent/loop-goal.toml` has two stale selectors (above).
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `Core\Debug` is the thinnest class at depth 3.5 (`dump` 2, `render` 5) — `python tools/gaps.py`.
- `array<T> as array<U>` does not lower, which is what makes `csv.rs:512`'s refusal unreachable —
  `docs/agent/playbook.md`.
- `Core\Math` at floor 2 (`atan2`, `ceil`, `floor`) is the next group after `Core\Str`.
