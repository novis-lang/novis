# Handoff

## State

**M4's Stage 7: ADR 0079 § 4's assertion roster is closed.** `python
tools/loop.py --list` reports no named `.nvst` case owed by any stage.

- **`Core\Test` has eight assertion members plus `expectFailure`.**
  `assertThrows` and § 20's `assertDoesNotThrow` landed beside the three
  equality and three predicate ones; `crates/nvs-stdlib/src/test.rs`'s module
  doc is the home of why each subject is the type it is, and
  `nvs_runtime::Ctx::pending_conforms_to` owns the decision that an ancestor
  class matches.
- **What Stage 7 still owes is the runner's, not the surface's**: § 22's
  `--format=junit` and `--format=json`, § 20's `retries:` and its `FLAKY`
  verdict, and §§ 8-9's `#[Fixture]` — which is also what
  `nvs_types::testing::check_method_shape`'s parameter bullet waits on. The
  roster is at `crates/nvs-cli/src/runner.rs`'s own module doc.
- The conformance corpus is at **733**.

## Next group

**The runner's report and its retry, § 22 and § 20.** The file set is
`crates/nvs-cli/src/runner.rs` (`Outcome` at `:46`, `run_case` at `:141`,
`skip_reason` at `:199`, the plaintext `report` at `:208`) plus
`crates/nvs-cli/src/main.rs`, which is where a subcommand's flags are read:

- [ ] **§ 22's `--format=json`** — one machine-readable object per run, built
      off the same `Outcome` the plaintext `report` at
      `crates/nvs-cli/src/runner.rs:208` renders, so the two cannot disagree
      about a verdict. The duration is already measured there. Decide where the
      flag is parsed (`crates/nvs-cli/src/main.rs`) and keep the default
      plaintext byte-identical, since
      `tests/conformance/lang/a-test-attribute-builds-a-table-the-runner-reports.nvst`
      pins it.
- [ ] **§ 22's `--format=junit`** — the same walk with the XML shape CI reads,
      which is why it is the second slice over one file rather than a group of
      its own.
- [ ] **One `.nvst` over both formats**, in `tests/conformance/lang/`, run
      through `--RUN--`'s `test` word as the report case already is — the
      only way either format is observable.

## Backlog

- § 20's `retries:` and the `FLAKY` verdict — `crates/nvs-cli/src/runner.rs`'s
  own module doc names it.
- §§ 8-9's `#[Fixture]`, which also unblocks the parameter bullet at
  `nvs_types::testing::check_method_shape` and the runner's
  constructor-with-parameters throw.
- A non-`Comparable` object under `assertEquals` should be refused where it is
  written — `nvs_stdlib::test`'s known gap 1, and `nvs_types`' to make.
- A `require` whose path is not a string literal runs nothing, silently, in
  both forms — `nvs_hir::requires`' own known gap.
- ADR 0033's container axis: a `secret` array element or shape-literal field
  carries no redaction bit — `nvs_stdlib::debug`'s known gap 1.
- ADR 0092 § 6's `Throwable` record producer, which waits on the crate edge
  above `nvs-runtime`'s fatal path.
