# Handoff

## State

**Goal `m8-db-queue`, stage 5: `streamAs<T>` is all that is left of it.** `serverVersion` is closed
end to end — each driver keeps what its handshake delivered, `Core\Db\Connection::serverVersion`
answers it, and `server_version_answers_what_the_connection_kept`
(`crates/nvs-stdlib/tests/db_stream.rs:658`) now asserts against the server that produced the string,
green on all five legs under `python tools/db-matrix.py --all`. What each driver is entitled to say is
ADR 0187 § 2's table and nothing else.

**The roster hole is closed.** `part_two_members` reads § 18's two `| Type | Members … |` tables as
well as its `| Member |` ones, so `close`, `driver`, `serverVersion`, `isOpen` and
`Transaction::rollBack` — and the `Rows`/`Row`/`Write`/`Column` rosters beside them — are enumerated
and gated rather than walked past. All four were already registered, so no ratchet key moved.

Nothing is blocked. Stage 1 is the carried floor and the driver's to run.

## Next group

**Stage 5: `streamAs<T>`, `stream` at a written type** — one file set:
`crates/nvs-stdlib/src/db/registry.rs`, `crates/nvs-stdlib/src/db/stream.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-types/src/derive.rs` and
`crates/nvs-stdlib/tests/db_stream.rs`. `rule:core-classes/db-streaming` is the member's contract and
spec § 18 its signature; the whole member is one slice's worth of work, so it is the group.

- [ ] **The two rows and the generic plumbing.** `streamAs` beside `stream` on both `Queryable`
      classes — `crates/nvs-stdlib/src/db/registry.rs:372` and `:678` are the return types to copy —
      answering `CoreTy::InstanceAt(STREAM_NAME, &[CoreTy::Written("T")])` where `queryAs`
      (`crates/nvs-stdlib/src/db/registry.rs:298`) answers `Rows<T>`, which makes `stream`'s own row
      `InstanceAt(STREAM_NAME, &[CoreTy::Instance(ROW_NAME)])`. `Core\Db\Stream`'s `T` is declared in
      `crates/nvs-stdlib/src/registry.rs:2970` `GENERIC_CLASSES` and `:3000` `ITERABLES`, and both
      spellings of the member go on `:2865` `WRITTEN_CLASS_MEMBERS` — `rule:classes/no-traits`'
      delegation is keyed by the declaring class, so the transaction's is a second row.
- [ ] **The body.** `nvs_core_db_connection_stream_as` beside `crates/nvs-stdlib/src/db/stream.rs:426`,
      reading the three written-class arguments the way `crates/nvs-stdlib/src/db/execute.rs:1221`
      does and refusing the `array<...>` form there for its reason. The class goes in a fourth
      `STREAM` slot (`crates/nvs-stdlib/src/db/stream.rs:66`) and is read where the row is parked —
      `crates/nvs-stdlib/src/db/stream.rs:106` `park_row`, through
      `crates/nvs-stdlib/src/db/row.rs:122` `hydrate`, which needs the `ctx` `stream_step`
      (`crates/nvs-stdlib/src/db/stream.rs:135`) already holds. The `address()` arm is
      `crates/nvs-stdlib/src/db/mod.rs:638`.
- [ ] **The compile-time refusal**, which the goal's own check names:
      `check_row_sites` (`crates/nvs-types/src/derive.rs:713`) asks `streamAs` the same question it
      asks `queryAs`, and `tests/conformance/reject/db-stream-as-over-a-class-without-db-derive-is-refused-while-compiling.nvst`
      is the case. `rule:core-classes/derive-attribute` owns the opt-in.
- [ ] **`stream_as_answers_rows_at_the_class_it_was_written_with`**, beside
      `crates/nvs-stdlib/tests/db_stream.rs:658` over the same `leg()`, plus the three `.nvst` cases a
      new member owes; then strike `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:32`.
      Run all five legs with `python tools/db-matrix.py --all`.

## Backlog

- `part_one_members` has the hole this session closed for Part II: § 7's
  `| Type | Members | Replaces |` table (`docs/spec/01-core-library.md:706`) is read by nothing.
  Owner: `crates/nvs-stdlib/tests/spec_registry_coverage.rs`.
- `crates/nvs-db/src/tds/token.rs:228` renders the version per call; a field on `TdsConn` holds it,
  so nothing caches — fine, but it is the one driver whose answer is built rather than kept.
- The `unowned` gaps in `crates/nvs-stdlib/src/db/mod.rs` and `crates/nvs-stdlib/src/queue.rs` are
  goal `unowned-closures`'s, per this goal's § *Standing decisions*.
