- **A `\` at the end of a line inside a Novis string literal is a literal backslash, not a
  continuation, and a long SQL statement is where that bites.** Rust's `"…\` + newline eats the
  newline and the next line's indentation, so a `create table` wrapped that way reaches the driver
  as `…, \` plus the indentation, and SQLite answers `unrecognized token: "\"` at an offset in a
  statement the case never wrote. Keep a statement literal on one line however long it gets, or
  build it by concatenation. [until: reviewed 2026-09-06]
