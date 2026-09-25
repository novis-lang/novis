- **A declaration inside a `try` block is in scope after the block, so a case that repeats a setup
  under a second heading fails with `E0406` rather than running.** The three `try` blocks of an
  agreement case naturally want the same variable names in the section that follows them, and the
  compiler sees one scope. Give the second set its own names; nothing about the assertion depends on
  reusing them. [until: gone crates/nvs-diagnostics/src/lib.rs:E0406]
