# Handoff

## State

**M4's Stage 7: § 22's report is closed, and what Stage 7 still owes is § 20's
retry and §§ 8-9's fixtures.** `python tools/loop.py --list` reports no named
`.nvst` case owed by any stage.

- **`nvs test` reports one run in three formats.** `--format=json` and
  `--format=junit` join the plaintext default, all three over one `Outcome`
  list; `crates/nvs-cli/src/runner.rs`'s module doc is the home of why a
  verdict is decided once and of where a machine format sends the program's
  own output (stderr — stdout is the document). ADR 0079 § 22 carries both
  decisions.
- **A `.nvst` can now read a machine report**: `--RUN--`'s roster is four
  spellings, `nvs_test::Subcommand::args` handing back a whole command line.
  `crates/nvs-test/src/lib.rs`'s module doc is the format's one home.
- **`Core\Test` has eight assertion members plus `expectFailure`**;
  `crates/nvs-stdlib/src/test.rs`'s module doc is the home of why each subject
  is the type it is.
- What Stage 7 still owes is the runner's: § 20's `retries:` and its `FLAKY`
  verdict, and §§ 8-9's `#[Fixture]` — which is also what
  `nvs_types::testing::check_method_shape`'s parameter bullet waits on. The
  roster is at `crates/nvs-cli/src/runner.rs`'s own module doc.
- The conformance corpus is at **735**.

## Next group

**§ 20's retry and its `FLAKY` verdict.** The file set is
`crates/nvs-cli/src/runner.rs` (the `Outcome` enum at `:110` with its
`verdict`/`mark`/`message` trio, the per-class loop at `:182`, `run_case` at
`:255`, `skip_reason` at `:313`, the three renderings at `:340`+) plus
`crates/nvs-types/src/testing.rs` (the option roster that already admits
`retries:`, `check_method_shape` at `:208`, `TestCase` at `:135`):

- [ ] **§ 20's `retries:` and the `FLAKY` verdict** — a test that failed and
      then passed within its allowance is neither passed nor failed. It is a
      fifth `Outcome` variant, so it takes a `verdict`, a `mark` and a row in
      each of the three documents at once, and the summary line § 22 writes
      already names it (`1 flaky in 41 ms`). The option is folded already —
      `nvs_types::testing` admits `retries: int` — so what is owed is reading
      it back the way `skip_reason` reads its own at
      `crates/nvs-cli/src/runner.rs:313`, and re-running the body with the
      ledger and the pending exception taken between attempts.
- [ ] **One `.nvst` over the retry**, in `tests/conformance/lang/`, run
      through `--RUN--\ntest`: a test that passes on its second attempt, one
      that exhausts its allowance and fails, and `retries: 0` beside them.
      The plaintext twin
      `a-test-attribute-builds-a-table-the-runner-reports.nvst` is the shape
      to copy.
- [ ] **The machine formats' `flaky` row**, folded into the two cases this
      session landed rather than given a third: both already name every other
      verdict, so a fifth that only one format learned fails there.

## Backlog

- §§ 8-9's `#[Fixture]` injection, and the parameter bullet
  `nvs_types::testing::check_method_shape` waits on it for — ADR 0079 §§ 8-9.
- § 2's isolate-per-test and parallelism — M5, `crates/nvs-cli/src/runner.rs`.
- A `require` whose path is not a string literal runs nothing, silently, in
  both forms — `nvs_hir::requires`' own known gap.
- ADR 0033's container axis: a `secret` array element or shape-literal field
  is not modelled — `nvs_stdlib::debug`'s known gap 1.
- § 6's `Throwable` producer for `Core\Debug` waits on the crate edge above —
  `nvs_stdlib::debug`.
- `nvs_types::signatures` leaves a class constant's declared type unmodelled,
  so `Class::TOKEN` infers `mixed` at every expression site.
