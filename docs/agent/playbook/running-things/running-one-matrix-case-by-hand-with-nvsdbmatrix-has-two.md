- **Running one matrix case by hand with `NVS_DB_MATRIX_*` has two traps, both a failing
  assertion.** `cargo test` runs in the crate directory, so a relative `NVS_DB_MATRIX_CA` resolves
  under `crates/nvs-db/` and fails `NotFound`; and the password's one home is
  `tests/db/compose.yaml`, so `matrix.rs` has no default. Give the anchor absolute (`docker compose
  -f tests/db/compose.yaml cp <svc>:/certs/ca.crt <dir>` exports it), or run `python
  tools/db-matrix.py --driver <name> --no-up`. [until: reviewed 2026-09-06]
