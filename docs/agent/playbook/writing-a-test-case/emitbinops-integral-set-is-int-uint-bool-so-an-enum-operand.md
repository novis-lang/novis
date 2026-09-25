- **`emit_binop`'s `integral` set is `Int | Uint | Bool`, so an enum operand needs the
  reinterpretation first.** `==` over two enum values lowers because `nvs-ir` compares one
  representation down — `InstKind::Reinterpret` to the backing integer is free, the row `$m as int`
  already uses. A new lowering that emits a `BinOp` over `Ty::Enum` directly fails with *"a `Eq`
  over representation Enum(Int)"*. [until: gone crates/nvs-codegen/src/emit.rs:InstKind::Reinterpret]
