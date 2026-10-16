- **`bun nv proofs --id` with no `--run` reads its roster from whatever `nvs` is already built, so a
  member added since that build is "no feature" while `cargo build` is green.** Only `--run`,
  `--bless`, `--verify` and `--record-perf` build a binary first; a plain read took a
  `target/release/nvs` older than the new `Core\Response\Stream::offset`, which
  `target/debug/nvs meta --json` already listed. Pass `--nvs target/debug/nvs.exe` to a roster read
  after a debug build. `until: gone tools/nv/cmd/proofs.ts:if ((executes || measures) && named === undefined)`
