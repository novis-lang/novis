# Handoff

## State

**Stage 4's two counts are the frontier — conformance 478 of 600, differential 90 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **478 passed, 0 failed** and `mwl test tests/` is **568 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook, twice),
and **rebuild `target/release/mwl.exe` first**, because the one on disk is the previous session's and
made a passing case look broken (playbook, *Running things*).

**The depth pass has reached every section**, so what matters from here is the *shape* a new case takes,
not which section it lands in. Four shapes are established and named in the plan's *Open now*: a
section's edges, invariance over a sweep, a bound on both sides, and — the one with the most room left —
**one question asked of every member that shares it, asserting their agreement rather than their
answers**. That fourth shape now carries three cases, all in `Core\Arr`: the identity table
(`arr-unique-and-the-set-members-ask-one-identity-question.mwlt`), spec § 2's callback protocol
(`arr-every-callback-is-offered-value-and-key.mwlt` — fourteen members asked their own question from a
one-parameter and a two-parameter closure, plus nine reports of the keys each was offered), and ADR 0069
§ 1's key-order rule (`arr-combination-members-share-one-key-order-rule.mwlt` — eight tables of mixed
key shapes, five claims per row). Each collapses a row to one character and asserts the tally against
the row count, so a member answering from its own rule still reads plausibly on its own line and fails
there.

Two spellings a case still cannot use: a closure called through the variable holding it (`mwl-ir` gap 1
— declare a `class` with a `public static function` in the case file instead, which every `arr` depth
case does), and `bool as int`, which panics `mwl-ir` outright — branch on the predicate rather than
counting it.

## Next group

All three share `crates/mwl-stdlib/src/arr.rs` and `tests/conformance/core/`, and the first two reuse
the fourth shape above. Take them in this order.

- [ ] **`Core\Arr::reduce`'s seed** — `arr.rs:2606`, spec § 2. The seed types the answer and the fold is
      left-to-right over insertion order; the edges are an empty subject (the seed, untouched, with no
      call made) and a seed of a different type from the elements. The arity half of `reduce` is already
      pinned by the callback case above, so this one is about the *carry*.
      `arr-reduce-folds-from-a-seed-that-types-the-answer.mwlt` is the thin case to deepen.
- [ ] **Every member that answers a key answers a `string`** — ADR 0069 § 5 and ADR 0007 § 5, one
      question asked of all seven: `keys` (`arr.rs:1488`), `flip` (`arr.rs:2039`), `firstKey`
      (`arr.rs:3298`), `lastKey` (`arr.rs:3313`), `findKey` (`arr.rs:3353`), `keyOf` (`arr.rs:3424`) all
      return `string`, while `hasKey` (`arr.rs:1407`) *takes* `int|string` and normalises it, so `$a[8]`
      and `$a["8"]` are one key from every direction. Sweep a table whose keys are integers, integer-like
      strings and non-numeric strings and assert the seven agree.
      `arr-keys-are-always-strings.mwlt` is what exists.
- [ ] **The numeric folds at their edges** — `min` (`arr.rs:4021`), `max` (`arr.rs:4033`), `sum`
      (`arr.rs:4046`), `product` (`arr.rs:4057`), `average` (`arr.rs:4076`), spec § 2: the empty subject
      is where they part company — `sum` and `product` have identities (`0`, `1`) and the other three do
      not — and a one-element subject is the last input on which all five agree. `min`/`max` also owe
      their agreement with `sort`'s first and last entry over the same subject.
      `arr-sum-product-and-average-over-numbers.mwlt`, `arr-min-and-max-agree-with-sort.mwlt` and
      `arr-members-over-an-empty-array.mwlt` are what exist.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `mwl_stdlib::json` gap 2, and ADR 0071's non-scalar fields.
- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows — `mwl_stdlib::hash`'s
  module doc.
- ADR 0086 § 1's substitution table for the terminal sink — `crates/mwl-stdlib/src/cli.rs` gap 1.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- `bool as int` does not lower — `crates/mwl-ir/src/lower/expr.rs:877`'s own message lists what does.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
