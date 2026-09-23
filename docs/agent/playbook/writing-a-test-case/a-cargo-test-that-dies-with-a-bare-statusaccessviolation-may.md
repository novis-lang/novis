- **A `cargo test` that dies with a bare `STATUS_ACCESS_VIOLATION` may not reproduce, so run it
  again before bisecting.** One arrived in `-p nvs-codegen --test throwing` on the first run after a
  relink and never returned. The deterministic cause — an empty argument slice, the neighbouring
  bullet — reproduces every time, which is how the two are told apart. [until: reviewed 2026-09-06]
