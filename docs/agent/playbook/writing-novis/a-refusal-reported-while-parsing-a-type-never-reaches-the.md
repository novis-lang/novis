- **A refusal reported while parsing a type never reaches the reader at statement-initial
  declaration position.** `parse_stmt_maybe_local_decl` settles `T $x = …;` with a trial parse and
  reads *any* diagnostic as proof the tokens were an expression, so `3.14 $b = 1.0;` reports
  `E0101 expected ';'` and never `E0120`. Probe a new type-position refusal from a parameter or
  return type, where nothing backs out, and say in its doc comment that the statement head is the
  one position it does not reach.
  [until: gone crates/nvs-syntax/src/parser/stmt.rs:diags_len]
