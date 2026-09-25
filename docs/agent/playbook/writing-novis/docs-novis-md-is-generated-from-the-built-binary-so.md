- **`docs/novis.md` is generated from the built binary, so regenerating before rebuilding writes
  nothing and says "unchanged".** `bun nv reference --no-examples` reads `nvs meta --json`
  out of `target/debug/nvs.exe`, which is true of the binary and false of the tree after a source
  edit. The order is `bun nv verify` (which rebuilds), then regenerate, then `--check`.
  [until: gone tools/nv/cmd/reference.ts:--no-examples]
