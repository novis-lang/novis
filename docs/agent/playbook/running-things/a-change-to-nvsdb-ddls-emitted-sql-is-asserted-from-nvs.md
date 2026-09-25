- **A change to `nvs_db::ddl`'s emitted SQL is asserted from `nvs-stdlib` too, so a green
  `nv verify -p nvs-db` says nothing about it.** `Core\Queue`'s schema is a `Schema` value and its
  cases read the DDL the emitter writes for it (`crates/nvs-stdlib/src/queue.rs`), which is the way
  `rule:core-classes/db-crate-boundary` points that edge on purpose — the statements have one home,
  and what asserts over them lives where both crates are visible. Run `nv verify -p nvs-stdlib`
  beside the `-p nvs-db` one, or go straight to the full gate. [until: gone crates/nvs-stdlib/src/queue.rs:nvs_db::ddl]
