- **A diagnostic raised while parsing a type is discarded when that type sits in a local
  declaration.** `parse_stmt_maybe_local_decl` trial-parses the type and backtracks on
  `self.diags.len() > cp.diags_len` (`crates/nvs-syntax/src/parser/stmt.rs:956`), so a precise
  refusal from `parse_type_atom` is dropped and the reader gets `expected an expression` instead.
  Write its fixture in a parameter, property or `type` alias, and collect it with
  `check_src_allowing_parse_errors`.
  [until: gone crates/nvs-syntax/src/parser/stmt.rs:cp.diags_len]
