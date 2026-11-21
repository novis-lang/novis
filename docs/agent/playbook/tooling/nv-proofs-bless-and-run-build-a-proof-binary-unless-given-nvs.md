- **`bun nv proofs --bless` and `--run` build a proof binary with cargo unless given `--nvs`.**
  A writing subagent's `--bless` started `cargo build --profile proof`, waited on another session's
  cargo lock and was killed by the hang guard after 90 s. Pass `--nvs target/debug/nvs.exe` to
  check examples against the debug build the session already has. See `tools/nv/cmd/proofs.ts`.
