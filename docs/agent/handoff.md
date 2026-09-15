# Handoff

## State

**Goal `m8-db-queue`, stage 5 is closed.** `streamAs<T>` is on both `Queryable` classes and runs end
to end: `Core\Db\Stream` is generic in `T`, `stream` answers it at `Core\Db\Row` and `streamAs` at the
call site's own class, and the walk builds one `T` per step in `park_row`
(`crates/nvs-stdlib/src/db/stream.rs:131`) where `queryAs` builds one per row handed out. The
call-site refusals are `E0806` while compiling on both members now — `check_row_sites` reads
`streamAs` sites as it reads `queryAs`'s — and the run-time pair stays as the backstop for a class
built by hand. `serverVersion` closed last session and is unchanged.

`tests/conformance/core/db-stream-as-answers-the-class-it-was-written-with.nvst` runs the whole member
against SQLite. The four wire drivers' walks are `crates/nvs-stdlib/tests/db_stream.rs` under `python
tools/db-matrix.py --all`, which was not run this session — nothing in this group touches a wire.

Nothing is blocked. Stage 1 is the carried floor and the driver's to run.

## Next group

**Stage 6: the row decoders** — one file set: `crates/nvs-types/src/derive.rs`,
`crates/nvs-runtime/src/object.rs`, `crates/nvs-stdlib/src/db/row.rs` and the two conformance cases
the stage names. `rule:core-classes/derive-attribute` owns the mapping, `rule:types/decimal` the
refusal a `NUMERIC` takes, and `rule:core-classes/db-column-types` the map both doors read.

- [ ] **A reachable field type erases to its own codec type.** `crates/nvs-types/src/derive.rs:533`
      `codec_ty` folds `decimal`, `instant` and `bytes` to `CodecTy::Opaque` today
      (`crates/nvs-types/src/derive.rs:562`); the cases to add are
      `nvs_runtime::CodecTy`'s own (`crates/nvs-runtime/src/object.rs:607`), which is what
      `a_decimal_an_instant_and_bytes_field_erase_to_their_own_codec_type` asks for.
- [ ] **An inline-shape field is refused while compiling.** The site is recorded at
      `crates/nvs-types/src/expr/args.rs:1659` and judged at `crates/nvs-types/src/derive.rs:717`
      `check_row_sites`, which is where the fourth `E0806` condition goes —
      `a_query_as_over_an_inline_shape_field_is_refused_while_compiling`, with
      `tests/conformance/reject/db-query-as-over-an-inline-shape-field-is-refused-while-compiling.nvst`
      beside it.
- [ ] **The three types decode per row, and a `NUMERIC` that will not fit throws naming the column.**
      `crates/nvs-stdlib/src/db/row.rs:226` `hydrated` is the per-field check and
      `crates/nvs-stdlib/src/db/column.rs` the map it reads —
      `a_numeric_30_10_value_past_decimals_range_throws_rather_than_truncating` and, on a leg,
      `a_numeric_30_10_postgres_column_throws_on_read_rather_than_truncating`.
- [ ] **A hand-written `fromRow` is called.** `crates/nvs-stdlib/src/db/row.rs:122` `hydrate` refuses
      a class with an empty `db_codec()`, so `Core\Db\Codec`'s own door is the one it does not try —
      `query_as_calls_a_hand_written_from_row`.

## Backlog

- A closure's `foreach` binding is recorded as a capture of an enclosing name of the same spelling and
  panics `nvs-ir` lowering — `crates/nvs-ir/src/lower/expr.rs:2667` is where it lands; no gap entry
  owns it yet.
- `crates/nvs-stdlib/src/db/mod.rs`'s gaps 3 and 4 are both this goal's and both now record landed
  work; closing them means renumbering, and `gap 4`/`gap 8` are named from `row.rs` and `execute.rs`.
- `[context] modules` did not name `crates/nvs-types/src/expr/args.rs` or
  `crates/nvs-types/src/intrinsics.rs`, which stage 5's compile-time refusal had to edit.
