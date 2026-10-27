- **`bun nv proofs --record-perf` under a borrowed `CARGO_TARGET_DIR` says "`cargo build --release -p
  nvs-cli` failed (exit 0)" after a whole release build.** The build went to that directory, and the
  recorder looks for the binary under the checkout's own `target/`. In a scratch worktree with a warm
  target borrowed from another, pass `--nvs <that dir>/release/nvs.exe`, and build it first with
  `cargo build --release --bin nvs`. See `tools/nv/proofs/run.ts`.
  `until: gone tools/nv/proofs/run.ts:!existsSync(path)`
