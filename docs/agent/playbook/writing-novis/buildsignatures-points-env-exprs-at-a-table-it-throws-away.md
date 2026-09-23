- **`build_signatures` points `env.exprs` at a table it throws away, so nothing the signature pass
  lowers is readable afterwards.** `lower_type` records every annotation it resolves under
  `ExprTypeTable::record_type`, which reads as though a property's declared type were available
  later — it is not, because that pass runs against `placeholder_exprs`. Handing it the real table
  is not the fix either: `nvs_ir::lower::lower_decl_type` consults `declared_ty` *first*, so every
  property annotation would silently change lowering path. Carry what a later pass needs across on
  its own, the way `SignatureTable::property_default_types` does.
  [until: gone crates/nvs-types/src/signatures.rs:let mut placeholder_exprs]
