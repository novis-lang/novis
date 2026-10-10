- **A type atom whose first token is already a statement keyword needs the *statement* arm guarded
  too, not just `can_start_type`.** `parse_statement`'s dispatch matches
  `TokenKind::Keyword(Keyword::Abstract | Keyword::Final | Keyword::Class)`
  (`crates/nvs-syntax/src/parser/stmt.rs`) above the `_ if self.can_start_type()` arm, so
  `class<Animal> $cls = …;` parsed as a class declaration with E0101s even with `Parser::at_class_reference` in `can_start_type`. Every keyword arm
  earlier in that match is a second door the new atom has to be let through; a test of `as` alone
  reports green on half a feature. `until: gone crates/nvs-syntax/src/parser/stmt.rs:can_start_type`
