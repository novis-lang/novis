- **`lsp_types` 0.97's `ServerCapabilities` has no `typeHierarchyProvider` member**, though it carries the
  three requests, their params and the client half, so the gap reads as a misspelling. A capability the
  struct cannot express is one no client ever asks about, which looks exactly like a handler nobody wired
  up. `nvs_lsp::declared_capabilities` serializes the struct and inserts the key into the object.
  [until: gone crates/nvs-lsp/src/capabilities.rs:typeHierarchyProvider]
