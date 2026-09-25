- **A parser loop that left-nests a tree is charged nothing by `enter_recursive`, so the guard
  bounds the descent and never the result.** A left-associative tier consumes its chain in a loop
  rather than by recursing, and the depth that matters is what the first recursive walk over the
  AST must survive, not the parser's own stack. A tier written outside `parse_left_assoc` charges
  one level per link and holds it to the end of the chain, as `parse_postfix`'s `chain_len` does.
  [until: gone crates/nvs-syntax/src/parser/expr.rs:parse_left_assoc]
