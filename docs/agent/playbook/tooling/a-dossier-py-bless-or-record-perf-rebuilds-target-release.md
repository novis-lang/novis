- **A `dossier.py --bless` or `--record-perf` rebuilds `target/release/nvs.exe` whenever any Rust in
  the tree has moved, and that build is minutes, not seconds.** A slice that writes its examples,
  blesses them, then edits a crate for the Rust-side test and blesses the next slice pays that build
  twice, and a `| tail` on the call hides every line of it so the second one reads as a hang. Make
  every Rust edit of the group first, then bless and record once for the whole group, and never pipe
  a `dossier.py` call. [until: reviewed 2026-09-20]
