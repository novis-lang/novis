# Handoff

## State

**Stage 5's pool item is landed, both halves.** An `open`'s bounds are the `[db.<name>.pool]` of the
block whose *settings hash* is that connection's — `crates/nvs-stdlib/src/db/pool.rs`'s
`settings_bounds`, over `open.rs`'s `block_settings_key`, which builds each block's key through
`settings_key` itself so the pool and the memo cannot disagree about what "the same connection" is.
And `pool = false` is now writable **unscoped**: `[db]` is a `nvs_config::tree::Databases` — the
switch beside a flattened map of the blocks, `Deref`ing to that map so `config.db.get("main")` is
unchanged — and `nvs_config::db::bounds_for` is the one place the switch is read, so it reaches
`connect`, a matching `open` and a matching-nothing `open` alike. An unscoped `[db.pool]` *table* of
bounds is a boot refusal rather than a second meaning. ADR 0067 § 13 carries all of it now.

Stage 5's last check is the statement timeout, and it is a feature before it is a test. Nothing is
blocked on a decision.

## Next group

**Stage 5 — `a_statement_timeout_reaches_the_socket_and_throws_on_expiry`, which is one gap in three
slices.** One file set: `crates/nvs-stdlib/src/db/mod.rs`, `crates/nvs-stdlib/src/db/registry.rs`,
`crates/nvs-stdlib/src/db/execute.rs`, `crates/nvs-db/src/pg.rs`.

- [ ] **Decide which timeout it is, and say so in the gap that asks.** `crates/nvs-stdlib/src/db/mod.rs:196`
      is gap 6 and states the blocker exactly: § 4's `{timeout?: Duration}` is in the spec signatures
      and in no registry row, because a deadline on a statement has no seam to reach the socket
      through. The candidates are a deadline on the *socket* (what `nvs_db::PgConn::connect` already
      takes) and a server-side `statement_timeout`; ADR 0067 § 11's `slow_query` is the neighbour
      that is neither. The socket deadline is the one that holds on all five drivers.
- [ ] **The seam on the statement path**, mirroring the one the handshake has —
      `crates/nvs-db/src/pg.rs:2822` is `start_statement`, the free function over the stream that a
      `-p nvs-db` test can already script a server for (the playbook's `PgConn` bullet is why it is
      shaped that way).
- [ ] **The row, then the test.** `crates/nvs-stdlib/src/db/registry.rs:317` is the comment saying
      § 18's `{timeout?, chunk?: uint}` is deliberately absent; the four executing members are
      `crates/nvs-stdlib/src/db/execute.rs`. A `-p nvs-stdlib` test cannot build a connection
      (playbook), so the assertion is over the split-out tail, as `pool.rs`'s two new cases are.

## Backlog

- `stream`'s `{chunk?: uint}` is the other half of gap 6 — `crates/nvs-stdlib/src/db/mod.rs:196`.
- An `open` naming an endpoint no block describes still takes the default bounds, and where those
  would be *written* is an open ADR 0067 § 13 question — `crates/nvs-stdlib/src/db/mod.rs` gap 1.
- `docs/agent/carried-gaps.md` lost its `[db.<name>.pool]` row; the rest of that table is untouched.
