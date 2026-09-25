- **A real SQL Server case cannot keep rows in a temporary table, and `sp_reset_connection` does not
  restore the isolation level.** A `CREATE TABLE #t` inside `sp_prepexec` is gone before the next
  statement, and the reset leaves `transaction_isolation_level` where the last transaction set it.
  Use a permanent table, `DROP TABLE IF EXISTS` first, and have `reset_session` send the isolation
  restore per `rule:security/db-pool-reset-is-a-boundary`. [until: gone crates/nvs-db/src/tds/plan.rs:reset_session]
