# Handoff

## State

**Goal `m8-db-queue`, stage 6: the JSON half is closed.** A `decimal` field decodes
(`crates/nvs-stdlib/src/json.rs:2250` `scalar`) and `undecoded` is down to `Opaque`, `Bytes` and
`Instant`, so `check_codec`'s pre-check and `decode_field`'s fault arm now refuse only those.

**A `decimal` is a JSON string at both doors, and that is a user-visible change**: `Core\Json::encode`
writes `"19.99"` where it wrote `19.99`. JSON has one number type and every consumer — including this
module's own `Decode` — reads it as an `f64`, so the number spelling could not be read back as what
was written. The reasoning and what it costs are in `json.rs`'s gap 1; the two conformance cases that
pinned the old spelling are updated, and `Core\Jwt`, `Core\Queue`, `Response::json` and `Core\Sse`
share the one encoder and so share the new spelling.

The `-p nvs-types` half of stage 6 was already green. Nothing is blocked. Stage 1 is the carried floor
and the driver's to run.

## Next group

**Stage 6: the `Core\Db` half, what the JSON half does not touch** — one file set:
`crates/nvs-db/src/pg.rs`, `crates/nvs-stdlib/src/db/column.rs`, `crates/nvs-stdlib/src/db/row.rs`
and `crates/nvs-stdlib/tests/db_stream.rs`. `rule:types/decimal` owns the refusal a `NUMERIC` takes
and `rule:core-classes/db-column-types` the map both doors read.

- [ ] **A `NUMERIC(30,10)` throws naming the column rather than truncating.**
      `crates/nvs-db/src/pg.rs:2006` is the `oid::NUMERIC` arm, and it **already refuses**:
      `Decimal::parse` answers `None` past the 96-bit mantissa and the arm raises
      `crates/nvs-db/src/pg.rs:2566` `malformed`, which names the column and deliberately not the
      value. So the work is not the refusal but telling its two causes apart — a value past
      `decimal`'s range is a schema mismatch an operator fixes with a column type, where a body that
      is not a decimal literal at all is a driver bug — plus the two tests
      `a_numeric_30_10_value_past_decimals_range_throws_rather_than_truncating` (`-p nvs-stdlib`) and
      `a_numeric_30_10_postgres_column_throws_on_read_rather_than_truncating`
      (`-p nvs-stdlib --test db_stream`, which needs the matrix). `crates/nvs-db/src/tds/value.rs:479`
      and `crates/nvs-db/src/mysql.rs:3068` are the same call on the other two drivers and take the
      same split. `crates/nvs-stdlib/src/db/column.rs:176` `column_value` is where the doc comment
      says this refusal lives, and that sentence goes stale if the split lands one layer down.
- [ ] **A hand-written `fromRow` is called.** `crates/nvs-stdlib/src/db/row.rs:123` `hydrate` —
      `rule:core-classes/derive-generates-what-is-missing` lets a class write one half and keep the
      generated other, and the test is `query_as_calls_a_hand_written_from_row` (`-p nvs-stdlib`).
- [ ] **`tests/conformance/core/db-query-as-hydrates-a-decimal-an-instant-and-bytes.nvst` is not on
      disk**, and stage 6's `nvs-suite` check names it. What it walks is
      `crates/nvs-stdlib/src/db/row.rs:322`, `converted`'s three arms for the wire types the erasure
      now gives their own codec type. Its sibling
      `tests/conformance/reject/db-query-as-over-an-inline-shape-field-is-refused-while-compiling.nvst`
      is written and green. The `:memory:` SQLite block a case opens is the playbook's own bullet.

## Backlog

- A literal type is `db_reachable` and erases to `CodecTy::Opaque`, so a well-formed
  `public true $flag;` on a `#[Db\Derive]` class refuses at every `queryAs` —
  `crates/nvs-types/src/derive.rs`'s `codec_ty` catch-all against `db_reachable`'s `Ty::True` row.
- An inline-shape field on a `#[Db\Derive]` class reports twice, `E0756` at the declaration and
  `E0806` at the call, and both name the same fix.
- `python tools/db-matrix.py --all` has not run since the four wire drivers' walks landed —
  `crates/nvs-stdlib/tests/db_stream.rs`.
- A `decimal` job argument crosses the queue payload as a string and comes back a string, because
  the read is untyped — `crates/nvs-stdlib/src/queue.rs:2253`.
- `json.rs` gap 1's remaining half is an `Instant` (RFC 3339 text, decided) and an inline shape
  reached as a field, `— owner: m8-stdlib-depth`.
