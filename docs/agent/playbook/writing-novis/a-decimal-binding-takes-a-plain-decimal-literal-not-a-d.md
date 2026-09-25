- **A `decimal` binding takes a plain decimal literal, not a `d` suffix.** `decimal $d = 1.25d;` is
  four errors deep — the lexer reads `1.25d` as a duration literal and reports `E0007: `.` has no
  meaning in a duration`, then the checker reports the wreckage as `E0401: expected `decimal`, found
  `mixed`` — and none of them names the real problem. `rule:types/numeric-literal-placement` makes a
  numeric literal untyped until placed, so `decimal $d = 1.25;` is the whole spelling.
  [until: gone crates/nvs-diagnostics/src/lib.rs:E0007]
