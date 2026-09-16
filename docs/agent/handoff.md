# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0 and 2–9 are done**, and stage 10's first item landed this
session: `Core\Csv::rows` streams a file. Its two remaining items — the `compareTo` dispatch rows
and the heap of `Core\Time\Instant` — are the next group below. Nothing is blocked.

`Core\Csv::rows(Core\IO\File $file, {…parse's options}): Core\Csv\Rows` is a **walk over an open
handle**, not a second reading of a document: `crates/nvs-stdlib/src/csv.rs:1` owns what it holds,
why it is taken once, and why taking a handle is R14's admitted shape rather than a breach of it.
The class carries `iterate`/`advance`/`current` itself (`crates/nvs-stdlib/src/instance.rs:212`'s
neighbours are the argument), and `csv-core`'s DFA plus the chunk last read are parked on the
request through `Ctx::hold_open_reader`, because a `Core` slot cannot hold native state. The walk
holds one chunk, one field buffer and one record whatever the document's size, which
`csv_rows_holds_one_record_and_never_the_document` weighs over 4 000 records.

**Two rules moved with it.** `rule:core-api/a-lifetime-is-an-object` now admits a member that takes
a handle it neither opens nor closes, and `crates/nvs-stdlib/tests/capability.rs:690`'s `READERS` is
the roster of them — one entry, and a second format that reads incrementally joins it there rather
than growing a member on `Core\IO\File`. `Core\Csv`'s shared dialect options are
`Qual::Neutral` now: a separator is compared against the document and never written into what
either member answers.

## Next group

**Stage 10: the dispatch roster, continued** — one file set: `crates/nvs-stdlib/src/instance.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/heap.rs` and
`crates/nvs-stdlib/src/time.rs`.

- [ ] **Every `Core` class declaring `compareTo` is reachable by that name** —
      `crates/nvs-stdlib/src/instance.rs:116` is the roster and
      `crates/nvs-stdlib/src/registry.rs:3157`'s `implements_comparable` is the five classes that
      qualify: `nvs_core_time_duration_compare_to` (`crates/nvs-stdlib/src/time.rs:343`),
      `…_instant_…` (`crates/nvs-stdlib/src/time.rs:1201`), `…_date_…`
      (`crates/nvs-stdlib/src/time.rs:2504`), `nvs_core_time_of_day_compare_to`
      (`crates/nvs-stdlib/src/time.rs:2800`) and `nvs_core_uri_compare_to`
      (`crates/nvs-stdlib/src/uri.rs:622`). **Read the convention before adding rows**, because it
      is the fork this item turns on: a roster row is called through
      `crates/nvs-runtime/src/dispatch.rs:1028`'s `call_at`, which *retains* every slot including
      the receiver and never releases it, while a registered `Core` member **borrows** — so listing
      a registered symbol there leaks one reference per operand per comparison. That is why
      `toString` is not a row (`crates/nvs-stdlib/src/instance.rs:111`) and is reached through
      `ClassDesc::renderer` instead, derived from the registry at
      `crates/nvs-stdlib/src/instance.rs:343`. Either give the descriptor a compare address the same
      way, or have the caller look the address up and call it with the borrowing convention; decide
      it in `instance.rs`'s module doc under `rule:classes/comparable`. Closes
      `every_core_class_declaring_compare_to_has_a_dispatch_row`.
- [ ] **A heap of `Core\Time\Instant` orders with no comparator** —
      `crates/nvs-stdlib/src/heap.rs:113` is `Core\Heap<T>` and
      `crates/nvs-stdlib/src/heap.rs:86` is the `compareTo` spelling it already holds; with the item
      above in place its default ordering reaches that member rather than refusing a non-scalar
      element. Closes `a_heap_of_core_instants_orders_by_compare_to_without_a_comparator` and the
      stage's `tests/conformance/core/heap-of-core-instants-needs-no-comparator.nvst`.

## Backlog

- `crates/nvs-stdlib/src/debug.rs` `# Known gaps` 1: nothing reads `[debug] inline` yet; its body
  half needs `response.rs`, which is goal `m7-server-surface`'s.
- `crates/nvs-stdlib/src/debug.rs` `# Known gaps` 2: `Core\Debug::render` does not answer the
  sink's carrier, because a union of two `Core` classes is inert to `rule:types/narrowing`.
- `Core\Csv::parse`'s `$text` is still unclassified, so a `tainted` document — an upload, which is
  the ordinary case — cannot be parsed at all. `crates/nvs-stdlib/src/registry.rs:4112` is the list.
