- **A deeply nested literal crashes `target/debug/nvs.exe` where the release binary refuses it
  cleanly.** The parser's recursion guard reports `E0108` from nineteen nested `[` on, but the debug
  build's larger frames overflow its 1 MiB main-thread stack one level past that, so the probe prints
  `has overflowed its stack` and no diagnostic. Probe a nesting limit with `target/release/nvs.exe`,
  which `parser::MAX_RECURSION_DEPTH`'s own doc comment now says.
  [until: gone crates/nvs-syntax/src/parser/mod.rs:This bounds the parse, not the stack a build runs it]
