# Handoff

## State

**Goal 20's Stage 4 is one item in: MySQL dials an address *or* a path.**
`nvs_db::Endpoint` is the shared vocabulary — `Tcp(SocketAddr)` and a `#[cfg(unix)]`
`Socket(PathBuf)`, in `crates/nvs-db/src/conn.rs` where the goal prose puts the meeting point — with
`is_socket_host` (a `host` beginning with a path separator, per
`rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host`) and `socket_endpoint`, whose
`#[cfg(not(unix))]` twin is the `Unsupported` refusal and never a loopback fallback.
`MySqlConn::connect` takes `impl Into<Endpoint>`, so the three remaining drivers widen the same way
without any TCP caller changing. That is the only shape that works: `crates/nvs-cli/src/schema.rs:378`
and `crates/nvs-cli/src/worker.rs:840` dial all five drivers through one macro body that binds
`address` itself, and `macro_rules` hygiene leaves no way for a per-driver argument to name it.

**`MyStream` is the transport enum, and the local arm carries no TLS.** That is a decision, recorded
in its doc comment in `crates/nvs-db/src/mysql.rs`: a path reaches the driver only where an operator
wrote one, it names no host for a certificate, and the bytes never leave the machine, so
`REQUIRED_OVER_A_SOCKET` drops `CLIENT_SSL` and the greeting's own `CLIENT_SSL` bit is masked off
before `authenticate` offers it back. The in-band upgrade stays mandatory on the TCP arm.
`read_greeting` takes the required set from its caller because it is the transport's requirement, not
the driver's.

`crates/nvs-stdlib/src/db/open.rs`'s `endpoint_of` closure is the one door a configured socket comes
through; the settings path beside it still resolves a pinned address only.
`a_mysql_socket_path_is_opened_as_written` drives a real dial against a bound `UnixListener` and
asserts the accept happened — run under WSL as well as on Windows, where the `#[cfg(not(unix))]` half
asserts `socket_endpoint`'s refusal instead. `rule:core-classes/db-unix-socket-path` now says
*shipped for MySQL only*. Nothing is blocked.

## Next group

**Stage 4: the remaining three drivers, over the `Endpoint` that is now landed** — one file set:
`crates/nvs-db/src/tds/mod.rs`, `crates/nvs-db/src/pg.rs`, `crates/nvs-db/src/maria.rs`.

- [ ] **`a_tds_target_refuses_a_socket_path`** — `crates/nvs-db/src/tds/mod.rs:381`, though the
      refusal is `TdsTarget`'s (`crates/nvs-db/src/tds/mod.rs:221`) rather than the dial's:
      `rule:core-classes/db-unix-socket-path` says MSSQL reports a path as **a target it does not
      speak**, not as a file it could not open, so it is a `BlockError` arm in
      `crates/nvs-db/src/conn.rs:258` beside the others. Cheapest of the three, and it needs no
      transport at all.
- [ ] **`a_postgres_socket_directory_becomes_the_engines_own_name`** —
      `crates/nvs-db/src/pg.rs:485`, widened to `impl Into<Endpoint>` exactly as
      `crates/nvs-db/src/mysql.rs:1545` now is. **The open design question, so it is not re-derived:**
      the derivation `<host>/.s.PGSQL.<port>` needs the port and `PgTarget` carries none — its doc
      says the address is not here. `Endpoint::Socket` holds a directory, so either `connect` takes
      the port beside the endpoint or `pg` grows a `socket_path(dir, port)` the test asserts on its
      own; the goal prose (`docs/agent/loop-goal.md:83`) wants the derivation asserted against the
      path the driver connects to, which the second spelling gives directly.
- [ ] **`a_driver_answers_the_same_over_either_transport`** — `crates/nvs-db/src/pg.rs:485` again,
      after the one above: one question asked over both arms, asserting that they **agree**.
- [ ] **MariaDB's own socket arm** — `crates/nvs-db/src/maria.rs:352`, still `SocketAddr`. It already
      shares MySQL's `MyStream` and `read_greeting`, so this is the same edit a second time with no
      new decision in it; `rule:core-classes/db-unix-socket-path` puts MariaDB with MySQL, on the
      socket file rather than a directory.

## Backlog

- `nvs-cli`'s two openers — `crates/nvs-cli/src/schema.rs:371` and `crates/nvs-cli/src/worker.rs:832`
  — still resolve `host` as a name, so a socket-configured `[db.<name>]` is unreachable from
  `nvs schema` and the queue worker.
- `nvs-config`'s boot-time refusal of a Unix `host` where there is no `AF_UNIX` transport — stage 4
  item 1 of `docs/agent/loop-goal.md:75`, caught at open time by `socket_endpoint` for now.
- The matrix's socket leg for the three drivers — `docs/agent/loop-goal.md:86`;
  `tests/db/compose.yaml` is where a published socket goes.
- `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it` and its two neighbours stay
  `designed` until the driver half is complete.
