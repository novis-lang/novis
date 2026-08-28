# Handoff

## State

**M4's Stage 8, depth.** The tree is at **835 conformance plus 189 differential**, all green.
Nothing is blocked.

`Core\Str`'s three floor-1 members each gained a case over `crates/nvs-stdlib/src/str.rs`'s own
doc comments. All three take the *invariance over a sweep* shape — one question asked of a whole
table and counted, so a member answering plausibly row by row still fails.

- **`replaceAll` is one pass, so the pairs' order is unobservable.** The cycle
  `a→b→c→a` built as all six insertion orders over four subjects: 24 of 24 answers agree, and
  0 of the 24 sequential folds `Core\Str::replace` gives agree with them — the `strtr`/`str_replace`
  split the doc comment argues, counted rather than shown. Plus the output-side reading of the
  same rule (a `c` in the output of `["a"=>"b","b"=>"c"]` comes only from a `b` or a `c` in the
  subject) and `replace`/`replaceAll` agreeing on all ten single-pair rows.
- **`reverse` is an involution that mirrors exactly the units `length` counts.** 14 subjects,
  14 double-reversals, 14 length agreements, and all 35 units in their mirrored place in
  `graphemes`' partition. Then `reverse($a . $b) == reverse($b) . reverse($a)` counted *against*
  `length($a . $b) == length($a) + length($b)` over 49 ordered pairs: 45 and 44, agreeing on 48.
  The single row where they part is a combining mark after a combining mark — one cluster, so
  not additive, but symmetric, so the law survives it.
- **`before`/`after` cut at the position `indexOf`/`lastIndexOf` report.** Twelve rows: the two
  cut members equal `slice($s, 0, $at)` and `slice($s, $at + length($n))` on all ten present rows,
  under both `{last: true}` and the default; 20 reconstructions put the needle back and get the
  subject; both absences are `null` from all three members. "First" and "last" are then said
  without a position — no occurrence in the head, none in the tail — which holds on nine of ten,
  the empty needle being the row that occurs everywhere including in the prefix it cuts.

`orient.py`'s pack was complete for this group; nothing outside it was read.

The gap fifteen handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s is
`E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**Whatever `python tools/gaps.py` now ranks at floor 1** — `Core\Str` is off the floor with these
three, so the next session runs the tool first and takes its top class rather than a name written
here. One shared file set per group: the class's own module in `crates/nvs-stdlib/src/` plus
`tests/conformance/core/`.

- [ ] **Run `python tools/gaps.py` and take the top class's thinnest member**, reading that
      member's doc comment for the rule no case observes — the shape that has worked three
      sessions running is *invariance over a sweep, counted*.
- [ ] **A second member of the same class**, so the group shares one file set and one build.
- [ ] **A third if the context gate still allows it** — a case over landed work costs a fraction
      of a lowering slice, which is what the 120k gate is counting.

## Backlog
- No `Core` class reaches `nvs_hir::implements_interface`, so `Core\Uri` cannot be compared with
  `<` — `docs/agent/loop-goal.md`, a session of its own.
- `docs/agent/guard-name-debt.md`: 54 of 156 guard names match nothing `cargo test` runs.
- `array<T> as array<U>` does not lower (`crates/nvs-ir/src/lower/expr.rs:877`), which is what
  keeps `Core\Csv::format`'s non-`string` cell refusal unreachable — playbook, *Divergences*.
- ADR 0028 § 2's abandoned-generator `finally`, still unlanded — `docs/agent/loop-goal.md`.
