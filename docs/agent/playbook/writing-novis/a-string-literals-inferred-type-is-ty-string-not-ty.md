- **A string literal's inferred type is `Ty::String`, not `Ty::StringLiteral`, unless the position
  already expected that exact literal.** `placed_literal` (`crates/nvs-types/src/expr/literals.rs:55`)
  places a literal type only against an expectation naming it, so a check that reads a written word off
  the inferred type sees `string` and matches nothing — which looks like the word was accepted. Read the
  word off the AST instead, `ExprKind::Str(span)` through `crate::string_lit::cook_string_literal`, the
  way `spawn script`'s `on:` does. [until: gone crates/nvs-types/src/expr/literals.rs:placed_literal]
