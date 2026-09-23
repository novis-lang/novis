- **A slice the plan files under `nvs-ir` can be unbuildable there, because that crate holds no
  class table.** `Lowering` carries `exprs`, `checked_types` and an `EnumTable` and nothing else
  about a declaration, so a set the checker derived from the hierarchy cannot be re-derived below
  the erasure. Record the set in the checker as `ExprInfo::EnumCase`/`ExprInfo::PropertyKey` do and
  read it back in `nvs-ir`, keyed by the annotation's span when the lowering holds the `Type` node
  and not the `Expr`. [until: reviewed 2026-09-06]
