# Handoff

## State

**Goal 21, stage 5 is the live stage**, and the driver's acceptance run still stops there: `native
examples/stream.nvs [5 db tail]` is item 8, its fixture is still session 0003's stand-in, and only 57
of the goal's ~364 checks are reached. Nothing regressed.

**Item 8's first slice is on disk.** A PostgreSQL statement's read state — columns, command tag,
`lastId`, ADR 0067 § 11's span — is now `PgCursor` (`crates/nvs-db/src/pg.rs:2283`) and is no longer
part of a borrow. `PgRows` keeps one beside its `&mut Wire`, which is the buffered path unchanged;
`PgConn` keeps an `Option<PgCursor>` (`crates/nvs-db/src/conn.rs:675`) for the held-cursor path; both
drive the one free function `next_row_of`, so the two cannot disagree about what ends a stream. The
wire surface slice 2 builds on is `PgConn::stream`, `stream_columns`, `stream_next_row`,
`stream_span`, `name_stream_connection` and `end_stream` (`crates/nvs-db/src/pg.rs:537`).
`State::Streaming` is now what refuses a second statement on a held cursor, which is the fixture's
`LogicError` line; § 13's reset drops the parked cursor, because "no open cursor" is one of the
properties it must establish and a command tag is one request's.

**Item 9 is where session 0004 left it** — `close`, `driver`, `isOpen` landed; `serverVersion` is
owed for a reason recorded as gap 5 in `crates/nvs-stdlib/src/db/mod.rs`.

Nothing is blocked on a decision.

## Next group

**Item 8's two remaining slices**, in this order. One file set:
`crates/nvs-stdlib/src/db/registry.rs`, `crates/nvs-stdlib/src/db/execute.rs`,
`crates/nvs-stdlib/src/instance.rs`, `crates/nvs-stdlib/src/cursor.rs`, `examples/stream.nvs`.

- [ ] **`Core\Db\Connection::stream` answers an `Iterable<Db\Row>` over the parked cursor** — the
      spec row is `docs/spec/01-core-library.md:1181`,
      `stream(string $sql, array<mixed> $params, {timeout?, chunk?: uint}): Iterable<Db\Row>`, a
      **sink** on `$sql`. The five edits: the registry row beside `close` at
      `crates/nvs-stdlib/src/db/registry.rs:310`, its `MethodDoc`, the body beside
      `crates/nvs-stdlib/src/db/execute.rs:169`'s `nvs_core_db_connection_query`, the `address()` arm,
      and three `.nvst` cases. What it answers is a **second internal class**, not
      `crates/nvs-stdlib/src/cursor.rs:102`'s `over` — that one walks a snapshot, and a streamed row
      does not exist until `advance()` asks for it — so it joins
      `crates/nvs-stdlib/src/instance.rs:83`'s `INTERNAL_CLASSES` with its own `advance`/`current`
      pair at `crates/nvs-stdlib/src/instance.rs:193`. Its slots hold the connection, which must stay
      filed (`crates/nvs-stdlib/src/db/pool.rs:262`'s `filed_connection`) for the cursor's whole life
      and be released on the `advance()` that answers `false`; `crates/nvs-stdlib/src/db/row.rs:122`'s
      `hydrate` is what one row becomes. ADR 0067 §§ 4 and 18.
- [ ] **`examples/stream.nvs` becomes the program its own comment describes** — `examples/stream.nvs:1`
      is the stand-in and its comment is the specification; the three frozen lines are
      `docs/agent/loop-goal.toml:4128`. **Its second line is the open question**: "rows held at once:
      1" has to be read off the connection rather than counted by the program, and this goal's
      standing decisions let it amend ADR 0067 § 13, § 3 and spec § 12 only — so it must be spelled
      with something that already exists rather than a new § 18 row. Decide that before writing the
      fixture. ADR 0067 § 4.

## Backlog

- `chunk?: uint` on the spec's `stream` row is unimplemented: `open_portal` sends `Execute(…, 0)`, so
  a portal is never suspended — `crates/nvs-db/src/pg.rs`'s module doc, and § 4 for what `chunk` owes.
- Only PostgreSQL has a parked cursor. MySQL, SQLite and TDS have the row-at-a-time reader but not the
  split — `crates/nvs-db/src/mysql.rs:3006`, `crates/nvs-db/src/sqlite.rs:482`,
  `crates/nvs-db/src/tds/rows.rs:177`. Item 8's fixture is PostgreSQL, so this is its tail, not its
  blocker.
- `streamAs<T>` is spec `docs/spec/01-core-library.md:1182` and has no registry row —
  `crates/nvs-stdlib/src/db/mod.rs`'s gap 5.
- Item 9's `serverVersion`: no driver keeps the server's version string, same gap 5.
