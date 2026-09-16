# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0 and 2–9 are done** — stage 9's last open item landed this
session, so the driver's earliest red check is stage 10's. Nothing is blocked.

A record's rendering is **selected** now, not fixed. `carrier_of`
(`crates/nvs-runtime/src/ctx/output.rs:58`) is the one table both channels read: `Ctx::carrier`
answers it for what `echo` writes to and the new `Ctx::diagnostic_carrier` for what a dump writes to,
and `nvs_stdlib::debug::rendered_for` turns a carrier into a rendering. `Core\Debug::dump` therefore
writes the collapsible block once its own channel is the HTML sink and the plaintext line everywhere
else — which is the wiring `[debug] inline` plugs into, not `echo`'s sink.

**`Core\Debug::render` did not take that split**, and `crates/nvs-stdlib/src/debug.rs`'s `# Known
gaps` 2 is the record: answering the sink's carrier means declaring two classes, and `instanceof`
against a `Core` class is `E0496`, so the union is inert to every spelling in
`rule:types/narrowing`. Declaring one class while answering the other was the worse trade — it hands
a `Core\Html\Markup` to the `as string` written for the terminal carrier. `debug::rendered`, whose
text `rule:testing/inline-snapshots` holds a snapshot of, is unchanged.

`[debug] inline` is a directive now: a `tree::Debug` field, a `RuntimeTighten`/`Reload` row, a
`default.toml` line, and `every_derived_default_names_a_directive_the_registry_holds` pairing the
registry with `mode::DERIVED`. Nothing reads it yet — gap 1 of the same block owns that, and its body
half needs `response.rs`, which is another goal's.

## Next group

**Stage 10: the dispatch roster** — one file set: `crates/nvs-stdlib/src/instance.rs`,
`crates/nvs-stdlib/src/csv.rs`, `crates/nvs-stdlib/src/io.rs` and `crates/nvs-stdlib/src/heap.rs`.

- [ ] **`Core\Csv::rows` streams a file, holding one record and never the document** —
      `crates/nvs-stdlib/src/csv.rs:151` holds `parse` and `format` and no third member. The goal's
      § *Standing decisions* fixes the spelling `rows(Core\IO\File $file, {…parse's options}):
      Core\Csv\Rows` under `rule:core-api/verb-lexicon`, and `{header: true}` keys each record by
      the first row. `Core\IO\Lines` is the shape to copy — `crates/nvs-stdlib/src/io.rs:1671` is
      its class, `crates/nvs-stdlib/src/io.rs:1632` its iterate symbol and
      `crates/nvs-stdlib/src/io.rs:1635` its one slot. Closes
      `csv_rows_reads_a_file_one_record_at_a_time` and
      `csv_rows_holds_one_record_and_never_the_document`.
- [ ] **Every `Core` class declaring `compareTo` has a `DISPATCH_ROSTER` row** —
      `crates/nvs-stdlib/src/instance.rs:116` is the roster, and it is what lets an operator
      lowering reach a `Core` class's own member (`docs/decisions/0013.md` § 2). The new `Rows`
      class needs its `iterate` row in the same table, so both items edit it. Closes
      `every_core_class_declaring_compare_to_has_a_dispatch_row`.
- [ ] **A heap of `Core\Time\Instant` orders with no comparator** —
      `crates/nvs-stdlib/src/heap.rs:113` is `Core\Heap<T>`; with the roster rows in place its
      default ordering reaches `compareTo` rather than refusing a non-scalar element. Closes
      `a_heap_of_core_instants_orders_by_compare_to_without_a_comparator` and, with the first item,
      the stage's two `.nvst` cases
      `tests/conformance/core/csv-rows-streams-a-file-and-keys-each-record-by-its-header.nvst` and
      `tests/conformance/core/heap-of-core-instants-needs-no-comparator.nvst`.

## Backlog

- `Core\Debug::render` answers the terminal carrier under every sink —
  `crates/nvs-stdlib/src/debug.rs`'s `# Known gaps` 2. It closes when a `Core` class is testable at
  run time, which is what `E0496` currently refuses; the same blocker sits under
  `Core\Out::capture`, which answers `crate::cli::built` unconditionally
  (`crates/nvs-stdlib/src/out.rs:145`) while `rule:security/capture-answers-the-carrier` says it
  answers the sink's carrier.
- The `[debug] inline` **reader** — the collapsible block appended to an HTML response body with its
  style under the CSP nonce — is `docs/decisions/0092.md:423-425` and gap 1 of the same block. The
  directive and the rendering both exist now; what is missing is `response.rs`, which this goal's
  § *Not this goal* gives to goal `m7-server-surface`.
- The `#[Test]` result producer is `crates/nvs-render/src/lib.rs:39`'s gap 1. Its code is
  `crates/nvs-cli/src/runner.rs` and `crates/nvs-stdlib/src/test.rs`, and `test.rs` is on this goal's
  § *Not this goal* list as goal `m7-server-surface`'s — which the chain has already walked. Stage 9's
  prose and acceptance ask for neither; the gap's owner line still says this goal.
- `crates/nvs-runtime/src/floor.rs:133`'s `report_argument` skips a non-scalar field, so a throw's
  `properties` do not reach the tier-3 handler's array — deliberate, and stated there.
- The plaintext rendering is still uncoloured — `crates/nvs-render/src/plain.rs:15`, waiting on
  nothing now that `Core\Cli` exists.
