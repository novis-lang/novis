- **A `.nvst` case cannot run `nvs queue migrate`, so a case over a converged database converges it
  itself.** `--RUN--`'s spellings are a closed set with no operator subcommand among them, and the
  schema has no program-reachable form — `nvs_stdlib::queue::schema()` is Rust. Take the DDL from
  `nvs queue migrate --config examples/queue-sqlite.toml --dry-run` into the case's own `--FILE--`,
  and create `nvs_dead_jobs` beside `nvs_jobs`: `status` reads a receipt back from both.
  [until: gone crates/nvs-stdlib/src/queue.rs:nvs_dead_jobs]
