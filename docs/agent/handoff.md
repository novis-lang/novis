# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way.** The queue's boot half is landed,
so the `Decided:` sentence that struck `crates/nvs-stdlib/src/queue.rs`'s last gap is met on both
sides: `crates/nvs-cli/src/serve.rs:1411`'s `queue_storage_is_current` opens the `[queue]` block's
own connection, introspects it through `crate::schema::introspected`, and hands the value to
`nvs_stdlib::queue::schema_shortfall`. A shortfall prints every line in the order that function
answers with — the dedupe key first — and refuses; an empty answer serves as before.

The refusal is the **process's** and not the core's: `arm_queue_workers` begins the drain through
`crate::stop::deliver_to` before it returns, so the cores already accepting stop too, and the core
that asked returns without taking a listener. An enqueue-only `workers = 0` instance and a tree
writing no `[queue]` block open nothing at all — the filter that answered them runs first.

`crate::schema::open`, `crate::schema::introspected` and `crate::queue::dialect_of` are `pub(crate)`
for this; each says in its own doc why. `python tools/owners.py --closes decided-closures` still
names 24 gaps — this group built a caller, not a gap.

Stage 4's first acceptance check is still red on
`json_nesting_is_bounded_by_the_heap_stack_not_the_native_one` and
`an_xml_element_answers_its_namespace_uri`, both for the ordinary reason: the member does not exist
yet. Checks 2 and 3 likewise.

## Next group

**Stage 4: the JSON codec** — one file set: `crates/nvs-stdlib/src/json.rs`, whose module doc holds
both gaps and both `Decided:` sentences. The first turns the stage's first acceptance check green.

- [ ] **`crates/nvs-stdlib/src/json.rs:229` — gap 2, the encode walk carries its own stack.**
      `Encodable` recurses through `serde_json`'s serializer today, so the real bound is the
      thread's stack and a deep-enough document aborts where `DEPTH_CEILING` should have thrown.
      Carry the walk on an explicit heap stack bounded by that ceiling, and state what it spends
      per request in the module doc (`rule:programs/memory-priority`). The acceptance check names
      the test: `json_nesting_is_bounded_by_the_heap_stack_not_the_native_one`.
- [ ] **`crates/nvs-stdlib/src/json.rs:216` — gap 1, keep the descriptor and widen it.** Default
      constants on `CodecField` and a `ClassDesc` method lookup for `toJson`, per the gap's
      `Decided:` line, and amend `rule:core-classes/derive-generates-what-is-missing` in the same
      slice — its fragment asks for IR emitted per derived class, which is not what is built.

## Backlog

- `crates/nvs-stdlib/src/xml.rs:128` gap 1 — a name answers its namespace URI, the other member
  stage 4's first acceptance check names.
- `crates/nvs-stdlib/src/regex.rs:83` gap 2 and `cldr.rs:212` gap 1 — stage 4's third check.
- Stage 4's second check: Zip64, the CRC refusal, `Core\Random\Seeded`, a UUID's bytes, a UNC root
  and the EBML case, over `zip.rs`, `random.rs`, `uuid.rs`, `path.rs` and `mime.rs`.
- `test.rs` gaps 1–2, `reflect.rs` gaps 1–2, `db/mod.rs` gaps 1–2 — later in stage 4.
- `[context.stage.4] rules` now names `core-classes/derive-generates-what-is-missing`, added here
  for the group above; `[context] modules` printed no `crates/nvs-cli/**`, which the driver's own
  sweep closes from this session's commits.
