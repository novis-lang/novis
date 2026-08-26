# Handoff

## State

**Stage 4's two counts are the frontier — conformance 476 of 600, differential 90 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean)
and `mwl test tests/conformance` is **476 passed, 0 failed** — run it as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice).

**`verify.py` is deterministic again.** The uncaught-throw failure was a use-after-free, not a flake: a
compiled `Unit` owns the `Rc<ClassTable>` whose descriptor addresses are baked into its code, and
`mwl-codegen`'s test harness dropped the unit while the `Ctx` still held an exception built from one.
`mwl_codegen::Unit::install_in` is now the single spelling for handing a context that table, both
embedders call it, and its doc comment is the home of the obligation. Measured 28/40 failures before,
0/40 after; the WSL valgrind leg over `uncaught.mwl`/`throw.mwl`/`errors.mwl` is clean.

**The depth pass has reached every section.** `arr` was the last and its three slices are all in — the
window pair, `sort`, and now `unique`. The remaining count to 600 comes from members whose section has
already had a pass, so what matters from here is the *shape* a new case takes, not which section it
lands in.

**A fourth case shape is established and it is the one to reach for next**: *one question asked of
every member that shares it*, asserting their **agreement** rather than their answers.
`arr-unique-and-the-set-members-ask-one-identity-question.mwlt` puts `unique`, `contains`, `diff` and
`intersect` to all 29 rows of ADR 0090 § 3's identity table plus § 5's cross-type pairs, collapses each
row's four verdicts to one character, and asserts the tally of unanimous rows against the row count — so
a member that grew its own comparison still answers plausibly on its own line and fails here. The other
three shapes (a section's edges, invariance over a sweep, a bound on both sides) are in the plan's
*Open now*.

Two spellings a case cannot use, both now in *Open now*: a closure called through the variable holding
it (`mwl-ir` gap 1 — declare a `class` with a `public static function` in the case file instead, which
all three `arr` depth cases do), and `bool as int`, which panics `mwl-ir` outright — branch on the
predicate rather than counting it.

## Next group

All three share `crates/mwl-stdlib/src/arr.rs` and `tests/conformance/core/`, and all three reuse the
fourth shape above. Take them in this order.

- [ ] **The `Core\Arr` callback protocol, asked of every member that takes one** — spec § 2 and
      `arr.rs:875`'s own paragraph: every callback receives `($value, $key)` and
      `mwl_runtime::call_closure` trims the tail to what the closure declared, so one one-argument
      closure and one two-argument closure must give the same answer from every such member.
      Anchors: `arr.rs:887` `filter`, `arr.rs:979` `map`, `arr.rs:1139` `groupBy`, `arr.rs:2606`
      `reduce` (three args — the seed types the answer), `arr.rs:2827` `sort {by}`, `arr.rs:2851`
      `sort {comparator}`, `arr.rs:3462` `unique {by}`. Existing thin cases:
      `arr-map-and-filter-keep-their-keys.mwlt`, `arr-find-any-and-all-answer-from-the-first-match.mwlt`.
- [ ] **`Core\Arr`'s combination trio to depth** — ADR 0069 in full: `overlay` (`arr.rs:3690`),
      `underlay` (`arr.rs:3734`) and `appendAll` (`arr.rs:3757`) combine by the member's name and never
      by a key's type, so the claim to pin is that `overlay($a, $b)` and `underlay($b, $a)` are one
      answer over a table mixing integer-like and string keys, and that `appendAll` is the one that
      renumbers. `arr-combination-members-treat-every-key-alike.mwlt` is what exists.
- [ ] **`Core\Arr::reduce`'s seed** — `arr.rs:2606`, spec § 2. The seed types the answer and the fold
      is left-to-right over insertion order; the edges are an empty subject (the seed, untouched) and a
      seed of a different type from the elements. `arr-reduce-folds-from-a-seed-that-types-the-answer.mwlt`
      is the thin case to deepen.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `mwl_stdlib::json` gap 2, and ADR 0071's non-scalar fields.
- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows — `mwl_stdlib::hash`'s
  module doc.
- ADR 0086 § 1's substitution table for the terminal sink — `crates/mwl-stdlib/src/cli.rs` gap 1.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- `bool as int` does not lower — `crates/mwl-ir/src/lower/expr.rs:877`'s own message lists what does.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
