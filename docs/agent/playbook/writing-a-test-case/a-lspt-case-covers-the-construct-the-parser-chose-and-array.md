- **A `.lspt` case covers the construct the *parser* chose, and `array<mixed> $rows = [1, 2, 3];`
  is a `LocalDecl` rather than the refused declaration it reads as.** The matrix's `Error` column
  comes from a member the parser could not read at all — `var $total = 1;` inside a class body —
  while a top-level annotated declaration parses cleanly and only the checker objects to it. Write
  the case, run `nvs lsp-test <dir> --coverage`, and read which cell moved; nothing else says where
  the cursor landed. [until: gone crates/nvs-lsp/src/definition.rs:LocalDecl]
