# Handoff

## State

**Goal `unowned-sweep`, stage 2.** ADR 0147's mechanism is landed end to end and `Core\Uri::with` is
the member spending it. Five of the stage-2 check's eight `-p nvs-stdlib` names now exist and pass —
the three that drive the landed `with` through `nvs_runtime::call`, in
`crates/nvs-stdlib/src/uri.rs`, and the two read off the registry rows, in
`crates/nvs-stdlib/src/registry.rs` beside the pairing guard they extend.

**The stage-2 line stays red until the last three names exist, and that is open work rather than a
regression.** They belong to the `queryParameter` / `withQueryParameter` pair —
`rule:core-classes/uri-removable-components`'s second level, which is why that rule is still
`designed`, and `crates/nvs-stdlib/src/uri.rs`'s known gap 1. That pair is the next group; it was not
taken here because it widens the file set to the spec, the reference page and the `.nvst` corpus.

`Core\Queue`'s gap 1 stays closed as a **decision**, and `crates/nvs-stdlib/src/queue.rs`'s module doc
is its home: `limits` and `grants` wait on enforcement in `nvs_types::expr::isolate`, not on a
spelling. Nothing is blocked.

## Next group

**Stage 2: the query-parameter pair, `rule:core-classes/uri-removable-components`'s second level** —
one file set: `crates/nvs-stdlib/src/uri.rs`, with `docs/spec/01-core-library.md` and
`docs/reference/core/Uri.md` edited in the same slice as the row that lands.

- [ ] **`queryParameter(string $name)` — the singular reader** — `crates/nvs-stdlib/src/uri.rs:398` is
      `CLASS`'s instance roster, `crates/nvs-stdlib/src/uri.rs:940` the `address()` arm whose miss is a
      runtime panic naming the symbol, and `crates/nvs-stdlib/src/uri.rs:2392` the `parseQuery` walk it
      composes with rather than parsing a second time. The gap it closes is
      `crates/nvs-stdlib/src/uri.rs:331` item 1; the row's spec line goes in beside
      `docs/spec/01-core-library.md:868`.
- [ ] **`withQueryParameter(string $name, mixed $value)` — the singular writer** —
      `crates/nvs-stdlib/src/uri.rs:2489` is `buildQuery` and `crates/nvs-stdlib/src/uri.rs:2111` is
      `with`, and writing the rebuilt query back through that member is what keeps one
      canonicalization. A `null` value removes the pair
      (`rule:core-api/a-written-null-removes`), removing the last one leaves no query at all rather
      than a bare `?`, and an array value rides the bracket convention `parseQuery` already reads.
- [ ] **The check's last three names, plus the conformance floor** —
      `crates/nvs-stdlib/src/uri.rs:3341` is the test module's `with_of` / `one` / `text_of` /
      `nullable` helpers, which drive a member the way these need. The names are
      `with_query_parameter_sets_one_pair_and_leaves_every_other_alone`,
      `a_null_value_removes_one_pair_and_the_last_one_leaves_no_query_at_all` and
      `a_query_parameter_round_trips_an_array_value_through_the_bracket_convention`; three `.nvst`
      cases per member under `tests/conformance/core/` are the separate floor
      `crates/nvs-stdlib/tests/conformance_coverage.rs` enforces.

## Backlog

- `Core\Queue` gap 2: the `existing` arm reads the table inside the statement that writes it, racy at
  `read committed` — `crates/nvs-stdlib/src/queue.rs`'s own `# Known gaps`.
- `limits`/`grants` become two ordinary options the day `spawn script` stops answering
  `E_SPAWN_OPTION_UNSUPPORTED` — same gap list, item 1.
- Stage 3, the `array<T>` covariant read the user already took —
  `crates/nvs-types/src/expr/assign.rs`.
- Stage 4, the panic hook and `[limits] max_output` — `crates/nvs-runtime/src/lib.rs`,
  `crates/nvs-stdlib/src/process.rs`.
- `Core\Db::open`'s half of the lifted blocker needs no edit of its own; the agreement is asserted
  from `crates/nvs-stdlib/src/queue.rs` over every registered row.
