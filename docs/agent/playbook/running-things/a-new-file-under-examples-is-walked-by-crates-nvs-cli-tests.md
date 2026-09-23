- **A new file under `examples/` is walked by `crates/nvs-cli/tests/ast.rs`, which asserts that every
  node's span sits inside its parent's and that siblings are in offset order.** An example writing
  `|>` fails both, because the parser substitutes the left side into a call written to its right, and
  the panic prints a JSON node rather than saying which property broke. Run `cargo test
  --test ast` when an example's tree will not follow its source, and hand that file the walk's
  `in_source_order = false`. [until: reviewed 2026-09-07]
