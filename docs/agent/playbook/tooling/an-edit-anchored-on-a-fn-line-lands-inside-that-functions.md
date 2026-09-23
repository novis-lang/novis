- **An `Edit` anchored on a `fn` line lands *inside* that function's doc comment, and nothing
  reports it.** An item inserted before `fn name(` splits the `///` block above it: the head becomes
  the new function's documentation, the tail the old one's, and `cargo check` is green because both
  are well-formed. Rust has no marker for where a doc block starts, so anchor on the blank line
  after the previous function's closing brace, or on the function's own first `///` line, and put
  the new item before it. [until: reviewed 2026-09-06]
