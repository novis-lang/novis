# Handoff

## State

**Goal `m8-db-queue`, stage 6 is half closed.** The erasure lands: `decimal`, `bytes` and
`Core\Time\Instant` each carry a `CodecTy` of their own
(`crates/nvs-runtime/src/object.rs:607`), so `CodecTy::Opaque` now means an inline shape reached as
a field and nothing else. `check_row_sites` gained its fourth `E0806` condition — a field the
erasure gave no wire type — so every way `queryAs<T>`/`streamAs<T>` is answered *no* is said while
compiling, and `hydrate`'s run-time pair stays as the backstop for a class built by hand. Stage 6's
`-p nvs-types` check is green: both tests it names exist and pass.

What stage 6 still owes is the `-p nvs-stdlib` half — the JSON decimal decoder, a `NUMERIC` that
will not fit throwing rather than truncating, and a hand-written `fromRow` being called. The DB
half of the per-row decode is already done: `converted` (`crates/nvs-stdlib/src/db/row.rs:321`) has
real arms for all three new wire types, because a driver hands a `DECIMAL` over as a `Tag::Decimal`
and a `BLOB` as a `Tag::Bytes` already.

Nothing is blocked. Stage 1 is the carried floor and the driver's to run.

## Next group

**Stage 6: the row decoders, the `nvs-stdlib` half** — one file set:
`crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/db/execute.rs`,
`crates/nvs-stdlib/src/db/column.rs` and `crates/nvs-stdlib/src/db/row.rs`. `rule:types/decimal`
owns the refusal a `NUMERIC` takes and `rule:core-classes/db-column-types` the map both doors read.

- [ ] **A JSON decimal field round-trips its digits.** `crates/nvs-stdlib/src/json.rs:1489`
      `undecoded` is the one roster to take `CodecTy::Decimal` off, and
      `crates/nvs-stdlib/src/json.rs:2177` `scalar` is where the arm goes. The wire form is already
      decided in that module's gap 1 — a JSON **string**, `"12.50"` — so the decode is
      `nvs_runtime::Decimal::parse` over a `Tag::Str`, and the encode side has to write the same
      spelling or `json_decode_into_a_decimal_field_round_trips_25_significant_digits` fails on the
      way out rather than the way in. `tests/conformance/core/json-decode-as-a-decimal-field-keeps-25-significant-digits.nvst`
      sits beside it.
- [ ] **A `NUMERIC(30,10)` throws naming the column rather than truncating.**
      `crates/nvs-stdlib/src/db/execute.rs:1089` is SQLite's three `ColumnType::Decimal` rows, which
      hand `Decimal::parse` a `None` today and drop to the generic refusal;
      `crates/nvs-db/src/pg.rs:1705` `into_value` is where PostgreSQL's does the same, and
      `crates/nvs-stdlib/src/db/column.rs:176` `column_value` is the one door that has the column
      name in hand to say it with — `a_numeric_30_10_value_past_decimals_range_throws_rather_than_truncating`,
      and `a_numeric_30_10_postgres_column_throws_on_read_rather_than_truncating` on a leg.
- [ ] **A hand-written `fromRow` is called.** `crates/nvs-stdlib/src/db/row.rs:123` `hydrate`
      throws `LogicError` on an empty `db_codec()`, so `Core\Db\Codec`'s own door is the one it
      never tries — `query_as_calls_a_hand_written_from_row`. Note this is the same knot
      `rule:core-classes/derive-generates-what-is-missing`'s hand-written half has on the JSON side,
      where no derived decoder calls one either.

## Backlog

- A literal type is `db_reachable` and erases to `CodecTy::Opaque`, so a well-formed
  `public true $flag;` on a `#[Db\Derive]` class now refuses at every `queryAs` —
  `crates/nvs-types/src/derive.rs`'s `codec_ty` catch-all against `db_reachable`'s `Ty::True`
  row.
- `crates/nvs-stdlib/src/json.rs`'s gap 1 is tagged `— owner: m8-stdlib-depth`, but stage 6's
  own check names `json_decode_into_a_decimal_field_round_trips_25_significant_digits`; one of
  the two is wrong about who owns the JSON decimal decoder.
- An inline-shape field on a `#[Db\Derive]` class now reports twice, `E0756` at the declaration
  and `E0806` at the call, and both name the same fix.
- `python tools/db-matrix.py --all` has not run since the four wire drivers' walks landed —
  `crates/nvs-stdlib/tests/db_stream.rs`.
