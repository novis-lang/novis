- **A config fixture that fills only `Snapshot.table` bounds nothing, so a `set` above the
  `[limits.hard]` ceiling is accepted.** `nvs_config::Request::ceiling` reads the typed tree at
  `base.config.limits.hard` while `get` and `all` read `base.table`, so half a fixture answers
  plausibly and refuses nothing. Parse the TOML once and deserialize the same table into `config`
  (`crates/nvs-stdlib/src/config.rs`'s `configured`); `Ctx::memory_limit` is then the ceiling less
  `limits.fatal_reserve_memory`, so assert that it moved. [until: gone crates/nvs-stdlib/src/config.rs:Ctx::memory_limit]
