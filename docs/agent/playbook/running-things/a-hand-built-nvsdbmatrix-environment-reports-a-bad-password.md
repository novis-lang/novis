- **A hand-built `NVS_DB_MATRIX_*` environment reports a bad password as a poisoned `Once` in
  whichever case ran second.** `schema()` holds a `Once`, so the first failed handshake poisons it
  and every later case panics at `crates/nvs-stdlib/tests/queue.rs:394` naming neither the server nor
  the case that failed. Drive the suite with `bun nv db-matrix --driver <name>`, which exports the
  whole group; by hand, the password is `tests/db/compose.yaml`'s `POSTGRES_PASSWORD` and not the
  user name. [until: gone tools/nv/cmd/db-matrix.ts:--driver]
