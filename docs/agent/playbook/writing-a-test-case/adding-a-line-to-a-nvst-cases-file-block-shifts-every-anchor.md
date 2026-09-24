- **Adding a line to a `.nvst` case's `--FILE--` block shifts every anchor its `--EXPECTF-ERROR--`
  names.** The expected diagnostics carry `--> case.nvs:NN:CC` counted from the start of that block, so
  one inserted `// covers:` marker moves all of them by one and the case goes red on the anchors alone.
  Bump each `NN` in the same `nv splice` patch, and run `target/release/nvs.exe test <case>.nvst` before
  the wrap. [until: reviewed 2026-09-20]
