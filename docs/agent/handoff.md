# Handoff

## State

**M4's Stage 8, depth.** The tree is at **832 conformance plus 189 differential**, all green.
Nothing is blocked.

`Core\Str` gained three cases over `crates/nvs-stdlib/src/str.rs`'s own doc comments, one per member
of the previous handoff's group. All three take the *invariance-over-a-sweep* or *agreement* shape:
one question asked of a whole table and **counted**, so a member answering plausibly row by row
still fails.

- **`lines` answers the same three lines whichever terminator spells them.** Four subjects with an
  interior empty line, four with a trailing break, three that are breaks and nothing else: every row
  re-joins with `"\n"` to one canonical subject, and the clusters the breaks cost is the *same* per
  spelling, which is GB3's "`\r\n` is one cluster" made countable — were it two, the `\r\n` rows
  would answer one more. `length("a\r\nb")` is 3 and `length("a\n\rb")` is 4 reads the same rule off
  `length` directly.
- **`lowerFirst`/`upperFirst` are `lower`/`upper` of the first character glued to the rest.** Over
  ten subjects both members equal `case(slice($s,0,1)) . slice($s,1)` on all ten, including `ǅ` and
  a decomposed `e`+acute; they agree with the *whole-string* member on only nine (`lower`) and two
  (`upper`), which is what a `lowerFirst` that had become `lower` would fail. `ß` is the one row
  where `upperFirst` expands and the one where `lowerFirst(upperFirst($s))` parts from `lowerFirst`.
- **`normalize`'s four forms over nine subjects**: 36 idempotence checks, 18 for the compatibility
  axis absorbing the canonical one, 18 for the canonical pair being lossless both ways, 9 for
  `Nfkc == Nfc(Nfkd(…))` — and `Nfc(Nfkc($s)) == Nfc($s)` on only **five** of the nine, which is
  "only the canonical pair round-trips" counted rather than shown. Plus 16 checks that the ASCII
  fast path is the identity for all four forms.

`orient.py`'s pack was complete for this item; nothing outside it was read.

The gap fifteen handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s is
`E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**`Core\Str`'s last three floor-1 members** — `gaps.py` still ranks the class at floor 1 with
`replaceAll 1, reverse 1, after 2`. Same file set as this session: `crates/nvs-stdlib/src/str.rs` and
`tests/conformance/core/`. Read the member's doc comment first and look for the rule stated there
that no case observes.

- [ ] **`replaceAll` against the one-pass rule it is written around**
      (`crates/nvs-stdlib/src/str.rs:1302`) — a table of pairs where one replacement's *output*
      contains another's *needle*, asserting the result is invariant under the pair order and that no
      replacement is applied to text a previous one produced. `replace` and `replaceAll` agreeing on
      a single-pair table is the second half.
- [ ] **`reverse` as an involution over the unit `length` counts**
      (`crates/nvs-stdlib/src/str.rs:1908`) — `reverse(reverse($s)) == $s` and
      `length(reverse($s)) == length($s)` over a table carrying a multi-code-point cluster, plus
      `graphemes(reverse($s))` being `graphemes($s)` back to front, which is what a byte-wise or
      code-point-wise reverse fails.
- [ ] **`after` against `before`, `indexOf` and `slice`**
      (`crates/nvs-stdlib/src/str.rs:1883`) — `before($s,$n) . $n . after($s,$n) == $s` on every row
      where `$n` occurs, and the miss answer named on both sides; the position `after` starts at is
      the grapheme index `indexOf` reports, so an ASCII-only table cannot tell them apart.

## Backlog

- No `Core` class implements a `Core` interface, so `Comparable` over `Core\Uri` is `E0411` — its own
  session; `docs/agent/handoff.md` has carried this since M4 Stage 6.
- `Core\Debug` is the thinnest class by median (3.5) and the only one outside `Core\Str` worth a
  group — `dump 2, render 5` (`python tools/gaps.py`).
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `array<T> as array<U>` does not lower, which is what leaves `Core\Csv::format`'s column refusal
  unreachable from source — `docs/agent/playbook.md` § *Writing a test case*.
- ADR 0007 § 2's `.` over a `Ty::Tagged` operand still reaches `nvs-codegen`'s mismatched-representation
  refusal with no diagnostic naming it — `docs/agent/playbook.md` § *Writing Novis itself*.
