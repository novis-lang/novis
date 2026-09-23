- **`python tools/dossier.py --record-perf` measures a member against `target/release/nvs.exe`
  without rebuilding it, so a session that edited `crates/` measures the previous session's code.**
  It prints `target/release/nvs.exe is missing or older than the tree` in one line and goes on, and
  the goal's own acceptance check reports that same line as a failure until somebody builds. Run
  `cargo build --release -p nvs-cli` after the last Rust edit and before `--bless` or
  `--record-perf`; `verify.py`'s `fmt` step rewriting a crate file counts as an edit and makes the
  binary stale again. [until: reviewed 2026-09-20]
