- **`nvs-ir` names `nvs_hir::…` freely in its doc comments and cannot name it in code: `nvs-hir` is
  one of its `[dev-dependencies]`, not a dependency.** `InstKind::New { class:
  nvs_hir::errors::FINISH_MARKER }` reads exactly like every neighbouring doc reference and fails
  with `E0433: unresolved module or unlinked crate`. The route down is a re-export through
  `nvs-types`, which depends on both — add one beside `CORE_SCRIPT_FINISH_CLASS` in
  `crates/nvs-types/src/lib.rs` rather than promoting the dev-dependency.
  [until: gone crates/nvs-ir/Cargo.toml:nvs-hir.workspace = true]
