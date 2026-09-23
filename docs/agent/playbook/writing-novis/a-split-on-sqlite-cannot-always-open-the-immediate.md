- **A `Split` on SQLite cannot always open the immediate transaction its statement doc names.**
  `nvs_db::SqliteConn::begin_immediate` refuses a connection already in one, being outermost by
  construction, so a member run on the request's shared connection would refuse the very enqueue
  `rule:concurrency/enqueue-commits-with-your-write` says commits with the caller's write. Branch on
  `depth()`: `begin_immediate` at zero and `begin(None, false)`'s savepoint inside one, which is
  `nvs_stdlib::queue`'s `sqlite_opened`.
  [until: gone crates/nvs-db/src/sqlite.rs:immediate transaction is an outermost one]
