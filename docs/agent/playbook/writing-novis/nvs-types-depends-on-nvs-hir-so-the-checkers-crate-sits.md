- **`nvs-types` depends on `nvs-hir`, so the checker's crate sits above name resolution, not below it.**
  A build that routes an earlier phase through something the checker owns — `require`'s path cooking
  through `nvs_types::string_lit` — is a dependency cycle Cargo refuses, not a call-site change, and the
  compile error names `and_then` or a trait bound rather than the cycle. Move the shared routine down to
  `nvs-syntax`, which `nvs-hir`, `nvs-types` and `nvs-ir` all already depend on, and leave a `pub use`
  where it was so no call site moves. [until: gone crates/nvs-types/Cargo.toml:nvs-hir.workspace]
