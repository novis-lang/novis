- **`bun nv proofs --id` says "no feature" for a `Core` member that a fresh `cargo build` just added.**
  Without `--nvs` the tool reads `target/release/nvs.exe`, which a debug build does not refresh. Pass
  `--nvs target/debug/nvs.exe`, or run a `--record-perf`, which builds the release binary first. See
  `tools/nv/proofs/collect.ts`.
