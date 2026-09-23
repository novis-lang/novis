- **A by-value `reset(self)` on a connection whose rows borrow it cannot be tested against a
  streaming result set; the case will not compile.** `SqliteRows` (`crates/nvs-db/src/sqlite.rs`)
  holds a `&Cell<State>` into its `SqliteConn`, so `conn.reset()` while rows are alive is E0505
  rather than the runtime refusal. Set `State::Poisoned` directly and assert the state a caller
  reaches with no rows in hand; the borrow checker holds the other half.
  [until: reviewed 2026-09-06]
