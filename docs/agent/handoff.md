# Handoff

## State

**Goal 20's Stage 4 has three of the four drivers.** MySQL opens the socket file as written,
PostgreSQL derives the engine's own name, and SQL Server refuses a path; MariaDB is the one left.
`crates/nvs-db/src/conn.rs`'s `Endpoint` is still the only meeting point and nothing about it moved.

**MSSQL's refusal is `TdsTarget::resolve`'s, not a dial's.** `BlockError::NoSocketTransport`
(`crates/nvs-db/src/conn.rs:322`) is raised where the block is read, because TDS has no `AF_UNIX`
transport on *any* platform, so there is no build where a path could reach a connect — the message
names `host` and the transport, and the case asserts it says nothing about opening a file, which is
`rule:core-classes/db-unix-socket-path`'s target-not-a-file distinction.

**PostgreSQL's derivation lives in `pg::socket_endpoint(directory, port)`
(`crates/nvs-db/src/pg.rs:466`), not in `connect`.** That was the open question, and this is why:
`PgTarget` carries no port, and pushing one onto it would put an address back in a type whose doc
says the address is not there. So the derivation sits beside the driver's other protocol trivia and
`PgConn::connect` dials what it is handed, exactly as MySQL's does. `PgStream`
(`crates/nvs-db/src/pg.rs:375`) is the transport enum — no TLS on the local arm, same argument as
`MyStream`'s — and it is now the default type parameter of both `Wire` and `PgRows`.

**The door picks the spelling.** `endpoint_of` in `crates/nvs-stdlib/src/db/open.rs:328` takes the
driver's own `fn(&str, u16) -> io::Result<Endpoint>`: `mysql::socket_endpoint` takes the port and
ignores it (a MySQL socket file has no convention to derive from), `pg::socket_endpoint` uses it.
The rustdoc gate is green again — the link to `Endpoint::Socket` was unresolvable on the build
where that variant does not exist, which is the property the sentence was describing. Nothing is
blocked.

## Next group

**Stage 4: the last driver and the agreement case** — one file set: `crates/nvs-db/src/maria.rs`,
`crates/nvs-db/src/pg.rs`, `crates/nvs-stdlib/src/db/open.rs`.

- [ ] **MariaDB's socket arm** — `crates/nvs-db/src/maria.rs:352`, still a `SocketAddr`. Widen to
      `impl Into<Endpoint>` exactly as `crates/nvs-db/src/mysql.rs:1566` is: MariaDB already shares
      `MyStream`, `read_greeting` and `REQUIRED_OVER_A_SOCKET`, so the arm is the same twenty lines
      with no decision in it, and `rule:core-classes/db-unix-socket-path` puts it on the socket
      **file**. Then the door's own arm, `crates/nvs-stdlib/src/db/open.rs:400`, swaps `address_of`
      for `endpoint_of(target.host, block.port, maria::DEFAULT_PORT, mysql::socket_endpoint)`.
- [ ] **`a_driver_answers_the_same_over_either_transport`** — the fourth name in the stage's check,
      and the only one still missing. **The shape, so it is not re-derived:** `crates/nvs-db/src/pg.rs:3707`'s
      `Peer` answers a flushed message with a closure's bytes, so one scripted query can be asked
      twice — once over `Wire<Peer>` and once over a `Wire<PgStream::Local>` whose far end is a
      thread that accepts on a `UnixListener`, reads once and writes the *same* canned reply — and
      the two answers asserted **equal** rather than each asserted right. `#[cfg(not(unix))]` gets
      the half the other two cases have. Goal prose `docs/agent/loop-goal.md:86` wants agreement
      across transports as the property a second transport must have.

## Backlog

- The matrix's socket leg for the three drivers — goal prose `docs/agent/loop-goal.md:86`, and it
  needs a container, so it is not a unit-test slice.
- `[context] modules` names no `nvs-stdlib/src/db/*` pattern, so the door this stage edits every
  session is absent from the map; the driver's sweep should now carry it.
- `Core\Net`'s `net.local` stays off the roster until there is a caller —
  `rule:config/net-local-is-named-and-not-on-the-roster`.
