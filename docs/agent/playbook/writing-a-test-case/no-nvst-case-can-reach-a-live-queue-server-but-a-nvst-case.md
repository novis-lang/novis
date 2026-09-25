- **No `.nvst` case can reach a live queue server, but a `.nvst` case can reach a live SQLite
  database.** `Core\Queue`'s statements exist for every driver but SQL Server
  (`nvs_stdlib::queue::runs`), so a runtime claim about a queue on a wire driver belongs in a
  `-p nvs-stdlib` `#[test]`, and a `queue-*.nvst` pushing to one ends in `--EXPECTF-ERROR--`. A `[db.main]` with `driver = "sqlite"` and `path = ":memory:"` opens inside a
  case, and a per-connection `:memory:` shows whether two calls shared one.
  [until: gone crates/nvs-stdlib/src/db/mod.rs:nvs_stdlib::queue::runs]
