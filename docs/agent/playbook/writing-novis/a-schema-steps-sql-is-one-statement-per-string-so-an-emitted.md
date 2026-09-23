- **A schema step's SQL is one statement per string, so an emitted `DECLARE … ; IF …` batch is wrong
  even where it is valid T-SQL.** `crates/nvs-db/src/direct.rs:142` is the contract — a driver takes a
  statement, never one string with several `;` in it. A step needing a value only the server holds
  writes one guarded statement instead, repeating its lookup rather than holding it in a variable:
  `crates/nvs-db/src/ddl.rs`'s `drop_default_constraint` is that shape.
  [until: gone crates/nvs-db/src/direct.rs:never one string with several]
