# Handoff

## State

**Goal 20's Stage 3 is landed with all three of its proofs.** A store spelled `unix:` in
`[cache.shared] url` is dialled over a socket and answers the same RESP, byte for byte, as one
spelled `redis://` — `a_record_written_through_a_socket_is_read_back_through_it` in
`crates/nvs-stdlib/src/cache/redis.rs` drives the exchange against a `UnixListener` where there is a
transport and asserts the door's refusal where there is not.

`nvs_runtime::capability::pinned_address` now refuses a **path** in front of the resolution, so a
program-supplied socket path is refused as a target this deployment cannot authorize and never as a
file that would not open; the refusal names `net.local`
(`rule:config/net-local-is-named-and-not-on-the-roster`) and reads the same whether or not anything
is bound at the path. `a_program_supplied_socket_path_is_never_a_target` in
`crates/nvs-stdlib/src/cache.rs` asserts that pair against the other authority: the same path an
operator wrote into the store's own `url` is an ordinary store.

`examples/cache-shared-socket.nvs` runs under `examples/cache-shared-socket.toml` — its own tree,
because `[cache.shared]` is unscoped — against the socket `tests/db/compose.yaml`'s `redis` service
now publishes at `/mnt/wsl/novis-redis/redis.sock`. Its `[[check]]` carries **`needs = "af-unix"`**,
a new optional key on the four program kinds: `tools/loop.py`'s `LEG_NEEDS` asks the leg rather than
naming it, so a Linux-native host runs the fixture on its own leg instead of being skipped for not
being called `wsl`. Nothing is blocked. The `unix:` and `[db.<name>] host` rules stay `designed`
until Stage 4 lands the driver half.

## Next group

**Stage 4: the driver transport, the same target-or-path one layer down** — one file set:
`crates/nvs-db/src/pg.rs`, `crates/nvs-db/src/mysql.rs`, `crates/nvs-db/src/tds/mod.rs`,
`crates/nvs-config/src/db.rs`.

- [ ] **`a_mysql_socket_path_is_opened_as_written`** — `crates/nvs-db/src/mysql.rs:1426`, the dial
      that takes a `SocketAddr` today, widened to the address-or-path `cache/redis.rs`'s `Transport`
      already is, per `rule:core-classes/db-unix-socket-path`. A bare absolute path in
      `[db.<name>] host` is the spelling; `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host`
      is why it is not `unix:` here.
- [ ] **`a_postgres_socket_directory_becomes_the_engines_own_name`** —
      `crates/nvs-db/src/pg.rs:486`. PostgreSQL names a *directory* and appends `.s.PGSQL.<port>`
      itself, which is the one place the two drivers do not agree; `rule:core-classes/db-unix-socket-path`
      states it and `crates/nvs-config/src/db.rs` is where the block's paths are already resolved.
- [ ] **`a_tds_target_refuses_a_socket_path`** — `crates/nvs-db/src/tds/mod.rs:218`. MSSQL has no
      socket transport, so the refusal is the feature; `rule:core-classes/db-unix-socket-path` names
      it, and the shape is `a_unix_url_is_refused_where_the_platform_has_no_transport`.
- [ ] **`a_driver_answers_the_same_over_either_transport`** — `crates/nvs-db/src/pg.rs:486`. The
      agreement case: one question asked over both transports, asserting they answer the same rather
      than what either answered. `tests/db/compose.yaml`'s `postgres` service needs the socket mount
      its `redis` sibling now has, and `[valgrind] skip` is where a fixture that cannot carry its own
      `--config` goes.

## Backlog

- `nvs_config::store::advise` reads only the **root** `[capabilities]`, so any deployment granting
  `cache.shared` in an `[[app]]` block gets W1008 at boot — `nvs.toml` and `examples/cache.nvs` do
  today. Its home is `crates/nvs-config/src/store.rs`.
- `[docker] services` does not name the socket mount as a dependency of anything; a machine that
  brings `redis` up by hand gets the socket for free and one that does not is skipped by `needs`.
- The `[cache.shared]`-over-TLS question is still open — `rediss://` is refused rather than
  half-served, per `crates/nvs-stdlib/src/cache.rs`'s module doc.
