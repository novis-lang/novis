# Handoff

## State

**Goal `m8-db-queue`, stage 6: `queryAs<T>` has two doors, and the class picks one.** A class
declaring `Core\Db\Codec`'s `fromRow` and no `#[Db\Derive]` is admitted at the call site by
`check_row_sites` (`crates/nvs-types/src/derive.rs:779`) and dispatched to at run time by `hydrate`'s
empty-`db_codec` arm through `hand_written` (`crates/nvs-stdlib/src/db/row.rs:245`), which hands it
the `Core\Db\Row` a `query` would have answered with. The route is `nvs_runtime::call_static_on`
(`crates/nvs-runtime/src/dispatch.rs:170`), `call_static`'s half for a caller that already holds the
descriptor rather than a label.

**The refusal that is left names both doors.** A class carrying neither is still `E0806` while
compiling and a `LogicError` at run time, and each message now says `fromRow` beside the attribute.

`nvs_types::derive` gap 2 is **narrowed, not closed**: its subject is the `CodecTy::Opaque` field the
reader has no case for, which is gap 1's erasure and still open. The paragraph added to it records
only that a hand-written class is not in it at all — it erases nothing.

Nothing is blocked. Stage 6's other three checks all have their tests on disk; the one item below is
the whole of what it still owes.

## Next group

**Stage 6: the conformance half, at both doors** — one file set: `tests/conformance/core/` and
`crates/nvs-stdlib/src/db/row.rs`. `rule:core-classes/db-column-types` owns the first,
`rule:core-classes/derive-generates-what-is-missing` the second.

- [ ] **`tests/conformance/core/db-query-as-hydrates-a-decimal-an-instant-and-bytes.nvst` is not on
      disk**, and stage 6's `nvs-suite` check names it. What it walks is
      `crates/nvs-stdlib/src/db/row.rs:334`, `converted`'s arms for the three wire types the erasure
      now gives a codec type of their own. Its sibling
      `tests/conformance/reject/db-query-as-over-an-inline-shape-field-is-refused-while-compiling.nvst`
      is written and green, so copy its header and its `--EXPECTF-ERROR--` neighbours for shape. The
      `:memory:` SQLite block a case opens is the playbook's own bullet, and it is the only driver a
      `.nvst` reaches.
- [ ] **No `.nvst` pins the hand-written door**, which is what this session landed and what no
      conformance case names. A class with `static fromRow(Core\Db\Row $row): static` and no
      attribute, over the same `:memory:` block, asserting the member ran — the Rust halves are
      `crates/nvs-stdlib/src/db/row.rs:1629` `query_as_calls_a_hand_written_from_row` and
      `crates/nvs-types/tests/derive.rs:805`, and neither reaches a real driver. Not named by a
      `[[check]]`; take it only after the item above.

## Backlog

- `nvs_stdlib::json` gap 3 — a hand-written `Core\Json\Codec` is still not consulted — is the same
  door one format over, and `nvs_runtime::call_static_on` is now the lookup it was missing:
  `crates/nvs-stdlib/src/json.rs:168`.
- A literal type is `db_reachable` and erases to `CodecTy::Opaque`, so a well-formed
  `public true $flag;` on a `#[Db\Derive]` class refuses at every `queryAs` —
  `crates/nvs-types/src/derive.rs`'s `codec_ty` catch-all against `db_reachable`'s `Ty::True` row.
- An inline-shape field on a `#[Db\Derive]` class reports twice, `E0756` at the declaration and
  `E0806` at the call, and both name the same fix.
- `python tools/db-matrix.py --all` has not run since the four wire drivers' walks landed, and now
  also gates `a_numeric_30_10_postgres_column_throws_on_read_rather_than_truncating` —
  `crates/nvs-stdlib/tests/db_stream.rs`.
- A `decimal` job argument crosses the queue payload as a string and comes back a string, because
  the read is untyped — `crates/nvs-stdlib/src/queue.rs:2253`.
- `json.rs` gap 1's remaining half is an `Instant` (RFC 3339 text, decided) and an inline shape
  reached as a field, `— owner: m8-stdlib-depth`.
