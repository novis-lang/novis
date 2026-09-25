- **`bun nv peek --locate` answers one line per file, so a name two `impl` blocks in one file share
  resolves silently to the first one.** `--locate set_peer` reports
  `crates/nvs-runtime/src/ctx/inbound.rs:623` and never the second `set_peer` in that same file. For
  a name a file carries twice, one `grep -n 'fn <name>' <file>` names both and is the only form that
  does. [until: gone tools/nv/cmd/peek.ts:--locate]
