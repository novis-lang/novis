# Handoff

## State

**PostgreSQL runs statements and resets, and both are on disk and green.** `crates/nvs-db/src/pg.rs`
now holds ADR 0132 § 4's state machine — `start_statement` writes Parse/Bind/Describe/Execute/Sync
into one buffer and flushes once, `PgRows` is the `Streaming` borrow, and every path back to `Idle`
goes through the `ReadyForQuery` that the batch's own `Sync` guarantees. `reset_session` is ADR 0067
§ 13's six commands pipelined as simple `Query` messages, one more round trip.

**Three calls this session recorded in that module's doc rather than in an ADR**, because § 13 states
properties and not command lists and 0132 § 4 delegates the choice to the driver: `Bind` sends and
requests **text** format (§ 9 maps `interval`/`hstore`/ranges/`inet` to `tainted string` *as the
server renders them*, which no binary body can produce); **abandonment is not poison** (the `Sync` is
already on the wire, so the drain is deterministic — § 4's per-driver call, decided in PostgreSQL's
favour, and poison is now only ever a wire that failed); and `DISCARD TEMP` is the spelling of § 13's
"drop the session's temporary schema", chosen over `DISCARD ALL` for that section's own reason.

**`PgConn::reset` takes `self` by value**, so a failed reset cannot hand a connection back — the type
is the enforcement, deliberately *not* `State::Poisoned`, which answers "is the wire at a known
boundary" and would be false. `Connection::reset` must mirror the shape when the pool lands.

Unchanged: nothing can handshake against `tests/db/compose.yaml` (self-signed, no anchor seam in
`nvs_host::tls`), and the goal's three fixtures are red at `E0405` because `Core\Db\Connection` has
no stdlib rows yet — that is this goal's ordinary state, not a regression.

**Manifest gap, still open after two sessions:** `[context] adrs` in `docs/agent/loop-goal.toml`
names no section of ADR 0132, so § 4 was paid for by hand again. Add §§ 1-5.

## Next group

**The SQL layer over the state machine** — `crates/nvs-db/src/pg.rs`, a new `crates/nvs-db/src/sql.rs`
and `crates/nvs-db/src/lib.rs`, and nothing outside the crate. Item 1 creates the file the other two
read from.

- [ ] **ADR 0067 § 5's `?`/`:name` rewriter and `inList` expansion** — one placeholder spelling in,
      PostgreSQL's `$n` out, plus the parameter order and the **expansion arity** § 1's cache keys
      on. A new `crates/nvs-db/src/sql.rs`, declared beside its siblings at
      `crates/nvs-db/src/lib.rs:121`, feeding the `sql` and `params` of
      `crates/nvs-db/src/pg.rs:783`.
- [ ] **ADR 0067 § 1's LRU statement cache** — replaces the unnamed statement at
      `crates/nvs-db/src/pg.rs:602` with a name keyed by SQL text plus expansion arity, sized by
      `statement_cache`, held on the connection at `crates/nvs-db/src/conn.rs:165`. `Parse` is then
      skipped on a hit, which is the round trip § 1 says PostgreSQL already does not pay.
- [ ] **ADR 0067 § 9's type map, the PostgreSQL half** — `crates/nvs-db/src/pg.rs:611`'s
      `PgColumn::type_oid` to a Novis type, decoding the text-format column bodies
      `crates/nvs-db/src/pg.rs:645` hands back. § 9's table is the specification and `nvs-runtime`'s
      values are the target.

## Backlog

- The private-root anchor seam ADR 0132 § 3 calls a "future `nvs.toml` anchor bundle" — until it
  exists no driver can reach `tests/db/compose.yaml`. `crates/nvs-db/src/lib.rs`'s module doc.
- `Core\Db\Connection`'s registry rows and helper bodies, which is what closes the three fixtures at
  `E0405`. `docs/spec/01-core-library.md`, and ADR 0067 §§ 2-4.
- ADR 0067 § 7's transaction closure wants the last `ReadyForQuery`'s transaction-status byte, which
  nothing keeps; `reset_session` sends `ROLLBACK` unconditionally because of it.
- `Connection::reset` must take `self` by value, mirroring `crates/nvs-db/src/pg.rs:366`.
- `[context] adrs` in `docs/agent/loop-goal.toml` needs ADR 0132 §§ 1-5.
- The four remaining drivers, PostgreSQL-first order per the goal's standing decisions. ADR 0067 § 12.
