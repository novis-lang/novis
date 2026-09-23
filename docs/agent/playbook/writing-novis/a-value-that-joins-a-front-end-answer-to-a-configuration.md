- **A value that joins a front-end answer to a configuration digest cannot be computed in the front
  end, and `Cargo.toml` says so before the anchor does.** `crates/nvs-hir` does not depend on
  `nvs-config`, so `Digest`, `EnvHash` and the program-id combine are unspellable there and the seam
  is the host, `crates/nvs-cli/src/main.rs`, which holds both halves. When an anchor names the crate
  holding one input, read the `Cargo.toml` of the crate holding the other before opening the file.
  [until: reviewed 2026-09-06]
