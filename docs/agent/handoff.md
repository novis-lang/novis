# Handoff

## State

**ADR 0067 § 7 is complete on PostgreSQL's wire, and § 8's kind rides with every refusal.**
`BEGIN`/`COMMIT`/`ROLLBACK` are simple `Query` messages beside § 13's reset, each carrying its own
implicit `Sync`; `crates/nvs-db/src/pg.rs`'s `begin`, `commit`, `roll_back` and `begin_command` doc
comments are the rule for the three calls a caller can be surprised by — a **nested** transaction is
a `SAVEPOINT` named by the connection's own depth (`PgConn::depth`, because § 7 gives no savepoint
API and no `inTransaction()`), a nested one may **not** carry `{isolation, readOnly}` and is refused
rather than silently run at the outer level, and `Snapshot` renders as `REPEATABLE READ` because
that is the same guarantee under PostgreSQL's name for it. `READ WRITE` is never rendered: the
absence of `readOnly` is the server's default and not an override of it.

**§ 8's `DbErrorKind` and `ServerError` are `crates/nvs-db/src/conn.rs`**, carried *inside* the
`io::Error` every entry point already answers with, so a caller that prints the sentence is
unchanged and § 7's retry rule asks `ServerError::of` instead of matching text. PostgreSQL's code
table is `pg.rs`'s `kind_of`; `is_retryable` is § 7's two codes and nothing else, and the backoff and
the re-run stay `nvs-stdlib`'s.

**Nothing of `Core\Db` exists in `nvs-stdlib`** — there is no `db.rs` and no `Core\Db` row in
`crates/nvs-stdlib/src/registry.rs`, which is why the goal's three fixtures are red at `E0405` and
the driver's acceptance check reports it every iteration. That is now the whole of what stands
between this goal and its checks. Unchanged and pointed at rather than restated: nothing resolves a
`[db.<name>]` block into a `PgTarget` yet, so `time_zone_for`'s `None` still has no boot refusal,
and nothing can handshake against `tests/db/compose.yaml` (self-signed, no anchor seam in
`nvs_host::tls`).

**Manifest gaps, in `docs/agent/loop-goal.toml`:** `[context] adrs` names none of ADR 0067 §§ 2, 4,
7, 8 — § 7 and § 8 were both read by hand this session and are the sections the next slices cite —
and no section of ADR 0132. `[context]` still reaches nothing under `docs/spec/`, which is where
§ 18's `Core\Db` class tables are; they were read by hand this session too, and the next group is
written against them.

## Next group

**`Core\Db`'s stdlib surface, which is what the acceptance check is failing on** — the file set is a
new `crates/nvs-stdlib/src/db.rs` plus its two registrations, and `crates/nvs-stdlib/src/json.rs` is
the five-edit worked example the conventions name. The one slice below that is not in that set is
the resolver the third needs.

- [ ] **`Core\Db::inList` and `quoteIdentifier`, the two members that need no connection** — the
      spec's own table is `docs/spec/01-core-library.md:1140`, and the expansion `inList` produces
      already exists on the wire side as `crates/nvs-db/src/sql.rs`'s. New
      `crates/nvs-stdlib/src/db.rs`, registered in `crates/nvs-stdlib/src/registry.rs:1079` and
      declared beside its neighbours at `crates/nvs-stdlib/src/lib.rs:213`.
- [ ] **A `[db.<name>]` block resolved into a `PgTarget`** — the readers are already
      `crates/nvs-db/src/sql.rs`'s, and what is missing is the one that builds
      `crates/nvs-db/src/pg.rs:154`'s target from them, including the boot refusal `time_zone_for`'s
      `None` is waiting for. ADR 0067 § 2 is the naming rule.
- [ ] **`Core\Db::connect` and `open` over it** — `docs/spec/01-core-library.md:1140` for the two
      signatures, ADR 0067 § 2 for the memoization and `{shared: false}`, and both are `db.connect`
      /`db.open` capability sinks. Same `crates/nvs-stdlib/src/db.rs` and
      `crates/nvs-stdlib/src/registry.rs:1079`.

## Backlog

- `Core\Db\Connection`'s `Queryable` five as rows over `PgConn` — `docs/spec/01-core-library.md:1152`.
- `start_statement` commits a cache name at the row description, so an `Execute` that then fails can
  leave a name cached over a rolled-back `Parse` — `crates/nvs-db/src/pg.rs`'s `start_statement`.
- The four remaining drivers, each its own module beside `pg.rs` — ADR 0132 § 5.
- § 13's pool itself, per core, with `max`/`idle`/`lifetime`/`acquire` — ADR 0067 § 13.
- No anchor seam in `nvs_host::tls`, so nothing handshakes against `tests/db/compose.yaml`.
- `Db\Transaction implements Queryable by $connection` needs ADR 0043's `by` delegation on a `Core`
  type, which no `Core` type uses yet — ADR 0067 § 7.
