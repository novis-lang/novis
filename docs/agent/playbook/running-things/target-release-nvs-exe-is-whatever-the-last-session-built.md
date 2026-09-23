- **`target/release/nvs.exe` is whatever the *last* session built, and rebuilding it costs two
  minutes for a verdict the debug binary already gives.** A `.nvst` case a stale binary fails may
  simply predate it, and `cargo build --release -p nvs-cli` relinks the world for a one-line edit.
  Build `cargo build` and run `target/debug/nvs.exe`: it is the binary `tools/loop.py` builds after
  every acceptance check, so it is the cheap answer and usually the faithful one. The exception is
  `tools/dossier.py`, which runs every proof program against release: `--bless` prints
  `target/release/nvs.exe is missing or older than the tree -- cargo build --release -p nvs-cli`
  and then runs that build itself (`tools/dossier.py:513`), so the line is the start of a
  six-minute wait rather than a refusal, and the driver's acceptance sweep reports the same line.
  Make every `crates/` edit of the group before the first `--bless` or `--record-perf`, because
  each edit buys another relink.
  [until: reviewed 2026-09-20]
