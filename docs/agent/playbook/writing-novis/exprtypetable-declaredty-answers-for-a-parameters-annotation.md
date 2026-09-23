- **`ExprTypeTable::declared_ty` answers for a parameter's annotation and not for a property's**: the
  signature pass lowers a property's type into a table of its own, and `nvs_types::check` copies types
  out of it and not spans. So a modifier read off `declared_ty` at a property declaration is always zero
  and fails no build. Read it off the access's own `ExprInfo::Property` entry instead, the way
  `nvs_lsp::semantic`'s `qualifiers_recorded` does — or, at a declaration where there is no access to
  ask, off `ExprTypeTable::property_default_ty`. [until: gone crates/nvs-lsp/src/semantic.rs:copies types out of and not]
