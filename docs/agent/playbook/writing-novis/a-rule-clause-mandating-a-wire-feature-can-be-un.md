- **A rule clause mandating a wire feature can be un-implementable for reasons only the sibling
  driver's code shows.** `COM_STMT_BULK_EXECUTE` for MariaDB's `executeMany` was settled against by
  `crates/nvs-db/src/pg.rs` flushing every `Bind`/`Execute`/`Sync` in one `wire.send` and by
  `mysql.rs`'s `Prepared` doc saying a prepare reports `0` columns for a data-dependent result set,
  neither of which the rule mentions. Read the sibling driver and the struct docs before budgeting a
  feature from the rule's text. [until: reviewed 2026-09-06]
