# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way.**
`crates/nvs-stdlib/src/queue.rs` answers the boot question as a pure function:
`schema_shortfall(&live)` lists what `schema()` asks for that a live `nvs_db::Schema` lacks, as
lines a caller can print, with `nvs_jobs_dedupe` first. The module's `# Known gaps` register is
gone — that was its last item — so `python tools/owners.py --closes decided-closures` names 24
gaps where it named 25.

**Nothing calls it yet.** The judgement is `nvs-stdlib`'s and the boot that acts on it is
`nvs-cli`'s, which is the next group below. The `Decided:` sentence is met only when both halves
are on disk.

A key is matched by the columns it covers and reported under `schema()`'s name for it — a server
holding the uniqueness under an identifier of its own is converged, because refusing a boot that
is fine is the worse of the two failures. The function's doc owns that, and
`rule:core-classes/queue-storage-is-a-table` owns the guarantee itself.

Stage 4's first acceptance check is still red on `json_nesting_is_bounded_by_the_heap_stack_not_the_native_one`
and `an_xml_element_answers_its_namespace_uri`, both for the ordinary reason — the member does not
exist yet. Checks 2 and 3 likewise.

## Next group

**Stage 4: the boot that refuses** — one file set: `crates/nvs-cli/src/serve.rs` and
`crates/nvs-cli/src/schema.rs`. The judgement is landed, pure and unit-tested, so this group is
the caller alone. `rule:core-classes/queue-storage-is-a-table` owns the dedupe guarantee; the
deleted gap's `Decided:` sentence — refuse to serve a queue whose schema is behind, checked at
boot — is the specification.

- [ ] **`crates/nvs-cli/src/serve.rs:1360` — `queue_on_this_core` asks before it serves.**
      Introspect the queue's own connection the way `crates/nvs-cli/src/schema.rs:253`'s
      `introspected` does (`nvs_db::direct::schema_of`), hand the value to
      `nvs_stdlib::queue::schema_shortfall`, and refuse the boot printing every line it answers
      with, in the order it answers them. An empty answer serves as it does today, and a
      deployment with no `[queue]` block reaches none of this.
- [ ] **`crates/nvs-cli/src/serve.rs:2371` — the refusal is asserted as a refusal**, in that crate's
      own tests: the shortfall's lines reach the text an operator reads, and the exit is a failure
      to start rather than a queue that serves with a warning behind it.

## Backlog

- `crates/nvs-stdlib/src/json.rs` — `json_nesting_is_bounded_by_the_heap_stack_not_the_native_one`,
  stage 4's next red test and its own file set.
- `crates/nvs-stdlib/src/xml.rs` — `an_xml_element_answers_its_namespace_uri`, the same check's
  fourth test.
- Stage 4 checks 2 and 3: the Zip64/CRC/seeded/UUID/UNC/EBML row, and the regex step budget as a
  `[limits]` directive with a prepared literal CLDR pattern beside it.
- `docs/agent/carried-gaps.md:51` points the `grants`/`limits` row at "`crates/nvs-stdlib/src/queue.rs`
  gap 1"; that module has no gap register now and the detail is `limits_recorded`'s own doc.
  Owner is `gap-zero`.
- `docs/agent/carried-gaps.md:148`'s bullet still asks the schema-behind question the sheet
  decided; its `[until: gone …:deployment that never ran]` trailer holds while the module prose
  names that deployment, so strike it with the boot half above.
