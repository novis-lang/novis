- **Relaxing a parser refusal can open an ICE three crates away, because an `Option` in the AST
  doubles as "a diagnostic was already reported".** `Param::ty` was `None` only after `E0101`,
  so `nvs_ir::lower::closure` read it through a `panic!` no program could reach — until a
  closure literal was allowed to omit the type. Grep the *consumers* of the field a relaxation
  leaves empty for `unwrap`, `expect` and `panic!`, and repair it by recording the checker's
  answer under a span they can still find.
  [until: gone crates/nvs-syntax/src/ast.rs:Option<Type>]
