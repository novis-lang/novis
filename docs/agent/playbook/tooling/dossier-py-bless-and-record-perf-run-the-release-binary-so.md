- **`dossier.py --bless` and `--record-perf` run the *release* binary, so the first call after any
  Rust edit spends minutes rebuilding it before it does anything.** The tool prints
  `target/release/nvs.exe is missing or older than the tree` and then stays silent, which reads
  like a hang rather than a build. Bless the examples and record the figures before you touch a
  crate, or start the call in the background and write the handoff while it runs.
  [until: reviewed 2026-09-21]
