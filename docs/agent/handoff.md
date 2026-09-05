# Handoff

## State

**Stage 5's statement timeout is landed, all three slices.** ADR 0067 § 4's
`{timeout?: Duration}` is a **deadline on the connection's socket** — the decision, the two
candidates and why the server-side `statement_timeout` lost are recorded in
`crates/nvs-stdlib/src/db/mod.rs`'s own `# A statement's `timeout` is a deadline on the socket`
section, which is that gap's home now. The seam is `nvs_host::net::Deadline`, a two-method trait
that reaches a socket through however many wrappers are over it, forwarded by `NvsTls<T>` and by
`nvs-db`'s TDS `Tunnel`; `nvs_db::Connection::set_deadline` is the five-arm door, and SQLite's arm
is `sqlite3_busy_timeout` because a lock wait is the only wait that backend takes. The option is on
all ten registry rows — five members declared twice, see the playbook — and `bound_connection` is
the one place a statement path reaches a connection, so a call that named no timeout lifts the one
before it and `warm_connection` lifts it again before § 13's reset.

Nothing is blocked on a decision. The next acceptance failure the driver reports outranks the group
below.

## Next group

**What `crates/nvs-stdlib/src/db/mod.rs`'s known gaps still owe on § 18's roster.** One file set:
`crates/nvs-stdlib/src/db/mod.rs`, `crates/nvs-stdlib/src/db/registry.rs`,
`crates/nvs-stdlib/src/db/stream.rs`, `crates/nvs-db/src/pg.rs`.

- [ ] **`stream`'s `{chunk?: uint}`, the other half of gap 6.** `crates/nvs-stdlib/src/db/mod.rs:248`
      states it: a chunk size has to reach the `Execute` that asks for a row count, and
      `crates/nvs-db/src/pg.rs:584`'s `stream` asks for one row. The registry rows are
      `crates/nvs-stdlib/src/db/registry.rs:348` and `crates/nvs-stdlib/src/db/registry.rs:633` —
      both, for the playbook's reason — and the member body is
      `crates/nvs-stdlib/src/db/stream.rs:228`. Decide first whether a chunk is worth a second
      `Execute` shape at all: § 4 promises constant memory, which one row already gives.
- [ ] **`serverVersion`, gap 5.** `crates/nvs-stdlib/src/db/mod.rs:237` is the inventory: no driver
      keeps the server's own version string, so the member cannot be written until PostgreSQL's
      `server_version` `ParameterStatus` is held on the connection —
      `crates/nvs-db/src/pg.rs:584` is the driver, and the field belongs beside `time_zone` on
      `crates/nvs-db/src/conn.rs:637`'s `PgConn`. One driver at a time is fine; the member throws
      for a backend that has not landed its half, as `stream` already does.

## Backlog

- A live-server assertion that a `timeout` really expires mid-statement — `tools/db-matrix.py`'s,
  not a `-p` check's; `crates/nvs-db/tests/handshake.rs` is the nearest shape.
- `orient.py` printed ADR 0067 § 13 and § 3 only; this item needed § 4 and § 11. Add `0067 §4` and
  `0067 §11` to `[context] adrs` in `docs/agent/loop-goal.toml`.
- The eight spec §§ 16-17 classes have no owner on the chain — `docs/agent/carried-gaps.md`.
