- **The first `python tools/dossier.py --run`, `--verify`, `--bless` or `--record-perf` after an edit
  under `crates/` sits for minutes, and it is building, not hanging.** Those four rebuild
  `target/release/nvs.exe` themselves when a build input is newer than it, and say so on stderr
  before cargo starts. Do not run `cargo build --release` for it and do not pass `--nvs
  target/debug/nvs.exe` to get around it: the attacks are sized for release, and one in five of them
  outlives its own `timeout-ms` against the debug binary.
  [until: gone tools/dossier.py:def current_binary]
