# Handoff

## State

**M4's Stage 7: ADR 0079 § 8's fixtures are resolved, built and injected end
to end, and what Stage 7 still owes is § 9's data rows.** `python
tools/loop.py --list` reports no named `.nvst` case owed by any stage.

- **A `#[Test]` or `#[Fixture]` parameter is resolved by type against the
  class's `#[Fixture]` roster**, in a pass that runs once the whole class is
  collected (`crates/nvs-types/src/testing.rs:351`), so a test written above
  the fixture supplying it resolves. The row records an **order**, not a set:
  the runner passes values positionally and holds no types.
- **A parameter nothing supplies is `E0736`** — one code for a test's and a
  fixture's own, one roster answering one question — and a fixture requiring
  itself is `E0737` naming the chain. A **variadic** or **`inout`** parameter
  is the declaration-shape code instead (`E0733`/`E0735`), no roster making
  either injectable.
- **The runner builds one `nvs_runtime::Fixtures` per class**, in dependency
  order, before its first test and dropped after its last; that type is where
  the ownership of a built value lives, which is what lets `nvs-cli` forbid
  `unsafe`. A skipped test's fixture is never built.
- **A fixture is shared, not copied**, until § 2's isolates (M5) — the same
  shape class storage already has here. `nvs_cli::runner`'s module doc and
  ADR 0079 § 8 are the two homes.
- **A `static` method called from native code needs its slot 0 filled with the
  called class** (`nvs_runtime::Value::class_desc`); the playbook bullet under
  *Running things* is that trap's home.
- **A retried test is flaky, in all three formats at once.**
  `crates/nvs-cli/src/runner.rs`'s `run_with_retries` is the home of why only
  a failure is retried.
- **`Core\Test` has eight assertion members plus `expectFailure`**;
  `crates/nvs-stdlib/src/test.rs`'s module doc is the home of why each subject
  is the type it is.
- The conformance corpus is at **740**.

## Next group

**§ 9's data rows, which are the second way a `#[Test]` parameter is filled
and the whole of what Stage 7 still owes.** The file set is
`crates/nvs-types/src/testing.rs` (the resolution at `:351`, `check_class_tests`
at `:234`, `reject_uninjectable_parameters` at `:718`) plus
`crates/nvs-types/src/derive.rs:77` (the `ATTRIBUTES` roster a
`#[TestWith]` marker joins) and `crates/nvs-cli/src/runner.rs`
(`build_fixtures` at `:352`, `run_case` at `:445`):

- [ ] **`#[TestWith(...)]` is a row on the `#[Test]` table, checked by name and
      by type** — ADR 0079 § 9 matches a shape literal against the method's
      parameters *by name*, so the marker joins `derive::ATTRIBUTES:77` and its
      payload is folded as `#[Test]`'s options already are; a field whose type
      does not match its parameter is refused where it is written, and a
      parameter filled by neither a row nor a fixture keeps `E0736` — that
      code's help gains the second answer.
- [ ] **Each row is its own reported, separately isolated case** — § 9's "each
      row is its own reported case", which the runner reads as one `Outcome`
      per row rather than one per method: `run_case:445` takes the row's values
      beside `Fixtures::values`, and the report names the row so two failures
      do not read as one flaky method.
- [ ] **The `.nvst` that pins both** — § 9's own worked example running,
      three rows over one method, a row mixed with a fixture parameter, and
      the reject twin over the by-name type mismatch.

## Backlog

- `#[TestSource(Fixtures::names)]` — rows checked when the suite runs rather
  than while compiling (ADR 0079 § 9).
- § 2's isolate per test and parallelism, which is also what makes § 8's
  fixture a graph copy rather than a shared value (M5).
- A `require` whose path is not a string literal runs nothing, silently, in
  both forms (`nvs_hir::requires`' own known gap).
- ADR 0079 § 19's "a test sees `private` members declared in the same file".
- `nvs_stdlib::debug`'s known gap 1: an ADR 0036 shape field and an
  `array<T>` element carry no `secret` bit.
