- **A stack overflow from `target/debug/nvs.exe` on a deep expression tree that
  `target/release/nvs.exe` walks is the debug build's fatter frames, not a bug you just wrote.** A
  48-stage `|>` chain runs in both, a 64-stage one only in release, and a 400-term `$n + 1 + 1 ...`
  does the same: the parser's guard counts stack frames, and a left-associative chain is a loop that
  charges it none. Run a hostile case or an example against both binaries before committing it;
  `bun nv proofs --run` runs on `target/proof/`, a release-profile build, and judges only that side.
  [until: gone crates/nvs-syntax/src/parser/mod.rs:enter_recursive]
