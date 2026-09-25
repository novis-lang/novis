- **A live-server case over `Core\Queue`'s statements belongs in `crates/nvs-stdlib/tests/queue.rs`,
  and a moved case can skip silently.** `rule:core-classes/db-crate-boundary` has `nvs-stdlib`
  depend on `nvs-db`, so a `-p nvs-db` test cannot `use nvs_stdlib::queue::CLAIM_POSTGRES`, and
  `bun nv db-matrix` runs only the crates in `SUITES`. Prove it asserted, not skipped: `docker exec
  novis-db-postgres-1 psql -c "select …"` for the row it wrote. [until: gone crates/nvs-stdlib/tests/queue.rs:Core\Queue]
