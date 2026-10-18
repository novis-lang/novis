- **`bun nv render --website` after `nv verify` can say "already current" over a card you just
  changed.** It runs the newest of `target/debug` and `target/release`, and `nv verify` builds into
  `target/covws/<triple>/debug`, so the render reads a binary built before the edit. Run it as
  `NVS_BIN=target/covws/x86_64-pc-windows-msvc/debug/nvs.exe bun nv render --website` (the triple
  your host has), and grep `website/src/data/core.json` for the new text before committing.
  See `tools/nv/renderers/website-core.ts`. `until: gone tools/nv/renderers/website-core.ts:the most recently built of`
