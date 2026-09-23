- **A `[limits]` reader answers off `Snapshot::table`, not the typed `Config` beside it.**
  `Ctx::configured_memory_limit` reads `self.base.table` through `Request::get`, so `Snapshot {
  config: Config { limits: Some(..), .. }, .. }` reports no ceiling. Build both halves from one TOML
  string (`parse::<toml::Table>()`, then `table.clone().try_into()`) as
  `crates/nvs-runtime/tests/configured_limits.rs` does.
  [until: gone crates/nvs-config/src/snapshot.rs:pub table: toml::Table]
