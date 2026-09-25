- **A hostile case built out of depth meets two limits that are not the ones it looks like: the
  parser stops at 96 levels of *grammar recursion*, about nineteen nested parentheses rather than
  ninety-six, and `var` refuses an array literal outright (`E0414`).** Both come back as a compile
  diagnostic, which the hostile runner counts as a failure rather than as the runtime surviving.
  Run the candidate with `target/debug/nvs.exe run` first, keep a legal nest near a dozen
  parentheses, and write the nested type out — `array<array<string>> $t = [["leaf"]];`.
  [until: gone crates/nvs-syntax/src/parser/mod.rs:MAX_RECURSION_DEPTH]
