- **An `@example` written inside `examples/` is refused unless it says `../examples/`, because the tag's
  two halves read two different paths.** `E0325`'s walked-directory test looks at the path *as written*
  (any `Normal` component named `examples` or `tests`), while `E0324`'s existence test resolves that same
  path relative to the file that wrote it — so `@example doc-comments.nvs`, sitting beside the file it
  names, is refused for being in no walked directory. Write the walked directory into the path and let
  `..` carry the resolution: `@example ../examples/doc-comments.nvs` passes both halves.
  [until: gone crates/nvs-hir/src/members.rs:let walked = written.components()]
