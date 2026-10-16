- **A change to a `crates/nvs-stdlib/src/queue.rs` statement that only `queue_sqlite.rs` follows leaves
  `tests/queue.rs` binding the old count, and `bun nv verify` stays green.** The server suite returns
  early when `NVS_DB_MATRIX_DRIVER` is unset, so only `bun nv db-matrix` runs it, and it fails as
  "the statement takes 14 parameter(s) and 13 were bound" on every server. Change the push helpers
  and the `stats` arity in `crates/nvs-stdlib/tests/queue.rs` in the same commit, and run `bun nv
  db-matrix --all` once. `until: gone crates/nvs-stdlib/tests/queue.rs:fn push_keyed`
