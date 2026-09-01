# Handoff

## State

**ADR 0067 § 4 is complete on PostgreSQL's wire.** `executeMany` is one `Parse` and a
`Bind`/`Execute`/`Sync` per set in a single flush, answering the summed affected count —
`crates/nvs-db/src/pg.rs`'s `execute_many` doc comment is the rule for both halves a caller can be
surprised by: **each execution carries its own `Sync`**, so a failure part way through does not roll
back the sets before it (one `Sync` for the whole batch would be § 4's refused hidden `BEGIN`, and
costs no fewer bytes), and an empty set list is § 4's no-op that never reaches the wire.

**A `Parse` is undone with the implicit transaction it ran in**, which is why the statement cache is
written only after a batch that drew no error at all — a name cached over a rolled-back parse is a
`26000` on the next hit, on a connection that is otherwise fine. `start_statement` commits earlier
(at the row description) and has the same exposure if the `Execute` then fails; it is in the backlog
rather than fixed here, because the fix threads the cache into `PgRows`.

**§ 4's refusal now has one home and names both fixes.** `second_statement` builds it for every
entry point that can refuse — `->all()` *or* a `{shared: false}` connection, which fix different
programs — and `nvs-stdlib` re-words it as § 4's `LogicError`, per `conn.rs`'s
`State::may_start_statement`.

Unchanged and pointed at rather than restated: the `[db.<name>]` readers are
`crates/nvs-db/src/sql.rs` beside `StatementCache`, and nothing resolves a block into a `PgTarget`
yet, so `time_zone_for`'s `None` still has no boot refusal. Nothing can handshake against
`tests/db/compose.yaml` (self-signed, no anchor seam in `nvs_host::tls`), and the goal's three
fixtures stay red at `E0405` because `Core\Db\Connection` has no stdlib rows — this goal's ordinary
state and what the driver's acceptance check reports every iteration.

**Manifest gaps, in `docs/agent/loop-goal.toml`:** `[context] adrs` still names no section of ADR
0132 (add §§ 1-5) and none of ADR 0067 §§ 2, 4, 5 — § 4 is the goal's central section and has now
been read by hand twice; § 7 was read by hand this session to name the next group and should join
it. `[context]` still reaches nothing under `docs/spec/`, which is where `lastId`'s `?uint` and
`Core\Db\Connection`'s class table live.

## Next group

**§ 7's transaction, on the wire** — the file set is `crates/nvs-db/src/pg.rs` alone, in the same
statement-sequencing half § 4 just finished, and `reset_session` at
`crates/nvs-db/src/pg.rs:2396` is the model for all three: a fixed list of simple `Query` messages
in one flush, each carrying its own implicit `Sync`.

- [ ] **`BEGIN`/`COMMIT`/`ROLLBACK` as the driver half of § 7's closure** — the closure itself is
      `nvs-stdlib`'s; this is the three commands, with `{isolation, readOnly}` rendered into the
      `BEGIN` and `Isolation`'s five levels mapped onto PostgreSQL's four (`Snapshot` is
      `REPEATABLE READ`). Beside `crates/nvs-db/src/pg.rs:2396`, refusing on a busy connection
      through `crates/nvs-db/src/pg.rs:2520`'s `second_statement`.
- [ ] **Nesting is `SAVEPOINT`/`ROLLBACK TO SAVEPOINT`/`RELEASE`, named by depth** — § 7 gives no
      explicit savepoint API, so the depth counter belongs to the connection beside its state at
      `crates/nvs-db/src/pg.rs:2066`'s neighbours, not to the caller.
- [ ] **The retry rule's driver half: classify a refusal's `SQLSTATE`** — § 7 re-runs on deadlock
      and serialization failure only (`40P01`, `40001`), so `crates/nvs-db/src/pg.rs:677`'s
      `server_error` has to hand the kind back rather than only the sentence. Backoff and the
      re-run stay `nvs-stdlib`'s.

## Backlog

- `start_statement`'s cache commit is one `Execute` too early — see `## State`; the fix threads
  `&mut StatementCache` into `PgRows`, whose fields are already disjoint borrows of `PgConn`.
- § 13's per-core pool, the reset that is a security boundary, and `pool = false` — ADR 0067 § 13,
  over `crates/nvs-db/src/conn.rs`; `reset_session` is already written and takes `self`.
- `Core\Db\Connection`'s stdlib rows, which is what the goal's three fixtures need — the class
  table in `docs/spec/01-core-library.md` § 6.
- The other four drivers, each its own state machine over the shared wire shape — ADR 0132 § 2.
- Resolving a `[db.<name>]` block into a `PgTarget`, with the boot refusal `time_zone_for`'s
  `None` is waiting for — `nvs-config`'s resolver owns every other boot error.
