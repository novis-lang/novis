# Handoff

## State

**Goal `m8-db-queue`, stage 6: the `decimal` half of the row decoders is closed on every wire
driver.** `nvs_runtime::Decimal::read` is `parse` with its two failures told apart — a literal past
96 mantissa bits or scale 28, and text that is not a literal at all — and `crates/nvs-db/src/pg.rs`
`PgColumn::decimal` owns why there are two. Each driver maps them to its own metadata:
`crates/nvs-db/src/mysql.rs:3200` and `crates/nvs-db/src/tds/value.rs:479` are the same split.

**A `NUMERIC(30,10)` now names what fixes it.** Past the range the refusal says the column's own type
is what narrows; the other cause keeps "did not decode as a decimal", which is also where
PostgreSQL's `NaN` lands — that type has no value for one. Neither message carries the value.

The last handoff flagged `crates/nvs-stdlib/src/db/column.rs` `column_value`'s doc as going stale if
the split landed a layer down: it holds no sentence about `decimal` at all, so nothing there moved.
The JSON half closed the session before. Nothing is blocked; stage 1 is the carried floor and the
driver's to run.

## Next group

**Stage 6: what `queryAs<T>` does with a class the derive did not write** — one file set:
`crates/nvs-stdlib/src/db/row.rs`, `crates/nvs-types/src/derive.rs` and `tests/conformance/core/`.
`rule:core-classes/derive-generates-what-is-missing` owns the first, `rule:core-classes/db-column-types` the second.

- [ ] **A hand-written `fromRow` is called.** The class carries **no** `#[Db\Derive]` — one that
      carries both is `E_DERIVE_BOTH_HALVES` at `crates/nvs-types/src/derive.rs:1240`, and
      `crates/nvs-types/tests/derive.rs:299` is that refusal with the class to copy. So the work is
      the other door: `crates/nvs-stdlib/src/db/row.rs:123` `hydrate` refuses an empty
      `desc.db_codec()` as a `LogicError`, and that arm is where a class declaring
      `Core\Db\Codec`'s `fromRow` is dispatched to instead of refused — ADR 0067 § 9's
      `docs/decisions/0067.md:283` admits it beside the derived one. `check_row_sites`' `E0806`
      (`crates/nvs-diagnostics/src/lib.rs:2830`) has to admit it too, or the call never reaches
      run time. The test is `query_as_calls_a_hand_written_from_row` (`-p nvs-stdlib`), and
      `crates/nvs-types/src/derive.rs:72-83` gap 2 is closed or struck with what you find.
- [ ] **`tests/conformance/core/db-query-as-hydrates-a-decimal-an-instant-and-bytes.nvst` is not on
      disk**, and stage 6's `nvs-suite` check names it. What it walks is
      `crates/nvs-stdlib/src/db/row.rs:322`, `converted`'s three arms for the wire types the erasure
      gives their own codec type. Its sibling
      `tests/conformance/reject/db-query-as-over-an-inline-shape-field-is-refused-while-compiling.nvst`
      is written and green. The `:memory:` SQLite block a case opens is the playbook's own bullet.

## Backlog

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
