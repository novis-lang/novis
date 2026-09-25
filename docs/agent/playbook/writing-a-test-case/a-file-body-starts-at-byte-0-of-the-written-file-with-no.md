- **A `--FILE--` body starts at byte 0 of the written file with no leading newline, which an
  offset-0 case relies on.** `crates/nvs-test/src/case.rs`'s section reader takes the lines after
  the header verbatim, so `a-shebang-line-opens-code.nvst` hands the lexer `#!` at offset 0. Had the
  harness kept the separator's newline, a case about the first bytes of a file
  (`rule:tooling/shebang-opens-code-mode`) would pass while testing nothing; know this before
  writing one. [until: gone tests/conformance/core/a-shebang-line-opens-code.nvst:rule:tooling/shebang-opens-code-mode]
