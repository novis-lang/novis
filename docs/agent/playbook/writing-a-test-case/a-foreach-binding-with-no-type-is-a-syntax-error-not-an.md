- **A `foreach` binding with no type is a syntax error, not an untyped binding**: `rule:types/grammar`.2
  makes every binding write its type, so `foreach ($xs as $v)` reports `E0101` at the name and a case
  asking what a bare binding carries never gets that far. A handoff item can name the construct in that
  PHP shape anyway, because nothing in an LSP file set contradicts it. Run `nvs check` over the
  `--FILE--` document before freezing anything about it. [until: gone crates/nvs-diagnostics/src/lib.rs:foreach]
