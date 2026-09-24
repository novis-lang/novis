- **`bun nv verify` runs `cargo fmt`, which rewrites the very text a perf record is keyed
  on, so a `--record-perf` taken before the wrap is stale by the time the gate reads it.** A
  member's figure is accepted while a hash of its implementing file matches, and formatting one line
  a session spliced in moves that hash for every member in the file. Measure the group after
  `nv verify` has come back green, not while it is still running. [until: reviewed 2026-09-20]
