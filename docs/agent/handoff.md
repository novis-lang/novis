# Handoff

## State

**M4's Stage 7: ADR 0079 § 9's data rows are checked, folded and resolved
against § 8's fixtures, and what Stage 7 owes is the runner half of § 9.**
`python tools/loop.py --list` reports no named `.nvst` case owed by any stage.

- **`#[TestWith(...)]` is `Core\Test\TestWith` on ADR 0071 § 1's recognized
  roster** (`crates/nvs-types/src/derive.rs:112`), the one such name that may
  repeat on a declaration. Its payload is matched against the *method's own
  parameter list*, by name and by type, which is why `nvs_types::testing` checks
  it and `crate::attributes` does not.
- **One function resolves both sources** — `resolve_parameters`
  (`crates/nvs-types/src/testing.rs:428`) — and asks the row's name **before**
  the fixture's type, so a row wins a parameter a fixture would also have
  answered. `TestCase::params` is one `Injection` per parameter and
  `TestCase::rows` is each row folded in parameter order.
- **Every way a row fails to describe its method is `E0738`**; a field twice is
  `E0304`, and a parameter neither roster reaches keeps `E0736`, whose help now
  names both answers and offers the row only to a `#[Test]`.
- **The runner does not run a row yet.** `nvs_cli::runner::run_case`
  (`crates/nvs-cli/src/runner.rs:463`) reports a row-filled test loudly rather
  than calling it with a hole; `fixtures_needed` is the fixture positions alone.
- **A fixture is shared, not copied**, until § 2's isolates (M5) — the same
  shape class storage already has here. `nvs_cli::runner`'s module doc and
  ADR 0079 § 8 are the two homes.
- **A `static` method called from native code needs its slot 0 filled with the
  called class** (`nvs_runtime::Value::class_desc`); the playbook bullet under
  *Running things* is that trap's home.
- **`Core\Test` has eight assertion members plus `expectFailure`**;
  `crates/nvs-stdlib/src/test.rs`'s module doc is the home of why each subject
  is the type it is.
- The conformance corpus is at **741**.

## Next group

**The runner half of § 9, which is the whole of what Stage 7 still owes.** The
file set is `crates/nvs-cli/src/runner.rs` (`run_case` at `:463`,
`fixtures_needed` at `:527`, `run_with_retries` at `:307`, the `Case`/`Outcome`
reporting at `:196`) plus `crates/nvs-runtime/src/dispatch.rs`
(`Fixtures::values` at `:423`, whose ownership shape a row's own values need
too):

- [ ] **Each row is its own reported, separately isolated case** — § 9's "each
      row is its own reported case": one `Case` per row, labelled so the three
      formats tell them apart, with the row's `ConstArg`s materialized into
      `Value`s. `nvs-cli` forbids `unsafe`, so the release of a materialized
      row belongs in a `nvs-runtime` owner beside `Fixtures` rather than in the
      runner; `NvsStr::new` (`crates/nvs-runtime/src/string.rs:253`) is the one
      allocation a row needs.
- [ ] **The `.nvst` that pins both** — § 9's own worked example running through
      `nvs test` (`--RUN--`, `crates/nvs-test`'s module doc), each row reported
      on its own line with its own verdict, one row failing while its siblings
      pass, and the fixture/row mix beside them.

## Backlog

- § 9's `#[TestSource(Fixtures::names)]` — rows computed at run time; ADR 0079
  § 9 names it and nothing implements it.
- § 2's isolate per test and § 20's parallelism (M5) — `nvs_cli::runner`'s own
  module doc names both.
- A `require` whose path is not a string literal runs nothing, silently —
  `nvs_hir::requires`' own known gap.
- `nvs_stdlib::debug` gap 1: an ADR 0036 shape field and an `array<T>` element
  carry no `secret` bit (ADR 0033's unmodelled container axis).
- `nvs_stdlib::test` gap 1: a non-`Comparable` object under `assertEquals`
  throws where ADR 0079 § 4 writes a compile error.
