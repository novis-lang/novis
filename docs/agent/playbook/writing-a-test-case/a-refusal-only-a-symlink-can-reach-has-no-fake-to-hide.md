- **A refusal only a symlink can reach has no fake to hide behind.**
  `crates/nvs-stdlib/tests/capability.rs` swaps in a fake canonicalizer for this, but a member
  resolving through `nvs_runtime::capability::canonicalize` reaches `nvs_config::resolve::Disk` and
  takes no resolver, and Windows will not create a link without the privilege. Create the real one,
  return with a printed reason where the host refuses, and prove the assertions run once under
  `wsl.exe -- bash -lc 'CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo test …'`.
  [until: gone crates/nvs-runtime/src/capability.rs:resolve::Disk]
