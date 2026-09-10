# Handoff

## State

**Goal `type-test` — stages 0, 2 and 3 are on disk.** `is` parses `expr is Type`, and the checker
answers `bool` for every subject, folds a settled answer to `true` or `false`, and refuses only the
right-hand side: `E0811` for `void`/`never`, `E0813` for a `tainted`/`secret` qualifier, `E0812` in
the parser for `$x is $cls`. `crates/nvs-types/src/expr/type_test.rs` is the whole of it and its
module doc is the map; the eight stage-3 acceptance tests live in `crates/nvs-types/tests/type_test.rs`.

**`E0810` is not this operator's, and the rule now says so.** That code has been
`E_DECODED_FIELD_NOT_TAINTED` since long before ADR 0150 wrote its table, so the qualifier refusal
took the band's next free code and `rule:types/type-test`'s cell, the goal's prose and the acceptance
check's test name were corrected together. [ADR 0150](../decisions/0150.md) § 6 keeps its `E0810` —
a record is frozen rationale and the fragment is the rule.

**Nothing lowers the node.** `nvs-ir` has never seen an `ExprKind::TypeTest`, so no program can run
an `is` yet and `examples/type-test.nvs` (stage 6) cannot exist until stage 5 lands. Narrowing is
stage 4 and is not written: the true edge still knows nothing.

## Next group

**Stage 4: narrowing, the fifth spelling** — one file set: `crates/nvs-types/src/locals.rs`,
`crates/nvs-types/src/expr_table.rs`, `crates/nvs-types/src/expr/type_test.rs`,
`crates/nvs-types/tests/narrowing.rs`.

- [ ] **The tested type is recorded at the test's span** — `locals::narrow` sees the AST and the
      table and nothing else, so `is` records what it lowered the way `instanceof` records its class
      at `crates/nvs-types/src/locals.rs:481`. The recording site is
      `crates/nvs-types/src/expr/type_test.rs:54`, and the variant is `expr_table.rs`'s
      `ExprInfo::InstanceOf`'s sibling. `rule:types/narrowing`.
- [ ] **`narrow` gains the `is` arm, on the true edge alone** —
      `crates/nvs-types/src/locals.rs:397`, beside the `instanceof` and `!= null` spellings. The
      false edge stays empty for all five spellings (ADR 0150 § 9). `rule:types/type-test`.
- [ ] **The subject-name walk accepts a `TypeTest`** — `crates/nvs-types/src/locals.rs:515` reads
      which binding a condition tested, and today matches `ExprKind::InstanceOf` alone.
      `rule:types/narrowing`.
- [ ] **The four acceptance tests** — `an_is_test_narrows_its_subject_on_the_true_edge` and its
      three siblings, named by the `stage = "4 narrowing"` check in `docs/agent/loop-goal.toml`, at
      `crates/nvs-types/tests/narrowing.rs:37` beside the `instanceof` ones, which is also the
      fixture shape they reuse. `rule:types/narrowing`.

## Backlog

- Stage 5: lowering and codegen — one tag comparison, `instanceof`'s descriptor walk and
  `as array<T>`'s element walk reused, never a second one. `docs/agent/loop-goal.md` § *Stage 5*.
- Stage 6: `examples/type-test.nvs`, whose thirteen `want` lines are frozen in
  `docs/agent/loop-goal.toml` — the acceptance check that has been red every session so far.
- A folded test still checks its subject and still costs a walk; whether the fold reaches the IR at
  all is stage 5's `a_folded_test_emits_no_code_at_all`.
- Carried gaps that outlive this goal: [carried-gaps.md](carried-gaps.md).
