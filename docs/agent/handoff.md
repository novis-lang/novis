# Handoff

## State

**Goal `test-doubles`, stage 4 is complete and green.** `Core\Test::assertCalled` and
`assertNeverCalled` are registry rows with bodies (`crates/nvs-stdlib/src/test.rs:755`), and five
cases cover them — the three the acceptance check names plus the two the class's floor of three per
member needed. The record they read is the one every double already kept: slot 0 of its fields, keyed
by method name, one list per call.

**The method reference is spelled bare and admitted by the row.** `Mailer::send` is written where a
class constant would go; a parameter written `CoreTy::MethodRef` is the whole set of positions the
spelling means anything at, and `nvs_types::core_lib::method_ref_param` reads that set back off the
row. Three crates carry it: `nvs_hir::members`' walk steps over the argument (the playbook bullet
below says why), `nvs_types::expr::calls::note_method_ref_args` marks the span, and
`nvs_types::expr::members::method_reference` folds it to the method's own name through the
`ExprInfo::ClassConst` entry `nvs-ir` already lowers — so `nvs-ir` gained no arm. `E0829` refuses a
name the class does not declare and `E0830` refuses anything at that position that is not a reference.
`rule:testing/interaction-after-the-fact` is now `shipped`, guarded by the three new cases.

**Stage 5 is untouched**, and it is the next acceptance check.

## Next group

**Stage 5: `assertCompletes` under the test's clock** — one file set: `crates/nvs-stdlib/src/test.rs`,
`crates/nvs-cli/src/runner.rs`, `tests/conformance/core/`.

- [ ] **`assertCompletes` is a registry row with a body** — goal prose stage 5. A row beside
      `assertNeverCalled`'s at `crates/nvs-stdlib/src/test.rs:755`, its card after
      `crates/nvs-stdlib/src/test.rs:1240`, its helper beside `nvs_core_test_advance` at
      `crates/nvs-stdlib/src/test.rs:1681` — which is where the virtual clock is already advanced and
      where the "no test fixed a clock" `LogicError` is already spelled — and an `address()` arm at
      `crates/nvs-stdlib/src/test.rs:2200`. `rule:testing/task-tree-and-virtual-clock`.
- [ ] **The refusal a `.nvst` can observe** —
      `tests/conformance/core/assert-completes-refuses-without-a-fixed-clock.nvst`, the exact name the
      stage's check lists. A case is top-level statements and never inside a `#[Test]`, so the
      unfixed-clock refusal is the only side of the member it can reach; shape it like
      `tests/conformance/core/assert-never-called-fails-naming-the-call-it-found.nvst:1`.
- [ ] **The accepted side is a runner test** — `assert_completes_passes_a_body_that_finishes_under_the_clock`
      and `assert_completes_fails_a_body_still_running_naming_the_duration`, beside
      `a_test_at_a_fixed_clock_reads_that_clock` at `crates/nvs-cli/src/runner.rs:2725`, which is the
      only place a `#[Test]` with a fixed clock actually runs.

## Backlog

- Stage 6 flips `rule:testing/doubles` to `shipped` and closes the goal — goal prose stage 6.
- `fn (): mixed => <a void call>` ICEs in lowering (`an operand used before it is defined`), with no
  double or method reference involved; `fn (): void => { … }` is fine. Found while probing, not this
  goal's — `crates/nvs-ir/src/lower/expr.rs`.
- `Mailer::send(...)` — the first-class-callable spelling of an instance method with no receiver —
  panics in `crates/nvs-ir/src/lower/expr.rs:2856` where `nvs_types` was expected to refuse it.
- The four proofs beyond ADR 0079's own bullets are goal `dossier`, not this one.
