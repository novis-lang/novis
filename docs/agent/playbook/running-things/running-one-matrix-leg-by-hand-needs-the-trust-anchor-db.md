- **Running one matrix leg by hand needs the trust anchor `db-matrix` exports and then deletes, and
  `NVS_DB_MATRIX_CA` must be absolute.** Cargo runs a unit test with the crate directory as its cwd,
  so a relative one panics as *the matrix server accepts a handshake* with a `NotFound` inside it.
  Copy it out with `docker compose -f tests/db/compose.yaml cp <service>:/certs/ca.crt <abs path>`,
  and prefix any `docker` call carrying a container-side path with `MSYS_NO_PATHCONV=1`, or Git Bash
  rewrites it to `C:/Program Files/Git/...` first. [until: reviewed 2026-09-15]
