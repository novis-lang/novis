- **A spelling admitted at an argument position is refused by `nvs-hir` first, and that crate cannot
  ask the registry.** `Mailer::send` parses as a `ClassConstAccess`, so `nvs_hir::members`' walk
  reports `E0309` and the run aborts before `nvs-types` sees the call at all. Carve the argument out
  in `walk_args_admitting_method_ref`'s roster as well as marking it in `nvs_types`.
  [until: gone crates/nvs-hir/src/members.rs:METHOD_REF_ARGS]
