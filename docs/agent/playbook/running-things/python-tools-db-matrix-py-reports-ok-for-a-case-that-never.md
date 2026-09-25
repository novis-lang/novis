- **`bun nv db-matrix` reports `ok` for a case that never ran, so a green matrix proves nothing
  about a case you just wrote.** Every case returns early when its gate answers `None` —
  `NVS_DB_MATRIX_DRIVER` unset, a gate misspelled, `framed()` where `postgres()` was meant — and the
  tool runs `cargo test -q`, so a skip and a real assertion print the same `ok`, and wall clock says
  nothing either. Confirm once by breaking the case's own assertion and re-running the tool: a leg
  naming your case in its `FAILED` line ran it, then revert. [until: gone tools/nv/cmd/db-matrix.ts:NVS_DB_MATRIX_DRIVER]
