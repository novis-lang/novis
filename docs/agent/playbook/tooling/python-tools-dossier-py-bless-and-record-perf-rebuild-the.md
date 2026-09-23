- **`python tools/dossier.py --bless` and `--record-perf` rebuild the release binary, so either one
  takes minutes rather than a second once a crate has been edited.** Both run the programs through
  `target/release/nvs.exe`, and cargo relinks the workspace as soon as its source has moved, with no
  output at all until the build is done — a call that returned instantly earlier in the same session
  reads as a hang. Bless the examples before touching Rust, or start the call in the background and
  write the next proof while it builds. [until: reviewed 2026-09-21]
