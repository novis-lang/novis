- **A test that walks this repository's `Cargo.toml` files cannot be proved by seeding the line it
  exists to catch into a workspace member, because cargo re-resolves every member before the test
  binary runs and a fictional dependency stops at the network.**
  `crates/nvs-runtime/tests/manifest_policy.rs` walks every `Cargo.toml` outside `target/` and
  `.git/`, and `benches/*` is a member of the root workspace while `fuzz/Cargo.toml` declares its own
  empty `[workspace]`. Seed the line in `fuzz/`, watch the assertion fail naming that path, and revert
  — that walk reads the file and cargo never resolves it. [until: gone crates/nvs-runtime/tests/manifest_policy.rs:fuzz]
