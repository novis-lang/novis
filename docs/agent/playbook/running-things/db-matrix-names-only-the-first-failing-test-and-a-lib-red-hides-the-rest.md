- **`bun nv db-matrix` names only the first failing test of a leg, and a red `nvs-db` lib test
  stops that leg before its integration suites run.** A leg runs `cargo test` over `nvs-db`, and
  cargo stops at the first binary that fails, so one stale unit test hides every live case and
  the next run finds the next one. Before a matrix run, run `bun nv verify --fast -p nvs-db` once to
  list every red in the crate, and fix them together. To iterate on one live case, run it alone with
  the six `NVS_DB_MATRIX_*` variables `tools/nv/cmd/db-matrix.ts` sets and `cargo test --test
  <file> -- <case>`. `until: gone tools/nv/cmd/db-matrix.ts:SUITES`
