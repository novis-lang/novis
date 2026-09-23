- **`cargo check --target aarch64-apple-darwin` cannot check an aarch64 `#[cfg]` from this machine.**
  It builds the whole graph for that target, and `ring`'s build script hands Apple flags (`-arch`,
  `-mmacosx-version-min=11.0`) to the local `cc`, which refuses them before any crate of this
  repository is reached. Keep architecture-dependent code to constants and pure functions a
  host-independent unit test can still exercise — `crates/nvs-cli/src/cache.rs`'s aarch64 relocation
  writers are the shape — and leave the cfg itself to the CI matrix.
  [until: reviewed 2026-09-17]
