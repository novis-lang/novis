- **`dossier.py --bless` and `--record-perf` run `target/release/nvs.exe`, so a Rust edit between
  writing a proof and blessing it costs a full release rebuild.** A `covers:` marker added to
  `arr.rs` after three examples were blessed made the binary stale, and the rebuild then ran twice
  because another process held a copy of `nvs.exe`. Make every Rust edit a slice needs first, then
  bless and record in one pass. [until: reviewed 2026-09-20]
