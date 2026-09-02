# Handoff

## State

**ADR 0067 § 11 is closed for `Core\Db`.** Every statement routine files ADR 0041's `query`
event — `query`, `queryAs`, `execute` and now `executeMany` — and a `[db.<name>] slow_query`
writes the same span to `Core\Log` as one `Warn` record. `QueryWatch`
(`crates/nvs-stdlib/src/db.rs:3526`) is the single reader of both halves, so a statement
renders its span at most once however many readers there are, and an unwatched statement pays
neither the rendering nor the clock.

**`executeMany` opens its own span** because `nvs_db::PgConn::execute_many` answers with a
count and lends no `PgRows` out; `crates/nvs-db/src/span.rs`'s module doc owns why
`QuerySpan::opened` is `pub` rather than `crate::pg`'s alone.

**The threshold is `nvs_config::db::slow_query_for`**, refused at boot by `nvs_config::db::validate`
and read per statement off the config snapshot. Unwritten costs a map lookup and nothing else —
`Setting` is only parsed for the block that opted in — and `Core\Db::open`, which has no config
block at all, is therefore never timed. `crates/nvs-config/src/tree.rs`'s field doc is the home
for what the key means.

**Two statement paths still bypass § 11**, both in the group below: `Core\Queue`'s own reads and
`transaction`'s `BEGIN`/`COMMIT`/`ROLLBACK`.

**Stage 9's other two items stay blocked three deep** — known gap 6 of
`crates/nvs-types/src/intrinsics.rs` — and stage 2's `local_infile_is_refused_and_no_file_is_sent`
still fails acceptance and always will until MySQL's driver exists.

**`orient.py` gaps:** `[context] modules` still wants an `nvs-runtime/src/ctx.rs` pattern, and now
also `nvs-config/src/db.rs` (plus its test file) and `nvs-stdlib/src/log.rs`, all of which this
session had to peek; `adrs` still wants `0041 § 1` beside ADR 0067 § 11.

## Next group

**The two statement paths § 11 does not reach yet — one file set:
`crates/nvs-stdlib/src/queue.rs` and `crates/nvs-stdlib/src/db.rs`, with `QueryWatch` at
`crates/nvs-stdlib/src/db.rs:3526` read by both.**

- [ ] **`Core\Queue`'s statements file a `query` event** — ADR 0067 § 11 over ADR 0084's tables.
      `statusOf` at `crates/nvs-stdlib/src/queue.rs:1465` and `statsOf` at
      `crates/nvs-stdlib/src/queue.rs:1636` drive `PgRows` directly rather than through
      `Core\Db`'s members, so a trace that shows every application query shows none of the
      queue's. Both hold the block name already — `crates/nvs-stdlib/src/queue.rs:1315` is where
      it is resolved — so `QueryWatch::of` takes it as `Core\Db`'s own reader does, and the
      `slow_query` half comes with it. `QueryWatch` is `pub(crate)`-able from `db.rs` or moves
      beside `postgres_of`; the queue's `push` and the worker's claim/report run their statements
      through the same helper and want the same treatment.
- [ ] **`transaction`'s three commands file nothing** — ADR 0067 § 7's `BEGIN`, `COMMIT`,
      `ROLLBACK` and the `SAVEPOINT` a nested call is, at
      `crates/nvs-stdlib/src/db.rs:4076`, go out over `nvs_db::PgConn::begin` and never reach a
      span, so a trace shows a transaction's statements with no transaction around them. Decide
      first whether § 11 means them at all — the ADR says *a query is a trace event* and a
      `COMMIT` is a statement — and record the answer on `QueryWatch` either way.

## Backlog

- MySQL's driver, which is what `local_infile_is_refused_and_no_file_is_sent` waits on — ADR
  0067's *Verification* section.
- `Core\Db::open`'s registry row and the shape-field intrinsic behind it — known gap 6 of
  `crates/nvs-types/src/intrinsics.rs`.
- ADR 0041 § 4's speedscope export, and § 2/§ 3's `gc`/`spawn` emitters — that ADR.
- ADR 0018's trace sink, which is what makes `Ctx::trace`'s vector and the text-rendered span
  temporary — `crates/nvs-runtime/src/ctx.rs:899`.
- A live proof of the `slow_query` line, which needs the matrix rather than a unit test —
  `crates/nvs-db/src/matrix.rs` is where a case finds a server.
