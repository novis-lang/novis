- **Giving a `Core` class its card breaks an LSP hover case in another crate.**
  `tests/lsp/hover/a-namespace-segment-answers-what-it-contains.lspt` freezes every class of a
  namespace with its card's `short`, so the `nvs-lsp` coverage test fails only at `nv verify`.
  Add the new `short` to that case's line for the class in the same slice as the card.
  [until: gone crates/nvs-stdlib/src/registry.rs:CLASSES_STILL_OWING_A_CARD]
