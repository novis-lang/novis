- **A closure parameter cannot be written without a type, and nothing in `nvs-types` is where that is
  decided.** `parse_param` (`crates/nvs-syntax/src/parser/expr.rs:1688`) is shared by every parameter
  list in the language and reports `E0101` for a missing type, so `fn ($u) => ...` never reaches the
  checker at all and a fixture written to test inference dies in `check_src`'s parse assertion rather
  than in the assertion it was written for. Relax it there, for a closure's list alone, before writing
  any checker test that omits a parameter type.
  [until: gone crates/nvs-syntax/src/parser/expr.rs:"expected a parameter type"]
