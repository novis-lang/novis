- **A `nv splice` patch cannot carry a patch, so an edit to prose *about* the patch format is one
  the `Edit` tool has to make.** A block runs from `<<<<<<< OLD` to the first `=======` after it, so
  a block whose own text quotes those markers ends in the middle of itself. The failure does not
  look like a parse error: it looks like a stale anchor in whatever file the truncated block landed
  on. [until: reviewed 2026-09-06]
