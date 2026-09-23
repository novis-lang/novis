- **A `peek.py` `"## Heading"` target never matches a `# Known gaps` block.** That heading lives
  inside a `//!` doc comment, so the line starts with the comment marker and the heading locator —
  which matches a line beginning with the hashes — reports no heading and prints nothing. Every
  module doc this goal reads holds one, so ask for it as `crates/nvs-ir/src/lib.rs:re:Known gaps:40`
  or take the line number from `python tools/owners.py` and read a range around it.
  [until: reviewed 2026-09-16]
