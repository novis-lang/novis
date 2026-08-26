# Handoff

## State

**Stage 4's two counts are the frontier — conformance 480 of 600, differential 90 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **480 passed, 0 failed** and `mwl test tests/` is **570 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook, twice),
and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than it (playbook,
*Running things*).

**The depth pass has reached every section**, so what matters from here is the *shape* a new case takes,
not which section it lands in; the four shapes are named in the plan's *Open now*. **A pass must land its
claim as a new case file** — the gate counts files, and this session's two slices each split into
"the thin case, unchanged" plus "the deep claim, named for itself" for exactly that reason (playbook,
*Writing a test case*).

Three spellings a case cannot use: a closure called through the variable holding it, the first-class
callable `Class::method(...)` (both `mwl-ir` gap 1 — declare a `class` with a `public static function`
and call it *directly*, which every `arr` depth case does), and `bool as int`, which panics `mwl-ir`;
`bool as string` lowers but renders `false` as nothing at all, so branch to a character instead.

## Next group

All three share `crates/mwl-stdlib/src/arr.rs` and `tests/conformance/core/`, and the second reuses the
fourth shape. Take them in this order.

- [ ] **The numeric folds at their edges** — `min` (`arr.rs:4021`), `max` (`arr.rs:4033`), `sum`
      (`arr.rs:4046`), `product` (`arr.rs:4057`), `average` (`arr.rs:4076`), spec § 2: the empty subject
      is where they part company — `sum` and `product` have identities (`0`, `1`) and the other three do
      not — and a one-element subject is the last input on which all five agree. `min`/`max` also owe
      their agreement with `sort`'s first and last entry over the same subject.
      `arr-sum-product-and-average-over-numbers.mwlt`, `arr-min-and-max-agree-with-sort.mwlt` and
      `arr-members-over-an-empty-array.mwlt` are what exist; the new claim is its own file.
- [ ] **The search members answer one question** — `find` (`arr.rs:3337`), `findKey` (`arr.rs:3353`),
      `any` (`arr.rs:3367`), `contains` (`arr.rs:3405`) and `keyOf` (`arr.rs:3424`), spec § 2 and ADR
      0090 § 3: "is there an entry equal to this, and where" asked five ways over one table, so
      `contains` is `keyOf(..) != null` is `find(..) != null` is `any(..)` is `findKey(..) != null`, all
      five say no over an empty subject, and each names the *first* match when the subject holds two.
      The identity table case already pins `contains`'s comparison; this pins the five agreeing.
- [ ] **`isList` and the members that answer one** — `isList` (`arr.rs:1431`), `values`
      (`arr.rs:2231`), `keys` (`arr.rs:1488`), `flip` (`arr.rs:2039`), `chunk` (`arr.rs:1704`),
      `flatten` (`arr.rs:2084`), `appendAll` (`arr.rs:3757`), ADR 0007 § 5: a list is exactly the keys
      `"0" … "n−1"` in order, which is also `Core\Json::encode`'s array test — so the members that
      renumber answer a list from any subject and the members that preserve keys answer one only when
      they were given one.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `mwl_stdlib::json` gap 2, and ADR 0071's non-scalar fields.
- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows — `mwl_stdlib::hash`'s
  module doc.
- ADR 0086 § 1's substitution table for the terminal sink — `crates/mwl-stdlib/src/cli.rs` gap 1.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `orient.py` did not print ADR 0069 § 5, ADR 0007 § 5 or any of `docs/spec/01-core-library.md`, all
  three of which this session's slices are specified by; `[context] adrs` needs `0069:5` and `0007:5`,
  and `[context]` has no selector for the spec at all.
