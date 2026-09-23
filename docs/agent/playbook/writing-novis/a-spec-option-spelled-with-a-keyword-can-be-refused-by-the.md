- **A spec option spelled with a keyword can be refused by the parser rather than by any rule, and
  the diagnostics never say "keyword".** `default`, `match`, `class` and `for` are all plausible
  option names, and `{default: "ada"}` produced `E0101 expected a field name` and a cascade of
  recoveries, none naming the cause. Before renaming an option away from what its rule spells, check
  `crates/nvs-syntax/src/token.rs`'s keyword table;
  `nvs_syntax::parser::expr::parse_object_literal_fields` is where an object-literal field was
  widened to take one. [until: reviewed 2026-09-06]
