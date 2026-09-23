- **A `close`d SQLite `:memory:` connection is handed straight back by the pool, so a case cannot
  assert that a reconnect is a fresh database.** `connect`, create a table, `close`, `connect` again
  prints `table: found`, because the release pooled the connection and the second `connect` got it
  back, database and all. Assert what is true of the two handles — the closed one is still refused,
  the new one runs a statement. [until: reviewed 2026-09-06]
