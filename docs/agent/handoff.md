# Handoff

## State

**Goal `type-test` — the operator is implemented, pinned by cases, and one gap short of the table it
advertises.** Five conformance cases under `tests/conformance/` pin `rule:types/type-test`: the sweep
over a `mixed` subject in both directions per row, totality, the `is`/`as ?T` distinction with both
divergence rows beside it, and one reject case per refusal (`E0811`, `E0812`, `E0813`).

**Six rows of the rule's own table are an ICE, not a diagnostic.** `$m is int|float`, an intersection,
`is iterable`, `is callable`, a shape, and an `array<T>` whose element type no tag decides each reach
`lower_type_test`'s panic and exit 101 — a compiler crash reachable from source the rule accepts, which
is a priority-2 problem and the reason this goal is not done. `test_shape`'s own known gap states it,
and the next group is closing it.

**The carried floor check that named `is_is_a_reserved_spelling` now names the successor.** ADR 0150
turned `is` into an operator, so that half of `rule:php-migration/let-and-is-are-reserved` is asserted
by `is_no_longer_reports_the_reserved_word_refusal_and_let_still_does`, which covers both sides —
`$a is Foo` parses, and `is($a)`, the one position where `is` is still a name, keeps the refusal.

**One consequence stays pinned rather than closed:** a `mixed` holding a backing integer answers
`is Rank::Silver` exactly as one holding the case does (`rule:enums/representation`).

## Next group

**Stage 5: the rows that do not lower** — one file set: `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-ir/src/lower/mod.rs`.

- [ ] **A union and an intersection lower as a chain over the shapes each member already has** — no
      new walk, `||` for a union and `&&` for an intersection, each arm the shape `test_shape` answers
      today. `crates/nvs-ir/src/lower/expr.rs:5174` is where the row is decided and
      `crates/nvs-ir/src/lower/expr.rs:4759` where the chain is emitted. `rule:types/type-test`.
- [ ] **`iterable` and `callable` are one tag test each**, which is what the rule's cost sentence
      promises — an array tag or a `Traversable` descriptor for the first, the closure tag for the
      second. `crates/nvs-ir/src/lower/expr.rs:5118` is `TestShape`, whose `Tag` row takes them.
      `rule:types/type-test`.
- [ ] **A shape, and an `array<T>` whose element type no tag decides, reuse the element walk
      `as array<T>` performs** — never a second one, per the goal's standing decision.
      `crates/nvs-ir/src/lower/mod.rs:3772` is `array_element_tags`, the `?` that gives up today.
      `rule:types/type-test`.
- [ ] **The sweep case grows the six rows once they answer** —
      `tests/conformance/lang/an-is-test-over-a-mixed-subject-answers-the-scalar-class-array-and-literal-rows.nvst:83`
      is the counted block, and its `--TEST--` line names the rows it covers. `rule:types/type-test`.

## Backlog

- A conformance case for narrowing — the true edge, the false edge, and the write that widens; ADR
  0150 § *Verification* names it and only `nvs-types`' unit tests cover it.
- `rule:types/type-test`'s table says nothing about what `is callable` asks of a callable signature's
  parameters; the row is `docs/rules/types/type-test.md:27`.
