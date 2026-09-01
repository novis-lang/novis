# Handoff

## State

**ADR 0067 § 4's `query` is on `Core\Db\Connection`, and it is `Core\Db\Queryable`'s first
member** — `crates/nvs-stdlib/src/db.rs:858`. The order is: § 5's `rewrite` is handed one
`Binding` per element of `$params` (an `inList` contributing its width), and what comes back is
one `Source` per marker, so the values go out in the *statement's* order and nothing here counts
placeholders. Each is rendered by `nvs_db::encode` (`crates/nvs-db/src/pg.rs:1529`), the text-format
twin of `PgColumn::decode`, and every row is read before the member answers, which is § 4's
buffered default.

**`Core\Db\Rows` is a one-slot handle until its readers land** — `crates/nvs-stdlib/src/db.rs:277`,
and it is the temporary entry on `registry.rs:3476`'s `HANDLES`, which `Core\Db\Connection` left.
Its slot holds every row already decoded, each a string-keyed array of its columns — `Row::toArray`'s
own shape, so § 18's six members are readers over it and never a second decoder.

**§ 9's five structured columns refuse rather than decode.** A `DATE`, `TIME`, `TIMESTAMP`,
`TIMESTAMPTZ` or `UUID` is a `Core\Time`/`Core\Uuid` *instance* only `nvs-stdlib` can allocate;
`PgColumn::scalar` hands back the parsed components (`crates/nvs-db/src/pg.rs:1322`) and
`structured_column` throws until something builds them. `crates/nvs-stdlib/src/db.rs`'s known gaps
1 and 5-7 own that, `open`, and why the row declares no `{timeout?: Duration}`.

**The acceptance check is still red, one member further along**: `examples/transaction.nvs` reaches
`->first()` on the `Rows` it now gets, which is the next group's first item.

## Next group

**The result set's own surface — the file set is `crates/nvs-stdlib/src/db.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-db/src/pg.rs` and `crates/nvs-stdlib/src/time.rs`.**

- [ ] **`Core\Db\Rows`' six members and the `Core\Db\Row` they yield** — `docs/spec/01-core-library.md:1190`
      for the table, ADR 0067 § 6 for the readers. The rows go on `crates/nvs-stdlib/src/db.rs:277`
      (`ROWS`, which then leaves `crates/nvs-stdlib/src/registry.rs:3487`'s `HANDLES`) and a `ROW`
      class beside it over one string-keyed slot; `all`, `first`, `value`, `column`, `count`,
      `columns` read the slot `query` filled at `crates/nvs-stdlib/src/db.rs:858`, and `Row`'s
      `has`/`get`/`toArray` plus the eleven typed readers are lookups in it, each `?T`. The floor
      is three `.nvst` cases a member and they can run without a server — the receiver is built by
      the case's own `query` call only in the error legs, so pin the compile-time shape.
- [ ] **§ 9's five structured columns, which is what `date`/`time`/`instant`/`uuid` need** —
      `crates/nvs-db/src/pg.rs:1322`'s `PgScalar` already parses the components; build the
      instances in `nvs-stdlib` (`crates/nvs-stdlib/src/time.rs:1112` is `INSTANT_NAME` and its
      class) and delete `crates/nvs-stdlib/src/db.rs:829`'s `structured_column` with its known gap.
- [ ] **§ 4's `execute` and the `Db\Write` it answers** — ADR 0067 § 4, `docs/spec/01-core-library.md:1158`.
      `nvs_db::PgRows::affected` and `last_id` are already on the wire half, so this is the same
      bind path as `query` with a different answer: a three-slot readonly carrier at
      `crates/nvs-stdlib/src/db.rs:277`'s neighbours.

## Backlog

- `Core\Db::open` waits on a `CoreTy` for a shape *parameter* — `crates/nvs-stdlib/src/db.rs`, gap 1.
- `query` declares no `{timeout?: Duration}`: a statement deadline needs a socket seam — gap 7.
- `Db\DbError`/`Db\RolledBack` are not in spec § 10's tree, so a server refusal is a bare
  `RuntimeError` — gap 4, and `examples/transaction.nvs` catches `RolledBack` by name.
- `transaction`, `stream`, `queryAs`, `executeMany` — ADR 0067 §§ 4 and 7, the rest of `Queryable`.
- The pool and its reset are Stages 3 to 7 — ADR 0067 § 13, `nvs_runtime::Ctx::hold_open_connection`.
- Only PostgreSQL connects; the other four have no connect path — `crates/nvs-stdlib/src/db.rs`, gap 2.
