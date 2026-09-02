# Handoff

## State

**ADR 0067 § 11 now covers every statement path there is.** `Core\Db`'s four members already filed
ADR 0041's `query` event; this session added `Core\Queue`'s four (`push`, `status`, `cancel`,
`stats`) and § 7's three commands, so a trace of a request shows the `BEGIN`, the statements inside
it and the `COMMIT` that closed them, and the `slow_query` half comes with each of them.

**`QueryWatch` is `pub(crate)` and keyed on a block *name*** — `crates/nvs-stdlib/src/db.rs:3556`'s
`QueryWatch::named` takes `Option<&str>`, and `of` is the `Value`-holding caller's spelling of it.
`name_span` took the same turn. That is the whole of what the queue needed: its block comes from
ADR 0084 § 2's `[queue] connection` and there is no `Core\Db\Connection` to read a name off.

**A span with no `PgRows` behind it is filed by `file_span`**
(`crates/nvs-stdlib/src/db.rs:3638`) — `executeMany` and § 7's commands share it. § 7's three go the
other way from `executeMany`: `nvs_db::PgConn::begin`/`commit`/`roll_back` now answer
`io::Result<QuerySpan>`, because which command a nesting depth gets is the connection's answer and
`Core\Db` cannot spell `SAVEPOINT nvs_2` for itself. `crates/nvs-db/src/span.rs`'s module doc is the
home for both shapes.

**Nothing in the queue's half is asserted by a test**, and cannot be until the matrix runs a case:
a `-p nvs-stdlib` test cannot build a `PgConn`. The driver's half is —
`a_transaction_command_answers_the_span_of_the_command_it_sent`
(`crates/nvs-db/src/pg.rs:5949`) pins all three commands' text over the fake wire.

**Stage 2's `local_infile_is_refused_and_no_file_is_sent` still fails acceptance** and will until
MySQL's driver exists, which is the next group. Stage 9's other two items stay blocked three deep —
known gap 6 of `crates/nvs-types/src/intrinsics.rs`.

**`orient.py` gaps:** `[context] modules` wants `nvs-db/src/conn.rs` (it holds `Driver` and the five
connection structs, and the next group lives in it); `adrs` wants ADR 0132 §§ 2 and 5 and ADR 0067
§ 3, which are what the MySQL slices are specified by.

## Next group

**MySQL's first connection — one file set: `crates/nvs-db/src/conn.rs`, a new
`crates/nvs-db/src/mysql.rs` beside `crates/nvs-db/src/pg.rs`, and `crates/nvs-db/src/lib.rs`'s
module list. `crates/nvs-db/src/pg.rs` is the shape all three slices copy.**

- [ ] **The handshake, authenticated and UTF-8 forced** — ADR 0132 § 2 (which protocol crate backs
      this driver) and § 3 (the parking stream), ADR 0067 § 3. `MySqlConn` is the placeholder at
      `crates/nvs-db/src/conn.rs:464` and `Driver::MySql` at `crates/nvs-db/src/conn.rs:52`;
      `crates/nvs-db/src/pg.rs:898` is `authenticate`, the routine this one mirrors, and
      `crates/nvs-db/src/matrix.rs` is where a live case finds a server. Read ADR 0132 § 2 before
      picking anything: the crate is already chosen there.
- [ ] **`local_infile` is refused and no file is sent** — ADR 0067 § 3, and this is the one check
      the driver has failed every iteration of the loop. It is a property of the *client*: the
      capability flag is never set, and a server that asks anyway is answered with an empty packet
      rather than a file. The test name acceptance looks for is
      `local_infile_is_refused_and_no_file_is_sent`, `-p nvs-db`; `crates/nvs-db/src/pg.rs:5961` is
      the fake-wire harness (`Peer::new`) a case like it is written over.
- [ ] **One statement over `COM_STMT_PREPARE`/`COM_STMT_EXECUTE`** — ADR 0067 § 1's two round trips
      on the first execution and one on a cached re-execution, § 5's placeholder rewrite. The entry
      points are the `Connection` enum's arms at `crates/nvs-db/src/conn.rs:523`, and § 11's span
      rides on it from the start: `crates/nvs-db/src/pg.rs:2679` is where PostgreSQL opens one.

## Backlog

- `Core\Db::open`'s registry row and the shape-field intrinsic behind it — known gap 6 of
  `crates/nvs-types/src/intrinsics.rs`.
- A live proof of the `slow_query` line and of the queue's `query` events, which need the matrix
  rather than a unit test — `crates/nvs-db/src/matrix.rs` is where a case finds a server.
- ADR 0041 § 4's speedscope export, and § 2/§ 3's `gc`/`spawn` emitters — that ADR.
- ADR 0018's trace sink, which is what makes `Ctx::trace`'s vector and the text-rendered span
  temporary — `crates/nvs-runtime/src/ctx.rs:899`.
- § 7's retry has no wait between attempts — known gap 9 of `crates/nvs-stdlib/src/db.rs`.
