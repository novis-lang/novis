# Handoff

## State

**Goal `type-test` — stages 0, 2, 3 and 4 are on disk.** `is` parses, answers `bool` for every
subject, folds a settled answer, refuses only its right-hand side (`E0811`, `E0813`, and the parser's
`E0812`), and now narrows its subject on the true edge as `rule:types/narrowing`'s fifth spelling.
`crates/nvs-types/src/expr/type_test.rs` and `crates/nvs-types/src/locals.rs`' narrowing section are
the two module docs that map it.

**The recorded entry is the contract between the two halves.** A test whose answer is a run-time
`bool` records `ExprInfo::TypeTest { tested }` on the `is` expression's own span; a folded or refused
one records nothing. `locals::type_test_residue` leans on exactly that — the entry's existence is why
it needs no guard against a target that would *widen* the binding, since such a target folded to
`true` and never arrives. `nvs-ir` reads the same entry in stage 5.

**Nothing lowers the node.** `nvs-ir` has never seen an `ExprKind::TypeTest`, so no program can run
an `is` and `examples/type-test.nvs` (stage 6) cannot exist yet — the driver's red check naming that
fixture is an item still open, not a regression. `E0810` is not this operator's; `rule:types/type-test`
is the home of the corrected cell and [ADR 0150](../decisions/0150.md) § 6 keeps its frozen `E0810`.

## Next group

**Stage 5: lowering and codegen, and no second walk** — one file set: `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-codegen/src/emit.rs`, `crates/nvs-codegen/tests/objects.rs`.

- [ ] **`lower_expr` gains a `TypeTest` arm, and a scalar test is one tag comparison** — the dispatch
      at `crates/nvs-ir/src/lower/expr.rs:47` has no arm for the node, which is the silent trap: a
      missing arm is a panic or a hole, not a compile error. The arm reads
      `ExprInfo::TypeTest { tested }` back off the span the checker recorded it under, beside the
      `ExprKind::InstanceOf` arm at `crates/nvs-ir/src/lower/expr.rs:267`. A folded test records
      nothing and reaches here as the literal it already is, which is
      `a_folded_test_emits_no_code_at_all`. `rule:types/type-test`.
- [ ] **The class path emits the descriptor walk `instanceof` already emits** — the same
      `InstKind::InstanceOf` built at `crates/nvs-ir/src/lower/expr.rs:4792` and emitted at
      `crates/nvs-codegen/src/emit.rs:707`, never a second one. `rule:types/type-test`, and the goal's
      § *Standing decisions* is the rule about reuse: a refactor to reach it is the slice, not a
      second emitter.
- [ ] **The `array<T>` path calls the same element walk `as array<T>` calls** — the conversion arm at
      `crates/nvs-ir/src/lower/expr.rs:391` owns that walk today, and two element walks in the tree
      means one is wrong. `rule:types/type-test` states the O(n) cost this pays.
- [ ] **The four acceptance tests**, in `crates/nvs-codegen/tests/objects.rs:169`'s file, which is
      where the `instanceof` codegen cases already live: `a_scalar_test_emits_one_tag_comparison`,
      `a_class_test_emits_the_same_descriptor_walk_instanceof_emits`,
      `an_array_element_test_calls_the_same_walk_the_conversion_calls_and_not_a_second_one`,
      `a_folded_test_emits_no_code_at_all`.

## Backlog

- Stage 6 is `examples/type-test.nvs`, the reference heading and the conformance case — all of it
  needs stage 5 first; `docs/agent/loop-goal.toml`'s stage 6 checks are the specification.
- `ExprInfo::TypeTest` has no unit case beside `an_instanceof_records_the_resolved_class` in
  `crates/nvs-types/src/expr_table.rs`' own tests; `crates/nvs-types/tests/narrowing.rs` covers the
  recording end to end instead.
- False-edge narrowing stays out of scope for all five spellings at once — ADR 0150 § 9.
