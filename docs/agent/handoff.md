# Handoff

## State

**Stage 5's acceptance check was two checks wearing one name, exactly as stage 4's was.** Three of its
seven tests are § 7's *closure* — the scope guard, the flag that survives a `catch (Throwable)`, the
retry loop — and none is a question `nvs-db` can be asked: the `BEGIN`, the `SAVEPOINT` and the
`ROLLBACK` are the driver's, but re-running a closure is
`nvs_core_db_connection_transaction`'s and nothing in a driver is ever asked to. They are now a
`-p nvs-stdlib` check of their own in both `docs/agent/loop-goal.toml` and
`docs/agent/goals/5-database.toml`; the four driver-level names stay where they were. The two goal files
are still **not** byte-identical — the live one carries ADR 0133's stage 0 — so an edit goes into both by
hand rather than by copying.

**§ 8's normalisation is now asserted across the drivers, not only down each one.**
`every_driver_normalises_its_codes_to_one_error_kind` in `crates/nvs-db/src/conn.rs` reads one condition
per row as PostgreSQL, MySQL and MariaDB each spell it and asserts the three answer one `DbErrorKind`;
the three `kind_of`s are `pub(crate)` for it, and its exhaustive `match` on the kind makes a twelfth
condition a build failure in that table. SQL Server and SQLite have no code table yet and so no column.

**`retries_recover_an_induced_deadlock` is the one name still open, and what it has is a wall rather
than a missing author.** `nvs_core_db_connection_transaction` reaches its connection through
`transacting`, which downcasts to a real `nvs_db::Connection::Postgres`/`::MySql`, and a `-p nvs-stdlib`
test can build neither — the playbook's `PgConn` bullet is the same wall one crate over. The closure
half is already reachable (`allocation_policy.rs`'s `closure_of`), so hoisting the loop off the ctx is
the whole of it, and it is the next group's first item.

**`nvs_stdlib::db`'s known gap 1 is untouched**: `open` still files its connection with no lease,
because § 13 keys an `open` pool on a hash of every settings field and
`nvs_runtime::pool::Ticket::for_block` takes a block *name*. Within a request § 2's memo holds.

## Next group

**The retry loop's testability, then § 13's pool for `open`. File set: `crates/nvs-stdlib/src/db.rs`
with `crates/nvs-runtime/src/pool.rs`.** The first item closes the stage 5 check this session split; the
second and third are the group the last two handoffs named and are unchanged.

- [ ] **The retry loop runs with no server in front of it** (0067 § 7). Hoist the body of the `loop` at
      `crates/nvs-stdlib/src/db.rs:5490` into a function that is *handed* its connection instead of
      reading it back out of the ctx at four points, so `retries_recover_an_induced_deadlock` can script
      one attempt that conflicts and one that commits. `crates/nvs-stdlib/src/db.rs:4293` is
      `Transacting` and `crates/nvs-stdlib/src/db.rs:4360` is the `transacting` that downcasts;
      `crates/nvs-stdlib/src/db.rs:7212` is the neighbouring case whose shape the new one takes.
- [ ] **A ticket keyed on the settings hash, so `open` pools** (0067 § 13).
      `crates/nvs-runtime/src/pool.rs:1` is the pool and `Ticket::for_block`'s block-name key;
      `crates/nvs-stdlib/src/db.rs:2995` is the merged-slot list `settings_key` hashes, and
      `crates/nvs-stdlib/src/db.rs:64` is known gap 1, which this closes. The key is § 2's — the hash,
      scoped to the configuration generation it was read from, exactly as a named block's is.
- [ ] **`{shared: false}` still draws from and returns to that pool** (0067 § 13).
      `crates/nvs-stdlib/src/db.rs:2986` is the comment that already says so with no lease behind it,
      and `crates/nvs-runtime/src/pool.rs:1` is where the draw happens.

## Backlog

- **`open`'s reset is the one a failed reset destroys** — `docs/adr/0067-core-db.md` § 13.
- **SQL Server and SQLite owe a `kind_of` and a column in the new agreement table** —
  `crates/nvs-db/src/conn.rs`, and § 8's "four drivers, five dialects" is not met until they have one.
- **§ 7's backoff and jitter between retries is unwritten** — `nvs_stdlib::db`'s known gap 9.
- **`Core\Db::open` has no pool of its own until known gap 1 closes** — `nvs_stdlib::db` known gap 1.
