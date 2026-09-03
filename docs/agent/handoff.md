# Handoff

## State

**§ 7's retry waits now, and known gap 9 is gone from `crates/nvs-stdlib/src/db.rs`'s list**, which
ends at 8 with no renumbering — it was the last one. `retry_backoff` draws full jitter uniform in
`[0, 10ms × 2^taken]`, capped at one second because `retries` is a `uint`; `wait_between_attempts`
gives the core back for it and turns `Woken::Cancelled` into `Ctx::cancel`. Both conflict channels
wait before the re-run: the commit's `io::Error` and the closure's pending `Core\Db\DbError`.

**The RNG question closed with no dependency.** `rand` is already `nvs-stdlib`'s, for
`crate::http::transport`'s ADR 0074 § 6 jitter, so ADR 0051 § 4 is not reopened and the draw follows
that module's full-jitter shape rather than `crate::queue`'s id-mixed ladder — a transaction has no
id to mix, which `retry_backoff`'s own doc argues.

**`Host::sleep`, not the bounded park the last handoff named.** That trait's doc separates a wait for
the *clock* from a wait for a peer, and nothing wakes a backoff: a connection coming free says
nothing about a deadlock already broken. `wait_between_attempts`'s doc owns the reading, including
why the no-host arm blocks.

**The driver's stage-7 check is unchanged and still correctly filed.**
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` needs SQL Server and there is no TDS
driver — `TdsConn` in `crates/nvs-db/src/conn.rs` is a busy-state cell. Do not rename it, split it,
or write a MySQL-only test under that name; MySQL's half is already asserted by
`a_reset_invalidates_the_statement_cache` in `crates/nvs-db/src/mysql.rs`.

## Next group

**One file, `crates/nvs-stdlib/src/db.rs`, plus `crates/nvs-db/src/maria.rs` read-only.** The first
is small and self-contained; the second is the re-scoped MariaDB item — it is **not** the one-arm
change the previous handoff priced, and the playbook bullet added this session is why.

- [ ] **§ 8's `sql` is the fifth raw value and no throw carries it** (0067 § 8).
      `crates/nvs-stdlib/src/db.rs:3653` is `statement_failure`, which builds the four that are
      carried; `crates/nvs-stdlib/src/db.rs:5799` is `transacted`, whose doc now owns the account of
      how a kind reaches `nvs_runtime::KIND_SLOT` and is where a fifth value has to fit. The SQL text
      is developer-authored, so § 8 lets it ride where a bound parameter may not.
- [ ] **A sharing seam for the two drivers over MySQL's framing** (0067 § 2, known gap 2), and take
      this before the arms. `crates/nvs-stdlib/src/db.rs:4647` is `mysql_rows` and
      `crates/nvs-stdlib/src/db.rs:4764` is `mysql_write`; both take `&mut nvs_db::MySqlConn` by
      name, and `crates/nvs-db/src/maria.rs:423` is `MariaConn::query`, which answers the *same*
      `crate::MySqlRows<'_, NvsTls<NvsTcp>>` through the same `crate::mysql::start_statement`.
      Hoisting the `.query(...)` to the call site is the cheaper of the two shapes — the alternative
      names that rows type in a local trait, which drags `nvs_host` types into this module.
- [ ] **Then `open` opens a MariaDB** (0067 § 2, known gap 2), which is six arms once the seam
      exists: `crates/nvs-stdlib/src/db.rs:3311` is `open`'s match (the Postgres arm above the
      `other =>` is the shape), `crates/nvs-stdlib/src/db.rs:4135` is `warm_connection`'s reset,
      `crates/nvs-stdlib/src/db.rs:4350` is the `Transacting` enum and its four delegating arms,
      `crates/nvs-stdlib/src/db.rs:4417` is `transacting`, `crates/nvs-stdlib/src/db.rs:4494` is
      `queried_rows`. `crates/nvs-db/src/maria.rs:215` is `MariaTarget`, which carries `time_zone`
      and `statement_cache` exactly as `MySqlTarget` does. Then rewrite known gap 2 at
      `crates/nvs-stdlib/src/db.rs:88`, gap 3 at `crates/nvs-stdlib/src/db.rs:108` and `driverless`
      at `crates/nvs-stdlib/src/db.rs:4537`, whose message names the list.

## Backlog

- `connect` opens no MariaDB either — `crates/nvs-stdlib/src/db.rs:2955` is its MySQL arm, and it is
  the same seam once `open`'s is done. Known gap 2.
- SQL Server has no driver at all, which is what holds the goal's stage-7 check open. `nvs-db`.
- `crate::queue`'s four members are still PostgreSQL-only. Known gap 2's last sentence.
- `[db.<name>.pool]` has no spelling an `open` can reach, and no `pool = false`. Known gap 1.
