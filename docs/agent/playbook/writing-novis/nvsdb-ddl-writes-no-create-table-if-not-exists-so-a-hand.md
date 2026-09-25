- **`nvs_db::ddl` writes no `CREATE TABLE IF NOT EXISTS`, so a hand-written DDL list cannot be
  swapped for a `Schema` value on its own.** The emitter is deliberately unguarded —
  `mysql_declares_an_index_inside_its_create_table_having_no_if_not_exists` at
  `crates/nvs-db/src/ddl.rs:1210` asserts the absence — because a plan is computed from the current
  state and a guard has no portable spelling for an index anyway. So the command that ran the list
  has to become a convergence in the same slice, and any column the value adds has to be nullable to
  arrive as a `Safe` step on a table that already has rows. [until: gone crates/nvs-db/src/ddl.rs:mysql_declares_an_index_inside_its_create_table_having_no_if_not_exists]
