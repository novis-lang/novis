# Handoff

## State

Goal `lang-classes` is walking its item list: 15 features under `docs/reference/lang/50-classes.md`
owe feature proofs, and 8 are complete — `comparable`, `constants-and-class`, `declaring-a-class`,
`properties`, `methods-self-static-and-parent`, `inheritance`, `interfaces` and `property-hooks`.
Nothing is blocked.

Each of the three features this session took was given its tests by `covers:` markers on cases the
corpus already held, plus one new case where a claim in the chapter had no case at all: `E0410` (a
subclass constructor skipping `parent::constructor(...)` on one path) was pinned only by an
`nvs-types` unit test, an interface's constant and `static` had no case reaching them through every
route, and no case pinned a `set` hook standing alone without a `get`. Reach for a marker before
writing a case here, then look for the half of the section nothing asks about.

The three figures sit close together and are all call-bound: one virtual call through an abstract
base plus an `is` test is 19.2 ns (6.15 units, 1 call, 0 allocations); a call through an interface
into a default the interface wrote is 31.7 ns (10.3 units, 2 calls); a hooked write plus a computed
read is 27.4 ns (9.09 units, 4 calls, 0 allocations). A property hook costs a call per read and per
write, and a computed property that reads another hooked property pays for both.

## Next group

**The compiler-declared interfaces — three adjacent sections of one chapter.** One file set:
`docs/reference/lang/50-classes.md`, `docs/examples/lang/classes/<slug>/`,
`tests/hostile/lang/classes/<slug>/`, `benches/members/lang/classes/<slug>.nvs` and
`tests/conformance/class/`. One slice is one feature with all of its feature proofs
(`rule:testing/one-slice-is-one-feature`), and `python tools/dossier.py --id '<feature>'` prints the
path of each.

- [ ] **`lang:classes/propertyobserver`** — owes examples, hostile, perf, tests.
      `rule:classes/property-observer` is what the section is written around; the corpus already
      holds `tests/conformance/class/a-property-observer-sees-every-write-its-class-makes.nvst`.
      `docs/reference/lang/50-classes.md:935`
- [ ] **`lang:classes/stringable`** — owes examples, hostile, perf, tests. The corpus holds
      `a-stringable-class-renders-through-tostring.nvst` and
      `a-stringable-object-stringifies-at-every-implicit-site.nvst`.
      `docs/reference/lang/50-classes.md:969`
- [ ] **`lang:classes/parses`** — owes examples, hostile, perf, tests. `Parses` is what a route
      capture, a `#[Core\Query]` parameter and a command argument are built through, so its examples
      have real work to show. `docs/reference/lang/50-classes.md:1052`

## Backlog

- `lang:classes/objects-are-handles`, `nullable-objects-and`, `object-the-top-of-every-class-type`
  and `what-a-class-cannot-declare` are the four features left after the group above —
  `docs/reference/lang/50-classes.md:1123`, `:1163`, `:1204`, `:1249`.
- `nvs-fmt` de-indents the statements inside a property hook's `set (T $v) { … }` body by four,
  against the chapter's own examples — found by this session's hooks attack, fixable in
  `crates/nvs-fmt` with a `.nvs` case beside it.
- The `lang:classes` perf figures are keyed to `docs/reference/lang/50-classes.md`, so an edit to
  that chapter makes every figure in the group stale at once — `python tools/dossier.py
  --record-perf --group lang:classes` is the one call that clears it.
