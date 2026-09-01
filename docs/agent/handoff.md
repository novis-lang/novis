# Handoff

## State

**PostgreSQL's opening is on disk and green.** `crates/nvs-db/src/pg.rs` is ADR 0132 § 3's three
exchanges — the `SSLRequest` upgrade, `NvsTls::over` on the same socket, then SASL — and `PgConn` now
carries a `Wire` (the stream plus its frame buffer), the `State` cell and the `CancelKey` the backend
hands over exactly once. That module's doc is the home of why channel binding is `unsupported`, why a
cleartext or MD5 request is refused, and why an `AuthenticationOk` with no exchange is not.

**Nothing can complete a handshake against `tests/db/compose.yaml`.** Those servers are self-signed
and `nvs_host::tls`'s anchor set has no seam for a private root, so a matrix case reaches the upgrade
byte and stops; `crates/nvs-db/src/lib.rs`'s module doc states it. That is why the exchange is
asserted in-crate against a SCRAM server that verifies the client's proof and signs its own final
message, and closing the seam is what turns ADR 0067's five-driver matrix on at all.

**`postgres-protocol` is taken**, with `bytes` and `fallible-iterator` — the two crates its surface is
spelled in. `THIRD-PARTY-LICENSES.txt` is regenerated and `--check-c-deps` still reads four in the
graph and four recorded, so `rusqlite` remains the one that moves it.

**Manifest gap, open a second session:** `[context] adrs` in `docs/agent/loop-goal.toml` names no
section of ADR 0132, so this session paid for §§ 2 and 3 by hand again. Add §§ 1-5.

## Next group

**The rest of the PostgreSQL driver**, all three in `crates/nvs-db/src/` and none of them outside it —
`pg.rs` is the file every one of them opens, so they are one group.

- [ ] **The extended-query state machine moves `State` through its four values** — ADR 0132 § 4 with
      ADR 0067 § 1: `Parse`/`Bind`/`Describe`/`Execute`/`Sync` written over `Wire::read_message`,
      `Executing` while a portal is in flight, `Streaming` until it drains, and `Poisoned` where a
      `Sync` cannot restore a message boundary. Anchors: `crates/nvs-db/src/pg.rs:208`,
      `crates/nvs-db/src/pg.rs:238`, `crates/nvs-db/src/conn.rs:111`.
- [ ] **The `?`/`:name` rewriter and `inList` expansion** — ADR 0067 § 5, in a new
      `crates/nvs-db/src/sql.rs`: `$1`…`$n` for PostgreSQL, the cache key that is SQL text *plus
      expansion arity*, and the `::` cast and jsonb `?` operator left alone. Anchors:
      `crates/nvs-db/src/pg.rs:348`, `crates/nvs-db/src/lib.rs:110`.
- [ ] **ADR 0067 § 13's PostgreSQL reset** — roll back, `RESET ALL`, `CLOSE ALL`, `UNLISTEN *`,
      `pg_advisory_unlock_all()`, drop the temp schema, and deliberately not `DISCARD ALL`; a
      connection that cannot prove it is clean is closed. Anchors: `crates/nvs-db/src/pg.rs:238`,
      `crates/nvs-db/src/conn.rs:165`.

## Backlog

- A trust anchor a matrix case can verify against — `crates/nvs-host/src/tls.rs:270`, ADR 0132 § 3's
  "future `nvs.toml` anchor bundle". Blocks every driver's first real connection.
- `Core\Db`'s registry rows and helper bodies in `nvs-stdlib` — what the goal's three fixtures are
  red at (`E0405`), loop-goal stage 3.
- ADR 0067 § 9's type map, Novis side — `docs/adr/0067-core-db.md` § 9.
- The per-core pool and its acquire path — ADR 0067 § 13.
- The four remaining drivers, MySQL next — ADR 0132 § 2's table.
- `[context] adrs` in `docs/agent/loop-goal.toml` still names no ADR 0132 section.
