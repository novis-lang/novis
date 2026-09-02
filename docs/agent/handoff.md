# Handoff

## State

**Stage 7's reset half is two thirds landed**, both as unit tests over already-landed code in
`crates/nvs-db/src/pg.rs`'s `mod tests`, beside the four reset cases that were already there.

`postgres_resets_without_losing_its_statement_cache` (`crates/nvs-db/src/pg.rs:5234`) asserts § 13's
`DISCARD ALL` exclusion from the far side: not what the reset's flush says but what the flush *after*
it costs. One `start_statement` over a `StatementCache`, a `reset_session`, then the same SQL again —
and the third flush's tags are `BDES`, no `P`, with the cache still holding one entry. The scripted
server answers only what each batch asked for, so a driver that re-parsed stalls rather than passes.

`no_session_state_survives_a_return_to_the_pool` (`crates/nvs-db/src/pg.rs:5284`) sweeps § 13's
property list — the seven states a reset must remove — against the commands `reset_session` writes,
each property named in its own failure message, plus the reverse direction: a command in the reset
that no property asks for is a round trip nothing justifies. Two properties share `RESET ALL`, which
is why it is a sweep and not a zip.

**Stage 2's check still fails, and permanently** — its remaining name,
`local_infile_is_refused_and_no_file_is_sent`, is MySQL's, and no session should try to close it.
**Stage 7's check inherits the same shape**: `mysql_and_mssql_reset_through_the_protocol_and_lose_theirs`
is a second and third driver, so it stays open for the same reason. Only two of that check's names
are reachable now, and they are the group below.

## Next group

**Stage 7's last two reachable names — the file set is `crates/nvs-db/src/pg.rs` and
`crates/nvs-runtime/src/pool.rs`, which is § 13's pool and where a released connection's fate is
decided. Read `pool.rs`'s module doc first: it is the crate that owns the drop.**

- [ ] **A failed reset destroys the connection** — § 13's "a connection that cannot be proven clean
      is closed", as `a_failed_reset_destroys_the_connection_rather_than_returning_it`.
      `crates/nvs-db/src/pg.rs:769` is `PgConn::reset`, which already takes `mut self` and hands the
      connection back only on `Ok`, so the type is the mechanism and the missing name is the
      assertion over it. `crates/nvs-db/src/pg.rs:5328` and `crates/nvs-db/src/pg.rs:5355` pin that a
      refused reset reports and that a wire failure poisons; neither says the connection is dropped.
      The wall is the playbook's: `PgConn`'s `wire` is `Wire<NvsTls<NvsTcp>>`, so a `-p nvs-db` unit
      test cannot build one and cannot call `PgConn::reset` at all. Either assert it where the drop
      actually happens — `crates/nvs-runtime/src/pool.rs:520` is `release`, which is handed a
      `Box<dyn HeldConnection>` — or assert the type-level claim in `nvs-db` and say in the commit
      which. The goal's check is `-p nvs-db` (`docs/agent/loop-goal.toml:2915`); if the name has to
      live elsewhere, move the check as ADR 0132 § 1's crate edge already forced once.
- [ ] **The pool is per core and keyed as `connect` and `open` key** — § 13's second bullet, as
      `the_pool_is_per_core_and_keyed_as_connect_and_open_key`.
      `crates/nvs-runtime/src/pool.rs:152` is `Ticket::for_block`, which builds the key and scopes it
      to the configuration generation; `crates/nvs-runtime/src/pool.rs:127` is `Ticket` itself and
      `crates/nvs-runtime/src/pool.rs:590` that module's `mod tests`. The claim has two halves — two
      config blocks are two pools, and one name under two generations is two pools — and the second
      is the one a reload would otherwise break. Same `-p nvs-db` mismatch as above: the pool is
      `nvs-runtime`'s and the key is computed in `nvs-stdlib`, so nothing in `nvs-db` can host this
      name and the check's `args` is what has to move.

## Backlog

- `mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` — stage 7, blocked behind the
  standing decision that PostgreSQL lands first (`docs/agent/loop-goal.md`).
- `local_infile_is_refused_and_no_file_is_sent` — stage 2, the same block, and the ledger's report
  from now on.
- Stage 7's remaining `pool_reuse.rs` names are all on disk (`crates/nvs-db/tests/pool_reuse.rs`).
- `examples/pool.nvs`'s four lines are stage 7's program leg (`docs/agent/loop-goal.toml:2950`).
- `open` still waits on a shape-parameter type (`docs/implementation-plan.md`, Open now).
